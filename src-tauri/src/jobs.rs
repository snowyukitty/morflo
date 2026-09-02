use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};

use tauri::{AppHandle, Emitter};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader},
    process::Command,
    sync::{OwnedSemaphorePermit, Semaphore},
    time::timeout,
};
use tokio_util::sync::CancellationToken;

use crate::{
    domain::{
        ConversionError, ConversionErrorCode, ConversionSettings, InspectedSource, JobPhase,
        JobProgressEvent, JobStatus, MediaKind, OutputFormat, OutputSummary,
    },
    engine::EngineRuntime,
    output::{OutputReservations, ReservedOutput},
    planner::{ConversionPlan, ConversionProgram, ConversionStage, plan_conversion},
    probe,
};

const PROCESS_EXIT_TIMEOUT: Duration = Duration::from_secs(3);
const STDERR_LIMIT: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct JobManager {
    inner: Arc<JobManagerInner>,
}

#[derive(Debug)]
struct JobManagerInner {
    jobs: Mutex<HashMap<String, JobRecord>>,
    outputs: OutputReservations,
    image_slots: Arc<Semaphore>,
    video_slots: Arc<Semaphore>,
}

#[derive(Debug, Clone)]
struct JobRecord {
    status: JobStatus,
    cancellation: CancellationToken,
    output_path: Option<PathBuf>,
}

