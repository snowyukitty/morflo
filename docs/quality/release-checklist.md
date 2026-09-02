# Release checklist

## Product

- [x] All ten acceptance scenarios have recorded evidence or an explicit blocker.
- [x] Verified support table matches exact development-engine tests and does not imply a bundled engine.
- [x] Source immutability and collision-safe output are regression tested.
- [x] Cancellation terminates process trees and removes recognized partials.

## Design and accessibility

- [x] Eight required screenshot states captured and inspected.
- [x] Ready/missing Engine Diagnostics and release-WebView ready/converted states captured and inspected.
- [x] GIF light, dark, and narrow Moment strip states plus installed real-frame,
      animated receipt, and canceled-video states captured and inspected.
- [x] Individual and partial-batch receipts use post-probe output facts; opaque
      previews do not imply transparency, and the minimum-size Reveal action is
      reachable.
- [x] Three visual refinement passes logged.
- [x] Keyboard-only primary flow passes.
- [x] Focus, contrast, status announcements, reduced motion, zoom, minimum size, and 100/125/150% scaling reviewed where the environment permits.

## Engineering and privacy

- [x] Frontend, privacy, native Windows release-profile Rust/Clippy, real-engine,
      cancellation, and Playwright gates pass. The standard fresh debug-profile
      `pnpm check` Rust step remains explicitly blocked by App Control rather
      than being called passed.
- [x] No user-controlled shell strings, broad frontend filesystem permission, remote runtime assets, analytics, crash upload, or updater.
- [x] CSP and Tauri capabilities are restrictive.
- [x] npm production advisory/license and Cargo license metadata audits reviewed; RustSec remains explicitly blocked rather than called passed.
- [ ] Resolve or formally assess Dependabot `GHSA-wrw7-89jp-8q8g` before a
      Linux public release. It affects transitive `glib 0.18.5` in Tauri's GTK
      graph; Morflo does not directly use the named API, but the first patched
      major is not currently graph-compatible.
- [x] Ten real conversions pass inside an isolated user/network namespace with Cargo offline.

## Packaging

- [x] Native Windows x64 executable and unsigned NSIS installer built, scanned,
      installed per-user, and smoke-launched through the delivered desktop
      shortcut. The installer contains no media engine.
- [x] Native release-WebView PNG → JPEG journey passed with stale inherited
      `PATH`, international filename, collision-safe suffix, immutable source
      and existing output, probeable visible result, decoded bounded preview,
      measured receipt, and no partial residue.
- [x] Native release-WebView GIF journey passed with seven decoded moments,
      dependent range controls, local clip, 24-frame output, decoded receipt,
      immutable source, and no partial residue.
- [x] Native release-WebView cancellation passed on a real 30-second MP4 →
      WebM encode; the current sanitized-environment package stopped in 1,420 ms
      with no published output, partial, or surviving FFmpeg child.
- [x] Windows Open with installation registers 13 conservative alternate
      handlers, preserves every observed default/`UserChoice`, quotes direct
      commands, removes only Morflo-owned values, and restores the existing
      IconFlow Desktop shortcut byte-for-byte.
- [x] Installed single-instance intake passed with PNG/MOV and relative WebP
      international paths, one Morflo process/WebView, clean secondary exits,
      immutable sources, and three visually inspected release captures.
- [x] Final Linux package contents inspected for unintended files, private paths, absent engine blobs, and included notices.
- [x] App icon passes the IconFlow rubric and automated small-size checks.
- [x] Final artifact version, metadata, desktop entry, 0644 icons/notices, and dependencies are inspected. Publisher remains intentionally unset pending an owner decision.
- [x] Manifest schema v1, offline explicit-directory verification, independent
      compiled manifest pin, exact hash/version/configuration/capability checks,
      time-bounded approval, and invalid-bundle terminal behavior have
      synthetic/runtime regression coverage.
- [x] Candidate dossier schema v1 and command require explicit execution
      acknowledgement, fixed executable names, an external exclusive output,
      complete stable file hashes before/after probing, compiler/library/
      capability observations, canonical `--check`, sanitized child
      environments, and a non-promotable `unreviewed` marker. The current Gyan
      build was exercised only as temporary non-redistributable evidence.
- [x] Normal Windows packaging has no engine resource input and clears inherited
      reviewed-manifest pin state. Reviewed staging requires both an explicit
      directory and independently reviewed exact manifest digest; it never uses
      PATH discovery.
- [x] Rebuilt the normal Windows installer with a deliberately inherited fake
      pin; the wrapper cleared it and used engine-free mode. Install-tree
      inspection found only Morflo, its uninstaller, and two icon resources—no
      FFmpeg, ffprobe, engine manifest, or engine support files.
- [ ] Select and legally review an exact redistributable engine bundle, then
      record its independent manifest digest. No current local engine satisfies
      this gate.
- [x] Signing status is stated accurately. Public distribution requires owner authorization and trusted signing credentials.
- [x] Manual Windows/macOS/Linux CI structure exists without publishing, uses
      the repository-owned `pnpm verify:quality` contract, and gates the
      metered platform matrix behind Ubuntu quality.
- [ ] Hosted cross-platform completion. Run
      [33318068468](https://github.com/snowyukitty/morflo/actions/runs/33318068468)
      was dispatched for commit `0aafdab`, but the Ubuntu job was refused
      before any step ran for reasons external to this repository; the
      dependent platform matrix was skipped. No hosted platform validation is
      claimed.

Manual boundary: the final Explorer Open with menu-selection gesture remains
unrun because the host's native desktop-control pipe was unavailable. Registry
lifecycle and exact installed-process invocation pass; public claims must retain
that distinction until a physical click-path check is recorded.
