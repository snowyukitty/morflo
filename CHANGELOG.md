# Changelog

## 0.3.0 source checkpoint

### Added

- Image outcomes for Easy to share, Smaller file and Keep transparency.
- Sharing fits known large images within 1920 × 1920 px without enlargement
  or cropping. Smaller-file processing preserves alpha and explains PNG growth.
- Per-image outcome resolution in mixed selections, with unchanged video,
  metadata, animation, destination and collision choices.
- English guides for offline compression, resizing, PNG to JPEG and WebP to
  PNG, plus contribution guidance and privacy-conscious issue forms.

### Fixed

- Default image recommendations now use the active capability registry. JPEG
  input no longer defaults to unavailable WebP on a computer without FFmpeg.
- Intake and inspection retry obtain current capabilities before choosing defaults.
- The version badge reads the package version instead of a hard-coded 0.1.
- Windows packaging validates the current version's installer while preserving
  older installer artifacts in the same output directory.
- Patched development-tool transitive dependencies and expanded the quality
  gate's npm audit to include development dependencies.
- README and distribution guidance distinguish built-in images from optional
  media-engine capabilities and no longer treat the application license as undecided.

This is a source checkpoint, not a published signed installer release.

## 0.2.2

- Compatibility retry for legacy ffprobe metadata sections.
- Legacy video frame-synchronization flags and native image-inspection fallback.
- Capability-aware fixture generation and version-aware installer testing.

## 0.2.1

- Header-based decode budgets protect the built-in engine from oversized
  declared image surfaces before materializing them.

## 0.2.0

- Built-in image inspection and PNG, JPEG and multi-resolution ICO output.
- Common image work can run without an external media engine.
