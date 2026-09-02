//! Morflo's built-in image engine.
//!
//! Morflo's media engine is FFmpeg, which the project cannot redistribute under
//! its licensing constraints, so a computer without a local FFmpeg used to be a
//! dead end: nothing could be inspected and nothing could be converted. This
//! module is a second engine compiled directly into Morflo from permissively
//! licensed Rust crates, so the common image journeys always work.
//!
//! It is deliberately narrower than FFmpeg and says so. It encodes PNG, JPEG
//! and multi-resolution ICO, which cover converting a common image, converting
//! a batch, and creating a favicon. WebP and AVIF output stay with the media
//! engine: the available pure-Rust WebP encoder is lossless only, so a photo
//! would grow instead of shrink, and advertising a quality control that cannot
//! act would break Morflo's promise that an offered capability actually works.
//!
//! Video, animated GIF creation, and every frame-accurate journey remain media
//! engine work. This module never spawns a process and never touches the
//! network.

use std::{
    fs::File,
    io::{BufWriter, Read as _},
    path::{Path, PathBuf},
};

use image::{
    ColorType, DynamicImage, ImageDecoder as _, ImageEncoder as _, ImageFormat, ImageReader, Rgba,
    RgbaImage,
    codecs::{
        ico::{IcoEncoder, IcoFrame},
        jpeg::JpegEncoder,
        png::{CompressionType, FilterType as PngFilterType, PngEncoder},
    },
    imageops::FilterType,
    metadata::Orientation,
};

use crate::domain::{
    ConversionError, ConversionErrorCode, EngineInfo, EngineSource, FormatCapability,
    ImageSettings, MetadataPolicy, OutputFormat, Quality, ResizeMode,
};

/// Windows icon sizes Morflo writes into every ICO, matching the media-engine
/// plan so both engines produce the same four-entry result.
pub const ICO_SIZES: [u32; 4] = [16, 32, 48, 256];

/// Formats this engine can write without a media engine.
pub const NATIVE_OUTPUTS: [OutputFormat; 3] =
    [OutputFormat::Png, OutputFormat::Jpeg, OutputFormat::Ico];

/// Upper bound on a decoded surface, mirroring the media-engine probe guard.
const MAX_SOURCE_DIMENSION: u32 = 100_000;

/// Ceiling for one decoded image. A larger source is refused with a clear
/// message rather than being allowed to exhaust memory.
///
/// This budget is enforced by Morflo, not by the decoding crate. `image`'s
/// `Limits::max_alloc` does not reach the PNG decoder's own buffers — its
/// `set_limits` checks dimensions only and carries an upstream TODO about
/// constraining internal allocation. Relying on it would leave the declared
/// dimension guard as the sole protection, and 100,000 x 100,000 RGBA is 40 GB,
/// which a file of a few hundred bytes can ask for. Morflo opens files other
/// people produced, so the budget is checked here against the header before any
/// surface is materialized.
const MAX_DECODE_ALLOCATION: u64 = 768 * 1024 * 1024;

/// Bytes one pixel occupies once decoded to RGBA8, the widest form Morflo
/// materializes.
const DECODED_BYTES_PER_PIXEL: u64 = 4;

/// Bytes of a file header scanned for animation markers.
const HEADER_SCAN_BYTES: usize = 64 * 1024;

/// Facts the built-in engine can establish about an image without a media
/// engine. Deliberately smaller than an FFprobe result: there are no streams,
/// chapters or codecs to report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeImageFacts {
    pub width: u32,
    pub height: u32,
    pub has_alpha: bool,
    pub animated: bool,
    /// Container label shaped like the FFprobe `format_name` Morflo already
    /// carries, so downstream code does not need a second vocabulary.
    pub format_name: &'static str,
    pub codec_name: &'static str,
}

/// One image conversion the built-in engine can perform end to end.
#[derive(Debug, Clone)]
pub struct NativeImageTask {
    pub source: PathBuf,
    pub target: PathBuf,
    pub format: OutputFormat,
    pub settings: ImageSettings,
}

/// Whether the built-in engine can write this format at all.
pub fn supports(format: OutputFormat) -> bool {
    NATIVE_OUTPUTS.contains(&format)
}

/// Capabilities to report when no media engine is present. Formats the built-in
/// engine cannot write are listed as unavailable with the reason, never hidden,
/// so the interface can disable them with an explanation.
pub fn outputs() -> Vec<FormatCapability> {
    OutputFormat::ALL
        .into_iter()
        .map(|format| FormatCapability {
            format,
            available: supports(format),
            reason: if supports(format) {
                None
            } else {
                Some(unavailable_reason(format).to_owned())
            },
        })
        .collect()
}

