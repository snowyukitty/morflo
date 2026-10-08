# Design review log

## Pass 0 — direction selection

- Selected Quiet Precision with a 44/45 rubric score.
- Rejected permanent navigation and dashboard cards; one queue surface plus a contextual inspector supports the product job with less scanning.
- Reserved eucalyptus for decisions and selection, not decoration.
- Kept the native window frame to preserve Windows behavior, DPI handling, and accessibility.

## Pass 1 — production-component review

Evidence: `screenshots/gate1/` at 1440×900, 1280×800, 780×620, and 100/125/150% device scale. Screenshots are local review artifacts and are intentionally ignored by Git.

Findings and changes:

- The initial dark capture was taken before the explicit theme effect settled, producing an unreadable pale selected row. Visual setup now waits for the requested theme and motion to settle; the actual dark tokens produce a deep sage selection with readable text.
- The first GIF inspector spent too much height repeating three full outcome cards, hiding the useful range controls. When GIF is selected, the outcomes now collapse into an MP4/WebM/GIF switch and the inspector receives a little more width. The preset is visible at 1280×800 without turning the screen into a timeline editor.
- The first empty-state orbit used a generic aperture and sparkle. The reviewed Morflo folded-strip mark now anchors both the application header and the local-processing path; a plain check replaces the decorative sparkle.
- Engine implementation detail competed with the primary action. “Local engine ready” is now the main status and the detected engine/version is secondary diagnostics text.
- At 780×620, the queue, international filenames, and conversion action remain readable. The inspector follows below in the same scroll surface rather than compressing both columns into unusable widths.

Remaining for later passes: validate real thumbnail/poster content, long error details, running/cancel transitions, and packaged WebView font metrics rather than browser-only metrics.

## Pass 2 — GIF workbench and real feedback

Evidence: refreshed `screenshots/gate1/video-to-gif-1280x800.png` plus real-engine poster, clip, and GIF probes.

Findings and changes:

- A decorative poster was not sufficient for a signature workflow. The desktop workbench now requests a bounded real frame at the chosen start point, debounced while the range moves; the illustration remains only as a deterministic demo/failure fallback.
- The play control previously implied an action without one. It now creates a local, muted, looping preview of at most three seconds, shows a genuine preparing state, and leaves conversion available if preview decoding fails.
- The original warning treated every 720 px selection as “Very large,” even when it was very short. Guidance now combines duration, FPS, and pixel area, keeping the default Web range calm while escalating genuinely expensive selections.
- Completed GIFs now animate in the success inspector when below a bounded 20 MiB IPC limit. Larger results use precise fallback copy and remain available through Reveal; they are never decoded wholesale in the frontend.
- The 1280×800 capture retains a clear two-second hierarchy: selected result, moment/range, then preset. Technical width/FPS/loop values remain below progressive disclosure.

## Pass 3 — hardening, contrast, and long-session comfort

Evidence: `screenshots/release/` across all eight required product states, the 780×620 minimum, and 100/125/150% device scale. Browser axe checks include computed color contrast in light queue, dark GIF, empty, and error states.

Findings and changes:

- Real browser contrast checks exposed tertiary text between 2.92:1 and 3.94:1 on selected and warning surfaces. The tertiary and warning tokens were deepened while preserving their hierarchy; all serious/critical axe findings now pass in the four representative states.
- At 200% text size the single scroll surface preserves access to Add files and Convert rather than compressing the inspector into an unusable column. At 780 px the inspector follows the queue naturally below the fold.
- Persistent “replace” was too destructive for a quiet default. Preferences now retain only suffix or skip; replacement remains an explicit, visibly warned choice for the current conversion.
- Retry now means real backend reinspection for damaged/unsupported sources and real requeue for conversion failures. A failed item never becomes convertible merely because its Try again control was pressed.
- The image inspector now requests one bounded 512 px local thumbnail for the active source. It preserves alpha, never decodes the full source in the webview, keeps no disk cache, and replaces the prior in-memory preview when selection changes.
- Final empty, queue, active, GIF, success, error, dark, and narrow captures were inspected for truncation, accidental borders, status noise, default-component styling, and ambiguous actions. No material visual defect remained.

## Native Windows delivery review

