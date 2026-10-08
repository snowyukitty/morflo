# Morflo status

Last updated: 2026-10-09

## Current gate

Gate 9 — released locally as unsigned 0.2.2. Morflo provides backward-compatible
media engine probing and video planning for FFmpeg < 6.0 (including 5.0.1),
coupled with built-in native image inspection fallbacks so host media engine
discrepancies never block format selection or image conversions.

Gate 8 and Gate 7 remain complete: built-in redistributable image engine, native
Windows first-run readiness, outcome confidence, GIF moment confidence, and
restrained Open with intake, with reviewed-sidecar intake and secure unreviewed
candidate dossiers technically ready. The private GitHub checkpoint is
established and synchronization to public Morflo is documented.

## Baseline

- Repository began as an empty directory with no Git metadata or project files.
- Initialized a local Git repository on `main`.
- Workspace lease: `morflow/full-build`, agent `codex-root`; released after final verification on 2026-08-30.
- Host: Windows 11-compatible x64 build 26200, 64 GB RAM.
- Tools: Node 24.18.0, pnpm 11.17.0, Rust/Cargo 1.97.1, Tauri CLI 2.11.4, FFmpeg/ffprobe 8.1.2.
- Missing optional local tools: libvips, gifski, ImageMagick.
- BrowserOS neo was used for read-only review of current official FFmpeg,
  ffprobe, FFmpeg legal-checklist, and SLSA provenance documentation. No account
  action, download, or external mutation was performed.

## Gate evidence

### Gate 0

- Completed focused competitor and official technical research.
- Selected Tauri/React/Rust and capability-probed FFmpeg adapter architecture.
- Recorded current GPL development-engine configuration and SHA-256 checksums without committing binaries.
- Defined P0 jobs, scope, conversion matrix, presets, architecture, accessibility, test, and release policies.

Checks run so far:

- Repository and workspace instruction audit: passed.
- Workspace lease conflict check: no conflicting Morflo lease.
- Local toolchain probe: passed; optional image/GIF engines absent.
- FFmpeg version/license/capability probe: passed for the observed development engine.
- Gate 0 documentation structure and secret-pattern scan: passed.

### Gate 1

- Scaffolded pinned React 19, strict TypeScript, Vite 8, Tauri 2, and Rust 2024 sources with lockfiles.
- Selected Quiet Precision after a three-direction 45-point rubric and implemented real empty, queue, active, GIF, completed, failure, dark, and narrow-window states.
- Designed and shipped the original folded-contact-strip application icon through IconFlow. Final rubric: legibility 4, distinctiveness 4, balance 5, color 5, scalability 4, craft 5; automated icon QA reported no warnings.
- Captured and inspected local screenshots at 1440×900, 1280×800, the supported 780×620 minimum, and 100/125/150% device scale.
- Completed visual refinement pass one: fixed dark-theme capture/state settling, compacted GIF outcome selection, replaced generic empty-state branding, clarified engine status, and preserved the narrow layout.

Checks run:

- `pnpm peers check`: passed.
- `pnpm lint`: passed with zero warnings.
- `pnpm typecheck`: passed.
- `pnpm build:frontend`: passed; 240.52 kB main JavaScript / 74.86 kB gzip at this checkpoint.
- `pnpm test:e2e`: passed 2 browser-driven screenshot tests (deterministic design states, not conversion tests).
- `cargo fmt --check`: passed on Windows.
- `cargo check`: passed on WSL2 Ubuntu with Rust 1.93.1, GTK 3.24.52, and WebKitGTK 2.52.3.
- At this checkpoint, native Windows `cargo check` was blocked before Morflo
  source compilation because enforced Windows Application Control rejected
  Cargo-generated unsigned build-script executables (`os error 4551`), including
  from a project-specific AppData target directory. Gate 7 follow-up resolves
  native packaging through the bounded path documented below.
- IconFlow `check`, source-bound `ship`, and casebook lint: passed.

### Gate 2

- Added a typed Rust domain boundary for inspection, capabilities, plans, jobs, progress, output policy, and errors.
- Added runtime FFmpeg/ffprobe discovery and capability probing; the frontend does not infer support from file extensions.
- Added structured ffprobe inspection with bounded concurrency and backend-only source paths.
- Implemented validated PNG/JPEG planning with explicit alpha flattening, metadata policy, resize filters, and direct argument-array process spawning.
- Added collision-safe output reservation, UUID-scoped `.morflo-part` files, output re-probing, and final publish only after success.
- Added a real job state machine, truthful FFmpeg progress parsing, image/video resource slots, bounded diagnostics, and process-tree cancellation hooks.
- Added session-scoped source/output storage, reveal behavior, and source/path forgetting when queue items are removed.
- Launched the compiled Tauri application under WSLg for a desktop-process smoke test; it remained responsive until deliberately terminated. Mesa emitted software/GPU bridge warnings in this environment.

Checks run:

- `cargo fmt --check`: passed.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed on WSL2.
- Rust unit tests: 14 passed.
- Real-engine integration tests: 2 passed using FFmpeg/ffprobe 8.1.2 through a local, unshipped WSL interoperability shim.
- Real-engine evidence covers PNG → JPEG, JPEG → PNG, explicit warm alpha flattening, output probing, source-byte immutability, suffix collision behavior, and paths containing Chinese, emoji, spaces, and an apostrophe.
- `pnpm format:check`, `pnpm lint`, `pnpm typecheck`, and `pnpm build:frontend`: passed; main JavaScript 240.29 kB / 74.85 kB gzip.
- Compiled desktop-process smoke launch: passed on WSL2/WSLg; native Windows
  compilation was still blocked at this checkpoint and was resolved in the
  Gate 7 follow-up below.

### Gate 3

- Completed PNG, JPEG, WebP, AVIF, and ICO output planning with one capability-probed Rust source of truth.
- Implemented AVIF alpha as the FFmpeg-specified color plus auxiliary alpha-stream structure and verified an AVIF → PNG transparency round trip.
- Implemented a single ICO containing 16, 32, 48, and 256 px PNG streams; non-square sources are fitted without distortion and padded transparently.
- Added width, height, percentage, contain, and cover controls; explicit animated-input first-frame consent; custom output-folder selection; and typed frontend command errors.
- Added frame-side-data inspection so EXIF orientation is reflected in displayed dimensions before metadata removal.
- Expanded deterministic legal image fixtures: EXIF orientation, transparent PNG/WebP, BMP, TIFF, AVIF, ICO, APNG, 4096×3072, unusual 17×2049, corrupt input, and international paths. HEIC remains skipped because the observed engine has no HEIF muxer.

Checks run:

- Rust unit tests: 15 passed.
- Real-engine integration tests: 6 passed / 0 failed / 0 mocked, covering 34+ conversions including the required 20-file batch.
- Verified input evidence: PNG, JPEG, WebP, BMP, TIFF, AVIF, ICO, and APNG inspection/handling. HEIC/HEIF remains unverified.
- Verified output evidence: PNG, JPEG, WebP, alpha AVIF, and four-stream ICO.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed on WSL2.
- Frontend unit/accessibility tests: 4 passed; Playwright visual checks: 2 passed.
- `pnpm format:check`, `pnpm lint`, and `pnpm typecheck`: passed.

### Gate 4

