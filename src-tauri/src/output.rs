use std::{
    collections::HashSet,
    fs::OpenOptions,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{
    CollisionPolicy, ConversionError, ConversionErrorCode, DestinationMode, OutputFormat,
    StartupReport,
};

const RECOVERY_DIRECTORY: &str = "morflo-recovery-v1";
static RECOVERY_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Serialize, Deserialize)]
struct RecoveryEntry {
    version: u8,
    path: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct OutputReservations {
    paths: Arc<Mutex<HashSet<PathBuf>>>,
}

#[derive(Debug)]
pub struct ReservedOutput {
    pub final_path: PathBuf,
    pub partial_path: PathBuf,
    pub replace_existing: bool,
    owner: OutputReservations,
}

impl Drop for ReservedOutput {
    fn drop(&mut self) {
        if let Ok(mut paths) = self.owner.paths.lock() {
            paths.remove(&normalized_key(&self.final_path));
        }
    }
}

impl OutputReservations {
    pub fn reserve(
        &self,
        source: &Path,
        format: OutputFormat,
        destination: DestinationMode,
        destination_path: Option<&str>,
        policy: CollisionPolicy,
    ) -> Result<ReservedOutput, ConversionError> {
        let directory = output_directory(source, destination, destination_path)?;
        validate_destination(&directory)?;
        let stem = source
            .file_stem()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("converted");
        let extension = format.extension();

        let mut reserved = self.paths.lock().map_err(|_| {
            ConversionError::new(
                ConversionErrorCode::EngineFailed,
                "Output could not be reserved",
                "The local job coordinator is unavailable.",
            )
        })?;
        let (final_path, replace_existing) = match policy {
            CollisionPolicy::Suffix => {
                let path = first_available_path(&directory, stem, extension, source, &reserved)?;
                (path, false)
            }
            CollisionPolicy::Skip => {
                let path = directory.join(format!("{stem}.{extension}"));
                if conflicts(&path, source, &reserved) {
                    return Err(output_exists_error());
                }
                (path, false)
            }
            CollisionPolicy::Replace => {
                let path = directory.join(format!("{stem}.{extension}"));
                if same_path(&path, source) {
                    return Err(ConversionError::invalid(
                        "The source file can never be used as the output target. Choose a different format or collision policy.",
                    ));
                }
                if reserved.contains(&normalized_key(&path)) {
                    return Err(output_exists_error());
                }
                (path, true)
            }
        };

        reserved.insert(normalized_key(&final_path));
        drop(reserved);
        let partial_path = create_owned_partial(&directory, stem, extension)?;

        Ok(ReservedOutput {
            final_path,
            partial_path,
            replace_existing,
            owner: self.clone(),
        })
    }
}

impl ReservedOutput {
    pub fn validate_free_space(&self, required_bytes: u64) -> Result<(), ConversionError> {
        let Some(directory) = self.partial_path.parent() else {
            return Err(ConversionError::invalid(
                "The temporary output has no usable parent folder.",
            ));
        };
        if let Ok(available_bytes) = available_space(directory)
            && available_bytes < required_bytes
        {
            return Err(ConversionError::new(
                ConversionErrorCode::InsufficientSpace,
                "Not enough free space",
                "Free some space or choose another output folder, then try again.",
            )
            .with_details(format!(
                "The destination reported {available_bytes} bytes free; this conversion needs a safety allowance of at least {required_bytes} bytes."
            )));
        }
        Ok(())
    }

    pub fn cleanup_partial(&self) {
        if is_recognized_partial(&self.partial_path) {
            remove_owned_temporary(&self.partial_path);
        }
    }

    pub fn create_temporary(
        &self,
        role: &str,
        extension: &str,
    ) -> Result<PathBuf, ConversionError> {
        if role != "palette"
            || extension.is_empty()
            || !extension
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        {
            return Err(ConversionError::invalid(
                "Morflo refused an unexpected temporary-file request.",
            ));
        }
        let directory = self.partial_path.parent().ok_or_else(|| {
            ConversionError::invalid("The temporary output has no usable parent folder.")
        })?;
        let stem = self
            .final_path
            .file_stem()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("converted");
        create_owned_auxiliary(directory, stem, role, extension)
    }

