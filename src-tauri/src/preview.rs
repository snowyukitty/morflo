use std::{path::Path, process::ExitStatus, process::Stdio, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
    task::JoinSet,
    time::timeout,
};

use crate::{
    domain::{ConversionError, ConversionErrorCode, PreviewAsset, StoryboardAsset},
    engine::EngineRuntime,
};

const PREVIEW_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_THUMBNAIL_BYTES: usize = 2 * 1024 * 1024;
const MAX_POSTER_BYTES: usize = 2 * 1024 * 1024;
const MAX_STORYBOARD_FRAME_BYTES: usize = 256 * 1024;
const MAX_CLIP_BYTES: usize = 6 * 1024 * 1024;
const MAX_DIAGNOSTIC_BYTES: usize = 16 * 1024;
const STORYBOARD_FRAME_COUNT: usize = 7;
const STORYBOARD_CONCURRENCY: usize = 2;

struct PreviewOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

pub async fn render_thumbnail(
    engine: &EngineRuntime,
    source: &Path,
) -> Result<PreviewAsset, ConversionError> {
    render_thumbnail_with_copy(
        engine,
        source,
        "The image preview could not be created",
        "Conversion remains available without a thumbnail.",
    )
    .await
}

pub async fn render_output_thumbnail(
    engine: &EngineRuntime,
    source: &Path,
) -> Result<PreviewAsset, ConversionError> {
    render_thumbnail_with_copy(
        engine,
        source,
        "The output preview could not be created",
        "The completed file is safe and remains available through Reveal in folder.",
    )
    .await
}

async fn render_thumbnail_with_copy(
    engine: &EngineRuntime,
    source: &Path,
    title: &'static str,
    message: &'static str,
) -> Result<PreviewAsset, ConversionError> {
    // Without a media engine the built-in image engine renders the same bounded
    // preview box, so a completed conversion still shows its result instead of
    // degrading to a bare filename.
    if !engine.has_media_engine() {
        return native_thumbnail(source, title, message).await;
    }
    let mut command = crate::engine_process::engine_command(&engine.media()?.ffmpeg);
    command
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-i"])
        .arg(source)
        .args([
            "-map",
            "0:v:0",
            "-frames:v",
            "1",
            "-vf",
            "scale=w='min(iw,512)':h='min(ih,512)':force_original_aspect_ratio=decrease:flags=lanczos",
            "-c:v",
            "png",
            "-f",
            "image2pipe",
            "pipe:1",
        ]);
    let bytes = run_preview(command, source, MAX_THUMBNAIL_BYTES, title, message).await?;
    Ok(PreviewAsset {
        mime_type: "image/png".to_owned(),
        data_base64: BASE64.encode(bytes),
    })
}

/// Render a preview with Morflo's built-in image engine.
///
/// A preview is a convenience, never the result, so a failure here keeps the
/// caller's calm copy and leaves the converted file untouched.
async fn native_thumbnail(
    source: &Path,
    title: &'static str,
    message: &'static str,
) -> Result<PreviewAsset, ConversionError> {
    let path = source.to_path_buf();
    let bytes = tokio::task::spawn_blocking(move || {
        crate::image_engine::render_preview_png(&path, MAX_THUMBNAIL_BYTES)
    })
    .await
    .map_err(|error| {
        ConversionError::new(ConversionErrorCode::EngineFailed, title, message)
            .with_details(error.to_string())
    })?
    .map_err(|error| {
        ConversionError::new(ConversionErrorCode::EngineFailed, title, message)
            .with_details(error.to_string())
    })?;

    Ok(PreviewAsset {
        mime_type: "image/png".to_owned(),
        data_base64: BASE64.encode(bytes),
    })
}

pub async fn render_poster(
    engine: &EngineRuntime,
    source: &Path,
    at_seconds: f64,
) -> Result<PreviewAsset, ConversionError> {
    render_poster_with_copy(
        engine,
        source,
        at_seconds,
        "The video preview could not be created",
        "Conversion is still available without a poster frame.",
    )
    .await
}

pub async fn render_output_poster(
    engine: &EngineRuntime,
    source: &Path,
    at_seconds: f64,
) -> Result<PreviewAsset, ConversionError> {
    render_poster_with_copy(
        engine,
        source,
        at_seconds,
        "The output preview could not be created",
        "The completed file is safe and remains available through Reveal in folder.",
    )
    .await
}