- Added validated Universal MP4 and Web-friendly WebM planners with direct argv spawning, runtime codec capability gates, VFR-friendly timestamps, and no upscaling.
- MP4 uses libx264/AAC/mov_text with fast-start; WebM uses VP9/Opus/WebVTT. Quality labels map only in Rust.
- Maps every audio and compatible subtitle track. Chapters follow the visible metadata policy. Additional visual streams, attachments, private data streams, and uncertain HDR handling produce explicit preflight warnings.
- Added per-job cancellation in the queue and fixed queued jobs so they cannot be removed while still active.
- Added synthetic 1080p MP4, portrait MOV, multi-stream/chapter MKV, WebM, silent video, VFR-like MKV, high-motion GIF source, and truncated video fixtures.
- Removed the unused video `preset` field; outcome, resolution, quality, and metadata are the only default video choices that earn a place.

Checks run:

- Rust unit tests: 17 passed, including structured command-argument construction and portrait/landscape resolution planning.
- Real-engine integration tests: 8 passed / 0 failed / 0 mocked. Video evidence covers MOV → MP4, MKV → MP4, MP4 → WebM, WebM → MP4, silent video, VFR-like input, multi-audio, subtitle, chapters, duration tolerance, and source immutability.
- Real process-tree cancellation test: passed; observed termination under 1 ms after cancellation in WSL2-to-Windows FFmpeg interoperability, with no FFmpeg process remaining. Native Windows cancellation is not claimed due the host compilation policy blocker.
- Frontend unit/accessibility tests: 5 passed.
- `cargo clippy --all-targets --all-features -- -D warnings`, frontend formatting, lint, typecheck, and production build: passed.

### Gate 5

- Added a two-stage FFmpeg palettegen/paletteuse GIF planner with explicit trim, width, FPS, quality, and loop validation; both stages emit scaled real progress and share process-tree cancellation.
- Added debounced real poster frames, an on-demand three-second local MP4 preview, and bounded completed-GIF preview. Preview process output is capped while streaming rather than checked only after unbounded buffering.
- Tuned Chat, Web, and High quality presets and replaced simple duration thresholds with duration/FPS/pixel-area guidance.
- Added a reproducible high-motion benchmark. The selected palette path measured SSIM 0.798505 / 504,092 bytes versus direct FFmpeg GIF at 0.783447 / 367,090 bytes on the recorded fixture. Gifski was license/package-reviewed but honestly not executed because it is absent and is AGPL by default.

Checks run:

- Rust unit tests: 18 passed.
- Real-engine integration tests: 10 passed / 0 failed / 0 mocked. GIF evidence covers trimmed MP4, MOV, and WebM inputs; animated probing; duration; width; forever and once loops; palette cleanup; poster decoding; and clip decoding.
- Frontend unit/accessibility tests: 7 passed, including the GIF workbench and dark-mode accessibility structure.
- Playwright visual checks: 2 passed, refreshing all required states and 100/125/150% scale captures.
- Frontend typecheck, lint, production build, Rust check, and all-target/all-feature Clippy: passed.

### Gate 6

- Added real inspection retry, failed-source session storage, and safe focus restoration. Failed inspection no longer becomes conversion-ready through a frontend-only state change.
- Added a versioned, path-free local preferences schema with migration readiness. Persistent collision defaults permit safe suffix or skip only; replace remains an explicit per-conversion warning.
- Added UUID-token recovery markers and startup cleanup that remove only exact, owned, regular, non-symlink Morflo partial/palette files. Added plausible free-space preflight based on decoded image/video work rather than compressed source size alone.
- Added a bounded 512 px, alpha-preserving image thumbnail generated by FFmpeg only for the selected source; no full-resolution browser decode or persistent thumbnail cache is used.
- Externalized presentation copy, added startup recovery feedback, and hardened keyboard deletion/settings focus behavior.
- Added a production privacy audit that allowlists runtime dependencies, scans frontend/Rust production sources for network APIs, and verifies the restrictive IPC-only CSP.
- Completed visual pass three. Browser axe found real light/dark contrast defects; token fixes now pass serious/critical checks across empty, queue, GIF dark, and error states. The 200% text-size flow remains operable.
- Added reproducible engine/frontend performance measurement. On this host: 88.1 ms cold probe, 60.1 ms warm mean, 145.4 ms to first useful engine progress, 524.9 ms and 133.5 MiB peak engine working set for 4096×3072 PNG → WebP, and <1 ms observed cancellation request latency. Browser/Vite cold/warm useful renders were 1314.4/160.2 ms and are not presented as packaged desktop startup.

Checks run:

- Rust unit tests: 23 passed, including recovery token validation, linked destination refusal, disk allowance, failed-source lifecycle, argument safety, and truthful progress.
- Real-engine integration tests: 10 passed / 0 failed / 0 mocked, now also covering a bounded alpha-preserving image thumbnail.
- Real process-tree cancellation: passed with no child process remaining.
- Frontend unit/accessibility tests: 12 passed.
- Playwright: 5 passed, covering required screenshots, DPI captures, four-state computed a11y, 200% text, and 20-item responsiveness.
- Frontend formatting, lint, strict typecheck, production build, Rust fmt, all-target/all-feature Clippy, and privacy audit: passed.

### Gate 7 — release-candidate hardening

- Added a manual, read-only GitHub Actions workflow for Ubuntu quality plus
  compile-only Windows x64, macOS arm64/x64, and Linux x64 jobs. It has no push
  trigger, artifact upload, credentials, or publish step and has not been run.
- Completed four separate product, design, engineering, and privacy/release
  red-team passes. Fixed misleading active-work copy, overclaimed Windows scope,
  private preview retention, Unix recovery-journal permissions/symlink handling,
  and missing deb notice configuration.
- Added acceptance evidence for scenarios A–J. At this checkpoint the packaged
  file-picker-to-completion interaction remained blocked by the Computer Use
  native pipe; the later Windows first-run milestone adds repeatable packaged
  startup-file-to-completion evidence without relabeling the picker gesture.
- Added reproducible Linux package smoke and offline real-engine scripts. Ten
  real conversions pass inside an isolated user/network namespace with Cargo
  offline.
- npm production audit reports no known vulnerabilities. Runtime npm licenses
  are MIT/Apache-2.0/ISC; recorded Cargo metadata has no missing license field.
  RustSec is not called passed because project-local `cargo-audit` installation
  could not complete from the available crates.io/cache state.

Current release-candidate checks:

- Frontend unit/accessibility tests: 12 passed; Playwright: 5 passed.
- Production frontend build: 257.58 kB JavaScript / 79.38 kB gzip.
- Rust unit tests: 24 passed; all-target/all-feature Clippy: passed with warnings denied.
- Real-engine integration: 10 passed / 0 failed / 0 mocked in 13.84 seconds.
- Real process-tree cancellation: passed; 0 ms observed at timer resolution and no child remained.
- Network-isolated real-engine integration: 10 passed / 0 failed / 0 mocked in 14.00 seconds.
- Privacy audit: passed across 25 production files and the IPC-only CSP.

### Final clean build and package

- Source: LF-only `git archive` of commit
  `f055876cc35416d114e874d4ebf54a2d35cb82de`; README CRLF count verified as zero.
- Frozen offline install: 371 packages reused / 0 downloaded. Deterministic
  fixture generation now runs first in `pnpm verify`; HEIC is an explicit
  capability skip.
- Clean frontend: formatting, lint, strict typecheck, 12 unit/accessibility
  tests, 5 Playwright checks, production build, privacy audit, and production
  npm advisory audit passed.
- Clean Rust: fmt, all-target/all-feature Clippy with warnings denied, 24 unit
  tests, 10 real-engine tests, and real process-tree cancellation passed using
  Rust 1.93.1 on WSL2.
