# Morflo

Morflo is a calm, local-first desktop converter for common image and video work.
Its primary flow is deliberately short: drop files, understand the recommended
result, convert, then reveal the output.

The focused video-to-GIF workbench shows seven real local source moments on one
accessible Start/End rail, offers a bounded clip preview, and returns a measured
animated-output receipt without becoming a timeline editor.

On Windows, the installer adds Morflo as a restrained **Open with** candidate
for 13 verified/common media extensions. It never takes over a file type or
changes the user's existing default. Files opened while Morflo is already
running join the same inspected queue instead of opening another window.

Morflo is not a cloud converter, media editor, document converter, downloader,
or codec control panel. It has no accounts, analytics, advertising,
subscriptions, crash uploads, or conversion-time network dependency.

## Privacy and output safety

- File contents, names, thumbnails, metadata, and paths stay on the device.
- Source files are immutable.
- The default collision policy appends a numeric suffix; replacement is never
  silent.
- Work is written beside the destination as an owned `.morflo-part` file and is
  published only after successful encoding and output probing.
- Recognized abandoned partials are tracked in a private, bounded recovery
  journal and cleaned on the next launch.
- The webview has no arbitrary shell or filesystem permission, and its CSP has
  no external network destination.
- Queue history and preview media are session-only. Only the selected preview is
  kept in memory.
- Completion receipts use the re-probed final artifact for format, bytes,
  dimensions, duration, and alpha state. Opaque outputs never receive a
  transparency-style preview background.

## Verified development-engine matrix

“Verified” means the real FFmpeg/ffprobe 8.1.2 development engine converted a
locally generated fixture and Morflo probed the result. It does **not** mean an
engine is bundled or legally cleared for redistribution.

| Input                         | Output                      | Evidence status                        |
| ----------------------------- | --------------------------- | -------------------------------------- |
| PNG                           | JPEG, WebP, AVIF, ICO       | Verified, including alpha decisions    |
| JPEG                          | PNG, WebP, ICO              | Verified                               |
| WebP                          | PNG, JPEG, ICO              | Verified, including alpha              |
| BMP, TIFF, ICO                | PNG/JPEG/WebP as applicable | Verified                               |
| AVIF                          | PNG, JPEG, WebP             | Verified, including alpha round-trip   |
| HEIC/HEIF                     | Common image outputs        | Capability-dependent; not verified     |
| MOV, MKV, WebM                | MP4                         | Verified                               |
| MP4                           | WebM                        | Verified                               |
| MP4, MOV, WebM                | Animated GIF                | Verified with trimmed palette pipeline |
| Other listed video containers | MP4/WebM                    | Capability-dependent; not verified     |

See the [conversion matrix](docs/media/conversion-matrix.md) for exact fixture,
stream, transparency, duration, cancellation, and batch evidence.

## Development setup

Required:

- Node.js 24 and pnpm 11.17.0
- Rust 1.93.1 or newer with the target platform toolchain
- Tauri 2 platform prerequisites
- A compatible `ffmpeg` and `ffprobe` pair for inspection and conversion

Morflo accepts a bundled sidecar only when its manifest, independent compiled
digest pin, files, exact versions, build configuration, full capability
inventory, notices, source-offer evidence, and time-bounded approval all pass
offline verification. A present invalid bundle is a package failure; automatic
discovery does not silently fall through to a development engine. When no
bundle is present, Morflo checks explicit project development paths and then a
compatible engine on `PATH`. On Windows it also reads the current persistent
machine/user `Path` values so a desktop shortcut is not stranded with
Explorer's older inherited environment. Engine Diagnostics can check again or
validate a fixed-name `ffmpeg`/`ffprobe` pair from a folder for the current
session; the selected location is not persisted or represented as reviewed.
Developers may set `MORFLO_FFMPEG_PATH` and `MORFLO_FFPROBE_PATH` for an explicit
development pair. The frontend never chooses executable names, supplies engine
arguments, or infers output support from extensions alone. Media-engine child
processes receive a deliberately small environment without inherited search
paths, homes, tokens, proxy settings, or unrelated application variables.

```powershell
pnpm install --frozen-lockfile
pnpm generate-fixtures
pnpm tauri:dev
```

Useful commands:

