# Morflo project instructions

If this checkout sits inside a workspace that carries its own `AGENTS.md` at the
directory above it, read that file explicitly before working here. It holds
cross-project policy that takes precedence over this file. Resolve its path from
this repository's location; do not assume a drive letter.

## Product contract

- Morflo is a local-first desktop image and video converter. It is not a media editor, cloud service, or all-format toolbox.
- The source file is immutable. Default output collision handling appends a numeric suffix and never overwrites.
- Frontend code never builds command lines and never receives arbitrary shell or filesystem authority.
- Format availability comes from the Rust capability registry and runtime engine probing, not filename extensions alone.
- UI copy, code, identifiers, comments, logs, filenames, commits, and technical documentation are English.
- Do not add analytics, remote runtime assets, hosted fonts, update checks, or conversion-time network calls.
- Morflo is dual licensed `MIT OR Apache-2.0`, chosen by the owner on
  2026-09-03. Keep `LICENSE.md`, `LICENSE-MIT`, `LICENSE-APACHE`, the Cargo
  `license` field, and the notices consistent with each other.
- Do not commit media-engine binaries. Development engines are discovered locally; reviewed sidecars are a release input.

## Commands

Use pnpm and the checked-in lockfile.

```powershell
pnpm install --frozen-lockfile
pnpm dev
pnpm tauri:dev
pnpm check
pnpm test
pnpm test:real
pnpm test:e2e
pnpm test:native:windows
pnpm test:native:gif:windows
pnpm test:native:cancel:windows
pnpm test:native:open-with:windows
pnpm test:installer:open-with:windows
pnpm engine:dossier --dir <explicit-absolute-candidate-directory> --out <explicit-absolute-new-json> --allow-execution
pnpm engine:verify --dir <explicit-absolute-directory>
pnpm build
pnpm package
pnpm package:reviewed-engine --dir <explicit-absolute-directory> --expected-manifest-sha256 <sha256>
pnpm package:inspect-engine-free --dir <explicit-absolute-installed-directory>
pnpm generate-fixtures
pnpm audit:prod
pnpm verify:quality
pnpm verify
```

Run `pnpm verify:quality` for the reproducible local/hosted quality contract.
Run `pnpm verify` before a release checkpoint. Real-engine checks require
compatible `ffmpeg` and `ffprobe` executables on `PATH` or in a documented
Morflo engine location.

`.github/workflows/quality-and-platforms.yml` is intentionally manual. It must
call `pnpm verify:quality` instead of duplicating its steps, gate expensive
platform compilation behind the Ubuntu quality job, and must not publish or
upload binaries. Dispatch it on an exact branch or commit only when the hosted
result will inform a checkpoint decision.

The four `test:native:*:windows` commands require a current Windows release
executable and generated fixtures. They cover the image receipt, GIF moment
strip and output, real process-tree cancellation, and single-window external
file intake. Each uses an ephemeral, process-local WebView2 debugging port;
debugging flags must never be added to production configuration or persisted in
the registry. The installer Open with test requires Morflo to be closed. It
performs a current-user install/uninstall/reinstall, preserves any existing
Desktop shortcut byte-for-byte, and leaves Morflo installed.

On Windows, `pnpm package` must remain routed through
`scripts/build-windows.ps1`. The wrapper is the reviewed App Control recovery
boundary: it may relink only an exact refused Cargo-generated artifact under the
release target and must never elevate, alter policy, trust a directory, or
delete source files.

Reviewed media-engine packaging is a separate explicit opt-in. The candidate
directory and independently reviewed manifest SHA-256 are both mandatory. The
verifier and runtime must stay offline, reject traversal/symlink/reparse and
untracked-file surprises, compare exact versions/configuration/capabilities,
and fail closed on missing or expired evidence. Normal builds must clear any
inherited pin and remain engine-free. Never derive a packaging input from PATH
or label a session/system/development engine as reviewed.

Candidate dossier capture is an earlier, explicitly unreviewed boundary. It
must require an explicit candidate directory, an external new output file (or
existing check file), and `--allow-execution`. It may observe only the fixed
root ffmpeg/ffprobe pair with structured arguments and a sanitized environment,
must hash the complete bounded regular-file inventory before and after probing,
and must never infer provenance, notices, source offers, licensing, approval,
or packaging eligibility. A dossier is not a bundle manifest and cannot become
a trust pin without separate human review and manifest authoring.

`src-tauri/windows/open-with.json` is the source of truth for the conservative
Windows handler set. Edit it, run `pnpm generate:windows-open-with`, and commit
the generated `hooks.nsh`. Never replace this with a default file association or
write Windows `UserChoice`.

On Linux/WSL, use `scripts/test-offline.sh` for the real-engine network-namespace
check and `scripts/smoke-linux-package.sh` to inspect and launch an unsigned deb.
Both require explicit absolute inputs and must remain fail-fast.

## Editing and verification

- Keep Rust domain and engine modules independent of Tauri where practical so they can be tested without a window.
- Use direct process spawning and argument arrays. Never invoke a shell with user-controlled input.
- Keep technical diagnostics bounded; ordinary UI errors must redact absolute path prefixes.
- Stage explicit paths only. Preserve unrelated work and never use destructive Git cleanup.
- Update `docs/STATUS.md` at each implementation gate with checks actually run and unresolved risks.