impl JobManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(JobManagerInner {
                jobs: Mutex::new(HashMap::new()),
                outputs: OutputReservations::default(),
                image_slots: Arc::new(Semaphore::new(3)),
                video_slots: Arc::new(Semaphore::new(1)),
            }),
        }
    }

    pub async fn start(
        &self,
        app: AppHandle,
        engine: Arc<EngineRuntime>,
        source: Arc<InspectedSource>,
        settings: ConversionSettings,
    ) -> Result<(), ConversionError> {
        let job_id = source.media.id.clone();
        let cancellation = self.begin(&job_id)?;
        let output_format = selected_output_format(&source, &settings)?;
        let output = match self.inner.outputs.reserve(
            &source.path,
            output_format,
            settings.destination,
            settings.destination_path.as_deref(),
            settings.collision_policy,
        ) {
            Ok(output) => output,
            Err(error) => {
                self.remove_record(&job_id);
                return Err(error);
            }
        };
        let plan = match plan_conversion(&engine, &source, &settings, &output) {
            Ok(plan) => plan,
            Err(error) => {
                output.cleanup_partial();
                self.remove_record(&job_id);
                return Err(error);
            }
        };
        if let Err(error) =
            output.validate_free_space(plausible_space_requirement(&source, &settings))
        {
            plan.cleanup_temporary_paths();
            output.cleanup_partial();
            self.remove_record(&job_id);
            return Err(error);
        }

        emit(
            &app,
            JobProgressEvent::state(&job_id, JobStatus::Queued, JobPhase::Waiting),
        );
        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            manager
                .run(app, engine, source, plan, output, cancellation)
                .await;
        });
        Ok(())
    }

    pub fn cancel(&self, job_id: &str) -> Result<(), ConversionError> {
        let jobs = self.inner.jobs.lock().map_err(job_store_error)?;
        let record = jobs.get(job_id).ok_or_else(|| {
            ConversionError::new(
                ConversionErrorCode::InvalidRequest,
                "This conversion is not running",
                "The job may already have finished or been removed.",
            )
        })?;
        if matches!(
            record.status,
            JobStatus::Queued | JobStatus::Running | JobStatus::Probing
        ) {
            record.cancellation.cancel();
        }
        Ok(())
    }

    pub fn output_path(&self, job_id: &str) -> Result<PathBuf, ConversionError> {
        let jobs = self.inner.jobs.lock().map_err(job_store_error)?;
        jobs.get(job_id)
            .and_then(|record| record.output_path.clone())
            .ok_or_else(|| {
                ConversionError::new(
                    ConversionErrorCode::InvalidRequest,
                    "No completed output is available",
                    "Convert this file successfully before revealing it.",
                )
            })
    }

    pub fn forget(&self, job_id: &str) -> Result<(), ConversionError> {
        let mut jobs = self.inner.jobs.lock().map_err(job_store_error)?;
        if let Some(record) = jobs.get(job_id)
            && matches!(
                record.status,
                JobStatus::Queued | JobStatus::Running | JobStatus::Probing
            )
        {
            return Err(ConversionError::invalid(
                "Cancel this conversion before removing it from the session.",
            ));
        }
        jobs.remove(job_id);
        Ok(())
    }

    pub fn clear(&self) -> Result<(), ConversionError> {
        let mut jobs = self.inner.jobs.lock().map_err(job_store_error)?;
        if jobs.values().any(|record| {
            matches!(
                record.status,
                JobStatus::Queued | JobStatus::Running | JobStatus::Probing
            )
        }) {
            return Err(ConversionError::invalid(
                "Cancel active conversions before clearing the session.",
            ));
        }
        jobs.clear();
        Ok(())
    }

    async fn run(
        &self,
        app: AppHandle,
        engine: Arc<EngineRuntime>,
        source: Arc<InspectedSource>,
        plan: ConversionPlan,
        output: ReservedOutput,
        cancellation: CancellationToken,
    ) {
        let job_id = source.media.id.clone();
        let permit = match self.acquire_slot(source.media.kind, &cancellation).await {
            Ok(Some(permit)) => permit,
            Ok(None) => {
                plan.cleanup_temporary_paths();
                output.cleanup_partial();
                self.finish_canceled(&app, &job_id);
                return;
            }
            Err(error) => {
                plan.cleanup_temporary_paths();
                output.cleanup_partial();
                self.finish_failed(&app, &job_id, error);
                return;
            }
        };

        self.set_status(&job_id, JobStatus::Running);
        let result = execute_plan(
            &app,
            &job_id,
            &engine,
            &source,
            &plan,
            &output,
            &cancellation,
        )
        .await;
        drop(permit);
        plan.cleanup_temporary_paths();

        match result {
            Ok(ExecutionResult::Canceled) => {
                output.cleanup_partial();
                self.finish_canceled(&app, &job_id);
            }
            Ok(ExecutionResult::Completed) => {
                emit(
                    &app,
                    JobProgressEvent::state(&job_id, JobStatus::Running, JobPhase::Finalizing),
                );
                match validate_and_publish(&engine, &plan, &output).await {
                    Ok(summary) => {
                        self.finish_succeeded(&app, &job_id, &output, summary);
                    }
                    Err(error) => {
                        output.cleanup_partial();
                        self.finish_failed(&app, &job_id, error);
                    }
                }
            }
            Err(error) => {
                output.cleanup_partial();
                self.finish_failed(&app, &job_id, error);
            }
        }
    }

    fn begin(&self, job_id: &str) -> Result<CancellationToken, ConversionError> {
        let mut jobs = self.inner.jobs.lock().map_err(job_store_error)?;
        if jobs.get(job_id).is_some_and(|record| {
            matches!(
                record.status,
                JobStatus::Queued | JobStatus::Running | JobStatus::Probing
            )
        }) {
            return Err(ConversionError::invalid(
                "This conversion is already queued or running.",
            ));
        }
        let cancellation = CancellationToken::new();
        jobs.insert(
            job_id.to_owned(),
            JobRecord {
                status: JobStatus::Queued,
                cancellation: cancellation.clone(),
                output_path: None,
            },
        );
        Ok(cancellation)
    }

    async fn acquire_slot(
        &self,
        kind: MediaKind,
        cancellation: &CancellationToken,
    ) -> Result<Option<OwnedSemaphorePermit>, ConversionError> {
        let semaphore = match kind {
            MediaKind::Image => self.inner.image_slots.clone(),
            MediaKind::Video => self.inner.video_slots.clone(),
            MediaKind::Unsupported => {
                return Err(ConversionError::new(
                    ConversionErrorCode::UnsupportedFormat,
                    "This file is not supported",
                    "Choose a supported image or video file.",
                ));
            }
        };
        tokio::select! {
            permit = semaphore.acquire_owned() => permit.map(Some).map_err(|_| {
                ConversionError::new(
                    ConversionErrorCode::EngineFailed,
                    "The conversion queue stopped",
                    "Restart Morflo and try again.",
                )
            }),
            () = cancellation.cancelled() => Ok(None),
        }
    }

    fn set_status(&self, job_id: &str, status: JobStatus) {
        if let Ok(mut jobs) = self.inner.jobs.lock()
            && let Some(record) = jobs.get_mut(job_id)
        {
            record.status = status;
        }
    }

    fn finish_succeeded(
        &self,
        app: &AppHandle,
        job_id: &str,
        output: &ReservedOutput,
        summary: OutputSummary,
    ) {
        if let Ok(mut jobs) = self.inner.jobs.lock()
            && let Some(record) = jobs.get_mut(job_id)
        {
            record.status = JobStatus::Succeeded;
            record.output_path = Some(output.final_path.clone());
        }
        let mut event = JobProgressEvent::state(job_id, JobStatus::Succeeded, JobPhase::Converted);
        event.progress = Some(1.0);
        event.output = Some(summary);
        emit(app, event);
    }

    fn finish_failed(&self, app: &AppHandle, job_id: &str, error: ConversionError) {
        self.set_status(job_id, JobStatus::Failed);
        let mut event =
            JobProgressEvent::state(job_id, JobStatus::Failed, JobPhase::NeedsAttention);
        event.error = Some(error);
        emit(app, event);
    }

    fn finish_canceled(&self, app: &AppHandle, job_id: &str) {
        self.set_status(job_id, JobStatus::Canceled);
        let mut event = JobProgressEvent::state(job_id, JobStatus::Canceled, JobPhase::Canceled);
        event.error = Some(ConversionError::new(
            ConversionErrorCode::Canceled,
            "Conversion canceled",
            "The source stayed untouched and the temporary output was removed.",
        ));
        emit(app, event);
    }

    fn remove_record(&self, job_id: &str) {
        if let Ok(mut jobs) = self.inner.jobs.lock() {
            jobs.remove(job_id);
        }
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}