async fn render_poster_with_copy(
    engine: &EngineRuntime,
    source: &Path,
    at_seconds: f64,
    title: &'static str,
    message: &'static str,
) -> Result<PreviewAsset, ConversionError> {
    let at = format!("{at_seconds:.3}");
    let mut command = crate::engine_process::engine_command(&engine.media()?.ffmpeg);
    command
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-ss",
            &at,
            "-i",
        ])
        .arg(source)
        .args([
            "-frames:v",
            "1",
            "-an",
            "-vf",
            "scale=w='min(iw,720)':h=-2:flags=lanczos",
            "-c:v",
            "mjpeg",
            "-q:v",
            "4",
            "-f",
            "image2pipe",
            "pipe:1",
        ]);
    let bytes = run_preview(command, source, MAX_POSTER_BYTES, title, message).await?;
    Ok(PreviewAsset {
        mime_type: "image/jpeg".to_owned(),
        data_base64: BASE64.encode(bytes),
    })
}

pub async fn render_storyboard(
    engine: &EngineRuntime,
    source: &Path,
    duration_seconds: f64,
) -> Result<StoryboardAsset, ConversionError> {
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Err(ConversionError::invalid(
            "A moment strip requires a video with a known duration.",
        ));
    }

    let last_sample = (duration_seconds - 0.05).max(0.0);
    let render = async {
        let mut frames = (0..STORYBOARD_FRAME_COUNT)
            .map(|_| None)
            .collect::<Vec<Option<PreviewAsset>>>();
        let mut tasks = JoinSet::new();
        let mut next_index = 0;
        while next_index < STORYBOARD_FRAME_COUNT && tasks.len() < STORYBOARD_CONCURRENCY {
            spawn_storyboard_frame(&mut tasks, engine, source, next_index, last_sample);
            next_index += 1;
        }
        while let Some(result) = tasks.join_next().await {
            let (index, frame) = result.map_err(|error| {
                ConversionError::new(
                    ConversionErrorCode::EngineFailed,
                    "The video moment strip could not be created",
                    "GIF conversion remains available without the moment strip.",
                )
                .with_details(error.to_string())
            })?;
            frames[index] = Some(frame?);
            if next_index < STORYBOARD_FRAME_COUNT {
                spawn_storyboard_frame(&mut tasks, engine, source, next_index, last_sample);
                next_index += 1;
            }
        }
        frames
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                ConversionError::new(
                    ConversionErrorCode::EngineFailed,
                    "The video moment strip could not be created",
                    "GIF conversion remains available without the moment strip.",
                )
            })
    };

    let frames = timeout(PREVIEW_TIMEOUT, render).await.map_err(|_| {
        ConversionError::new(
            ConversionErrorCode::EngineFailed,
            "Video moments took too long to prepare",
            "GIF conversion remains available without the moment strip.",
        )
    })??;
    Ok(StoryboardAsset { frames })
}

fn spawn_storyboard_frame(
    tasks: &mut JoinSet<(usize, Result<PreviewAsset, ConversionError>)>,
    engine: &EngineRuntime,
    source: &Path,
    index: usize,
    last_sample: f64,
) {
    let engine = engine.clone();
    let source = source.to_path_buf();
    let fraction = index as f64 / (STORYBOARD_FRAME_COUNT - 1) as f64;
    tasks.spawn(async move {
        let frame = render_storyboard_frame(&engine, &source, last_sample * fraction).await;
        (index, frame)
    });
}

async fn render_storyboard_frame(
    engine: &EngineRuntime,
    source: &Path,
    at_seconds: f64,
) -> Result<PreviewAsset, ConversionError> {
    let at = format!("{at_seconds:.3}");
    let mut command = crate::engine_process::engine_command(&engine.media()?.ffmpeg);
    command
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-ss",
            &at,
            "-i",
        ])
        .arg(source)
        .args([
            "-map",
            "0:v:0",
            "-frames:v",
            "1",
            "-an",
            "-vf",
            "scale=w=160:h=90:force_original_aspect_ratio=decrease:flags=lanczos,pad=160:90:(ow-iw)/2:(oh-ih)/2:color=0x14221f,setsar=1",
            "-c:v",
            "mjpeg",
            "-q:v",
            "5",
            "-f",
            "image2pipe",
            "pipe:1",
        ]);
    let bytes = run_preview(
        command,
        source,
        MAX_STORYBOARD_FRAME_BYTES,
        "The video moment strip could not be created",
        "GIF conversion remains available without the moment strip.",
    )
    .await?;
    Ok(PreviewAsset {
        mime_type: "image/jpeg".to_owned(),
        data_base64: BASE64.encode(bytes),
    })
}