fn unavailable_reason(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Webp => {
            "Needs a local media engine. Morflo's built-in engine can only write WebP losslessly, which would enlarge most photographs."
        }
        OutputFormat::Avif => "Needs a local media engine with Morflo's validated AV1 encoder.",
        OutputFormat::Mp4 | OutputFormat::Webm | OutputFormat::Gif => {
            "Needs a local media engine. Morflo's built-in engine converts images only."
        }
        OutputFormat::Png | OutputFormat::Jpeg | OutputFormat::Ico => {
            "Available from Morflo's built-in image engine."
        }
    }
}

/// Engine identity shown when Morflo is running on the built-in engine alone.
pub fn engine_info(diagnostic: Option<String>) -> EngineInfo {
    EngineInfo {
        available: true,
        name: "Morflo built-in image engine".to_owned(),
        version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        source: EngineSource::Native,
        diagnostic,
    }
}

/// Read an image header and report what Morflo needs for planning.
///
/// Returns `None` when the file is not an image this engine decodes, which is
/// the signal to fall back to the media engine rather than an error: a video or
/// an AVIF is not a failure here, just not this engine's work.
pub fn inspect(path: &Path) -> Option<NativeImageFacts> {
    let reader = ImageReader::open(path).ok()?.with_guessed_format().ok()?;
    let format = reader.format()?;
    let (format_name, codec_name) = format_labels(format)?;

    let mut reader = reader;
    reader.limits(decode_limits());
    let decoder = reader.into_decoder().ok()?;
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 || width > MAX_SOURCE_DIMENSION || height > MAX_SOURCE_DIMENSION {
        return None;
    }
    let has_alpha = decoder.color_type().has_alpha();

    Some(NativeImageFacts {
        width,
        height,
        has_alpha,
        animated: detects_animation(path, format),
        format_name,
        codec_name,
    })
}

/// Map a decoded container to the labels Morflo already displays. Formats the
/// built-in engine should not claim return `None` so the media engine keeps
/// ownership of them.
fn format_labels(format: ImageFormat) -> Option<(&'static str, &'static str)> {
    match format {
        ImageFormat::Png => Some(("png", "png")),
        ImageFormat::Jpeg => Some(("jpeg", "mjpeg")),
        ImageFormat::WebP => Some(("webp", "webp")),
        ImageFormat::Gif => Some(("gif", "gif")),
        ImageFormat::Bmp => Some(("bmp", "bmp")),
        ImageFormat::Tiff => Some(("tiff", "tiff")),
        ImageFormat::Ico => Some(("ico", "ico")),
        _ => None,
    }
}

fn decode_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_DIMENSION);
    limits.max_image_height = Some(MAX_SOURCE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOCATION);
    limits
}

/// Detect animation from container markers instead of decoding every frame.
///
/// Morflo refuses to silently pick one frame out of an animation, so this
/// answer gates a real user decision and must not be guessed.
fn detects_animation(path: &Path, format: ImageFormat) -> bool {
    let Some(header) = read_header(path) else {
        return false;
    };
    match format {
        // An APNG declares `acTL` before the first `IDAT`.
        ImageFormat::Png => find_before(&header, b"acTL", b"IDAT"),
        // An animated WebP carries an `ANIM` chunk in its RIFF header.
        ImageFormat::WebP => header.windows(4).any(|window| window == b"ANIM"),
        ImageFormat::Gif => gif_has_multiple_frames(&header),
        _ => false,
    }
}

fn read_header(path: &Path) -> Option<Vec<u8>> {
    let mut file = File::open(path).ok()?;
    let mut buffer = vec![0_u8; HEADER_SCAN_BYTES];
    let mut filled = 0;
    while filled < buffer.len() {
        match file.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(count) => filled += count,
            Err(_) => return None,
        }
    }
    buffer.truncate(filled);
    Some(buffer)
}

fn find_before(header: &[u8], needle: &[u8], boundary: &[u8]) -> bool {
    let needle_at = header
        .windows(needle.len())
        .position(|window| window == needle);
    let boundary_at = header
        .windows(boundary.len())
        .position(|window| window == boundary);
    match (needle_at, boundary_at) {
        (Some(needle_at), Some(boundary_at)) => needle_at < boundary_at,
        (Some(_), None) => true,
        _ => false,
    }
}

