# Morflo development

Build and verify Morflo with the checked-in lockfiles. Common image conversion
works without FFmpeg; real-engine fixtures and video tests require a compatible
local FFmpeg/ffprobe pair.

## Development setup

Required:

- Node.js 24 and pnpm 11.17.0
- Rust 1.93.1 or newer with the target platform toolchain
- Tauri 2 platform prerequisites
- Optional: a compatible `ffmpeg` and `ffprobe` pair for video, GIF, WebP/AVIF output, and real-engine checks

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
pnpm audit:all             # development + production advisories at high severity
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
./scripts/smoke-linux-package.sh /absolute/path/to/Morflo_<version>_amd64.deb
```

Fixture generation is deterministic and synthetic; no third-party media is
downloaded or committed.

## Quality checkpoint

Run `pnpm verify:quality` for formatting, lint, types, Rust, unit/integration and
browser checks, frontend build, privacy audit, and production dependency audit.
Run `pnpm verify` before a release checkpoint. Native Windows journeys require
a current release executable and generated fixtures.

The manual `quality-and-platforms.yml` workflow calls the same quality contract,
then compiles Windows, Linux and both macOS architectures. It does not upload
or publish binaries. Dispatch only when the result will inform a decision.
Compilation alone is not packaged runtime validation.

See [architecture](architecture/overview.md), [acceptance](quality/acceptance.md),
[release checklist](quality/release-checklist.md), and
[media-engine distribution](legal/media-engine-distribution.md).