    pub fn cleanup_temporary(path: &Path) {
        if is_recognized_auxiliary(path) {
            remove_owned_temporary(path);
        }
    }

    pub fn finalize(&self) -> Result<(), ConversionError> {
        let metadata = std::fs::metadata(&self.partial_path).map_err(|error| {
            ConversionError::new(
                ConversionErrorCode::EngineFailed,
                "Converted output is missing",
                "The media engine finished without producing a complete temporary file.",
            )
            .with_details(error.to_string())
        })?;
        if !metadata.is_file() || metadata.len() == 0 {
            return Err(ConversionError::new(
                ConversionErrorCode::EngineFailed,
                "Converted output is incomplete",
                "Morflo did not publish an empty or invalid temporary file.",
            ));
        }

        let finalized = if self.replace_existing {
            atomic_replace(&self.partial_path, &self.final_path)
        } else {
            atomic_publish_new(&self.partial_path, &self.final_path)
        };
        if finalized.is_ok() {
            unregister_recovery(&self.partial_path);
        }
        finalized.map_err(map_finalize_error)
    }

    pub fn display_name(&self) -> String {
        self.final_path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Converted file".to_owned())
    }
}

#[cfg(unix)]
fn available_space(directory: &Path) -> std::io::Result<u64> {
    use std::{ffi::CString, mem::MaybeUninit, os::unix::ffi::OsStrExt as _};

    let path = CString::new(directory.as_os_str().as_bytes()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "output path contains an embedded NUL",
        )
    })?;
    let mut stats = MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `path` is NUL-terminated and `stats` points to writable, correctly sized memory.
    if unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: statvfs returned success and initialized the output structure.
    let stats = unsafe { stats.assume_init() };
    Ok(stats.f_bavail.saturating_mul(stats.f_frsize))
}

#[cfg(windows)]
fn available_space(directory: &Path) -> std::io::Result<u64> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let path = directory
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut available_bytes = 0_u64;
    // SAFETY: `path` is a live NUL-terminated UTF-16 buffer and the output pointer is valid.
    if unsafe {
        GetDiskFreeSpaceExW(
            path.as_ptr(),
            &mut available_bytes,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } == 0
    {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(available_bytes)
    }
}

fn output_directory(
    source: &Path,
    destination: DestinationMode,
    destination_path: Option<&str>,
) -> Result<PathBuf, ConversionError> {
    match destination {
        DestinationMode::Same => source.parent().map(Path::to_path_buf).ok_or_else(|| {
            ConversionError::invalid("The source file does not have a usable parent folder.")
        }),
        DestinationMode::Custom => {
            let raw = destination_path
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| ConversionError::invalid("Choose an output folder."))?;
            let path = PathBuf::from(raw);
            if !path.is_absolute() {
                return Err(ConversionError::invalid(
                    "The output folder must be an absolute path.",
                ));
            }
            Ok(path)
        }
    }
}

fn validate_destination(directory: &Path) -> Result<(), ConversionError> {
    let metadata = std::fs::symlink_metadata(directory).map_err(|error| {
        ConversionError::new(
            ConversionErrorCode::PermissionDenied,
            "The output folder is unavailable",
            "Choose an existing folder that Morflo can write to.",
        )
        .with_details(error.to_string())
    })?;
    if metadata.file_type().is_symlink() {
        return Err(ConversionError::new(
            ConversionErrorCode::InvalidRequest,
            "Linked output folders are not accepted",
            "Choose the destination folder directly.",
        ));
    }
    if !metadata.is_dir() {
        return Err(ConversionError::invalid(
            "The selected output destination is not a folder.",
        ));
    }
    probe_writable(directory)
}