- The installed MSVC/NSIS build and exact IconFlow desktop shortcut both opened
  a responding `Morflo` WebView2 window with the native frame intact.
- The earlier desktop-control helper remained unavailable, so a narrower,
  repeatable test-only WebView2 CDP route was implemented instead. It controls
  the real release executable without changing production settings.

## Pass 4 — Windows first-run and engine confidence

Evidence: `screenshots/release/engine-diagnostics-*.png` plus the latest ignored
`work/morflo/native-windows/*/01-native-ready.png`,
`02-native-diagnostics.png`, and `03-native-converted.png`.

Findings and changes:

- The old missing-engine alert promised “Open Diagnostics” without providing a
  control or a Diagnostics surface. It is now an explicit action, and the
  footer engine status is a keyboard-operable entry point once files exist.
- The first Diagnostics composition placed a narrow hint beside two actions,
  forcing both button labels onto noisy two-line blocks. The final footer gives
  the requirement its own line and uses a stable two-column action grid.
- Immediate native screenshot capture caught the sheet midway through its
  opacity transition, which looked falsely translucent. The evidence harness
  now waits for the restrained finite transition; product animation remains
  unchanged and reduced-motion support still applies.
- The ready sheet uses one trust summary, two engine facts, and an eight-item
  capability grid. Technical paths stay behind disclosure, while refresh and
  folder selection remain the only primary actions.
- The missing-engine dark state was reviewed separately. Warning, unavailable,
  privacy, focus, and action states remain distinguishable without relying on
  color alone; axe reports no serious/critical findings.
- Native ready and completed states preserve the two-second hierarchy at
  Windows 150% device scale: queue first, selected result second, engine status
  quiet in the footer. Long international filenames remain readable without
  pushing conversion controls out of view.

## Pass 5 — outcome confidence

Evidence: refreshed `screenshots/release/completed-*.png`,
`partial-completion-1280x800.png`, and the latest ignored
`work/morflo/native-windows/*/03-native-converted.png`.

### Refinement 1 — replace the dead terminal state

- The original success panel stated “Converted” but left most of the inspector
  empty and gave no evidence about what had changed. The final receipt shows a
  bounded output preview, original/output formats and bytes, measured size
  change, dimensions/duration, final name, Reveal, and source safety.
- Queue rows and the footer reuse the same verified output facts; no parallel
  frontend estimate or extension-based support claim was introduced.

### Refinement 2 — preserve truth across batch, theme, and size

- The first batch composition risked letting a green success state dominate a
  failed item. The selected-batch receipt now says “1 of 2 outputs is ready,”
  keeps the attention count in a separate text-and-icon band, and directs the
  user to select one result.
- Light and dark screenshots retain hierarchy without additional borders or
  glow. At 780×620, the receipt keeps readable density and Reveal remains
  reachable through the existing single scroll surface.

### Refinement 3 — inspect the real artifact, not the green check

- The first native preview used a checkerboard behind every image result, which
  could falsely imply that JPEG retained transparency. Actual alpha presence is
  now included in the backend-probed output summary; only genuinely transparent
  outputs use the checkerboard.
- Inspecting the JPEG revealed a visually blank result. The converter was
  correctly flattening the file, but the generated source fixture accidentally
  had zero alpha for every pixel. The generator now uses `drawbox:replace=1`,
  and a real-engine test verifies visible foreground survival as well as the
  transparent background color. The refreshed native preview shows the green
  and warm-orange forms on Morflo's chosen background.
- The final 2.1 KB → 4.2 KB result honestly reports “106% larger” in warning
  color. Morflo does not turn every successful conversion into a compression
  claim.

## Pass 6 — moment confidence

Evidence: `screenshots/release/video-to-gif-{1280x800,dark-1280x800,narrow-780x620}.png`
plus the latest ignored
`work/morflo/native-gif-windows/*/{01-native-moments-and-preview,02-native-gif-converted}.png`
and `work/morflo/native-cancellation-windows/*/01-native-canceled.png`.

### Refinement 1 — turn decoration into source evidence

- The old eight-cell strip was an attractive gradient but contained no media
  information. It competed with the real poster while requiring users to infer
  time from two unrelated sliders.
- The final strip shows seven bounded real frames from the session-owned video.
  Selected content stays full strength, excluded content dims, and handles sit
  directly on the content boundary. The deterministic CSS scene exists only in
  visual-demo mode; a production decode failure says “Moments unavailable.”
