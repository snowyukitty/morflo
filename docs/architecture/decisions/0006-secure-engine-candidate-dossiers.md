# ADR 0006: Secure unreviewed engine-candidate dossiers

- Status: Accepted
- Date: 2026-08-31

## Context

The reviewed bundle gate deliberately requires provenance, license, notice,
source-offer, reviewer, and independent digest decisions that software cannot
make. Reviewers still need a repeatable way to collect the candidate's observed
bytes, build identity, resolved FFmpeg library versions, and capabilities
without copying host paths into evidence or accidentally presenting observation
as approval.

Capability collection necessarily executes the candidate. An unknown binary
can use the current user's OS permissions, inspect inherited process state, use
the network, spawn descendants, or mutate files. Fixed FFmpeg arguments alone
are not a sandbox.

## Decision

Add `src-tauri/engine-candidate-dossier.schema.json` schema version 1 and the
project-owned `pnpm engine:dossier` command.

The dossier command:

1. requires an explicit absolute candidate directory, exactly one explicit
   external `--out` or `--check` file, and the standalone
   `--allow-execution` acknowledgement;
2. accepts only the platform's fixed root `ffmpeg` and `ffprobe` filenames;
3. rejects symlinks, reparse points, canonical escape, special files, empty
   directories, nonportable paths, more than 256 entries, more than eight
   levels, files over 1 GiB, and candidates over 2 GiB;
4. hashes every regular file through a stable open handle and repeats the
   complete sorted inventory after all probes;
5. records matching tool version/configuration, compiler identity, sorted
   FFmpeg library versions, complete decoder/encoder/demuxer/muxer/filter
   inventories, and derived Morflo outputs;
6. emits canonical path-neutral JSON without a generated timestamp, writes
   with exclusive-create semantics, and can later compare exact canonical bytes
   with current candidate evidence; and
7. hard-codes `assurance.status = unreviewed` and a statement that the evidence
   is not approved for bundling, licensing, or redistribution.

The dossier schema and reviewed manifest schema remain separate. There is no
automatic promotion or conversion command. A reviewer must independently
establish provenance, license, notices, source/source-offer obligations,
intended distribution scope, approval identity, validity window, and the final
manifest digest.

All Morflo-owned FFmpeg/ffprobe launches now clear the parent environment and
restore only deterministic `LC_ALL`/`LANG` plus the minimal Windows OS/temp
variables needed by ordinary native processes. Search paths, home directories,
tokens, proxy variables, dynamic-loader overrides, and unrelated application
state are not inherited. Executables are still launched directly with argument
arrays and never through a shell.

## Threat boundary

The controls prevent accidental execution, executable-name substitution,
dossier self-inclusion, output overwrite, path redirection, unnoticed candidate
mutation during observation, credential leakage through inherited environment,
and an unreviewed dossier masquerading as a reviewed manifest.

They do not sandbox the candidate, revoke its user permissions, guarantee that
it makes no network request, terminate every descendant a malicious binary may
detach, prove provenance, or answer copyright/patent/distribution questions.
The explicit execution acknowledgement makes this residual authority visible;
reviewers should execute unknown candidates only in an environment appropriate
to their source and risk.

Same-user filesystem races cannot be eliminated by a portable user-space
command. Stable-handle hashing, canonical checks, pre/post inventories, later
`--check`, manifest hashes, staging copies, and runtime re-verification provide
layered detection rather than a sandbox claim.

## Evidence basis

- [FFmpeg tool documentation](https://ffmpeg.org/ffmpeg.html) defines the
  version, build-configuration, and capability-listing interfaces.
- [ffprobe documentation](https://ffmpeg.org/ffprobe.html) defines structured
  program and library version output.
- [FFmpeg's legal checklist](https://ffmpeg.org/legal.html) treats exact source
  correspondence and build configuration as distribution evidence rather than
  something a binary probe can approve.
- [SLSA provenance v1.1](https://slsa.dev/spec/v1.1/provenance) distinguishes
  artifact digests, build definition, run details, and resolved dependencies.

These references inform evidence shape only. Morflo does not claim FFmpeg,
SLSA, or this ADR grants legal approval.

## Consequences

- Candidate observations are reproducible and reviewable without committing or
  packaging a media engine.
- Adding, removing, replacing, or changing any candidate file or observed
  capability invalidates `--check`.
- Reviewers receive compiler/library evidence without silently converting it
  into provenance or license claims.
- Development, session, system, reviewed-bundle, and unreviewed-dossier
  identities remain distinct.
