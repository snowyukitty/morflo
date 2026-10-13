import type { ConversionSettings, MediaFile } from "../types";
import { formatBytes, formatDimensions } from "./format";

export type LedgerView = "all" | "attention" | "completed";

export function needsAttention(file: MediaFile): boolean {
  return file.status === "failed" || file.status === "canceled";
}

export function matchesLedgerView(file: MediaFile, view: LedgerView): boolean {
  return (
    view === "all" ||
    (view === "attention" ? needsAttention(file) : file.status === "succeeded")
  );
}

export function measuredChange(input: number, output: number) {
  const delta = output - input;
  const sign = delta > 0 ? "+" : delta < 0 ? "−" : "";
  return {
    bytes: `${sign}${formatBytes(Math.abs(delta))}`,
    percent: input > 0 ? `${sign}${Math.abs((delta / input) * 100).toFixed(1)}%` : "—",
    tone: delta > 0 ? "larger" : delta < 0 ? "smaller" : "equal",
  };
}

export function measuredBatch(files: MediaFile[]) {
  const measured = files.filter((file) => file.status === "succeeded" && file.output);
  const input = measured.reduce((sum, file) => sum + file.sizeBytes, 0);
  const output = measured.reduce((sum, file) => sum + (file.output?.sizeBytes ?? 0), 0);
  return { count: measured.length, input, output, change: measuredChange(input, output) };
}

// Describes the request, never an estimated output or a promised file size.
export function plannedSettings(file: MediaFile, settings?: ConversionSettings): string {
  if (!settings || file.kind === "unsupported") return "Choose a supported source";
  if (file.kind === "video") {
    if (settings.video.outputFormat === "gif") {
      return `${settings.gif.width} px · ${settings.gif.fps} fps · ${settings.gif.startSeconds}–${settings.gif.endSeconds} s`;
    }
    return `${settings.video.resolution === "original" ? "Original dimensions" : settings.video.resolution} · ${settings.video.quality} quality`;
  }
  const image = settings.image;
  if (image.outputFormat === "ico") return "16–256 px · multiple sizes";
  const dimensions =
    image.resizeMode === "original"
      ? file.width && file.height
        ? formatDimensions(file.width, file.height)
        : "Original dimensions"
      : image.resizeMode === "percentage"
        ? `${image.percentage ?? "—"}% of original`
        : image.resizeMode === "width"
          ? `${image.width ?? "—"} px wide`
          : image.resizeMode === "height"
            ? `${image.height ?? "—"} px high`
            : `${image.resizeMode === "contain" ? "Fit within" : "Fill"} ${image.width ?? "—"} × ${image.height ?? "—"}`;
  return `${dimensions} · ${image.outputFormat === "png" ? "lossless" : `${image.quality} quality`}`;
}
