# Get started with Morflo

Morflo converts images on your computer without uploading them. Common image
conversion works with the built-in engine; no FFmpeg installation is needed.
Windows 11 x64 is the primary target. Public installers are not yet available;
see [build instructions](../development.md).

## Your first conversion

1. Open Morflo and choose **Choose files**, or drop images into the window.
2. Select a file and review **What do you need?** Choose **Easy to share**,
   **Smaller file**, or **Keep transparency** when available.
3. Check the output format, dimensions and any transparency or animation warning.
4. Choose **Convert**. After completion, inspect the measured size change and
   choose **Reveal in folder**.

The default destination is beside the source. If that output name exists,
Morflo adds a number. The original file stays unchanged.

## More than one image

Choose multiple files, then select their checkboxes or **Select all** before
applying an outcome. Each compatible image gets its own settings. For example,
**Smaller file** uses JPEG for an opaque photo and keeps alpha in a transparent
image. Video settings stay unchanged. The Convert action starts all ready files
in the queue, including ready files outside the current selection.

## Understand availability

With the built-in engine, PNG, JPEG and ICO output are ready. WebP input can
be converted to PNG or JPEG. WebP and AVIF output, video conversion and animated
GIF creation need a compatible local media engine. Unavailable formats are
marked with a reason; Engine Diagnostics explains the active engine.

Do not install an executable from an untrusted source. Morflo's engine
selection checks compatibility, not the safety of an arbitrary downloaded file.

## Common questions

**Will my image become smaller?** The result depends on the source and chosen
format. Morflo measures the actual output. PNG may grow; JPEG compression loses
some detail. See [compress images offline](compress-images-offline.md).

**What happens to transparent areas?** JPEG uses the visible background color.
Choose PNG to retain transparency. The warning appears before conversion.

**What happens to metadata?** The default image policy is Remove. The built-in
engine cannot honor Preserve; that explicit request is refused. Outcomes retain
your metadata choice. Color-profile preservation across formats is best-effort,
not a guarantee of identical color.

**What happens to an animated input?** Still-image conversion requires an
explicit **Use the first frame** choice. Outcomes do not make that choice.

**Can I report a problem without sending my file?** Yes. Describe the format,
version and steps in the [bug form](https://github.com/snowyukitty/morflo/issues/new?template=bug_report.yml).
Remove private paths from diagnostics and screenshots. A synthetic example is
preferable to personal media.
