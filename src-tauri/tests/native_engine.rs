//! End-to-end evidence that Morflo works with no media engine installed.
//!
//! Every test here runs against `EngineRuntime::native_only`, which is the
//! exact runtime a computer with no FFmpeg receives. Nothing is mocked and no
//! external process is available to these tests, so a pass means the built-in
//! image engine really carried the journey: inspection, planning, encoding,
//! collision-safe naming, and verification of the published file.
//!
//! Unlike `real_engine.rs`, this file is not feature-gated. It must run
//! everywhere, because "works without an engine" is the property under test.

use std::path::{Path, PathBuf};

use image::{DynamicImage, Rgb, RgbImage, Rgba, RgbaImage};
use morflo_lib::{
    domain::{
        AnimationPolicy, CollisionPolicy, ConversionError, ConversionSettings, DestinationMode,
        EngineSource, GifPreset, GifSettings, ImageSettings, LoopBehavior, MediaKind,
        MetadataPolicy, OutputFormat, Quality, ResizeMode, Resolution, VideoSettings,
    },
    engine::EngineRuntime,
    image_engine,
    output::{OutputReservations, ReservedOutput},
    planner::{ConversionProgram, plan_conversion},
    probe::inspect_path,
};
use sha2::{Digest as _, Sha256};
use tempfile::tempdir;

/// The runtime a computer with no media engine actually gets.
fn engine_free_runtime() -> EngineRuntime {
    EngineRuntime::native_only("No development or system engine pair was found".to_owned())
}

fn settings(output_format: OutputFormat) -> ConversionSettings {
    ConversionSettings {
        image: ImageSettings {
            output_format,
            quality: Quality::Balanced,
            resize_mode: ResizeMode::Original,
            width: None,
            height: None,
            percentage: None,
            background: "#F5F1E8".to_owned(),
            metadata: MetadataPolicy::Remove,
            animation: AnimationPolicy::Ask,
        },
        video: VideoSettings {
            output_format: OutputFormat::Mp4,
            resolution: Resolution::Original,
            quality: Quality::Balanced,
            metadata: MetadataPolicy::Preserve,
        },
        gif: GifSettings {
            preset: GifPreset::Web,
            start_seconds: 0.0,
            end_seconds: 2.0,
            width: 480,
            fps: 12,
            quality: Quality::Balanced,
            r#loop: LoopBehavior::Forever,
        },
        destination: DestinationMode::Same,
        destination_path: None,
        collision_policy: CollisionPolicy::Suffix,
    }
}

fn write_png(directory: &Path, name: &str, image: DynamicImage) -> PathBuf {
    let path = directory.join(name);
    image.save(&path).expect("write source fixture");
    path
}

fn digest(path: &Path) -> String {
    let bytes = std::fs::read(path).expect("read file for hashing");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

/// Run the whole engine-free journey for one source and return the published
/// file, mirroring what the job manager does around the same calls.
async fn convert_without_a_media_engine(
    source: &Path,
    settings: &ConversionSettings,
) -> Result<PathBuf, ConversionError> {
    let engine = engine_free_runtime();
    let inspected = inspect_path(&engine, source).await?;
    let outputs = OutputReservations::default();
    let reservation = outputs.reserve(
        &inspected.path,
        settings.image.output_format,
        settings.destination,
        settings.destination_path.as_deref(),
        settings.collision_policy,
    )?;
    let plan = plan_conversion(&engine, &inspected, settings, &reservation)?;

    let ConversionProgram::Native(task) = &plan.program else {
        panic!("a machine without a media engine must plan a built-in conversion");
    };
    image_engine::convert(task)?;

    // The job manager verifies the published bytes before finalizing. Do the
    // same here so the test proves the file is really the requested format.
    let published = inspect_path(&engine, &reservation.partial_path).await?;
    assert_eq!(published.media.kind, MediaKind::Image);

    let final_path = reservation.final_path.clone();
    reservation.finalize()?;
    Ok(final_path)
}

#[tokio::test]
async fn a_png_becomes_a_jpeg_with_no_media_engine_installed() {
    let directory = tempdir().expect("temporary test directory");
    let source = write_png(
        directory.path(),
        "holiday.png",
        DynamicImage::ImageRgb8(RgbImage::from_pixel(320, 200, Rgb([18, 140, 210]))),
    );
    let source_digest = digest(&source);

    let published = convert_without_a_media_engine(&source, &settings(OutputFormat::Jpeg))
        .await
        .expect("engine-free JPEG conversion");

    assert_eq!(published.file_name().unwrap(), "holiday.jpg");
    let decoded = image::open(&published).expect("decode published JPEG");
    assert_eq!((decoded.width(), decoded.height()), (320, 200));
    assert_eq!(
        digest(&source),
        source_digest,
        "the source file must be byte-identical after conversion"
    );
}

#[tokio::test]
async fn a_favicon_is_created_with_no_media_engine_installed() {
    let directory = tempdir().expect("temporary test directory");
    let source = write_png(
        directory.path(),
        "mark.png",
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(512, 512, Rgba([240, 90, 40, 255]))),
    );

    let published = convert_without_a_media_engine(&source, &settings(OutputFormat::Ico))
        .await
        .expect("engine-free ICO conversion");

    let bytes = std::fs::read(&published).expect("read published ICO");
    assert_eq!(&bytes[0..4], &[0, 0, 1, 0], "ICO header");
    assert_eq!(
        u16::from_le_bytes([bytes[4], bytes[5]]),
        image_engine::ICO_SIZES.len() as u16,
        "the favicon must carry every declared size"
    );
}

