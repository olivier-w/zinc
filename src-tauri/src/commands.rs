use crate::config::AppConfig;
use crate::deno_manager::{DenoManager, DenoStatus};
use crate::network::{self, NetworkInterface};
use crate::transcription::{EngineInfo, InstallProgress as TranscriptionInstallProgress, TranscribeProgress, TranscriptionModel as TranscriptionModelInfo};
use crate::transcription_manager::TranscriptionManager;
use crate::whisper::Whisper;
use crate::whisper_manager::{InstallProgress as WhisperInstallProgress, WhisperManager, WhisperModel, WhisperStatus};
use crate::ytdlp::{DownloadOptions, DownloadProgress, VideoInfo, YtDlp};
use crate::ytdlp_manager::{InstallProgress, YtDlpManager, YtDlpStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, watch, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleSettings {
    pub enabled: bool,
    pub engine: String,  // "whisper_rs", "moonshine"
    pub model: String,
    pub style: String,   // "word" or "sentence"
}

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
    pub duration: Option<f64>,
    pub whisper_model: Option<String>,
    pub transcription_engine: Option<String>,
    pub transcription_progress: Option<f64>,
    pub transcription_message: Option<String>,
    pub task_type: String,           // "download" | "local_transcribe"
    pub source_path: Option<String>, // Input file path for local transcriptions
}

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub downloads: Mutex<HashMap<String, Download>>,
    pub cancel_senders: Mutex<HashMap<String, watch::Sender<bool>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: Mutex::new(AppConfig::load()),
            downloads: Mutex::new(HashMap::new()),
            cancel_senders: Mutex::new(HashMap::new()),
        }
    }
}

#[tauri::command]
pub async fn check_ytdlp() -> Result<bool, String> {
    Ok(YtDlp::check_installed().await)
}

