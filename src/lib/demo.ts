import type { CapabilityRegistry, MediaFile } from "../types";

const transparentWarning = {
  code: "alpha-flatten",
  title: "JPEG has no transparency",
  message: "Transparent areas will use the background color you choose.",
  severity: "warning" as const,
};

const files = {
  queue: [
    {
      id: "summer-poster",
      name: "夏祭り poster.png",
      extension: "png",
      sizeBytes: 4_862_021,
      kind: "image",
      status: "ready",
      phase: "Waiting",
      width: 2400,
      height: 3000,
      hasAlpha: true,
      warnings: [transparentWarning],
    },
    {
      id: "product-photo",
      name: "O'Reilly cup.jpg",
      extension: "jpg",
      sizeBytes: 8_243_113,
      kind: "image",
      status: "ready",
      phase: "Waiting",
      width: 4032,
      height: 3024,
      hasAlpha: false,
      warnings: [],
    },
    {
      id: "portrait-mov",
      name: "京都散歩 🧳.mov",
      extension: "mov",
      sizeBytes: 184_432_782,
      kind: "video",
      status: "ready",
      phase: "Waiting",
      width: 1080,
      height: 1920,
      durationSeconds: 42.2,
      videoCodec: "HEVC",
      audioTracks: 1,
      subtitleTracks: 0,
      chapterCount: 0,
      warnings: [],
    },
  ],
  active: [
    {
      id: "active-mov",
      name: "Tokyo morning.mov",
      extension: "mov",
      sizeBytes: 428_043_221,
      kind: "video",
      status: "running",
      phase: "Encoding",
      progress: 0.62,
      width: 3840,
      height: 2160,
      durationSeconds: 95,
      videoCodec: "HEVC",
      audioTracks: 2,
      subtitleTracks: 1,
      chapterCount: 4,
      warnings: [
        {
          code: "streams",
          title: "This file has extra streams",
          message:
            "Morflo will preserve both audio tracks, the compatible subtitle, and chapters.",
          severity: "info",
        },
      ],
    },
    {
      id: "waiting-jpg",
      name: "cover.jpg",
      extension: "jpg",
      sizeBytes: 3_128_500,
      kind: "image",
      status: "queued",
      phase: "Waiting",
      width: 2560,
      height: 1440,
      warnings: [],
    },
  ],
  gif: [
    {
      id: "gif-video",
      name: "launch-loop.mp4",
      extension: "mp4",
      sizeBytes: 32_310_118,
      kind: "video",
      status: "ready",
      phase: "Waiting",
      width: 1920,
      height: 1080,
      durationSeconds: 12.4,
      videoCodec: "H.264",
      audioTracks: 1,
      subtitleTracks: 0,
      chapterCount: 0,
      warnings: [],
    },
  ],
  complete: [
    {
      id: "complete-mov",
      name: "京都散歩 🧳.mov",
      extension: "mov",
      sizeBytes: 184_432_782,
      kind: "video",
      status: "succeeded",
      phase: "Converted",
      progress: 1,
      width: 1080,
      height: 1920,
      durationSeconds: 42.2,
      videoCodec: "HEVC",
      audioTracks: 1,
      subtitleTracks: 0,
      chapterCount: 0,
      warnings: [],
      output: {
        name: "京都散歩 🧳.mp4",
        format: "mp4",
        sizeBytes: 72_118_406,
        hasAlpha: false,
        width: 1080,
        height: 1920,
        durationSeconds: 42.2,
      },
    },
    {
      id: "complete-webp",
      name: "hero.jpg",
      extension: "jpg",
      sizeBytes: 8_432_144,
      kind: "image",
      status: "succeeded",
      phase: "Converted",
      progress: 1,
      width: 3200,
      height: 1800,
      warnings: [],
      output: {
        name: "hero.webp",
        format: "webp",
        sizeBytes: 2_146_780,
        hasAlpha: false,
        width: 3200,
        height: 1800,
      },
    },
  ],
  error: [
    {
      id: "broken-file",
      name: "damaged-scan.webp",
      extension: "webp",
      sizeBytes: 128_104,
      kind: "image",
      status: "failed",
      phase: "Needs attention",
      warnings: [],
      error: {
        code: "damaged_input",
        title: "This image appears to be damaged",
        message: "Morflo could read the file header, but the image data ended unexpectedly.",
        technicalDetails: "ffprobe exited with code 1: invalid WebP chunk at byte 127,992",
      },
    },
  ],
} satisfies Record<string, MediaFile[]>;

const batchFiles: MediaFile[] = Array.from({ length: 20 }, (_, index) => ({
  id: `batch-${index + 1}`,
  name: `Product photo ${String(index + 1).padStart(2, "0")}.jpg`,
  extension: "jpg",
  sizeBytes: 3_200_000 + index * 41_337,
  kind: "image",
  status: "ready",
  phase: "Waiting",
  width: 2400,
  height: 1600,
  hasAlpha: false,
  warnings: [],
}));

export const demoCapabilities: CapabilityRegistry = {
  engine: {
    available: true,
    name: "FFmpeg",
    version: "8.1.2",
    source: "system",
  },
  outputs: ["png", "jpeg", "webp", "avif", "ico", "mp4", "webm", "gif"].map((format) => ({
    format: format as CapabilityRegistry["outputs"][number]["format"],
    available: true,
  })),
};

export const missingDemoCapabilities: CapabilityRegistry = {
  engine: {
    available: false,
    name: "FFmpeg",
    source: "missing",
    diagnostic: "No compatible ffmpeg and ffprobe pair was detected for this session.",
  },
  outputs: ["png", "jpeg", "webp", "avif", "ico", "mp4", "webm", "gif"].map((format) => ({
    format: format as CapabilityRegistry["outputs"][number]["format"],
    available: false,
    reason: "A compatible local media engine was not found.",
  })),
};

/// The registry a computer with no media engine now receives: the built-in
/// image engine is available and every format it cannot write says why.
export const nativeDemoCapabilities: CapabilityRegistry = {
  engine: {
    available: true,
    name: "Morflo built-in image engine",
    version: "0.2.0",
    source: "native",
    diagnostic: "No development or system engine pair was found",
  },
  outputs: [
    { format: "png", available: true },
    { format: "jpeg", available: true },
    { format: "ico", available: true },
    {
      format: "webp",
      available: false,
      reason:
        "Needs a local media engine. Morflo's built-in engine can only write WebP losslessly, which would enlarge most photographs.",
    },
    {
      format: "avif",
      available: false,
      reason: "Needs a local media engine with Morflo's validated AV1 encoder.",
    },
    {
      format: "mp4",
      available: false,
      reason: "Needs a local media engine. Morflo's built-in engine converts images only.",
    },
    {
      format: "webm",
      available: false,
      reason: "Needs a local media engine. Morflo's built-in engine converts images only.",
    },
    {
      format: "gif",
      available: false,
      reason: "Needs a local media engine. Morflo's built-in engine converts images only.",
    },
  ],
};

export function getDemoFiles(state: string | null): MediaFile[] {
  if (state === "batch") return structuredClone(batchFiles);
  if (state === "partial") {
    const completedFile = files.complete.at(1);
    const failedFile = files.error.at(0);
    return completedFile && failedFile ? structuredClone([completedFile, failedFile]) : [];
  }
  const selected = state && state in files ? files[state as keyof typeof files] : undefined;
  return structuredClone(selected ?? []);
}
