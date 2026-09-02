# Preset rationale

## Image quality

Primary labels are `Smaller`, `Balanced`, and `Best quality`. They map to encoder-native settings only inside the Rust planner. The UI may reveal the numeric value under Technical details, but does not pretend numbers are comparable across JPEG, WebP, and AVIF.

- **Balanced** is the default: useful visual quality without extreme encode time or size.
- **Smaller** accepts visible loss for sharing.
- **Best quality** reduces loss but does not claim losslessness unless the format is lossless.
- PNG and ICO are lossless; their primary quality control is therefore omitted.

AVIF uses the validated `libaom-av1` adapter only: CRF 40 / 30 / 20 for Smaller / Balanced / Best quality and `cpu-used 6` for a practical desktop encode. Transparent input is represented as a color stream plus a grayscale auxiliary alpha stream, matching the [official FFmpeg AVIF muxer contract](https://ffmpeg.org/ffmpeg-formats.html#avif). Other AV1 encoders remain unavailable until their option mapping and output have independent fixture evidence.

ICO always creates one PNG-compressed container with 16, 32, 48, and 256 px entries. Images are fitted and transparently padded rather than stretched or cropped.

Metadata defaults to **Remove metadata** for privacy, stated beside the control. Visual orientation is applied before removal. `Preserve metadata` maps what the engine can carry, but Morflo does not promise byte-identical EXIF or ICC preservation across formats.

Image recommendations avoid unexplained same-format work: PNG begins with JPEG
and its explicit alpha decision, JPEG and other common image families begin with
WebP, transparent WebP begins with alpha-preserving PNG, and opaque WebP begins
with easy-to-share JPEG. The user can always choose another capability-proven
output.

## Video

- **Universal MP4**: H.264, AAC, `yuv420p`, source frame rate, source dimensions unless a maximum is selected, and web-friendly fast-start. Chosen for broad playback compatibility.
- **Smaller MP4**: H.264 with a higher constant-quality value and a slower encoder preset; dimensions remain unchanged unless selected.
- **Web-friendly WebM**: VP9 and Opus, source dimensions unless constrained.
- Resolution choices are `Keep original`, `Up to 1080p`, and `Up to 720p`. They never upscale and never change aspect ratio.

Quality maps to MP4 CRF 28 / 23 / 18 and WebM CRF 38 / 31 / 24 for Smaller / Balanced / Best quality. MP4 uses x264 `medium` except Best quality (`slow`); WebM uses libvpx-vp9 `deadline=good`, `cpu-used=2`, and row multithreading. These are v0.1 software paths chosen for deterministic availability; hardware encoders are not silently substituted.

Every audio track is re-encoded for the target. Compatible subtitle tracks are converted to mov_text (MP4) or WebVTT (WebM); an incompatible subtitle fails visibly instead of disappearing. Chapters follow the visible metadata choice. Unsupported stream types, extra video streams, attachments, HDR, and subtitle conversion limits become preflight warnings.

## GIF

| Preset       | Width cap | FPS | Palette colors | Intent                                            |
| ------------ | --------: | --: | -------------: | ------------------------------------------------- |
| Chat         |    360 px |  10 |            128 | Short, lightweight reactions                      |
| Web          |    540 px |  12 |            192 | Default balance for pages and documentation       |
| High quality |    720 px |  18 |            256 | Motion detail where a larger result is acceptable |

All preserve aspect ratio and avoid upscaling. A dimensionless complexity signal (`duration × FPS × (width / 540)²`) produces qualitative `Comfortable`, `Likely large`, or `Very large` guidance. This makes a short 720 px loop less alarming than a long one while escalating duration, frame rate, and pixel area together. Morflo never silently trims and never presents a fake byte estimate.

Range selection uses seven evenly spaced, bounded source frames as orientation
evidence. They do not estimate GIF appearance or size; the separate local clip
preview confirms motion, while the completed receipt previews the actual GIF.

### GIF engine comparison

The reproducible `pnpm benchmark:gif` run uses a synthetic high-motion 2.2 second range at 540 px and 12 fps. With FFmpeg 8.1.2, palettegen + paletteuse produced 504,092 bytes at full-frame RGB SSIM 0.798505. FFmpeg's direct GIF path produced 367,090 bytes at SSIM 0.783447. SSIM is only a repeatable regression signal—not a complete perceptual GIF metric—but the palette path improved fidelity on the deliberately difficult gradients and motion at a 37.3% size cost.

FFmpeg palette generation was selected over gifski for v0.1 because it uses the already-probed engine, exposes two cancellable process stages, supports the product's loop contract, and avoids adding a second executable. Gifski was reviewed from its official project and license but was not executed: it is absent from the test host, AGPL-3.0-or-later by default, and would require a separately reviewed sidecar or commercial arrangement before the root project license is chosen. Gifski remains a future opt-in adapter candidate after license and distribution review; no quality or speed claim is made for an unrun tool.
