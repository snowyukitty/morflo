use std::{path::PathBuf, process::Stdio, sync::Arc};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

#[cfg(windows)]
use std::ffi::OsString;

use tauri::{AppHandle, State};
use tokio::{process::Command, sync::Semaphore, task::JoinSet};

use crate::{
    domain::{
        CapabilityRegistry, ConversionError, ConversionErrorCode, MediaFile, MediaKind,
        PreviewAsset, StartJobRequest, StartupReport, StoryboardAsset,
    },
    preview,
    probe::{failed_media_file, inspect_path},
    state::AppState,
};

const MAX_FILES_PER_ADD: usize = 512;
const PROBE_CONCURRENCY: usize = 4;
const MAX_GIF_PREVIEW_BYTES: u64 = 20 * 1024 * 1024;

#[tauri::command]
pub fn runtime_ready() -> bool {
    true
}

#[tauri::command]
pub fn startup_report(state: State<'_, AppState>) -> StartupReport {
    state.startup_report
}

#[tauri::command]
pub fn pending_files(state: State<'_, AppState>) -> Result<Vec<String>, ConversionError> {
    state.take_pending_files()
}

#[tauri::command]
pub async fn get_capabilities(state: State<'_, AppState>) -> Result<CapabilityRegistry, ()> {
    Ok(state.engine.capabilities().await)
}

#[tauri::command]
pub async fn refresh_capabilities(state: State<'_, AppState>) -> Result<CapabilityRegistry, ()> {
    Ok(state.engine.refresh().await)
}

#[tauri::command]
pub async fn select_engine_directory(
    state: State<'_, AppState>,
    directory: String,
) -> Result<CapabilityRegistry, ConversionError> {
    state
        .engine
        .select_directory(PathBuf::from(directory))
        .await
}

#[tauri::command]
pub async fn inspect_files(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Vec<MediaFile>, ConversionError> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    if paths.len() > MAX_FILES_PER_ADD {
        return Err(ConversionError::invalid(format!(
            "Add no more than {MAX_FILES_PER_ADD} files at once."
        )));
    }
    let engine = state.engine.runtime().await?;
    let semaphore = Arc::new(Semaphore::new(PROBE_CONCURRENCY));
    let mut tasks = JoinSet::new();
    let count = paths.len();

    for (index, raw_path) in paths.into_iter().enumerate() {
        let engine = engine.clone();
        let semaphore = semaphore.clone();
        let path = PathBuf::from(raw_path);
        tasks.spawn(async move {
            let result = match semaphore.acquire_owned().await {
                Ok(_permit) => inspect_path(&engine, &path).await,
                Err(_) => Err(ConversionError::new(
                    ConversionErrorCode::EngineFailed,
                    "File inspection stopped",
                    "Restart Morflo and add the files again.",
                )),
            };
            (index, path, result)
        });
    }

    let mut ordered = (0..count).map(|_| None).collect::<Vec<
        Option<(
            PathBuf,
            Result<crate::domain::InspectedSource, ConversionError>,
        )>,
    >>();
    while let Some(result) = tasks.join_next().await {
        let (index, path, inspection) = result.map_err(|error| {
            ConversionError::new(
                ConversionErrorCode::EngineFailed,
                "File inspection stopped",
                "A local inspection task ended unexpectedly.",
            )
            .with_details(error.to_string())
        })?;
        if let Some(slot) = ordered.get_mut(index) {
            *slot = Some((path, inspection));
        }
    }

    let mut files = Vec::with_capacity(count);
    for item in ordered.into_iter().flatten() {
        let (path, inspection) = item;
        match inspection {
            Ok(source) => {
                let media = source.media.clone();
                state.sources.insert(source)?;
                files.push(media);
            }
            Err(error) => {
                let media = failed_media_file(&path, error);
                state.sources.insert_failed(media.id.clone(), &path)?;
                files.push(media);
            }
        }
    }
    Ok(files)
}