```powershell
pnpm check                 # formatting, lint, TypeScript, Rust fmt + Clippy
pnpm test                  # frontend and Rust unit tests
pnpm test:real             # ignored-by-default real-engine tests; no mocks
pnpm test:e2e              # visual, accessibility, zoom, and responsiveness
pnpm engine:verify --dir C:\absolute\reviewed-bundle
                            # offline, explicit-directory intake verification
pnpm engine:dossier --dir C:\absolute\candidate `
  --out C:\absolute\evidence\candidate-dossier.json --allow-execution
                            # deterministic, explicitly unreviewed candidate evidence
pnpm test:native:windows   # release EXE + native WebView2 + real conversion
pnpm test:native:gif:windows    # real moments, range preview, GIF, and receipt
pnpm test:native:cancel:windows # real release encode + process-tree cleanup
pnpm test:native:open-with:windows    # real second-process file handoff
pnpm test:installer:open-with:windows # reversible registry install/uninstall audit
pnpm build                 # frontend plus target-platform release binary
pnpm package               # unsigned NSIS package on a compatible Windows host
pnpm package:inspect-engine-free --dir C:\absolute\installed\Morflo
                            # inspect an installed normal package tree
pnpm measure:performance   # local engine and frontend observations
pnpm audit:privacy         # production source, dependencies, and CSP audit
pnpm audit:prod            # production dependency advisories at high severity
pnpm verify:quality        # reproducible gate; no media engine required
pnpm verify                # full local gate, real engine, and release build
```

### Candidate dossier boundary

Before legal or distribution review, an operator can capture a deterministic
technical dossier without constructing a reviewed manifest:

```powershell
pnpm engine:dossier `
  --dir C:\absolute\candidate `
  --out C:\absolute\evidence\candidate-dossier.json `
  --allow-execution

pnpm engine:dossier `
  --dir C:\absolute\candidate `
  --check C:\absolute\evidence\candidate-dossier.json `
  --allow-execution
```

The command requires exact root `ffmpeg`/`ffprobe` names, inventories and hashes
every regular file, records matching versions, configure line, compiler and
library versions, and complete observed capabilities, then repeats the file
inventory after probing. The JSON contains no absolute path or timestamp, is
written only to an explicit new file outside the candidate, and is permanently
marked `unreviewed`. It is a review input, not a manifest generator, trust pin,
notice classifier, source offer, license conclusion, or packaging input.

`--allow-execution` is intentionally mandatory: observing capabilities runs the
candidate executables. Morflo supplies fixed arguments and a sanitized child
environment, but does not claim to sandbox an unknown binary or prevent that
binary from using the current user's OS permissions. Run it only after deciding
that executing the exact candidate is appropriate in the current environment.

On Windows, `pnpm package` uses a bounded App Control recovery wrapper. It may
relink only the exact Cargo-generated build helper or proc-macro artifact named
in an error, and only under `src-tauri/target/{release/build,release/deps}`. It
does not elevate, weaken policy, trust a directory, or modify source files.
Normal packaging clears any inherited reviewed-manifest pin and has no engine
resource input, so it remains engine-free. A future reviewed bundle can be
staged only through the explicit opt-in path below; the digest must come from a
separate review of the exact manifest, not be copied automatically from the
candidate during packaging:

```powershell
pnpm engine:verify --dir C:\absolute\reviewed-bundle
pnpm package:reviewed-engine `
  --dir C:\absolute\reviewed-bundle `
  --expected-manifest-sha256 <64-lowercase-hex-reviewed-digest>
```

The opt-in command verifies before and after copying only manifest-declared
files into an ephemeral Cargo target directory, embeds the digest pin, and adds
that directory through a temporary Tauri resource overlay. It never searches
`PATH`, WinGet locations, or other folders for packaging input. This technical
gate does not approve a distributor, license, codec patent position, source
offer, or release.

On a Linux host or WSL checkout with Linux dependencies installed:

```bash
MORFLO_FFMPEG_PATH=/absolute/path/to/ffmpeg \
MORFLO_FFPROBE_PATH=/absolute/path/to/ffprobe \
./scripts/test-offline.sh