/// Walk GIF blocks far enough to see whether a second image descriptor exists.
///
/// A GIF's frame count is not in its header, so this follows the block
/// structure. It stops at the end of the scanned prefix and reports what it
/// actually saw, treating a truncated walk as "not proven animated".
fn gif_has_multiple_frames(header: &[u8]) -> bool {
    // Screen descriptor is 13 bytes; a global color table may follow.
    let Some(&packed) = header.get(10) else {
        return false;
    };
    let mut cursor = 13;
    if packed & 0b1000_0000 != 0 {
        cursor += 3 * (1_usize << ((packed & 0b0000_0111) + 1));
    }

    let mut descriptors = 0_u32;
    while cursor < header.len() {
        match header[cursor] {
            // Image descriptor.
            0x2C => {
                descriptors += 1;
                if descriptors > 1 {
                    return true;
                }
                let Some(&local) = header.get(cursor + 9) else {
                    return false;
                };
                cursor += 10;
                if local & 0b1000_0000 != 0 {
                    cursor += 3 * (1_usize << ((local & 0b0000_0111) + 1));
                }
                // LZW minimum code size, then sub-blocks.
                cursor += 1;
                let Some(next) = skip_sub_blocks(header, cursor) else {
                    return false;
                };
                cursor = next;
            }
            // Extension block.
            0x21 => {
                cursor += 2;
                let Some(next) = skip_sub_blocks(header, cursor) else {
                    return false;
                };
                cursor = next;
            }
            // Trailer or anything unexpected.
            _ => return false,
        }
    }
    false
}

fn skip_sub_blocks(header: &[u8], mut cursor: usize) -> Option<usize> {
    loop {
        let &size = header.get(cursor)?;
        cursor += 1;
        if size == 0 {
            return Some(cursor);
        }
        cursor += usize::from(size);
        if cursor > header.len() {
            return None;
        }
    }
}

/// Convert one image into `task.target`.
///
/// The caller owns `target` as an exclusively created partial file, so this
/// writes into that existing path and never chooses a name or publishes a
/// result. Blocking work; run it off the async runtime.
pub fn convert(task: &NativeImageTask) -> Result<(), ConversionError> {
    if !supports(task.format) {
        return Err(ConversionError::new(
            ConversionErrorCode::UnsupportedCodec,
            "This output is unavailable",
            unavailable_reason(task.format),
        ));
    }

    let image = decode(&task.source)?;
    let resized = apply_resize(image, &task.settings)?;

    match task.format {
        OutputFormat::Png => write_png(&resized, task),
        OutputFormat::Jpeg => write_jpeg(&resized, task),
        OutputFormat::Ico => write_ico(&resized, task),
        _ => Err(ConversionError::new(
            ConversionErrorCode::UnsupportedCodec,
            "This output is unavailable",
            unavailable_reason(task.format),
        )),
    }
}

fn decode(path: &Path) -> Result<DynamicImage, ConversionError> {
    let reader = ImageReader::open(path)
        .map_err(|error| decode_error("This image could not be opened", &error.to_string()))?
        .with_guessed_format()
        .map_err(|error| decode_error("This image could not be read", &error.to_string()))?;
    let mut reader = reader;
    reader.limits(decode_limits());
    let mut decoder = reader
        .into_decoder()
        .map_err(|error| decode_error("This image could not be decoded", &error.to_string()))?;
    let (width, height) = decoder.dimensions();
    within_decode_budget(width, height)?;
    // EXIF orientation is metadata, but ignoring it would rotate the visible
    // result, so it is baked into the pixels the same way a viewer shows them.
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder)
        .map_err(|error| decode_error("This image could not be decoded", &error.to_string()))?;
    image.apply_orientation(orientation);
    Ok(image)
}

/// Refuse a surface Morflo would have to materialize beyond its budget.
///
/// The header is cheap to read and states the dimensions, so a decompression
/// bomb is rejected before a single pixel is allocated. The message names the
/// media engine because FFmpeg streams such an image instead of holding it
/// whole, so that path can still convert it.
fn within_decode_budget(width: u32, height: u32) -> Result<(), ConversionError> {
    let required = u64::from(width)
        .saturating_mul(u64::from(height))
        .saturating_mul(DECODED_BYTES_PER_PIXEL);
    if required > MAX_DECODE_ALLOCATION {
        return Err(ConversionError::new(
            ConversionErrorCode::UnsupportedFormat,
            "This image is too large for Morflo's built-in engine",
            "Install a compatible FFmpeg build, or choose an engine folder in Diagnostics, to convert images this large.",
        )
        .with_details(format!(
            "{width}x{height} needs about {required} bytes decoded; the built-in allowance is {MAX_DECODE_ALLOCATION} bytes."
        )));
    }
    Ok(())
}

