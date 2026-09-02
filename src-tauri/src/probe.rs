use std::{collections::HashMap, path::Path, process::Stdio, time::Duration};

use serde::Deserialize;
use tokio::time::timeout;
use uuid::Uuid;

use crate::{
    domain::{
        ConversionError, ConversionErrorCode, InspectedSource, JobPhase, JobStatus, MediaFile,
        MediaKind, MediaWarning, WarningSeverity,
    },
    engine::EngineRuntime,
};

const PROBE_TIMEOUT: Duration = Duration::from_secs(45);
const MAX_DIMENSION: u32 = 100_000;

#[derive(Debug, Default, Deserialize)]
struct RawProbe {
    #[serde(default)]
    streams: Vec<RawStream>,
    #[serde(default)]
    format: RawFormat,
    #[serde(default)]
    chapters: Vec<RawChapter>,
    #[serde(default)]
    frames: Vec<RawFrame>,
}

#[derive(Debug, Default, Deserialize)]
struct RawFormat {
    #[serde(default)]
    format_name: String,
    duration: Option<String>,
    #[serde(default)]
    tags: HashMap<String, String>,
}

#[derive(Debug, Default, Deserialize)]
struct RawStream {
    #[serde(default)]
    index: u32,
    #[serde(default)]
    codec_type: String,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    pix_fmt: Option<String>,
    duration: Option<String>,
    nb_frames: Option<String>,
    color_transfer: Option<String>,
    #[serde(default)]
    tags: HashMap<String, String>,
    #[serde(default)]
    disposition: RawDisposition,
    #[serde(default)]
    side_data_list: Vec<RawSideData>,
}

#[derive(Debug, Default, Deserialize)]
struct RawDisposition {
    #[serde(default)]
    attached_pic: u8,
}