pnpm tauri build --bundles deb
./scripts/smoke-linux-package.sh /absolute/path/to/Morflo_0.1.0_amd64.deb
```

Fixture generation is deterministic and synthetic; no third-party media is
downloaded or committed.

## GitHub checkpoint workflow

The private canonical repository uses a deliberately manual workflow so
metered Windows and macOS runners are spent only when their result will change a
decision. The Ubuntu quality job runs the same `pnpm verify:quality` contract as
local development. Platform jobs start only after that gate passes, compile
Windows x64, macOS Apple Silicon/Intel, and Linux x64 without bundling, and
never publish or upload binaries.

From an authorized checkout:

```powershell
gh workflow run quality-and-platforms.yml --ref main
gh run watch --exit-status
```

Hosted compilation is evidence that source compiles for a target; it is not a
claim that the application was packaged or interactively validated on that
operating system.

## Media-engine and licensing boundary

No FFmpeg, ffprobe, gifski, or libvips binary is committed. The observed Gyan
FFmpeg 8.1.2 `full_build` enables GPL components and reports GPLv3-or-later; it
is used for non-redistributable local validation only and is not an eligible
packaging input. Candidate dossier schema v1 records unreviewed observations;
manifest schema v1 and the fail-closed intake gate establish technical
readiness. Neither is legal approval. A public normal-use package remains
blocked until the owner chooses an application-license strategy and an exact
redistributable engine build receives provenance, configuration, checksum,
codec, patent, source-offer, notice, and distribution review.

See [media-engine distribution](docs/legal/media-engine-distribution.md) and
[third-party notices](THIRD_PARTY_NOTICES.md).

## Packaging and platform validation

| Platform        | Actual evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Windows 11 x64  | Native MSVC app and unsigned NSIS installer built on Windows 11 x64, Defender-scanned with zero detections, installed per-user, and verified through the delivered IconFlow shortcut. Ten real conversion cases plus the engine-discovery identity regression pass. Installed release-WebView journeys cover PNG → JPEG confidence, a real seven-frame GIF moment strip through decoded GIF receipt, 30-second MP4 → WebM cancellation, and single-window Open with intake. Installer evidence proves 13 alternate-handler registrations without changing existing defaults or `UserChoice`. |
| Linux x64       | Unsigned `.deb` built and smoke-launched under WSL2 Ubuntu/WSLg. This is local WSL evidence, not a broad physical-Linux compatibility claim.                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| macOS arm64/x64 | Manual CI build structure exists; no build or runtime validation has been performed.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |

The primary local artifact is
`src-tauri/target/release/bundle/nsis/Morflo_0.1.0_x64-setup.exe`. The Linux
artifact remains at `artifacts/linux-x64/Morflo_0.1.0_amd64.deb`. Neither
contains a media engine; both are unsigned and are not authorized for public
distribution. Windows public distribution needs a trusted code-signing
certificate; macOS needs Apple signing and notarization credentials.

## Known limitations

- Normal packaged conversion currently depends on a compatible locally
  discoverable engine because no release sidecar is approved.
- No reviewed manifest digest or media-engine binary is committed. The opt-in
  packaging path is intentionally unusable until an exact bundle is separately
  reviewed and supplied.
- HEIC/HEIF is not verified on the observed engine.
- HDR preservation is not guaranteed; detected HDR and unsupported streams are
  warned before conversion.
- ICC/EXIF preservation is best-effort across formats, not byte-identical.
- Hardware encoders, audio extraction, deeper Explorer actions, automatic
  updates, and custom reusable presets are intentionally deferred.
- The Windows installer registers only a conservative Open with candidate set:
  PNG, JPEG, WebP, BMP, TIFF, AVIF, ICO, MP4, MOV, MKV, and WebM. HEIC/HEIF and
  unverified video containers are not advertised. The actual Explorer menu
  selection gesture remains manually unverified on this host because the
  desktop-control pipe was unavailable; registry lifecycle and real command-line
  handoff through the installed app are automated and pass.
- Native packaged automation is currently Windows-only. macOS and physical
  Linux click-path validation remain unrun.

## Architecture

The frontend sends typed conversion intent. Rust owns validation, planning,
process execution, truthful progress, resource-aware scheduling, cancellation,
safe output finalization, recovery, and capability discovery. See the
[architecture overview](docs/architecture/overview.md) and current
[status](docs/STATUS.md).

## License

Morflo is dual licensed under either [Apache-2.0](LICENSE-APACHE) or
[MIT](LICENSE-MIT), at your option — the customary Rust-ecosystem license, and
the same terms as every crate Morflo compiles. See [LICENSE.md](LICENSE.md).

This covers Morflo's own source, including its built-in image engine. It does
not cover FFmpeg: Morflo runs a locally installed `ffmpeg`/`ffprobe` as external
programs and ships no media-engine binary, so an FFmpeg build's own terms bind
whoever distributes that build. Third-party components are recorded in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
