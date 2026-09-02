use std::{
    collections::HashSet,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::sync::Mutex;

#[cfg(windows)]
use std::{
    ffi::c_void,
    os::windows::{ffi::OsStringExt, fs::MetadataExt},
    ptr,
};

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::ERROR_SUCCESS,
    System::Registry::{
        HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ,
        RegGetValueW,
    },
};

use crate::domain::{
    CapabilityRegistry, ConversionError, ConversionErrorCode, EngineInfo, EngineSource,
    FormatCapability, OutputFormat,
};
use crate::{
    engine_bundle::{EMBEDDED_REVIEWED_MANIFEST_SHA256, verify_bundle},
    engine_probe::{EngineProbe, derive_outputs, probe_pair},
};

/// The external FFmpeg pair and what it proved it can encode.
///
/// Optional at runtime: Morflo also runs on its built-in image engine alone,
/// and every caller that needs a subprocess must ask for these tools and handle
/// their absence rather than assuming a media engine exists.
#[derive(Debug, Clone)]
pub struct MediaTools {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    encoders: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct EngineRuntime {
    media: Option<MediaTools>,
    pub registry: CapabilityRegistry,
}

impl EngineRuntime {
    /// A runtime with no media engine, backed by Morflo's compiled-in image
    /// engine. This is a working state, not a failure: image journeys run and
    /// video formats report why they are unavailable.
    pub fn native_only(diagnostic: String) -> Self {
        Self {
            media: None,
            registry: CapabilityRegistry {
                engine: crate::image_engine::engine_info(Some(diagnostic)),
                outputs: crate::image_engine::outputs(),
            },
        }
    }

    /// The FFmpeg pair, or a typed error naming what is missing.
    pub fn media(&self) -> Result<&MediaTools, ConversionError> {
        self.media.as_ref().ok_or_else(|| {
            ConversionError::new(
                ConversionErrorCode::EngineMissing,
                "This step needs a local media engine",
                "Morflo's built-in engine converts images. Install a compatible FFmpeg build or choose an engine folder in Diagnostics for video, GIF, WebP and AVIF.",
            )
        })
    }

    /// Whether a media engine is present at all.
    pub fn has_media_engine(&self) -> bool {
        self.media.is_some()
    }

    pub fn supports(&self, format: OutputFormat) -> bool {
        self.registry
            .outputs
            .iter()
            .any(|capability| capability.format == format && capability.available)
    }

    fn encoders(&self) -> Option<&HashSet<String>> {
        self.media.as_ref().map(|media| &media.encoders)
    }

    pub fn preferred_h264_encoder(&self) -> Option<&'static str> {
        self.encoders()?.contains("libx264").then_some("libx264")
    }

    pub fn preferred_vp9_encoder(&self) -> Option<&'static str> {
        self.encoders()?
            .contains("libvpx-vp9")
            .then_some("libvpx-vp9")
    }

    pub fn preferred_av1_encoder(&self) -> Option<&'static str> {
        self.encoders()?
            .contains("libaom-av1")
            .then_some("libaom-av1")
    }

    pub fn preferred_webp_encoder(&self) -> Option<&'static str> {
        let encoders = self.encoders()?;
        ["libwebp", "libwebp_anim"]
            .into_iter()
            .find(|encoder| encoders.contains(*encoder))
    }

    #[cfg(test)]
    pub(crate) fn for_planner_tests() -> Self {
        Self {
            media: Some(MediaTools {
                ffmpeg: PathBuf::from("ffmpeg"),
                ffprobe: PathBuf::from("ffprobe"),
                encoders: ["libx264", "libvpx-vp9", "libwebp", "libaom-av1"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            }),
            registry: CapabilityRegistry {
                engine: EngineInfo {
                    available: true,
                    name: "FFmpeg test adapter".to_owned(),
                    version: Some("test".to_owned()),
                    source: EngineSource::Project,
                    diagnostic: None,
                },
                outputs: OutputFormat::ALL
                    .into_iter()
                    .map(|format| FormatCapability {
                        format,
                        available: true,
                        reason: None,
                    })
                    .collect(),
            },
        }
    }
}

