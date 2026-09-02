use std::{
    collections::HashSet,
    fs::{self, File, Metadata},
    io::Read,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

use crate::{
    domain::OutputFormat,
    engine_probe::{EngineProbe, probe_pair},
};

pub const MANIFEST_FILENAME: &str = "morflo-engine-bundle.json";
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_BUNDLE_FILES: usize = 256;
const MAX_BUNDLE_DEPTH: usize = 8;
const MAX_REVIEW_DAYS: i64 = 366;

pub const EMBEDDED_REVIEWED_MANIFEST_SHA256: Option<&str> =
    option_env!("MORFLO_REVIEWED_ENGINE_MANIFEST_SHA256");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineBundleManifest {
    pub schema_version: u32,
    pub engine: ManifestEngine,
    pub target: ManifestTarget,
    pub provenance: ManifestProvenance,
    pub build_configuration: String,
    pub declared_license: String,
    pub executables: ManifestExecutables,
    #[serde(default)]
    pub supporting_files: Vec<SupportingFile>,
    pub reviewed_capabilities: ReviewedCapabilities,
    pub notice_references: Vec<EvidenceFile>,
    pub source_offer_references: Vec<EvidenceFile>,
    pub review: ManifestReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestEngine {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestTarget {
    pub platform: TargetPlatform,
    pub architecture: TargetArchitecture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetPlatform {
    Windows,
    Macos,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetArchitecture {
    X86_64,
    Aarch64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestProvenance {
    pub source_url_or_identifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestExecutables {
    pub ffmpeg: ExecutableFile,
    pub ffprobe: ExecutableFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutableFile {
    pub filename: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SupportingFile {
    pub path: String,
    pub role: SupportingFileRole,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportingFileRole {
    RuntimeLibrary,
    Data,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceFile {
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewedCapabilities {
    pub decoders: Vec<String>,
    pub encoders: Vec<String>,
    pub demuxers: Vec<String>,
    pub muxers: Vec<String>,
    pub filters: Vec<String>,
    pub outputs: Vec<OutputFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestReview {
    pub status: ReviewStatus,
    pub reviewed_by: String,
    pub reviewed_at: String,
    pub valid_until: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug)]
pub struct VerifiedBundle {
    pub manifest: EngineBundleManifest,
    pub manifest_sha256: String,
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub probe: EngineProbe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleVerificationError {
    code: &'static str,
    detail: String,
}

impl BundleVerificationError {
    fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: bounded(&detail.into(), 360),
        }
    }
}

impl std::fmt::Display for BundleVerificationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for BundleVerificationError {}

#[derive(Debug)]
struct PreparedBundle {
    manifest: EngineBundleManifest,
    manifest_sha256: String,
    root: PathBuf,
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
}

pub async fn verify_bundle(
    directory: &Path,
    expected_manifest_sha256: Option<&str>,
) -> Result<VerifiedBundle, BundleVerificationError> {
    let today = current_utc_day()?;
    let prepared = prepare_bundle(directory, expected_manifest_sha256, today)?;
    let probe = probe_pair(&prepared.ffmpeg, &prepared.ffprobe)
        .await
        .map_err(|error| BundleVerificationError::new("ENGINE_PROBE_FAILED", error.to_string()))?;
    compare_probe(&prepared.manifest, &probe)?;

    // Recheck every byte after process probing so a mid-verification replacement
    // cannot become the accepted runtime pair.
    verify_tracked_files(&prepared.root, &prepared.manifest)?;

    Ok(VerifiedBundle {
        manifest: prepared.manifest,
        manifest_sha256: prepared.manifest_sha256,
        ffmpeg: prepared.ffmpeg,
        ffprobe: prepared.ffprobe,
        probe,
    })
}

fn prepare_bundle(
    directory: &Path,
    expected_manifest_sha256: Option<&str>,
    today: i64,
) -> Result<PreparedBundle, BundleVerificationError> {
    validate_bundle_root(directory)?;
    let root = fs::canonicalize(directory).map_err(|error| {
        BundleVerificationError::new("BUNDLE_UNAVAILABLE", sanitized_io(&error))
    })?;
    let manifest_path = root.join(MANIFEST_FILENAME);
    validate_regular_file(&manifest_path, MANIFEST_FILENAME)?;
    let metadata = fs::metadata(&manifest_path).map_err(|error| {
        BundleVerificationError::new("MANIFEST_UNAVAILABLE", sanitized_io(&error))
    })?;
    if metadata.len() == 0 || metadata.len() > MAX_MANIFEST_BYTES {
        return Err(BundleVerificationError::new(
            "MANIFEST_SIZE_INVALID",
            "the manifest must be between 1 byte and 1 MiB",
        ));
    }
    let manifest_bytes = fs::read(&manifest_path).map_err(|error| {
        BundleVerificationError::new("MANIFEST_READ_FAILED", sanitized_io(&error))
    })?;
    let manifest_sha256 = sha256_bytes(&manifest_bytes);
    if let Some(expected) = expected_manifest_sha256 {
        validate_sha256(expected, "expected manifest digest")?;
        if manifest_sha256 != expected {
            return Err(BundleVerificationError::new(
                "MANIFEST_TRUST_PIN_MISMATCH",
                "the manifest does not match the separately reviewed digest",
            ));
        }
    }
    let manifest =
        serde_json::from_slice::<EngineBundleManifest>(&manifest_bytes).map_err(|error| {
            BundleVerificationError::new(
                "MANIFEST_SCHEMA_INVALID",
                format!(
                    "schema decoding failed at line {} column {}",
                    error.line(),
                    error.column()
                ),
            )
        })?;
    validate_manifest(&manifest, today)?;
    verify_tracked_files(&root, &manifest)?;

    let relative_files = tracked_relative_files(&manifest)?;
    verify_inventory(&root, &relative_files)?;
    let ffmpeg = root.join(&manifest.executables.ffmpeg.filename);
    let ffprobe = root.join(&manifest.executables.ffprobe.filename);
    Ok(PreparedBundle {
        manifest,
        manifest_sha256,
        root,
        ffmpeg,
        ffprobe,
    })
}

fn validate_manifest(
    manifest: &EngineBundleManifest,
    today: i64,
) -> Result<(), BundleVerificationError> {
    if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
        return Err(BundleVerificationError::new(
            "MANIFEST_SCHEMA_UNSUPPORTED",
            format!("schemaVersion must be {MANIFEST_SCHEMA_VERSION}"),
        ));
    }
    if manifest.engine.name != "FFmpeg" {
        return Err(BundleVerificationError::new(
            "ENGINE_NAME_INVALID",
            "engine.name must be FFmpeg",
        ));
    }
    validate_text(&manifest.engine.version, 128, "engine.version")?;
    validate_text(
        &manifest.provenance.source_url_or_identifier,
        2048,
        "provenance.sourceUrlOrIdentifier",
    )?;
    validate_text(&manifest.build_configuration, 65_536, "buildConfiguration")?;
    if !manifest.build_configuration.starts_with("--") {
        return Err(BundleVerificationError::new(
            "BUILD_CONFIGURATION_INVALID",
            "buildConfiguration must contain the exact configure argument line",
        ));
    }
    validate_text(&manifest.declared_license, 256, "declaredLicense")?;
    validate_target(&manifest.target)?;
    validate_executable_name(&manifest.executables.ffmpeg.filename, "ffmpeg")?;
    validate_executable_name(&manifest.executables.ffprobe.filename, "ffprobe")?;
    validate_artifact_record(
        &manifest.executables.ffmpeg.filename,
        manifest.executables.ffmpeg.size_bytes,
        &manifest.executables.ffmpeg.sha256,
        "ffmpeg",
    )?;
    validate_artifact_record(
        &manifest.executables.ffprobe.filename,
        manifest.executables.ffprobe.size_bytes,
        &manifest.executables.ffprobe.sha256,
        "ffprobe",
    )?;

    if manifest.notice_references.is_empty() {
        return Err(BundleVerificationError::new(
            "NOTICE_EVIDENCE_MISSING",
            "at least one hashed notice reference is required",
        ));
    }
    if manifest.source_offer_references.is_empty() {
        return Err(BundleVerificationError::new(
            "SOURCE_OFFER_EVIDENCE_MISSING",
            "at least one hashed source-offer reference is required",
        ));
    }
    for (label, evidence) in manifest
        .notice_references
        .iter()
        .map(|item| ("notice", item))
        .chain(
            manifest
                .source_offer_references
                .iter()
                .map(|item| ("source offer", item)),
        )
    {
        validate_artifact_record(&evidence.path, evidence.size_bytes, &evidence.sha256, label)?;
    }
    for supporting in &manifest.supporting_files {
        validate_artifact_record(
            &supporting.path,
            supporting.size_bytes,
            &supporting.sha256,
            "supporting file",
        )?;
    }
    validate_capabilities(&manifest.reviewed_capabilities)?;
    validate_review(&manifest.review, today)
}

fn validate_target(target: &ManifestTarget) -> Result<(), BundleVerificationError> {
    let current_platform = match std::env::consts::OS {
        "windows" => TargetPlatform::Windows,
        "macos" => TargetPlatform::Macos,
        "linux" => TargetPlatform::Linux,
        _ => {
            return Err(BundleVerificationError::new(
                "HOST_TARGET_UNSUPPORTED",
                "this operating system cannot verify a reviewed sidecar",
            ));
        }
    };
    let current_architecture = match std::env::consts::ARCH {
        "x86_64" => TargetArchitecture::X86_64,
        "aarch64" => TargetArchitecture::Aarch64,
        _ => {
            return Err(BundleVerificationError::new(
                "HOST_ARCHITECTURE_UNSUPPORTED",
                "this architecture cannot verify a reviewed sidecar",
            ));
        }
    };
    if target.platform != current_platform || target.architecture != current_architecture {
        return Err(BundleVerificationError::new(
            "TARGET_MISMATCH",
            "the declared platform or architecture does not match this host",
        ));
    }
    Ok(())
}

fn validate_executable_name(
    filename: &str,
    expected_stem: &str,
) -> Result<(), BundleVerificationError> {
    let expected = if cfg!(windows) {
        format!("{expected_stem}.exe")
    } else {
        expected_stem.to_owned()
    };
    if !filename.eq_ignore_ascii_case(&expected) {
        return Err(BundleVerificationError::new(
            "EXECUTABLE_NAME_INVALID",
            format!("the reviewed {expected_stem} filename must be {expected}"),
        ));
    }
    validate_relative_path(filename).map(|_| ())
}

fn validate_artifact_record(
    path: &str,
    size_bytes: u64,
    sha256: &str,
    label: &str,
) -> Result<(), BundleVerificationError> {
    validate_relative_path(path)?;
    if size_bytes == 0 {
        return Err(BundleVerificationError::new(
            "ARTIFACT_SIZE_INVALID",
            format!("{label} sizeBytes must be greater than zero"),
        ));
    }
    validate_sha256(sha256, label)
}

fn validate_capabilities(
    capabilities: &ReviewedCapabilities,
) -> Result<(), BundleVerificationError> {
    for (label, values) in [
        ("decoders", &capabilities.decoders),
        ("encoders", &capabilities.encoders),
        ("demuxers", &capabilities.demuxers),
        ("muxers", &capabilities.muxers),
        ("filters", &capabilities.filters),
    ] {
        if values.is_empty() {
            return Err(BundleVerificationError::new(
                "CAPABILITY_EVIDENCE_INCOMPLETE",
                format!("reviewedCapabilities.{label} must not be empty"),
            ));
        }
        if values.len() > 4096
            || values
                .iter()
                .any(|value| value.is_empty() || value.len() > 128 || !is_safe_token(value))
        {
            return Err(BundleVerificationError::new(
                "CAPABILITY_EVIDENCE_INVALID",
                format!("reviewedCapabilities.{label} contains an invalid token"),
            ));
        }
        if values.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(BundleVerificationError::new(
                "CAPABILITY_EVIDENCE_NONCANONICAL",
                format!("reviewedCapabilities.{label} must be sorted and unique"),
            ));
        }
    }
    if capabilities.outputs.is_empty()
        || capabilities.outputs.iter().collect::<HashSet<_>>().len() != capabilities.outputs.len()
    {
        return Err(BundleVerificationError::new(
            "OUTPUT_EVIDENCE_INVALID",
            "reviewedCapabilities.outputs must be non-empty and unique",
        ));
    }
    let canonical_outputs = OutputFormat::ALL
        .into_iter()
        .filter(|format| capabilities.outputs.contains(format))
        .collect::<Vec<_>>();
    if capabilities.outputs != canonical_outputs {
        return Err(BundleVerificationError::new(
            "OUTPUT_EVIDENCE_NONCANONICAL",
            "reviewedCapabilities.outputs must use Morflo's canonical order",
        ));
    }
    Ok(())
}

fn validate_review(review: &ManifestReview, today: i64) -> Result<(), BundleVerificationError> {
    if review.status != ReviewStatus::Approved {
        return Err(BundleVerificationError::new(
            "REVIEW_NOT_APPROVED",
            "review.status must be approved",
        ));
    }
    validate_text(&review.reviewed_by, 256, "review.reviewedBy")?;
    let reviewed_at = parse_iso_date(&review.reviewed_at)?;
    let valid_until = parse_iso_date(&review.valid_until)?;
    if reviewed_at > today {
        return Err(BundleVerificationError::new(
            "REVIEW_DATE_INVALID",
            "review.reviewedAt cannot be in the future",
        ));
    }
    if valid_until < reviewed_at || valid_until - reviewed_at > MAX_REVIEW_DAYS {
        return Err(BundleVerificationError::new(
            "REVIEW_WINDOW_INVALID",
            "review.validUntil must be within 366 days of review.reviewedAt",
        ));
    }
    if today > valid_until {
        return Err(BundleVerificationError::new(
            "REVIEW_EXPIRED",
            "the reviewed bundle approval has expired",
        ));
    }
    Ok(())
}

fn compare_probe(
    manifest: &EngineBundleManifest,
    probe: &EngineProbe,
) -> Result<(), BundleVerificationError> {
    if manifest.engine.version != probe.version {
        return Err(BundleVerificationError::new(
            "ENGINE_VERSION_MISMATCH",
            "the runtime engine version does not match the manifest",
        ));
    }
    if manifest.build_configuration != probe.build_configuration {
        return Err(BundleVerificationError::new(
            "BUILD_CONFIGURATION_MISMATCH",
            "the runtime build configuration does not match the manifest",
        ));
    }
    let declared = &manifest.reviewed_capabilities;
    for (label, expected, actual) in [
        ("decoders", &declared.decoders, &probe.decoders),
        ("encoders", &declared.encoders, &probe.encoders),
        ("demuxers", &declared.demuxers, &probe.demuxers),
        ("muxers", &declared.muxers, &probe.muxers),
        ("filters", &declared.filters, &probe.filters),
    ] {
        if expected != actual {
            return Err(BundleVerificationError::new(
                "CAPABILITY_MISMATCH",
                format!("the runtime {label} inventory does not match the manifest"),
            ));
        }
    }
    if declared.outputs != probe.available_outputs() {
        return Err(BundleVerificationError::new(
            "OUTPUT_CAPABILITY_MISMATCH",
            "the derived Morflo output capabilities do not match the manifest",
        ));
    }
    Ok(())
}

fn verify_tracked_files(
    root: &Path,
    manifest: &EngineBundleManifest,
) -> Result<(), BundleVerificationError> {
    verify_file_record(
        root,
        &manifest.executables.ffmpeg.filename,
        manifest.executables.ffmpeg.size_bytes,
        &manifest.executables.ffmpeg.sha256,
        "ffmpeg",
    )?;
    verify_file_record(
        root,
        &manifest.executables.ffprobe.filename,
        manifest.executables.ffprobe.size_bytes,
        &manifest.executables.ffprobe.sha256,
        "ffprobe",
    )?;
    for evidence in &manifest.notice_references {
        verify_file_record(
            root,
            &evidence.path,
            evidence.size_bytes,
            &evidence.sha256,
            "notice",
        )?;
    }
    for evidence in &manifest.source_offer_references {
        verify_file_record(
            root,
            &evidence.path,
            evidence.size_bytes,
            &evidence.sha256,
            "source offer",
        )?;
    }
    for supporting in &manifest.supporting_files {
        verify_file_record(
            root,
            &supporting.path,
            supporting.size_bytes,
            &supporting.sha256,
            "supporting file",
        )?;
    }
    Ok(())
}

fn verify_file_record(
    root: &Path,
    relative: &str,
    expected_size: u64,
    expected_hash: &str,
    label: &str,
) -> Result<(), BundleVerificationError> {
    let relative_path = validate_relative_path(relative)?;
    let path = root.join(&relative_path);
    validate_regular_file(&path, label)?;
    validate_components_without_reparse(root, &relative_path)?;
    let canonical = fs::canonicalize(&path).map_err(|error| {
        BundleVerificationError::new("ARTIFACT_UNAVAILABLE", sanitized_io(&error))
    })?;
    if !canonical.starts_with(root) {
        return Err(BundleVerificationError::new(
            "ARTIFACT_ESCAPES_BUNDLE",
            format!("{label} resolves outside the reviewed directory"),
        ));
    }
    let metadata = fs::metadata(&canonical).map_err(|error| {
        BundleVerificationError::new("ARTIFACT_UNAVAILABLE", sanitized_io(&error))
    })?;
    if metadata.len() != expected_size {
        return Err(BundleVerificationError::new(
            "ARTIFACT_SIZE_MISMATCH",
            format!("{label} size does not match the manifest"),
        ));
    }
    let actual_hash = sha256_file(&canonical)?;
    if actual_hash != expected_hash {
        return Err(BundleVerificationError::new(
            "ARTIFACT_HASH_MISMATCH",
            format!("{label} SHA-256 does not match the manifest"),
        ));
    }
    Ok(())
}

fn tracked_relative_files(
    manifest: &EngineBundleManifest,
) -> Result<Vec<PathBuf>, BundleVerificationError> {
    let mut paths = vec![
        PathBuf::from(MANIFEST_FILENAME),
        validate_relative_path(&manifest.executables.ffmpeg.filename)?,
        validate_relative_path(&manifest.executables.ffprobe.filename)?,
    ];
    paths.extend(
        manifest
            .notice_references
            .iter()
            .map(|item| validate_relative_path(&item.path))
            .collect::<Result<Vec<_>, _>>()?,
    );
    paths.extend(
        manifest
            .source_offer_references
            .iter()
            .map(|item| validate_relative_path(&item.path))
            .collect::<Result<Vec<_>, _>>()?,
    );
    paths.extend(
        manifest
            .supporting_files
            .iter()
            .map(|item| validate_relative_path(&item.path))
            .collect::<Result<Vec<_>, _>>()?,
    );

    let mut identities = HashSet::new();
    for path in &paths {
        let identity = path_identity(path);
        if !identities.insert(identity) {
            return Err(BundleVerificationError::new(
                "DUPLICATE_ARTIFACT_PATH",
                "each manifest artifact path must be unique",
            ));
        }
    }
    Ok(paths)
}

fn verify_inventory(root: &Path, tracked_files: &[PathBuf]) -> Result<(), BundleVerificationError> {
    let expected_files = tracked_files
        .iter()
        .map(|path| path_identity(path))
        .collect::<HashSet<_>>();
    let mut expected_directories = HashSet::new();
    for path in tracked_files {
        let mut parent = path.parent();
        while let Some(directory) = parent {
            if directory.as_os_str().is_empty() {
                break;
            }
            expected_directories.insert(path_identity(directory));
            parent = directory.parent();
        }
    }
    let mut count = 0usize;
    inspect_directory(
        root,
        Path::new(""),
        &expected_files,
        &expected_directories,
        &mut count,
        0,
    )
}

fn inspect_directory(
    root: &Path,
    relative: &Path,
    expected_files: &HashSet<String>,
    expected_directories: &HashSet<String>,
    count: &mut usize,
    depth: usize,
) -> Result<(), BundleVerificationError> {
    if depth > MAX_BUNDLE_DEPTH {
        return Err(BundleVerificationError::new(
            "BUNDLE_TOO_DEEP",
            "the reviewed directory exceeds the nesting limit",
        ));
    }
    let entries = fs::read_dir(root.join(relative)).map_err(|error| {
        BundleVerificationError::new("BUNDLE_READ_FAILED", sanitized_io(&error))
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            BundleVerificationError::new("BUNDLE_READ_FAILED", sanitized_io(&error))
        })?;
        *count += 1;
        if *count > MAX_BUNDLE_FILES {
            return Err(BundleVerificationError::new(
                "BUNDLE_FILE_LIMIT_EXCEEDED",
                "the reviewed directory contains too many entries",
            ));
        }
        let child_relative = relative.join(entry.file_name());
        let identity = path_identity(&child_relative);
        let link_metadata = fs::symlink_metadata(entry.path()).map_err(|error| {
            BundleVerificationError::new("BUNDLE_READ_FAILED", sanitized_io(&error))
        })?;
        if link_metadata.file_type().is_symlink() || is_reparse_point(&link_metadata) {
            return Err(BundleVerificationError::new(
                "REPARSE_POINT_REJECTED",
                format!(
                    "{} is a symlink or reparse point",
                    display_relative(&child_relative)
                ),
            ));
        }
        if link_metadata.is_dir() {
            if !expected_directories.contains(&identity) {
                return Err(BundleVerificationError::new(
                    "UNTRACKED_BUNDLE_ENTRY",
                    format!(
                        "{} is not declared by the manifest",
                        display_relative(&child_relative)
                    ),
                ));
            }
            inspect_directory(
                root,
                &child_relative,
                expected_files,
                expected_directories,
                count,
                depth + 1,
            )?;
        } else if !link_metadata.is_file() || !expected_files.contains(&identity) {
            return Err(BundleVerificationError::new(
                "UNTRACKED_BUNDLE_ENTRY",
                format!(
                    "{} is not declared by the manifest",
                    display_relative(&child_relative)
                ),
            ));
        }
    }
    Ok(())
}

fn validate_bundle_root(directory: &Path) -> Result<(), BundleVerificationError> {
    let metadata = fs::symlink_metadata(directory).map_err(|error| {
        BundleVerificationError::new("BUNDLE_UNAVAILABLE", sanitized_io(&error))
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err(BundleVerificationError::new(
            "BUNDLE_ROOT_REJECTED",
            "the explicit bundle root must be a regular directory, not a symlink or reparse point",
        ));
    }
    Ok(())
}

fn validate_regular_file(path: &Path, label: &str) -> Result<(), BundleVerificationError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| {
        BundleVerificationError::new("ARTIFACT_UNAVAILABLE", format!("{label} is missing"))
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err(BundleVerificationError::new(
            "ARTIFACT_TYPE_REJECTED",
            format!("{label} must be a regular non-reparse file"),
        ));
    }
    Ok(())
}

fn validate_components_without_reparse(
    root: &Path,
    relative: &Path,
) -> Result<(), BundleVerificationError> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&current).map_err(|error| {
            BundleVerificationError::new("ARTIFACT_UNAVAILABLE", sanitized_io(&error))
        })?;
        if metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
            return Err(BundleVerificationError::new(
                "REPARSE_POINT_REJECTED",
                "an artifact path crosses a symlink or reparse point",
            ));
        }
    }
    Ok(())
}

fn validate_relative_path(value: &str) -> Result<PathBuf, BundleVerificationError> {
    if value.is_empty() || value.len() > 512 || value.contains('\\') || value.contains('\0') {
        return Err(BundleVerificationError::new(
            "ARTIFACT_PATH_INVALID",
            "artifact paths must be bounded slash-separated relative paths",
        ));
    }
    let path = PathBuf::from(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(BundleVerificationError::new(
            "ARTIFACT_PATH_INVALID",
            "artifact paths cannot be absolute or contain traversal components",
        ));
    }
    Ok(path)
}

fn validate_sha256(value: &str, label: &str) -> Result<(), BundleVerificationError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(BundleVerificationError::new(
            "SHA256_INVALID",
            format!("{label} SHA-256 must be 64 lowercase hexadecimal characters"),
        ));
    }
    Ok(())
}

fn validate_text(
    value: &str,
    max_bytes: usize,
    label: &str,
) -> Result<(), BundleVerificationError> {
    if value.trim() != value
        || value.is_empty()
        || value.len() > max_bytes
        || value
            .chars()
            .any(|character| character.is_control() && character != '\t')
    {
        return Err(BundleVerificationError::new(
            "MANIFEST_TEXT_INVALID",
            format!("{label} is missing, unbounded, or contains control characters"),
        ));
    }
    Ok(())
}

fn is_safe_token(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b','))
}