#[tauri::command]
pub async fn get_video_info(
    state: State<'_, Arc<AppState>>,
    url: String,
) -> Result<VideoInfo, String> {
    let cookies_browser = state.config.lock().await.cookies_browser.clone();
    YtDlp::get_video_info(&url, cookies_browser.as_deref()).await
}

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    url: String,
    format: String,
    title: String,
    thumbnail: Option<String>,
    subtitle_settings: Option<SubtitleSettings>,
    duration: Option<f64>,
) -> Result<String, String> {
    let download_id = Uuid::new_v4().to_string();
    let config = state.config.lock().await;

    // Use per-video subtitle settings if provided, otherwise fall back to global config
    let (generate_subtitles, transcription_engine, transcription_model, transcription_style) = match &subtitle_settings {
        Some(settings) => (
            settings.enabled,
            settings.engine.clone(),
            settings.model.clone(),
            settings.style.clone(),
        ),
        None => (
            config.generate_subtitles,
            config.transcription_engine.clone(),
            config.transcription_model.clone(),
            "sentence".to_string(), // Default style when using global config
        ),
    };

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
        duration,
        whisper_model: if generate_subtitles { Some(transcription_model.clone()) } else { None },
        transcription_engine: if generate_subtitles { Some(transcription_engine.clone()) } else { None },
        transcription_progress: None,
        transcription_message: None,
        task_type: "download".to_string(),
        source_path: None,
    };

    state.downloads.lock().await.insert(download_id.clone(), download.clone());

    // Parse format string - can be "quality" or "quality:container"
    let (quality, container) = if format.contains(':') {
        let parts: Vec<&str> = format.split(':').collect();
        (parts[0].to_string(), Some(parts[1].to_string()))
    } else {
        (format.clone(), None)
    };

    let format_string = YtDlp::get_format_presets()
        .get(&quality)
        .cloned()
        .unwrap_or_else(|| quality.clone());

    // Determine container format
    let is_audio_only = quality == "audio" || quality == "mp3";
    let container_format = match &container {
        Some(c) if c == "original" => None, // Original format doesn't need remux
        Some(c) => Some(c.clone()),
        None if is_audio_only => None, // Audio-only doesn't need container
        None => Some(config.default_format.clone()),
    };

    let options = DownloadOptions {
        format: format_string,
        output_dir: config.output_dir.clone(),
        filename_template: None,
        container_format,
        generate_subtitles,
        whisper_model: Some(transcription_model.clone()),
        source_address: config.network_interface.clone(),
        cookies_browser: config.cookies_browser.clone(),
    };

    drop(config);

    let (progress_tx, mut progress_rx) = mpsc::channel::<DownloadProgress>(100);
    let (cancel_tx, cancel_rx) = watch::channel(false);

    // Store the cancel sender so we can signal cancellation later
    state.cancel_senders.lock().await.insert(download_id.clone(), cancel_tx);

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

        let cancel_rx_for_transcription = cancel_rx.clone();
        match YtDlp::start_download(&url, options.clone(), progress_tx, download_id_clone.clone(), cancel_rx).await {
            Ok(path) => {
                let path_str = path.to_string_lossy().to_string();

                // Check if subtitle generation is enabled
                log::info!("Download completed. generate_subtitles={}, is_audio_only={}", options.generate_subtitles, is_audio_only);
                if options.generate_subtitles && !is_audio_only {
                    // Update status to transcribing
                    {
                        let mut downloads = state_clone.downloads.lock().await;
                        if let Some(download) = downloads.get_mut(&download_id_clone) {
                            download.status = "transcribing".to_string();
                            download.output_path = Some(path_str.clone());
                            let _ = app_clone.emit("download-progress", download.clone());
                        }
                    }

                    // Create progress channel for transcription
                    let (transcribe_tx, mut transcribe_rx) = mpsc::channel::<TranscribeProgress>(100);

                    let app_for_transcribe = app_clone.clone();
                    let download_id_for_transcribe = download_id_clone.clone();
                    let state_for_transcribe = state_clone.clone();

                    // Spawn task to forward transcription progress
                    tokio::spawn(async move {
                        while let Some(progress) = transcribe_rx.recv().await {
                            // Skip the "complete" stage - we handle completion in the main flow
                            if progress.stage == "complete" {
                                continue;
                            }
                            let mut downloads = state_for_transcribe.downloads.lock().await;
                            if let Some(download) = downloads.get_mut(&download_id_for_transcribe) {
                                // Don't overwrite if already completed
                                if download.status == "completed" {
                                    continue;
                                }
                                download.status = format!("transcribing:{}", progress.stage);
                                download.transcription_progress = Some(progress.progress);
                                download.transcription_message = Some(progress.message.clone());
                                let _ = app_for_transcribe.emit("transcribe-progress", &progress);
                                let _ = app_for_transcribe.emit("download-progress", download.clone());
                            }
                        }
                    });

                    let video_path = PathBuf::from(&path_str);
                    let transcription_manager = TranscriptionManager::new();

                    log::info!("Starting transcription for: {:?} with engine: {}, model: {}, style: {}",
                        video_path, transcription_engine, transcription_model, transcription_style);

                    match transcription_manager.process_video(
                        &video_path,
                        &transcription_engine,
                        &transcription_model,
                        None, // Language is auto-detected by all engines
                        &transcription_style,
                        transcribe_tx,
                        cancel_rx_for_transcription,
                    ).await {
                        Ok(result) => {
                            log::info!("Transcription successful: {:?}", result);
                            let mut downloads = state_clone.downloads.lock().await;
                            if let Some(download) = downloads.get_mut(&download_id_clone) {
                                download.status = "completed".to_string();
                                download.progress = 100.0;
                                log::info!("Emitting download-progress with status: completed for id: {}", download.id);
                                let emit_result = app_clone.emit("download-progress", download.clone());
                                log::info!("Emit result: {:?}", emit_result);
                            } else {
                                log::warn!("Download not found in state after transcription: {}", download_id_clone);
                            }
                        }
                        Err(e) => {
                            // Transcription failed, but download succeeded
                            // Mark as completed with a warning
                            log::error!("Transcription failed: {}", e);
                            let mut downloads = state_clone.downloads.lock().await;
                            if let Some(download) = downloads.get_mut(&download_id_clone) {
                                download.status = "completed".to_string();
                                download.progress = 100.0;
                                download.error = Some(format!("Subtitle generation failed: {}", e));
                                let _ = app_clone.emit("download-progress", download.clone());
                            }
                        }
                    }
                } else {
                    let mut downloads = state_clone.downloads.lock().await;
                    if let Some(download) = downloads.get_mut(&download_id_clone) {
                        download.status = "completed".to_string();
                        download.progress = 100.0;
                        download.output_path = Some(path_str);
                        let _ = app_clone.emit("download-progress", download.clone());
                    }
                }
            }
            Err(e) => {
                let mut downloads = state_clone.downloads.lock().await;
                if let Some(download) = downloads.get_mut(&download_id_clone) {
                    // Only set error status if not already cancelled
                    if download.status != "cancelled" {
                        download.status = "error".to_string();
                        download.error = Some(e);
                    }
                    let _ = app_clone.emit("download-progress", download.clone());
                }
            }
        }

        // Clean up cancel sender
        state_clone.cancel_senders.lock().await.remove(&download_id_clone);
    });

    Ok(download_id)
}

