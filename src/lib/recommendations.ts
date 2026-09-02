import type { ImageSettings, MediaFile } from "../types";

export function recommendedImageOutput(
  file: Pick<MediaFile, "extension" | "hasAlpha">,
): ImageSettings["outputFormat"] {
  const extension = file.extension.replace(/^\./u, "").toLowerCase();
  if (extension === "png") return "jpeg";
  if (extension === "webp") return file.hasAlpha ? "png" : "jpeg";
  return "webp";
}
