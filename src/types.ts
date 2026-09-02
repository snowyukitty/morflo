export type MediaKind = "image" | "video" | "unsupported";
export type JobStatus =
  "queued" | "probing" | "ready" | "running" | "succeeded" | "failed" | "canceled";
export type JobPhase =
  | "Inspecting"
  | "Waiting"
  | "Preparing palette"
  | "Encoding"
  | "Finalizing"
  | "Converted"
  | "Canceled"
  | "Needs attention";
export type OutputFormat = "png" | "jpeg" | "webp" | "avif" | "ico" | "mp4" | "webm" | "gif";
export type Quality = "smaller" | "balanced" | "best";
export type MetadataPolicy = "remove" | "preserve";
export type CollisionPolicy = "suffix" | "skip" | "replace";
export type ResizeMode = "original" | "width" | "height" | "percentage" | "contain" | "cover";
export type AnimationPolicy = "ask" | "first";
export type Resolution = "original" | "1080p" | "720p";
export type GifPreset = "chat" | "web" | "high";

export interface MediaWarning {
  code: string;
  title: string;
  message: string;
  severity: "info" | "warning" | "danger";
}

export interface OutputSummary {
  name: string;
  format: OutputFormat;
  sizeBytes: number;
  hasAlpha?: boolean | undefined;
  width?: number | undefined;
  height?: number | undefined;
  durationSeconds?: number | undefined;
}

export interface MediaFile {
  id: string;
  name: string;
  extension: string;
  sizeBytes: number;
  kind: MediaKind;
  status: JobStatus;
  phase: JobPhase;
  progress?: number | undefined;
  width?: number | undefined;
  height?: number | undefined;
  durationSeconds?: number | undefined;
  hasAlpha?: boolean | undefined;
  animated?: boolean | undefined;
  videoCodec?: string | undefined;
  videoTracks?: number | undefined;
  audioTracks?: number | undefined;
  subtitleTracks?: number | undefined;
  chapterCount?: number | undefined;
  attachmentTracks?: number | undefined;
  dataTracks?: number | undefined;
  hdr?: boolean | undefined;
  warnings: MediaWarning[];
  output?: OutputSummary | undefined;
  error?: ConversionError | undefined;
}

export interface ImageSettings {
  outputFormat: Extract<OutputFormat, "png" | "jpeg" | "webp" | "avif" | "ico">;
  quality: Quality;
  resizeMode: ResizeMode;
  width?: number | undefined;
  height?: number | undefined;
  percentage?: number | undefined;
  background: string;
  metadata: MetadataPolicy;
  animation: AnimationPolicy;
}

export interface VideoSettings {
  outputFormat: Extract<OutputFormat, "mp4" | "webm" | "gif">;
  resolution: Resolution;
  quality: Quality;
  metadata: MetadataPolicy;
}

export interface GifSettings {
  preset: GifPreset;
  startSeconds: number;
  endSeconds: number;
  width: number;
  fps: number;
  quality: Quality;
  loop: "forever" | "once";
}

export interface ConversionSettings {
  image: ImageSettings;
  video: VideoSettings;
  gif: GifSettings;
  destination: "same" | "custom";
  destinationPath?: string | undefined;
  collisionPolicy: CollisionPolicy;
}

export interface EngineInfo {
  available: boolean;
  name: string;
  version?: string | undefined;
  source: "bundled" | "project" | "selected" | "system" | "native" | "missing";
  diagnostic?: string | undefined;
}

export interface FormatCapability {
  format: OutputFormat;
  available: boolean;
  reason?: string | undefined;
}

export interface CapabilityRegistry {
  engine: EngineInfo;
  outputs: FormatCapability[];
}

export interface PreviewAsset {
  mimeType: string;
  dataBase64: string;
}

export interface StoryboardAsset {
  frames: PreviewAsset[];
}

export interface StartupReport {
  recoveredTemporaryFiles: number;
  recoveryFailures: number;
}

export interface ConversionError {
  code:
    | "unsupported_format"
    | "unsupported_codec"
    | "damaged_input"
    | "permission_denied"
    | "insufficient_space"
    | "engine_missing"
    | "output_exists"
    | "canceled"
    | "engine_failed"
    | "invalid_request";
  title: string;
  message: string;
  technicalDetails?: string | undefined;
}

export interface JobProgressEvent {
  jobId: string;
  status: JobStatus;
  phase: JobPhase;
  progress?: number | undefined;
  output?: OutputSummary | undefined;
  error?: ConversionError | undefined;
}
