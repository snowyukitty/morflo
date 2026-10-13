# Finish a batch after an interruption

A failed or canceled file does not erase the other results. Morflo keeps each
successful output in the conversion ledger, with its measured size and Reveal
action. Originals stay unchanged.

## Find what still needs work

Use **Needs attention** above the ledger to show only failed and canceled files.
The number beside each filter is a count for the entire queue. **Completed**
shows successful conversions; **All files** restores the whole queue.

The measured total always compares the same successful-file subset of the whole
queue, even when those rows are hidden. A larger output is still completed; it
is not silently classified as an error.

Selecting a filter removes hidden files from the settings selection. **Select
shown** selects only the rows in the current view. Keyboard Delete also removes
only selected, visible, inactive rows; it does not delete source or output files.

## Prepare, review, convert

1. Select an affected row to read its error. Restore a moved source, choose an
   available destination, or set up the required local media engine as appropriate.
2. In **Needs attention**, choose **Prepare N retries**. This prepares all failed
   and canceled files in the queue, regardless of checkbox selection. For one
   file, use its Retry action instead.
3. Morflo keeps the existing settings for previously inspected files. Files that
   could not be inspected are inspected again, one at a time. If inspection
   still fails, that file stays in Needs attention. A newly inspectable file gets
   defaults based on its current capabilities.
4. Prepared files return to **All files**, selected for review. Check format,
   destination, transparency and animation choices. Preparing retries never starts
   conversion or grants first-frame consent.
5. Choose **Convert**. Its count covers **all ready files in the entire queue**,
   including any hidden or unselected ready files. Completed files are not rerun.

Bulk retry preparation is unavailable while conversion or inspection is active.
An empty filtered view means that no files currently match it; it does not mean
the queue was cleared. Choose **Show all files** to return.

## Common recovery choices

- **Source moved or disconnected:** restore it to its original location before
  retrying, or remove the row and add the file from its new location.
- **Damaged input:** retrying cannot repair corrupt media. Keep the original,
  obtain a readable copy if possible, and add that copy separately.
- **Video engine unavailable:** use **Set up video conversion**, select a trusted
  compatible FFmpeg/ffprobe pair, and retry inspection. Built-in image conversion
  remains available without that pair.
- **Canceled conversion:** review the unchanged settings and prepare a retry.
  Cancellation removes the owned partial output; the original remains available.

See [getting started](getting-started.md) and
[image settings and tradeoffs](compress-images-offline.md).
