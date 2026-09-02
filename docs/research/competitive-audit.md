# Competitive audit

Research date: 2026-08-30. Sources are current public product pages and official project documentation; no proprietary code or assets were copied.

## Workflow comparison

| Product                                                   | Primary workflow                                                    | Strength                                                             | Cost Morflo should avoid                                                |
| --------------------------------------------------------- | ------------------------------------------------------------------- | -------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| [CloudConvert](https://cloudconvert.com/)                 | Select/drop → output → options → cloud job → download               | Clear input/output grammar and conversion-specific controls          | Upload trust, extreme catalog breadth, API/business surface             |
| [Convertio](https://convertio.co/)                        | Choose/upload → format → convert → download                         | Very obvious first action and low learning cost                      | Accounts, cloud retention, format-count marketing, competing navigation |
| [HandBrake](https://handbrake.fr/features.php)            | Open source → choose outcome/device preset → inspect → queue/encode | Durable presets, preview, truthful technical depth, mature queue     | Dense panes and expert vocabulary before intent is fully clear          |
| [Shutter Encoder](https://www.shutterencoder.com/)        | Add files → choose function → configure large panel → start/queue   | Strong local workflow, trim preview, diagnostics, broad FFmpeg power | Feature accumulation, settings wall, editing/downloading scope creep    |
| [File Converter](https://github.com/Tichau/FileConverter) | Explorer context menu → preset → background conversion              | Extremely low-friction desktop integration and tuned presets         | Windows-only architecture and low discoverability outside Explorer      |

## Ten useful patterns

1. Make file selection the unmistakable first action.
2. Express the default decision as input → useful output, not encoder selection.
3. Change controls with the chosen operation instead of showing a universal settings form.
4. Offer an outcome preset that is safe without editing it.
5. Keep detailed source information available but secondary.
6. Support queueing and partial success for real batch work.
7. Preview trim boundaries for short-loop extraction.
8. Let users save effort with remembered format-specific choices, without forcing accounts.
9. Integrate reveal/open actions into the completion state.
10. Pair broad engine capability with a deliberately smaller, tested product surface.

## Ten anti-patterns

1. Competing format-count claims instead of verified journeys.
2. Uploading private media for a task that can run locally.
3. Login, pricing, API, OCR, and unrelated navigation around the primary task.
4. A function dropdown containing dozens of expert operations.
5. Codec, bitrate, profile, and pixel-format controls in the default view.
6. Presets that silently drop surround audio, subtitles, chapters, or animation.
7. Decorative progress that does not reflect engine work.
8. Treating file extension as proof that decoding or encoding works.
9. Explorer-only discoverability or platform-specific domain concepts.
10. Adding trimming, filters, overlays, downloaders, or AI until a converter becomes an editor.

## Product implications

- Morflo uses one primary surface: file queue at left/center, contextual intent inspector at right, and one stable action bar.
- Empty and working states share the same shell so learning transfers immediately.
- The default video choices are Universal MP4, Smaller MP4, and Web-friendly WebM; codec details live in Technical details.
- GIF is a dedicated outcome with a compact range control, not merely another extension.
- Capability discovery and per-item warnings precede conversion; format catalogs never overrule the active engine.
- A focused Windows Open with entry earns its place as a low-friction P1
  addition, but the primary app surface remains complete and discoverable on
  every platform. It must not become a Windows-only domain abstraction.

## Follow-up: restrained Windows shell intake

- Microsoft's [Default Programs guidance](https://learn.microsoft.com/en-us/windows/win32/shell/default-programs)
  keeps the user's default choice authoritative. Morflo therefore registers as
  an alternate handler and never writes a file type default or `UserChoice`.
- Microsoft's [Open With registration guidance](https://learn.microsoft.com/en-us/windows/win32/shell/how-to-include-an-application-on-the-open-with-dialog-box)
  supports an application-specific ProgID and `OpenWithProgids`; the installed
  handler command quotes the executable and `%1` independently.
- Microsoft's [file-association best practices](https://learn.microsoft.com/en-us/windows/win32/shell/fa-best-practices)
  favor versioned ProgIDs and removal of only application-owned registration.
  Morflo uses `Morflo.Media.1` and audits install/uninstall/reinstall against
  pre-existing Photos, Photoshop, and VLC choices.
- Tauri's [single-instance plugin](https://v2.tauri.app/plugin/single-instance/)
  provides later-process arguments and working directory. Morflo registers it
  before other plugins, validates the paths in Rust, and sends only a count-only
  arrival event to React.

Implementation decision: offer Open with for the 13 common image/video
extensions already inside Morflo's verified or conservative inspected surface.
Do not advertise HEIC/HEIF or less-tested video containers merely because an
extension can be registered. Do not add per-format context-menu verbs, convert
in the background, or make the shell the only way to discover Morflo.

## Technical and accessibility findings

- [FFmpeg progress output](https://ffmpeg.org/ffmpeg.html) provides periodic machine-readable key/value updates and a terminal `progress=end`, which supports honest progress without parsing decorative console stats.
- [ffprobe JSON](https://ffmpeg.org/ffprobe.html) provides a structured inspection boundary for format and stream data.
- [libvips](https://www.libvips.org/) is demand-driven, threaded, low-memory, LGPL-2.1-or-later, and supports the target image families when built with their optional libraries. It is attractive later but adds native dependency packaging now.
- [gifski](https://github.com/ImageOptim/gifski) can improve temporal palettes and motion quality but is AGPL-3.0-or-later and its direct video build adds FFmpeg linkage complexity. Morflo v0.1 therefore uses FFmpeg palette generation while preserving an adapter boundary.
- [Tauri capabilities](https://v2.tauri.app/reference/acl/capability/) provide a narrow IPC permission boundary. Remote capability origins are omitted.
- [Tauri CSP guidance](https://v2.tauri.app/security/csp/) explicitly recommends avoiding remote scripts and CDNs. Morflo ships all runtime assets locally.
- WCAG 2.2 and [Microsoft desktop accessibility guidance](https://learn.microsoft.com/en-us/windows/apps/develop/accessibility) drive keyboard completeness, visible focus, semantic roles/names/values, status announcements, 4.5:1 normal-text contrast, non-color status cues, reflow, target size, reduced motion, and DPI/text scaling checks.

## Follow-up: range confidence without editor scope

- [Permute's official help](https://software.charliemonroe.net/help/permute/overview.html) demonstrates useful automatic grouping by media kind and one setting surface per compatible group. Its customizable drops and broad workshop surface are intentionally not copied: Morflo keeps a selected-file inspector and no automation matrix in v0.1.
- [HandBrake's preview workflow](https://handbrake.fr/docs/en/latest/workflow/preview-settings.html) validates the value of encoding a short representative segment before committing to a long job. Morflo keeps that action local and bounded to the selected GIF range rather than opening a separate preview editor.
- The [WAI-ARIA multi-thumb slider pattern](https://www.w3.org/WAI/ARIA/apg/patterns/slider-multithumb/) requires stable thumb tab order, dependent minimum/maximum values, and understandable value text. Morflo uses two native range inputs over one visual rail, keeps Start before End in tab order, updates their constraints as the other value moves, and exposes precise timestamps.
- [FFmpeg's input-seek documentation](https://ffmpeg.org/ffmpeg.html) explains that `-ss` before `-i` performs seek-point positioning with accurate decode/discard enabled by default for transcoding. The moment strip uses that bounded path instead of decoding from the beginning for late samples.

Implementation decision: replace the decorative strip with seven real local
moments, but do not add waveform editing, timeline zoom, tracks, keyframes, or
drag-to-trim media editing. The strip is navigation evidence; the poster and
three-second preview remain the confirmation surfaces.