fn sha256_file(path: &Path) -> Result<String, BundleVerificationError> {
    let mut file = File::open(path).map_err(|error| {
        BundleVerificationError::new("ARTIFACT_READ_FAILED", sanitized_io(&error))
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            BundleVerificationError::new("ARTIFACT_READ_FAILED", sanitized_io(&error))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn parse_iso_date(value: &str) -> Result<i64, BundleVerificationError> {
    if value.len() != 10 || value.as_bytes()[4] != b'-' || value.as_bytes()[7] != b'-' {
        return Err(BundleVerificationError::new(
            "REVIEW_DATE_INVALID",
            "review dates must use YYYY-MM-DD",
        ));
    }
    let year = value[0..4].parse::<i64>().ok();
    let month = value[5..7].parse::<u32>().ok();
    let day = value[8..10].parse::<u32>().ok();
    let (Some(year), Some(month), Some(day)) = (year, month, day) else {
        return Err(BundleVerificationError::new(
            "REVIEW_DATE_INVALID",
            "review dates must use YYYY-MM-DD",
        ));
    };
    if !(2000..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
    {
        return Err(BundleVerificationError::new(
            "REVIEW_DATE_INVALID",
            "review date is not a valid calendar day",
        ));
    }
    Ok(days_from_civil(year, month, day))
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

fn days_from_civil(mut year: i64, month: u32, day: u32) -> i64 {
    year -= i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let shifted_month = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn current_utc_day() -> Result<i64, BundleVerificationError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| {
            BundleVerificationError::new(
                "SYSTEM_TIME_INVALID",
                "the system clock predates the Unix epoch",
            )
        })?
        .as_secs();
    Ok((seconds / 86_400) as i64)
}

fn path_identity(path: &Path) -> String {
    let normalized = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");
    if cfg!(windows) {
        normalized.to_ascii_lowercase()
    } else {
        normalized
    }
}

fn display_relative(path: &Path) -> String {
    bounded(&path_identity(path), 180)
}

#[cfg(windows)]
fn is_reparse_point(metadata: &Metadata) -> bool {
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_: &Metadata) -> bool {
    false
}

fn sanitized_io(error: &std::io::Error) -> String {
    match error.raw_os_error() {
        Some(code) => format!("filesystem operation failed (OS error {code})"),
        None => format!("filesystem operation failed ({:?})", error.kind()),
    }
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
    use tempfile::tempdir;

    const TEST_DAY: i64 = 20_331;

    fn test_probe() -> EngineProbe {
        let mut probe = EngineProbe {
            version: "8.1.2-test".to_owned(),
            build_configuration: "--disable-network --enable-small".to_owned(),
            decoders: vec!["png".to_owned()],
            encoders: vec!["mjpeg".to_owned(), "png".to_owned()],
            demuxers: vec!["image2".to_owned()],
            muxers: vec!["image2".to_owned()],
            filters: vec!["scale".to_owned()],
        };
        probe.decoders.sort();
        probe.encoders.sort();
        probe.demuxers.sort();
        probe.muxers.sort();
        probe.filters.sort();
        probe
    }

    fn executable_name(stem: &str) -> String {
        if cfg!(windows) {
            format!("{stem}.exe")
        } else {
            stem.to_owned()
        }
    }

    fn write_file(root: &Path, relative: &str, bytes: &[u8]) -> EvidenceFile {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("file parent")).expect("create fixture parent");
        fs::write(&path, bytes).expect("write fixture file");
        EvidenceFile {
            path: relative.to_owned(),
            size_bytes: bytes.len() as u64,
            sha256: sha256_bytes(bytes),
        }
    }

    fn fixture_manifest(root: &Path, probe: &EngineProbe) -> EngineBundleManifest {
        let ffmpeg = write_file(
            root,
            &executable_name("ffmpeg"),
            b"synthetic ffmpeg adapter",
        );
        let ffprobe = write_file(
            root,
            &executable_name("ffprobe"),
            b"synthetic ffprobe adapter",
        );
        let notice = write_file(root, "notices/NOTICE.txt", b"synthetic notice evidence");
        let source = write_file(
            root,
            "source/SOURCE_OFFER.txt",
            b"synthetic source-offer evidence",
        );
        EngineBundleManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            engine: ManifestEngine {
                name: "FFmpeg".to_owned(),
                version: probe.version.clone(),
            },
            target: ManifestTarget {
                platform: match std::env::consts::OS {
                    "windows" => TargetPlatform::Windows,
                    "macos" => TargetPlatform::Macos,
                    _ => TargetPlatform::Linux,
                },
                architecture: match std::env::consts::ARCH {
                    "aarch64" => TargetArchitecture::Aarch64,
                    _ => TargetArchitecture::X86_64,
                },
            },
            provenance: ManifestProvenance {
                source_url_or_identifier: "synthetic:test-adapter:v1".to_owned(),
            },
            build_configuration: probe.build_configuration.clone(),
            declared_license: "LicenseRef-Synthetic-Test-Only".to_owned(),
            executables: ManifestExecutables {
                ffmpeg: ExecutableFile {
                    filename: ffmpeg.path,
                    size_bytes: ffmpeg.size_bytes,
                    sha256: ffmpeg.sha256,
                },
                ffprobe: ExecutableFile {
                    filename: ffprobe.path,
                    size_bytes: ffprobe.size_bytes,
                    sha256: ffprobe.sha256,
                },
            },
            supporting_files: Vec::new(),
            reviewed_capabilities: ReviewedCapabilities {
                decoders: probe.decoders.clone(),
                encoders: probe.encoders.clone(),
                demuxers: probe.demuxers.clone(),
                muxers: probe.muxers.clone(),
                filters: probe.filters.clone(),
                outputs: probe.available_outputs(),
            },
            notice_references: vec![notice],
            source_offer_references: vec![source],
            review: ManifestReview {
                status: ReviewStatus::Approved,
                reviewed_by: "Morflo synthetic test adapter".to_owned(),
                reviewed_at: "2025-08-01".to_owned(),
                valid_until: "2026-08-01".to_owned(),
            },
        }
    }

    fn write_manifest(root: &Path, manifest: &EngineBundleManifest) -> String {
        let bytes = serde_json::to_vec_pretty(manifest).expect("serialize manifest");
        fs::write(root.join(MANIFEST_FILENAME), &bytes).expect("write manifest");
        sha256_bytes(&bytes)
    }

    fn verify_with_probe_at(
        root: &Path,
        expected: Option<&str>,
        probe: EngineProbe,
        today: i64,
    ) -> Result<VerifiedBundle, BundleVerificationError> {
        let prepared = prepare_bundle(root, expected, today)?;
        compare_probe(&prepared.manifest, &probe)?;
        Ok(VerifiedBundle {
            manifest: prepared.manifest,
            manifest_sha256: prepared.manifest_sha256,
            ffmpeg: prepared.ffmpeg,
            ffprobe: prepared.ffprobe,
            probe,
        })
    }

    #[test]
    fn valid_synthetic_adapter_passes_with_independent_manifest_pin() {
        let root = tempdir().expect("create bundle");
        let probe = test_probe();
        let manifest = fixture_manifest(root.path(), &probe);
        let digest = write_manifest(root.path(), &manifest);

        let verified = verify_with_probe_at(root.path(), Some(&digest), probe, TEST_DAY)
            .expect("verify synthetic bundle");
        assert_eq!(verified.manifest.engine.version, "8.1.2-test");
        assert_eq!(verified.manifest_sha256, digest);
    }

    #[test]
    fn manifest_schema_rejects_unknown_fields() {
        let root = tempdir().expect("create bundle");
        let probe = test_probe();
        let manifest = fixture_manifest(root.path(), &probe);
        let mut value = serde_json::to_value(manifest).expect("serialize manifest");
        value
            .as_object_mut()
            .expect("manifest object")
            .insert("inventedApproval".to_owned(), serde_json::Value::Bool(true));
        fs::write(
            root.path().join(MANIFEST_FILENAME),
            serde_json::to_vec_pretty(&value).expect("serialize altered manifest"),
        )
        .expect("write altered manifest");

        let error = prepare_bundle(root.path(), None, TEST_DAY).expect_err("reject unknown field");
        assert_eq!(error.code, "MANIFEST_SCHEMA_INVALID");
    }

    #[test]
    fn published_json_schema_is_versioned_and_well_formed() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../engine-bundle.schema.json"))
                .expect("parse published engine bundle schema");
        assert_eq!(schema["properties"]["schemaVersion"]["const"], 1);
        assert_eq!(schema["additionalProperties"], false);
    }

    #[test]
    fn hash_mismatch_is_rejected() {
        let root = tempdir().expect("create bundle");
        let probe = test_probe();
        let manifest = fixture_manifest(root.path(), &probe);
        write_manifest(root.path(), &manifest);
        fs::write(
            root.path().join(executable_name("ffmpeg")),
            b"substituted executable bytes",
        )
        .expect("replace fixture executable");

        let error = prepare_bundle(root.path(), None, TEST_DAY).expect_err("reject mismatch");
        assert!(matches!(
            error.code,
            "ARTIFACT_SIZE_MISMATCH" | "ARTIFACT_HASH_MISMATCH"
        ));
    }

    #[test]
    fn version_mismatch_is_rejected() {
        let root = tempdir().expect("create bundle");
        let manifest_probe = test_probe();
        let manifest = fixture_manifest(root.path(), &manifest_probe);
        write_manifest(root.path(), &manifest);
        let mut actual_probe = manifest_probe;
        actual_probe.version = "8.1.3-test".to_owned();

        let error = verify_with_probe_at(root.path(), None, actual_probe, TEST_DAY)
            .expect_err("reject version mismatch");
        assert_eq!(error.code, "ENGINE_VERSION_MISMATCH");
    }

    #[test]
    fn capability_mismatch_is_rejected() {
        let root = tempdir().expect("create bundle");
        let manifest_probe = test_probe();
        let manifest = fixture_manifest(root.path(), &manifest_probe);
        write_manifest(root.path(), &manifest);
        let mut actual_probe = manifest_probe;
        actual_probe.encoders.push("webp".to_owned());
        actual_probe.encoders.sort();

        let error = verify_with_probe_at(root.path(), None, actual_probe, TEST_DAY)
            .expect_err("reject capability mismatch");
        assert_eq!(error.code, "CAPABILITY_MISMATCH");
    }

    #[test]
    fn missing_notice_evidence_is_rejected() {
        let root = tempdir().expect("create bundle");
        let probe = test_probe();
        let mut manifest = fixture_manifest(root.path(), &probe);
        manifest.notice_references.clear();
        write_manifest(root.path(), &manifest);

        let error = prepare_bundle(root.path(), None, TEST_DAY).expect_err("reject missing notice");
        assert_eq!(error.code, "NOTICE_EVIDENCE_MISSING");
    }

    #[test]
    fn unicode_bundle_and_evidence_paths_are_supported() {
        let parent = tempdir().expect("create parent");
        let root = parent.path().join("審核済み 京都 🧳");
        fs::create_dir(&root).expect("create Unicode bundle");
        let probe = test_probe();
        let mut manifest = fixture_manifest(&root, &probe);
        fs::remove_file(root.join("notices/NOTICE.txt")).expect("remove ASCII notice fixture");
        let unicode_notice = write_file(&root, "notices/京都 注意事項.txt", b"Unicode notice");
        manifest.notice_references = vec![unicode_notice];
        write_manifest(&root, &manifest);

        verify_with_probe_at(&root, None, probe, TEST_DAY).expect("verify Unicode bundle");
    }

    #[test]
    fn traversal_and_expired_review_are_rejected() {
        let root = tempdir().expect("create bundle");
        let probe = test_probe();
        let mut manifest = fixture_manifest(root.path(), &probe);
        manifest.notice_references[0].path = "../NOTICE.txt".to_owned();
        write_manifest(root.path(), &manifest);
        let traversal = prepare_bundle(root.path(), None, TEST_DAY).expect_err("reject traversal");
        assert_eq!(traversal.code, "ARTIFACT_PATH_INVALID");

        manifest.notice_references[0].path = "notices/NOTICE.txt".to_owned();
        manifest.review.valid_until = "2025-08-02".to_owned();
        write_manifest(root.path(), &manifest);
        let expired = prepare_bundle(root.path(), None, TEST_DAY).expect_err("reject stale review");
        assert_eq!(expired.code, "REVIEW_EXPIRED");
    }

    #[test]
    fn untracked_files_are_rejected() {
        let root = tempdir().expect("create bundle");
        let probe = test_probe();
        let manifest = fixture_manifest(root.path(), &probe);
        write_manifest(root.path(), &manifest);
        fs::write(root.path().join("unexpected.dll"), b"unreviewed").expect("write extra file");

        let error = prepare_bundle(root.path(), None, TEST_DAY).expect_err("reject extra file");
        assert_eq!(error.code, "UNTRACKED_BUNDLE_ENTRY");
    }

    #[test]
    fn iso_date_parser_rejects_impossible_days() {
        assert!(parse_iso_date("2026-02-29").is_err());
        assert!(parse_iso_date("2024-02-29").is_ok());
        assert_eq!(
            parse_iso_date("1970-01-01").unwrap_err().code,
            "REVIEW_DATE_INVALID"
        );
    }
}
