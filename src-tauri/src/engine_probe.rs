use std::{
    collections::{BTreeSet, HashSet},
    path::Path,
    time::Duration,
};

use tokio::time::timeout;

use crate::{
    domain::{FormatCapability, OutputFormat},
    engine_process::engine_command,
};

const ENGINE_TIMEOUT: Duration = Duration::from_secs(12);
const MAX_CAPTURE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineProbe {
    pub version: String,
    pub build_configuration: String,
    pub decoders: Vec<String>,
    pub encoders: Vec<String>,
    pub demuxers: Vec<String>,
    pub muxers: Vec<String>,
    pub filters: Vec<String>,
}

impl EngineProbe {
    pub fn outputs(&self) -> Vec<FormatCapability> {
        derive_outputs(
            &self.encoders.iter().cloned().collect(),
            &self.muxers.iter().cloned().collect(),
            &self.filters.iter().cloned().collect(),
        )
    }

    pub fn available_outputs(&self) -> Vec<OutputFormat> {
        self.outputs()
            .into_iter()
            .filter_map(|capability| capability.available.then_some(capability.format))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineProbeError(String);

impl EngineProbeError {
    fn new(message: impl Into<String>) -> Self {
        Self(bounded(&message.into(), 320))
    }
}

impl std::fmt::Display for EngineProbeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for EngineProbeError {}

pub async fn probe_pair(ffmpeg: &Path, ffprobe: &Path) -> Result<EngineProbe, EngineProbeError> {
    let ffmpeg_version = capture(ffmpeg, &["-version"], "ffmpeg version").await?;
    let version = parse_tool_version(&ffmpeg_version, "ffmpeg")
        .ok_or_else(|| EngineProbeError::new("ffmpeg did not report a compatible version"))?;
    let build_configuration = parse_build_configuration(&ffmpeg_version)
        .ok_or_else(|| EngineProbeError::new("ffmpeg did not report its build configuration"))?;

    let ffprobe_version = capture(ffprobe, &["-version"], "ffprobe version").await?;
    let probe_version = parse_tool_version(&ffprobe_version, "ffprobe")
        .ok_or_else(|| EngineProbeError::new("ffprobe did not report a compatible version"))?;
    let probe_configuration = parse_build_configuration(&ffprobe_version)
        .ok_or_else(|| EngineProbeError::new("ffprobe did not report its build configuration"))?;
    if version != probe_version {
        return Err(EngineProbeError::new(
            "ffmpeg and ffprobe reported different versions",
        ));
    }
    if build_configuration != probe_configuration {
        return Err(EngineProbeError::new(
            "ffmpeg and ffprobe reported different build configurations",
        ));
    }

    Ok(EngineProbe {
        version,
        build_configuration,
        decoders: probe_listing(ffmpeg, "-decoders", "decoder").await?,
        encoders: probe_listing(ffmpeg, "-encoders", "encoder").await?,
        demuxers: probe_listing(ffmpeg, "-demuxers", "demuxer").await?,
        muxers: probe_listing(ffmpeg, "-muxers", "muxer").await?,
        filters: probe_listing(ffmpeg, "-filters", "filter").await?,
    })
}

async fn probe_listing(
    ffmpeg: &Path,
    argument: &'static str,
    label: &'static str,
) -> Result<Vec<String>, EngineProbeError> {
    let output = capture(ffmpeg, &["-hide_banner", argument], label).await?;
    let listing = parse_listing(&output);
    if listing.is_empty() {
        return Err(EngineProbeError::new(format!(
            "ffmpeg returned an empty {label} inventory"
        )));
    }
    Ok(listing)
}

async fn capture(
    path: &Path,
    args: &[&str],
    label: &'static str,
) -> Result<String, EngineProbeError> {
    let mut command = engine_command(path);
    command.args(args).kill_on_drop(true);
    let output = match timeout(ENGINE_TIMEOUT, command.output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(error)) => {
            return Err(EngineProbeError::new(format!(
                "{label} probe could not start: {}",
                bounded(&error.to_string(), 160)
            )));
        }
        Err(_) => return Err(EngineProbeError::new(format!("{label} probe timed out"))),
    };
    if !output.status.success() {
        return Err(EngineProbeError::new(format!(
            "{label} probe failed with exit code {}",
            output.status.code().unwrap_or(-1)
        )));
    }
    if output.stdout.len() > MAX_CAPTURE_BYTES || output.stderr.len() > MAX_CAPTURE_BYTES {
        return Err(EngineProbeError::new(format!(
            "{label} probe exceeded the output limit"
        )));
    }
    String::from_utf8(output.stdout)
        .map_err(|_| EngineProbeError::new(format!("{label} probe returned non-UTF-8 output")))
}

pub(crate) fn parse_tool_version(output: &str, tool: &str) -> Option<String> {
    let first_line = output.lines().next()?;
    let mut fields = first_line.split_whitespace();
    (fields.next()? == tool && fields.next()? == "version")
        .then(|| fields.next().map(str::to_owned))
        .flatten()
}

fn parse_build_configuration(output: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| line.strip_prefix("configuration: ").map(str::to_owned))
}

