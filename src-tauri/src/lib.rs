mod commands;
mod config;
mod deno_manager;
mod network;
mod sherpa_manager;
mod transcription;
mod transcription_manager;
mod whisper;
mod whisper_manager;
mod ytdlp;
mod ytdlp_manager;

use commands::AppState;
use std::sync::Arc;

/// On macOS, GUI apps launched from Finder/Spotlight get a minimal PATH
/// that doesn't include Homebrew or MacPorts paths. This ensures commonly
/// installed binaries like ffmpeg are discoverable.
#[cfg(target_os = "macos")]
fn fix_path_env() {
    use std::env;
    let path = env::var("PATH").unwrap_or_default();
    let extra_paths = [
        "/opt/homebrew/bin",       // Homebrew (Apple Silicon)
        "/opt/homebrew/sbin",
        "/usr/local/bin",          // Homebrew (Intel) / MacPorts
        "/usr/local/sbin",
        "/opt/local/bin",          // MacPorts
        "/opt/local/sbin",
    ];
    let mut new_path = path.clone();
    for p in &extra_paths {
        if !path.split(':').any(|seg| seg == *p) {
            new_path = format!("{}:{}", new_path, p);
        }
    }
    env::set_var("PATH", &new_path);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "macos")]
    fix_path_env();

    let state = Arc::new(AppState::default());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::check_ytdlp,
            commands::get_video_info,
            commands::start_download,
            commands::cancel_download,
            commands::get_downloads,
            commands::clear_download,
            commands::clear_completed_downloads,
            commands::get_config,
            commands::update_config,
            commands::open_file,
            commands::open_folder,
            commands::get_format_presets,
            commands::get_ytdlp_status,
            commands::get_ytdlp_status_fast,
            commands::install_ytdlp,
            commands::update_ytdlp,
            commands::check_ytdlp_update,
            commands::get_whisper_status,
            commands::install_whisper,
            commands::download_whisper_model,
            commands::get_available_whisper_models,
            commands::check_ffmpeg,
            // Transcription engine commands
            commands::get_transcription_engines,
            commands::get_engine_models,
            commands::install_transcription_engine,
            commands::download_transcription_model,
            commands::get_transcription_speed_multiplier,
            // Local file transcription (unified with downloads)
            commands::add_local_transcription,
            commands::start_local_transcription,
            commands::update_transcription_settings,
            // Network interface
            commands::list_network_interfaces,
            // Deno manager
            commands::get_deno_status,
            commands::install_deno,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
