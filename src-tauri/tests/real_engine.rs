#![cfg(feature = "real-engine")]

use std::{path::Path, process::Stdio, time::Instant};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use morflo_lib::{
    domain::{
        AnimationPolicy, CollisionPolicy, ConversionSettings, DestinationMode, EngineSource,
        GifPreset, GifSettings, ImageSettings, LoopBehavior, MediaKind, MetadataPolicy,
        OutputFormat, Quality, ResizeMode, Resolution, VideoSettings,
    },
    engine::EngineService,
    output::OutputReservations,
    planner::{ConversionPlan, plan_conversion},
    preview::{
        render_clip, render_output_poster, render_output_thumbnail, render_poster,
        render_storyboard, render_thumbnail,
    },
    probe::inspect_path,
    progress::ProgressParser,
};
use tokio::process::Command;

fn balanced_settings(output_format: OutputFormat) -> ConversionSettings {
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
            end_seconds: 1.0,
            width: 540,
            fps: 12,
            quality: Quality::Balanced,
            r#loop: LoopBehavior::Forever,
        },
        destination: DestinationMode::Same,
        destination_path: None,
        collision_policy: CollisionPolicy::Suffix,
    }
}

#[tokio::test]
#[ignore = "requires a compatible real ffmpeg/ffprobe pair"]
async fn automatic_and_session_engine_discovery_remain_distinct() {
    let automatic = EngineService::new()
        .runtime()
        .await
        .expect("discover a real development or system engine");
    assert!(matches!(
        automatic.registry.engine.source,
        EngineSource::Project | EngineSource::System
    ));
    let directory = automatic
        .media()
        .expect("a real engine run requires the media tools")
        .ffmpeg
        .parent()
        .expect("real ffmpeg has a parent directory")
        .to_path_buf();

    let selected = EngineService::new()
        .select_directory(directory)
        .await
        .expect("select the same real pair for this session");
    assert_eq!(selected.engine.source, EngineSource::Selected);
    assert!(selected.engine.available);
}

async fn execute_real_plan(
    engine: &morflo_lib::engine::EngineRuntime,
    plan: &ConversionPlan,
) -> Vec<f64> {
    let mut all_progress = Vec::new();
    for stage in plan.program.stages() {
        let tools = engine.media().expect("real engine media tools");
        let output = Command::new(&tools.ffmpeg)
            .args(&stage.args)
            .stdin(Stdio::null())
            .output()
            .await
            .expect("start real FFmpeg conversion stage");
        assert!(
            output.status.success(),
            "real FFmpeg conversion stage failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let mut parser = ProgressParser::new(stage.duration_seconds);
        all_progress.extend(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| parser.push_line(line))
                .map(|progress| stage.progress_start + progress * stage.progress_span),
        );
    }
    all_progress
}

fn video_settings(output_format: OutputFormat, resolution: Resolution) -> ConversionSettings {
    let mut settings = balanced_settings(OutputFormat::Png);
    settings.video.output_format = output_format;
    settings.video.resolution = resolution;
    settings
}

async fn run_image_conversion(
    engine: &morflo_lib::engine::EngineRuntime,
    source_path: &Path,
    output_format: OutputFormat,
) -> std::path::PathBuf {
    run_image_conversion_with_settings(engine, source_path, balanced_settings(output_format)).await
}

async fn run_image_conversion_with_settings(
    engine: &morflo_lib::engine::EngineRuntime,
    source_path: &Path,
    settings: ConversionSettings,
) -> std::path::PathBuf {
    let source = inspect_path(engine, source_path)
        .await
        .expect("inspect generated source fixture");
    let output_format = settings.image.output_format;
    let outputs = OutputReservations::default();
    let reservation = outputs
        .reserve(
            &source.path,
            output_format,
            settings.destination,
            settings.destination_path.as_deref(),
            settings.collision_policy,
        )
        .expect("reserve collision-safe output");
    let plan = plan_conversion(engine, &source, &settings, &reservation)
        .expect("build validated conversion plan");
    let _ = execute_real_plan(engine, &plan).await;

    let converted = inspect_path(engine, &reservation.partial_path)
        .await
        .expect("probe converted partial output");
    if output_format != OutputFormat::Ico {
        assert_eq!(converted.media.width, source.media.width);
        assert_eq!(converted.media.height, source.media.height);
    }
    reservation.finalize().expect("publish verified output");
    reservation.final_path.clone()
}

