# Convert PNG to JPEG and WebP to PNG offline

These everyday image conversions work with Morflo's built-in engine. Your
media stays on your device and source files remain unchanged. See
[getting started](getting-started.md) for the current source checkpoint.

## PNG to JPEG

Add the PNG and choose **JPEG** under Output format. Use **Easy to share** if
you also want a large image fitted within 1920 × 1920 px. For full dimensions,
select **Keep original dimensions** after applying that outcome.

If the PNG has transparency, inspect the **JPEG has no transparency** warning
and choose a background color. JPEG cannot retain transparent pixels. Convert,
inspect the finished output and reveal it in its folder. Numeric suffixing
protects an existing output with the same name.

JPEG is useful for photographs and broad compatibility. PNG can be more
appropriate for transparent graphics, sharp text and screenshots.

## WebP to PNG

Add the WebP and choose **PNG**. A transparent WebP also offers **Keep
transparency**. WebP input needs no external media engine even though WebP
output currently does. Conversion cannot restore detail already lost in the
source, and the PNG can be larger.

An animated WebP requires explicit first-frame consent before still-image
conversion. Morflo does not silently flatten an animation into one image.

## Create an ICO

Choose **ICO** to write one icon containing 16, 32, 48 and 256 px entries. Morflo
fits and pads the image without stretching. This converts an existing image;
it does not design a logo or promise an operating-system association.

## Troubleshooting

An unavailable output has a capability reason. A damaged source should produce
an actionable error rather than a partially published file. Metadata Preserve
requires an external media engine and remains best-effort across formats.
For private reporting guidance, see [Security](../../SECURITY.md).
