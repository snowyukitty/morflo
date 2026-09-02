use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};

const MAX_PENDING_FILES: usize = 512;

use crate::{
    domain::{ConversionError, ConversionErrorCode, InspectedSource, StartupReport},
    engine::EngineService,
    jobs::JobManager,
    output::recover_abandoned_outputs,
};

#[derive(Debug, Default)]
pub struct SourceStore {
    sources: RwLock<HashMap<String, Arc<InspectedSource>>>,
    failed_paths: RwLock<HashMap<String, PathBuf>>,
}

impl SourceStore {
    pub fn insert(&self, source: InspectedSource) -> Result<(), ConversionError> {
        self.failed_paths
            .write()
            .map_err(store_error)?
            .remove(&source.media.id);
        let mut sources = self.sources.write().map_err(store_error)?;
        sources.insert(source.media.id.clone(), Arc::new(source));
        Ok(())
    }

    pub fn insert_failed(&self, id: String, path: &Path) -> Result<(), ConversionError> {
        self.failed_paths
            .write()
            .map_err(store_error)?
            .insert(id, path.to_path_buf());
        Ok(())
    }

    pub fn failed_path(&self, id: &str) -> Result<PathBuf, ConversionError> {
        self.failed_paths
            .read()
            .map_err(store_error)?
            .get(id)
            .cloned()
            .ok_or_else(|| {
                ConversionError::new(
                    ConversionErrorCode::InvalidRequest,
                    "This file cannot be inspected again",
                    "Remove it from the queue and add the source again.",
                )
            })
    }

    pub fn get(&self, id: &str) -> Result<Arc<InspectedSource>, ConversionError> {
        let sources = self.sources.read().map_err(store_error)?;
        sources.get(id).cloned().ok_or_else(|| {
            ConversionError::new(
                ConversionErrorCode::InvalidRequest,
                "This queue item is no longer available",
                "Remove the item, add the source file again, and retry.",
            )
        })
    }

    pub fn remove(&self, id: &str) -> Result<(), ConversionError> {
        self.sources.write().map_err(store_error)?.remove(id);
        self.failed_paths.write().map_err(store_error)?.remove(id);
        Ok(())
    }

    pub fn clear(&self) -> Result<(), ConversionError> {
        self.sources.write().map_err(store_error)?.clear();
        self.failed_paths.write().map_err(store_error)?.clear();
        Ok(())
    }
}

fn store_error<T>(_: std::sync::PoisonError<T>) -> ConversionError {
    ConversionError::new(
        ConversionErrorCode::EngineFailed,
        "The local session state is unavailable",
        "Restart Morflo and add the files again.",
    )
}

#[derive(Debug)]
pub struct AppState {
    pub engine: EngineService,
    pub sources: SourceStore,
    pub jobs: JobManager,
    pub startup_report: StartupReport,
    pending_files: Mutex<Vec<PathBuf>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            engine: EngineService::new(),
            sources: SourceStore::default(),
            jobs: JobManager::new(),
            startup_report: recover_abandoned_outputs(),
            pending_files: Mutex::new(file_arguments(std::env::args_os(), None)),
        }
    }
}

impl AppState {
    pub fn take_pending_files(&self) -> Result<Vec<String>, ConversionError> {
        let mut files = self.pending_files.lock().map_err(store_error)?;
        Ok(files
            .drain(..)
            .filter_map(|path| path.to_str().map(str::to_owned))
            .collect())
    }

    pub fn enqueue_file_arguments<I, S>(
        &self,
        arguments: I,
        current_directory: &Path,
    ) -> Result<usize, ConversionError>
    where
        I: IntoIterator<Item = S>,
        S: Into<std::ffi::OsString>,
    {
        let incoming = file_arguments(arguments, Some(current_directory));
        let mut files = self.pending_files.lock().map_err(store_error)?;
        let available = MAX_PENDING_FILES.saturating_sub(files.len());
        let accepted = incoming.len().min(available);
        files.extend(incoming.into_iter().take(available));
        Ok(accepted)
    }
}

fn file_arguments<I, S>(arguments: I, current_directory: Option<&Path>) -> Vec<PathBuf>
where
    I: IntoIterator<Item = S>,
    S: Into<std::ffi::OsString>,
{
    let mut seen = HashSet::new();
    arguments
        .into_iter()
        .skip(1)
        .map(|argument| PathBuf::from(argument.into()))
        .map(|path| {
            if path.is_relative() {
                current_directory.map_or(path.clone(), |directory| directory.join(path))
            } else {
                path
            }
        })
        .filter(|path| {
            std::fs::symlink_metadata(path)
                .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        })
        .filter(|path| seen.insert(path.clone()))
        .take(MAX_PENDING_FILES)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn failed_inspection_paths_are_session_scoped_and_removable() {
        let store = SourceStore::default();
        let path = Path::new("international/京都 🧳 O'Reilly.webp");
        store
            .insert_failed("failed-job".to_owned(), path)
            .expect("record failed inspection source");

        assert_eq!(
            store
                .failed_path("failed-job")
                .expect("recover retry source"),
            path
        );
        store.remove("failed-job").expect("forget failed source");
        assert!(store.failed_path("failed-job").is_err());
    }

    #[test]
    fn startup_arguments_accept_individual_unicode_files_without_scanning_folders() {
        let root = tempdir().expect("create temporary startup directory");
        let source = root.path().join("京都 🧳 O'Reilly.png");
        std::fs::write(&source, b"fixture").expect("write startup fixture");

        let files = file_arguments(
            [
                std::ffi::OsString::from("morflo.exe"),
                source.as_os_str().to_owned(),
                root.path().as_os_str().to_owned(),
                root.path().join("missing.png").into_os_string(),
            ],
            None,
        );

        assert_eq!(files, vec![source]);
    }

    #[test]
    fn later_activation_resolves_its_working_directory_and_deduplicates_files() {
        let root = tempdir().expect("create activation directory");
        let first = root.path().join("京都 image.png");
        let second = root.path().join("O'Reilly clip.mov");
        std::fs::write(&first, b"image").expect("write first activation file");
        std::fs::write(&second, b"video").expect("write second activation file");

        let files = file_arguments(
            [
                "morflo.exe",
                "京都 image.png",
                "O'Reilly clip.mov",
                "京都 image.png",
                ".",
                "missing.webp",
            ],
            Some(root.path()),
        );

        assert_eq!(files, vec![first, second]);
    }
}