/// Decide whether a destination folder accepts new files by creating one.
///
/// The stored read-only flag cannot answer this. Windows sets
/// `FILE_ATTRIBUTE_READONLY` on customized shell folders such as `Downloads`,
/// `Pictures` and `Videos`, where it marks a `desktop.ini` customization and
/// never blocks writing, while a folder genuinely closed by an access control
/// entry carries no such flag at all. Unix mode bits are equally indirect once
/// group or ACL entries are involved. A single create-and-remove probe answers
/// the question Morflo actually has, for the account Morflo actually runs as.
fn probe_writable(directory: &Path) -> Result<(), ConversionError> {
    for _ in 0..8 {
        let token = Uuid::new_v4().simple().to_string();
        let path = directory.join(format!(".morflo-access-{}.tmp", &token[..12]));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                drop(file);
                let _ = std::fs::remove_file(&path);
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(ConversionError::new(
                    ConversionErrorCode::PermissionDenied,
                    "Morflo cannot write to this folder",
                    "Choose another output folder or update its permissions.",
                )
                .with_details(error.to_string()));
            }
        }
    }
    Err(ConversionError::new(
        ConversionErrorCode::PermissionDenied,
        "Morflo cannot write to this folder",
        "Choose another output folder or update its permissions.",
    ))
}

fn first_available_path(
    directory: &Path,
    stem: &str,
    extension: &str,
    source: &Path,
    reserved: &HashSet<PathBuf>,
) -> Result<PathBuf, ConversionError> {
    for index in 1..=10_000_u32 {
        let name = if index == 1 {
            format!("{stem}.{extension}")
        } else {
            format!("{stem} ({index}).{extension}")
        };
        let candidate = directory.join(name);
        if !conflicts(&candidate, source, reserved) {
            return Ok(candidate);
        }
    }
    Err(ConversionError::new(
        ConversionErrorCode::OutputExists,
        "No safe output name is available",
        "Move older conversions or choose another output folder.",
    ))
}

fn conflicts(candidate: &Path, source: &Path, reserved: &HashSet<PathBuf>) -> bool {
    candidate.exists()
        || same_path(candidate, source)
        || reserved.contains(&normalized_key(candidate))
}

fn same_path(left: &Path, right: &Path) -> bool {
    if left.exists()
        && right.exists()
        && let (Ok(left), Ok(right)) = (std::fs::canonicalize(left), std::fs::canonicalize(right))
    {
        return normalized_key(&left) == normalized_key(&right);
    }
    normalized_key(left) == normalized_key(right)
}

fn normalized_key(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from(path.to_string_lossy().to_ascii_lowercase())
    }
    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

fn create_owned_partial(
    directory: &Path,
    stem: &str,
    extension: &str,
) -> Result<PathBuf, ConversionError> {
    for _ in 0..8 {
        let token = Uuid::new_v4().simple().to_string();
        let path = directory.join(format!(".{stem}.morflo-part-{}.{extension}", &token[..12]));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => {
                if let Err(error) = register_recovery(&path) {
                    let _ = std::fs::remove_file(&path);
                    return Err(error);
                }
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(ConversionError::new(
                    ConversionErrorCode::PermissionDenied,
                    "Morflo cannot write to this folder",
                    "Choose another output folder or update its permissions.",
                )
                .with_details(error.to_string()));
            }
        }
    }
    Err(ConversionError::new(
        ConversionErrorCode::EngineFailed,
        "Temporary output could not be created",
        "Try the conversion again or choose another output folder.",
    ))
}

fn create_owned_auxiliary(
    directory: &Path,
    stem: &str,
    role: &str,
    extension: &str,
) -> Result<PathBuf, ConversionError> {
    for _ in 0..8 {
        let token = Uuid::new_v4().simple().to_string();
        let path = directory.join(format!(
            ".{stem}.morflo-{role}-{}.{extension}",
            &token[..12]
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => {
                if let Err(error) = register_recovery(&path) {
                    let _ = std::fs::remove_file(&path);
                    return Err(error);
                }
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(ConversionError::new(
                    ConversionErrorCode::PermissionDenied,
                    "Morflo cannot prepare this conversion",
                    "Choose another output folder or update its permissions.",
                )
                .with_details(error.to_string()));
            }
        }
    }
    Err(ConversionError::new(
        ConversionErrorCode::EngineFailed,
        "Conversion preparation could not finish",
        "Try the conversion again or choose another output folder.",
    ))
}

fn is_recognized_partial(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name.starts_with('.') && name.contains(".morflo-part-"))
}