enum ExecutionResult {
    Completed,
    Canceled,
}

struct ExecutionContext<'a> {
    app: &'a AppHandle,
    job_id: &'a str,
    engine: &'a EngineRuntime,
    source: &'a InspectedSource,
    plan: &'a ConversionPlan,
    output: &'a ReservedOutput,
    cancellation: &'a CancellationToken,
}

async fn execute_plan(
    app: &AppHandle,
    job_id: &str,
    engine: &EngineRuntime,
    source: &InspectedSource,
    plan: &ConversionPlan,
    output: &ReservedOutput,
    cancellation: &CancellationToken,
) -> Result<ExecutionResult, ConversionError> {
    let stages = match &plan.program {
        ConversionProgram::Native(task) => {
            return execute_native(app, job_id, task, cancellation).await;
        }
        ConversionProgram::Engine(stages) => stages,
    };
    if stages.is_empty() {
        return Err(ConversionError::new(
            ConversionErrorCode::EngineFailed,
            "Conversion plan is empty",
            "Restart Morflo and try this conversion again.",
        ));
    }
    let context = ExecutionContext {
        app,
        job_id,
        engine,
        source,
        plan,
        output,
        cancellation,
    };
    for stage in stages {
        emit(
            app,
            JobProgressEvent::state(job_id, JobStatus::Running, stage.phase),
        );
        match execute_stage(&context, stage).await? {
            ExecutionResult::Completed => {}
            ExecutionResult::Canceled => return Ok(ExecutionResult::Canceled),
        }
    }
    Ok(ExecutionResult::Completed)
}