fn decode_error(title: &str, details: &str) -> ConversionError {
    ConversionError::new(
        ConversionErrorCode::DamagedInput,
        title,
        "The file appears incomplete, damaged, or in a form Morflo's built-in engine cannot read.",
    )
    .with_details(details.to_owned())
}

fn encode_error(details: &str) -> ConversionError {
    ConversionError::new(
        ConversionErrorCode::EngineFailed,
        "The converted image could not be written",
        "Try the conversion again or choose another output folder.",
    )
    .with_details(details.to_owned())
}

/// Resize using the same modes and bounds as the media-engine plan, so a
/// setting means the same thing whichever engine runs it.
fn apply_resize(
    image: DynamicImage,
    settings: &ImageSettings,
) -> Result<DynamicImage, ConversionError> {
    let (width, height) = (image.width(), image.height());
    let resized = match settings.resize_mode {
        ResizeMode::Original => return Ok(image),
        ResizeMode::Width => {
            let target = validate_dimension(settings.width, "width")?;
            let scaled = scale_other(height, width, target);
            image.resize_exact(target, scaled, FilterType::Lanczos3)
        }
        ResizeMode::Height => {
            let target = validate_dimension(settings.height, "height")?;
            let scaled = scale_other(width, height, target);
            image.resize_exact(scaled, target, FilterType::Lanczos3)
        }
        ResizeMode::Percentage => {
            let percentage = settings.percentage.ok_or_else(|| {
                ConversionError::invalid("Enter a scale percentage between 1 and 400.")
            })?;
            if !(1..=400).contains(&percentage) {
                return Err(ConversionError::invalid(
                    "Scale percentage must be between 1 and 400.",
                ));
            }
            let factor = f64::from(percentage) / 100.0;
            let target_width = scaled_dimension(width, factor);
            let target_height = scaled_dimension(height, factor);
            image.resize_exact(target_width, target_height, FilterType::Lanczos3)
        }
        ResizeMode::Contain => {
            let target_width = validate_dimension(settings.width, "width")?;
            let target_height = validate_dimension(settings.height, "height")?;
            image.resize(target_width, target_height, FilterType::Lanczos3)
        }
        ResizeMode::Cover => {
            let target_width = validate_dimension(settings.width, "width")?;
            let target_height = validate_dimension(settings.height, "height")?;
            image.resize_to_fill(target_width, target_height, FilterType::Lanczos3)
        }
    };
    Ok(resized)
}

/// Preserve aspect ratio the way the media-engine `scale=w:-2` filter does,
/// keeping the derived side even and at least one pixel.
fn scale_other(other: u32, driver: u32, target: u32) -> u32 {
    if driver == 0 {
        return 1;
    }
    let exact = f64::from(other) * f64::from(target) / f64::from(driver);
    let rounded = exact.round().max(1.0) as u32;
    (rounded & !1).clamp(2, u32::MAX - 1)
}

fn scaled_dimension(value: u32, factor: f64) -> u32 {
    let scaled = (f64::from(value) * factor).trunc();
    (scaled.max(1.0) as u32).clamp(1, MAX_SOURCE_DIMENSION)
}

fn validate_dimension(value: Option<u32>, label: &str) -> Result<u32, ConversionError> {
    let value =
        value.ok_or_else(|| ConversionError::invalid(format!("Enter an output {label}.")))?;
    if !(1..=32_768).contains(&value) {
        return Err(ConversionError::invalid(format!(
            "Output {label} must be between 1 and 32,768 pixels."
        )));
    }
    Ok(value)
}

fn write_png(image: &DynamicImage, task: &NativeImageTask) -> Result<(), ConversionError> {
    let compression = match task.settings.quality {
        Quality::Smaller => CompressionType::Best,
        Quality::Balanced => CompressionType::Default,
        Quality::Best => CompressionType::Default,
    };
    let rgba = image.to_rgba8();
    let writer = BufWriter::new(open_target(&task.target)?);
    PngEncoder::new_with_quality(writer, compression, PngFilterType::Adaptive)
        .write_image(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
            ColorType::Rgba8.into(),
        )
        .map_err(|error| encode_error(&error.to_string()))
}

