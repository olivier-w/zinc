mod commands;
mod config;
mod ytdlp;
mod ytdlp_manager;

use commands::AppState;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            commands::select_directory,
            commands::open_file,
            commands::open_folder,
            commands::get_format_presets,
            commands::get_ytdlp_status,
            commands::install_ytdlp,
            commands::update_ytdlp,
            commands::check_ytdlp_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