#[derive(Debug, Default)]
struct EngineState {
    selected_directory: Option<PathBuf>,
    runtime: Option<Result<Arc<EngineRuntime>, ConversionError>>,
}

#[derive(Debug, Default)]
pub struct EngineService {
    state: Mutex<EngineState>,
}

impl EngineService {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn runtime(&self) -> Result<Arc<EngineRuntime>, ConversionError> {
        let mut state = self.state.lock().await;
        if let Some(runtime) = &state.runtime {
            return runtime.clone();
        }

        let discovered = discover_engine(state.selected_directory.as_deref())
            .await
            .map(Arc::new);
        state.runtime = Some(discovered.clone());
        discovered
    }

    pub async fn capabilities(&self) -> CapabilityRegistry {
        match self.runtime().await {
            Ok(runtime) => runtime.registry.clone(),
            Err(error) => missing_registry(error.message),
        }
    }

    pub async fn refresh(&self) -> CapabilityRegistry {
        self.state.lock().await.runtime = None;
        self.capabilities().await
    }

    pub async fn select_directory(
        &self,
        directory: PathBuf,
    ) -> Result<CapabilityRegistry, ConversionError> {
        let metadata = std::fs::symlink_metadata(&directory).map_err(|error| {
            ConversionError::invalid("Choose a folder that contains ffmpeg and ffprobe.")
                .with_details(error.to_string())
        })?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
            return Err(ConversionError::invalid(
                "Choose a folder that contains ffmpeg and ffprobe.",
            ));
        }

        let runtime = probe_candidate(pair_in(directory.clone(), EngineSource::Selected)).await?;
        let registry = runtime.registry.clone();
        let mut state = self.state.lock().await;
        state.selected_directory = Some(directory);
        state.runtime = Some(Ok(Arc::new(runtime)));
        Ok(registry)
    }
}

#[derive(Debug)]
struct EngineCandidate {
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
    source: EngineSource,
}

async fn discover_engine(
    selected_directory: Option<&Path>,
) -> Result<EngineRuntime, ConversionError> {
    let mut diagnostics = Vec::new();

    if let Some(directory) = selected_directory {
        // A session folder that verified earlier can disappear later. Report why
        // it stopped working, but keep the built-in image engine available
        // instead of turning the whole application back into a dead end.
        match probe_candidate(pair_in(directory.to_path_buf(), EngineSource::Selected)).await {
            Ok(runtime) => return Ok(runtime),
            Err(error) => diagnostics.push(format!("Session engine folder: {error}")),
        }
    }

    // A packaged engine that fails verification stays terminal. Falling back
    // here would let a tampered bundle degrade into a working application and
    // hide that failure, so this error is deliberately not recoverable.
    if let Some(directory) = bundled_engine_directory()? {
        return probe_reviewed_bundle(&directory).await;
    }

    let candidates = development_engine_candidates();

    for candidate in candidates {
        if let Err(reason) = validate_candidate(&candidate) {
            diagnostics.push(reason);
            continue;
        }
        match probe_candidate(candidate).await {
            Ok(runtime) => return Ok(runtime),
            Err(error) => diagnostics.push(error.to_string()),
        }
    }

    let details = if diagnostics.is_empty() {
        "No development or system engine pair was found".to_owned()
    } else {
        diagnostics
            .into_iter()
            .take(8)
            .collect::<Vec<_>>()
            .join("\n")
    };
    // A missing media engine is no longer a failure. Morflo continues on its
    // built-in image engine and keeps the reason, so Diagnostics can still
    // explain exactly what video, GIF, WebP and AVIF output would need.
    Ok(EngineRuntime::native_only(details))
}

