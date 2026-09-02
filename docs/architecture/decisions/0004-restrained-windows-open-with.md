# ADR 0004: restrained Windows Open with intake

Status: accepted, 2026-08-30

## Context

Opening a common media file directly from Explorer can remove a complete picker
round trip. The unsafe shortcut is to declare normal Tauri file associations:
the generated Windows installer writes the extension's default ProgID, which is
broader than Morflo needs and can surprise the user. Windows defaults are a user
choice, and Morflo must remain useful without shell integration.

Relevant primary guidance:

- [Microsoft Default Programs](https://learn.microsoft.com/en-us/windows/win32/shell/default-programs)
- [Microsoft Open With registration](https://learn.microsoft.com/en-us/windows/win32/shell/how-to-include-an-application-on-the-open-with-dialog-box)
- [Microsoft file-association best practices](https://learn.microsoft.com/en-us/windows/win32/shell/fa-best-practices)
- [Tauri single-instance plugin](https://v2.tauri.app/plugin/single-instance/)
- [Tauri NSIS installer hooks](https://v2.tauri.app/distribute/windows-installer/)

## Decision

1. Keep one reviewed JSON manifest for the Windows candidate set and generate
   NSIS hooks from it. A stale generated hook fails `pnpm check`.
2. Register versioned ProgID `Morflo.Media.1`, `OpenWithProgids`,
   `RegisteredApplications`, `SupportedTypes`, and safely quoted direct-open
   commands under the current user only.
3. Never write an extension default, `UserChoice`, or a shell command string
   containing untrusted data. Set `AllowSilentDefaultTakeOver` on the ProgID.
4. Advertise only PNG, JPG/JPEG, WebP, BMP, TIF/TIFF, AVIF, ICO, MP4, MOV, MKV,
   and WebM. Actual conversion choices remain capability-probed after intake.
5. Register the single-instance plugin first. Later processes pass arguments and
   their working directory to the existing Rust state, then exit. Accept only
   existing regular non-symlink files, never scan folders, deduplicate one
   activation, and cap pending intake at 512 paths.
6. React subscribes before its first drain and serializes subsequent drains.
   Arrival is communicated by the queue's own row transition, not a redundant
   toast or fabricated progress state.
7. Uninstall removes only Morflo-owned values/keys and refreshes Shell. The
   reversible installer test snapshots defaults and `UserChoice` before install,
   audits all three lifecycle phases, and preserves an existing Desktop shortcut
   byte-for-byte.

## Consequences

- Explorer remains a convenient entrance, never the product's information
  architecture or an implicit default-app claim.
- The registry candidate list is conservative while decoder/output availability
  remains runtime capability-driven.
- Windows packaging owns the registration detail; Rust's file-intake model and
  React queue remain platform-neutral for future macOS/Linux open-file events.
- An actual Explorer menu-selection gesture still requires native GUI evidence.
  Registry lifecycle plus real installed-process handoff are automated and must
  not be mislabeled as that final gesture.
