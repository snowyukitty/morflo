# Release red-team review

Four separate review passes were performed on 2026-08-30. Each pass began from
its own acceptance question and inspected current source, tests, screenshots,
or package evidence rather than relying on the implementation checklist.

## Product review

Question: Is Morflo materially simpler and safer than a traditional transcoder?

- The primary hierarchy remains source → recommended result → Convert; codec
  controls do not dominate the default image, video, or GIF journeys.
- Mixed-media files retain compatible per-item outcomes instead of inheriting an
  unsafe global format.
- Finding: the footer counted a running job and a queued job as “2 active.” The
  copy now says “2 in progress,” accurately describing both without adding UI.
- Finding: scope text promised an unsigned Windows artifact despite a known host
  compile blocker. It now names Windows as the target and requires the closest
  host-supported artifact without weakening host security.
- The focused Open with entrance earned its place by removing a picker round
  trip without adding a settings screen. Audio extraction, hardware encoding,
  deeper Explorer actions, persistent history, updater, and reusable presets
  remain deferred.

Residual, retired by ADR 0007: a first-run engine-missing state was previously
unavoidable until a redistributable engine was approved. Morflo now compiles its
own permissively licensed image engine, so a computer with no FFmpeg converts
images, batches and favicons immediately. What remains is a narrower and honest
statement: video, animated GIF, WebP and AVIF are disabled with the reason that
names what to install, and keeping source metadata is refused rather than
silently downgraded.

## Design review

Question: Can the hierarchy, trust state, and next action be understood within
two seconds across long-session states?

- Re-inspected empty, queue, active, GIF, completion, error, dark, and 780×620
  captures. The selected source/result, primary action, warning, and status are
  visually distinct without dashboard chrome or decorative motion.
- The error state names damage and offers retry plus secondary technical details;
  success foregrounds Reveal without exaggerated celebration.
- Computed serious/critical axe checks and the 200% text-size path pass. Status
  is conveyed with text and icons as well as color.
- The native window frame and single scroll surface preserve platform behavior
  and keyboard/zoom resilience.

The deterministic state screenshots remain browser evidence. They are now
supplemented—not relabeled—by release and installed Windows/WebView2 ready,
Diagnostics, and completed screenshots at the host's 150% device scale.

Outcome-confidence follow-up: the old terminal panel did not answer “what did I
get?” The replacement uses actual post-probe output facts and one clear Reveal
action. A batch receipt keeps failed/canceled counts beside successful output
totals. The result remains calm in light/dark and scroll-reachable at 780×620.
Visual inspection also removed a misleading universal checkerboard: only an
output whose probe reports alpha receives transparency treatment.

## Engineering review

Question: Can malformed input, hostile paths, cancellation, or a crash corrupt
data or escape the trusted command boundary?

- All engine and reveal processes use structured argument arrays; the frontend
  cannot supply a shell command or broad filesystem operation.
- Output is created with `create_new`, probed before publish, finalized beside
  the destination, and collision-reserved. A failed/canceled run never receives
  the final name.
- Finding: the Unix recovery journal originally used a shared temporary
  directory. It is now keyed by effective user ID, refuses a symlink/non-folder,
  enforces mode 0700, and creates markers with mode 0600; a regression test
  checks privacy and symlink refusal.
- Real cancellation terminates the process group within the required two-second
  budget and host inspection finds no surviving FFmpeg child.
- Capability availability comes from the Rust engine probe and planner, not a
  frontend extension table.
- Finding: normal Tauri Windows file associations would write an extension
  default ProgID, which is broader than Morflo's promise. Generated NSIS hooks
  now register only alternate Open with candidates. The lifecycle test proves
  existing defaults and `UserChoice` survive install/uninstall/reinstall, and
  uninstall removes only Morflo-owned keys.
- Later-instance arguments never reach a shell. Rust accepts only existing
  regular non-symlink files, resolves relative paths against the activation
  directory, bounds pending intake at 512, then sends the files through normal
  inspection. Native evidence leaves one process and one WebView.

The native Windows job-object path now passes a real 120-second encoder
cancellation test in 667 ms with no child remaining. A separate Windows-only
regression also prevents verbatim canonical paths from changing FFprobe demuxer
selection for APNG.

Finding: the first complete native GUI conversion exposed JPEG finalization
rejecting FFprobe's `image2` label. Output publication now validates both
container evidence and the primary codec; the native collision-safe journey and
unit matrix pass after the fix.

