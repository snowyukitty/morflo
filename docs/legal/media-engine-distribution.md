# Media-engine distribution

This document is engineering evidence, not legal advice.

## Policy

Morflo does not commit, download, discover for packaging, or silently
redistribute media-engine executables. Engine-bundle manifest schema v1 is at
`src-tauri/engine-bundle.schema.json`. A candidate records exact engine
name/version, platform/architecture, source URL or provenance identifier,
configure line, declared license, exact executable names/sizes/SHA-256,
optional hashed runtime files, complete probed codec/container/filter inventory,
derived Morflo outputs, hashed notice/source-offer references, and a named,
dated, time-bounded review status.

The separate `src-tauri/engine-candidate-dossier.schema.json` schema v1 records
only unreviewed observations: complete candidate file hashes, matching program
and build identity, compiler and FFmpeg library versions, and capability
inventory. `pnpm engine:dossier` cannot declare provenance, classify notices or
source offers, choose a license, approve distribution, create a manifest, or
produce a packaging trust pin.

The manifest is evidence, not its own authority. Reviewed packaging requires a
separately supplied SHA-256 of that exact manifest, and runtime acceptance
requires the same digest compiled into Morflo. A changed manifest cannot update
its own trust anchor. `pnpm engine:verify --dir <explicit-absolute-directory>`
makes no network requests and reports technical integrity only.

Morflo's dossier/verifier code makes no network requests, but dossier probing
executes the candidate binary. A malicious candidate retains the current user's
OS authority and is not network-sandboxed. `--allow-execution` is therefore
mandatory, the child environment is stripped of inherited credentials/search
paths/proxy variables, and the command makes no stronger containment claim.

The project license has not been chosen. Engine choices must preserve the owner's ability to choose proprietary, source-available, or open-source terms.

## Development engine observed on 2026-08-30

- Source/distributor: Gyan FFmpeg build installed through Microsoft WinGet (`Gyan.FFmpeg`)
- Version: `8.1.2-full_build-www.gyan.dev`
- Compiler: GCC 16.1.0, MSYS2
- Configuration: static full build with `--enable-gpl --enable-version3`, including libx264, libx265, libvpx, libwebp, libaom, libsvtav1, libopus, and many optional libraries
- Runtime-reported effective license: GPL version 3 or later
- Platform: Windows x64
- ffmpeg SHA-256: `AD8F211BC894755E0061C55AB280AE00E8D3D4F15A8CC4372B24CFA247B5942E`
- ffprobe SHA-256: `9DF3B0B5275E830961DF6D94E1F7A71121A7ABD5FF708E9FEC8A0B6084A55015`
- Use: local development and test evidence only; not committed or automatically bundled
- Intake status: not reviewed for packaging and not eligible for the opt-in
  sidecar input in this checkpoint
- Unreviewed dossier evidence: capture and subsequent `--check` produced the
  same SHA-256
  `c549cbb8d4b3a2d7f7e6c9bb32f8ea4f2caa899774b5800bb84ea21eb3e94e06`
  over three regular files, seven reported FFmpeg libraries, and the complete
  observed capability inventory. The temporary dossier was deleted; this
  digest is technical observation, not an approved manifest or retained release
  input.

The host-specific absolute installation path is intentionally omitted from project documentation.

## License implications

[FFmpeg's official legal page](https://ffmpeg.org/legal.html) states that FFmpeg is LGPL-2.1-or-later by default, but enabling GPL components makes the FFmpeg build GPL-2.0-or-later; build configuration and source availability therefore matter. Codec patents and distribution rules are separate from copyright licenses. Morflo makes no commercial-safety or patent-clearance claim.

[gifski](https://github.com/ImageOptim/gifski) is AGPL-3.0-or-later by default and offers separate licensing by arrangement. It is evaluated but not linked or bundled in v0.1.

[libvips](https://www.libvips.org/) is LGPL-2.1-or-later, while optional loaders/encoders bring their own terms. It is evaluated but not included in v0.1.

## Technical intake versus distribution approval

Passing the dossier `--check` means current candidate bytes and technical probe
observations match a canonical file explicitly marked unreviewed. Passing the
bundle schema/verifier means the candidate bytes match the declared
version, configure line, capability inventory, notices, and source-offer files
at that time. It does not prove that those declarations are legally sufficient,
that the distributor had authority, that a source offer is compliant, or that
any use is commercially, patent, LGPL, GPL, or redistribution safe. Those are
separate owner/legal review decisions.

The approval window is limited to 366 days so stale evidence cannot continue to
appear reviewed indefinitely. Any binary/configuration/capability/evidence
change requires a new manifest and independent digest review even before legal
questions are considered.

## Release-sidecar checklist

1. Decide whether the exact candidate may be executed in the current review
   environment, then capture and re-check an unreviewed external dossier.
2. Choose an FFmpeg configuration compatible with the owner's application-license decision.
3. Record the complete configure line and exact upstream source revision.
4. Inventory every enabled external library and its license.
5. Review H.264/H.265/AAC and other codec patent implications for intended distribution regions.
6. Build or acquire through a reproducible, pinned process; publish source/offer and notices required by the effective license.
7. Verify signatures where available and record SHA-256 for each target.
8. Run `pnpm engine:verify --dir <absolute-directory>`, independently review the
   printed manifest digest, and use that exact digest for opt-in packaging.
9. Run `pnpm test:real` against the exact sidecar on each target platform and
   inspect the resulting package contents.
10. Sign the application, installer, and sidecars as required; do not confuse
    code signing with license clearance.

## Current release constraint

Unsigned Morflo builds may discover the compatible development engine locally.
Normal package construction is engine-free, and the opt-in staging gate exists
without an approved candidate or committed digest. A public, normal-use
installer remains blocked on an exact reviewed redistributable engine, the
owner's application-license/distribution decisions, applicable notice/source
obligations, patent review, and signing. This limitation does not block
application implementation or engine-free local packaging.