async fn stream_dimensions(
    engine: &morflo_lib::engine::EngineRuntime,
    path: &Path,
) -> Vec<(u32, u32)> {
    let tools = engine.media().expect("real engine media tools");
    let output = Command::new(&tools.ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .await
        .expect("probe image stream dimensions");
    assert!(
        output.status.success(),
        "dimension probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut values = line.trim().split(',');
            Some((values.next()?.parse().ok()?, values.next()?.parse().ok()?))
        })
        .collect()
}

struct VideoRun {
    path: std::path::PathBuf,
    progress: Vec<f64>,
}

async fn run_video_conversion(
    engine: &morflo_lib::engine::EngineRuntime,
    source_path: &Path,
    output_format: OutputFormat,
    resolution: Resolution,
) -> VideoRun {
    let source = inspect_path(engine, source_path)
        .await
        .expect("inspect generated video fixture");
    assert_eq!(source.media.kind, MediaKind::Video);
    let settings = video_settings(output_format, resolution);
    let outputs = OutputReservations::default();
    let reservation = outputs
        .reserve(
            &source.path,
            output_format,
            settings.destination,
            settings.destination_path.as_deref(),
            settings.collision_policy,
        )
        .expect("reserve collision-safe video output");
    let plan = plan_conversion(engine, &source, &settings, &reservation)
        .expect("build validated video plan");
    let progress = execute_real_plan(engine, &plan).await;
    assert_eq!(progress.last().copied(), Some(1.0));
    let converted = inspect_path(engine, &reservation.partial_path)
        .await
        .expect("probe converted video partial output");
    assert_eq!(converted.media.kind, MediaKind::Video);
    let source_duration = source.media.duration_seconds.expect("source duration");
    let output_duration = converted.media.duration_seconds.expect("output duration");
    assert!(
        (source_duration - output_duration).abs() <= 0.35,
        "video duration drifted from {source_duration:.3}s to {output_duration:.3}s"
    );
    reservation
        .finalize()
        .expect("publish verified video output");
    VideoRun {
        path: reservation.final_path.clone(),
        progress,
    }
}