#[tauri::command]
pub async fn retry_inspection(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<MediaFile, ConversionError> {
    let path = state.sources.failed_path(&job_id)?;
    let engine = state.engine.runtime().await?;
    match inspect_path(&engine, &path).await {
        Ok(mut source) => {
            source.media.id.clone_from(&job_id);
            let media = source.media.clone();
            state.sources.insert(source)?;
            Ok(media)
        }
        Err(error) => {
            let mut media = failed_media_file(&path, error);
            media.id = job_id;
            Ok(media)
        }
    }
}

#[tauri::command]
pub async fn start_job(
    app: AppHandle,
    state: State<'_, AppState>,
    request: StartJobRequest,
) -> Result<(), ConversionError> {
    let source = state.sources.get(&request.job_id)?;
    let engine = state.engine.runtime().await?;
    state
        .jobs
        .start(app, engine, source, request.settings)
        .await
}

#[tauri::command]
pub fn cancel_job(state: State<'_, AppState>, job_id: String) -> Result<(), ConversionError> {
    state.jobs.cancel(&job_id)
}

#[tauri::command]
pub async fn reveal_output(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<(), ConversionError> {
    let path = state.jobs.output_path(&job_id)?;
    reveal_path(path).await
}

#[tauri::command]
pub fn forget_source(state: State<'_, AppState>, job_id: String) -> Result<(), ConversionError> {
    state.jobs.forget(&job_id)?;
    state.sources.remove(&job_id)
}

#[tauri::command]
pub fn clear_session(state: State<'_, AppState>) -> Result<(), ConversionError> {
    state.jobs.clear()?;
    state.sources.clear()
}

#[tauri::command]
pub async fn poster_frame(
    state: State<'_, AppState>,
    job_id: String,
    at_seconds: f64,
) -> Result<PreviewAsset, ConversionError> {
    let source = state.sources.get(&job_id)?;
    if source.media.kind != MediaKind::Video {
        return Err(ConversionError::invalid(
            "Poster frames are available only for video sources.",
        ));
    }
    let duration = source.media.duration_seconds.ok_or_else(|| {
        ConversionError::invalid("This video's duration is unavailable for preview.")
    })?;
    if !at_seconds.is_finite() || at_seconds < 0.0 || at_seconds > duration {
        return Err(ConversionError::invalid(
            "Choose a preview time inside the source video.",
        ));
    }
    let engine = state.engine.runtime().await?;
    preview::render_poster(&engine, &source.path, at_seconds).await
}

#[tauri::command]
pub async fn video_storyboard(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<StoryboardAsset, ConversionError> {
    let source = state.sources.get(&job_id)?;
    if source.media.kind != MediaKind::Video {
        return Err(ConversionError::invalid(
            "Moment strips are available only for video sources.",
        ));
    }
    let duration = source.media.duration_seconds.ok_or_else(|| {
        ConversionError::invalid("This video's duration is unavailable for a moment strip.")
    })?;
    if !duration.is_finite() || duration <= 0.0 {
        return Err(ConversionError::invalid(
            "This video does not have a usable duration for a moment strip.",
        ));
    }
    let engine = state.engine.runtime().await?;
    preview::render_storyboard(&engine, &source.path, duration).await
}

#[tauri::command]
pub async fn image_thumbnail(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<PreviewAsset, ConversionError> {
    let source = state.sources.get(&job_id)?;
    if source.media.kind != MediaKind::Image {
        return Err(ConversionError::invalid(
            "Image thumbnails are available only for image sources.",
        ));
    }
    let engine = state.engine.runtime().await?;
    preview::render_thumbnail(&engine, &source.path).await
}

#[tauri::command]
pub async fn preview_clip(
    state: State<'_, AppState>,
    job_id: String,
    start_seconds: f64,
    end_seconds: f64,
) -> Result<PreviewAsset, ConversionError> {
    let source = state.sources.get(&job_id)?;
    let duration = source.media.duration_seconds.ok_or_else(|| {
        ConversionError::invalid("This video's duration is unavailable for preview.")
    })?;
    if source.media.kind != MediaKind::Video
        || !start_seconds.is_finite()
        || !end_seconds.is_finite()
        || start_seconds < 0.0
        || end_seconds <= start_seconds
        || end_seconds > duration + 0.05
    {
        return Err(ConversionError::invalid(
            "Choose a preview range inside the source video.",
        ));
    }
    let engine = state.engine.runtime().await?;
    preview::render_clip(&engine, &source.path, start_seconds, end_seconds).await
}

#[tauri::command]
pub async fn output_preview(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<PreviewAsset, ConversionError> {
    let path = state.jobs.output_path(&job_id)?;
    let engine = state.engine.runtime().await?;
    let inspected = inspect_path(&engine, &path).await?;
    let is_gif = inspected.format_name.split(',').any(|name| name == "gif")
        && inspected.primary_codec.as_deref() == Some("gif");
    if !is_gif {
        return match inspected.media.kind {
            MediaKind::Image => preview::render_output_thumbnail(&engine, &path).await,
            MediaKind::Video => {
                let at_seconds = inspected
                    .media
                    .duration_seconds
                    .filter(|duration| duration.is_finite() && *duration > 0.0)
                    .map_or(0.0, |duration| (duration * 0.15).min(1.0));
                preview::render_output_poster(&engine, &path, at_seconds).await
            }
            MediaKind::Unsupported => Err(ConversionError::invalid(
                "A preview is unavailable for this completed output.",
            )),
        };
    }
    let metadata = std::fs::symlink_metadata(&path).map_err(|error| {
        ConversionError::new(
            ConversionErrorCode::InvalidRequest,
            "The output preview is no longer available",
            "The output may have been moved or deleted.",
        )
        .with_details(error.to_string())
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(ConversionError::invalid(
            "Morflo will preview only the output file it created.",
        ));
    }
    if metadata.len() > MAX_GIF_PREVIEW_BYTES {
        return Err(ConversionError::new(
            ConversionErrorCode::InvalidRequest,
            "This animated output is too large for an in-app preview",
            "The completed file is safe. Reveal it in your folder to open it normally.",
        ));
    }
    let bytes = tokio::task::spawn_blocking(move || std::fs::read(path))
        .await
        .map_err(|error| {
            ConversionError::new(
                ConversionErrorCode::EngineFailed,
                "The output preview could not be loaded",
                "Reveal the completed file to open it normally.",
            )
            .with_details(error.to_string())
        })?
        .map_err(|error| {
            ConversionError::new(
                ConversionErrorCode::PermissionDenied,
                "The output preview could not be loaded",
                "Reveal the completed file to open it normally.",
            )
            .with_details(error.to_string())
        })?;
    Ok(PreviewAsset {
        mime_type: "image/gif".to_owned(),
        data_base64: BASE64.encode(bytes),
    })
}

async fn reveal_path(path: PathBuf) -> Result<(), ConversionError> {
    let metadata = std::fs::symlink_metadata(&path).map_err(|error| {
        ConversionError::new(
            ConversionErrorCode::InvalidRequest,
            "The converted file is no longer available",
            "It may have been moved or deleted after conversion.",
        )
        .with_details(error.to_string())
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(ConversionError::new(
            ConversionErrorCode::InvalidRequest,
            "The converted file is no longer available",
            "Morflo will only reveal the output file it created.",
        ));
    }

    let mut command = reveal_command(&path)?;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| {
            ConversionError::new(
                ConversionErrorCode::EngineFailed,
                "The output folder could not be opened",
                "Open the destination folder manually from your file manager.",
            )
            .with_details(error.to_string())
        })?;
    Ok(())
}

#[cfg(windows)]
fn reveal_command(path: &std::path::Path) -> Result<Command, ConversionError> {
    let mut selection = OsString::from("/select,");
    selection.push(path.as_os_str());
    let mut command = Command::new("explorer.exe");
    command.arg(selection);
    Ok(command)
}

#[cfg(target_os = "macos")]
fn reveal_command(path: &std::path::Path) -> Result<Command, ConversionError> {
    let mut command = Command::new("open");
    command.arg("-R").arg(path);
    Ok(command)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn reveal_command(path: &std::path::Path) -> Result<Command, ConversionError> {
    let parent = path.parent().ok_or_else(|| {
        ConversionError::invalid("The converted file does not have a revealable parent folder.")
    })?;
    let mut command = Command::new("xdg-open");
    command.arg(parent);
    Ok(command)
}