fn is_recognized_auxiliary(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| {
            name.starts_with('.') && name.contains(".morflo-palette-") && name.ends_with(".png")
        })
}

pub fn recover_abandoned_outputs() -> StartupReport {
    let directory = recovery_directory();
    if prepare_recovery_directory(&directory).is_err() {
        return StartupReport {
            recovered_temporary_files: 0,
            recovery_failures: 1,
        };
    }
    recover_from(&directory)
}

fn register_recovery(path: &Path) -> Result<(), ConversionError> {
    let token = recovery_token(path).ok_or_else(|| {
        ConversionError::invalid("Morflo refused to track an unexpected temporary file.")
    })?;
    if !path.is_absolute() {
        return Err(ConversionError::invalid(
            "Morflo temporary outputs must use an absolute path.",
        ));
    }
    let _guard = RECOVERY_LOCK.lock().map_err(|_| recovery_error())?;
    let directory = recovery_directory();
    prepare_recovery_directory(&directory)?;
    let marker = directory.join(format!("{token}.json"));
    let bytes = serde_json::to_vec(&RecoveryEntry {
        version: 1,
        path: path.to_path_buf(),
    })
    .map_err(|_| recovery_error())?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(&marker).map_err(|_| recovery_error())?;
    use std::io::Write as _;
    file.write_all(&bytes).map_err(|_| recovery_error())?;
    file.sync_all().map_err(|_| recovery_error())
}

fn unregister_recovery(path: &Path) {
    let Some(token) = recovery_token(path) else {
        return;
    };
    if let Ok(_guard) = RECOVERY_LOCK.lock() {
        let directory = recovery_directory();
        if prepare_recovery_directory(&directory).is_ok() {
            let _ = std::fs::remove_file(directory.join(format!("{token}.json")));
        }
    }
}

fn remove_owned_temporary(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(()) => unregister_recovery(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => unregister_recovery(path),
        Err(_) => {}
    }
}

fn recovery_directory() -> PathBuf {
    #[cfg(unix)]
    {
        // SAFETY: geteuid has no preconditions and does not dereference pointers.
        let user_id = unsafe { libc::geteuid() };
        std::env::temp_dir().join(format!("{RECOVERY_DIRECTORY}-{user_id}"))
    }
    #[cfg(not(unix))]
    {
        std::env::temp_dir().join(RECOVERY_DIRECTORY)
    }
}

fn prepare_recovery_directory(directory: &Path) -> Result<(), ConversionError> {
    match std::fs::symlink_metadata(directory) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => return Err(recovery_error()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match std::fs::create_dir(directory) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(recovery_error()),
            }
            let metadata = std::fs::symlink_metadata(directory).map_err(|_| recovery_error())?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(recovery_error());
            }
        }
        Err(_) => return Err(recovery_error()),
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| recovery_error())?;
    }
    Ok(())
}