#[derive(Debug, Default, Deserialize)]
struct RawSideData {
    rotation: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
struct RawFrame {
    #[serde(default)]
    stream_index: u32,
    #[serde(default)]
    side_data_list: Vec<RawSideData>,
}

#[derive(Debug, Default, Deserialize)]
struct RawChapter {
    #[serde(default)]
    _id: Option<u32>,
}

pub async fn inspect_path(
    engine: &EngineRuntime,
    requested_path: &Path,
) -> Result<InspectedSource, ConversionError> {
    let path = validate_source(requested_path)?;
    let metadata = std::fs::metadata(&path).map_err(map_source_io_error)?;
    if metadata.len() == 0 {
        return Err(ConversionError::new(
            ConversionErrorCode::DamagedInput,
            "This file is empty",
            "Choose a complete image or video file.",
        ));
    }

    // FFprobe stays authoritative whenever a media engine is present, so an
    // installation that has one behaves exactly as before. Only a machine
    // without one falls back to the built-in engine, which reads common images
    // and honestly declines everything else.
    if !engine.has_media_engine() {
        return native_inspection(path, metadata.len());
    }

    let output = run_ffprobe(engine, &path).await?;
    if !output.status.success() {
        return Err(map_probe_failure(&output.stderr, &path));
    }
    let probe: RawProbe = serde_json::from_slice(&output.stdout).map_err(|error| {
        ConversionError::new(
            ConversionErrorCode::DamagedInput,
            "This file could not be inspected",
            "The media engine returned incomplete information for this file.",
        )
        .with_details(error.to_string())
    })?;

    normalize_probe(path, metadata.len(), probe)
}

pub fn failed_media_file(path: &Path, error: ConversionError) -> MediaFile {
    let metadata = std::fs::metadata(path).ok();
    MediaFile {
        id: Uuid::new_v4().to_string(),
        name: display_name(path),
        extension: extension(path),
        size_bytes: metadata.as_ref().map_or(0, std::fs::Metadata::len),
        kind: MediaKind::Unsupported,
        status: JobStatus::Failed,
        phase: JobPhase::NeedsAttention,
        progress: None,
        width: None,
        height: None,
        duration_seconds: None,
        has_alpha: None,
        animated: None,
        video_codec: None,
        video_tracks: None,
        audio_tracks: None,
        subtitle_tracks: None,
        chapter_count: None,
        attachment_tracks: None,
        data_tracks: None,
        hdr: None,
        warnings: Vec::new(),
        output: None,
        error: Some(error),
    }
}

fn validate_source(path: &Path) -> Result<std::path::PathBuf, ConversionError> {
    let metadata = std::fs::symlink_metadata(path).map_err(map_source_io_error)?;
    if metadata.file_type().is_symlink() {
        return Err(ConversionError::new(
            ConversionErrorCode::InvalidRequest,
            "Symbolic links are not accepted",
            "Choose the original image or video file directly.",
        ));
    }
    if !metadata.is_file() {
        return Err(ConversionError::new(
            ConversionErrorCode::UnsupportedFormat,
            "This is not a media file",
            "Morflo accepts individual image and video files, not folders.",
        ));
    }
    // std::fs::canonicalize returns a verbatim `\\?\` path on Windows. FFmpeg
    // accepts the file but can lose its extension-based demuxer selection (an
    // APNG becomes single-frame image2 evidence). Dunce preserves canonical
    // resolution while simplifying only paths that have an equivalent form
    // understood by external Windows media tools.
    dunce::canonicalize(path).map_err(map_source_io_error)
}

async fn run_ffprobe(
    engine: &EngineRuntime,
    path: &Path,
) -> Result<std::process::Output, ConversionError> {
    let mut command = crate::engine_process::engine_command(&engine.media()?.ffprobe);
    command
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_streams",
            "-show_format",
            "-show_chapters",
            "-read_intervals",
            "%+#1",
            "-show_frames",
            "-show_entries",
            "format=format_name,duration:format_tags=major_brand:stream=index,codec_type,codec_name,width,height,pix_fmt,duration,nb_frames,color_transfer:stream_tags=rotate,title:stream_disposition=attached_pic:stream_side_data=rotation:frame=stream_index:frame_side_data=rotation:chapter=id",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .kill_on_drop(true);

    match timeout(PROBE_TIMEOUT, command.output()).await {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(error)) => Err(ConversionError::new(
            ConversionErrorCode::EngineFailed,
            "This file could not be inspected",
            "The local media engine could not read the selected file.",
        )
        .with_details(error.to_string())),
        Err(_) => Err(ConversionError::new(
            ConversionErrorCode::DamagedInput,
            "Inspection took too long",
            "The file may be damaged, incomplete, or on an unavailable drive.",
        )),
    }
}

/// Inspect an image with Morflo's built-in engine, for machines that have no
/// media engine at all.
///
/// This reports strictly what the built-in engine can establish from the file
/// itself: dimensions, alpha and animation. There are no streams, chapters or
/// codecs to describe, so the stream-shaped fields stay empty rather than being
/// filled with invented values.
fn native_inspection(
    path: std::path::PathBuf,
    size_bytes: u64,
) -> Result<InspectedSource, ConversionError> {
    let facts = crate::image_engine::inspect(&path).ok_or_else(|| {
        ConversionError::new(
            ConversionErrorCode::UnsupportedFormat,
            "This file needs a local media engine",
            "Morflo's built-in engine reads common images. Install a compatible FFmpeg build, or choose an engine folder in Diagnostics, to open videos and other formats.",
        )
    })?;

    let warnings = build_warnings(WarningFacts {
        kind: MediaKind::Image,
        animated: facts.animated,
        audio_tracks: 0,
        subtitle_tracks: 0,
        chapter_count: 0,
        hdr: false,
        video_tracks: 1,
        attachment_tracks: 0,
        data_tracks: 0,
    });

    let media = MediaFile {
        id: Uuid::new_v4().to_string(),
        name: display_name(&path),
        extension: extension(&path),
        size_bytes,
        kind: MediaKind::Image,
        status: JobStatus::Ready,
        phase: JobPhase::Waiting,
        progress: None,
        width: Some(facts.width),
        height: Some(facts.height),
        duration_seconds: None,
        has_alpha: Some(facts.has_alpha),
        animated: Some(facts.animated),
        video_codec: None,
        video_tracks: None,
        audio_tracks: None,
        subtitle_tracks: None,
        chapter_count: None,
        attachment_tracks: None,
        data_tracks: None,
        hdr: None,
        warnings,
        output: None,
        error: None,
    };

    Ok(InspectedSource {
        media,
        path,
        format_name: facts.format_name.to_owned(),
        primary_codec: Some(facts.codec_name.to_owned()),
        primary_stream_index: 0,
        alpha_stream_index: None,
    })
}