- The installed Windows capture verifies seven decoded 160 px frames with a
  Chinese/emoji/apostrophe filename, not mocked artwork.

### Refinement 2 — make one range operable in every modality

- Start and End now share one rail while remaining two semantic native sliders.
  Dynamic `min`/`max`, fixed tab order, visible readouts, tenths-of-a-second
  value text, keyboard arrows, handle drag, and nearest-handle rail clicks are
  covered by unit, Playwright, and axe checks.
- Dark mode keeps the outside mask quiet without hiding the real frames. Focus
  receives an explicit ring; forced-colors replaces branded selection and
  handles with system Highlight/Canvas colors.

### Refinement 3 — fix the layout and inspect the real release

- The first 780×620 GIF capture exposed a specificity bug: the wide GIF column
  rule overrode the responsive single-column rule and cropped the queue. The
  narrow rule now explicitly covers the `:has(.gif-inspector)` variant; the
  inspector follows at full width in the existing scroll surface.
- The installed release capture confirmed actual WebView2 font metrics, vivid
  source frames, native video controls, the 0.4–2.4 second band, and the final
  24-frame GIF receipt. Critical range readouts were increased one type step
  after that review.
- A separate installed cancellation capture remains calm and actionable:
  “Canceled,” no false progress, no final output, and no celebratory treatment.

## Pass 7 — one-window arrival

Evidence:
`work/morflo/native-open-with-windows/2026-08-30T14-29-10-433Z/` at the host's
Windows 150% device scale: `01-native-empty.png`,
`02-native-open-with-joined.png`, and `03-native-relative-joined.png`.

### Refinement 1 — use the product surface as feedback

- A second-instance toast would repeat the filename, compete with inspection,
  and create another announcement/focus path. The existing queue is clearer:
  arrivals use one restrained 180 ms opacity/4 px transform and immediately show
  real probing state.
- Reduced motion collapses the transition. No progress or completion state is
  animated before backend evidence exists.

### Refinement 2 — keep the shell invisible after entry

- PNG and MOV files opened together appear as ordinary compatible queue rows;
  a later relative WebP joins the same list. There is no Windows-only banner,
  wizard, or duplicate window.
- The three native captures were inspected for hierarchy, long-name truncation,
  mixed-script rendering, excess status noise, and accidental empty space. The
  Chinese, Japanese, emoji, spaces, and apostrophe names remain legible; the
  selected inspector exposes the full active name.
- The first three-file capture exposed an unearned WebP → WebP default inherited
  from the broad “non-PNG becomes WebP” rule. Morflo is not an image optimizer,
  so the recommendation now gives transparent WebP an alpha-preserving PNG destination
  and opaque WebP an easy-to-share JPEG destination. A pure recommendation test
  and the installed release journey prevent the same-format surprise returning.

### Refinement 3 — preserve the user's desktop affordance

- Silent installer QA initially replaced the custom IconFlow shortcut because
  Tauri creates a Desktop link in silent mode. The reversible harness now uses
  Tauri's no-shortcut switch and restores the pre-existing `.lnk` byte-for-byte
  across uninstall/reinstall.
- IconFlow re-delivered and read back the final direct shortcut with no arguments,
  the installed working directory, and the immutable content-addressed Morflo
  icon. Product UI needed no compensating shortcut-management surface.

## Pass 8 — image outcomes

- Added three vertically stacked outcome buttons above the existing format
  controls. Each names the actual format and quality/dimension change before
  application. Manual controls remain available in the same inspector.
- Selected outcomes have a visible check and an accessible pressed state.
  Descriptions are associated through `aria-describedby`; unavailable outcomes
  explain the missing capability or absence of alpha.
- Inspected the 1280 × 800 light and 780 × 620 dark sharing captures. Text wraps,
  the focus ring remains visible, and the narrow inspector follows the queue
  within the existing scroll surface. The fixed Convert action remains reachable.
- The built-in-engine example distinguishes WebP input from disabled WebP output
  and keeps the transparent image as PNG when Smaller file is applied.
- Screenshot capture states now have individual tests rather than sharing one
  aggregate timeout. Behavioral assertions and per-test deadlines remain intact.
