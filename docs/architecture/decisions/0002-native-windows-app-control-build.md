# ADR 0002: Native Windows build under App Control

- Status: Accepted
- Date: 2026-08-30

## Context

The primary Windows 11 x64 host enforces App Control. Cargo could link fresh
build-script executables and proc-macro DLLs, but their first load was rejected
with `os error 4551` and Code Integrity events 3077/3033. Moving the target
directory did not change the policy outcome. WSL can produce a Linux package,
but it is not an acceptable Windows runtime or primary release architecture.

An independent, read-only Agent Task Delegation review recommended keeping the
MSVC build and recovering only the exact generated artifact named by Cargo. It
rejected elevation, policy weakening, path trust, self-signed certificates,
cross-compilation as the primary route, and WSL/WSLg as runtime architecture.

## Decision

Keep `x86_64-pc-windows-msvc` and Tauri's native NSIS bundler. Route
`pnpm package` through `scripts/build-windows.ps1`, which:

- has fixed 40-round and 45-minute limits;
- fixes `CARGO_TARGET_DIR` to this repository's `src-tauri/target`;
- recognizes only Cargo's refused build helper or proc-macro evidence;
- validates that every recovery target is an exact direct child of
  `target/release/build` or `target/release/deps`;
- relinks only that generated artifact and stops on every unrelated failure;
- never elevates, modifies App Control/Defender, trusts a directory, executes a
  downloaded binary, or deletes source.

Use `dunce::canonicalize` at the external media-engine boundary. Rust's verbatim
Windows canonical form can change FFprobe's extension-based demuxer selection;
the simplified equivalent remains canonical and compatible with FFmpeg tools.

## Consequences

- Native Windows packaging is reproducible on this host without WSL runtime or
  security-policy changes, but generated unsigned hashes may require bounded
  relinking again after dependency or toolchain changes.
- The wrapper is intentionally package-specific. A fresh debug-profile Cargo
  quality command may still be rejected; release-profile Clippy and tests are
  the recorded Windows gate.
- Public distribution remains blocked by code signing and media-engine/license
  decisions. Successful local construction is not a signature or trust claim.
- The path simplification has a Windows regression test and the APNG behavior is
  covered by the real-engine suite.

## Rejected alternatives

- **WSL2/WSLg runtime:** wrong platform semantics and explicitly outside the
  product's native Windows promise.
- **Disabling or weakening App Control/Defender:** unacceptable security change.
- **Trusting the whole target directory:** grants broader authority than needed.
- **Self-signing generated build artifacts:** does not establish production
  trust and adds certificate lifecycle risk.
- **Cross-compiling NSIS as the primary path:** Tauri documents this as an
  experimental last resort; it does not validate native Windows runtime
  behavior.