async fn rgb_pixel_at(
    engine: &morflo_lib::engine::EngineRuntime,
    path: &Path,
    x: u32,
    y: u32,
) -> [u8; 3] {
    let filter = format!("crop=2:2:{x}:{y},scale=1:1:flags=area,format=rgb24");
    let tools = engine.media().expect("real engine media tools");
    let output = Command::new(&tools.ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(path)
        .args(["-vf", &filter, "-frames:v", "1", "-f", "rawvideo", "pipe:1"])
        .stdin(Stdio::null())
        .output()
        .await
        .expect("decode first output pixel");
    assert!(
        output.status.success(),
        "pixel probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.len() >= 3,
        "pixel probe returned no RGB sample"
    );
    [output.stdout[0], output.stdout[1], output.stdout[2]]
}

#[tokio::test]
#[ignore = "requires a compatible real FFmpeg/ffprobe pair"]
async fn png_to_jpeg_and_jpeg_to_png_use_real_engine() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root");
    let fixture_root = root.join("fixtures").join("generated");
    let run = tempfile::tempdir_in(&fixture_root).expect("create fixture-scoped run directory");
    let transparent_source = run.path().join("轉換 🧳 O'Reilly.png");
    let jpeg_source = run.path().join("warm landscape.jpg");
    std::fs::copy(
        fixture_root.join("transparent-grid.png"),
        &transparent_source,
    )
    .expect("copy transparent fixture");
    std::fs::copy(fixture_root.join("warm-landscape.jpg"), &jpeg_source)
        .expect("copy JPEG fixture");
    let original_png = std::fs::read(&transparent_source).expect("read original fixture");

    let engine = EngineService::new()
        .runtime()
        .await
        .expect("discover compatible real engine");
    let jpeg_output = run_image_conversion(&engine, &transparent_source, OutputFormat::Jpeg).await;
    let png_output = run_image_conversion(&engine, &jpeg_source, OutputFormat::Png).await;

    assert!(jpeg_output.exists());
    assert!(png_output.exists());
    let flattened = rgb_pixel_at(&engine, &jpeg_output, 0, 0).await;
    assert!(
        flattened.iter().all(|channel| *channel > 200),
        "transparent corner was not flattened onto the selected warm background: {flattened:?}"
    );
    let foreground = rgb_pixel_at(&engine, &jpeg_output, 320, 210).await;
    let color_distance = foreground
        .iter()
        .zip(flattened)
        .map(|(channel, background)| u8::abs_diff(*channel, background) as u16)
        .sum::<u16>();
    assert!(
        color_distance > 80,
        "visible foreground content was lost while flattening alpha: background={flattened:?}, foreground={foreground:?}"
    );
    assert_eq!(
        std::fs::read(&transparent_source).expect("read source after conversion"),
        original_png,
        "source bytes changed during conversion"
    );
}

#[tokio::test]
#[ignore = "requires a compatible real FFmpeg/ffprobe pair"]
async fn existing_output_gets_a_suffix_without_overwrite() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root");
    let fixture_root = root.join("fixtures").join("generated");
    let run = tempfile::tempdir_in(&fixture_root).expect("create fixture-scoped run directory");
    let source = run.path().join("collision.png");
    let existing = run.path().join("collision.jpg");
    std::fs::copy(fixture_root.join("transparent-grid.png"), &source).expect("copy source fixture");
    std::fs::write(&existing, b"existing-output-must-survive").expect("write collision sentinel");

    let engine = EngineService::new()
        .runtime()
        .await
        .expect("discover compatible real engine");
    let output = run_image_conversion(&engine, &source, OutputFormat::Jpeg).await;

    assert_eq!(
        output.file_name().and_then(|name| name.to_str()),
        Some("collision (2).jpg")
    );
    assert_eq!(
        std::fs::read(&existing).expect("read original collision file"),
        b"existing-output-must-survive"
    );
}

#[tokio::test]
#[ignore = "requires a compatible real FFmpeg/ffprobe pair"]
async fn webp_avif_and_multiresolution_ico_use_real_engine() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root");
    let fixture_root = root.join("fixtures").join("generated");
    let run = tempfile::tempdir_in(&fixture_root).expect("create fixture-scoped run directory");
    let transparent_source = run.path().join("transparent source.png");
    let jpeg_source = run.path().join("opaque source.jpg");
    std::fs::copy(
        fixture_root.join("transparent-grid.png"),
        &transparent_source,
    )
    .expect("copy transparent fixture");
    std::fs::copy(fixture_root.join("warm-landscape.jpg"), &jpeg_source)
        .expect("copy opaque fixture");

    let engine = EngineService::new()
        .runtime()
        .await
        .expect("discover compatible real engine");
    let webp = run_image_conversion(&engine, &transparent_source, OutputFormat::Webp).await;
    let avif = run_image_conversion(&engine, &transparent_source, OutputFormat::Avif).await;
    let ico_from_png = run_image_conversion(&engine, &transparent_source, OutputFormat::Ico).await;
    let ico_from_jpeg = run_image_conversion(&engine, &jpeg_source, OutputFormat::Ico).await;
    let ico_from_webp = run_image_conversion(&engine, &webp, OutputFormat::Ico).await;
    let avif_roundtrip = run_image_conversion(&engine, &avif, OutputFormat::Png).await;

    let webp_probe = inspect_path(&engine, &webp)
        .await
        .expect("probe WebP output");
    let avif_probe = inspect_path(&engine, &avif)
        .await
        .expect("probe AVIF output");
    let roundtrip_probe = inspect_path(&engine, &avif_roundtrip)
        .await
        .expect("probe AVIF alpha roundtrip");
    assert_eq!(webp_probe.media.has_alpha, Some(true));
    assert_eq!(avif_probe.media.has_alpha, Some(true));
    assert!(avif_probe.alpha_stream_index.is_some());
    assert_eq!(roundtrip_probe.media.has_alpha, Some(true));
    for ico in [&ico_from_png, &ico_from_jpeg, &ico_from_webp] {
        assert_eq!(
            stream_dimensions(&engine, ico).await,
            vec![(16, 16), (32, 32), (48, 48), (256, 256)]
        );
    }
}

