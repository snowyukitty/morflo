# Third-party notices

Morflo's own source is dual licensed `MIT OR Apache-2.0`; see [LICENSING.md](LICENSING.md). This file records third-party components used by the source tree or local validation, under their own terms.

## Runtime source dependencies

The locked production npm graph was audited locally on 2026-08-30. Its runtime
packages declare only MIT, Apache-2.0, and ISC terms:

- Tauri JavaScript API and dialog plugin — Apache-2.0 OR MIT
- React, React DOM, and Scheduler — MIT
- Lucide React — ISC (glyphs are interface aids, not the Morflo identity)

The locked Rust graph contains 501 registry packages for the recorded
all-platform metadata resolution and no missing declared license field. Direct
runtime crates include Tauri, serde/serde_json, SHA-2, Tokio/tokio-util, thiserror,
base64, UUID, which, dunce, libc, windows-sys, the Tauri single-instance
plugin, and the `image` crate under their individual lock-resolved terms. `tauri-plugin-single-instance`
declares Apache-2.0 OR MIT. `dunce` declares CC0-1.0 OR MIT-0 OR Apache-2.0.
`src-tauri/Cargo.lock` and `pnpm-lock.yaml` remain the exact component inventories;
this summary does not replace their upstream license texts.

## Built-in image engine

Morflo compiles its own image engine from `image` 0.25 and that crate's decoder
and encoder graph, so common image conversion works with no external media
engine installed. Unlike FFmpeg, every one of these components is permissively
licensed and can be redistributed inside Morflo. The terms resolved on
2026-09-03 were:

| Component                               | License                              |
| --------------------------------------- | ------------------------------------ |
| `image`                                 | MIT OR Apache-2.0                    |
| `image-webp`                            | MIT OR Apache-2.0                    |
| `zune-jpeg`, `zune-core`                | MIT OR Apache-2.0 OR Zlib            |
| `gif`, `weezl`                          | MIT OR Apache-2.0                    |
| `half`, `byteorder-lite`                | MIT OR Apache-2.0 / Unlicense OR MIT |
| `tiff`, `fax`, `color_quant`, `crunchy` | MIT                                  |
| `quick-error`                           | MIT/Apache-2.0                       |
| `moxcms`, `pxfm`                        | BSD-3-Clause OR Apache-2.0           |
| `zerocopy`, `zerocopy-derive`           | BSD-2-Clause OR Apache-2.0 OR MIT    |

None of these is copyleft, so the redistribution constraint recorded below for
FFmpeg does not apply to them. `src-tauri/Cargo.lock` remains the exact
inventory; this table does not replace the upstream license texts, which must be
reproduced in the release package before public distribution.

## Development/test dependencies

- Vite — MIT
- Vitest — MIT
- Testing Library packages — MIT
- Playwright — Apache-2.0
- axe-core — MPL-2.0

## External media engines

FFmpeg and ffprobe are executed as external programs when a compatible
installation is discovered. Since ADR 0007 they are optional: without them
Morflo converts images on its built-in engine, and video, animated GIF, WebP and
AVIF report that they need a media engine. They are not committed to this repository. The observed local development build reports GPLv3-or-later because GPL components are enabled. See `docs/legal/media-engine-distribution.md` for exact local evidence and release obligations.

Morflo includes a versioned technical intake schema and offline verifier for a
future reviewed sidecar, but no sidecar manifest digest or media-engine binary
is approved, committed, or included at this checkpoint. A verifier pass would
not itself establish license, patent, commercial-use, or redistribution safety.
The candidate dossier command records explicitly unreviewed file, build,
library, and capability observations only. It does not generate a notice,
source offer, license declaration, approval, or right to distribute any
observed component.

gifski and libvips were researched but are not included in Morflo v0.1.

This notice must be regenerated and reviewed against exact lockfiles and any packaged sidecars before public distribution.

## Audit boundary

`pnpm audit --prod --audit-level high` reported no known vulnerabilities on
2026-08-30 and again on 2026-09-03.

A RustSec advisory result is now claimed. `cargo-audit 0.22.2` installed
successfully on 2026-09-03 and scanned `src-tauri/Cargo.lock` against 1,239
advisories covering 502 crate dependencies. It exited 0 with **zero
vulnerabilities**. Seventeen warnings were reported: sixteen `unmaintained` and
one `unsound`. None carries a severity, and every affected crate is either a
build-time macro/Unicode helper (`proc-macro-error`, the `unic-*` family) or
part of Tauri's Linux GTK 3 stack (`atk`, `gdk`, `gtk`, `glib` and their `-sys`
crates), which upstream has marked no longer maintained.

The one `unsound` warning is `RUSTSEC-2024-0429` against `glib 0.18.5`, the same
issue GitHub Dependabot reports as `GHSA-wrw7-89jp-8q8g`. Its scope is now
established rather than assumed: `cargo tree --target x86_64-pc-windows-msvc -i
glib` finds nothing, so the crate is not in the Windows dependency graph at all
and cannot affect Morflo's primary release target. It reaches the tree only on
Linux, through `glib -> atk -> gtk -> muda -> tauri`. Morflo does not use the
affected `VariantStrIter` API directly, and the first patched `glib 0.20.0`
remains outside that graph's compatible major line.

What remains open is narrower than before: a Linux release needs either an
upstream-compatible GTK upgrade or an explicit owner assessment of these
unmaintained bindings. Public release review must still rerun advisory and
license audits against the final target-specific dependency graph.