/// Run one conversion on Morflo's built-in image engine.
///
/// There is no child process to terminate here, and no progress channel to
/// read: a single image encodes as one bounded unit of work. Cancellation is
/// therefore observed around that unit rather than inside it. The in-flight
/// encode is always awaited before returning, because abandoning it would let a
/// background thread keep writing to a partial file the caller is about to
/// delete.
async fn execute_native(
    app: &AppHandle,
    job_id: &str,
    task: &crate::image_engine::NativeImageTask,
    cancellation: &CancellationToken,
) -> Result<ExecutionResult, ConversionError> {
    if cancellation.is_cancelled() {
        return Ok(ExecutionResult::Canceled);
    }
    emit(
        app,
        JobProgressEvent::state(job_id, JobStatus::Running, JobPhase::Encoding),
    );

    let owned = task.clone();
    let outcome = tokio::task::spawn_blocking(move || crate::image_engine::convert(&owned))
        .await
        .map_err(|error| {
            ConversionError::new(
                ConversionErrorCode::EngineFailed,
                "Conversion process was interrupted",
                "Morflo's built-in image engine stopped unexpectedly.",
            )
            .with_details(error.to_string())
        })?;

    if cancellation.is_cancelled() {
        return Ok(ExecutionResult::Canceled);
    }
    outcome?;

    let mut event = JobProgressEvent::state(job_id, JobStatus::Running, JobPhase::Encoding);
    event.progress = Some(1.0);
    emit(app, event);
    Ok(ExecutionResult::Completed)
}