fn recover_from(directory: &Path) -> StartupReport {
    let Ok(_guard) = RECOVERY_LOCK.lock() else {
        return StartupReport {
            recovered_temporary_files: 0,
            recovery_failures: 1,
        };
    };
    let Ok(markers) = std::fs::read_dir(directory) else {
        return StartupReport::default();
    };
    let mut report = StartupReport::default();
    for marker in markers.flatten() {
        let marker_path = marker.path();
        let marker_token = marker_path
            .file_stem()
            .and_then(|value| value.to_str())
            .filter(|value| is_recovery_token(value));
        let entry = std::fs::read(&marker_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<RecoveryEntry>(&bytes).ok());
        let Some((token, entry)) = marker_token.zip(entry) else {
            report.recovery_failures += 1;
            let _ = std::fs::remove_file(&marker_path);
            continue;
        };
        let safe_target = entry.version == 1
            && entry.path.is_absolute()
            && recovery_token(&entry.path) == Some(token)
            && (is_recognized_partial(&entry.path) || is_recognized_auxiliary(&entry.path));
        if !safe_target {
            report.recovery_failures += 1;
            let _ = std::fs::remove_file(&marker_path);
            continue;
        }
        match std::fs::symlink_metadata(&entry.path) {
            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                if std::fs::remove_file(&entry.path).is_ok() {
                    report.recovered_temporary_files += 1;
                    let _ = std::fs::remove_file(&marker_path);
                } else {
                    report.recovery_failures += 1;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let _ = std::fs::remove_file(&marker_path);
            }
            _ => {
                report.recovery_failures += 1;
                let _ = std::fs::remove_file(&marker_path);
            }
        }
    }
    report
}

fn recovery_token(path: &Path) -> Option<&str> {
    let name = path.file_name()?.to_str()?;
    let suffix = name
        .rsplit_once(".morflo-part-")
        .map(|(_, suffix)| suffix)
        .or_else(|| {
            name.rsplit_once(".morflo-palette-")
                .map(|(_, suffix)| suffix)
        })?;
    let token = suffix.split_once('.')?.0;
    is_recovery_token(token).then_some(token)
}

fn is_recovery_token(token: &str) -> bool {
    token.len() == 12 && token.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn recovery_error() -> ConversionError {
    ConversionError::new(
        ConversionErrorCode::EngineFailed,
        "Temporary output safety could not be prepared",
        "Restart Morflo and try the conversion again.",
    )
}

fn output_exists_error() -> ConversionError {
    ConversionError::new(
        ConversionErrorCode::OutputExists,
        "The output already exists",
        "Choose automatic suffixes, a different folder, or explicitly replace the existing file.",
    )
}

fn map_finalize_error(error: std::io::Error) -> ConversionError {
    let code = if error.kind() == std::io::ErrorKind::AlreadyExists {
        ConversionErrorCode::OutputExists
    } else if error.kind() == std::io::ErrorKind::PermissionDenied {
        ConversionErrorCode::PermissionDenied
    } else {
        ConversionErrorCode::EngineFailed
    };
    let (title, message) = match code {
        ConversionErrorCode::OutputExists => (
            "The output appeared while converting",
            "Morflo left the newer file untouched. Try again to choose another safe name.",
        ),
        ConversionErrorCode::PermissionDenied => (
            "The output could not be finalized",
            "Check the destination permissions or choose another folder.",
        ),
        _ => (
            "The output could not be finalized",
            "The converted temporary file was not published under the final name.",
        ),
    };
    ConversionError::new(code, title, message).with_details(error.to_string())
}

fn atomic_publish_new(partial: &Path, final_path: &Path) -> std::io::Result<()> {
    if final_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "final path already exists",
        ));
    }

    #[cfg(windows)]
    {
        move_file_windows(partial, final_path, false)
    }
    #[cfg(not(windows))]
    {
        std::fs::hard_link(partial, final_path)?;
        let _ = std::fs::remove_file(partial);
        Ok(())
    }
}

fn atomic_replace(partial: &Path, final_path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        move_file_windows(partial, final_path, true)
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(partial, final_path)
    }
}

