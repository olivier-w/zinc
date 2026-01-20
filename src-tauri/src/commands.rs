use crate::config::AppConfig;
use crate::ytdlp::{DownloadOptions, DownloadProgress, VideoInfo, YtDlp};
use crate::ytdlp_manager::{InstallProgress, YtDlpManager, YtDlpStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    pub id: String,
    pub url: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub status: String,
    pub progress: f64,
    pub speed: Option<String>,
    pub eta: Option<String>,
    pub output_path: Option<String>,
    pub format: String,
    pub error: Option<String>,
}

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub downloads: Mutex<HashMap<String, Download>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: Mutex::new(AppConfig::load()),
            downloads: Mutex::new(HashMap::new()),
        }
    }
}

#[tauri::command]
pub async fn check_ytdlp() -> Result<bool, String> {
    Ok(YtDlp::check_installed().await)
}

#[tauri::command]
pub async fn get_video_info(url: String) -> Result<VideoInfo, String> {
    YtDlp::get_video_info(&url).await
}

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    url: String,
    format: String,
    title: String,
    thumbnail: Option<String>,
) -> Result<String, String> {
    let download_id = Uuid::new_v4().to_string();
    let config = state.config.lock().await;

    let download = Download {
        id: download_id.clone(),
        url: url.clone(),
        title: title.clone(),
        thumbnail: thumbnail.clone(),
        status: "pending".to_string(),
        progress: 0.0,
        speed: None,
        eta: None,
        output_path: None,
        format: format.clone(),
        error: None,
    };

    state.downloads.lock().await.insert(download_id.clone(), download.clone());

    let format_string = YtDlp::get_format_presets()
        .get(&format)
        .cloned()
        .unwrap_or_else(|| format.clone());

    let container_format = if format == "audio" || format == "mp3" || config.default_format == "original" {
        None // Audio-only or original format doesn't need remux
    } else {
        Some(config.default_format.clone())
    };

    let options = DownloadOptions {
        format: format_string,
        output_dir: config.output_dir.clone(),
        filename_template: None,
        container_format,
    };

    drop(config);

    let (progress_tx, mut progress_rx) = mpsc::channel::<DownloadProgress>(100);

    let app_clone = app.clone();
    let download_id_clone = download_id.clone();
    let state_clone = Arc::clone(&state.inner());

    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            let mut downloads = state_clone.downloads.lock().await;
            if let Some(download) = downloads.get_mut(&progress.download_id) {
                download.progress = progress.progress;
                download.speed = progress.speed.clone();
                download.eta = progress.eta.clone();
                download.status = progress.status.clone();
                if let Some(filename) = &progress.filename {
                    download.output_path = Some(filename.clone());
                }

                let _ = app_clone.emit("download-progress", download.clone());
            }
        }
    });

    let state_clone = Arc::clone(&state.inner());
    let app_clone = app.clone();

    tokio::spawn(async move {
        {
            let mut downloads = state_clone.downloads.lock().await;
            if let Some(download) = downloads.get_mut(&download_id_clone) {
                download.status = "downloading".to_string();
                let _ = app_clone.emit("download-progress", download.clone());
            }
        }

        match YtDlp::start_download(&url, options, progress_tx, download_id_clone.clone()).await {
            Ok(path) => {
                let mut downloads = state_clone.downloads.lock().await;
                if let Some(download) = downloads.get_mut(&download_id_clone) {
                    download.status = "completed".to_string();
                    download.progress = 100.0;
                    download.output_path = Some(path.to_string_lossy().to_string());
                    let _ = app_clone.emit("download-progress", download.clone());
                }
            }
            Err(e) => {
                let mut downloads = state_clone.downloads.lock().await;
                if let Some(download) = downloads.get_mut(&download_id_clone) {
                    download.status = "error".to_string();
                    download.error = Some(e);
                    let _ = app_clone.emit("download-progress", download.clone());
                }
            }
        }
    });

    Ok(download_id)
}

#[tauri::command]
pub async fn cancel_download(
    state: State<'_, Arc<AppState>>,
    download_id: String,
) -> Result<(), String> {
    let mut downloads = state.downloads.lock().await;
    if let Some(download) = downloads.get_mut(&download_id) {
        download.status = "cancelled".to_string();
    }
    Ok(())
}

#[tauri::command]
pub async fn get_downloads(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<Download>, String> {
    let downloads = state.downloads.lock().await;
    Ok(downloads.values().cloned().collect())
}

#[tauri::command]
pub async fn clear_download(
    state: State<'_, Arc<AppState>>,
    download_id: String,
) -> Result<(), String> {
    state.downloads.lock().await.remove(&download_id);
    Ok(())
}

#[tauri::command]
pub async fn clear_completed_downloads(
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let mut downloads = state.downloads.lock().await;
    downloads.retain(|_, d| d.status != "completed" && d.status != "error" && d.status != "cancelled");
    Ok(())
}

#[tauri::command]
pub async fn get_config(
    state: State<'_, Arc<AppState>>,
) -> Result<AppConfig, String> {
    Ok(state.config.lock().await.clone())
}

#[tauri::command]
pub async fn update_config(
    state: State<'_, Arc<AppState>>,
    config: AppConfig,
) -> Result<(), String> {
    config.save()?;
    *state.config.lock().await = config;
    Ok(())
}

#[tauri::command]
pub async fn select_directory() -> Result<Option<String>, String> {
    // This is handled by the frontend using the dialog plugin directly
    Ok(None)
}

#[tauri::command]
pub async fn open_file(path: String) -> Result<(), String> {
    opener::open(&path).map_err(|e| format!("Failed to open file: {}", e))
}

#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    let path = PathBuf::from(&path);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        // On Windows, use explorer.exe /select, to open folder with file selected
        if path.exists() {
            std::process::Command::new("explorer.exe")
                .raw_arg(format!("/select,\"{}\"", path.display()))
                .spawn()
                .map_err(|e| format!("Failed to open folder: {}", e))?;
            return Ok(());
        }
    }

    // Fallback: open the parent directory if file doesn't exist or on other platforms
    let folder = path.parent().map(|p| p.to_path_buf()).unwrap_or(path);
    if folder.exists() {
        opener::open(&folder).map_err(|e| format!("Failed to open folder: {}", e))
    } else {
        Err(format!("Path does not exist: {}", folder.display()))
    }
}

#[tauri::command]
pub fn get_format_presets() -> HashMap<String, String> {
    YtDlp::get_format_presets()
}

// yt-dlp manager commands

#[tauri::command]
pub async fn get_ytdlp_status() -> Result<YtDlpStatus, String> {
    Ok(YtDlpManager::check_status().await)
}

#[tauri::command]
pub async fn install_ytdlp(app: AppHandle) -> Result<String, String> {
    let app_clone = app.clone();

    let version = YtDlpManager::install(move |progress: InstallProgress| {
        let _ = app_clone.emit("ytdlp-install-progress", progress);
    })
    .await?;

    Ok(version)
}

#[tauri::command]
pub async fn update_ytdlp(app: AppHandle) -> Result<String, String> {
    let app_clone = app.clone();

    let version = YtDlpManager::update(move |progress: InstallProgress| {
        let _ = app_clone.emit("ytdlp-install-progress", progress);
    })
    .await?;

    Ok(version)
}

#[tauri::command]
pub async fn check_ytdlp_update() -> Result<Option<String>, String> {
    let current = match YtDlpManager::get_installed_version().await {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };

    let latest = YtDlpManager::get_latest_version().await?;

    if current != latest {
        Ok(Some(latest))
    } else {
        Ok(None)
    }
}
