# Acceptance evidence

Evidence was recorded on 2026-08-30. “Real engine” means FFmpeg/ffprobe 8.1.2
executed against synthetic local fixtures; no mock result is used here. Browser
checks exercise the production React surface, while desktop/package evidence is
identified separately.

| Scenario                        | Status                                 | Evidence                                                                                                                                                                                                                                                                                                                                                                                                          |
| ------------------------------- | -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A — effortless image conversion | Verified except native picker gesture  | The installed NSIS app launches with an international PNG through the normal startup-file queue, exposes the recommended JPEG without Advanced settings, converts through the semantic GUI, probes and collision-safely publishes the result, preserves source/existing-output hashes, and exposes Reveal. Browser tests cover the Choose files control; the native OS picker/drop gesture itself remains manual. |
| B — transparency safety         | Verified by layered evidence           | Queue/accessibility tests expose the JPEG alpha warning and background control. The real transparent PNG → JPEG test checks the chosen warm background pixel and confirms the source hash is unchanged.                                                                                                                                                                                                           |
| C — favicon creation            | Verified, real engine                  | PNG, JPEG, and WebP each produce one probeable ICO with 16, 32, 48, and 256 px PNG streams.                                                                                                                                                                                                                                                                                                                       |
| D — image batch                 | Verified, real engine                  | A generated 20-file JPEG batch converts to 20 probeable WebP outputs with per-job completion and intact sources; the production 20-item queue responsiveness check passes.                                                                                                                                                                                                                                        |
| E — common video conversion     | Verified, including installed UI       | Portrait MOV and multi-stream MKV convert to MP4 with stream/chapter/dimension/duration assertions. The installed release starts a real 30-second MP4 → WebM encode, cancels through the visible job action in 328 ms, preserves the source, publishes no final output, leaves no partial, and has no surviving direct FFmpeg child.                                                                              |
| F — GIF extraction              | Verified, including installed UI       | Real MP4, MOV, and WebM ranges produce animated GIFs through cancellable palettegen/paletteuse. The installed release shows seven decoded source moments, selects 0.4–2.4 seconds, decodes a local clip, and creates a 463,498-byte, 540×304, 24-frame, 2.0-second GIF with decoded receipt, immutable source, and zero partial residue.                                                                          |
| G — failure without anxiety     | Verified                               | Truncated media maps to a typed damaged-input error; the UI presents calm primary copy, retry, and collapsed technical details instead of raw console text.                                                                                                                                                                                                                                                       |
| H — output collision            | Verified, real engine                  | An existing JPEG sentinel stays byte-identical while the conversion publishes `collision (2).jpg`; same-format source targeting is also regression tested.                                                                                                                                                                                                                                                        |
| I — offline use                 | Verified within the available boundary | Ten real conversion cases plus the engine-discovery identity regression pass inside an isolated Linux user/network namespace with Cargo offline and again natively on Windows. The production audit finds no network APIs or runtime CDN and validates the IPC-only CSP. The Windows package contains no network-dependent runtime asset; no conversion data is transmitted.                                      |
| J — international paths         | Verified, real engine                  | Paths containing Traditional Chinese, Japanese kana, emoji, spaces, and an apostrophe convert and probe successfully through direct argument-array spawning. Long-name planning and layout are covered; no unsupported success claim is made for every network/removable filesystem.                                                                                                                              |

## Outcome-receipt addendum

The latest installed Windows journey strengthens scenarios A, B, G, H, and J:
the real transparent fixture visibly exposes “JPEG has no transparency” before
conversion; the successful receipt decodes a bounded preview of the published
640×420 JPEG, shows the collision-suffixed international filename, reports the
actual 2.1 KB → 4.2 KB result as 106% larger, and confirms the source and
pre-existing target hashes are unchanged. The real-engine regression samples
both the flattened transparent corner and a visible foreground region.

Selected multi-file browser evidence also shows partial success as “1 of 2
outputs is ready” alongside “1 needs attention”; a success state cannot hide a
failed queue item.

## Moment-confidence addendum

The native GIF journey strengthens scenarios E, F, and J with one semantic
release-WebView path. It validates the seven-frame IPC route, precise dependent
range controls, bounded local clip, palette conversion, post-probe receipt, and
international path handling together. A separate release-WebView cancellation
journey proves that the same video UI terminates real work and cleans output;
it is not inferred from a mock or a Rust-only process test.

## Windows Open with addendum

The current-user NSIS installer registers `Morflo.Media.1` as an alternate
handler for 13 common media extensions. A reversible install/uninstall/reinstall
audit proves that every Morflo candidate/capability value appears and is removed
at the correct phase while all pre-existing extension defaults and Windows
`UserChoice` values remain equivalent. Both registered handler commands quote
the executable and `%1` independently; uninstall leaves no Morflo-owned
association key. The test also restores the existing IconFlow Desktop shortcut
byte-for-byte.

The installed single-window journey starts an empty Morflo, launches a second
real process with a PNG and MOV containing Chinese, Japanese, emoji, spaces, and
an apostrophe, and observes both in the first window after 1,357 ms. That process
exits cleanly. A third process resolves a relative WebP against its activation
directory, receives alpha-preserving PNG with a visible “Keeps transparency”
rationale, and avoids an unexplained WebP → WebP job. Exactly one Morflo process
and one app WebView remain, and all source hashes are unchanged. This is installed
release code, not a mock.

## Engine-free addendum

Scenarios A, B, C, D and H no longer depend on a local media engine being
installed. `src-tauri/tests/native_engine.rs` runs the real
`EngineRuntime::native_only` runtime, the exact one a computer without FFmpeg
receives, and is deliberately not feature-gated so it runs everywhere. It proves
engine-free PNG to JPEG at the original dimensions, a favicon carrying all four
declared icon entries, `collision (2).jpg` published beside a byte-identical
pre-existing output, transparent areas flattened onto the chosen background with
the foreground still visible, and a byte-identical source after every run.

The same suite proves the honest boundaries: a video is declined with a reason
that names the missing media engine rather than crashing, an explicitly
requested metadata preservation is refused with its remedy, an animated source
still requires an explicit animation choice, and every unavailable format
carries a reason. This is engine-free evidence, not a claim that video journeys
work without an engine.

The installed engine-free GUI gesture is unrun. Morflo's discovery reads the
persistent machine and user registry `Path`, and staging that state on this host
would require removing the owner's WinGet FFmpeg installation.

## Acceptance boundary

All media, safety, error, collision, offline, and international-path contracts
have real-engine or computed UI evidence. The installed native app now has
complete semantic GUI image, GIF, and video-cancellation journeys with engine
discovery, inspection, conversion/process termination, finalization, and result
evidence. The native OS picker/drop gesture itself remains manual and is not
inferred from startup-file automation. The Explorer menu-selection gesture was
not run because the native desktop-control pipe was unavailable after bounded
retries; registry lifecycle and the exact installed command handoff pass, but
are not mislabeled as that missing click-path evidence.