async fn execute_stage(
    context: &ExecutionContext<'_>,
    stage: &ConversionStage,
) -> Result<ExecutionResult, ConversionError> {
    let mut command = crate::engine_process::engine_command(&context.engine.media()?.ffmpeg);
    command
        .args(&stage.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    configure_process_group(&mut command);
    let mut child = command.spawn().map_err(|error| {
        ConversionError::new(
            ConversionErrorCode::EngineFailed,
            "Conversion could not start",
            "The local media engine could not start this job.",
        )
        .with_details(error.to_string())
    })?;
    let process_id = child.id();
    let stdout = child.stdout.take().ok_or_else(|| {
        ConversionError::new(
            ConversionErrorCode::EngineFailed,
            "Conversion progress is unavailable",
            "The local engine did not provide its progress channel.",
        )
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        ConversionError::new(
            ConversionErrorCode::EngineFailed,
            "Conversion diagnostics are unavailable",
            "The local engine did not provide its diagnostic channel.",
        )
    })?;

    let progress_task = tokio::spawn(read_progress(
        stdout,
        context.app.clone(),
        context.job_id.to_owned(),
        stage.phase,
        stage.duration_seconds,
        stage.progress_start,
        stage.progress_span,
    ));
    let stderr_task = tokio::spawn(read_limited(stderr, STDERR_LIMIT));

    let (was_canceled, status) = tokio::select! {
        result = child.wait() => (false, result),
        () = context.cancellation.cancelled() => {
            terminate_process_tree(process_id, false).await;
            let status = match timeout(PROCESS_EXIT_TIMEOUT, child.wait()).await {
                Ok(result) => result,
                Err(_) => {
                    terminate_process_tree(process_id, true).await;
                    let _ = child.kill().await;
                    child.wait().await
                }
            };
            (true, status)
        }
    };
    let status = status.map_err(|error| {
        ConversionError::new(
            ConversionErrorCode::EngineFailed,
            "Conversion process was interrupted",
            "The local media engine stopped unexpectedly.",
        )
        .with_details(error.to_string())
    })?;
    let _ = progress_task.await;
    let stderr = stderr_task.await.unwrap_or_default();

    if was_canceled {
        return Ok(ExecutionResult::Canceled);
    }
    if !status.success() {
        return Err(map_engine_failure(
            &stderr,
            &context.source.path,
            &context.output.partial_path,
            &context.plan.temporary_paths,
        ));
    }
    Ok(ExecutionResult::Completed)
}

async fn validate_and_publish(
    engine: &EngineRuntime,
    plan: &ConversionPlan,
    output: &ReservedOutput,
) -> Result<OutputSummary, ConversionError> {
    let inspected = probe::inspect_path(engine, &output.partial_path).await?;
    let expected_kind = match plan.output_format {
        OutputFormat::Mp4 | OutputFormat::Webm => MediaKind::Video,
        OutputFormat::Png
        | OutputFormat::Jpeg
        | OutputFormat::Webp
        | OutputFormat::Avif
        | OutputFormat::Ico
        | OutputFormat::Gif => MediaKind::Image,
    };
    if inspected.media.kind != expected_kind
        || !matches_expected_format(&inspected, plan.output_format)
    {
        return Err(ConversionError::new(
            ConversionErrorCode::EngineFailed,
            "The converted output could not be verified",
            "Morflo removed the incomplete temporary result and did not publish it.",
        )
        .with_details(format!(
            "Expected {:?}; detected container '{}' with codec '{}'.",
            plan.output_format,
            inspected.format_name,
            inspected.primary_codec.as_deref().unwrap_or("unknown")
        )));
    }
    let summary = OutputSummary {
        name: output.display_name(),
        format: plan.output_format,
        size_bytes: inspected.media.size_bytes,
        has_alpha: inspected.media.has_alpha,
        width: inspected.media.width,
        height: inspected.media.height,
        duration_seconds: inspected.media.duration_seconds,
    };
    output.finalize()?;
    Ok(summary)
}

/// Confirm a published file really is the format that was asked for.
///
/// Morflo has two inspectors and they name containers differently: FFprobe
/// reports demuxer names such as `png_pipe`, while the built-in image engine
/// reports the decoded container directly as `png`. Both vocabularies are
/// accepted here rather than making one engine imitate the other's labels,
/// which would hide which inspector actually produced the evidence.
fn matches_expected_format(source: &InspectedSource, format: OutputFormat) -> bool {
    let names = source.format_name.split(',').collect::<Vec<_>>();
    let codec = source.primary_codec.as_deref();
    match format {
        OutputFormat::Png => {
            names.iter().any(|name| matches!(*name, "png_pipe" | "png")) && codec == Some("png")
        }
        OutputFormat::Jpeg => {
            names
                .iter()
                .any(|name| matches!(*name, "image2" | "jpeg_pipe" | "jpeg"))
                && codec == Some("mjpeg")
        }
        OutputFormat::Webp => names.contains(&"webp_pipe") && codec == Some("webp"),
        OutputFormat::Avif => {
            (names.contains(&"avif") || names.contains(&"mov") || names.contains(&"mp4"))
                && codec == Some("av1")
        }
        OutputFormat::Ico => names.contains(&"ico") && matches!(codec, Some("png" | "bmp" | "ico")),
        OutputFormat::Mp4 => {
            names.iter().any(|name| matches!(*name, "mov" | "mp4")) && codec == Some("h264")
        }
        OutputFormat::Webm => {
            names
                .iter()
                .any(|name| matches!(*name, "matroska" | "webm"))
                && codec == Some("vp9")
        }
        OutputFormat::Gif => names.contains(&"gif") && codec == Some("gif"),
    }
}

async fn read_progress(
    stdout: impl AsyncRead + Unpin,
    app: AppHandle,
    job_id: String,
    phase: JobPhase,
    duration_seconds: Option<f64>,
    progress_start: f64,
    progress_span: f64,
) {
    let mut lines = BufReader::new(stdout).lines();
    let mut parser = crate::progress::ProgressParser::new(duration_seconds);
    while let Ok(Some(line)) = lines.next_line().await {
        if let Some(progress) = parser.push_line(&line) {
            let mut event = JobProgressEvent::state(&job_id, JobStatus::Running, phase);
            event.progress = Some((progress_start + progress * progress_span).clamp(0.0, 1.0));
            emit(&app, event);
        }
    }
}

async fn read_limited(mut input: impl AsyncRead + Unpin, limit: usize) -> Vec<u8> {
    let mut retained = Vec::new();
    let mut chunk = [0_u8; 4_096];
    loop {
        match input.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                retained.extend_from_slice(&chunk[..read]);
                if retained.len() > limit {
                    retained.drain(..retained.len() - limit);
                }
            }
        }
    }
    retained
}

