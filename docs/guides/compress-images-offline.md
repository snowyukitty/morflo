# Compress and resize images offline on Windows

Use Morflo to prepare local images for sharing while preserving the original.
JPEG and PNG output work with the built-in image engine. The following controls
are available in the 0.3.0 source checkpoint; see
[getting started](getting-started.md) for availability.

## Make a JPEG smaller

1. Add the JPEG and select it.
2. Choose **Smaller file** under **What do you need?**
3. Review **JPEG**, **Smaller** quality, and **Keep original dimensions**.
4. Convert and compare the actual bytes in the completion receipt.

This deliberately permits JPEG to JPEG: the goal is a useful compressed copy,
not a different extension. Re-encoding can introduce visible artifacts, and
an already compressed source may not shrink. Do not repeatedly compress the
compressed copy; start with the original when trying another setting.

## Resize a large photo for sharing

Choose **Easy to share**. Large images fit inside 1920 × 1920 px as a balanced
JPEG, maintaining aspect ratio. A 4032 × 3024 image becomes 1920 × 1440; a
640 × 420 image retains its dimensions. This outcome does not crop or enlarge.
For another size, use the dimension controls after applying the outcome.

Resizing changes the available detail. Inspect the finished image before
choosing it for printing or another task that needs the full resolution.

## Keep transparent areas

**Smaller file** uses WebP for an image with alpha when that encoder is
available. With the built-in engine it uses PNG and explicitly says **Size may
grow**. **Keep transparency** chooses PNG at the original dimensions.

PNG preserves the decoded pixels but is not a promise of smaller output,
byte-identical metadata or identical color management. To create a JPEG
instead, choose **Easy to share** and review the flattening background.

## Process a batch

Select the images, choose the outcome, then convert. Each image is evaluated
individually; an opaque photo and a transparent graphic do not receive the
same format merely because they share a selection. Inspect any failed items
separately. The Convert action processes all ready queue items.

Outcomes retain metadata policy, animation consent, destination and collision
policy. The default naming adds a numeric suffix to existing outputs.

Morflo does not currently promise a target byte size such as 500 KB or 1 MB.
Try a smaller dimension or quality setting and use the measured result.
