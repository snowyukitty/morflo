# Test plan

## Layers

1. **Rust unit tests**: compatibility, preset validation, collision naming, argument construction, progress parsing, error mapping, capability parsing, state transitions, and path validation.
2. **Backend real-engine tests**: generated fixtures converted by the discovered engine; outputs exist, probe successfully, match type/dimensions/duration/alpha expectations, and leave no partials.
3. **Frontend tests**: queue selection, compatible bulk settings, progressive disclosure, warnings, disabled states, keyboard behavior, status announcements, and error details.
4. **Product-journey tests**: PNG → JPEG, batch JPEG → WebP, MOV → MP4, trimmed GIF, cancellation, retry, collision suffixing, reveal contract, corrupt input, and international paths.
5. **Visual review**: deterministic states at 1440×900, 1280×800, minimum window, light/dark, active, GIF, complete, and error; three critique passes are logged.
6. **Browser accessibility**: computed contrast and serious/critical axe rules across empty, queue, GIF dark, and error states; 200% text-size operation; 100/125/150% scale captures.
7. **Performance**: cold/warm inspection, first real progress, large-image child-process memory, process-tree cancellation, useful frontend render, and 20-item queue interaction. See [performance observations](performance.md).
8. **Clean build/package**: frozen install, checks, tests, production build, unsigned package, launch smoke test, and offline smoke test where feasible.
9. **Native Windows package**: MSVC/NSIS construction, Defender scan, silent
   install, Authenticode status, Tauri bundle-token traceability, direct desktop
   shortcut read-back/launch, WebView2 process health, Windows job-object
   cancellation, external-tool-safe canonical paths, and a release-executable
   semantic GUI journey over an ephemeral process-local CDP endpoint. The native
   journeys remove inherited engine `PATH` entries and use international source
   filenames. The image route checks the alpha warning, collision/source hashes,
   output probing, decoded receipt, and partial cleanup. The GIF route checks
   seven decoded source moments, dependent Start/End controls, a local clip,
   palette output frames/duration, immutable source, and decoded receipt. The
   cancellation route starts a real 30-second MP4 → WebM encode and requires no
   final output, partial file, or surviving FFmpeg child.
10. **Windows Open with**: generated-hook freshness, exact current-user registry
    lifecycle, existing default/`UserChoice` preservation, safely quoted handler
    commands, uninstall ownership, and Desktop shortcut restoration. A separate
    installed release journey launches two later processes with absolute and
    relative international paths, requires clean handoff into the first queue,
    one process/WebView, and unchanged source hashes.
11. **Reviewed engine intake**: Rust runtime and Node operator-verifier parity
    tests cover strict/unknown-field schema handling, manifest trust-pin
    mismatch, artifact hash/size mismatch, exact engine and ffprobe version,
    configure-line drift, full decoder/encoder/demuxer/muxer/filter mismatch,
    derived-output mismatch, missing notice/source-offer evidence, expired
    review, Unicode paths, traversal, duplicate/untracked files, and
    symlink/reparse rejection. Discovery regressions keep reviewed bundle,
    development, session, system, and unavailable identities separate; a
    present invalid bundle must be terminal for automatic fallback.
12. **Candidate dossier boundary**: Node tests require explicit execution
    acknowledgement, exact root executable names, explicit absolute paths, an
    external exclusive output, canonical deterministic JSON, and a permanently
    unreviewed assurance marker. They cover Unicode paths, full sorted file
    inventory, stable-handle hashing, pre/post-probe mutation, stale evidence,
    output overwrite, malformed/oversized probe evidence, dossier-versus-
    manifest separation, and symlink/reparse rejection. Node and Rust tests
    verify that media-engine children do not inherit search paths, homes,
    credentials, proxies, or unrelated parent variables. Real evidence records
    a temporary unreviewed dossier digest and immediately checks it against the
    same non-redistributable development candidate.
13. **Package intake boundary**: normal Windows packaging clears inherited
    manifest-pin state and uses no resource overlay. Reviewed packaging must
    reject a missing directory/digest pair, verify before and after staging,
    copy only manifest-declared files into the Cargo target, and pass an exact
    temporary Tauri resource map. Package inspection must list no FFmpeg,
    ffprobe, engine manifest, or engine-support file in a normal installer.

Mocks and synthetic engine adapters are labelled `mock` or `synthetic`; no such
result can move a conversion route into the verified matrix, establish a
distributor/license decision, or approve a bundle digest. Real-engine evidence
uses the current local engine only as non-redistributable development evidence.

## Generated fixture categories

- JPEG with EXIF orientation; transparent PNG and WebP; large and unusual-dimension image; ICO; AVIF if encoded; damaged image.
- Short 1080p MP4 with audio; portrait MOV; WebM; MKV; silent video; VFR-like sample; high-motion GIF source; truncated video.
- Spaces, Traditional Chinese, Japanese kana, emoji, apostrophe, long name, collision, unwritable destination, and cancellation.

Fixture generation is deterministic and uses synthetic gradients, shapes, tones, and test signals; no external copyrighted media is included.

## Evidence rules

- Record engine version/configuration and host next to real results.
- State skipped capability-dependent tests as skipped, not passed.
- Capture failure output without private absolute paths in committed artifacts.
- Keep native CDP flags test-only and process-scoped; never persist them or add
  them to packaged configuration.
- Keep Open with as an alternate handler. Tests must fail if install/reinstall
  changes an extension default or `UserChoice`, or if uninstall removes a
  non-Morflo value. Do not substitute registry evidence for the final Explorer
  menu-selection gesture in platform claims.
- A transparency fixture must contain both transparent and visibly nontransparent
  pixels. JPEG flattening tests sample a transparent corner and a foreground
  region so a malformed fixture or content-erasing pipeline cannot pass by
  producing a uniform background.
- Never run a network test by uploading media. Offline behavior is checked by source/dependency audit and local execution.
- Run `pnpm benchmark:gif` when GIF parameters change; compare the palette result with the direct FFmpeg baseline and record both quality proxy and bytes rather than optimizing either in isolation.
- Moment-strip tests must distinguish real engine frames from deterministic UI
  artwork. Probe at least the first and last returned frames, require distinct
  payloads, and keep decode/byte/process limits under regression coverage.
- Engine-bundle tests must distinguish schema/hash/probe logic exercised through
  a synthetic adapter from a real executable. A successful `engine:verify`
  candidate result is still technical integrity evidence; only a separately
  reviewed exact digest can become a compiled reviewed-bundle trust anchor.
- Candidate-dossier tests and output must use `unreviewed`, never `reviewed` or
  `approved`. `--allow-execution` acknowledges that an exact candidate will run;
  it is not a sandbox, malware verdict, provenance proof, or network-isolation
  claim. Dossier files stay outside candidates and packages.
- Capability comparison uses complete sorted inventories, not only the formats
  Morflo currently exposes. Both additions and removals require re-review.
- Ordinary verifier and UI diagnostics must remain bounded and omit absolute
  directory prefixes. Tests may assert stable error codes, not host paths.