#[tokio::test]
#[ignore = "requires a compatible real FFmpeg/ffprobe pair"]
async fn twenty_image_batch_produces_probeable_webp_outputs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root");
    let fixture_root = root.join("fixtures").join("generated");
    let run = tempfile::tempdir_in(&fixture_root).expect("create fixture-scoped run directory");
    let engine = EngineService::new()
        .runtime()
        .await
        .expect("discover compatible real engine");

    for index in 1..=20 {
        let source = run.path().join(format!("batch 日本語 {index:02}.jpg"));
        std::fs::copy(fixture_root.join("warm-landscape.jpg"), &source)
            .expect("copy batch fixture");
        let output = run_image_conversion(&engine, &source, OutputFormat::Webp).await;
        let inspected = inspect_path(&engine, &output)
            .await
            .expect("probe batch WebP output");
        assert_eq!(inspected.media.width, Some(720));
        assert_eq!(inspected.media.height, Some(480));
    }
}

#[tokio::test]
#[ignore = "requires a compatible real FFmpeg/ffprobe pair"]
async fn orientation_and_animation_safety_use_real_probe_evidence() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root");
    let fixture_root = root.join("fixtures").join("generated");
    let run = tempfile::tempdir_in(&fixture_root).expect("create fixture-scoped run directory");
    let oriented = run.path().join("oriented.jpg");
    let animated = run.path().join("animated.png");
    std::fs::copy(fixture_root.join("exif-orientation-6.jpg"), &oriented)
        .expect("copy oriented fixture");
    std::fs::copy(fixture_root.join("animated-input.png"), &animated)
        .expect("copy animated fixture");

    let engine = EngineService::new()
        .runtime()
        .await
        .expect("discover compatible real engine");
    let oriented_probe = inspect_path(&engine, &oriented)
        .await
        .expect("inspect EXIF-oriented JPEG");
    assert_eq!(oriented_probe.media.width, Some(240));
    assert_eq!(oriented_probe.media.height, Some(360));
    let oriented_output = run_image_conversion(&engine, &oriented, OutputFormat::Png).await;
    assert_eq!(
        stream_dimensions(&engine, &oriented_output).await,
        vec![(240, 360)]
    );

    let animated_probe = inspect_path(&engine, &animated)
        .await
        .expect("inspect animated PNG");
    assert_eq!(
        animated_probe.media.animated,
        Some(true),
        "expected animated probe evidence; ffprobe={} format={} duration={:?}",
        engine
            .media()
            .expect("real engine media tools")
            .ffprobe
            .display(),
        animated_probe.format_name,
        animated_probe.media.duration_seconds
    );
    let mut settings = balanced_settings(OutputFormat::Png);
    let outputs = OutputReservations::default();
    let reservation = outputs
        .reserve(
            &animated_probe.path,
            OutputFormat::Png,
            settings.destination,
            settings.destination_path.as_deref(),
            settings.collision_policy,
        )
        .expect("reserve rejected-animation output");
    let error = plan_conversion(&engine, &animated_probe, &settings, &reservation)
        .expect_err("animation must require an explicit first-frame decision");
    reservation.cleanup_partial();
    assert_eq!(error.title, "Choose how to handle animation");

    settings.image.animation = AnimationPolicy::First;
    let still_output = run_image_conversion_with_settings(&engine, &animated, settings).await;
    let still_probe = inspect_path(&engine, &still_output)
        .await
        .expect("probe explicit first-frame output");
    assert_eq!(still_probe.media.animated, Some(false));
}