fn write_jpeg(image: &DynamicImage, task: &NativeImageTask) -> Result<(), ConversionError> {
    // JPEG has no alpha, so a transparent source is flattened onto the chosen
    // background before encoding rather than losing its visible content.
    let flattened = flatten(image, parse_background(&task.settings.background)?);
    let quality = match task.settings.quality {
        Quality::Smaller => 78,
        Quality::Balanced => 85,
        Quality::Best => 93,
    };
    let mut writer = BufWriter::new(open_target(&task.target)?);
    JpegEncoder::new_with_quality(&mut writer, quality)
        .encode(
            flattened.as_raw(),
            flattened.width(),
            flattened.height(),
            ColorType::Rgb8.into(),
        )
        .map_err(|error| encode_error(&error.to_string()))
}

/// Write the same four-entry icon the media-engine plan produces: each size is
/// contained inside the square and centered on transparent padding.
fn write_ico(image: &DynamicImage, task: &NativeImageTask) -> Result<(), ConversionError> {
    let mut frames = Vec::with_capacity(ICO_SIZES.len());
    for size in ICO_SIZES {
        let contained = image.resize(size, size, FilterType::Lanczos3).to_rgba8();
        let mut canvas = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 0]));
        let offset_x = (size - contained.width()) / 2;
        let offset_y = (size - contained.height()) / 2;
        image::imageops::replace(
            &mut canvas,
            &contained,
            i64::from(offset_x),
            i64::from(offset_y),
        );
        let frame = IcoFrame::as_png(canvas.as_raw(), size, size, ColorType::Rgba8.into())
            .map_err(|error| encode_error(&error.to_string()))?;
        frames.push(frame);
    }
    let writer = BufWriter::new(open_target(&task.target)?);
    IcoEncoder::new(writer)
        .encode_images(&frames)
        .map_err(|error| encode_error(&error.to_string()))
}

/// Open the caller's already-reserved partial file for writing.
///
/// The path was created exclusively by the output reservation, so this
/// truncates that owned file instead of creating a new one.
fn open_target(path: &Path) -> Result<File, ConversionError> {
    File::create(path).map_err(|error| {
        ConversionError::new(
            ConversionErrorCode::PermissionDenied,
            "Morflo cannot write to this folder",
            "Choose another output folder or update its permissions.",
        )
        .with_details(error.to_string())
    })
}

fn flatten(image: &DynamicImage, background: [u8; 3]) -> image::RgbImage {
    let rgba = image.to_rgba8();
    let mut flattened = image::RgbImage::new(rgba.width(), rgba.height());
    for (target, source) in flattened.pixels_mut().zip(rgba.pixels()) {
        let alpha = f32::from(source.0[3]) / 255.0;
        for (channel, under) in background.iter().enumerate() {
            let over = f32::from(source.0[channel]) * alpha;
            let under = f32::from(*under) * (1.0 - alpha);
            target.0[channel] = (over + under).round().clamp(0.0, 255.0) as u8;
        }
    }
    flattened
}

fn parse_background(background: &str) -> Result<[u8; 3], ConversionError> {
    let value = background.strip_prefix('#').unwrap_or(background);
    if value.len() != 6 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(ConversionError::invalid(
            "Choose a valid six-digit background color.",
        ));
    }
    let mut channels = [0_u8; 3];
    for (index, channel) in channels.iter_mut().enumerate() {
        let start = index * 2;
        *channel = u8::from_str_radix(&value[start..start + 2], 16)
            .map_err(|_| ConversionError::invalid("Choose a valid six-digit background color."))?;
    }
    Ok(channels)
}

/// Longest edge of a preview image, matching the media-engine preview box so
/// the interface receives the same shape from either engine.
pub const PREVIEW_EDGE: u32 = 512;

/// Render a bounded PNG preview of an image.
///
/// Used for the outcome receipt when no media engine is present, so a
/// successful conversion still shows what was produced instead of degrading to
/// a filename. Blocking work; run it off the async runtime.
pub fn render_preview_png(path: &Path, max_bytes: usize) -> Result<Vec<u8>, ConversionError> {
    let image = decode(path)?;
    let bounded = if image.width() > PREVIEW_EDGE || image.height() > PREVIEW_EDGE {
        image.resize(PREVIEW_EDGE, PREVIEW_EDGE, FilterType::Lanczos3)
    } else {
        image
    };
    let rgba = bounded.to_rgba8();
    let mut encoded = Vec::new();
    PngEncoder::new_with_quality(
        &mut encoded,
        CompressionType::Default,
        PngFilterType::Adaptive,
    )
    .write_image(
        rgba.as_raw(),
        rgba.width(),
        rgba.height(),
        ColorType::Rgba8.into(),
    )
    .map_err(|error| encode_error(&error.to_string()))?;

    if encoded.len() > max_bytes {
        return Err(ConversionError::new(
            ConversionErrorCode::EngineFailed,
            "The preview was larger than Morflo allows",
            "The file itself is unaffected and remains available.",
        ));
    }
    Ok(encoded)
}

