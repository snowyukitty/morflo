use std::ffi::OsString;

use crate::{
    domain::{
        AnimationPolicy, ConversionError, ConversionErrorCode, ConversionSettings, GifPreset,
        GifSettings, ImageSettings, InspectedSource, JobPhase, LoopBehavior, MediaKind,
        MetadataPolicy, OutputFormat, Quality, ResizeMode, Resolution, VideoSettings,
    },
    engine::EngineRuntime,
    output::ReservedOutput,
};

#[derive(Debug, Clone)]
pub struct ConversionPlan {
    pub program: ConversionProgram,
    pub output_format: OutputFormat,
    pub temporary_paths: Vec<std::path::PathBuf>,
}

/// How a planned conversion will actually be carried out.
///
/// Morflo has two engines. The media engine runs argument arrays in a child
/// process; the built-in image engine runs in this process with no subprocess
/// at all. Making that an explicit choice keeps the difference visible to
/// execution, progress reporting and output validation instead of hiding a
/// second code path behind an empty stage list.
#[derive(Debug, Clone)]
pub enum ConversionProgram {
    Engine(Vec<ConversionStage>),
    Native(Box<crate::image_engine::NativeImageTask>),
}

impl ConversionProgram {
    pub fn stages(&self) -> &[ConversionStage] {
        match self {
            Self::Engine(stages) => stages,
            Self::Native(_) => &[],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConversionStage {
    pub args: Vec<OsString>,
    pub phase: JobPhase,
    pub duration_seconds: Option<f64>,
    pub progress_start: f64,
    pub progress_span: f64,
}

impl ConversionPlan {
    pub fn cleanup_temporary_paths(&self) {
        for path in &self.temporary_paths {
            ReservedOutput::cleanup_temporary(path);
        }
    }
}

pub fn plan_conversion(
    engine: &EngineRuntime,
    source: &InspectedSource,
    settings: &ConversionSettings,
    output: &ReservedOutput,
) -> Result<ConversionPlan, ConversionError> {
    match source.media.kind {
        MediaKind::Image => plan_image(engine, source, &settings.image, output),
        MediaKind::Video if settings.video.output_format == OutputFormat::Gif => {
            plan_gif(engine, source, &settings.gif, output)
        }
        MediaKind::Video => plan_video(engine, source, &settings.video, output),
        MediaKind::Unsupported => Err(ConversionError::new(
            ConversionErrorCode::UnsupportedFormat,
            "This file is not supported",
            "Choose a supported image or video file.",
        )),
    }
}

fn plan_video(
    engine: &EngineRuntime,
    source: &InspectedSource,
    settings: &VideoSettings,
    output: &ReservedOutput,
) -> Result<ConversionPlan, ConversionError> {
    if !matches!(
        settings.output_format,
        OutputFormat::Mp4 | OutputFormat::Webm
    ) {
        return Err(ConversionError::invalid(
            "Choose MP4 or WebM for this video conversion.",
        ));
    }
    if !engine.supports(settings.output_format) {
        return Err(ConversionError::new(
            ConversionErrorCode::UnsupportedCodec,
            "This video result is unavailable",
            "The detected local media engine does not provide Morflo's validated video encoder path.",
        ));
    }
    let duration = source.media.duration_seconds.filter(|value| *value > 0.0);
    let mut args = vec![
        OsString::from("-hide_banner"),
        OsString::from("-loglevel"),
        OsString::from("error"),
        OsString::from("-nostdin"),
        OsString::from("-y"),
        OsString::from("-i"),
        source.path.as_os_str().to_owned(),
        OsString::from("-map"),
        OsString::from(format!("0:{}", source.primary_stream_index)),
        OsString::from("-map"),
        OsString::from("0:a?"),
        OsString::from("-map"),
        OsString::from("0:s?"),
    ];

    if let Some(filter) = video_scale_filter(source, settings.resolution)? {
        args.extend([OsString::from("-vf"), OsString::from(filter)]);
    }
    match settings.metadata {
        MetadataPolicy::Remove => args.extend([
            OsString::from("-map_metadata"),
            OsString::from("-1"),
            OsString::from("-map_chapters"),
            OsString::from("-1"),
        ]),
        MetadataPolicy::Preserve => args.extend([
            OsString::from("-map_metadata"),
            OsString::from("0"),
            OsString::from("-map_chapters"),
            OsString::from("0"),
        ]),
    }

    match settings.output_format {
        OutputFormat::Mp4 => {
            let encoder = engine.preferred_h264_encoder().ok_or_else(|| {
                ConversionError::new(
                    ConversionErrorCode::UnsupportedCodec,
                    "Universal MP4 is unavailable",
                    "The detected local media engine does not include the validated H.264 encoder.",
                )
            })?;
            let (crf, preset, audio_bitrate) = match settings.quality {
                Quality::Smaller => ("28", "medium", "128k"),
                Quality::Balanced => ("23", "medium", "160k"),
                Quality::Best => ("18", "slow", "192k"),
            };
            args.extend([
                OsString::from("-c:v"),
                OsString::from(encoder),
                OsString::from("-preset"),
                OsString::from(preset),
                OsString::from("-crf"),
                OsString::from(crf),
                OsString::from("-pix_fmt"),
                OsString::from("yuv420p"),
                OsString::from("-c:a"),
                OsString::from("aac"),
                OsString::from("-b:a"),
                OsString::from(audio_bitrate),
                OsString::from("-c:s"),
                OsString::from("mov_text"),
                OsString::from("-movflags"),
                OsString::from("+faststart"),
                OsString::from("-f"),
                OsString::from("mp4"),
            ]);
        }
        OutputFormat::Webm => {
            let encoder = engine.preferred_vp9_encoder().ok_or_else(|| {
                ConversionError::new(
                    ConversionErrorCode::UnsupportedCodec,
                    "Web-friendly WebM is unavailable",
                    "The detected local media engine does not include the validated VP9 encoder.",
                )
            })?;
            let (crf, audio_bitrate) = match settings.quality {
                Quality::Smaller => ("38", "96k"),
                Quality::Balanced => ("31", "128k"),
                Quality::Best => ("24", "160k"),
            };
            args.extend([
                OsString::from("-c:v"),
                OsString::from(encoder),
                OsString::from("-crf"),
                OsString::from(crf),
                OsString::from("-b:v"),
                OsString::from("0"),
                OsString::from("-deadline"),
                OsString::from("good"),
                OsString::from("-cpu-used"),
                OsString::from("2"),
                OsString::from("-row-mt"),
                OsString::from("1"),
                OsString::from("-c:a"),
                OsString::from("libopus"),
                OsString::from("-b:a"),
                OsString::from(audio_bitrate),
                OsString::from("-c:s"),
                OsString::from("webvtt"),
                OsString::from("-f"),
                OsString::from("webm"),
            ]);
        }
        OutputFormat::Png
        | OutputFormat::Jpeg
        | OutputFormat::Webp
        | OutputFormat::Avif
        | OutputFormat::Ico
        | OutputFormat::Gif => {
            return Err(ConversionError::invalid(
                "Choose MP4 or WebM for this video conversion.",
            ));
        }
    }
    args.extend([
        OsString::from("-fps_mode"),
        OsString::from("vfr"),
        OsString::from("-max_muxing_queue_size"),
        OsString::from("2048"),
        OsString::from("-stats_period"),
        OsString::from("0.25"),
        OsString::from("-progress"),
        OsString::from("pipe:1"),
        OsString::from("-nostats"),
        output.partial_path.as_os_str().to_owned(),
    ]);
    Ok(single_stage_plan(args, settings.output_format, duration))
}

fn video_scale_filter(
    source: &InspectedSource,
    resolution: Resolution,
) -> Result<Option<String>, ConversionError> {
    let width = source
        .media
        .width
        .ok_or_else(|| ConversionError::invalid("The source video width is unavailable."))?;
    let height = source
        .media
        .height
        .ok_or_else(|| ConversionError::invalid("The source video height is unavailable."))?;
    let landscape = width >= height;
    let (landscape_width, landscape_height) = match resolution {
        Resolution::Original => {
            return if width % 2 == 0 && height % 2 == 0 {
                Ok(None)
            } else {
                Ok(Some(
                    "pad=ceil(iw/2)*2:ceil(ih/2)*2:(ow-iw)/2:(oh-ih)/2:color=black".to_owned(),
                ))
            };
        }
        Resolution::Hd1080 => (1920, 1080),
        Resolution::Hd720 => (1280, 720),
    };
    let (max_width, max_height) = if landscape {
        (landscape_width, landscape_height)
    } else {
        (landscape_height, landscape_width)
    };
    Ok(Some(format!(
        "scale=w='min(iw,{max_width})':h='min(ih,{max_height})':force_original_aspect_ratio=decrease:force_divisible_by=2:flags=lanczos,setsar=1"
    )))
}

fn plan_gif(
    engine: &EngineRuntime,
    source: &InspectedSource,
    settings: &GifSettings,
    output: &ReservedOutput,
) -> Result<ConversionPlan, ConversionError> {
    if !engine.supports(OutputFormat::Gif) {
        return Err(ConversionError::new(
            ConversionErrorCode::UnsupportedCodec,
            "Animated GIF is unavailable",
            "The detected local media engine does not include Morflo's validated palette filters and GIF encoder.",
        ));
    }
    let source_duration = source.media.duration_seconds.ok_or_else(|| {
        ConversionError::invalid(
            "The source duration is unavailable, so a GIF range cannot be set.",
        )
    })?;
    if !settings.start_seconds.is_finite()
        || !settings.end_seconds.is_finite()
        || settings.start_seconds < 0.0
        || settings.end_seconds <= settings.start_seconds
    {
        return Err(ConversionError::invalid(
            "Choose a valid GIF range with the end after the start.",
        ));
    }
    if settings.end_seconds > source_duration + 0.05 {
        return Err(ConversionError::invalid(
            "The GIF range cannot extend beyond the source video.",
        ));
    }
    if !(160..=1_280).contains(&settings.width) {
        return Err(ConversionError::invalid(
            "GIF width must be between 160 and 1,280 pixels.",
        ));
    }
    if !(5..=30).contains(&settings.fps) {
        return Err(ConversionError::invalid(
            "GIF frame rate must be between 5 and 30 frames per second.",
        ));
    }

    let selected_duration = settings.end_seconds - settings.start_seconds;
    let colors = match settings.preset {
        GifPreset::Chat => 128,
        GifPreset::Web => 192,
        GifPreset::High => 256,
    };
    let dither = match settings.quality {
        Quality::Smaller => "bayer:bayer_scale=4",
        Quality::Balanced | Quality::Best => "sierra2_4a",
    };
    let start = format!("{:.3}", settings.start_seconds);
    let end = format!("{:.3}", settings.end_seconds);
    let frames = format!(
        "trim=start={start}:end={end},setpts=PTS-STARTPTS,fps={},scale=w='max(2,trunc(min(iw,{})/2)*2)':h=-2:flags=lanczos",
        settings.fps, settings.width
    );
    let palette_path = output.create_temporary("palette", "png")?;
    let palette_filter =
        format!("{frames},palettegen=max_colors={colors}:stats_mode=diff:reserve_transparent=0");
    let palette_args = vec![
        OsString::from("-hide_banner"),
        OsString::from("-loglevel"),
        OsString::from("error"),
        OsString::from("-nostdin"),
        OsString::from("-y"),
        OsString::from("-i"),
        source.path.as_os_str().to_owned(),
        OsString::from("-vf"),
        OsString::from(palette_filter),
        OsString::from("-frames:v"),
        OsString::from("1"),
        OsString::from("-map_metadata"),
        OsString::from("-1"),
        OsString::from("-c:v"),
        OsString::from("png"),
        OsString::from("-f"),
        OsString::from("image2"),
        OsString::from("-stats_period"),
        OsString::from("0.25"),
        OsString::from("-progress"),
        OsString::from("pipe:1"),
        OsString::from("-nostats"),
        palette_path.as_os_str().to_owned(),
    ];

    let palette_use = format!(
        "[0:v]{frames}[morflo_frames];[morflo_frames][1:v]paletteuse=dither={dither}:diff_mode=rectangle[morflo_gif]"
    );
    let loop_count = match settings.r#loop {
        LoopBehavior::Forever => "0",
        LoopBehavior::Once => "-1",
    };
    let encode_args = vec![
        OsString::from("-hide_banner"),
        OsString::from("-loglevel"),
        OsString::from("error"),
        OsString::from("-nostdin"),
        OsString::from("-y"),
        OsString::from("-i"),
        source.path.as_os_str().to_owned(),
        OsString::from("-i"),
        palette_path.as_os_str().to_owned(),
        OsString::from("-filter_complex"),
        OsString::from(palette_use),
        OsString::from("-map"),
        OsString::from("[morflo_gif]"),
        OsString::from("-an"),
        OsString::from("-map_metadata"),
        OsString::from("-1"),
        OsString::from("-loop"),
        OsString::from(loop_count),
        OsString::from("-gifflags"),
        OsString::from("+offsetting+transdiff"),
        OsString::from("-f"),
        OsString::from("gif"),
        OsString::from("-stats_period"),
        OsString::from("0.25"),
        OsString::from("-progress"),
        OsString::from("pipe:1"),
        OsString::from("-nostats"),
        output.partial_path.as_os_str().to_owned(),
    ];

    Ok(ConversionPlan {
        program: ConversionProgram::Engine(vec![
            ConversionStage {
                args: palette_args,
                phase: JobPhase::PreparingPalette,
                duration_seconds: Some(selected_duration),
                progress_start: 0.0,
                progress_span: 0.34,
            },
            ConversionStage {
                args: encode_args,
                phase: JobPhase::Encoding,
                duration_seconds: Some(selected_duration),
                progress_start: 0.34,
                progress_span: 0.66,
            },
        ]),
        output_format: OutputFormat::Gif,
        temporary_paths: vec![palette_path],
    })
}

fn plan_image(
    engine: &EngineRuntime,
    source: &InspectedSource,
    settings: &ImageSettings,
    output: &ReservedOutput,
) -> Result<ConversionPlan, ConversionError> {
    if !matches!(
        settings.output_format,
        OutputFormat::Png
            | OutputFormat::Jpeg
            | OutputFormat::Webp
            | OutputFormat::Avif
            | OutputFormat::Ico
    ) {
        return Err(ConversionError::invalid(
            "Choose an image output format for an image source.",
        ));
    }
    if !engine.supports(settings.output_format) {
        return Err(ConversionError::new(
            ConversionErrorCode::UnsupportedCodec,
            "This output is unavailable",
            "The detected local media engine does not provide the required encoder or container.",
        ));
    }
    if source.media.animated == Some(true) && settings.animation != AnimationPolicy::First {
        return Err(ConversionError::new(
            ConversionErrorCode::InvalidRequest,
            "Choose how to handle animation",
            "Morflo will not silently select one frame from an animated image.",
        ));
    }

    // Without a media engine the built-in image engine does the work. With one,
    // the media engine keeps every image journey exactly as it was, so an
    // installation that has FFmpeg sees no change in behavior or output.
    if !engine.has_media_engine() {
        return plan_native_image(source, settings, output);
    }

    let resize_filter = image_resize_filter(settings)?;
    let mut args = vec![
        OsString::from("-hide_banner"),
        OsString::from("-loglevel"),
        OsString::from("error"),
        OsString::from("-nostdin"),
        OsString::from("-y"),
        OsString::from("-i"),
        source.path.as_os_str().to_owned(),
    ];
    match settings.output_format {
        OutputFormat::Ico => append_ico_mapping(&mut args, source),
        OutputFormat::Avif => append_avif_mapping(&mut args, source, resize_filter.as_deref()),
        OutputFormat::Png | OutputFormat::Jpeg | OutputFormat::Webp => {
            append_regular_image_mapping(&mut args, source, settings, resize_filter.as_deref())?;
        }
        OutputFormat::Mp4 | OutputFormat::Webm | OutputFormat::Gif => {
            return Err(ConversionError::invalid(
                "Choose an image output format for an image source.",
            ));
        }
    }

    match settings.metadata {
        MetadataPolicy::Remove => {
            args.extend([OsString::from("-map_metadata"), OsString::from("-1")]);
        }
        MetadataPolicy::Preserve => {
            args.extend([OsString::from("-map_metadata"), OsString::from("0")]);
        }
    }

    match settings.output_format {
        OutputFormat::Jpeg => {
            let quality = match settings.quality {
                Quality::Smaller => "5",
                Quality::Balanced => "3",
                Quality::Best => "2",
            };
            args.extend([
                OsString::from("-c:v"),
                OsString::from("mjpeg"),
                OsString::from("-q:v"),
                OsString::from(quality),
                OsString::from("-pix_fmt"),
                OsString::from("yuvj420p"),
            ]);
        }
        OutputFormat::Png => {
            args.extend([OsString::from("-frames:v"), OsString::from("1")]);
            args.extend([
                OsString::from("-c:v"),
                OsString::from("png"),
                OsString::from("-compression_level"),
                OsString::from("6"),
                OsString::from("-pred"),
                OsString::from("mixed"),
            ]);
        }
        OutputFormat::Webp => {
            args.extend([OsString::from("-frames:v"), OsString::from("1")]);
            let encoder = engine.preferred_webp_encoder().ok_or_else(|| {
                ConversionError::new(
                    ConversionErrorCode::UnsupportedCodec,
                    "WebP output is unavailable",
                    "The detected local media engine does not include a WebP encoder.",
                )
            })?;
            let quality = match settings.quality {
                Quality::Smaller => "70",
                Quality::Balanced => "82",
                Quality::Best => "92",
            };
            args.extend([
                OsString::from("-c:v"),
                OsString::from(encoder),
                OsString::from("-quality"),
                OsString::from(quality),
                OsString::from("-compression_level"),
                OsString::from("5"),
            ]);
        }
        OutputFormat::Avif => {
            let encoder = engine.preferred_av1_encoder().ok_or_else(|| {
                ConversionError::new(
                    ConversionErrorCode::UnsupportedCodec,
                    "AVIF output is unavailable",
                    "The detected local media engine does not include Morflo's validated AV1 encoder.",
                )
            })?;
            let quality = match settings.quality {
                Quality::Smaller => "40",
                Quality::Balanced => "30",
                Quality::Best => "20",
            };
            args.extend([
                OsString::from("-c:v"),
                OsString::from(encoder),
                OsString::from("-still-picture"),
                OsString::from("1"),
                OsString::from("-crf"),
                OsString::from(quality),
                OsString::from("-b:v"),
                OsString::from("0"),
                OsString::from("-cpu-used"),
                OsString::from("6"),
            ]);
            if source.media.has_alpha == Some(true) {
                args.extend([
                    OsString::from("-pix_fmt:v:0"),
                    OsString::from("yuv444p"),
                    OsString::from("-pix_fmt:v:1"),
                    OsString::from("gray"),
                    OsString::from("-frames:v:0"),
                    OsString::from("1"),
                    OsString::from("-frames:v:1"),
                    OsString::from("1"),
                ]);
            } else {
                args.extend([
                    OsString::from("-pix_fmt"),
                    OsString::from("yuv420p"),
                    OsString::from("-frames:v"),
                    OsString::from("1"),
                ]);
            }
            args.extend([OsString::from("-f"), OsString::from("avif")]);
        }
        OutputFormat::Ico => {
            args.extend([
                OsString::from("-c:v"),
                OsString::from("png"),
                OsString::from("-frames:v:0"),
                OsString::from("1"),
                OsString::from("-frames:v:1"),
                OsString::from("1"),
                OsString::from("-frames:v:2"),
                OsString::from("1"),
                OsString::from("-frames:v:3"),
                OsString::from("1"),
                OsString::from("-f"),
                OsString::from("ico"),
            ]);
        }
        OutputFormat::Mp4 | OutputFormat::Webm | OutputFormat::Gif => {
            return Err(ConversionError::invalid(
                "Choose an image output format for an image source.",
            ));
        }
    }

    args.extend([
        OsString::from("-progress"),
        OsString::from("pipe:1"),
        OsString::from("-nostats"),
        output.partial_path.as_os_str().to_owned(),
    ]);
    Ok(single_stage_plan(args, settings.output_format, None))
}

/// Plan an image conversion for Morflo's built-in engine.
///
/// The built-in encoders write no source metadata, so a `Preserve` request is
/// refused with the reason instead of being quietly downgraded to `Remove`.
/// Losing metadata the user explicitly asked to keep would be a silent
/// data-loss surprise, which is exactly what Morflo promises not to do.
fn plan_native_image(
    source: &InspectedSource,
    settings: &ImageSettings,
    output: &ReservedOutput,
) -> Result<ConversionPlan, ConversionError> {
    if !crate::image_engine::honors_metadata(settings.metadata) {
        return Err(ConversionError::new(
            ConversionErrorCode::UnsupportedCodec,
            "Keeping metadata needs a local media engine",
            "Morflo's built-in image engine writes no source metadata. Choose Remove metadata, or install a compatible FFmpeg build to keep it.",
        ));
    }
    // Validate the resize request through the shared rules before any work
    // starts, so an impossible size fails at planning time on both engines.
    image_resize_filter(settings)?;

    Ok(ConversionPlan {
        program: ConversionProgram::Native(Box::new(crate::image_engine::NativeImageTask {
            source: source.path.clone(),
            target: output.partial_path.clone(),
            format: settings.output_format,
            settings: settings.clone(),
        })),
        output_format: settings.output_format,
        temporary_paths: Vec::new(),
    })
}

fn single_stage_plan(
    args: Vec<OsString>,
    output_format: OutputFormat,
    duration_seconds: Option<f64>,
) -> ConversionPlan {
    ConversionPlan {
        program: ConversionProgram::Engine(vec![ConversionStage {
            args,
            phase: JobPhase::Encoding,
            duration_seconds,
            progress_start: 0.0,
            progress_span: 1.0,
        }]),
        output_format,
        temporary_paths: Vec::new(),
    }
}

fn append_regular_image_mapping(
    args: &mut Vec<OsString>,
    source: &InspectedSource,
    settings: &ImageSettings,
    resize_filter: Option<&str>,
) -> Result<(), ConversionError> {
    let flatten_alpha = source.media.has_alpha == Some(true)
        && matches!(settings.output_format, OutputFormat::Jpeg);
    if flatten_alpha {
        let background = validated_background(&settings.background)?;
        let (prefix, input) = filter_source(source);
        let resize = resize_filter
            .map(|filter| format!("{filter},"))
            .unwrap_or_default();
        let graph = format!(
            "{prefix}{input}{resize}format=rgba,split=2[morflo_fg][morflo_canvas];[morflo_canvas]drawbox=color={background}:t=fill[morflo_bg];[morflo_bg][morflo_fg]overlay=shortest=1:format=auto,format=yuvj420p[morflo_out]"
        );
        args.extend([
            OsString::from("-filter_complex"),
            OsString::from(graph),
            OsString::from("-map"),
            OsString::from("[morflo_out]"),
        ]);
    } else if source.alpha_stream_index.is_some() {
        let (prefix, input) = filter_source(source);
        let transform = resize_filter.unwrap_or("null");
        args.extend([
            OsString::from("-filter_complex"),
            OsString::from(format!("{prefix}{input}{transform}[morflo_out]")),
            OsString::from("-map"),
            OsString::from("[morflo_out]"),
        ]);
    } else {
        args.extend([
            OsString::from("-map"),
            OsString::from(format!("0:{}", source.primary_stream_index)),
        ]);
        if let Some(filter) = resize_filter {
            args.extend([OsString::from("-vf"), OsString::from(filter)]);
        }
    }
    if matches!(settings.output_format, OutputFormat::Jpeg) {
        args.extend([OsString::from("-frames:v"), OsString::from("1")]);
    }
    Ok(())
}

fn append_avif_mapping(
    args: &mut Vec<OsString>,
    source: &InspectedSource,
    resize_filter: Option<&str>,
) {
    if source.media.has_alpha == Some(true) {
        let (prefix, input) = filter_source(source);
        let resize = resize_filter
            .map(|filter| format!("{filter},"))
            .unwrap_or_default();
        let graph = format!(
            "{prefix}{input}{resize}format=rgba,split=2[morflo_color][morflo_alpha];[morflo_color]format=yuv444p[morflo_color_out];[morflo_alpha]alphaextract,format=gray,setparams=colorspace=bt709:color_primaries=bt709:color_trc=bt709:range=full[morflo_alpha_out]"
        );
        args.extend([
            OsString::from("-filter_complex"),
            OsString::from(graph),
            OsString::from("-map"),
            OsString::from("[morflo_color_out]"),
            OsString::from("-map"),
            OsString::from("[morflo_alpha_out]"),
        ]);
    } else {
        args.extend([
            OsString::from("-map"),
            OsString::from(format!("0:{}", source.primary_stream_index)),
        ]);
        if let Some(filter) = resize_filter {
            args.extend([OsString::from("-vf"), OsString::from(filter)]);
        }
    }
}

fn append_ico_mapping(args: &mut Vec<OsString>, source: &InspectedSource) {
    let (prefix, input) = filter_source(source);
    let graph = format!(
        "{prefix}{input}format=rgba,split=4[m16][m32][m48][m256];[m16]scale=16:16:force_original_aspect_ratio=decrease:flags=lanczos,pad=16:16:(ow-iw)/2:(oh-ih)/2:color=0x00000000[o16];[m32]scale=32:32:force_original_aspect_ratio=decrease:flags=lanczos,pad=32:32:(ow-iw)/2:(oh-ih)/2:color=0x00000000[o32];[m48]scale=48:48:force_original_aspect_ratio=decrease:flags=lanczos,pad=48:48:(ow-iw)/2:(oh-ih)/2:color=0x00000000[o48];[m256]scale=256:256:force_original_aspect_ratio=decrease:flags=lanczos,pad=256:256:(ow-iw)/2:(oh-ih)/2:color=0x00000000[o256]"
    );
    args.extend([
        OsString::from("-filter_complex"),
        OsString::from(graph),
        OsString::from("-map"),
        OsString::from("[o16]"),
        OsString::from("-map"),
        OsString::from("[o32]"),
        OsString::from("-map"),
        OsString::from("[o48]"),
        OsString::from("-map"),
        OsString::from("[o256]"),
    ]);
}

fn filter_source(source: &InspectedSource) -> (String, String) {
    if let Some(alpha_index) = source.alpha_stream_index {
        (
            format!(
                "[0:{}][0:{alpha_index}]alphamerge[morflo_source];",
                source.primary_stream_index
            ),
            "[morflo_source]".to_owned(),
        )
    } else {
        (
            String::new(),
            format!("[0:{}]", source.primary_stream_index),
        )
    }
}

fn image_resize_filter(settings: &ImageSettings) -> Result<Option<String>, ConversionError> {
    let filter = match settings.resize_mode {
        ResizeMode::Original => return Ok(None),
        ResizeMode::Width => {
            let width = validate_dimension(settings.width, "width")?;
            format!("scale={width}:-2:flags=lanczos")
        }
        ResizeMode::Height => {
            let height = validate_dimension(settings.height, "height")?;
            format!("scale=-2:{height}:flags=lanczos")
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
            format!("scale=trunc(iw*{percentage}/100):trunc(ih*{percentage}/100):flags=lanczos")
        }
        ResizeMode::Contain => {
            let width = validate_dimension(settings.width, "width")?;
            let height = validate_dimension(settings.height, "height")?;
            format!("scale={width}:{height}:force_original_aspect_ratio=decrease:flags=lanczos")
        }
        ResizeMode::Cover => {
            let width = validate_dimension(settings.width, "width")?;
            let height = validate_dimension(settings.height, "height")?;
            format!(
                "scale={width}:{height}:force_original_aspect_ratio=increase:flags=lanczos,crop={width}:{height}"
            )
        }
    };
    Ok(Some(filter))
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

fn validated_background(background: &str) -> Result<String, ConversionError> {
    let value = background.strip_prefix('#').unwrap_or(background);
    if value.len() != 6 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(ConversionError::invalid(
            "Choose a valid six-digit background color.",
        ));
    }
    Ok(format!("0x{}", value.to_ascii_uppercase()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        CollisionPolicy, DestinationMode, GifPreset, GifSettings, JobPhase, JobStatus,
        LoopBehavior, MediaFile, MediaWarning, MetadataPolicy, ResizeMode,
    };

    fn image_settings(resize_mode: ResizeMode) -> ImageSettings {
        ImageSettings {
            output_format: OutputFormat::Jpeg,
            quality: Quality::Balanced,
            resize_mode,
            width: Some(800),
            height: Some(600),
            percentage: Some(50),
            background: "#F5F1E8".to_owned(),
            metadata: MetadataPolicy::Remove,
            animation: AnimationPolicy::Ask,
        }
    }

    fn video_source(width: u32, height: u32) -> InspectedSource {
        InspectedSource {
            media: MediaFile {
                id: "video".to_owned(),
                name: "video.mp4".to_owned(),
                extension: "mp4".to_owned(),
                size_bytes: 1,
                kind: MediaKind::Video,
                status: JobStatus::Ready,
                phase: JobPhase::Waiting,
                progress: None,
                width: Some(width),
                height: Some(height),
                duration_seconds: Some(10.0),
                has_alpha: None,
                animated: None,
                video_codec: Some("h264".to_owned()),
                video_tracks: Some(1),
                audio_tracks: Some(1),
                subtitle_tracks: Some(0),
                chapter_count: Some(0),
                attachment_tracks: Some(0),
                data_tracks: Some(0),
                hdr: Some(false),
                warnings: Vec::<MediaWarning>::new(),
                output: None,
                error: None,
            },
            path: "video.mp4".into(),
            format_name: "mov,mp4".to_owned(),
            primary_codec: Some("h264".to_owned()),
            primary_stream_index: 0,
            alpha_stream_index: None,
        }
    }

    #[test]
    fn builds_validated_resize_filters() {
        assert_eq!(
            image_resize_filter(&image_settings(ResizeMode::Width)).expect("width filter"),
            Some("scale=800:-2:flags=lanczos".to_owned())
        );
        assert_eq!(
            image_resize_filter(&image_settings(ResizeMode::Cover)).expect("cover filter"),
            Some(
                "scale=800:600:force_original_aspect_ratio=increase:flags=lanczos,crop=800:600"
                    .to_owned()
            )
        );
    }

    #[test]
    fn validates_background_as_data_not_filter_syntax() {
        assert_eq!(
            validated_background("#f5f1e8").expect("validated color"),
            "0xF5F1E8"
        );
        assert!(validated_background("red;movie=/private/file").is_err());
    }

    #[test]
    fn caps_landscape_and_portrait_without_upscaling() {
        let landscape = video_scale_filter(&video_source(3840, 2160), Resolution::Hd1080)
            .expect("landscape filter")
            .expect("scaled landscape");
        assert!(landscape.contains("min(iw,1920)"));
        assert!(landscape.contains("min(ih,1080)"));

        let portrait = video_scale_filter(&video_source(2160, 3840), Resolution::Hd720)
            .expect("portrait filter")
            .expect("scaled portrait");
        assert!(portrait.contains("min(iw,720)"));
        assert!(portrait.contains("min(ih,1280)"));
        assert_eq!(
            video_scale_filter(&video_source(1280, 720), Resolution::Original)
                .expect("original filter"),
            None
        );
    }

    #[test]
    fn constructs_video_arguments_without_shell_text() {
        let directory = tempfile::tempdir().expect("temporary planner directory");
        let source_path = directory.path().join("日本語 O'Reilly video.mp4");
        std::fs::write(&source_path, b"fixture placeholder").expect("write source placeholder");
        let mut source = video_source(1920, 1080);
        source.path = source_path.clone();
        let settings = ConversionSettings {
            image: image_settings(ResizeMode::Original),
            video: VideoSettings {
                output_format: OutputFormat::Mp4,
                resolution: Resolution::Hd720,
                quality: Quality::Balanced,
                metadata: MetadataPolicy::Preserve,
            },
            gif: GifSettings {
                preset: GifPreset::Web,
                start_seconds: 0.0,
                end_seconds: 2.0,
                width: 540,
                fps: 12,
                quality: Quality::Balanced,
                r#loop: LoopBehavior::Forever,
            },
            destination: DestinationMode::Same,
            destination_path: None,
            collision_policy: CollisionPolicy::Suffix,
        };
        let outputs = crate::output::OutputReservations::default();
        let reservation = outputs
            .reserve(
                &source_path,
                OutputFormat::Mp4,
                DestinationMode::Same,
                None,
                CollisionPolicy::Suffix,
            )
            .expect("reserve planner output");
        let engine = EngineRuntime::for_planner_tests();
        let plan = plan_conversion(&engine, &source, &settings, &reservation)
            .expect("construct video plan");
        let args = plan.program.stages()[0]
            .args
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(
            plan.program.stages()[0]
                .args
                .iter()
                .any(|value| value == source_path.as_os_str())
        );
        assert!(args.windows(2).any(|pair| pair == ["-c:v", "libx264"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "0:a?"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "0:s?"]));
        assert!(args.iter().any(|value| value.contains("min(iw,1280)")));
        assert!(!args.iter().any(|value| value == "sh" || value == "cmd.exe"));
        reservation.cleanup_partial();
    }
}