#[tokio::test]
#[ignore = "requires a compatible real FFmpeg/ffprobe pair"]
async fn curated_image_inputs_and_large_dimensions_convert() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root");
    let fixture_root = root.join("fixtures").join("generated");
    let run = tempfile::tempdir_in(&fixture_root).expect("create fixture-scoped run directory");
    let engine = EngineService::new()
        .runtime()
        .await
        .expect("discover compatible real engine");
    let cases = [
        ("sample.bmp", OutputFormat::Png),
        ("sample.tiff", OutputFormat::Webp),
        ("multi-resolution.ico", OutputFormat::Png),
        ("unusual-17x2049.png", OutputFormat::Webp),
        ("large-image.png", OutputFormat::Jpeg),
    ];

    for (name, output_format) in cases {
        let source = run.path().join(name);
        std::fs::copy(fixture_root.join(name), &source).expect("copy curated image fixture");
        let before = std::fs::metadata(&source)
            .expect("read source metadata")
            .len();
        let output = run_image_conversion(&engine, &source, output_format).await;
        assert!(output.exists());
        assert_eq!(
            std::fs::metadata(&source)
                .expect("read source metadata after conversion")
                .len(),
            before
        );
    }
}

#[tokio::test]
#[ignore = "requires a compatible real FFmpeg/ffprobe pair"]
async fn common_video_routes_preserve_compatible_streams_and_duration() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root");
    let fixture_root = root.join("fixtures").join("generated");
    let run = tempfile::tempdir_in(&fixture_root).expect("create fixture-scoped run directory");
    for name in [
        "portrait-phone.mov",
        "multi-stream.mkv",
        "short-1080p.mp4",
        "sample.webm",
        "silent-video.mp4",
        "variable-frame-rate.mkv",
    ] {
        std::fs::copy(fixture_root.join(name), run.path().join(name)).expect("copy video fixture");
    }

    let engine = EngineService::new()
        .runtime()
        .await
        .expect("discover compatible real engine");
    let mov_source = run.path().join("portrait-phone.mov");
    let mov_before = std::fs::read(&mov_source).expect("read MOV source");
    let mov = run_video_conversion(
        &engine,
        &mov_source,
        OutputFormat::Mp4,
        Resolution::Original,
    )
    .await;
    let mov_probe = inspect_path(&engine, &mov.path)
        .await
        .expect("probe MOV to MP4");
    assert_eq!(
        (mov_probe.media.width, mov_probe.media.height),
        (Some(720), Some(1280))
    );
    assert_eq!(mov_probe.media.audio_tracks, Some(1));
    assert_eq!(
        std::fs::read(&mov_source).expect("re-read MOV source"),
        mov_before
    );

    let mkv = run_video_conversion(
        &engine,
        &run.path().join("multi-stream.mkv"),
        OutputFormat::Mp4,
        Resolution::Original,
    )
    .await;
    let mkv_probe = inspect_path(&engine, &mkv.path)
        .await
        .expect("probe MKV to MP4");
    assert_eq!(mkv_probe.media.audio_tracks, Some(2));
    assert_eq!(mkv_probe.media.subtitle_tracks, Some(1));
    assert_eq!(mkv_probe.media.chapter_count, Some(2));

    let webm = run_video_conversion(
        &engine,
        &run.path().join("short-1080p.mp4"),
        OutputFormat::Webm,
        Resolution::Hd720,
    )
    .await;
    let webm_probe = inspect_path(&engine, &webm.path)
        .await
        .expect("probe MP4 to WebM");
    assert_eq!(
        (webm_probe.media.width, webm_probe.media.height),
        (Some(1280), Some(720))
    );
    assert_eq!(webm_probe.media.video_codec.as_deref(), Some("vp9"));
    assert!(
        webm.progress
            .iter()
            .any(|value| *value > 0.0 && *value < 1.0)
    );

    let webm_to_mp4 = run_video_conversion(
        &engine,
        &run.path().join("sample.webm"),
        OutputFormat::Mp4,
        Resolution::Original,
    )
    .await;
    let webm_to_mp4_probe = inspect_path(&engine, &webm_to_mp4.path)
        .await
        .expect("probe WebM to MP4");
    assert_eq!(webm_to_mp4_probe.media.video_codec.as_deref(), Some("h264"));

    let silent = run_video_conversion(
        &engine,
        &run.path().join("silent-video.mp4"),
        OutputFormat::Webm,
        Resolution::Original,
    )
    .await;
    let silent_probe = inspect_path(&engine, &silent.path)
        .await
        .expect("probe silent WebM");
    assert_eq!(silent_probe.media.audio_tracks, Some(0));

    let vfr = run_video_conversion(
        &engine,
        &run.path().join("variable-frame-rate.mkv"),
        OutputFormat::Mp4,
        Resolution::Original,
    )
    .await;
    assert!(vfr.path.exists());
}