- Clean release build: Tauri CLI 2.11.4 from the lockfile-matching Linux npm
  package; optimized Linux x64 binary and deb produced from WSL ext4 to preserve
  package modes.
- Artifact: `artifacts/linux-x64/Morflo_0.1.0_amd64.deb`, 3,679,346 bytes,
  installed size 10,837 KiB, SHA-256
  `DE2B056B2FF204B6F1736A13910D2499C261C8071A5F67E99FFEE504B4F68245`.
- Package inspection: Morflo 0.1.0 amd64, maintainer Snowy, 11,009,024-byte ELF,
  desktop entry, three 0644 icons, README, third-party notices, and engine
  distribution record. No FFmpeg/ffprobe/gifski blob or private build path.
- Package smoke: dependency resolution passed and the extracted app stayed
  alive for four seconds under WSLg until deliberately terminated. Mesa emitted
  the expected WSL GPU fallback warnings; no application failure was observed.
- This Linux clean-build checkpoint preceded the native Windows recovery below.
  No security policy was changed or bypassed in either path.

### Native Windows recovery and package

- An independent Agent Task Delegation review (`grok:a`, analysis-only,
  confidence 0.8) recommended native MSVC plus a strictly bounded
  delete-and-relink loop for only the exact generated artifact rejected by App
  Control. It explicitly rejected policy weakening, elevation, path trust,
  self-signing, cross-compilation as the primary route, and WSL as runtime
  architecture.
- Windows App Control events 3077/3033 identified policy
  `{0283ac0f-fff1-49ae-ada1-8a933130cad6}` rejecting newly linked Cargo build
  helpers and proc-macro DLLs with `os error 4551`. `scripts/build-windows.ps1`
  now retries within 40 rounds/45 minutes and may remove only the exact direct
  child under `target/release/build` or `target/release/deps` named by Cargo.
  It does not elevate, modify policy, trust a path, or touch source files.
- `pnpm package` completed natively with Rust/Cargo 1.97.1 and the
  `x86_64-pc-windows-msvc` target. The final NSIS run converged in one round.
- A Windows-only probe regression was found by real-engine testing:
  `std::fs::canonicalize` produced a verbatim `\\?\` path that made FFprobe
  classify APNG as single-frame `image2`. `dunce::canonicalize` preserves
  canonical resolution while returning an external-tool-compatible Windows
  path; the targeted regression and full real-engine suite pass.
- Final Windows checks: 24 Rust unit tests passed / 1 real-engine cancellation
  test ignored by default; 10 real-engine integration tests passed / 0 failed /
  0 mocked; native Windows process-tree cancellation passed in 667 ms; release
  all-target/all-feature Clippy passed with warnings denied.
- Frontend checks: formatting, lint, strict typecheck, 14 unit/accessibility
  tests, production build, privacy audit, and 5 Playwright visual/a11y/
  responsiveness checks passed.
- Final installer:
  `src-tauri/target/release/bundle/nsis/Morflo_0.1.0_x64-setup.exe`, 2,313,773
  bytes, SHA-256
  `74E08E6C82644EB92205E503EA29DE98E9149E6BE086332EC3A750D3DEAA021F`.
  It is unsigned and contains no media engine or private build path.
- The release EXE is 10,615,808 bytes with SHA-256
  `D1764AB666F9EAA72C255F55152E8968B844604915B21189F823847672EC94EA`.
- The NSIS installer completed silently with exit code 0 and installed
  the installed `morflo.exe`, SHA-256
  `AF2E5F4ACDB2378834A3C4322D221025021D3D5D26BF92C7BFDA06EF72C36900`.
  Its three-byte difference from the restored release EXE is exactly Tauri's
  documented bundle token patch from `UNK` to `NSS` at offset 7,935,538.
- Microsoft Defender custom scans reported zero detections for the release EXE,
  installer, and installed EXE. The installed process opened a responding
  `Morflo` WebView2 window. No Defender setting or App Control policy changed.
- IconFlow delivered and read back
  the Desktop shortcut `Morflo.lnk`: direct target is the installed
  native EXE, arguments are empty, and the icon is the immutable
  `shortcut-icon-52092a77e7df.ico`. Launching that exact shortcut produced a
  responding native process.
- The old WSL launcher processes, bridge files, generated Cargo experiment
  cache, and versioned WSL Morflo runtime were removed. No other WSL process or
  user media was touched.
- The first post-install shortcut launch reached a responding native window in
  1,843 ms. Three subsequent window-ready measurements were 142/78/67 ms
  (95.7 ms mean) at approximately 20.1 MiB main-process working set. This is a
  process/window readiness metric; it is not a full painted-content metric and
  OS caches were not flushed.
- The earlier Computer Use helper remained unavailable, but a narrower
  repeatable path now attaches Playwright to the real release WebView2 through
  an ephemeral process-local CDP endpoint. No production flag, registry value,
  automation plugin, or security-policy change is involved.
- The complete standard debug-profile `pnpm check` now passes, including
  all-target/all-feature Clippy with warnings denied. App Control can still
  transiently refuse a freshly linked test executable before its first run; an
  unchanged-hash retry is recorded rather than weakening policy.

### Windows first-run engine readiness milestone

- Reproduced the installed shortcut defect: the current shell resolved the
  WinGet FFmpeg 8.1.2 pair, while the desktop-launched process inherited an
  older Explorer `PATH` and reported a missing engine.
- Engine discovery is now refreshable and supplements inherited `PATH` with
  read-only current Windows machine/user `Path` values through `RegGetValueW`.
  It performs no shell invocation or registry write. FFmpeg and ffprobe must
  report the same version before their capabilities are accepted.
- Added a session-only engine-folder choice. The selected directory is probed
  before activation, never persisted, and never shown in ordinary UI copy.
- Replaced the dead-end engine warning with an accessible Diagnostics sheet:
  ready/missing status, source/version, eight runtime-derived output states,
  refresh, folder choice, privacy copy, and expandable/copyable technical
  details. Escape, focus containment/restoration, reduced motion, light/dark,
  and minimum-width behavior are covered.
- Added one-shot startup file intake for existing regular non-symlink files,
  capped at 512. It uses the normal inspection path and never scans folders.
- Added `pnpm test:native:windows`. The harness intentionally removes the one
  inherited FFmpeg `PATH` entry, launches the release EXE with
  `Native launch 京都 🧳 O'Reilly.png`, attaches to `http://tauri.localhost/`,
  and drives semantic controls.
- Installed-app result: capability summary 8/8; PNG → default JPEG completed in 3,693
  ms; source SHA-256 unchanged; pre-existing JPEG SHA-256 unchanged; suffixed
  output probed as MJPEG 16×2048; zero partial outputs. Ready, Diagnostics, and
  converted native screenshots were captured and visually reviewed.
- The first complete native conversion exposed a real finalization bug: this
  FFprobe reports JPEG as `image2`, while the old validator required
  `jpeg_pipe`. Verification now checks both container and primary codec for all
  outputs and accepts JPEG only as `image2`/`jpeg_pipe` plus `mjpeg`. Failure
  copy now truthfully says the incomplete partial was removed.
- Current checks: 24 Rust unit tests passed; 10 real-engine integration tests
  passed / 0 failed / 0 mocked; native process-tree cancellation passed in 667
  ms; 14 frontend unit/accessibility tests passed; 5 Playwright suites passed,
  including ready/missing Diagnostics captures and serious/critical axe rules.
- The first freshly linked real-engine test executable was transiently refused
  by App Control before any test ran (`os error 4551`); an unchanged-hash retry
  passed. No policy or trust setting was changed.