#[tauri::command]
pub async fn cancel_download(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    download_id: String,
) -> Result<(), String> {
    // Send cancellation signal to the download task
    if let Some(cancel_tx) = state.cancel_senders.lock().await.get(&download_id) {
        let _ = cancel_tx.send(true);
    }

    // Update the download status
    let mut downloads = state.downloads.lock().await;
    if let Some(download) = downloads.get_mut(&download_id) {
        download.status = "cancelled".to_string();
        let _ = app.emit("download-progress", download.clone());
    }

    // Clean up the cancel sender
    state.cancel_senders.lock().await.remove(&download_id);

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
pub async fn get_ytdlp_status(
    state: State<'_, Arc<AppState>>,
) -> Result<YtDlpStatus, String> {
    let channel = state.config.lock().await.ytdlp_channel.clone();
    Ok(YtDlpManager::check_status(&channel).await)
}

#[tauri::command]
pub async fn get_ytdlp_status_fast() -> Result<YtDlpStatus, String> {
    Ok(YtDlpManager::check_status_fast().await)
}

#[tauri::command]
pub async fn install_ytdlp(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let channel = state.config.lock().await.ytdlp_channel.clone();
    let app_clone = app.clone();

    let version = YtDlpManager::install(&channel, move |progress: InstallProgress| {
        let _ = app_clone.emit("ytdlp-install-progress", progress);
    })
    .await?;

    // Auto-install Deno if not already present (needed for YouTube EJS)
    if let Ok(deno_path) = DenoManager::get_binary_path() {
        if !deno_path.exists() {
            let app_for_deno = app.clone();
            let _ = DenoManager::install(move |progress: InstallProgress| {
                let _ = app_for_deno.emit("deno-install-progress", progress);
            })
            .await;
        }
    }

    Ok(version)
}

#[tauri::command]
pub async fn update_ytdlp(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let channel = state.config.lock().await.ytdlp_channel.clone();
    let app_clone = app.clone();

    let version = YtDlpManager::update(&channel, move |progress: InstallProgress| {
        let _ = app_clone.emit("ytdlp-install-progress", progress);
    })
    .await?;

    Ok(version)
}

#[tauri::command]
pub async fn check_ytdlp_update(
    state: State<'_, Arc<AppState>>,
) -> Result<Option<String>, String> {
    let channel = state.config.lock().await.ytdlp_channel.clone();
    let current = match YtDlpManager::get_installed_version().await {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };

    let latest = YtDlpManager::get_latest_version(&channel).await?;

    if current != latest {
        Ok(Some(latest))
    } else {
        Ok(None)
    }
}

// Whisper manager commands

#[tauri::command]
pub async fn get_whisper_status() -> Result<WhisperStatus, String> {
    Ok(WhisperManager::check_status().await)
}

#[tauri::command]
pub async fn install_whisper(app: AppHandle) -> Result<String, String> {
    let app_clone = app.clone();

    let version = WhisperManager::install(move |progress: WhisperInstallProgress| {
        let _ = app_clone.emit("whisper-install-progress", progress);
    })
    .await?;

    Ok(version)
}

#[tauri::command]
pub async fn download_whisper_model(app: AppHandle, model: String) -> Result<(), String> {
    let app_clone = app.clone();

    WhisperManager::download_model(&model, move |progress: WhisperInstallProgress| {
        let _ = app_clone.emit("whisper-install-progress", progress);
    })
    .await?;

    Ok(())
}

#[tauri::command]
pub async fn get_available_whisper_models() -> Result<Vec<WhisperModel>, String> {
    Ok(WhisperManager::get_available_models().await)
}

#[tauri::command]
pub async fn check_ffmpeg() -> Result<bool, String> {
    Ok(Whisper::check_ffmpeg().await)
}

// Transcription engine commands

#[tauri::command]
pub async fn get_transcription_engines() -> Result<Vec<EngineInfo>, String> {
    let manager = TranscriptionManager::new();
    Ok(manager.get_engines().await)
}

#[tauri::command]
pub async fn get_engine_models(engine_id: String) -> Result<Vec<TranscriptionModelInfo>, String> {
    let manager = TranscriptionManager::new();
    Ok(manager.get_engine_models(&engine_id).await)
}

#[tauri::command]
pub async fn install_transcription_engine(app: AppHandle, engine_id: String) -> Result<(), String> {
    let app_clone = app.clone();
    let manager = TranscriptionManager::new();

    manager
        .install_engine(&engine_id, move |progress: TranscriptionInstallProgress| {
            let _ = app_clone.emit("transcription-install-progress", &progress);
        })
        .await
}

#[tauri::command]
pub async fn download_transcription_model(
    app: AppHandle,
    engine_id: String,
    model_id: String,
) -> Result<(), String> {
    let app_clone = app.clone();
    let manager = TranscriptionManager::new();

    manager
        .download_model(&engine_id, &model_id, move |progress: TranscriptionInstallProgress| {
            let _ = app_clone.emit("transcription-install-progress", &progress);
        })
        .await
}

#[tauri::command]
pub async fn get_transcription_speed_multiplier(
    engine_id: String,
    model_id: String,
    use_gpu: bool,
) -> Result<f64, String> {
    let manager = TranscriptionManager::new();
    Ok(manager.get_speed_multiplier(&engine_id, &model_id, use_gpu))
}

// Local file transcription - unified with downloads system

/// Add a local file for transcription (creates a pending task)
#[tauri::command]
pub async fn add_local_transcription(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    file_path: String,
    title: String,
    engine: String,
    model: String,
    _style: String,
) -> Result<String, String> {
    let video_path = PathBuf::from(&file_path);

    if !video_path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    let task_id = Uuid::new_v4().to_string();

    let download = Download {
        id: task_id.clone(),
        url: String::new(),
        title,
        thumbnail: None,
        status: "pending".to_string(),
        progress: 0.0,
        speed: None,
        eta: None,
        output_path: Some(file_path.clone()),
        format: String::new(),
        error: None,
        duration: None,
        whisper_model: Some(model),
        transcription_engine: Some(engine),
        transcription_progress: None,
        transcription_message: None,
        task_type: "local_transcribe".to_string(),
        source_path: Some(file_path),
    };

    state.downloads.lock().await.insert(task_id.clone(), download.clone());
    let _ = app.emit("download-progress", download);

    Ok(task_id)
}

/// Start transcription for a pending local transcription task
#[tauri::command]
pub async fn start_local_transcription(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    task_id: String,
) -> Result<(), String> {
    // Get task info
    let (file_path, engine_id, model_id, style) = {
        let downloads = state.downloads.lock().await;
        let task = downloads.get(&task_id)
            .ok_or_else(|| format!("Task not found: {}", task_id))?;

        if task.task_type != "local_transcribe" {
            return Err("Not a local transcription task".to_string());
        }
        if task.status != "pending" {
            return Err(format!("Task is not pending, status: {}", task.status));
        }

        let source = task.source_path.clone()
            .ok_or_else(|| "No source path".to_string())?;
        let engine = task.transcription_engine.clone()
            .ok_or_else(|| "No engine specified".to_string())?;
        let model = task.whisper_model.clone()
            .ok_or_else(|| "No model specified".to_string())?;

        (source, engine, model, "sentence".to_string())  // TODO: store style in Download
    };

    let video_path = PathBuf::from(&file_path);

    // Create cancel channel
    let (cancel_tx, cancel_rx) = watch::channel(false);
    state.cancel_senders.lock().await.insert(task_id.clone(), cancel_tx);

    // Update status to transcribing
    {
        let mut downloads = state.downloads.lock().await;
        if let Some(download) = downloads.get_mut(&task_id) {
            download.status = "transcribing:extracting".to_string();
            let _ = app.emit("download-progress", download.clone());
        }
    }

    let state_clone = Arc::clone(&state.inner());
    let app_clone = app.clone();
    let task_id_clone = task_id.clone();

    tokio::spawn(async move {
        // Create progress channel for transcription
        let (transcribe_tx, mut transcribe_rx) = mpsc::channel::<TranscribeProgress>(100);

        let app_for_transcribe = app_clone.clone();
        let task_id_for_progress = task_id_clone.clone();
        let state_for_progress = state_clone.clone();

        // Spawn task to forward transcription progress
        tokio::spawn(async move {
            while let Some(progress) = transcribe_rx.recv().await {
                // Skip the "complete" stage - we handle completion in the main flow
                if progress.stage == "complete" {
                    continue;
                }
                let mut downloads = state_for_progress.downloads.lock().await;
                if let Some(download) = downloads.get_mut(&task_id_for_progress) {
                    // Don't overwrite if already completed
                    if download.status == "completed" {
                        continue;
                    }
                    download.status = format!("transcribing:{}", progress.stage);
                    download.transcription_progress = Some(progress.progress);
                    download.transcription_message = Some(progress.message.clone());
                    let _ = app_for_transcribe.emit("transcribe-progress", &progress);
                    let _ = app_for_transcribe.emit("download-progress", download.clone());
                }
            }
        });

        let transcription_manager = TranscriptionManager::new();

        log::info!(
            "Starting local file transcription for: {:?} with engine: {}, model: {}",
            video_path,
            engine_id,
            model_id
        );

        match transcription_manager
            .process_video(
                &video_path,
                &engine_id,
                &model_id,
                None, // Language is auto-detected
                &style,
                transcribe_tx,
                cancel_rx,
            )
            .await
        {
            Ok(result) => {
                log::info!("Local file transcription successful: {:?}", result);
                let mut downloads = state_clone.downloads.lock().await;
                if let Some(download) = downloads.get_mut(&task_id_clone) {
                    download.status = "completed".to_string();
                    download.progress = 100.0;
                    let _ = app_clone.emit("download-progress", download.clone());
                }
            }
            Err(e) => {
                log::error!("Local file transcription failed: {}", e);
                let mut downloads = state_clone.downloads.lock().await;
                if let Some(download) = downloads.get_mut(&task_id_clone) {
                    // Only set error status if not already cancelled
                    if download.status != "cancelled" {
                        download.status = "error".to_string();
                        download.error = Some(e);
                    }
                    let _ = app_clone.emit("download-progress", download.clone());
                }
            }
        }

        // Clean up cancel sender
        state_clone.cancel_senders.lock().await.remove(&task_id_clone);
    });

    Ok(())
}

/// Update transcription settings for a pending task
#[tauri::command]
pub async fn update_transcription_settings(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    task_id: String,
    engine: Option<String>,
    model: Option<String>,
    style: Option<String>,
) -> Result<(), String> {
    let mut downloads = state.downloads.lock().await;
    let download = downloads.get_mut(&task_id)
        .ok_or_else(|| format!("Task not found: {}", task_id))?;

    if download.status != "pending" {
        return Err("Can only update settings for pending tasks".to_string());
    }

    if let Some(e) = engine {
        download.transcription_engine = Some(e);
    }
    if let Some(m) = model {
        download.whisper_model = Some(m);
    }
    // Note: style is not currently stored in Download struct,
    // could add it in a future enhancement

    let _ = style; // Acknowledge unused for now

    let _ = app.emit("download-progress", download.clone());

    Ok(())
}

// Deno manager commands

#[tauri::command]
pub async fn get_deno_status() -> Result<DenoStatus, String> {
    Ok(DenoManager::check_status().await)
}

#[tauri::command]
pub async fn install_deno(app: AppHandle) -> Result<String, String> {
    let app_clone = app.clone();

    let version = DenoManager::install(move |progress: InstallProgress| {
        let _ = app_clone.emit("deno-install-progress", progress);
    })
    .await?;

    Ok(version)
}

// Network interface commands

#[tauri::command]
pub async fn list_network_interfaces() -> Result<Vec<NetworkInterface>, String> {
    network::get_network_interfaces()
}
