# ADR 0003: Windows engine readiness and native testing

- Status: Accepted
- Date: 2026-08-30

## Context

A desktop process inherits a copy of its parent's environment. A long-running
Windows shell such as Explorer can therefore launch Morflo with an older
`PATH` even after WinGet has installed FFmpeg and updated the persistent user
environment. The shell could resolve the tools while the installed desktop
shortcut still reported “Media engine not found.”

The release also needed native GUI evidence without weakening Windows policy,
changing registry state, adding a production automation plugin, or depending
on WSL/WSLg.

Primary references:

- [Microsoft: User Environment Variables](https://learn.microsoft.com/en-us/windows/win32/shell/user-environment-variables) documents per-process inherited environment blocks.
- [Microsoft: RegGetValueW](https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-reggetvaluew) documents bounded Unicode registry reads and automatic `REG_EXPAND_SZ` expansion unless explicitly disabled.
- [Microsoft: Debug WebView2 with Visual Studio Code](https://learn.microsoft.com/en-us/microsoft-edge/webview2/how-to/debug-visual-studio-code) documents process-scoped `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` remote debugging.
- [Tauri: WebDriver testing](https://v2.tauri.app/develop/tests/webdriver/) recommends WebdriverIO for a future cross-platform desktop suite.

## Decision

1. Engine discovery remains capability-driven and checks, in order: a folder
   chosen for this session, explicit development overrides, reviewed bundled
   locations, project-local tools, the inherited process `PATH`, then the
   current Windows machine and user `Path` registry values.
2. Windows registry access is read-only through `RegGetValueW`. Morflo never
   invokes a shell, writes the registry, broadcasts environment changes, or
   exposes detected paths to the ordinary UI.
3. A selected engine folder must contain regular `ffmpeg` and `ffprobe`
   executables with permitted names. Both tools are launched directly,
   versions must match, and output capability is probed before the selection
   replaces the current session runtime.
4. Capability discovery is refreshable. A failed first check is not cached for
   the lifetime of the application.
5. Engine-folder selection is session-only and is never added to filename
   history or durable preferences.
6. Native Windows smoke testing uses the existing Playwright dependency and a
   process-local WebView2 CDP flag on an ephemeral loopback port. The harness
   launches the real release/installed executable, strips inherited FFmpeg
   `PATH` entries, drives semantic GUI controls, performs a real conversion,
   and terminates the test process in `finally`.
7. The CDP flag is test-only. It is absent from production configuration and
   does not change Windows registry or WebView2 policy. WebdriverIO remains the
   preferred later route when cross-platform packaged automation justifies its
   test-only plugin and maintenance cost.

## Consequences

- A newly installed engine becomes discoverable from a desktop shortcut
  without signing out or restarting Explorer.
- Users have an actionable Diagnostics surface instead of a dead-end warning.
- Packaged-native UI and conversion evidence is distinct from deterministic
  browser design-state evidence.
- The native CDP harness is intentionally Windows-only; macOS and Linux still
  require their own packaged automation validation.
- No engine binary is bundled, and this decision does not alter the media
  engine licensing boundary.
