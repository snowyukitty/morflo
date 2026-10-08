import type { CapabilityRegistry, ImageSettings, MediaFile } from "../types";

type ImageSource = Pick<MediaFile, "extension" | "hasAlpha" | "width" | "height">;
type ImageFormat = ImageSettings["outputFormat"];
export type ImageOutcome = "share" | "smaller" | "transparent";
export const imageOutcomes: ImageOutcome[] = ["share", "smaller", "transparent"];

function available(format: ImageFormat, registry: CapabilityRegistry): boolean {
  return registry.outputs.some((output) => output.format === format && output.available);
}

export function recommendedImageOutput(
  file: ImageSource,
  registry: CapabilityRegistry,
): ImageFormat | undefined {
  const extension = file.extension.replace(/^\./u, "").toLowerCase();
  const preferred: ImageFormat[] =
    extension === "png"
      ? ["jpeg", "png", "webp"]
      : extension === "webp"
        ? file.hasAlpha
          ? ["png", "webp"]
          : ["jpeg", "png", "webp"]
        : file.hasAlpha
          ? ["webp", "png"]
          : ["webp", "jpeg", "png"];
  return preferred.find((format) => available(format, registry));
}

/** Resolve each file independently; presets never grant an unprobed format. */
export function imageOutcomeSettings(
  outcome: ImageOutcome,
  file: ImageSource,
  settings: ImageSettings,
  registry: CapabilityRegistry,
): ImageSettings | undefined {
  const format: ImageFormat =
    outcome === "transparent"
      ? "png"
      : outcome === "smaller" && file.hasAlpha
        ? available("webp", registry)
          ? "webp"
          : "png"
        : "jpeg";
  if (!available(format, registry) || (outcome === "transparent" && !file.hasAlpha)) {
    return undefined;
  }

  const result: ImageSettings = {
    ...settings,
    outputFormat: format,
    quality: outcome === "smaller" ? "smaller" : "balanced",
    resizeMode: "original",
    width: undefined,
    height: undefined,
    percentage: undefined,
  };
  // Known source dimensions keep the sharing preset from enlarging a small image.
  if (
    outcome === "share" &&
    file.width &&
    file.height &&
    Math.max(file.width, file.height) > 1920
  ) {
    result.resizeMode = "contain";
    result.width = 1920;
    result.height = 1920;
  }
  return result;
}

export function matchesImageOutcome(
  settings: ImageSettings,
  candidate: ImageSettings,
): boolean {
  return (
    settings.outputFormat === candidate.outputFormat &&
    settings.quality === candidate.quality &&
    settings.resizeMode === candidate.resizeMode &&
    settings.width === candidate.width &&
    settings.height === candidate.height &&
    settings.percentage === candidate.percentage
  );
}