#[tokio::test]
async fn an_existing_output_is_never_overwritten_without_a_media_engine() {
    let directory = tempdir().expect("temporary test directory");
    let source = write_png(
        directory.path(),
        "collision.png",
        DynamicImage::ImageRgb8(RgbImage::from_pixel(40, 40, Rgb([200, 30, 30]))),
    );
    let sentinel = directory.path().join("collision.jpg");
    std::fs::write(&sentinel, b"pre-existing file").expect("write collision sentinel");
    let sentinel_digest = digest(&sentinel);

    let published = convert_without_a_media_engine(&source, &settings(OutputFormat::Jpeg))
        .await
        .expect("engine-free conversion beside an existing output");

    assert_eq!(published.file_name().unwrap(), "collision (2).jpg");
    assert_eq!(
        digest(&sentinel),
        sentinel_digest,
        "the pre-existing output must stay byte-identical"
    );
}

#[tokio::test]
async fn a_transparent_source_is_flattened_onto_the_chosen_background() {
    let directory = tempdir().expect("temporary test directory");
    let mut rgba = RgbaImage::from_pixel(32, 32, Rgba([0, 0, 0, 0]));
    for x in 12..20 {
        for y in 12..20 {
            rgba.put_pixel(x, y, Rgba([20, 190, 60, 255]));
        }
    }
    let source = write_png(directory.path(), "logo.png", DynamicImage::ImageRgba8(rgba));

    let published = convert_without_a_media_engine(&source, &settings(OutputFormat::Jpeg))
        .await
        .expect("engine-free flattening conversion");

    let decoded = image::open(&published)
        .expect("decode flattened JPEG")
        .to_rgb8();
    let corner = decoded.get_pixel(1, 1).0;
    assert!(
        corner[0] > 235 && corner[1] > 228 && corner[2] > 210,
        "the transparent area must carry the chosen background, found {corner:?}"
    );
    let centre = decoded.get_pixel(16, 16).0;
    assert!(
        centre[1] > centre[0] && centre[1] > centre[2],
        "the opaque foreground must stay visibly green, found {centre:?}"
    );
}

#[tokio::test]
async fn a_video_is_declined_with_an_actionable_reason_not_a_crash() {
    let directory = tempdir().expect("temporary test directory");
    let video = directory.path().join("clip.mp4");
    std::fs::write(&video, b"\x00\x00\x00\x18ftypmp42 not a real sample").expect("write video");

    let engine = engine_free_runtime();
    let error = inspect_path(&engine, &video)
        .await
        .expect_err("a video cannot be inspected without a media engine");

    // A person reads the title and the message together, so the guidance is
    // asserted across both rather than against one field.
    let shown = format!("{} {}", error.title, error.message).to_lowercase();
    assert!(
        shown.contains("media engine"),
        "the failure must name what is missing, found {shown:?}"
    );
    assert!(
        !error.title.is_empty() && !error.message.is_empty(),
        "the failure must stay presentable to a person"
    );
}