pub(crate) fn parse_listing(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let flags = fields.next()?;
            let name = fields.next()?;
            let looks_like_flags = flags.len() <= 7
                && flags
                    .chars()
                    .all(|character| character == '.' || character.is_ascii_alphabetic());
            (looks_like_flags && name != "=").then(|| name.to_owned())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn derive_outputs(
    encoders: &HashSet<String>,
    muxers: &HashSet<String>,
    filters: &HashSet<String>,
) -> Vec<FormatCapability> {
    OutputFormat::ALL
        .into_iter()
        .map(|format| {
            let (available, reason) = match format {
                OutputFormat::Png => requirement(encoders.contains("png"), "PNG encoder"),
                OutputFormat::Jpeg => requirement(encoders.contains("mjpeg"), "JPEG encoder"),
                OutputFormat::Webp => requirement(
                    encoders.contains("libwebp") || encoders.contains("libwebp_anim"),
                    "WebP encoder",
                ),
                OutputFormat::Avif => requirement(
                    muxers.contains("avif") && encoders.contains("libaom-av1"),
                    "AVIF muxer and the validated libaom AV1 encoder",
                ),
                OutputFormat::Ico => requirement(
                    muxers.contains("ico") && encoders.contains("png") && filters.contains("scale"),
                    "ICO muxer, PNG encoder, and scale filter",
                ),
                OutputFormat::Mp4 => requirement(
                    muxers.contains("mp4")
                        && encoders.contains("libx264")
                        && encoders.contains("aac")
                        && filters.contains("scale"),
                    "MP4 muxer, libx264, AAC encoder, and scale filter",
                ),
                OutputFormat::Webm => requirement(
                    muxers.contains("webm")
                        && encoders.contains("libvpx-vp9")
                        && encoders.contains("libopus")
                        && filters.contains("scale"),
                    "WebM muxer, VP9, Opus, and scale filter",
                ),
                OutputFormat::Gif => requirement(
                    encoders.contains("gif")
                        && encoders.contains("png")
                        && filters.contains("palettegen")
                        && filters.contains("paletteuse")
                        && filters.contains("scale")
                        && filters.contains("fps"),
                    "GIF/PNG encoders plus palette, scale, and frame-rate filters",
                ),
            };
            FormatCapability {
                format,
                available,
                reason,
            }
        })
        .collect()
}

fn requirement(available: bool, feature: &str) -> (bool, Option<String>) {
    (
        available,
        (!available).then(|| format!("The detected FFmpeg build is missing {feature}.")),
    )
}

fn bounded(value: &str, limit: usize) -> String {
    let mut output = value.chars().take(limit).collect::<String>();
    if value.chars().count() > limit {
        output.push('…');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_version_configuration_and_sorted_inventory() {
        let version = "ffmpeg version 8.1.2-test Copyright\nconfiguration: --disable-network";
        assert_eq!(
            parse_tool_version(version, "ffmpeg"),
            Some("8.1.2-test".to_owned())
        );
        assert_eq!(
            parse_build_configuration(version),
            Some("--disable-network".to_owned())
        );
        assert_eq!(
            parse_listing(" V..... png desc\n V..... mjpeg desc\n V..... png duplicate"),
            vec!["mjpeg".to_owned(), "png".to_owned()]
        );
    }
}