fn bundled_engine_directory() -> Result<Option<PathBuf>, ConversionError> {
    let executable = std::env::current_exe().map_err(|_| {
        reviewed_bundle_error(
            "BUNDLE_BOUNDARY_UNAVAILABLE: the application location could not be determined",
        )
    })?;
    let parent = executable.parent().ok_or_else(|| {
        reviewed_bundle_error(
            "BUNDLE_BOUNDARY_UNAVAILABLE: the application location has no parent directory",
        )
    })?;
    let directory = parent.join("engines");
    match std::fs::symlink_metadata(&directory) {
        Ok(_) => Ok(Some(directory)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(reviewed_bundle_error(
            "BUNDLE_BOUNDARY_UNREADABLE: the packaged engine location could not be inspected",
        )),
    }
}

async fn probe_reviewed_bundle(directory: &Path) -> Result<EngineRuntime, ConversionError> {
    let expected_manifest_sha256 = EMBEDDED_REVIEWED_MANIFEST_SHA256.ok_or_else(|| {
        reviewed_bundle_error(
            "MANIFEST_TRUST_PIN_MISSING: this build has no separately reviewed manifest digest",
        )
    })?;
    let verified = verify_bundle(directory, Some(expected_manifest_sha256))
        .await
        .map_err(|error| reviewed_bundle_error(&error.to_string()))?;
    let diagnostic = format!(
        "Reviewed bundled engine; manifest schema {}, exact manifest and artifact hashes, matching tool versions, build configuration, and capability inventory verified offline; approval valid through {}",
        verified.manifest.schema_version, verified.manifest.review.valid_until
    );
    Ok(runtime_from_probe(
        verified.ffmpeg,
        verified.ffprobe,
        EngineSource::Bundled,
        verified.manifest.engine.name,
        verified.probe,
        diagnostic,
    ))
}

fn reviewed_bundle_error(details: &str) -> ConversionError {
    ConversionError::new(
        ConversionErrorCode::EngineFailed,
        "Packaged media engine could not be verified",
        "This Morflo package cannot use its media engine. Choose a session engine or install a corrected package.",
    )
    .with_details(format!("Reviewed bundle rejected: {details}"))
}

fn development_engine_candidates() -> Vec<EngineCandidate> {
    let mut candidates = Vec::new();

    if let (Some(ffmpeg), Some(ffprobe)) = (
        std::env::var_os("MORFLO_FFMPEG_PATH"),
        std::env::var_os("MORFLO_FFPROBE_PATH"),
    ) {
        candidates.push(EngineCandidate {
            ffmpeg: PathBuf::from(ffmpeg),
            ffprobe: PathBuf::from(ffprobe),
            source: EngineSource::Project,
        });
    }

    if let Ok(directory) = std::env::current_dir() {
        candidates.push(pair_in(directory.join("engines"), EngineSource::Project));
        candidates.push(pair_in(
            directory.join("src-tauri").join("binaries"),
            EngineSource::Project,
        ));
    }

    if let (Ok(ffmpeg), Ok(ffprobe)) = (which::which("ffmpeg"), which::which("ffprobe")) {
        candidates.push(EngineCandidate {
            ffmpeg,
            ffprobe,
            source: EngineSource::System,
        });
    }

    #[cfg(windows)]
    candidates.extend(current_windows_path_candidates());

    candidates
}

#[cfg(windows)]
fn current_windows_path_candidates() -> Vec<EngineCandidate> {
    const MACHINE_ENVIRONMENT: &str =
        r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";
    const USER_ENVIRONMENT: &str = "Environment";

    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    let path_values = [
        read_registry_string(HKEY_LOCAL_MACHINE, MACHINE_ENVIRONMENT, "Path"),
        read_registry_string(HKEY_CURRENT_USER, USER_ENVIRONMENT, "Path"),
    ];

    for value in path_values.into_iter().flatten() {
        for directory in std::env::split_paths(&value) {
            if directory.as_os_str().is_empty() {
                continue;
            }
            let candidate = pair_in(directory, EngineSource::System);
            if !candidate.ffmpeg.is_file() || !candidate.ffprobe.is_file() {
                continue;
            }
            let key = format!(
                "{}|{}",
                candidate.ffmpeg.to_string_lossy().to_ascii_lowercase(),
                candidate.ffprobe.to_string_lossy().to_ascii_lowercase()
            );
            if seen.insert(key) {
                candidates.push(candidate);
            }
        }
    }
    candidates
}

#[cfg(windows)]
fn read_registry_string(root: HKEY, subkey: &str, value: &str) -> Option<OsString> {
    let subkey = wide_null(subkey);
    let value = wide_null(value);
    let flags = RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ;
    let mut byte_count = 0u32;

    // SAFETY: The key handles are predefined, both string pointers are NUL-terminated, and the
    // first call intentionally supplies a null data buffer so Windows reports the required size.
    let status = unsafe {
        RegGetValueW(
            root,
            subkey.as_ptr(),
            value.as_ptr(),
            flags,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut byte_count,
        )
    };
    if status != ERROR_SUCCESS || byte_count < 2 {
        return None;
    }

    let mut buffer = vec![0u16; (byte_count as usize).div_ceil(2)];
    // SAFETY: `buffer` owns at least `byte_count` writable bytes and all other pointers remain
    // valid for the duration of the call. RegGetValueW writes a UTF-16 string for these flags.
    let status = unsafe {
        RegGetValueW(
            root,
            subkey.as_ptr(),
            value.as_ptr(),
            flags,
            ptr::null_mut(),
            buffer.as_mut_ptr().cast::<c_void>(),
            &mut byte_count,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }

    let used = (byte_count as usize / 2).min(buffer.len());
    buffer.truncate(used);
    while buffer.last() == Some(&0) {
        buffer.pop();
    }
    Some(OsString::from_wide(&buffer))
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn pair_in(directory: PathBuf, source: EngineSource) -> EngineCandidate {
    EngineCandidate {
        ffmpeg: directory.join(engine_filename("ffmpeg")),
        ffprobe: directory.join(engine_filename("ffprobe")),
        source,
    }
}

fn engine_filename(stem: &str) -> OsString {
    #[cfg(windows)]
    {
        OsString::from(format!("{stem}.exe"))
    }
    #[cfg(not(windows))]
    {
        OsString::from(stem)
    }
}

fn validate_candidate(candidate: &EngineCandidate) -> Result<(), String> {
    let label = source_label(candidate.source);
    validate_engine_file(&candidate.ffmpeg, "ffmpeg")
        .map_err(|reason| format!("{label}: {reason}"))?;
    validate_engine_file(&candidate.ffprobe, "ffprobe")
        .map_err(|reason| format!("{label}: {reason}"))?;
    Ok(())
}

fn validate_engine_file(path: &Path, allowed_stem: &str) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| format!("{allowed_stem} candidate is unavailable"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err(format!("{allowed_stem} candidate is not a regular file"));
    }
    let file_stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if file_stem != allowed_stem && !file_stem.starts_with(&format!("{allowed_stem}-")) {
        return Err(format!(
            "Refused an unexpected executable name for {allowed_stem}"
        ));
    }
    Ok(())
}

async fn probe_candidate(candidate: EngineCandidate) -> Result<EngineRuntime, ConversionError> {
    let probe = probe_pair(&candidate.ffmpeg, &candidate.ffprobe)
        .await
        .map_err(|error| {
            ConversionError::new(
                ConversionErrorCode::EngineFailed,
                "Media engine capability check failed",
                "The detected FFmpeg build could not report its available codecs and filters.",
            )
            .with_details(format!(
                "{} rejected: {}",
                source_label(candidate.source),
                error
            ))
        })?;
    let diagnostic = format!(
        "{}; matching tool versions and runtime capabilities verified locally",
        source_label(candidate.source)
    );
    Ok(runtime_from_probe(
        candidate.ffmpeg,
        candidate.ffprobe,
        candidate.source,
        "FFmpeg".to_owned(),
        probe,
        diagnostic,
    ))
}

fn runtime_from_probe(
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
    source: EngineSource,
    name: String,
    probe: EngineProbe,
    diagnostic: String,
) -> EngineRuntime {
    let encoders = probe.encoders.iter().cloned().collect::<HashSet<_>>();
    let muxers = probe.muxers.iter().cloned().collect::<HashSet<_>>();
    let filters = probe.filters.iter().cloned().collect::<HashSet<_>>();
    EngineRuntime {
        media: Some(MediaTools {
            ffmpeg,
            ffprobe,
            encoders: encoders.clone(),
        }),
        registry: CapabilityRegistry {
            engine: EngineInfo {
                available: true,
                name,
                version: Some(probe.version),
                source,
                diagnostic: Some(diagnostic),
            },
            outputs: derive_outputs(&encoders, &muxers, &filters),
        },
    }
}

fn source_label(source: EngineSource) -> &'static str {
    match source {
        EngineSource::Bundled => "Reviewed bundled engine",
        EngineSource::Project => "Development engine",
        EngineSource::Selected => "Session engine",
        EngineSource::System => "System engine",
        EngineSource::Native => "Built-in image engine",
        EngineSource::Missing => "Unavailable",
    }
}

#[cfg(windows)]
fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_: &std::fs::Metadata) -> bool {
    false
}