### Outcome-confidence design milestone

- Replaced the sparse terminal state with a conversion receipt based only on
  post-encode probe evidence: actual output name, format, bytes, alpha presence,
  dimensions, and duration. Absolute output paths remain backend-only.
- Added bounded output previews for images and video posters, plus the existing
  bounded GIF route. The backend retrieves only a job-owned output path,
  re-probes it as a regular non-symlink file, and enforces process, byte, and
  dimension limits before returning preview data.
- Added individual original → output receipts, honest smaller/similar/larger
  outcomes, saved-name and source-immutability reassurance, plus a selected-batch
  summary that keeps partial failure visible instead of celebrating over it.
- Completed three focused visual refinements across 1440×900, 1280×800,
  780×620, light, dark, partial-success, and Windows 150% scale evidence. The
  minimum-size Reveal action is scroll-reachable and covered by Playwright.
- Visual inspection caught a fixture-quality defect that ordinary probe tests
  missed: the synthetic PNG carried colored RGB but zero alpha everywhere.
  Fixture generation now writes the intended partial alpha, and the real-engine
  regression asserts both the warm flattened corner and preserved foreground
  color. The final native receipt therefore previews the actual visible result.
- Final checks: 25 Rust unit tests, 17 frontend unit/accessibility tests, 10
  real-engine integration tests / 0 mocked, native process-tree cancellation in
  583 ms, 7 Playwright tests, strict TypeScript, lint, Rust fmt/Clippy, privacy
  audit, and production build passed.
- The final installed release-WebView journey completed in 6,253 ms with an explicit
  alpha warning, 8/8 capabilities, international filename, immutable source and
  collision target, valid 640×420 MJPEG output, no partial residue, a decoded
  opaque output preview, and an honest 2.1 KB → 4.2 KB “106% larger” result.
- Direct Node launch of the newly linked workspace EXE was rejected by enforced
  Code Integrity events 3033/3077. The NSIS installer completed with exit code 0
  and the installed per-user EXE passed; no policy, trust, or Defender setting
  changed.
- Current unsigned Windows artifacts: release EXE 10,629,120 bytes, SHA-256
  `7C9F0336974513FC7CE279AF4CCD6A42027E6228C27A4258C9D9A6AF4E76E662`;
  NSIS installer 2,315,460 bytes, SHA-256
  `8ABF0B50943C2ECE73D3D1392FB429E0B0CA4CCE87B80C3999031BFAA98D5BAE`;
  installed EXE SHA-256
  `E93585BD0E6ED21B9C0BC9E80821C7371E2FE106B68D7DAAEBB3280478AFF52F`.
  Defender custom scans found no threats in all three.

### GIF moment-confidence milestone

- Replaced the decorative GIF range strip with seven real local source moments.
  The frontend sends only the session job ID; Rust resolves the source, uses
  accurate input seeks, returns 160×90 JPEGs capped at 256 KiB each, runs no
  more than two samplers, and applies one 20-second deadline. A missing strip
  never blocks conversion.
- Unified Start and End over the real frames while retaining two native range
  inputs. Dependent bounds prevent crossing, tab order remains stable, precise
  value text includes tenths, rail clicks move the nearest handle, and visible
  values stay outside the image. Forced-colors, focus, dark mode, and keyboard
  behavior are covered.
- Completed three screenshot refinements. They replaced non-informative art,
  fixed a `:has(.gif-inspector)` specificity defect that cropped the 780×620
  layout, and increased critical range-readout type after inspecting the real
  installed WebView2 result. Evidence includes GIF light/dark/narrow browser
  states and installed real-frame, animated receipt, and canceled states.
- Real-engine preview evidence: seven distinct 160×90 frames decode and probe;
  isolated sampling measured 394 ms and the full concurrent real suite measured
  572 ms. The same test still covers bounded poster, output poster/thumbnail,
  and a decoded three-second clip.
- Final installed GIF journey:
  `work/morflo/native-gif-windows/2026-08-30T13-21-19-615Z`. It removed the
  inherited FFmpeg `PATH` entry, decoded seven real moments in 1,898 ms, selected
  0.4–2.4 seconds, decoded the local clip, and produced a 463,498-byte, 540×304,
  24-frame, 2.0-second GIF. Source hash stayed unchanged, the output receipt
  decoded, and no partial remained; total journey time was 8,063 ms.
- Final installed cancellation journey:
  `work/morflo/native-cancellation-windows/2026-08-30T13-21-38-046Z`. A real
  38,342,014-byte, 30-second MP4 → WebM job stopped 1,390 ms after the visible
  Cancel action. The source hash stayed unchanged; no final output, recognized
  partial, or direct FFmpeg child remained.
- Current checks: strict formatting/lint/TypeScript/Rust fmt/Clippy passed; 20
  frontend tests and 25 Rust unit tests passed; 10 real-engine integration tests
  passed / 0 failed / 0 mocked; 8 Playwright visual/a11y/zoom tests passed.
  Before the two-lane preview-only optimization, the real process-tree test
  passed in 782 ms. Its freshly relinked final debug EXE was later refused before
  execution by Application Control (`os error 4551`), including unchanged-hash
  retries after a zero-threat Defender scan; the installed release cancellation
  journey above supplies final-artifact evidence without weakening policy.
- A deliberately parallel Cargo/Playwright check exposed Windows `EBUSY` in
  Vite's irrelevant watch of `src-tauri/target`. The dev server now ignores
  Rust artifacts and `work/` evidence; the complete eight-test E2E suite then
  passed alone. Normal release verification remains sequential.
- Final native image regression after the shared harness refactor passed in
  3,877 ms with 8/8 capabilities, international filename, immutable source and
  collision target, 640×420 MJPEG output, decoded opaque receipt, and no partial.
- Final unsigned Windows artifacts were built natively in one NSIS round and
  Defender reported no threats:
  - release EXE: 10,749,952 bytes, SHA-256
    `FA33C6B93F45BB7196D69BF71A16AAA7ED9FC3CB44A2D846AC41B699E5BB32A5`;
  - NSIS installer: 2,338,071 bytes, SHA-256
    `12A833BCC0FF0C3C2DBBFE51067EED7E662EE179244E74CBAE2B80AADE124C4A`;
  - installed EXE: 10,749,952 bytes, SHA-256
    `BE4C2063D8AD55F98E9E1913ED73D46B29D87564CB9F14A1C02D4E06747E387B`.
    All remain intentionally unsigned and contain no media engine.
- The IconFlow shortcut at
  the Desktop shortcut `Morflo.lnk` still resolves directly to the
  installed EXE with no arguments; its immutable custom ICO exists and reads
  back successfully.

### Restrained Windows Open with milestone

- Researched current Microsoft Default Programs, Open With, association, and
  Tauri single-instance/installer-hook guidance. Rejected Tauri's normal Windows
  `fileAssociations` output because the generated installer writes an extension
  default ProgID; Morflo must remain an alternate handler chosen by the user.
- Added a reviewed JSON source of truth and generated NSIS hooks for 13
  conservative extensions: PNG, JPG/JPEG, WebP, BMP, TIF/TIFF, AVIF, ICO, MP4,
  MOV, MKV, and WebM. The hooks register versioned `Morflo.Media.1`,
  `OpenWithProgids`, `RegisteredApplications`, `SupportedTypes`, and independently
  quoted commands. They never write an extension default or `UserChoice`.
- Added secure single-instance intake. Initial and later activations accept only
  existing regular non-symlink files, never scan folders, resolve later relative
  paths against that activation's directory, deduplicate one activation, and cap
  pending intake at 512. Later processes restore/focus the existing window, emit
  only a typed count, and exit.
