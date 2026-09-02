# ADR 0005: Fail-closed reviewed media-engine intake

- Status: Accepted
- Date: 2026-08-31

## Context

Morflo needs to be able to consume a future reviewed FFmpeg/ffprobe sidecar,
but no exact redistributable engine, root application license, patent position,
or distribution approval exists yet. A manifest stored beside an executable is
not a trust anchor: an attacker who can replace both could update the manifest
hashes and self-assert `approved`.

Development still needs explicit session folders, project-local paths, and
system discovery. Those sources are useful runtime evidence but must never be
presented as reviewed package inputs.

## Decision

Adopt `src-tauri/engine-bundle.schema.json` schema version 1 and enforce it in
both the project-owned operator command and the Rust runtime boundary.

The manifest records exact engine name/version, target, provenance identifier,
FFmpeg configure line, declared license, executable names/sizes/SHA-256,
optional supporting files, complete probed decoder/encoder/demuxer/muxer/filter
inventories, derived Morflo outputs, hashed notice and source-offer references,
and reviewer/status/date/validity evidence. Approval expires no later than 366
days after review. The verifier is offline and accepts only one explicit
absolute directory.

Manifest integrity and reviewed identity are separate claims:

1. `pnpm engine:verify --dir <absolute-directory>` proves current local
   structure, bytes, declared evidence, tool agreement, and capability agreement.
   It prints the exact manifest SHA-256 but does not grant legal or release
   approval.
2. Reviewed packaging additionally requires that digest as a separate explicit
   input. The build compiles it into Morflo and stages only verified,
   manifest-declared files through a temporary Tauri resource overlay.
3. Runtime bundled discovery requires the compiled digest to match the on-disk
   manifest before executing either tool. Hashes are rechecked after probing.
4. A present bundled directory with a missing pin or any failed evidence is a
   terminal package error. Automatic discovery does not fall back and cannot
   retain a reviewed label. A deliberate session-folder selection remains an
   explicit, separately labelled recovery path.

Normal packaging clears inherited manifest-pin state and supplies no engine
resource mapping. It never discovers packaging input from `PATH`, registry,
WinGet, or project development directories.

## Threat boundary

The gate prevents a forged manifest from substituting an executable while a
trusted Morflo binary retains the independently reviewed digest. It rejects
relative traversal, absolute artifact paths, symlinks, Windows reparse points,
canonical paths outside the bundle, duplicate/case-colliding paths, untracked
files, target mismatch, version/configuration/capability drift, missing legal
evidence references, and expired review.

Frontend input cannot name an executable or supply arguments. The session
folder command constructs only the platform's fixed `ffmpeg` and `ffprobe`
names in Rust, verifies matching versions and capabilities, and keeps the
choice in memory. It is intentionally not a reviewed source.

An attacker who can replace the Morflo executable can also replace its compiled
pin and verifier. Preventing that requires trusted signing and installation ACLs,
which are outside this milestone. Same-user replacement after successful
runtime verification is likewise outside the current unsigned package's
integrity guarantee. No copyright-license, codec-patent, commercial-use, or
redistribution-safety conclusion follows from this technical gate.

## Consequences

- Future bundle review produces one stable digest that can be reviewed and
  supplied independently at build time.
- Capability additions and removals both invalidate the manifest; reviewers
  must examine a new exact inventory instead of inheriting unreviewed codecs.
- Dynamic builds can declare hashed supporting libraries/data, but every bundle
  entry must be declared; extra files fail closed.
- The normal package stays small and engine-free until external owner/legal
  decisions are complete.
