use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Image,
    Video,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Probing,
    Ready,
    Running,
    Succeeded,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobPhase {
    Inspecting,
    Waiting,
    #[serde(rename = "Preparing palette")]
    PreparingPalette,
    Encoding,
    Finalizing,
    Converted,
    Canceled,
    #[serde(rename = "Needs attention")]
    NeedsAttention,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Png,
    Jpeg,
    Webp,
    Avif,
    Ico,
    Mp4,
    Webm,
    Gif,
}

impl OutputFormat {
    pub const ALL: [Self; 8] = [
        Self::Png,
        Self::Jpeg,
        Self::Webp,
        Self::Avif,
        Self::Ico,
        Self::Mp4,
        Self::Webm,
        Self::Gif,
    ];

    pub const fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
            Self::Avif => "avif",
            Self::Ico => "ico",
            Self::Mp4 => "mp4",
            Self::Webm => "webm",
            Self::Gif => "gif",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputSummary {
    pub name: String,
    pub format: OutputFormat,
    pub size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_alpha: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WarningSeverity {
    Info,
    Warning,
    Danger,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaWarning {
    pub code: String,
    pub title: String,
    pub message: String,
    pub severity: WarningSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaFile {
    pub id: String,
    pub name: String,
    pub extension: String,
    pub size_bytes: u64,
    pub kind: MediaKind,
    pub status: JobStatus,
    pub phase: JobPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_alpha: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_codec: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_tracks: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tracks: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle_tracks: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_tracks: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_tracks: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdr: Option<bool>,
    pub warnings: Vec<MediaWarning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ConversionError>,
}

#[derive(Debug, Clone)]
pub struct InspectedSource {
    pub media: MediaFile,
    pub path: PathBuf,
    pub format_name: String,
    pub primary_codec: Option<String>,
    pub primary_stream_index: u32,
    pub alpha_stream_index: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    Smaller,
    Balanced,
    Best,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetadataPolicy {
    Remove,
    Preserve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollisionPolicy {
    Suffix,
    Skip,
    Replace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResizeMode {
    Original,
    Width,
    Height,
    Percentage,
    Contain,
    Cover,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnimationPolicy {
    Ask,
    First,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Resolution {
    Original,
    #[serde(rename = "1080p")]
    Hd1080,
    #[serde(rename = "720p")]
    Hd720,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GifPreset {
    Chat,
    Web,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoopBehavior {
    Forever,
    Once,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DestinationMode {
    Same,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageSettings {
    pub output_format: OutputFormat,
    pub quality: Quality,
    pub resize_mode: ResizeMode,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub percentage: Option<u16>,
    pub background: String,
    pub metadata: MetadataPolicy,
    pub animation: AnimationPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoSettings {
    pub output_format: OutputFormat,
    pub resolution: Resolution,
    pub quality: Quality,
    pub metadata: MetadataPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GifSettings {
    pub preset: GifPreset,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub width: u32,
    pub fps: u16,
    pub quality: Quality,
    pub r#loop: LoopBehavior,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionSettings {
    pub image: ImageSettings,
    pub video: VideoSettings,
    pub gif: GifSettings,
    pub destination: DestinationMode,
    pub destination_path: Option<String>,
    pub collision_policy: CollisionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartJobRequest {
    pub job_id: String,
    pub settings: ConversionSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineSource {
    Bundled,
    Project,
    Selected,
    System,
    /// Morflo's own compiled-in image engine, used when no media engine is
    /// present. It is always available and needs no local installation.
    Native,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    pub available: bool,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub source: EngineSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatCapability {
    pub format: OutputFormat,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityRegistry {
    pub engine: EngineInfo,
    pub outputs: Vec<FormatCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewAsset {
    pub mime_type: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryboardAsset {
    pub frames: Vec<PreviewAsset>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupReport {
    pub recovered_temporary_files: usize,
    pub recovery_failures: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversionErrorCode {
    UnsupportedFormat,
    UnsupportedCodec,
    DamagedInput,
    PermissionDenied,
    InsufficientSpace,
    EngineMissing,
    OutputExists,
    Canceled,
    EngineFailed,
    InvalidRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, Error)]
#[error("{title}: {message}")]
#[serde(rename_all = "camelCase")]
pub struct ConversionError {
    pub code: ConversionErrorCode,
    pub title: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub technical_details: Option<String>,
}

impl ConversionError {
    pub fn new(
        code: ConversionErrorCode,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            title: title.into(),
            message: message.into(),
            technical_details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.technical_details = Some(details.into());
        self
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(
            ConversionErrorCode::InvalidRequest,
            "Check the conversion settings",
            message,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgressEvent {
    pub job_id: String,
    pub status: JobStatus,
    pub phase: JobPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ConversionError>,
}

impl JobProgressEvent {
    pub fn state(job_id: impl Into<String>, status: JobStatus, phase: JobPhase) -> Self {
        Self {
            job_id: job_id.into(),
            status,
            phase,
            progress: None,
            output: None,
            error: None,
        }
    }
}