fn normalize_probe(
    path: std::path::PathBuf,
    size_bytes: u64,
    probe: RawProbe,
) -> Result<InspectedSource, ConversionError> {
    let alpha_stream_index = probe
        .streams
        .iter()
        .find(|stream| stream_is_alpha_auxiliary(stream))
        .map(|stream| stream.index);
    let primary = probe
        .streams
        .iter()
        .filter(|stream| {
            stream.codec_type == "video"
                && stream.disposition.attached_pic == 0
                && !stream_is_alpha_auxiliary(stream)
        })
        .max_by_key(|stream| {
            u64::from(stream.width.unwrap_or_default())
                * u64::from(stream.height.unwrap_or_default())
        })
        .ok_or_else(|| {
            ConversionError::new(
                ConversionErrorCode::UnsupportedFormat,
                "No image or video was found",
                "This file does not contain a visual stream Morflo can convert.",
            )
        })?;

    let mut width = primary.width;
    let mut height = primary.height;
    if width.is_none_or(|value| value == 0 || value > MAX_DIMENSION)
        || height.is_none_or(|value| value == 0 || value > MAX_DIMENSION)
    {
        return Err(ConversionError::new(
            ConversionErrorCode::DamagedInput,
            "The media dimensions are not valid",
            "The file header contains missing or unreasonable dimensions.",
        ));
    }
    if is_quarter_turn(rotation(primary, &probe.frames)) {
        std::mem::swap(&mut width, &mut height);
    }

    let duration = parse_number(probe.format.duration.as_deref())
        .or_else(|| parse_number(primary.duration.as_deref()));
    let audio_tracks = count_streams(&probe.streams, "audio");
    let subtitle_tracks = count_streams(&probe.streams, "subtitle");
    let video_tracks = u32::try_from(
        probe
            .streams
            .iter()
            .filter(|stream| {
                stream.codec_type == "video"
                    && stream.disposition.attached_pic == 0
                    && !stream_is_alpha_auxiliary(stream)
            })
            .count(),
    )
    .unwrap_or(u32::MAX);
    let attachment_tracks = count_streams(&probe.streams, "attachment");
    let data_tracks = count_streams(&probe.streams, "data");
    let major_brand = probe
        .format
        .tags
        .get("major_brand")
        .map(|value| value.to_ascii_lowercase());
    let kind = classify_media(
        &probe.format.format_name,
        major_brand.as_deref(),
        primary,
        duration,
        audio_tracks,
    );
    let animated = (kind == MediaKind::Image)
        .then(|| is_animated(primary, duration, &probe.format.format_name));
    let has_alpha = (kind == MediaKind::Image).then(|| {
        alpha_stream_index.is_some()
            || primary
                .pix_fmt
                .as_deref()
                .is_some_and(pixel_format_may_have_alpha)
    });
    let hdr = (kind == MediaKind::Video).then(|| {
        primary
            .color_transfer
            .as_deref()
            .is_some_and(|transfer| matches!(transfer, "smpte2084" | "arib-std-b67"))
    });
    let chapter_count = u32::try_from(probe.chapters.len()).unwrap_or(u32::MAX);
    let warnings = build_warnings(WarningFacts {
        kind,
        animated: animated.unwrap_or(false),
        audio_tracks,
        subtitle_tracks,
        chapter_count,
        hdr: hdr.unwrap_or(false),
        video_tracks,
        attachment_tracks,
        data_tracks,
    });
    let id = Uuid::new_v4().to_string();
    let media = MediaFile {
        id,
        name: display_name(&path),
        extension: extension(&path),
        size_bytes,
        kind,
        status: JobStatus::Ready,
        phase: JobPhase::Waiting,
        progress: None,
        width,
        height,
        duration_seconds: (kind == MediaKind::Video || animated == Some(true))
            .then_some(duration)
            .flatten(),
        has_alpha,
        animated,
        video_codec: (kind == MediaKind::Video)
            .then(|| primary.codec_name.clone())
            .flatten(),
        video_tracks: (kind == MediaKind::Video).then_some(video_tracks),
        audio_tracks: (kind == MediaKind::Video).then_some(audio_tracks),
        subtitle_tracks: (kind == MediaKind::Video).then_some(subtitle_tracks),
        chapter_count: (kind == MediaKind::Video).then_some(chapter_count),
        attachment_tracks: (kind == MediaKind::Video).then_some(attachment_tracks),
        data_tracks: (kind == MediaKind::Video).then_some(data_tracks),
        hdr,
        warnings,
        output: None,
        error: None,
    };

    Ok(InspectedSource {
        media,
        path,
        format_name: probe.format.format_name,
        primary_codec: primary.codec_name.clone(),
        primary_stream_index: primary.index,
        alpha_stream_index,
    })
}