Finding: the transparency fixture had colored RGB data but zero alpha in every
pixel, so a uniform background could satisfy the former flattening assertion.
Fixture generation now explicitly replaces alpha for its visible shapes, and
the real-engine test samples both background and foreground. The production
flatten pipeline preserves visible content in the refreshed native preview.

Output receipt facts are produced before final publication from the probed
partial and emitted only after finalization succeeds. The frontend receives no
absolute output path. Preview commands accept a job identifier, resolve only the
job manager's final path, probe it again, reject non-regular/symlink targets,
and return bounded image/poster/GIF data.

Finding: the original GIF strip visually resembled frames but contained only a
gradient. The replacement command accepts a session-owned job ID rather than a
path, returns exactly seven 160×90 JPEGs, caps each at 256 KiB, permits two
samplers, and enforces a 20-second overall deadline. Input seek avoids decoding
from the beginning for late moments. Failure leaves the semantic range and
conversion available.

The installed release now supplies direct cancellation evidence: a 30-second
MP4 → WebM job reached Running, stopped 328 ms after the visible cancel action,
published no final output, left no recognized partial, preserved the source,
and had no surviving direct FFmpeg child.

## Built-in image engine review

Question: Does compiling a second engine into Morflo widen what a hostile file
can do to the application?

The built-in engine decodes untrusted bytes in Morflo's own process, where
FFmpeg previously did that work behind a process boundary. It spawns nothing,
reads no environment, and touches no network, so the command-injection and
inherited-credential surfaces do not grow. Memory safety is the decoder crates'
own guarantee rather than Morflo's. What does change is that a decoded surface
is now materialized inside Morflo, which makes resource exhaustion the real
question.

Finding, found by testing rather than review: the decode allowance did not work.
`image::Limits::max_alloc` was set but never reaches the PNG decoder's buffers —
its `set_limits` checks dimensions only and carries an upstream TODO about
constraining internal allocation. That left `max_image_width`/`max_image_height`
as the sole guard, and 100,000 x 100,000 RGBA is roughly 40 GB, which a PNG of a
few hundred bytes can declare. Morflo exists to open files other people
produced, so this was a denial of service against the whole application from an
ordinary-looking input.

Morflo now enforces its own budget against the header before any surface is
materialized, and refuses an oversized declaration with a message naming the
media engine, which streams such an image instead of holding it whole. A
regression test constructs a real 50,000 x 50,000 PNG bomb, asserts the refusal
comes from Morflo's budget rather than an incidental parse failure, and checks
that the reserved partial file stays empty. A companion test fixes the
boundary: a 16,000 x 12,000 scan is admitted, the maximum declarable surface is
not.

Residual: the budget is a fixed ceiling, not an adaptive one. A machine with
little free memory can still be pushed harder than it likes by a large but
legitimate image, and the decoder crates' own intermediate allocations are not
counted. Morflo makes no memory-safety claim on behalf of its decoding
dependencies.

## Privacy and release review

Question: Is user media transmitted, retained unnecessarily, or packaged with
untraceable components?

- The production audit finds no runtime fetch/XHR/WebSocket/analytics/crash
  upload/updater and validates the strict IPC-only CSP. Ten real conversions pass
  inside a network-isolated namespace.
- Finding: preview maps could retain several prior thumbnails/clips for the
  session. They now hold only one active image, poster, clip, and completed GIF;
  completed GIF preview is loaded only when selected.
- Completed-output previews retain the same session-only policy. Images and
  posters are decoded through bounded backend processes; GIF direct reads are
  capped at 20 MiB. No path, filename, thumbnail, or output fact is transmitted.
- Finding: the first Linux package omitted notices. The deb configuration now
  includes README, third-party notices, and the media-engine distribution record
  under `/usr/share/doc/morflo/`. Final package inspection verifies all three at
  mode 0644; rebuilding from WSL ext4 removed DrvFS's accidental executable bits.
- No engine blob or private build path is permitted in the package. The manual
  CI workflow has read-only contents permission, no secrets, no upload, no
  publish, and no automatic trigger.
- The association manifest contains only extensions, product metadata, and a
  fixed executable name. Registry commands are generated, checked for staleness,
  and quote executable/path placeholders independently. No filename, media
  metadata, or path is persisted by the registration layer.
