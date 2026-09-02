# Conversion matrix

Legend:

- **Implemented**: product path exists.
- **Verified**: a real-engine fixture was converted and the output probed on the recorded host.
- **Capability-dependent**: path is intended but only displayed when engine probing passes.
- **Planned**: not available in v0.1.

No route is marked verified until the real-engine suite records evidence and probes the output.

| Input                         | Output        | Status               | Notes                                                              |
| ----------------------------- | ------------- | -------------------- | ------------------------------------------------------------------ |
| PNG                           | JPEG          | Verified             | Alpha is flattened onto the explicit/default background.           |
| JPEG                          | PNG           | Verified             | Dimensions and output media type are re-probed before publish.     |
| PNG/JPEG/WebP                 | ICO           | Verified             | One file with 16, 32, 48, and 256 px PNG frames.                   |
| PNG/JPEG/WebP/BMP/TIFF/ICO    | PNG/JPEG/WebP | Verified             | Real outputs are re-probed; large and unusual dimensions covered.  |
| AVIF                          | PNG/JPEG/WebP | Verified             | Alpha auxiliary stream is merged before still-image output.        |
| HEIC/HEIF                     | PNG/JPEG/WebP | Capability-dependent | Never claimed from extension alone; fixture unavailable initially. |
| PNG                           | AVIF          | Verified             | Color plus auxiliary AV1 alpha stream when transparency exists.    |
| MOV/MKV/WebM                  | MP4           | Verified             | H.264 + AAC; compatible audio/subtitles and chapters verified.     |
| MP4                           | WebM          | Verified             | VP9 + Opus; 1080p source capped to 720p without aspect change.     |
| MP4/MOV/MKV/M4V/AVI/MPEG/WebM | MP4/WebM      | Capability-dependent | Other container directions require a real fixture before claim.    |
| MP4/MOV/WebM                  | GIF           | Verified             | Trimmed two-stage palette pipeline; forever and once loops tested. |

## Gate 2 verification evidence

- Engine: FFmpeg/ffprobe 8.1.2, Windows GPL development build. Initial evidence
  used a local unshipped WSL path-translation shim; the complete ten-case suite
  was later repeated natively on Windows with the same installed engine.
- Fixtures: locally generated transparent PNG and JPEG; test paths contain Chinese characters, emoji, spaces, and an apostrophe.
- Assertions: engine exit success, source bytes unchanged, output can be probed, dimensions preserved, transparent corner flattened to the chosen warm background, collision suffix selected, and the pre-existing collision target remains byte-identical.
- This proves the conversion core and development engine combination. It is not a claim that the GPL engine binary is bundled or cleared for distribution.

## Gate 3 verification evidence

- Six ignored-by-default real-engine test cases passed with no mock engine. The suite executes more than 34 conversions, including a 20-image JPEG → WebP batch.
- PNG, JPEG, and WebP each produced a four-stream ICO at 16, 32, 48, and 256 px.
- Transparent PNG → WebP retained alpha; transparent PNG → AVIF produced color and alpha streams; AVIF → PNG restored RGBA.
- BMP → PNG, TIFF → WebP, ICO → PNG, 17×2049 PNG → WebP, and 4096×3072 PNG → JPEG passed with source dimensions and source files preserved.
- EXIF orientation 6 was discovered from decoded frame side data; JPEG 360×240 displayed and converted as 240×360.
- APNG is rejected until the user explicitly selects “Use the first frame”; the confirmed still output is non-animated.

## Gate 4 verification evidence

- Portrait MOV 720×1280 → MP4 retained portrait dimensions, duration tolerance, and audio.
- MKV → MP4 retained two AAC audio tracks, converted the compatible SRT subtitle to mov_text, and retained two chapters under Preserve metadata.
- 1920×1080 MP4 → WebM produced VP9/Opus at 1280×720 with at least one measured intermediate progress event before `progress=end`.
- WebM → MP4 produced H.264/AAC; silent MP4 → WebM remained silent; VFR-like MKV → MP4 remained probeable with duration within 350 ms.
- Truncated MP4 maps to a human `damaged_input` error; raw ffprobe text is available only in technical details.
- A real 120-second VP9 process was canceled after startup. The initial WSL
  interoperability run completed under the two-second budget; the final native
  Windows job-object run completed in 667 ms and left no FFmpeg process.

## Gate 5 verification evidence

- MP4, portrait MOV, and WebM each produced an animated, probeable GIF through the same two-stage palette planner. The MP4 case verified a 0.5–2.7 second trim, 540 px width, 12 fps intent, 2.2 second duration tolerance, monotonic stage-scaled progress, and an infinite-loop extension.
- The MOV and WebM cases verified the 360 px Chat route and once-only behavior without an infinite-loop extension.
- Palette temporary files are UUID-scoped, live beside the reserved output, and were absent after the run. The final filename appeared only after the partial GIF had been probed.
- Real preview evidence covers a 720 px JPEG poster, seven distinct 160×90 bounded moment frames, and a decodable 640×360 fragmented MP4 range preview bounded to three seconds.
- On the high-motion synthetic fixture, the selected palette pipeline measured SSIM 0.798505 at 504,092 bytes versus FFmpeg's direct GIF path at SSIM 0.783447 and 367,090 bytes. The quality gain costs 37.3% more bytes on this fixture; Morflo therefore keeps qualitative size guidance and does not promise smaller output.

## Native Windows final verification evidence

- Ten ignored-by-default real-engine integration tests passed / 0 failed / 0
  mocked using the installed Windows FFmpeg/ffprobe 8.1.2 pair.
- The suite covered corrupt input, collision safety, PNG/JPEG, bounded previews,
  EXIF/APNG safety, curated and large images, WebP/AVIF/ICO, trimmed GIF, a
  20-image batch, and video streams/duration.
- A Windows-specific regression verifies canonical source paths do not use the
  verbatim `\\?\` form presented to external tools. This prevents APNG from
  being misclassified as the single-frame `image2` demuxer.
- Native process-tree cancellation completed in 667 ms and left no encoder
  child. Source immutability, partial cleanup, and output probing assertions
  remained active in the native run.
- A release-executable WebView2 journey launched a PNG whose path contains
  Chinese characters, emoji, spaces, and an apostrophe, accepted the default
  JPEG recommendation, and published `name (2).jpg` while a pre-existing
  `name.jpg` remained byte-identical. The source SHA-256 remained unchanged,
  FFprobe reported MJPEG at 16×2048, and no `.morflo-part` file remained.
- That native journey exposed a finalization defect not covered by the earlier
  direct-plan tests: FFprobe identifies JPEG stills as the `image2` demuxer on
  this engine. Final verification now checks both container evidence and the
  primary codec (`image2` + `mjpeg`) rather than accepting or rejecting on a
  demuxer label alone.
