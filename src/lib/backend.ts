import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

import type {
  CapabilityRegistry,
  ConversionError,
  ConversionSettings,
  JobProgressEvent,
  MediaFile,
  PreviewAsset,
  StoryboardAsset,
  StartupReport,
} from "../types";
import { copy } from "../i18n/en";

export async function chooseFiles(): Promise<string[]> {
  if (!isTauri()) return [];
  const selected = await open({
    multiple: true,
    directory: false,
    title: copy.dialogs.chooseFiles,
    filters: [
      {
        name: copy.dialogs.mediaFilter,
        extensions: [
          "png",
          "jpg",
          "jpeg",
          "webp",
          "bmp",
          "tif",
          "tiff",
          "avif",
          "heic",
          "heif",
          "ico",
          "mp4",
          "mov",
          "mkv",
          "webm",
          "avi",
          "m4v",
          "mpeg",
          "mpg",
        ],
      },
    ],
  });
  if (!selected) return [];
  return Array.isArray(selected) ? selected : [selected];
}

export async function chooseOutputFolder(): Promise<string | undefined> {
  if (!isTauri()) return undefined;
  const selected = await open({
    multiple: false,
    directory: true,
    title: copy.dialogs.chooseOutputFolder,
  });
  return typeof selected === "string" ? selected : undefined;
}

export async function chooseEngineFolder(): Promise<string | undefined> {
  if (!isTauri()) return undefined;
  const selected = await open({
    multiple: false,
    directory: true,
    title: copy.dialogs.chooseEngineFolder,
  });
  return typeof selected === "string" ? selected : undefined;
}

export async function inspectFiles(paths: string[]): Promise<MediaFile[]> {
  return invoke<MediaFile[]>("inspect_files", { paths });
}

export async function retryInspection(jobId: string): Promise<MediaFile> {
  return invoke<MediaFile>("retry_inspection", { jobId });
}

export async function getCapabilities(): Promise<CapabilityRegistry> {
  return invoke<CapabilityRegistry>("get_capabilities");
}

export async function refreshCapabilities(): Promise<CapabilityRegistry> {
  return invoke<CapabilityRegistry>("refresh_capabilities");
}

export async function selectEngineDirectory(directory: string): Promise<CapabilityRegistry> {
  return invoke<CapabilityRegistry>("select_engine_directory", { directory });
}

export async function getStartupReport(): Promise<StartupReport> {
  return invoke<StartupReport>("startup_report");
}

export async function getPendingFiles(): Promise<string[]> {
  return invoke<string[]>("pending_files");
}

export async function startJob(file: MediaFile, settings: ConversionSettings): Promise<void> {
  await invoke("start_job", { request: { jobId: file.id, settings } });
}

export async function cancelJob(jobId: string): Promise<void> {
  await invoke("cancel_job", { jobId });
}

export async function revealOutput(jobId: string): Promise<void> {
  await invoke("reveal_output", { jobId });
}

function previewUrl(asset: PreviewAsset): string {
  return `data:${asset.mimeType};base64,${asset.dataBase64}`;
}

export async function getPosterFrame(jobId: string, atSeconds: number): Promise<string> {
  const asset = await invoke<PreviewAsset>("poster_frame", { jobId, atSeconds });
  return previewUrl(asset);
}

export async function getVideoStoryboard(jobId: string): Promise<string[]> {
  const asset = await invoke<StoryboardAsset>("video_storyboard", { jobId });
  return asset.frames.map(previewUrl);
}

export async function getImageThumbnail(jobId: string): Promise<string> {
  const asset = await invoke<PreviewAsset>("image_thumbnail", { jobId });
  return previewUrl(asset);
}

export async function getPreviewClip(
  jobId: string,
  startSeconds: number,
  endSeconds: number,
): Promise<string> {
  const asset = await invoke<PreviewAsset>("preview_clip", {
    jobId,
    startSeconds,
    endSeconds,
  });
  return previewUrl(asset);
}

export async function getOutputPreview(jobId: string): Promise<string> {
  const asset = await invoke<PreviewAsset>("output_preview", { jobId });
  return previewUrl(asset);
}

export async function forgetSource(jobId: string): Promise<void> {
  if (!isTauri()) return;
  await invoke("forget_source", { jobId });
}

export async function clearSession(): Promise<void> {
  if (!isTauri()) return;
  await invoke("clear_session");
}

export async function listenForJobProgress(
  onProgress: (event: JobProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<JobProgressEvent>("morflo://job-progress", ({ payload }) => {
    onProgress(payload);
  });
}

export async function listenForExternalFiles(
  onFiles: (count: number) => void,
): Promise<UnlistenFn> {
  return listen<{ count: number }>("morflo://external-files", ({ payload }) => {
    onFiles(payload.count);
  });
}

export function hasDesktopRuntime(): boolean {
  return isTauri();
}

export function normalizeCommandError(
  error: unknown,
  fallback: ConversionError,
): ConversionError {
  if (typeof error !== "object" || error === null) {
    return {
      ...fallback,
      technicalDetails: error instanceof Error ? error.message : String(error),
    };
  }
  const value = error as Partial<ConversionError>;
  if (
    typeof value.code === "string" &&
    typeof value.title === "string" &&
    typeof value.message === "string"
  ) {
    return {
      code: value.code,
      title: value.title,
      message: value.message,
      technicalDetails:
        typeof value.technicalDetails === "string" ? value.technicalDetails : undefined,
    };
  }
  return { ...fallback, technicalDetails: JSON.stringify(error) };
}
