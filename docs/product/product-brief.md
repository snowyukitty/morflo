# Morflo product brief

## Promise

Morflo turns common image and video conversion into a calm, trustworthy local workflow:

> Drop files → understand the recommended result → convert → reveal the output

Nothing about the source media leaves the device, and the source is immutable.

## Audience

The primary user has a concrete outcome—make this image a JPEG, create a favicon, make this video broadly playable, or extract a short GIF—but does not want to reason about codecs. A secondary advanced user wants accurate, progressively disclosed controls and useful diagnostics.

## Five first-release jobs, in priority order

1. Convert a common image to PNG, JPEG, WebP, or AVIF without damaging the original.
2. Convert a batch of images with one coherent preset and collision-safe naming.
3. Create a valid multi-resolution ICO from a square source.
4. Turn MOV, MKV, or another common video into a broadly compatible MP4 or web-friendly WebM.
5. Extract a short, visually good animated GIF using a simple range and outcome preset.

## Product principles that affect implementation

- Recommendations lead; technical options follow.
- Every status is truthful. Progress comes from the engine, never a timer.
- Destructive behavior is opt-in. Numeric suffixing is the default collision policy.
- Inspection precedes planning. Alpha, animation, stream, chapter, subtitle, rotation, and HDR risks become visible warnings.
- Mixed queues remain mixed. Settings are applied only to compatible selected items.
- One primary surface is enough. Queue and contextual inspector replace navigation pages and ordinary-flow modals.
- Unsupported capability is disabled with a reason, not advertised and allowed to fail later.

## Success criteria

- A new user can finish PNG → JPEG without opening Advanced.
- JPEG selection for an alpha image always exposes background flattening.
- One output action is obvious after completion.
- Keyboard users can complete the primary flow.
- Corrupt files fail with an actionable category and optional technical details.
- A disconnected machine can inspect and convert supported local files.
- A machine with no media engine installed can still finish jobs one, two and
  three: convert an image, convert a batch, and create a favicon.