fn stream_is_alpha_auxiliary(stream: &RawStream) -> bool {
    stream.codec_type == "video"
        && stream.tags.iter().any(|(key, value)| {
            key.eq_ignore_ascii_case("title") && value.eq_ignore_ascii_case("alpha")
        })
}

fn classify_media(
    format_name: &str,
    major_brand: Option<&str>,
    stream: &RawStream,
    duration: Option<f64>,
    audio_tracks: u32,
) -> MediaKind {
    let demuxers = format_name.split(',').collect::<Vec<_>>();
    let known_image_demuxer = demuxers.iter().any(|name| {
        matches!(
            *name,
            "png_pipe"
                | "jpeg_pipe"
                | "webp_pipe"
                | "bmp_pipe"
                | "tiff_pipe"
                | "ico"
                | "gif"
                | "apng"
                | "image2"
                | "image2pipe"
        )
    });
    let image_brand = major_brand.is_some_and(|brand| {
        brand.starts_with("avif")
            || brand.starts_with("avis")
            || brand.starts_with("heic")
            || brand.starts_with("heix")
            || brand.starts_with("heif")
            || brand.starts_with("mif1")
    });
    let frame_count = parse_integer(stream.nb_frames.as_deref());
    let still_codec = stream
        .codec_name
        .as_deref()
        .is_some_and(|codec| matches!(codec, "png" | "mjpeg" | "webp" | "bmp" | "tiff" | "gif"));
    let still_shape = audio_tracks == 0
        && duration.is_none_or(|value| value <= 1.0)
        && frame_count.is_none_or(|value| value <= 1);

    if known_image_demuxer || image_brand || (still_codec && still_shape) {
        MediaKind::Image
    } else {
        MediaKind::Video
    }
}

fn is_animated(stream: &RawStream, duration: Option<f64>, format_name: &str) -> bool {
    parse_integer(stream.nb_frames.as_deref()).is_some_and(|frames| frames > 1)
        || duration.is_some_and(|seconds| seconds > 0.1)
        || format_name
            .split(',')
            .any(|format| matches!(format, "apng" | "gif"))
}

fn pixel_format_may_have_alpha(pixel_format: &str) -> bool {
    let format = pixel_format.to_ascii_lowercase();
    format.contains("rgba")
        || format.contains("bgra")
        || format.contains("argb")
        || format.contains("abgr")
        || format.contains("yuva")
        || format.contains("gbrap")
        || format.starts_with("ya")
        || format == "pal8"
}

fn rotation(stream: &RawStream, frames: &[RawFrame]) -> i32 {
    stream
        .side_data_list
        .iter()
        .find_map(|side_data| side_data.rotation)
        .or_else(|| {
            stream
                .tags
                .get("rotate")
                .and_then(|value| value.parse().ok())
        })
        .or_else(|| {
            frames
                .iter()
                .find(|frame| frame.stream_index == stream.index)
                .and_then(|frame| {
                    frame
                        .side_data_list
                        .iter()
                        .find_map(|side_data| side_data.rotation)
                })
        })
        .unwrap_or_default()
}