- npm production advisory audit passes. Runtime npm and Cargo license metadata
  have no undeclared license fields in the recorded local audit. RustSec advisory
  scanning remains unrun because a project-local `cargo-audit` install could not
  complete from the available crates.io cache/network.

Release blockers: no approved redistributable engine manifest, no selected root
license, no Windows/Apple signing credentials, macOS builds unrun, CI workflow
unrun, and macOS/physical-Linux packaged GUI interaction unrun. Native Windows
construction and one installed semantic GUI conversion are proven through
bounded paths, but the result remains unsigned.

Evidence boundary: the exact installed Open with command and later-process
handoff pass, but the final Explorer menu-selection gesture is unrun because the
native desktop-control pipe was unavailable. This is a QA evidence gap, not a
claim that the gesture passed.

## Reviewed sidecar intake review

Question: Can a candidate acquire or retain “reviewed” identity without matching
the exact evidence approved for this build?

- A forged manifest cannot substitute an executable while the Morflo binary and
  its compiled pin remain trusted. The manifest SHA-256 is a separate build
  input; executable/support/evidence hashes are checked before probing and
  rechecked afterward. Replacing both manifest and executable changes the pin.
- Bundle roots, every path component, and the complete recursive inventory are
  checked. Absolute/traversal/backslash paths, duplicate or Windows
  case-colliding names, canonical escape, symlink/reparse entries, empty
  evidence, and undeclared extra files fail closed.
- The frontend can submit only a user-picked folder for session use. Rust
  constructs the two fixed executable names and all argument arrays. It cannot
  select an arbitrary filename or mark that folder reviewed.
- Exact ffmpeg/ffprobe version and configure lines must agree with each other
  and the manifest. Complete probed inventories and derived Morflo outputs must
  match exactly, so a changed/stale build fails. Approval also expires no later
  than 366 days after review.
- A present bundled directory with a missing pin or failed evidence returns a
  packaged-engine verification error and stops automatic fallback. A deliberate
  later session-folder choice is visibly labelled Session engine.
- Normal packaging clears inherited pin state and supplies no engine resource.
  The reviewed path takes one explicit directory plus digest, stages only
  verified declared files, and never reads PATH or the installed Gyan build.

Residual boundary: an attacker able to replace the unsigned Morflo executable
can replace the compiled trust pin and verification code. Same-user mutation
after runtime verification is also not prevented by this milestone. Trusted
signing and installation integrity remain required; no legal or patent safety
claim follows from the technical controls.

## Unreviewed candidate dossier review

Question: Can observation accidentally execute the wrong file, leak parent
credentials, become a reviewed identity, or conceal candidate mutation?

- Capture refuses to run without the standalone `--allow-execution` flag. An
  invalid/existing output is rejected before probing so a bad destination
  cannot trigger candidate side effects.
- Only exact root `ffmpeg`/`ffprobe` names execute. The frontend cannot invoke
  the command, supply an executable, choose arguments, or consume its output as
  a runtime engine.
- Candidate roots, every entry, and the external dossier path reject symlink,
  reparse, canonical escape, special-file, depth/count/size, control-character,
  and output-inside-candidate surprises. Output uses exclusive creation and is
  never overwritten.
- Every regular file is hashed through a stable open handle with file identity,
  size, mtime, and ctime checks. The complete sorted inventory is repeated
  after version/configuration/compiler/library/capability probes; any difference
  fails closed. A later `--check` requires both current evidence and exact
  canonical bytes.
- Dossier schema and reviewed manifest schema have different required kinds and
  fields. The dossier hard-codes `unreviewed`, contains no provenance/license/
  notice/source-offer/reviewer claim, has no automatic promotion path, and is
  rejected by the reviewed bundle verifier.
- Node and Rust media-engine commands clear the parent environment. Only
  deterministic locale plus minimal Windows OS/temp variables remain; PATH,
  HOME, tokens, proxy settings, and dynamic-loader overrides do not cross the
  process boundary.

Residual boundary: `--allow-execution` is informed consent, not containment. A
malicious candidate can act with the user's filesystem/network authority,
detach descendants, or race another same-user process. Stable hashing and
pre/post checks detect evidence changes but cannot make hostile execution safe.
Unknown candidates require an appropriately isolated external review
environment; Morflo makes no sandbox, malware, provenance, license, patent, or
redistribution claim.