#[cfg(windows)]
fn move_file_windows(source: &Path, target: &Path, replace: bool) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let target = target
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut flags = MOVEFILE_WRITE_THROUGH;
    if replace {
        flags |= MOVEFILE_REPLACE_EXISTING;
    }
    // SAFETY: both pointers reference NUL-terminated UTF-16 buffers that remain alive for the call.
    if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), flags) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn suffix_policy_never_reuses_an_existing_name() {
        let directory = tempdir().expect("temporary test directory");
        let source = directory.path().join("summer.png");
        std::fs::write(&source, b"source").expect("write source fixture");
        std::fs::write(directory.path().join("summer.jpg"), b"existing")
            .expect("write collision fixture");

        let outputs = OutputReservations::default();
        let reservation = outputs
            .reserve(
                &source,
                OutputFormat::Jpeg,
                DestinationMode::Same,
                None,
                CollisionPolicy::Suffix,
            )
            .expect("reserve suffixed output");

        assert_eq!(
            reservation
                .final_path
                .file_name()
                .and_then(|name| name.to_str()),
            Some("summer (2).jpg")
        );
        reservation.cleanup_partial();
    }

    #[test]
    fn same_format_conversion_cannot_target_the_source() {
        let directory = tempdir().expect("temporary test directory");
        let source = directory.path().join("京都 🧳.png");
        std::fs::write(&source, b"source").expect("write source fixture");

        let outputs = OutputReservations::default();
        let reservation = outputs
            .reserve(
                &source,
                OutputFormat::Png,
                DestinationMode::Same,
                None,
                CollisionPolicy::Suffix,
            )
            .expect("reserve safe same-format output");

        assert_eq!(
            reservation
                .final_path
                .file_name()
                .and_then(|name| name.to_str()),
            Some("京都 🧳 (2).png")
        );
        reservation.cleanup_partial();
    }

    #[test]
    fn partial_is_published_only_after_nonempty_output() {
        let directory = tempdir().expect("temporary test directory");
        let source = directory.path().join("input.png");
        std::fs::write(&source, b"source").expect("write source fixture");
        let outputs = OutputReservations::default();
        let reservation = outputs
            .reserve(
                &source,
                OutputFormat::Jpeg,
                DestinationMode::Same,
                None,
                CollisionPolicy::Suffix,
            )
            .expect("reserve output");

        assert!(reservation.finalize().is_err());
        assert!(!reservation.final_path.exists());
        std::fs::write(&reservation.partial_path, b"converted").expect("write converted fixture");
        reservation.finalize().expect("publish output");
        assert_eq!(
            std::fs::read(&reservation.final_path).expect("read published output"),
            b"converted"
        );
    }

    #[test]
    fn palette_temporary_is_owned_and_narrowly_cleaned() {
        let directory = tempdir().expect("temporary test directory");
        let source = directory.path().join("clip.mp4");
        std::fs::write(&source, b"source").expect("write source fixture");
        let outputs = OutputReservations::default();
        let reservation = outputs
            .reserve(
                &source,
                OutputFormat::Gif,
                DestinationMode::Same,
                None,
                CollisionPolicy::Suffix,
            )
            .expect("reserve GIF output");
        let palette = reservation
            .create_temporary("palette", "png")
            .expect("reserve palette temporary");
        let unrelated = directory.path().join("palette.png");
        std::fs::write(&unrelated, b"keep").expect("write unrelated file");

        assert!(palette.exists());
        ReservedOutput::cleanup_temporary(&unrelated);
        assert!(unrelated.exists());
        ReservedOutput::cleanup_temporary(&palette);
        assert!(!palette.exists());
        reservation.cleanup_partial();
    }

    #[test]
    fn startup_recovery_removes_only_marker_owned_temporary_files() {
        let directory = tempdir().expect("temporary test directory");
        let marker_directory = directory.path().join("markers");
        std::fs::create_dir(&marker_directory).expect("create marker directory");
        let token = "a1b2c3d4e5f6";
        let abandoned = directory
            .path()
            .join(format!(".clip.morflo-part-{token}.gif"));
        let unrelated = directory.path().join("clip.gif");
        std::fs::write(&abandoned, b"partial").expect("write abandoned partial");
        std::fs::write(&unrelated, b"keep").expect("write unrelated output");
        std::fs::write(
            marker_directory.join(format!("{token}.json")),
            serde_json::to_vec(&RecoveryEntry {
                version: 1,
                path: abandoned.clone(),
            })
            .expect("serialize recovery entry"),
        )
        .expect("write recovery marker");

        let report = recover_from(&marker_directory);

        assert_eq!(report.recovered_temporary_files, 1);
        assert_eq!(report.recovery_failures, 0);
        assert!(!abandoned.exists());
        assert_eq!(
            std::fs::read(&unrelated).expect("read unrelated output"),
            b"keep"
        );
    }

    #[test]
    fn startup_recovery_rejects_a_marker_token_mismatch() {
        let directory = tempdir().expect("temporary test directory");
        let marker_directory = directory.path().join("markers");
        std::fs::create_dir(&marker_directory).expect("create marker directory");
        let target = directory.path().join(".clip.morflo-part-a1b2c3d4e5f6.gif");
        std::fs::write(&target, b"do-not-delete").expect("write mismatched target");
        std::fs::write(
            marker_directory.join("111111111111.json"),
            serde_json::to_vec(&RecoveryEntry {
                version: 1,
                path: target.clone(),
            })
            .expect("serialize mismatched entry"),
        )
        .expect("write mismatched marker");

        let report = recover_from(&marker_directory);

        assert_eq!(report.recovered_temporary_files, 0);
        assert_eq!(report.recovery_failures, 1);
        assert_eq!(
            std::fs::read(&target).expect("read protected target"),
            b"do-not-delete"
        );
    }

    #[test]
    fn a_writable_destination_passes_and_leaves_no_probe_file() {
        let directory = tempdir().expect("temporary test directory");

        validate_destination(directory.path()).expect("a writable folder must pass validation");

        let leftovers: Vec<_> = std::fs::read_dir(directory.path())
            .expect("read probed directory")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .collect();
        assert!(
            leftovers.is_empty(),
            "the access probe must clean up after itself, found {leftovers:?}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_customized_windows_folder_is_not_mistaken_for_read_only() {
        let directory = tempdir().expect("temporary test directory");
        let customized = directory.path().join("Downloads");
        std::fs::create_dir(&customized).expect("create customized output directory");
        let mut permissions = std::fs::metadata(&customized)
            .expect("read customized directory metadata")
            .permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&customized, permissions).expect("set the read-only attribute");
        assert!(
            std::fs::metadata(&customized)
                .expect("re-read customized directory metadata")
                .permissions()
                .readonly(),
            "the fixture must carry the attribute Windows sets on shell folders"
        );

        validate_destination(&customized)
            .expect("a customized but writable Windows folder must pass validation");
    }

    #[cfg(unix)]
    #[test]
    fn a_destination_that_refuses_new_files_is_rejected() {
        use std::os::unix::fs::PermissionsExt as _;

        let directory = tempdir().expect("temporary test directory");
        let closed = directory.path().join("closed-output");
        std::fs::create_dir(&closed).expect("create closed output directory");
        std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o500))
            .expect("close the output directory");

        let error =
            validate_destination(&closed).expect_err("an unwritable folder must be rejected");
        assert_eq!(error.code, ConversionErrorCode::PermissionDenied);

        std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o700))
            .expect("restore the output directory");
    }

    #[cfg(unix)]
    #[test]
    fn linked_output_directory_is_rejected() {
        use std::os::unix::fs::symlink;

        let directory = tempdir().expect("temporary test directory");
        let real = directory.path().join("real-output");
        let linked = directory.path().join("linked-output");
        std::fs::create_dir(&real).expect("create real output directory");
        symlink(&real, &linked).expect("create output directory symlink");

        let error = validate_destination(&linked)
            .expect_err("linked destinations must not pass validation");
        assert_eq!(error.code, ConversionErrorCode::InvalidRequest);
    }

    #[cfg(unix)]
    #[test]
    fn recovery_directory_is_private_and_cannot_be_a_symlink() {
        use std::os::unix::fs::{PermissionsExt as _, symlink};

        let directory = tempdir().expect("temporary test directory");
        let private = directory.path().join("private-recovery");
        prepare_recovery_directory(&private).expect("create private recovery directory");
        let mode = std::fs::metadata(&private)
            .expect("read private recovery metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700);

        let linked = directory.path().join("linked-recovery");
        symlink(&private, &linked).expect("create recovery symlink");
        assert!(prepare_recovery_directory(&linked).is_err());
    }
}