fn is_quarter_turn(rotation: i32) -> bool {
    rotation.rem_euclid(180) == 90
}

fn count_streams(streams: &[RawStream], kind: &str) -> u32 {
    u32::try_from(
        streams
            .iter()
            .filter(|stream| stream.codec_type == kind)
            .count(),
    )
    .unwrap_or(u32::MAX)
}

#[derive(Debug, Clone, Copy)]
struct WarningFacts {
    kind: MediaKind,
    animated: bool,
    audio_tracks: u32,
    subtitle_tracks: u32,
    chapter_count: u32,
    hdr: bool,
    video_tracks: u32,
    attachment_tracks: u32,
    data_tracks: u32,
}

fn build_warnings(facts: WarningFacts) -> Vec<MediaWarning> {
    let WarningFacts {
        kind,
        animated,
        audio_tracks,
        subtitle_tracks,
        chapter_count,
        hdr,
        video_tracks,
        attachment_tracks,
        data_tracks,
    } = facts;
    let mut warnings = Vec::new();
    if animated {
        warnings.push(MediaWarning {
            code: "animated-input".to_owned(),
            title: "This image is animated".to_owned(),
            message: "Choose how animation should be handled before creating a still image."
                .to_owned(),
            severity: WarningSeverity::Warning,
        });
    }
    if kind == MediaKind::Video && audio_tracks == 0 {
        warnings.push(MediaWarning {
            code: "no-audio".to_owned(),
            title: "No audio track".to_owned(),
            message: "The converted video will remain silent.".to_owned(),
            severity: WarningSeverity::Info,
        });
    } else if audio_tracks > 1 {
        warnings.push(MediaWarning {
            code: "multiple-audio".to_owned(),
            title: "Multiple audio tracks".to_owned(),
            message: "Morflo will re-encode every audio track for the selected container."
                .to_owned(),
            severity: WarningSeverity::Warning,
        });
    }
    if subtitle_tracks > 0 {
        warnings.push(MediaWarning {
            code: "subtitles".to_owned(),
            title: "Subtitle tracks detected".to_owned(),
            message: "Morflo will convert compatible subtitle tracks. An incompatible subtitle causes a clear failure instead of being dropped.".to_owned(),
            severity: WarningSeverity::Warning,
        });
    }
    if chapter_count > 0 {
        warnings.push(MediaWarning {
            code: "chapters".to_owned(),
            title: "Chapters detected".to_owned(),
            message: "Chapters are kept when Preserve metadata is selected and removed when Remove metadata is selected.".to_owned(),
            severity: WarningSeverity::Info,
        });
    }
    if hdr {
        warnings.push(MediaWarning {
            code: "hdr".to_owned(),
            title: "HDR video detected".to_owned(),
            message: "The first release cannot guarantee HDR preservation across formats."
                .to_owned(),
            severity: WarningSeverity::Warning,
        });
    }
    if video_tracks > 1 {
        warnings.push(MediaWarning {
            code: "multiple-video".to_owned(),
            title: "Additional video streams detected".to_owned(),
            message: "Morflo converts the primary picture stream only. Additional angles or visual streams are not included."
                .to_owned(),
            severity: WarningSeverity::Warning,
        });
    }
    if attachment_tracks > 0 || data_tracks > 0 {
        warnings.push(MediaWarning {
            code: "container-extras".to_owned(),
            title: "Container extras detected".to_owned(),
            message: "Embedded attachments and private data streams are not copied to the simplified output."
                .to_owned(),
            severity: WarningSeverity::Warning,
        });
    }
    warnings
}

fn map_source_io_error(error: std::io::Error) -> ConversionError {
    let (code, title, message) = if error.kind() == std::io::ErrorKind::PermissionDenied {
        (
            ConversionErrorCode::PermissionDenied,
            "This file cannot be opened",
            "Check its permissions or choose a local copy.",
        )
    } else {
        (
            ConversionErrorCode::DamagedInput,
            "This file is unavailable",
            "It may have moved, been disconnected, or become unreadable.",
        )
    };
    ConversionError::new(code, title, message).with_details(error.to_string())
}

