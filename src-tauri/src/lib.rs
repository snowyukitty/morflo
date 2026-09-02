mod commands;
pub mod domain;
pub mod engine;
pub mod engine_bundle;
mod engine_probe;
mod engine_process;
pub mod image_engine;
mod jobs;
pub mod output;
pub mod planner;
pub mod preview;
pub mod probe;
pub mod progress;
mod state;

#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
use std::path::Path;

#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
use serde::Serialize;
#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
use tauri::{Emitter, Manager};

#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
const EXTERNAL_FILES_EVENT: &str = "morflo://external-files";

#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExternalFilesAvailable {
    count: usize,
}

#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
fn receive_external_files(app: &tauri::AppHandle, arguments: Vec<String>, cwd: String) {
    let accepted = app
        .state::<state::AppState>()
        .enqueue_file_arguments(arguments, Path::new(&cwd))
        .unwrap_or(0);

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        if accepted > 0 {
            let _ = window.emit(
                EXTERNAL_FILES_EVENT,
                ExternalFilesAvailable { count: accepted },
            );
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(any(target_os = "macos", windows, target_os = "linux"))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, arguments, cwd| {
        receive_external_files(app, arguments, cwd)
    }));

    builder
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::runtime_ready,
            commands::startup_report,
            commands::pending_files,
            commands::get_capabilities,
            commands::refresh_capabilities,
            commands::select_engine_directory,
            commands::inspect_files,
            commands::retry_inspection,
            commands::start_job,
            commands::cancel_job,
            commands::reveal_output,
            commands::forget_source,
            commands::clear_session,
            commands::image_thumbnail,
            commands::poster_frame,
            commands::video_storyboard,
            commands::preview_clip,
            commands::output_preview,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| eprintln!("Morflo failed to start: {error}"));
}