fn missing_registry(diagnostic: String) -> CapabilityRegistry {
    CapabilityRegistry {
        engine: EngineInfo {
            available: false,
            name: "FFmpeg".to_owned(),
            version: None,
            source: EngineSource::Missing,
            diagnostic: Some(diagnostic),
        },
        outputs: OutputFormat::ALL
            .into_iter()
            .map(|format| FormatCapability {
                format,
                available: false,
                reason: Some("A compatible local media engine was not found.".to_owned()),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn session_candidate_uses_only_the_fixed_ffmpeg_pair() {
        let directory = PathBuf::from("京都 session engine");
        let candidate = pair_in(directory.clone(), EngineSource::Selected);
        assert_eq!(candidate.source, EngineSource::Selected);
        assert_eq!(candidate.ffmpeg, directory.join(engine_filename("ffmpeg")));
        assert_eq!(
            candidate.ffprobe,
            directory.join(engine_filename("ffprobe"))
        );
    }

    #[tokio::test]
    async fn invalid_bundled_candidate_is_terminal_without_a_trust_pin() {
        let root = tempdir().expect("create invalid bundle");
        assert!(EMBEDDED_REVIEWED_MANIFEST_SHA256.is_none());
        let error = probe_reviewed_bundle(root.path())
            .await
            .expect_err("reject an unpinned bundled candidate without fallback");
        assert_eq!(error.code, ConversionErrorCode::EngineFailed);
        assert!(error.message.contains("corrected package"));
        assert!(
            error
                .technical_details
                .as_deref()
                .is_some_and(|details| details.contains("MANIFEST_TRUST_PIN_MISSING"))
        );
    }

    #[test]
    fn automatic_development_discovery_retains_system_candidates() {
        let candidate = EngineCandidate {
            ffmpeg: PathBuf::from(engine_filename("ffmpeg")),
            ffprobe: PathBuf::from(engine_filename("ffprobe")),
            source: EngineSource::System,
        };
        assert_eq!(candidate.source, EngineSource::System);
        assert_eq!(source_label(candidate.source), "System engine");
    }

    #[test]
    fn derives_only_evidenced_capabilities() {
        let encoders = ["png", "mjpeg", "libwebp", "gif"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let muxers = ["image2", "gif"].into_iter().map(str::to_owned).collect();
        let filters = ["palettegen", "paletteuse", "scale", "fps"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let outputs = derive_outputs(&encoders, &muxers, &filters);

        assert!(
            outputs
                .iter()
                .any(|item| item.format == OutputFormat::Png && item.available)
        );
        assert!(
            outputs
                .iter()
                .any(|item| item.format == OutputFormat::Gif && item.available)
        );
        assert!(
            outputs
                .iter()
                .any(|item| item.format == OutputFormat::Mp4 && !item.available)
        );
    }
}
