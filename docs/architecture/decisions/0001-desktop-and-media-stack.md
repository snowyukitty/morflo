# ADR 0001: Desktop and media stack

- Status: Accepted
- Date: 2026-08-30

## Decision

Use Tauri 2, React 19 with strict TypeScript, Vite, and a Rust backend. Use FFmpeg/ffprobe behind an `EngineAdapter` as the v0.1 image and video engine. Use FFmpeg `palettegen` + `paletteuse` as the v0.1 GIF pipeline. Discover engines in this order: reviewed bundled sidecar, project-local development engine, compatible user installation.

## Why

- Tauri provides a small native shell, Rust command boundary, restrictive capabilities, CSP, and platform installers without a local HTTP server.
- Rust makes validation, typed errors, process lifecycle, atomic output handling, and tests independent of webview behavior.
- One probed engine produces a coherent first vertical slice and avoids shipping a second native runtime before image packaging is proven.
- libvips is technically strong for large image throughput and memory use, but the current machine lacks it and its optional codecs create a separate cross-platform native packaging matrix.
- gifski has excellent quality goals but uses AGPL-3.0-or-later by default and its direct video path complicates FFmpeg linkage. Selecting it now could prematurely constrain the owner's project-license choice.

## Consequences

- FFmpeg image metadata and color-profile behavior must be documented conservatively and tested; Morflo does not promise perfect profile preservation.
- Capability availability varies with the selected binary and is reflected at runtime.
- A public installer needs a reviewed engine build, corresponding notices/source obligations, codec/patent review, checksums, and signing. The local GPL full build is not silently redistributed.
- `ImageEngineAdapter` remains replaceable by libvips in a future release if benchmarks justify the packaging cost.

## Rejected alternatives

- **Electron:** larger runtime and broader attack/dependency surface without a critical benefit for this workflow.
- **Frontend process construction:** violates the trust boundary and makes argument injection easier.
- **Bundled arbitrary FFmpeg download:** weak provenance and license evidence.
- **Native Rust codecs only:** incomplete HEIF/video support and a fragmented capability story for v0.1.