/// Whether the requested metadata policy can be honored by this engine.
///
/// The built-in encoders write no source metadata, so `Preserve` cannot be
/// satisfied. Morflo states that instead of silently dropping the request.
pub fn honors_metadata(policy: MetadataPolicy) -> bool {
    matches!(policy, MetadataPolicy::Remove)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};
    use tempfile::tempdir;

    fn settings(format: OutputFormat) -> ImageSettings {
        ImageSettings {
            output_format: format,
            quality: Quality::Balanced,
            resize_mode: ResizeMode::Original,
            width: None,
            height: None,
            percentage: None,
            background: "#F5F1E8".to_owned(),
            metadata: MetadataPolicy::Remove,
            animation: crate::domain::AnimationPolicy::Ask,
        }
    }

    fn write_source(directory: &Path, name: &str, image: &DynamicImage) -> PathBuf {
        let path = directory.join(name);
        image.save(&path).expect("write source fixture");
        path
    }

    fn task(source: &Path, target: &Path, format: OutputFormat) -> NativeImageTask {
        NativeImageTask {
            source: source.to_path_buf(),
            target: target.to_path_buf(),
            format,
            settings: settings(format),
        }
    }

    fn reserve(directory: &Path, name: &str) -> PathBuf {
        let path = directory.join(name);
        File::create(&path).expect("reserve partial");
        path
    }

    #[test]
    fn png_source_converts_to_jpeg_without_a_media_engine() {
        let directory = tempdir().expect("temporary test directory");
        let source = write_source(
            directory.path(),
            "source.png",
            &DynamicImage::ImageRgb8(RgbImage::from_pixel(64, 48, Rgb([12, 200, 90]))),
        );
        let target = reserve(directory.path(), "out.jpg");

        convert(&task(&source, &target, OutputFormat::Jpeg)).expect("native JPEG conversion");

        let decoded = image::open(&target).expect("decode published JPEG");
        assert_eq!((decoded.width(), decoded.height()), (64, 48));
    }

    #[test]
    fn transparent_source_is_flattened_onto_the_chosen_background() {
        let directory = tempdir().expect("temporary test directory");
        let mut rgba = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 0]));
        rgba.put_pixel(4, 4, Rgba([255, 0, 0, 255]));
        let source = write_source(
            directory.path(),
            "alpha.png",
            &DynamicImage::ImageRgba8(rgba),
        );
        let target = reserve(directory.path(), "flat.jpg");

        convert(&task(&source, &target, OutputFormat::Jpeg)).expect("native flattening conversion");

        let decoded = image::open(&target)
            .expect("decode flattened JPEG")
            .to_rgb8();
        let corner = decoded.get_pixel(0, 0).0;
        // #F5F1E8 survives JPEG's lossy round trip within a small tolerance.
        assert!(
            corner[0] > 235 && corner[1] > 230 && corner[2] > 215,
            "transparent corner should carry the background, found {corner:?}"
        );
        let centre = decoded.get_pixel(4, 4).0;
        assert!(
            centre[0] > centre[1] && centre[0] > centre[2],
            "opaque foreground should stay visibly red, found {centre:?}"
        );
    }

    #[test]
    fn ico_output_carries_every_declared_size() {
        let directory = tempdir().expect("temporary test directory");
        let source = write_source(
            directory.path(),
            "icon.png",
            &DynamicImage::ImageRgba8(RgbaImage::from_pixel(512, 512, Rgba([10, 40, 200, 255]))),
        );
        let target = reserve(directory.path(), "icon.ico");

        convert(&task(&source, &target, OutputFormat::Ico)).expect("native ICO conversion");

        let bytes = std::fs::read(&target).expect("read published ICO");
        assert_eq!(&bytes[0..4], &[0, 0, 1, 0], "ICO header");
        assert_eq!(
            u16::from_le_bytes([bytes[4], bytes[5]]),
            ICO_SIZES.len() as u16,
            "every declared icon size must be present"
        );
        for (index, size) in ICO_SIZES.iter().enumerate() {
            let entry = 6 + index * 16;
            let recorded = u32::from(bytes[entry]);
            let expected = if *size == 256 { 0 } else { *size };
            assert_eq!(recorded, expected, "entry {index} width byte");
        }
    }

    #[test]
    fn a_non_square_source_is_centered_inside_each_icon() {
        let directory = tempdir().expect("temporary test directory");
        let source = write_source(
            directory.path(),
            "wide.png",
            &DynamicImage::ImageRgba8(RgbaImage::from_pixel(400, 100, Rgba([200, 30, 30, 255]))),
        );
        let target = reserve(directory.path(), "wide.ico");

        convert(&task(&source, &target, OutputFormat::Ico)).expect("native ICO conversion");

        let icon = image::open(&target)
            .expect("decode published ICO")
            .to_rgba8();
        assert_eq!((icon.width(), icon.height()), (256, 256));
        assert_eq!(
            icon.get_pixel(128, 0).0[3],
            0,
            "padding above a wide source must stay transparent"
        );
        assert_eq!(
            icon.get_pixel(128, 128).0[3],
            255,
            "the centered source must remain opaque"
        );
    }

    #[test]
    fn resize_by_width_preserves_the_aspect_ratio() {
        let directory = tempdir().expect("temporary test directory");
        let source = write_source(
            directory.path(),
            "wide.png",
            &DynamicImage::ImageRgb8(RgbImage::from_pixel(800, 400, Rgb([30, 30, 30]))),
        );
        let target = reserve(directory.path(), "resized.png");
        let mut resize = task(&source, &target, OutputFormat::Png);
        resize.settings.resize_mode = ResizeMode::Width;
        resize.settings.width = Some(200);

        convert(&resize).expect("native resize conversion");

        let decoded = image::open(&target).expect("decode resized PNG");
        assert_eq!((decoded.width(), decoded.height()), (200, 100));
    }

    #[test]
    fn inspection_reports_dimensions_and_alpha_without_a_media_engine() {
        let directory = tempdir().expect("temporary test directory");
        let opaque = write_source(
            directory.path(),
            "opaque.jpg",
            &DynamicImage::ImageRgb8(RgbImage::from_pixel(20, 10, Rgb([1, 2, 3]))),
        );
        let transparent = write_source(
            directory.path(),
            "alpha.png",
            &DynamicImage::ImageRgba8(RgbaImage::from_pixel(5, 6, Rgba([0, 0, 0, 0]))),
        );

        let opaque = inspect(&opaque).expect("inspect JPEG");
        assert_eq!((opaque.width, opaque.height), (20, 10));
        assert!(!opaque.has_alpha);
        assert!(!opaque.animated);
        assert_eq!(opaque.format_name, "jpeg");

        let transparent = inspect(&transparent).expect("inspect PNG");
        assert_eq!((transparent.width, transparent.height), (5, 6));
        assert!(transparent.has_alpha);
    }

    #[test]
    fn inspection_declines_files_this_engine_does_not_own() {
        let directory = tempdir().expect("temporary test directory");
        let text = directory.path().join("notes.txt");
        std::fs::write(&text, b"not an image").expect("write text fixture");
        assert!(inspect(&text).is_none());
    }

    #[test]
    fn an_animated_gif_is_recognized_before_a_still_conversion() {
        let directory = tempdir().expect("temporary test directory");
        let still = directory.path().join("still.gif");
        let animated = directory.path().join("animated.gif");
        encode_gif(&still, 1);
        encode_gif(&animated, 3);

        assert!(
            !inspect(&still).expect("inspect still GIF").animated,
            "a single-frame GIF must not be reported as animated"
        );
        assert!(
            inspect(&animated).expect("inspect animated GIF").animated,
            "a multi-frame GIF must be reported as animated"
        );
    }

    fn encode_gif(path: &Path, frames: usize) {
        use image::{AnimationDecoder as _, Delay, Frame};
        let file = File::create(path).expect("create GIF fixture");
        let mut encoder = image::codecs::gif::GifEncoder::new(file);
        let built = (0..frames)
            .map(|index| {
                let shade = 40_u8.saturating_add((index * 60) as u8);
                Frame::from_parts(
                    RgbaImage::from_pixel(8, 8, Rgba([shade, shade, shade, 255])),
                    0,
                    0,
                    Delay::from_numer_denom_ms(100, 1),
                )
            })
            .collect::<Vec<_>>();
        encoder.encode_frames(built).expect("encode GIF fixture");
        drop(encoder);
        // Confirm the fixture really holds the requested frame count.
        let decoded = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(
            File::open(path).expect("reopen GIF fixture"),
        ))
        .expect("decode GIF fixture");
        assert_eq!(decoded.into_frames().count(), frames);
    }

    /// Build a PNG whose header declares an enormous surface.
    ///
    /// This is the classic decompression bomb: a hundred bytes on disk that ask
    /// a decoder for gigabytes of memory. Morflo opens files other people
    /// produced, so refusing one must be proven, not assumed.
    fn declared_dimension_bomb(width: u32, height: u32) -> Vec<u8> {
        fn crc32(bytes: &[u8]) -> u32 {
            let mut crc = 0xFFFF_FFFF_u32;
            for byte in bytes {
                crc ^= u32::from(*byte);
                for _ in 0..8 {
                    let mask = (crc & 1).wrapping_neg();
                    crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
                }
            }
            !crc
        }
        fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
            let mut out = Vec::new();
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            let mut body = kind.to_vec();
            body.extend_from_slice(data);
            out.extend_from_slice(&body);
            out.extend_from_slice(&crc32(&body).to_be_bytes());
            out
        }

        let mut header = Vec::new();
        header.extend_from_slice(&width.to_be_bytes());
        header.extend_from_slice(&height.to_be_bytes());
        // 8-bit RGBA, deflate, adaptive filtering, no interlace.
        header.extend_from_slice(&[8, 6, 0, 0, 0]);

        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend_from_slice(&chunk(b"IHDR", &header));
        // A well-formed but deliberately short zlib stream. The decoder must
        // refuse on the declared size before it ever needs these bytes.
        png.extend_from_slice(&chunk(b"IDAT", &[0x78, 0x01, 0x01, 0x00, 0x00, 0xFF, 0xFF]));
        png.extend_from_slice(&chunk(b"IEND", &[]));
        png
    }

    #[test]
    fn a_declared_dimension_bomb_is_refused_before_any_pixel_is_allocated() {
        let directory = tempdir().expect("temporary test directory");
        // The dimension guard alone would admit this: both sides are under
        // MAX_SOURCE_DIMENSION, yet the surface is roughly 10 GB as RGBA.
        let source = directory.path().join("bomb.png");
        std::fs::write(&source, declared_dimension_bomb(50_000, 50_000))
            .expect("write decompression bomb fixture");
        let target = reserve(directory.path(), "bomb.jpg");

        let error = convert(&task(&source, &target, OutputFormat::Jpeg))
            .expect_err("an oversized declared surface must be refused");

        assert_eq!(error.code, ConversionErrorCode::UnsupportedFormat);
        let details = error.technical_details.unwrap_or_default();
        assert!(
            details.contains("50000x50000") && details.contains("allowance"),
            "the refusal must come from Morflo's own budget, found {details:?}"
        );
        assert_eq!(
            std::fs::metadata(&target)
                .expect("partial still exists")
                .len(),
            0,
            "nothing may be written for a refused source"
        );
    }

    #[test]
    fn the_decode_budget_admits_a_large_but_real_photograph() {
        // Roughly a 190-megapixel scan: large, plausible, and inside the budget.
        within_decode_budget(16_000, 12_000).expect("a large real image must still convert");
        within_decode_budget(100_000, 100_000)
            .expect_err("the maximum declarable surface must not be admitted");
    }

    #[test]
    fn header_inspection_stays_cheap_for_an_oversized_declaration() {
        let directory = tempdir().expect("temporary test directory");
        let source = directory.path().join("wide.png");
        // Beyond the dimension guard, so inspection declines it outright.
        std::fs::write(&source, declared_dimension_bomb(200_000, 4))
            .expect("write oversized declaration fixture");

        assert!(
            inspect(&source).is_none(),
            "a surface beyond the dimension guard must not be accepted"
        );
    }

    #[test]
    fn unsupported_native_outputs_are_refused_with_their_reason() {
        let directory = tempdir().expect("temporary test directory");
        let source = write_source(
            directory.path(),
            "source.png",
            &DynamicImage::ImageRgb8(RgbImage::from_pixel(4, 4, Rgb([9, 9, 9]))),
        );
        let target = reserve(directory.path(), "out.webp");

        let error = convert(&task(&source, &target, OutputFormat::Webp))
            .expect_err("WebP is not a built-in output");
        assert_eq!(error.code, ConversionErrorCode::UnsupportedCodec);
    }

    #[test]
    fn reported_capabilities_name_every_format_and_explain_the_gaps() {
        let outputs = outputs();
        assert_eq!(outputs.len(), OutputFormat::ALL.len());
        for capability in outputs {
            if NATIVE_OUTPUTS.contains(&capability.format) {
                assert!(
                    capability.available,
                    "{:?} must be offered",
                    capability.format
                );
            } else {
                assert!(
                    !capability.available,
                    "{:?} must not be offered",
                    capability.format
                );
                assert!(
                    capability.reason.is_some(),
                    "{:?} must explain why it is unavailable",
                    capability.format
                );
            }
        }
    }
}