pub async fn render_clip(
    engine: &EngineRuntime,
    source: &Path,
    start_seconds: f64,
    end_seconds: f64,
) -> Result<PreviewAsset, ConversionError> {
    let encoder = engine.preferred_h264_encoder().ok_or_else(|| {
        ConversionError::new(
            ConversionErrorCode::UnsupportedCodec,
            "Video preview is unavailable",
            "The source poster and GIF conversion remain available.",
        )
    })?;
    let start = format!("{start_seconds:.3}");
    let preview_duration = format!("{:.3}", (end_seconds - start_seconds).min(3.0));
    let mut command = crate::engine_process::engine_command(&engine.media()?.ffmpeg);
    command
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-ss",
            &start,
            "-i",
        ])
        .arg(source)
        .args([
            "-t",
            &preview_duration,
            "-an",
            "-vf",
            "scale=w='min(iw,640)':h=-2:flags=lanczos",
            "-c:v",
            encoder,
            "-preset",
            "ultrafast",
            "-crf",
            "28",
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "+frag_keyframe+empty_moov+default_base_moof",
            "-f",
            "mp4",
            "pipe:1",
        ]);
    let bytes = run_preview(
        command,
        source,
        MAX_CLIP_BYTES,
        "Video preview could not be created",
        "The source poster and GIF conversion remain available.",
    )
    .await?;
    Ok(PreviewAsset {
        mime_type: "video/mp4".to_owned(),
        data_base64: BASE64.encode(bytes),
    })
}

async fn run_preview(
    mut command: Command,
    source: &Path,
    max_bytes: usize,
    title: &'static str,
    message: &'static str,
) -> Result<Vec<u8>, ConversionError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(|error| {
        ConversionError::new(ConversionErrorCode::EngineFailed, title, message)
            .with_details(error.to_string())
    })?;
    let stdout = child.stdout.take().ok_or_else(|| {
        ConversionError::new(ConversionErrorCode::EngineFailed, title, message)
            .with_details("The preview process did not expose standard output.")
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        ConversionError::new(ConversionErrorCode::EngineFailed, title, message)
            .with_details("The preview process did not expose diagnostics.")
    })?;

    let run = async {
        let stdout_future = read_limited(stdout, max_bytes);
        let stderr_future = read_limited(stderr, MAX_DIAGNOSTIC_BYTES);
        let (stdout, stderr, status) = tokio::join!(stdout_future, stderr_future, child.wait());
        Ok::<_, std::io::Error>(PreviewOutput {
            status: status?,
            stdout: stdout?,
            stderr: stderr?,
        })
    };
    let output = match timeout(PREVIEW_TIMEOUT, run).await {
        Ok(Ok(output)) => output,
        Ok(Err(error)) => {
            return Err(
                ConversionError::new(ConversionErrorCode::EngineFailed, title, message)
                    .with_details(error.to_string()),
            );
        }
        Err(_) => {
            let _ = child.kill().await;
            return Err(ConversionError::new(
                ConversionErrorCode::EngineFailed,
                "Video preview took too long",
                message,
            ));
        }
    };
    if !output.status.success() || output.stdout.is_empty() || output.stdout.len() > max_bytes {
        let details = String::from_utf8_lossy(&output.stderr)
            .replace(source.to_string_lossy().as_ref(), "<source>");
        return Err(
            ConversionError::new(ConversionErrorCode::EngineFailed, title, message)
                .with_details(details.chars().take(2_000).collect::<String>()),
        );
    }
    Ok(output.stdout)
}

async fn read_limited(
    reader: impl AsyncRead + Unpin,
    max_bytes: usize,
) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(max_bytes.min(64 * 1024));
    reader
        .take(max_bytes.saturating_add(1) as u64)
        .read_to_end(&mut bytes)
        .await?;
    Ok(bytes)
}