fn map_probe_failure(stderr: &[u8], path: &Path) -> ConversionError {
    let raw = String::from_utf8_lossy(stderr);
    let lower = raw.to_ascii_lowercase();
    let (code, title, message) = if lower.contains("permission denied") {
        (
            ConversionErrorCode::PermissionDenied,
            "This file cannot be opened",
            "Check its permissions or choose a local copy.",
        )
    } else if lower.contains("decoder") && lower.contains("not found") {
        (
            ConversionErrorCode::UnsupportedCodec,
            "This codec is not available",
            "The detected local media engine cannot decode this file.",
        )
    } else if lower.contains("invalid data")
        || lower.contains("moov atom not found")
        || lower.contains("end of file")
    {
        (
            ConversionErrorCode::DamagedInput,
            "This file appears to be damaged",
            "The file header or media data ended unexpectedly.",
        )
    } else {
        (
            ConversionErrorCode::UnsupportedFormat,
            "This file is not supported",
            "The detected local media engine could not identify a compatible image or video stream.",
        )
    };
    ConversionError::new(code, title, message).with_details(redact_details(&raw, path))
}

fn redact_details(details: &str, path: &Path) -> String {
    let filename = display_name(path);
    let full = path.to_string_lossy();
    let redacted = details.replace(full.as_ref(), &format!("<source>/{filename}"));
    redacted.chars().take(4_000).collect()
}

fn parse_number(value: Option<&str>) -> Option<f64> {
    value?
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite() && *number >= 0.0)
}

fn parse_integer(value: Option<&str>) -> Option<u64> {
    value?.parse().ok()
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Unnamed media".to_owned())
}

fn extension(path: &Path) -> String {
    path.extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream(codec: &str, frames: Option<&str>, alpha: Option<&str>) -> RawStream {
        RawStream {
            index: 0,
            codec_type: "video".to_owned(),
            codec_name: Some(codec.to_owned()),
            width: Some(100),
            height: Some(100),
            pix_fmt: alpha.map(str::to_owned),
            nb_frames: frames.map(str::to_owned),
            ..RawStream::default()
        }
    }

    #[test]
    fn classifies_probe_evidence_instead_of_extension() {
        let png = stream("png", Some("1"), Some("rgba"));
        assert_eq!(
            classify_media("png_pipe", None, &png, None, 0),
            MediaKind::Image
        );

        let h264 = stream("h264", Some("300"), Some("yuv420p"));
        assert_eq!(
            classify_media(
                "mov,mp4,m4a,3gp,3g2,mj2",
                Some("isom"),
                &h264,
                Some(10.0),
                1
            ),
            MediaKind::Video
        );
    }

    #[test]
    fn detects_common_alpha_pixel_formats() {
        assert!(pixel_format_may_have_alpha("rgba"));
        assert!(pixel_format_may_have_alpha("yuva420p"));
        assert!(pixel_format_may_have_alpha("pal8"));
        assert!(!pixel_format_may_have_alpha("yuv420p"));
        assert!(!pixel_format_may_have_alpha("rgb24"));
    }

    #[test]
    fn identifies_avif_alpha_auxiliary_streams() {
        let mut alpha = stream("av1", Some("1"), Some("gray"));
        alpha.tags.insert("title".to_owned(), "Alpha".to_owned());
        assert!(stream_is_alpha_auxiliary(&alpha));
    }

    #[test]
    fn rotates_display_dimensions_only_for_quarter_turns() {
        assert!(is_quarter_turn(90));
        assert!(is_quarter_turn(-90));
        assert!(!is_quarter_turn(180));
    }

    #[cfg(windows)]
    #[test]
    fn canonical_source_path_is_compatible_with_external_windows_tools() {
        let directory = tempfile::tempdir().expect("create source path fixture");
        let source = directory.path().join("animated.png");
        std::fs::write(&source, b"fixture").expect("write source path fixture");

        let validated = validate_source(&source).expect("validate source path");

        assert!(validated.is_absolute());
        assert!(!validated.to_string_lossy().starts_with(r"\\?\"));
    }
}