#[tokio::test]
#[ignore = "requires a compatible real FFmpeg/ffprobe pair"]
async fn damaged_video_maps_to_a_human_error() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root");
    let fixture = root
        .join("fixtures")
        .join("generated")
        .join("damaged-video.mp4");
    let engine = EngineService::new()
        .runtime()
        .await
        .expect("discover compatible real engine");
    let error = inspect_path(&engine, &fixture)
        .await
        .expect_err("truncated video must not inspect successfully");
    assert_eq!(
        error.code,
        morflo_lib::domain::ConversionErrorCode::DamagedInput
    );
    assert!(!error.message.to_ascii_lowercase().contains("ffprobe"));
}

#[tokio::test]
#[ignore = "requires a compatible real FFmpeg/ffprobe pair"]
async fn bounded_media_previews_are_decodable_and_resized() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root");
    let fixture_root = root.join("fixtures").join("generated");
    let run = tempfile::tempdir_in(&fixture_root).expect("create fixture-scoped run directory");
    let source = fixture_root.join("short-1080p.mp4");
    let engine = EngineService::new()
        .runtime()
        .await
        .expect("discover compatible real engine");

    let thumbnail = render_thumbnail(&engine, &fixture_root.join("transparent-grid.png"))
        .await
        .expect("render a bounded image thumbnail");
    assert_eq!(thumbnail.mime_type, "image/png");
    let thumbnail_path = run.path().join("thumbnail.png");
    std::fs::write(
        &thumbnail_path,
        BASE64
            .decode(thumbnail.data_base64)
            .expect("decode thumbnail"),
    )
    .expect("write thumbnail fixture");
    let thumbnail_probe = inspect_path(&engine, &thumbnail_path)
        .await
        .expect("probe image thumbnail");
    assert_eq!(thumbnail_probe.media.kind, MediaKind::Image);
    assert_eq!(thumbnail_probe.media.width, Some(512));
    assert_eq!(thumbnail_probe.media.height, Some(336));
    assert_eq!(thumbnail_probe.media.has_alpha, Some(true));

    let poster = render_poster(&engine, &source, 1.25)
        .await
        .expect("render a bounded poster frame");
    assert_eq!(poster.mime_type, "image/jpeg");
    let poster_path = run.path().join("poster.jpg");
    std::fs::write(
        &poster_path,
        BASE64.decode(poster.data_base64).expect("decode poster"),
    )
    .expect("write poster fixture");
    let poster_probe = inspect_path(&engine, &poster_path)
        .await
        .expect("probe poster frame");
    assert_eq!(poster_probe.media.kind, MediaKind::Image);
    assert_eq!(poster_probe.media.width, Some(720));
    assert_eq!(poster_probe.media.height, Some(406));

    let storyboard_started = Instant::now();
    let storyboard = render_storyboard(&engine, &source, 3.0)
        .await
        .expect("render a bounded seven-frame moment strip");
    eprintln!(
        "seven-frame moment strip: {} ms",
        storyboard_started.elapsed().as_millis()
    );
    assert_eq!(storyboard.frames.len(), 7);
    assert_ne!(
        storyboard.frames.first().map(|frame| &frame.data_base64),
        storyboard.frames.last().map(|frame| &frame.data_base64),
        "the first and last sampled moments should be visually distinct"
    );
    for (index, frame) in storyboard
        .frames
        .iter()
        .enumerate()
        .filter(|(index, _)| *index == 0 || *index == 6)
    {
        assert_eq!(frame.mime_type, "image/jpeg");
        let frame_path = run.path().join(format!("moment-{index}.jpg"));
        std::fs::write(
            &frame_path,
            BASE64
                .decode(&frame.data_base64)
                .expect("decode moment frame"),
        )
        .expect("write moment frame");
        let frame_probe = inspect_path(&engine, &frame_path)
            .await
            .expect("probe moment frame");
        assert_eq!(frame_probe.media.kind, MediaKind::Image);
        assert_eq!(frame_probe.media.width, Some(160));
        assert_eq!(frame_probe.media.height, Some(90));
    }

    let output_thumbnail = render_output_thumbnail(&engine, &fixture_root.join("sample.avif"))
        .await
        .expect("render a bounded completed-image preview");
    assert_eq!(output_thumbnail.mime_type, "image/png");
    assert!(!output_thumbnail.data_base64.is_empty());

    let output_poster = render_output_poster(&engine, &source, 0.5)
        .await
        .expect("render a bounded completed-video poster");
    assert_eq!(output_poster.mime_type, "image/jpeg");
    assert!(!output_poster.data_base64.is_empty());

    let clip = render_clip(&engine, &source, 0.0, 3.0)
        .await
        .expect("render a bounded three-second clip");
    assert_eq!(clip.mime_type, "video/mp4");
    let clip_path = run.path().join("preview.mp4");
    std::fs::write(
        &clip_path,
        BASE64
            .decode(clip.data_base64)
            .expect("decode preview clip"),
    )
    .expect("write preview clip");
    let clip_probe = inspect_path(&engine, &clip_path)
        .await
        .expect("probe preview clip");
    assert_eq!(clip_probe.media.kind, MediaKind::Video);
    assert_eq!(clip_probe.media.width, Some(640));
    assert_eq!(clip_probe.media.height, Some(360));
    let preview_duration = clip_probe.media.duration_seconds.expect("preview duration");
    assert!(
        (preview_duration - 3.0).abs() <= 0.15,
        "preview duration was {preview_duration:.3}s"
    );
}