- React subscribes before its first drain and serializes later drains. New rows
  use one restrained 180 ms opacity/4 px transition that collapses under reduced
  motion; no toast, fake progress, or Windows-only workspace was added.
- The reversible current-user installer test passed at
  `work/morflo/open-with-installer/2026-08-30T14-29-01-776Z`. Across install,
  uninstall, and reinstall, all 13 registrations appeared/cleared correctly;
  observed Photos, Photoshop, and VLC defaults plus Windows `UserChoice` stayed
  unchanged; uninstall left no Morflo-owned association; the existing IconFlow
  Desktop shortcut was restored byte-for-byte.
- The installed release handoff passed at
  `work/morflo/native-open-with-windows/2026-08-30T14-29-10-433Z`. PNG and MOV
  paths containing Chinese, Japanese, emoji, spaces, and an apostrophe arrived
  in the original window after 1,357 ms. A later relative transparent WebP
  resolved against its activation directory and received the alpha-preserving
  PNG recommendation with “Keeps transparency” rationale. Secondary processes
  exited cleanly, one Morflo process/WebView remained, all source hashes stayed
  unchanged, and three native screenshots were visually inspected.
- Final regression evidence on the same release: 23 frontend tests and 26 Rust
  unit tests passed; 10 real-engine tests passed / 0 failed / 0 mocked; 8
  Playwright visual/accessibility/zoom tests passed; native PNG → JPEG passed in
  5,370 ms; GIF moments/output passed in 2,375/6,788 ms; installed cancellation
  passed in 328 ms; direct real-process cancellation passed in 561 ms; privacy
  and production advisory audits passed.
- Native Explorer GUI control was attempted only after narrower verification,
  but its local control pipe returned OS error 2 on bounded retries. No click was
  performed. Registry lifecycle and exact installed-process invocation pass;
  the menu-selection gesture remains explicitly unverified.
- Final unsigned native artifacts after exact-file Microsoft Defender custom
  scans with antivirus and real-time protection enabled; zero matching new
  detections were reported:
  - release EXE: 10,791,936 bytes, SHA-256
    `A8D31F0020A98A1DB91859E152D26E591F6F99E7D33BF32142F2D19D53FB938B`;
  - NSIS installer: 2,353,788 bytes, SHA-256
    `885E06F8633383137EA161CCE66F176E6C38D1C194670AF9D8DEF5B4627BA2D5`;
  - installed EXE: 10,791,936 bytes, SHA-256
    `094261AAD070E2823ED7259AB90E8805D36BC452560936A2ECB28D4903EA6D57`.
    All are unsigned and contain no media engine.
- IconFlow re-delivered
  the Desktop shortcut `Morflo.lnk` after installer QA. It targets
  the installed EXE directly, has no arguments, uses the installed working
  directory, and references immutable
  `ShortcutAssets\shortcut-icon-52092a77e7df.ico` (SHA-256
  `52092A77E7DF48E29821903A6D6DC7BE6EA10DBDC95D69DDBD7236FD712826F9`).

### Private GitHub and control-plane checkpoint

- Created the private canonical repository
  `https://github.com/snowyukitty/morflo`, set `main` as the default branch,
  kept Issues enabled, disabled the Wiki, enabled vulnerability alerts, and set
  GitHub Actions' default token permission to read-only without pull-request
  approval authority. No release, artifact upload, visibility change, or
  binary publication was performed.
- Replaced duplicated workflow commands with one repository-owned
  `pnpm verify:quality` contract. It includes formatting, generated Windows
  Open With freshness, lint, strict TypeScript, Rust fmt/Clippy, frontend and
  Rust tests, Playwright accessibility/zoom journeys, frontend production
  build, privacy audit, and high-severity production dependency audit. The
  metered Windows/macOS/Linux compile matrix now waits for Ubuntu quality and
  duplicate runs on the same ref are canceled.
- Local `pnpm verify:quality` passed at commit `0aafdab`: 23 frontend tests, 26
  Rust tests, 8 Playwright tests, production build, 28-file privacy audit, and
  production advisory audit with no known vulnerabilities. It requires no
  media engine and is distinct from the already recorded 10-test real-engine
  and native Windows release evidence.