fn selected_output_format(
    source: &InspectedSource,
    settings: &ConversionSettings,
) -> Result<OutputFormat, ConversionError> {
    match source.media.kind {
        MediaKind::Image => Ok(settings.image.output_format),
        MediaKind::Video => Ok(settings.video.output_format),
        MediaKind::Unsupported => Err(ConversionError::new(
            ConversionErrorCode::UnsupportedFormat,
            "This file is not supported",
            "Choose a supported image or video file.",
        )),
    }
}

fn plausible_space_requirement(source: &InspectedSource, settings: &ConversionSettings) -> u64 {
    const MIB: u64 = 1024 * 1024;
    let source_allowance = source.media.size_bytes.saturating_mul(2);
    match source.media.kind {
        MediaKind::Image => {
            let decoded = u64::from(source.media.width.unwrap_or_default())
                .saturating_mul(u64::from(source.media.height.unwrap_or_default()))
                .saturating_mul(4);
            32_u64
                .saturating_mul(MIB)
                .saturating_add(source_allowance.max(decoded))
        }
        MediaKind::Video if settings.video.output_format == OutputFormat::Gif => {
            let source_width = u64::from(source.media.width.unwrap_or(settings.gif.width));
            let source_height = u64::from(source.media.height.unwrap_or(settings.gif.width));
            let width = u64::from(settings.gif.width).min(source_width.max(1));
            let height = width
                .saturating_mul(source_height)
                .checked_div(source_width.max(1))
                .unwrap_or(width)
                .max(1);
            let frames = ((settings.gif.end_seconds - settings.gif.start_seconds).max(0.0)
                * f64::from(settings.gif.fps))
            .ceil()
            .clamp(0.0, u32::MAX as f64) as u64;
            let indexed_frames = width.saturating_mul(height).saturating_mul(frames);
            64_u64
                .saturating_mul(MIB)
                .saturating_add(source_allowance.max(indexed_frames))
        }
        MediaKind::Video => 128_u64.saturating_mul(MIB).saturating_add(source_allowance),
        MediaKind::Unsupported => 32_u64.saturating_mul(MIB),
    }
}

fn map_engine_failure(
    stderr: &[u8],
    source: &Path,
    partial: &Path,
    temporary_paths: &[PathBuf],
) -> ConversionError {
    let raw = String::from_utf8_lossy(stderr);
    let lower = raw.to_ascii_lowercase();
    let (code, title, message) = if lower.contains("no space left on device") {
        (
            ConversionErrorCode::InsufficientSpace,
            "Not enough free space",
            "Free some space or choose another output folder, then try again.",
        )
    } else if lower.contains("permission denied") || lower.contains("access is denied") {
        (
            ConversionErrorCode::PermissionDenied,
            "The output could not be written",
            "Check the destination permissions or choose another folder.",
        )
    } else if lower.contains("unknown encoder") || lower.contains("encoder not found") {
        (
            ConversionErrorCode::UnsupportedCodec,
            "The required codec is unavailable",
            "The detected local media engine cannot create this result.",
        )
    } else if lower.contains("invalid data") || lower.contains("corrupt") {
        (
            ConversionErrorCode::DamagedInput,
            "The source could not be decoded",
            "The file appears incomplete or damaged.",
        )
    } else {
        (
            ConversionErrorCode::EngineFailed,
            "Conversion did not finish",
            "The local media engine stopped before a valid output was ready.",
        )
    };
    let source_name = source
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "source".to_owned());
    let mut details = raw.replace(
        source.to_string_lossy().as_ref(),
        &format!("<source>/{source_name}"),
    );
    details = details.replace(partial.to_string_lossy().as_ref(), "<temporary-output>");
    for temporary in temporary_paths {
        details = details.replace(temporary.to_string_lossy().as_ref(), "<temporary-output>");
    }
    ConversionError::new(code, title, message)
        .with_details(details.chars().take(8_000).collect::<String>())
}

