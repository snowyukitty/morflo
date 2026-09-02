# ADR 0007: A built-in image engine so Morflo is never a dead end

- Status: Accepted
- Date: 2026-09-03

## Context

Morflo's media engine is FFmpeg. The reviewed-sidecar gate (ADR 0005) and the
candidate dossier gate (ADR 0006) make packaging one safe, but neither removes
the underlying constraint: the available FFmpeg build is GPLv3 full, so Morflo
cannot redistribute it, and no approved redistributable manifest exists.

The consequence reached the person using the application. A computer without a
local FFmpeg could not inspect a single file, because `inspect_files` required
`EngineService::runtime()` and that call returned `EngineMissing`. Nothing could
be added to the queue, no capability was available, and no conversion could be
planned. The release red-team review recorded this as an accepted residual: "a
first-run engine-missing state remains necessary until a redistributable engine
is approved."

That residual is larger than it looks. The product brief's five first-release
jobs are ordered by priority, and the first three — convert a common image,
convert a batch of images, create a multi-resolution ICO — need no video
machinery at all. They were blocked only because one engine served every
journey.

Meanwhile the Rust image ecosystem publishes decoders and encoders under MIT or
Apache-2.0. Those can be compiled into Morflo and shipped, which is exactly what
FFmpeg cannot be.

## Decision

Add `src-tauri/src/image_engine.rs`, a second engine compiled into Morflo, and
make the media engine optional rather than required.

1. `EngineRuntime` no longer owns an FFmpeg pair directly. It holds
   `Option<MediaTools>`, and every caller that needs a subprocess asks through
   `EngineRuntime::media()`, which returns a typed error naming what is missing.
   The compiler now enforces that no code path assumes FFmpeg exists.

2. `discover_engine` returns `EngineRuntime::native_only(diagnostic)` instead of
   an error when no media engine is found, keeping the reason for Diagnostics.
   A session engine folder that stops verifying degrades the same way. A
   packaged engine that fails verification stays terminal and does **not** fall
   back, because a tampered bundle must never quietly become a working
   application.

3. The built-in engine writes PNG, JPEG and multi-resolution ICO, and decodes
   PNG, JPEG, WebP, GIF, BMP, TIFF and ICO. It applies EXIF orientation, mirrors
   the media-engine resize modes and bounds, flattens alpha onto the chosen
   background for JPEG, and emits the same four-entry 16/32/48/256 icon.

4. `ConversionPlan` carries a `ConversionProgram` that is either `Engine`
   argument-array stages or one `Native` task, so execution, progress reporting
   and verification can see which engine ran instead of inferring it.

5. When a media engine is present, every image journey still goes through it.
   The built-in engine is the path only when no media engine exists.

## Why the built-in engine is not the default when FFmpeg is present

Making it default would change output bytes, metadata handling and alpha
behavior for every existing installation, invalidating acceptance evidence that
was gathered against real-engine output. The gain would be speed on batches; the
cost would be re-proving journeys that already pass. Keeping FFmpeg
authoritative where it exists means this milestone adds a capability without
retracting any evidence. Making the built-in engine the default for images is a
separate decision that needs its own comparative evidence.

## Why WebP and AVIF are not offered natively

The pure-Rust WebP encoder available today is lossless only, and its own
documentation notes it does not reach lossless WebP's size potential. Offering
"Smaller / Balanced / Best" for a lossless-only encoder would present a control
that cannot act, and a photograph converted this way would grow rather than
shrink. AVIF encoding is possible in pure Rust but has not been evidenced here.

Morflo's product principle is that unsupported capability is disabled with a
reason rather than advertised and allowed to fail later, so both formats report
that they need a media engine, with the specific reason.

## Why preserving metadata is refused rather than downgraded

The built-in encoders write no source metadata. `Preserve` is an explicit,
non-default choice for images, so silently writing a file stripped of the
metadata the person asked to keep would be exactly the quiet data-loss surprise
Morflo promises not to produce. The request is refused with its reason and the
remedy.

## Consequences

- A computer with no FFmpeg can inspect common images, convert them to PNG or
  JPEG, run a batch, create a favicon, see a bounded outcome preview, and get
  collision-safe naming with an immutable source.
- Video, animated GIF, WebP and AVIF remain honestly unavailable with a reason
  that names what to install.
- Morflo carries new compiled dependencies. All are MIT, Apache-2.0, BSD or
  Unlicense; none is copyleft, so the redistribution constraint that applies to
  FFmpeg does not apply to them. They are recorded in `THIRD_PARTY_NOTICES.md`.
- Cancellation on the built-in path is observed around one bounded encode
  rather than by terminating a process. The in-flight encode is always awaited,
  because abandoning it would let a background thread keep writing to a partial
  file the caller is about to delete.
- `tests/native_engine.rs` is deliberately not feature-gated. "Works without an
  engine" is the property under test, so it must run everywhere.

## Alternatives considered

- **Bundle FFmpeg anyway.** Blocked by the unresolved license decision that
  ADR 0005 exists to gate. Not available.
- **Keep the dead end and improve its copy.** The previous milestone already
  did that; better wording does not convert a file.
- **Route only the missing formats to the built-in engine while FFmpeg serves
  the rest.** Rejected as the trigger because "which engine ran" would then
  depend on a per-format capability table that varies by build, making results
  hard to reason about. Presence of a media engine is a single, explainable
  switch.