#[tokio::test]
async fn keeping_metadata_is_refused_with_its_reason() {
    let directory = tempdir().expect("temporary test directory");
    let source = write_png(
        directory.path(),
        "shot.png",
        DynamicImage::ImageRgb8(RgbImage::from_pixel(24, 24, Rgb([7, 7, 7]))),
    );
    let mut requested = settings(OutputFormat::Jpeg);
    requested.image.metadata = MetadataPolicy::Preserve;

    let error = convert_without_a_media_engine(&source, &requested)
        .await
        .expect_err("the built-in engine cannot preserve metadata");

    let shown = format!("{} {}", error.title, error.message).to_lowercase();
    assert!(
        shown.contains("metadata"),
        "the refusal must name metadata, found {shown:?}"
    );
}

#[tokio::test]
async fn an_animated_source_still_requires_an_explicit_choice() {
    let directory = tempdir().expect("temporary test directory");
    let animated = directory.path().join("loop.gif");
    encode_animated_gif(&animated);

    let engine = engine_free_runtime();
    let inspected = inspect_path(&engine, &animated)
        .await
        .expect("inspect animated GIF without a media engine");
    assert_eq!(inspected.media.animated, Some(true));
    assert!(
        inspected
            .media
            .warnings
            .iter()
            .any(|warning| warning.code == "animated-input"),
        "an animated source must warn before a still conversion"
    );

    let outputs = OutputReservations::default();
    let requested = settings(OutputFormat::Png);
    let reservation = outputs
        .reserve(
            &inspected.path,
            OutputFormat::Png,
            requested.destination,
            None,
            requested.collision_policy,
        )
        .expect("reserve output");
    let error = plan_conversion(&engine, &inspected, &requested, &reservation)
        .expect_err("Morflo must not silently pick one frame");
    let shown = format!("{} {}", error.title, error.message).to_lowercase();
    assert!(
        shown.contains("animat"),
        "the refusal must name animation, found {shown:?}"
    );
}

#[test]
fn engine_free_capabilities_offer_images_and_explain_every_gap() {
    let runtime = engine_free_runtime();
    let registry = &runtime.registry;

    assert!(
        registry.engine.available,
        "a machine without FFmpeg is a working state, not an unavailable one"
    );
    assert_eq!(registry.engine.source, EngineSource::Native);
    assert!(!runtime.has_media_engine());

    for capability in &registry.outputs {
        match capability.format {
            OutputFormat::Png | OutputFormat::Jpeg | OutputFormat::Ico => {
                assert!(
                    capability.available,
                    "{:?} must be offered without a media engine",
                    capability.format
                );
            }
            other => {
                assert!(
                    !capability.available,
                    "{other:?} must not be offered without a media engine"
                );
                let reason = capability
                    .reason
                    .as_deref()
                    .unwrap_or_else(|| panic!("{other:?} must explain why it is unavailable"));
                assert!(
                    reason.to_lowercase().contains("media engine"),
                    "{other:?} must name what it needs, found {reason:?}"
                );
            }
        }
    }
}

#[test]
fn asking_for_media_tools_without_them_names_what_is_missing() {
    let runtime = engine_free_runtime();
    let error = runtime
        .media()
        .expect_err("there are no media tools on this runtime");
    let shown = format!("{} {}", error.title, error.message).to_lowercase();
    assert!(shown.contains("media engine"), "found {shown:?}");
}

fn encode_animated_gif(path: &Path) {
    use image::{Delay, Frame};
    let file = std::fs::File::create(path).expect("create GIF fixture");
    let mut encoder = image::codecs::gif::GifEncoder::new(file);
    let frames = (0..3)
        .map(|index| {
            let shade = 40_u8.saturating_add((index * 60) as u8);
            Frame::from_parts(
                RgbaImage::from_pixel(16, 16, Rgba([shade, shade, shade, 255])),
                0,
                0,
                Delay::from_numer_denom_ms(100, 1),
            )
        })
        .collect::<Vec<_>>();
    encoder.encode_frames(frames).expect("encode GIF fixture");
}

/// Keep the unused-import surface honest: `ReservedOutput` is part of the
/// journey these tests exercise through `OutputReservations`.
const _: fn(&ReservedOutput) -> &Path = |output| output.partial_path.as_path();