fn job_store_error<T>(_: std::sync::PoisonError<T>) -> ConversionError {
    ConversionError::new(
        ConversionErrorCode::EngineFailed,
        "The local job queue is unavailable",
        "Restart Morflo and try the conversion again.",
    )
}

fn emit(app: &AppHandle, event: JobProgressEvent) {
    if app.emit("morflo://job-progress", event).is_err() {
        eprintln!("Morflo could not deliver a local job-state event");
    }
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    command.process_group(0);
}

#[cfg(windows)]
fn configure_process_group(command: &mut Command) {
    use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW};
    command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
}

#[cfg(unix)]
async fn terminate_process_tree(process_id: Option<u32>, force: bool) {
    let Some(process_id) = process_id.and_then(|value| i32::try_from(value).ok()) else {
        return;
    };
    let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
    // SAFETY: the child was launched as a new process group whose id is its positive process id.
    let _ = unsafe { libc::kill(-process_id, signal) };
}

#[cfg(windows)]
async fn terminate_process_tree(process_id: Option<u32>, _force: bool) {
    let Some(process_id) = process_id else {
        return;
    };
    let process_id = process_id.to_string();
    let mut command = Command::new("taskkill.exe");
    command
        .args(["/PID", &process_id, "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let _ = timeout(PROCESS_EXIT_TIMEOUT, command.status()).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AnimationPolicy, CollisionPolicy, DestinationMode, GifPreset, GifSettings, ImageSettings,
        JobPhase, JobStatus, LoopBehavior, MediaFile, MediaWarning, MetadataPolicy, Quality,
        ResizeMode, Resolution, VideoSettings,
    };

    fn inspected(format_name: &str, codec: &str) -> InspectedSource {
        InspectedSource {
            media: MediaFile {
                id: "job".to_owned(),
                name: "sample".to_owned(),
                extension: "tmp".to_owned(),
                size_bytes: 1,
                kind: MediaKind::Image,
                status: JobStatus::Ready,
                phase: JobPhase::Waiting,
                progress: None,
                width: Some(10),
                height: Some(10),
                duration_seconds: None,
                has_alpha: Some(false),
                animated: Some(false),
                video_codec: None,
                video_tracks: None,
                audio_tracks: None,
                subtitle_tracks: None,
                chapter_count: None,
                attachment_tracks: None,
                data_tracks: None,
                hdr: None,
                warnings: Vec::<MediaWarning>::new(),
                output: None,
                error: None,
            },
            path: PathBuf::from("sample.tmp"),
            format_name: format_name.to_owned(),
            primary_codec: Some(codec.to_owned()),
            primary_stream_index: 0,
            alpha_stream_index: None,
        }
    }

    #[test]
    fn validates_probe_format_against_requested_output() {
        assert!(matches_expected_format(
            &inspected("png_pipe", "png"),
            OutputFormat::Png
        ));
        assert!(!matches_expected_format(
            &inspected("png_pipe", "png"),
            OutputFormat::Jpeg
        ));
        assert!(matches_expected_format(
            &inspected("matroska,webm", "vp9"),
            OutputFormat::Webm
        ));
        assert!(matches_expected_format(
            &inspected("image2", "mjpeg"),
            OutputFormat::Jpeg
        ));
        assert!(!matches_expected_format(
            &inspected("image2", "png"),
            OutputFormat::Jpeg
        ));
    }

    #[test]
    fn serializes_verified_output_facts_for_the_frontend() {
        let mut event = JobProgressEvent::state("job", JobStatus::Succeeded, JobPhase::Converted);
        event.progress = Some(1.0);
        event.output = Some(OutputSummary {
            name: "夏祭り result.webp".to_owned(),
            format: OutputFormat::Webp,
            size_bytes: 42_108,
            has_alpha: Some(true),
            width: Some(640),
            height: Some(480),
            duration_seconds: None,
        });

        let value = serde_json::to_value(event).expect("serialize output event");
        assert_eq!(value["output"]["name"], "夏祭り result.webp");
        assert_eq!(value["output"]["format"], "webp");
        assert_eq!(value["output"]["sizeBytes"], 42_108);
        assert_eq!(value["output"]["hasAlpha"], true);
        assert_eq!(value["output"]["width"], 640);
        assert_eq!(value["output"]["height"], 480);
        assert!(value.get("outputPath").is_none());
    }

    #[test]
    fn free_space_allowance_accounts_for_decoded_images_and_gif_frames() {
        let mut source = inspected("png_pipe", "png");
        source.media.size_bytes = 4 * 1024 * 1024;
        source.media.width = Some(4_000);
        source.media.height = Some(3_000);
        let mut settings = ConversionSettings {
            image: ImageSettings {
                output_format: OutputFormat::Png,
                quality: Quality::Balanced,
                resize_mode: ResizeMode::Original,
                width: None,
                height: None,
                percentage: None,
                background: "#F5F1E8".to_owned(),
                metadata: MetadataPolicy::Remove,
                animation: AnimationPolicy::Ask,
            },
            video: VideoSettings {
                output_format: OutputFormat::Mp4,
                resolution: Resolution::Original,
                quality: Quality::Balanced,
                metadata: MetadataPolicy::Preserve,
            },
            gif: GifSettings {
                preset: GifPreset::High,
                start_seconds: 0.0,
                end_seconds: 15.0,
                width: 720,
                fps: 18,
                quality: Quality::Best,
                r#loop: LoopBehavior::Forever,
            },
            destination: DestinationMode::Same,
            destination_path: None,
            collision_policy: CollisionPolicy::Suffix,
        };

        assert_eq!(
            plausible_space_requirement(&source, &settings),
            32 * 1024 * 1024 + 4_000 * 3_000 * 4
        );

        source.media.kind = MediaKind::Video;
        source.media.width = Some(1_920);
        source.media.height = Some(1_080);
        settings.video.output_format = OutputFormat::Gif;
        assert!(plausible_space_requirement(&source, &settings) > 128 * 1024 * 1024);
    }

    #[cfg(feature = "real-engine")]
    #[tokio::test]
    #[ignore = "requires a compatible real FFmpeg/ffprobe pair"]
    async fn real_process_tree_cancellation_stops_the_encoder_promptly() {
        let engine = crate::engine::EngineService::new()
            .runtime()
            .await
            .expect("discover compatible real engine");
        let tools = engine
            .media()
            .expect("a real engine run requires the media tools");
        let mut command = crate::engine_process::engine_command(&tools.ffmpeg);
        command
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-re",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=s=1280x720:r=30:d=120",
                "-c:v",
                "libvpx-vp9",
                "-f",
                "null",
                "-",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        configure_process_group(&mut command);
        let mut child = command.spawn().expect("spawn cancellable real encoder");
        let process_id = child.id();
        tokio::time::sleep(Duration::from_millis(350)).await;

        let started = std::time::Instant::now();
        terminate_process_tree(process_id, false).await;
        let status = match timeout(Duration::from_secs(2), child.wait()).await {
            Ok(result) => result.expect("wait for canceled encoder"),
            Err(_) => {
                terminate_process_tree(process_id, true).await;
                let _ = child.kill().await;
                panic!("real encoder did not stop within two seconds");
            }
        };
        assert!(!status.success());
        let elapsed = started.elapsed();
        eprintln!("cancellation_ms={}", elapsed.as_millis());
        assert!(elapsed < Duration::from_secs(2));
    }
}