#[tokio::test]
#[ignore = "requires a compatible real FFmpeg/ffprobe pair"]
async fn trimmed_two_stage_gif_is_animated_looped_and_probeable() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root");
    let fixture_root = root.join("fixtures").join("generated");
    let run = tempfile::tempdir_in(&fixture_root).expect("create fixture-scoped run directory");
    let source_path = run.path().join("high motion source.mp4");
    std::fs::copy(
        fixture_root.join("high-motion-gif-source.mp4"),
        &source_path,
    )
    .expect("copy GIF source fixture");
    let engine = EngineService::new()
        .runtime()
        .await
        .expect("discover compatible real engine");
    let source = inspect_path(&engine, &source_path)
        .await
        .expect("inspect GIF source");
    let mut settings = video_settings(OutputFormat::Gif, Resolution::Original);
    settings.gif = GifSettings {
        preset: GifPreset::Web,
        start_seconds: 0.5,
        end_seconds: 2.7,
        width: 540,
        fps: 12,
        quality: Quality::Balanced,
        r#loop: LoopBehavior::Forever,
    };
    let outputs = OutputReservations::default();
    let reservation = outputs
        .reserve(
            &source.path,
            OutputFormat::Gif,
            settings.destination,
            settings.destination_path.as_deref(),
            settings.collision_policy,
        )
        .expect("reserve GIF output");
    let plan = plan_conversion(&engine, &source, &settings, &reservation)
        .expect("build validated two-stage GIF plan");
    assert_eq!(plan.program.stages().len(), 2);
    assert_eq!(
        plan.program.stages()[0].phase,
        morflo_lib::domain::JobPhase::PreparingPalette
    );
    assert_eq!(
        plan.program.stages()[1].phase,
        morflo_lib::domain::JobPhase::Encoding
    );
    assert_eq!(plan.temporary_paths.len(), 1);

    let progress = execute_real_plan(&engine, &plan).await;
    assert!(progress.windows(2).all(|pair| pair[0] <= pair[1]));
    assert_eq!(progress.last().copied(), Some(1.0));
    plan.cleanup_temporary_paths();
    assert!(!plan.temporary_paths[0].exists());
    let gif_probe = inspect_path(&engine, &reservation.partial_path)
        .await
        .expect("probe animated GIF output");
    assert_eq!(gif_probe.media.kind, MediaKind::Image);
    assert_eq!(gif_probe.media.animated, Some(true));
    assert_eq!(gif_probe.media.width, Some(540));
    let duration = gif_probe.media.duration_seconds.expect("GIF duration");
    assert!((duration - 2.2).abs() <= 0.15);
    reservation.finalize().expect("publish verified GIF output");
    let gif_bytes = std::fs::read(&reservation.final_path).expect("read looped GIF");
    assert!(
        gif_bytes
            .windows(b"NETSCAPE2.0".len())
            .any(|window| window == b"NETSCAPE2.0"),
        "forever-loop GIF is missing its loop extension"
    );

    for fixture_name in ["portrait-phone.mov", "sample.webm"] {
        let route_source_path = run.path().join(fixture_name);
        std::fs::copy(fixture_root.join(fixture_name), &route_source_path)
            .expect("copy additional GIF route fixture");
        let route_source = inspect_path(&engine, &route_source_path)
            .await
            .expect("inspect additional GIF route source");
        let mut route_settings = video_settings(OutputFormat::Gif, Resolution::Original);
        route_settings.gif = GifSettings {
            preset: GifPreset::Chat,
            start_seconds: 0.0,
            end_seconds: 1.0,
            width: 360,
            fps: 10,
            quality: Quality::Smaller,
            r#loop: LoopBehavior::Once,
        };
        let route_reservation = outputs
            .reserve(
                &route_source.path,
                OutputFormat::Gif,
                route_settings.destination,
                route_settings.destination_path.as_deref(),
                route_settings.collision_policy,
            )
            .expect("reserve additional GIF output");
        let route_plan =
            plan_conversion(&engine, &route_source, &route_settings, &route_reservation)
                .expect("build additional GIF route plan");
        let route_progress = execute_real_plan(&engine, &route_plan).await;
        assert_eq!(route_progress.last().copied(), Some(1.0));
        route_plan.cleanup_temporary_paths();
        let route_probe = inspect_path(&engine, &route_reservation.partial_path)
            .await
            .expect("probe additional GIF route output");
        assert_eq!(route_probe.media.kind, MediaKind::Image);
        assert_eq!(route_probe.media.animated, Some(true));
        assert_eq!(route_probe.media.width, Some(360));
        route_reservation
            .finalize()
            .expect("publish additional GIF route output");
        let route_bytes =
            std::fs::read(&route_reservation.final_path).expect("read once-only GIF route output");
        assert!(
            !route_bytes
                .windows(b"NETSCAPE2.0".len())
                .any(|window| window == b"NETSCAPE2.0"),
            "once-only GIF unexpectedly includes an infinite loop extension"
        );
    }
}
