# Morflo product scope

## P0

- Windows 11 x64 desktop application using Tauri 2.
- Multi-file drag-and-drop and native file picker.
- Capability-driven inspection and format availability.
- A built-in image engine compiled into Morflo. PNG, JPEG and multi-resolution
  ICO output, and common image inspection, work with no media engine installed.
- Image output: PNG, JPEG, WebP, AVIF, and multi-resolution ICO when the active engine proves the required encoders and muxers.
- Video output: Universal MP4, Smaller MP4, Web-friendly WebM, and animated GIF.
- Image controls: outcome quality, original or constrained dimensions, alpha flattening background, metadata policy, destination, and naming.
- Image outcome shortcuts: Easy to share, Smaller file, and Keep transparency.
  Each resolves against actual capabilities for each selected image, retains
  metadata and animation consent, and leaves video settings unchanged. Sharing
  fits known large sources inside 1920 × 1920 px without enlarging small images;
  smaller-file processing preserves alpha and makes no size guarantee.
- Video controls: outcome preset, original/1080p/720p resolution, quality, destination; stream-loss warnings before execution.
- GIF controls: poster/visual preview, start/end range, width, FPS, quality preset, loop behavior, and qualitative large-file guidance.
- Mixed-media queue with item selection, compatible group settings, per-item override, real progress, cancellation, retry, partial success, and reveal.
- Bounded crash-recovery journal for recognized partial outputs.
- Light, dark, reduced-motion, keyboard, zoom, and high-contrast-aware behavior.
- Windows 11 x64 is the primary release target; produce a native unsigned NSIS
  artifact without weakening host security policy, plus a manual cross-platform
  CI build structure.
- The Windows installer registers Morflo as an alternate Open with handler for
  a conservative 13-extension subset. Later activations join the existing queue
  through one application instance; no default association is claimed.

## Capability-dependent, not unconditional promises

- Video, animated GIF, WebP and AVIF output need a local media engine. Without
  one they are disabled with the reason, not hidden and not offered.
- Keeping source metadata needs a local media engine. The built-in image engine
  writes no source metadata and refuses the request rather than silently
  removing what was asked to be kept.
- AVIF and HEIC/HEIF decoding and AVIF encoding depend on the reviewed engine build.
- H.264/AAC, VP9/Opus, image encoders, ICO muxing, palette filters, and subtitle conversion are independently probed.
- Network and removable-drive paths work only where the OS and active engine permit direct access.

## Explicitly deferred

- Audio extraction, lossless rewrap, deeper Explorer actions, reusable custom
  presets, and hardware encoding are P1 candidates. The focused Open with intake
  is the intentionally bounded Explorer integration shipped in v0.1.
- Documents, PDFs, OCR, archives, CAD, ebooks, fonts, subtitle editing, timeline editing, professional grading, recording, downloading, accounts, cloud storage, AI, plugins, analytics, payments, and automatic updates are out of scope.

## Scope rationale

The matrix covers the most common personal image exchange formats, web image output, favicon/application-icon creation, broadly playable MP4, open web video, and the high-value GIF extraction journey. It deliberately excludes breadth that would weaken warnings, packaging evidence, defaults, or fixture coverage.