- Manual hosted run
  [33318068468](https://github.com/snowyukitty/morflo/actions/runs/33318068468)
  targeted exact commit `0aafdab637aa7d76d77353b9be0c38b79217d6b1`.
  The Ubuntu job was refused before any workflow step ran, for reasons external
  to this repository, and the dependent platform matrix was skipped. This is
  recorded as blocked, not as a failed or passed test.
- Re-ran IconFlow 0.5.0 against the current source and Tauri target contract.
  Automated checks reported no warnings; the source-bound review sheet was
  inspected at 16–256 px on light, dark, and mid-grey surfaces; ship passed
  with legibility/distinctiveness/balance/color/scalability/craft scores of
  4/4/5/5/4/5. `master-review.json` and the shipped case are now versioned;
  the seven generated icon outputs remained byte-identical.
- Registered Morflo in the owner's private control plane, recording the local
  checkout alias, publication intent, risk level, exact GitHub identity, and
  SHA-256-bound IconFlow outputs. Validation and render-freshness checks passed
  with zero errors; the only warnings were pre-existing and unrelated. The
  control plane is permanently private, so its repository name, commit
  identifiers, and registry contents are deliberately not reproduced here.

### Reviewed media-engine intake gate (implementation gate)

- Added manifest schema v1 and a shared Rust verifier for exact engine identity,
  target, provenance, configure line, declared license, executable/evidence
  hashes and sizes, complete decoder/encoder/demuxer/muxer/filter inventories,
  derived Morflo outputs, approval identity/date, and a maximum 366-day validity
  window. This is technical integrity evidence, not legal approval.
- A manifest cannot grant itself reviewed status. Reviewed packaging and runtime
  acceptance additionally require the exact manifest SHA-256 as an independent
  build input compiled into Morflo. Missing or changed pins, files, versions,
  configurations, capabilities, notices, source-offer evidence, or review dates
  fail closed. Untracked files, traversal, symlinks, and Windows reparse points
  are rejected.
- Automatic discovery treats a present invalid bundled directory as a terminal
  package failure instead of silently falling back. Explicit session folders,
  project development paths, and system discovery remain separately labelled.
- Current targeted evidence: 40 Rust tests, 7 Node operator-verifier tests, and
  24 frontend tests passed; strict formatting, lint, TypeScript, Rust fmt, and
  Clippy passed. Eleven real-engine integration tests—including automatic versus
  session discovery identity—and real process-tree cancellation passed against
  the current non-redistributable development engine. The standalone Rust
  verifier launch attempts in both debug and release profiles were refused
  before execution by the existing Windows App Control
  policy (`os error 4551`), so the operator command now uses the parity-tested
  Node implementation while the application retains the Rust runtime gate.
  `pnpm engine:verify` runs and rejects missing or unavailable explicit input
  with bounded path-redacted diagnostics. Final normal packaging evidence
  is recorded below.
- Native normal packaging was run with a deliberately inherited all-zero
  reviewed-manifest digest. The wrapper explicitly entered engine-free mode,
  cleared the inherited pin for the build, used no resource overlay, and
  completed the unsigned NSIS package in one round. Exact artifacts:
  - release EXE: 10,943,488 bytes, SHA-256
    `3F9A7C7802CF5D92E6969CBC972A70575BDF5585F320840752F7634536169FCA`;
  - NSIS installer: 2,404,212 bytes, SHA-256
    `6C7F678CDECCB6B8ADF6C25EFF263FD26F707960B18ADE8EE6401E084C3815F1`;
  - installed EXE: 10,943,488 bytes, SHA-256
    `1DBAF954F5D3048B7AB55DB467C9A55208B1467DE89412DFC9D49C1D9C8B2050`.
    All three remain intentionally unsigned.
- The reversible installer contract passed at
  `work/morflo/open-with-installer/2026-08-30T15-40-10-487Z`: all 13 alternate
  handlers survived install/uninstall/reinstall without changing observed
  defaults or `UserChoice`, uninstall removed only Morflo-owned registration,
  the Desktop shortcut was restored byte-for-byte, and Morflo was left
  installed.
- Engine-free inspection of the resulting installed tree found exactly four
  files: `morflo.exe`, `uninstall.exe`, and two `ShortcutAssets` icons. There
  was no `engines` directory, FFmpeg, ffprobe, engine manifest, or support file.
  Exact-file Microsoft Defender custom scans of the release EXE and installer
  reported zero matching new detections.

### Secure unreviewed engine-candidate dossier gate

- Added a separate schema-v1 dossier and `pnpm engine:dossier` command. Capture
  requires an explicit absolute candidate, an explicit new external JSON path,
  and `--allow-execution`; check mode requires the same candidate and an
  existing external dossier. Neither mode searches for candidates or makes a
  network request.
- The dossier is deterministic, path-neutral, timestamp-free, exclusively
  created, and hard-coded `unreviewed`. It records exact file sizes/SHA-256,
  matching ffmpeg/ffprobe version and configure line, compiler identity, seven
  structured FFmpeg library versions on the current candidate, complete sorted
  capability inventories, and derived Morflo outputs. It contains no automatic
  provenance, license, notice, source-offer, reviewer, approval, manifest, or
  trust-pin claim.
- Candidate files are bounded and hashed through stable handles; symlink,
  reparse, canonical escape, special file, empty directory, nonportable path,
  duplicate, depth/count/size, output overlap, overwrite, malformed probe, and
  pre/post-probe mutation conditions fail closed. Exact canonical bytes are
  required by later `--check`.
- All production Node/Rust FFmpeg and ffprobe launches now clear inherited
  environment state. They retain deterministic locale and minimal Windows
  OS/temp values, but no PATH, home, credentials, proxy settings,
  dynamic-loader overrides, or unrelated parent variables.
- Synthetic evidence: 17 Node engine-gate tests and 41 Rust unit tests passed,
  including explicit consent, fixed executable names, Unicode paths,
  determinism, exclusive output, stale/mutated candidate detection,
  dossier/manifest separation, symlink/reparse rejection, and sanitized
  environment assertions. Formatting, lint, TypeScript, Rust fmt, and Clippy
  passed.
- Real non-redistributable evidence: the local three-file Gyan candidate was
  hashed before and after probing; capture and `--check` produced the same
  temporary dossier SHA-256
  `c549cbb8d4b3a2d7f7e6c9bb32f8ea4f2caa899774b5800bb84ea21eb3e94e06`.
  It observed 557 decoders, 243 encoders, 366 demuxers, 184 muxers, 576 filters,
  eight Morflo outputs, seven FFmpeg libraries, and `ffplay.exe` as an
  unclassified additional file. The dossier was deleted and was never a
  packaging input. Eleven real conversion/discovery integrations and
  process-tree cancellation passed with the sanitized environment.
- Native normal packaging was rebuilt with a deliberately inherited all-zero
  reviewed-manifest pin. The wrapper cleared it, selected engine-free mode, used
  no resource overlay, and completed in one round. Current unsigned artifacts:
  - release EXE: 10,953,728 bytes, SHA-256
    `F1AAF1FFAC9C06DC60C296801F2A8575A29C58C644926A2C10F176296DF870FF`;
  - NSIS installer: 2,404,507 bytes, SHA-256
    `59DAC1D1291A47900785CA8ABBA239F4740F8447D95C5CEBC0BD5D4614F2D68E`;
  - installed EXE: 10,953,728 bytes, SHA-256
    `EC2921EA8EDFF5363F12654136244ECBE2765C005F1121A53B0AA443DEF13EA2`.
- Installer evidence at
  `work/morflo/open-with-installer/2026-08-30T20-40-07-487Z` again passed all 13
  alternate handlers, default/`UserChoice` preservation, owned uninstall, safe
  quoting, byte-for-byte Desktop shortcut restoration, and final reinstall.
  Installed-tree inspection found only Morflo, its uninstaller, and two icons;
  no engine, manifest, dossier, or support file was present.
- Current release-package journeys passed with the sanitized engine environment:
  PNG → JPEG with measured receipt, seven real GIF moments and a 24-frame GIF,
  30-second MP4 → WebM cancellation in 1,420 ms with no child/partial/final
  residue, and absolute/relative international Open-with handoff into one
  process/WebView. Exact-file Defender scans of the release EXE and installer
  completed with zero recent detections.

### Windows engine-process console suppression

- Windows GUI startup no longer permits FFmpeg/ffprobe capability probes to
  create visible console windows. The shared Rust engine-process constructor
  now applies `CREATE_NO_WINDOW`; startup probing, inspection, previews, and
  conversion all use that boundary. The conversion path retains its combined
  `CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW` mask for process-tree
  cancellation.
- A Windows-only behavioral regression test launches the current test binary
  through the production engine-process constructor, calls `GetConsoleWindow`
  in the child, and requires both a null console handle and intact captured
  stdout. The focused test passed, and the full Rust library suite passed 42 / 42.
- Clippy, direct Rust formatting, Prettier, generated Open-with freshness,
  ESLint, TypeScript, 24 frontend tests, 17 engine-intake tests, eight browser
  E2E tests, the frontend production build, privacy audit, and production npm
  advisory audit passed. Eleven real-engine integration tests passed, and real
  process-tree cancellation completed in 617 ms with the existing Windows
  process-group behavior intact.
- Native Windows engine-free packaging completed in one bounded round. The
  unsigned release executable is 10,953,728 bytes with SHA-256
  `7D7454A5658F221F4F87F2965D7390A1A808388ABE9C81809DF48A0AA8A61E49`;
  the unsigned NSIS installer is 2,403,302 bytes with SHA-256
  `D113181971B76F0E5F6BC9BC69B9C004A076014F3784BCA697A7C8B03AAE6D3F`.
- The first native WebView2 journey connected to the process-local debugging
  endpoint but did not enumerate the Morflo page, so it made no product
  assertion. An immediate retry against the unchanged release binary passed at
  `work/morflo/native-windows/2026-08-31T20-18-24-696Z`: engine diagnostics
  reported 8 / 8 outputs, PNG → JPEG completed with source and collision-target
  hashes preserved, the suffixed output probed as 640×420 MJPEG, the decoded
  receipt remained opaque, and no partial output remained.
- The current-user installer lifecycle then passed for all 13 Open-with
  handlers, preserving every observed default and `UserChoice`, safely quoting
  commands, removing only Morflo-owned registration during uninstall, and
  restoring the Desktop shortcut byte-for-byte before leaving the fixed build
  installed. The installed executable is 10,953,728 bytes, unsigned, and has
  SHA-256
  `767F9A8724108A63930259F82AB6703D9F69F13C97B7A8ECB313763B5F626D49`.
  The same native PNG → JPEG journey passed against that installed executable
  at `work/morflo/native-windows/2026-08-31T20-30-54-788Z` with diagnostics
  reporting 8 / 8 outputs and no source, collision-target, or partial-output
  regression.

### Destination writability and 0.1.1 maintenance release

- Fixed a destination validation defect reported from ordinary use: choosing
  `%USERPROFILE%\Downloads` failed with "The output folder is read-only".
  `validate_destination` trusted `Permissions::readonly()`, which on Windows
  reports `FILE_ATTRIBUTE_READONLY`. Windows sets that attribute on customized
  shell folders to mark a `desktop.ini` customization; it does not restrict
  writing to a directory. The check was unsound in both directions, because a
  folder closed by an access control entry carries no such attribute and
  therefore passed validation before failing mid-conversion.
- Replaced the attribute read with a create-and-remove probe in the destination
  folder, which answers writability for the account Morflo actually runs as.
  Measured against real paths with the same standard-library calls:

  | Folder                       | Attribute check | Probe check  | Truth        |
  | ---------------------------- | --------------- | ------------ | ------------ |
  | `%USERPROFILE%\Downloads`    | not writable    | writable     | writable     |
  | `%USERPROFILE%\Videos`       | not writable    | writable     | writable     |
  | `%USERPROFILE%\Documents`    | writable        | writable     | writable     |
  | `C:\Windows\System32\config` | writable        | not writable | not writable |

- Added a Windows regression test that carries the shell-folder attribute on a
  real directory and a Unix test for a folder that refuses new files, plus a
  test asserting the probe leaves no file behind.
- Updated dependencies to current compatible versions: ESLint 10,
  `@testing-library/jest-dom` 7, `@tauri-apps/plugin-dialog` 2.7.3,
  `lucide-react` 1.39.0, `typescript-eslint` 8.69.0, `globals` 17.12.0,
  `@types/node` 26.4.1, and the Cargo lockfile.
- TypeScript stays on 6.0.3. `typescript-eslint` 8.69 refuses to load against
  the TypeScript 7.0 API, so adopting TypeScript 7 would disable the lint gate
  rather than advance it. Upstream tracks support for TypeScript 7.1 and later.
- `eslint-plugin-jsx-a11y` 6.10.2 declares a peer range ending at ESLint 9.
  Its rules were confirmed to still report under ESLint 10 against a probe
  component covering `alt-text`, `anchor-is-valid`,
  `click-events-have-key-events`, and `no-static-element-interactions`.

Checks run:

- `pnpm check`: passed, including `cargo clippy` with `-D warnings`.
- `pnpm test`: passed 24 Vitest, the Node engine tests, and 44 Rust tests.
- `pnpm package`: succeeded after 1 round, producing unsigned
  `Morflo_0.1.1_x64-setup.exe`. The stale 0.1.0 installer had to be removed
  first because the wrapper requires exactly one Morflo NSIS installer in the
  bundle directory.
- Silent current-user install exited 0 and replaced the installed executable at
  the installed `morflo.exe` with reported file version
  0.1.1. The installed tree remains engine-free.
- The installer rewrote the Desktop shortcut `Morflo.lnk` and
  cleared its icon reference. The shortcut was restored to the immutable
  IconFlow icon `ShortcutAssets\shortcut-icon-52092a77e7df.ico`, whose SHA-256
  still matches the recorded
  `52092A77E7DF48E29821903A6D6DC7BE6EA10DBDC95D69DDBD7236FD712826F9`. Target,
  empty arguments, working directory, and em-dash description are unchanged.
- Launching that exact shortcut produced a responding native `Morflo` window at
  roughly 26 MiB working set.

### Built-in image engine milestone

- Morflo no longer requires a local media engine to do anything. A computer
  without FFmpeg previously could not inspect a single file, because
  `inspect_files` required `EngineService::runtime()` and that call returned
  `EngineMissing`. The release red-team recorded this as an accepted residual;
  ADR 0007 retires it.
- Added `src-tauri/src/image_engine.rs`, a second engine compiled into Morflo
  from permissively licensed crates. It writes PNG, JPEG and multi-resolution
  ICO, and reads PNG, JPEG, WebP, GIF, BMP, TIFF and ICO. It applies EXIF
  orientation, mirrors the media-engine resize modes and bounds, flattens alpha
  onto the chosen background for JPEG, and emits the same four-entry
  16/32/48/256 icon.
- `EngineRuntime` now holds `Option<MediaTools>`, and every subprocess caller
  asks through `EngineRuntime::media()`. The compiler enforces that no code path
  assumes FFmpeg exists. Discovery returns a native-only runtime with the reason
  instead of an error.
- A packaged engine that fails verification stays terminal and does not fall
  back to the built-in engine. Falling back would let a tampered bundle degrade
  into a working application and hide the failure.
- `ConversionPlan` carries a `ConversionProgram` of either media-engine stages
  or one native task, so execution, progress and verification can see which
  engine ran rather than inferring it from an empty stage list.
- With a media engine present, every image journey still runs through it. All
  existing real-engine acceptance evidence therefore continues to apply
  unchanged.
- WebP and AVIF are deliberately not offered natively. The available pure-Rust
  WebP encoder is lossless only, so a photograph would grow rather than shrink
  and the quality control could not act. Both report that they need a media
  engine, with the reason.
- Preserving source metadata is refused on the built-in engine with its reason
  and remedy, not downgraded to Remove.

Checks run:

- `pnpm verify:quality`: passed. 24 Vitest, the Node engine tests, 54 Rust unit
  tests, 9 engine-free integration tests, 8 Playwright end-to-end tests,
  `cargo clippy` with `-D warnings`, the privacy audit, and
  `pnpm audit --prod --audit-level high` with no known vulnerabilities.
- `pnpm test:real`: passed 11 real-engine tests plus the real process-tree
  cancellation gate, against FFmpeg/ffprobe 8.1.2. This is the regression
  evidence that the media-engine path is unchanged by this milestone.
- `src-tauri/tests/native_engine.rs` runs against `EngineRuntime::native_only`,
  the exact runtime a computer without FFmpeg receives, and is not
  feature-gated. It proves engine-free PNG to JPEG, engine-free favicon
  creation with all four icon entries, collision-safe naming beside an existing
  output, alpha flattening onto the chosen background, an immutable source, a
  video declined with an actionable reason instead of a crash, a refused
  metadata request, an animated source still requiring an explicit choice, and
  capabilities that explain every gap.
- Rust dependency licenses were re-resolved: 501 registry packages, none missing
  a declared license field, and every crate added for the built-in engine is
  MIT, Apache-2.0, BSD or Unlicense. None is copyleft, so the FFmpeg
  redistribution constraint does not extend to them. Recorded in
  `THIRD_PARTY_NOTICES.md`.

Release 0.2.0 evidence:

- `pnpm package`: succeeded after 1 round, producing unsigned
  `Morflo_0.2.0_x64-setup.exe` at 2,861,152 bytes.
- Silent current-user install exited 0. The installed executable reports file
  version 0.2.0 at the installed `morflo.exe`.
- `pnpm package:inspect-engine-free` against the installed directory passed with
  exactly four files: `morflo.exe`, `uninstall.exe`, and the two `ShortcutAssets`
  icons. The built-in engine is compiled into the executable and adds no
  packaged binary, so the installed tree is still engine-free.
- The three installed-app journeys pass against 0.2.0 with exit code 0. The
  image journey converts to a collision-suffixed international filename with an
  unchanged source hash, a preserved pre-existing output, a correct 640x420
  receipt and a rendered opaque output preview in 2,244 ms. The GIF journey
  keeps its source hash with zero partial residue. Real cancellation stops the
  encoder in 1,395 ms with zero surviving FFmpeg children and zero partials.
- The installer rewrote the Desktop shortcut and cleared its icon reference
  again. It was restored to the immutable IconFlow icon, whose SHA-256 still
  matches `52092A77E7DF48E29821903A6D6DC7BE6EA10DBDC95D69DDBD7236FD712826F9`,
  with target, empty arguments, working directory and em-dash description
  unchanged.

Decode-budget fix and 0.2.1:

- Writing a decompression-bomb test exposed that the built-in engine's decode
  allowance never fired. `image::Limits::max_alloc` was set but does not reach
  the PNG decoder's buffers; its `set_limits` checks dimensions only and carries
  an upstream TODO about constraining internal allocation. The dimension guard
  was therefore the sole protection, and 100,000 x 100,000 RGBA is roughly
  40 GB, declarable by a PNG of a few hundred bytes.
- Morflo now enforces its own budget against the header before materializing any
  surface, refusing an oversized declaration with a message naming the media
  engine, which streams such an image instead of holding it whole. A regression
  test builds a real 50,000 x 50,000 PNG bomb, asserts the refusal comes from
  Morflo's budget rather than an incidental parse failure, and checks the
  reserved partial stays empty. A companion test fixes the boundary at a
  16,000 x 12,000 scan.
- `pnpm check` and `pnpm test` pass with 57 Rust unit tests, 24 Vitest tests and
  9 engine-free integration tests. `pnpm test:real` passes 11 real-engine tests
  plus the cancellation gate.
- `pnpm package` produced unsigned `Morflo_0.2.1_x64-setup.exe`; silent install
  exited 0; the installed executable reports 0.2.1; engine-free inspection still
  finds exactly four files; and all three installed-app journeys exit 0.

### Gate 9 — legacy FFmpeg backward compatibility, native fallback, and 0.2.2

- Diagnosed and resolved the `stream_side_data` probe failure on host machines running
  FFmpeg/FFprobe < 6.0 (e.g., 5.0.1). FFprobe prior to 6.0 does not recognize
  `stream_side_data` and `frame_side_data` sections in `-show_entries` and exits with
  `Invalid argument`.
- Added dynamic probe retry with `SHOW_ENTRIES_LEGACY` in `run_ffprobe` to preserve
  complete metadata extraction across both modern (6.x+) and legacy (4.x/5.x)
  FFprobe builds.
- Added native image inspection fallback: if external media engine inspection fails or
  returns invalid output for supported image formats, Morflo falls back to pure-Rust
  `native_inspection`, ensuring zero-dependency resilience for images and preventing
  blocked format selection or conversion failures.
- Added engine-aware frame synchronization flag selection (`-vsync vfr` for legacy
  FFmpeg < 6.0 and `-fps_mode vfr` for modern builds), preventing video conversion
  failures on legacy engines.
- Adapted fixture generation (`scripts/generate-fixtures.ps1`) to probe for AVIF
  muxer and `-fps_mode` availability dynamically, preventing fixture generation
  failures on older FFmpeg installations.
- Updated `scripts/test-open-with-installer-windows.mjs` to resolve the current installer
  version dynamically from `package.json`.
- Verified clean across the complete quality and release contract:
  - `pnpm verify:quality` passed: 58 Rust unit tests, 9 engine-free integration tests,
    8 Playwright E2E tests, 24 Vitest tests, 17 engine verifier tests, strict
    formatting/lint/typecheck, privacy audit (32 files), and zero npm advisories.
  - `pnpm test:real` passed: 11 real-engine integration tests against host FFmpeg 5.0.1,
    including real process-tree cancellation (694 ms).
  - All four native Windows release journeys passed: `test:native:windows` (3,357 ms),
    `test:native:gif:windows` (3,682 ms), `test:native:cancel:windows` (1,879 ms),
    `test:native:open-with:windows` (887 ms arrival).
  - Installer Open with lifecycle test passed: install, uninstall, and reinstall across
    all 13 conservative extensions with byte-for-byte Desktop shortcut restoration and
    `UserChoice` preservation.
  - Packaged unsigned Windows installer `Morflo_0.2.2_x64-setup.exe` in 1 round (50.55s),
    silently installed and verified installed `morflo.exe` reports version 0.2.2.

Evidence boundary:

- The engine-free path is proven by the non-gated integration suite running the
  real `native_only` runtime, not by an installed GUI session with no engine
  present. That gesture was not performed on this host: Morflo's discovery
  deliberately reads the persistent machine and user registry `Path`, where this
  computer has a WinGet FFmpeg, and removing or renaming that installation to
  stage the state would mutate the owner's system. This is a QA evidence gap,
  not a claim that the installed engine-free session was observed.

## Next

Choose the root application license. This is now the first blocker rather than
one of several, because Morflo can already ship a working image converter
without resolving the media-engine question at all.

Commission an owner/legal review of one exact redistributable FFmpeg/ffprobe
bundle, then create and pin its first real schema-v1 manifest. The dossier
command can supply unreviewed technical observations to that review but cannot
make or automate the decision. This now unblocks video, animated GIF, WebP and
AVIF rather than the whole application.

Two follow-ups are open from ADR 0007 and are deliberately separate decisions:
whether the built-in engine should become the default for images even when a
media engine is present, which needs comparative output and speed evidence
first; and whether a pure-Rust lossy WebP or AVIF encoder can be added without
offering a quality control that cannot act.

Signing and broader platform QA follow the license decision.

## Known risks

- The available FFmpeg build is GPLv3 full, so it is local test evidence rather
  than a redistributable release sidecar. The new gate has no approved real
  manifest or compiled digest. Since ADR 0007 this no longer blocks the common
  image journeys, which run on Morflo's own redistributable built-in engine, but
  video, animated GIF, WebP and AVIF still depend on a local media engine.
- Dossier probing intentionally executes the exact candidate after explicit
  acknowledgement but does not sandbox it. Unknown candidates still require an
  appropriately isolated external review environment.
- `glib 0.18.5` carries `RUSTSEC-2024-0429`, reported by GitHub Dependabot as
  `GHSA-wrw7-89jp-8q8g`. This is now scoped rather than assumed:
  `cargo tree --target x86_64-pc-windows-msvc -i glib` finds nothing, so the
  crate is absent from the Windows dependency graph and cannot affect the
  primary release target. It reaches the tree only on Linux, through
  `glib -> atk -> gtk -> muda -> tauri`. The first patched `glib 0.20.0` is
  still outside that GTK major line, so a Linux release needs an
  upstream-compatible upgrade or an explicit owner assessment of Tauri's
  unmaintained GTK 3 bindings.
- HEIC/HEIF remains unverified until a legal synthetic fixture and decoder path are available.
- Packaged semantic GUI automation is currently Windows-only. macOS and
  physical Linux packaged click paths remain unrun.
- The Windows association lifecycle and exact command handoff are automated,
  but the Explorer menu-selection gesture remains unrun because native desktop
  control was unavailable on this host.
- Windows package construction is proven through the bounded recovery path on
  this host, but its generated executables are unsigned and the standard fresh
  debug-profile quality command can still be rejected by enforced App Control.
- The unsigned Linux package does not bundle a media engine; normal packaged
  conversion still depends on a compatible local engine until distribution and
  license decisions are resolved.
- The manual cross-platform workflow was dispatched but refused before any step
  ran, for reasons external to this repository. Hosted Windows, macOS and Linux
  compilation therefore remains unvalidated, and macOS has never been built and
  has no runtime evidence.
