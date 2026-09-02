# Publication readiness

Assessed 2026-09-03 against Morflo 0.2.1. This records what has been verified,
what was fixed while preparing, and what only the owner can decide. It is about
making the **source repository** public. Distributing signed binaries is a
separate, larger question tracked in `release-checklist.md`.

Nothing here changes repository visibility. That action requires an explicit
instruction naming it, and this document does not substitute for one.

## Verified clean

| Check                                                        | Result                                                                                         |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------- |
| Credentials, API keys, tokens, private keys in tracked files | None found                                                                                     |
| Email addresses                                              | None found                                                                                     |
| Runtime network calls, analytics, telemetry, updater         | None; `pnpm audit:privacy` passes                                                              |
| npm production advisories                                    | `pnpm audit --prod --audit-level high`: no known vulnerabilities                               |
| Rust advisories                                              | `cargo-audit 0.22.2`, 1,239 advisories over 502 dependencies: **zero vulnerabilities**, exit 0 |
| Push-triggered automation                                    | None. The only workflow is `workflow_dispatch`, so publishing fires nothing                    |
| Committed binaries                                           | No media engine, no build outputs; the installed tree is four files                            |

The seventeen `cargo-audit` warnings are all `unmaintained` or `unsound`, never
a vulnerability. Every affected crate is a build-time helper or part of Tauri's
Linux GTK 3 stack. The one `unsound` entry, `RUSTSEC-2024-0429` against
`glib 0.18.5`, is absent from the Windows dependency graph entirely —
`cargo tree --target x86_64-pc-windows-msvc -i glib` finds nothing — so it
cannot affect the primary release target.

## Fixed while preparing

- Absolute personal paths were replaced with their meaning throughout the
  evidence log. They carried a username and a cloud-storage layout without
  adding evidentiary value.
- References to the owner's private control plane, including a commit
  identifier and registry classifications, were removed. That control plane is
  permanently private and its contents must not reach a public artifact.
- The project instructions stopped hardcoding a drive letter when pointing at
  the workspace contract, which was wrong independently of publication.
- A decode-budget defect in the built-in image engine was found and fixed. See
  the built-in image engine section of `red-team.md`.

## Only the owner can decide

### 1. The root license — decided 2026-09-03

**Resolved: `MIT OR Apache-2.0`.** The owner selected the customary
Rust-ecosystem dual license, which matches the terms of every crate Morflo
compiles. `LICENSE.md`, `LICENSE-MIT`, `LICENSE-APACHE`, the Cargo `license`
field, `package.json`, `README.md`, `AGENTS.md`, and the notices now agree.

The dependency situation never constrained the choice: every compiled
dependency is MIT, Apache-2.0, BSD or Unlicense, and FFmpeg's GPL terms bind
only a distributed FFmpeg binary, of which Morflo ships none.

### 2. Repository history — decided 2026-09-03

The working tree is clean, but publishing a repository publishes its history.
Six commits contain absolute personal paths in earlier revisions of the
evidence log, and two contain the private control plane's name together with a
commit identifier and registry classifications.

None of it is a credential. The paths reveal a username that is already public
as the GitHub account name, plus a redirected-Desktop layout. The control-plane
references are the more meaningful ones, because that repository is permanently
private by policy.

**Resolved: publish a fresh repository from the current tree.** The existing
private repository keeps the complete development record, and the public one
starts from the cleaned tree. Nothing is rewritten and nothing leaks. The cost
is accepted: the public repository has no commit history behind its first
commit, and `STATUS.md` remains the narrative record of how the work happened.

The two rejected options, for the record: publishing the existing repository
directly would expose the history described above, and rewriting history would
invalidate every existing clone and commit identifier in a one-way operation.

### 3. Whether the risk notes should be public

`STATUS.md` and `red-team.md` document security residuals in detail: the
unsigned-executable trust boundary, the dossier command's lack of containment,
the decode budget's fixed ceiling. Publishing them is defensible and arguably
admirable — it is what a security-conscious project looks like — but the
workspace contract names risk notes among the things to keep out of anything
public, so it is a deliberate exception, not a default.

## Publication-time edits, once the decisions above are made

- `README.md` describes "the private canonical repository" in its checkpoint
  section. Accurate today; reword when that stops being true.
- `THIRD_PARTY_NOTICES.md` states that it must be regenerated and reviewed
  against exact lockfiles before public distribution. The license summary is
  current as of 2026-09-03 but has not been through that formal pass.
- ~~Add the chosen `LICENSE` file and reference it from `README.md` and the
  notices.~~ Done 2026-09-03.

## Not blockers for publishing source

These matter for shipping binaries, not for making the repository readable:

- Executables are unsigned; Windows needs a code-signing certificate and macOS
  needs Apple signing and notarization.
- macOS has never been built, and hosted CI has not run.
- A Linux release needs an assessment of Tauri's unmaintained GTK 3 bindings.
- No reviewed redistributable media-engine bundle exists, so video, animated
  GIF, WebP and AVIF still depend on a locally installed FFmpeg.

## Assessment

The repository is technically ready to be read by strangers. The code compiles
clean, the tests are real, the evidence is honest, and there is nothing secret
in it.

It is not ready to be **published**, because publishing without choosing a
license publishes something nobody is allowed to use, and because the history
still names a repository that policy keeps private. Both are decisions, not
work.
