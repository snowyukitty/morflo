# Morflo design system

## Direction study

Three directions were evaluated on the same queue-and-inspector screen, using realistic filenames, metadata, warnings, and job states.

### A. Quiet Precision — selected

- Typography: Segoe UI Variable/system sans, compact labels, generous readable body text.
- Palette: warm mineral canvas, paper surfaces, deep green-gray ink, restrained eucalyptus accent, ochre warnings.
- Spacing: 4 px base with 8/12/16/24/32/48 steps; medium density that remains calm during long jobs.
- Surfaces: quiet tonal layers and hairlines; depth comes from contrast and one restrained shadow, not card stacks.
- Icons: rounded 1.75 px interface strokes; original Morflo mark remains separate from the glyph library.
- Motion: 120–220 ms opacity/transform transitions; none required to understand progress.
- Fit: strongest trust, clarity, and long-session comfort without looking like a generic admin UI.

### B. Editorial Utility

- Typography: stronger size contrast, narrower measure, small uppercase metadata.
- Palette: parchment, ink, muted rust.
- Spacing: larger editorial gaps with denser individual rows.
- Surfaces: mostly flat, ruled dividers, almost no shadow.
- Icons: sparing and compact.
- Motion: cross-fades only.
- Fit: distinctive and readable, but the magazine-like rhythm makes an active queue feel less operational.

### C. Soft Industrial

- Typography: sturdier weights and compact technical numerals.
- Palette: gray-green workbench, pale panels, amber safety accent.
- Spacing: tighter, more data-forward.
- Surfaces: inset controls, stronger dividers, tactile button states.
- Icons: squarer terminals and stronger weight.
- Motion: short mechanical slides.
- Fit: capable and durable, but slightly intimidating for occasional converters and visually heavier during idle states.

## Selection rubric

Scores are 1–5. Durability includes responsiveness, theming, and implementation maintenance.

| Direction         | Clarity | Comfort | Distinctive | Trust | Hierarchy | Accessibility | Platform fit | Durability | Long sessions |  Total |
| ----------------- | ------: | ------: | ----------: | ----: | --------: | ------------: | -----------: | ---------: | ------------: | -----: |
| Quiet Precision   |       5 |       5 |           4 |     5 |         5 |             5 |            5 |          5 |             5 | **44** |
| Editorial Utility |       4 |       5 |           5 |     4 |         4 |             4 |            4 |          4 |             4 |     38 |
| Soft Industrial   |       4 |       3 |           4 |     5 |         4 |             4 |            5 |          5 |             3 |     37 |

## Tokens

- Background layers: warm canvas → application surface → raised/selected surface.
- Text: high-contrast ink, secondary graphite, tertiary muted label.
- Accent: eucalyptus; never used as a gradient-filled primary button.
- Status: pine success, ochre warning, brick danger, slate informational.
- Radii: 8 px controls, 12 px rows, 18 px major surfaces, 26 px empty-state landing area.
- Typography: 12 px labels, 14 px controls/body, 16 px emphasized body, 24–36 px headings.
- Elevation: one low ambient shadow for floating action/surface separation; borders remain subtle.
- Motion: `fast` 120 ms, `standard` 180 ms, `slow` 240 ms; ease-out for entrance and ease-in for exit.

Light mode avoids pure white as a canvas. Dark mode uses deep warm green-black rather than black. Forced-colors keeps native borders and focus indicators. Every state combines color with text and an icon or progress semantics.

## Layout

- Empty and working states share a stable 64 px application header.
- Working layout uses a flexible queue and a 360–400 px contextual inspector.
- The primary action remains in a stable bottom action rail.
- Below 900 px, the inspector becomes a full-width lower section; below 780 px is outside the supported desktop minimum.
- Text truncation uses a single line in rows while the selected inspector exposes the full filename.

## Outcome confidence

- Completion is an evidence state, not a decorative celebration. The inspector
  leads with the actual output, then shows original → output facts, saved name,
  Reveal, and a quiet source-immutability confirmation.
- A smaller result uses success color; a larger result uses restrained warning
  color without implying failure. Size language is calculated only after the
  final output has been probed.
- Checkerboard preview backgrounds appear only when the probed output actually
  contains alpha. Opaque JPEG and video results use a neutral surface so the
  preview never implies transparency that does not exist.
- Multi-selection replaces per-file settings with one batch receipt. Completed,
  failed, and canceled counts remain visible together so partial success cannot
  hide work that still needs attention.
- Preview is bounded supporting evidence, not an editing canvas. Morflo does not
  add A/B scrubbers, zoom tools, or another toolbar after conversion.

## Moment selection

- The GIF range is one visual sentence: real source moments, the selected band,
  Start/End handles, precise readouts, selected duration, then preset.
- Seven local frames are enough to locate a short loop without resembling an
  editing timeline. Unselected content is dimmed; color is not the only range
  cue because the band also has a persistent outline and handles.
- Two native range inputs share the rail. Their tab order never changes, their
  dependent bounds prevent crossing, values expose tenths of a second, and a
  click on empty rail space moves the nearest handle.
- Poster playback confirms up to three seconds of the chosen range. It remains
  separate from the strip so the user can distinguish locating a moment from
  checking motion.

## External file arrival

- Open with is an entrance, not a new screen. A file joins the same queue,
  receives the same inspection state, and leaves selection/settings behavior
  unchanged.
- A 4 px opacity/translate row entrance gives spatial confirmation without a
  toast, badge, or shell-specific copy. It uses the standard 180 ms token and
  collapses under `prefers-reduced-motion`.
- When a later Windows activation arrives, the existing window is restored and
  focused. The queue is the confirmation surface; no decorative success state
  appears before inspection succeeds.
