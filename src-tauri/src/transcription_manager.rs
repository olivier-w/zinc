use crate::transcription::{
    EngineInfo, EngineStatus, InstallProgress, TranscribeProgress, TranscriptionDispatcher,
    TranscriptionModel,
};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::{mpsc, watch};

/// Manages all transcription engines and provides a unified API
pub struct TranscriptionManager {
    dispatcher: TranscriptionDispatcher,
}

impl TranscriptionManager {
    pub fn new() -> Self {
        Self {
            dispatcher: TranscriptionDispatcher::new(),
        }
    }

    /// Get info for all engines
    pub async fn get_engines(&self) -> Vec<EngineInfo> {
        self.dispatcher.get_engine_infos().await
    }

    /// Get info for a specific engine
    #[allow(dead_code)]
    pub async fn get_engine_info(&self, engine_id: &str) -> Option<EngineInfo> {
        match self.dispatcher.get_engine(engine_id) {
            Some(engine) => Some(engine.get_info().await),
            None => None,
        }
    }

    /// Get available models for an engine
    pub async fn get_engine_models(&self, engine_id: &str) -> Vec<TranscriptionModel> {
        match self.dispatcher.get_engine(engine_id) {
            Some(engine) => engine.available_models().await,
            None => vec![],
        }
    }

    /// Install an engine
    pub async fn install_engine<F>(
        &self,
        engine_id: &str,
        progress_callback: F,
    ) -> Result<(), String>
    where
        F: Fn(InstallProgress) + Send + 'static,
    {
        let engine = self
            .dispatcher
            .get_engine(engine_id)
            .ok_or_else(|| format!("Engine '{}' not found", engine_id))?;

        engine.install(Box::new(progress_callback)).await
    }

    /// Download a model for an engine
    pub async fn download_model<F>(
        &self,
        engine_id: &str,
        model_id: &str,
        progress_callback: F,
    ) -> Result<(), String>
    where
        F: Fn(InstallProgress) + Send + 'static,
    {
        let engine = self
            .dispatcher
            .get_engine(engine_id)
            .ok_or_else(|| format!("Engine '{}' not found", engine_id))?;

        engine.download_model(model_id, Box::new(progress_callback)).await
    }

    /// Check if ffmpeg is available
    pub async fn check_ffmpeg() -> bool {
        let mut cmd = Command::new(if cfg!(target_os = "windows") {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });
        cmd.arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);

        cmd.status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Transcribe a video/audio file
    pub async fn transcribe(
        &self,
        file_path: &Path,
        engine_id: &str,
        model_id: &str,
        language: Option<&str>,
        style: &str,
        progress_tx: mpsc::Sender<TranscribeProgress>,
        cancel_rx: watch::Receiver<bool>,
    ) -> Result<PathBuf, String> {
        let engine = self
            .dispatcher
            .get_engine(engine_id)
            .ok_or_else(|| format!("Engine '{}' not found", engine_id))?;

        // Verify engine is available
        if !engine.is_available().await.unwrap_or(false) {
            let info = engine.get_info().await;
            return Err(match info.status {
                EngineStatus::NotInstalled => {
                    format!("Engine '{}' is not installed", engine_id)
                }
                EngineStatus::Unavailable { reason } => {
                    format!("Engine '{}' is unavailable: {}", engine_id, reason)
                }
                _ => format!("Engine '{}' is not available", engine_id),
            });
        }

        // Run transcription
        engine
            .transcribe(file_path, model_id, language, style, progress_tx, cancel_rx)
            .await
    }

    /// Extract audio from video file to 16kHz mono WAV format (required by most transcription engines)
    async fn extract_audio(
        video_path: &Path,
        progress_tx: &mpsc::Sender<TranscribeProgress>,
        cancel_rx: &watch::Receiver<bool>,
    ) -> Result<PathBuf, String> {
        // Check for cancellation before starting
        if *cancel_rx.borrow() {
            return Err("Cancelled".to_string());
        }

        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "extracting".to_string(),
                progress: 0.0,
                message: "Extracting audio...".to_string(),
            })
            .await;

        // Create temp directory in Downloads folder for audio extraction
        let video_dir = video_path.parent().unwrap_or(Path::new("."));
        let temp_dir = video_dir.join(".zinc_temp");
        fs::create_dir_all(&temp_dir)
            .await
            .map_err(|e| format!("Failed to create temp directory: {}", e))?;

        let audio_path = temp_dir.join("audio.wav");

        // Extract audio using ffmpeg: 16kHz mono WAV (required by sherpa-onnx and whisper)
        let mut cmd = Command::new(if cfg!(target_os = "windows") {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });

        cmd.args([
            "-i",
            video_path.to_str().unwrap_or(""),
            "-vn",           // No video
            "-acodec", "pcm_s16le",  // PCM 16-bit little-endian
            "-ar", "16000",  // 16kHz sample rate
            "-ac", "1",      // Mono
            "-y",            // Overwrite output file
            audio_path.to_str().unwrap_or(""),
        ]);

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

        log::info!("Extracting audio from {:?} to {:?}", video_path, audio_path);

        // Spawn the process and monitor for cancellation
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

        let mut cancel_rx_clone = cancel_rx.clone();

        // Wait for process completion or cancellation
        tokio::select! {
            result = child.wait() => {
                let status = result.map_err(|e| format!("Failed to wait for ffmpeg: {}", e))?;
                if !status.success() {
                    // Read stderr for error message
                    return Err("Audio extraction failed".to_string());
                }
            }
            _ = cancel_rx_clone.changed() => {
                if *cancel_rx_clone.borrow() {
                    // Kill the process
                    let _ = child.kill().await;
                    // Clean up temp files
                    let _ = fs::remove_file(&audio_path).await;
                    let _ = fs::remove_dir(&temp_dir).await;
                    return Err("Cancelled".to_string());
                }
            }
        }

        log::info!("Audio extraction complete: {:?}", audio_path);

        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "extracting".to_string(),
                progress: 100.0,
                message: "Audio extracted".to_string(),
            })
            .await;

        Ok(audio_path)
    }

    /// Full pipeline: transcribe video and embed subtitles
    pub async fn process_video(
        &self,
        video_path: &Path,
        engine_id: &str,
        model_id: &str,
        language: Option<&str>,
        style: &str,
        progress_tx: mpsc::Sender<TranscribeProgress>,
        cancel_rx: watch::Receiver<bool>,
    ) -> Result<PathBuf, String> {
        log::info!(
            "process_video called for: {:?} with engine: {}, model: {}, style: {}",
            video_path,
            engine_id,
            model_id,
            style
        );

        // Check for cancellation
        if *cancel_rx.borrow() {
            return Err("Cancelled".to_string());
        }

        // Check ffmpeg availability
        if !Self::check_ffmpeg().await {
            return Err("ffmpeg is required for subtitle generation but was not found. Please install ffmpeg.".to_string());
        }

        let video_dir = video_path.parent().unwrap_or(Path::new("."));
        let video_stem = video_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("video");
        let video_ext = video_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4");

        // Generate SRT path
        let srt_path = video_dir.join(format!("{}.srt", video_stem));

        // Generate output path (video with subtitles)
        let output_path = video_dir.join(format!("{}_subtitled.{}", video_stem, video_ext));

        log::info!("SRT path: {:?}, Output path: {:?}", srt_path, output_path);

        // Step 1: Extract audio from video (16kHz mono WAV)
        let audio_path = Self::extract_audio(video_path, &progress_tx, &cancel_rx).await?;

        // Check for cancellation before transcription
        if *cancel_rx.borrow() {
            // Clean up temp audio file
            let _ = fs::remove_file(&audio_path).await;
            let temp_dir = audio_path.parent().unwrap_or(Path::new("."));
            let _ = fs::remove_dir(temp_dir).await;
            return Err("Cancelled".to_string());
        }

        // Step 2: Transcribe
        log::info!(
            "Starting transcription with engine: {}, model: {}, language: {:?}, style: {}",
            engine_id,
            model_id,
            language,
            style
        );

        let generated_srt = self
            .transcribe(
                &audio_path,
                engine_id,
                model_id,
                language,
                style,
                progress_tx.clone(),
                cancel_rx.clone(),
            )
            .await;

        // Clean up temp audio file regardless of result
        let temp_dir = audio_path.parent().unwrap_or(Path::new("."));
        let _ = fs::remove_file(&audio_path).await;
        let _ = fs::remove_dir(temp_dir).await; // Only succeeds if empty

        let generated_srt = generated_srt?;

        // Move generated SRT to expected location if different
        if generated_srt != srt_path {
            fs::rename(&generated_srt, &srt_path)
                .await
                .map_err(|e| format!("Failed to move SRT file: {}", e))?;
        }

        log::info!(
            "Transcription complete, SRT exists: {}",
            srt_path.exists()
        );

        // Check for cancellation before embedding
        if *cancel_rx.borrow() {
            let _ = fs::remove_file(&srt_path).await;
            return Err("Cancelled".to_string());
        }

        // Step 2: Embed subtitles
        log::info!("Starting subtitle embedding...");
        Self::embed_subtitles(video_path, &srt_path, &output_path, language, &progress_tx, &cancel_rx).await?;
        log::info!(
            "Embedding complete, output exists: {}",
            output_path.exists()
        );

        // Step 3: Replace original with subtitled version
        log::info!("Replacing original with subtitled version...");
        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "finalizing".to_string(),
                progress: 0.0,
                message: "Finalizing...".to_string(),
            })
            .await;

        // Rename: original -> backup, subtitled -> original
        let backup_path = video_dir.join(format!("{}_original.{}", video_stem, video_ext));
        fs::rename(video_path, &backup_path)
            .await
            .map_err(|e| format!("Failed to backup original: {}", e))?;

        fs::rename(&output_path, video_path)
            .await
            .map_err(|e| format!("Failed to replace with subtitled version: {}", e))?;

        // Delete backup
        let _ = fs::remove_file(&backup_path).await;

        // Delete SRT file (subtitles are now embedded in video)
        let _ = fs::remove_file(&srt_path).await;

        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "complete".to_string(),
                progress: 100.0,
                message: "Subtitles added".to_string(),
            })
            .await;

        Ok(video_path.to_path_buf())
    }

    /// Convert ISO 639-1 language code to ISO 639-2 (3-letter) code and full name
    fn get_language_metadata(language: Option<&str>) -> (&'static str, &'static str) {
        match language {
            Some("en") | Some("auto") | None => ("eng", "English"),
            Some("es") => ("spa", "Spanish"),
            Some("fr") => ("fra", "French"),
            Some("de") => ("deu", "German"),
            Some("it") => ("ita", "Italian"),
            Some("pt") => ("por", "Portuguese"),
            Some("ru") => ("rus", "Russian"),
            Some("zh") => ("zho", "Chinese"),
            Some("ja") => ("jpn", "Japanese"),
            Some("ko") => ("kor", "Korean"),
            Some("ar") => ("ara", "Arabic"),
            Some("hi") => ("hin", "Hindi"),
            Some("nl") => ("nld", "Dutch"),
            Some("pl") => ("pol", "Polish"),
            Some("tr") => ("tur", "Turkish"),
            Some("vi") => ("vie", "Vietnamese"),
            Some("th") => ("tha", "Thai"),
            Some("id") => ("ind", "Indonesian"),
            Some("uk") => ("ukr", "Ukrainian"),
            Some("cs") => ("ces", "Czech"),
            Some("sv") => ("swe", "Swedish"),
            Some("da") => ("dan", "Danish"),
            Some("fi") => ("fin", "Finnish"),
            Some("no") => ("nor", "Norwegian"),
            Some("he") => ("heb", "Hebrew"),
            Some("el") => ("ell", "Greek"),
            Some("hu") => ("hun", "Hungarian"),
            Some("ro") => ("ron", "Romanian"),
            Some("sk") => ("slk", "Slovak"),
            Some("bg") => ("bul", "Bulgarian"),
            Some("hr") => ("hrv", "Croatian"),
            Some("sr") => ("srp", "Serbian"),
            Some("sl") => ("slv", "Slovenian"),
            Some("et") => ("est", "Estonian"),
            Some("lv") => ("lav", "Latvian"),
            Some("lt") => ("lit", "Lithuanian"),
            Some("ms") => ("msa", "Malay"),
            Some("tl") => ("tgl", "Tagalog"),
            // Default to English for unrecognized codes
            Some(_) => ("eng", "English"),
        }
    }

    /// Embed SRT subtitles into video file
    async fn embed_subtitles(
        video_path: &Path,
        srt_path: &Path,
        output_path: &Path,
        language: Option<&str>,
        progress_tx: &mpsc::Sender<TranscribeProgress>,
        cancel_rx: &watch::Receiver<bool>,
    ) -> Result<PathBuf, String> {
        // Check for cancellation before starting
        if *cancel_rx.borrow() {
            return Err("Cancelled".to_string());
        }

        log::info!(
            "embed_subtitles called: video={:?}, srt={:?}, output={:?}",
            video_path,
            srt_path,
            output_path
        );

        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "embedding".to_string(),
                progress: 0.0,
                message: "Embedding subtitles...".to_string(),
            })
            .await;

        // Determine subtitle codec based on output format
        let ext = output_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4")
            .to_lowercase();

        // WebM only supports WebVTT subtitles
        // MKV supports SRT
        // MP4 supports mov_text
        let (subtitle_codec, needs_conversion) = match ext.as_str() {
            "webm" => ("webvtt", true),
            "mkv" => ("srt", false),
            _ => ("mov_text", false),
        };

        // Get language metadata for the new subtitle stream
        let (lang_code, lang_title) = Self::get_language_metadata(language);

        let mut cmd = Command::new(if cfg!(target_os = "windows") {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });

        // Build the metadata argument for the new subtitle stream
        let lang_metadata = format!("language={}", lang_code);
        let title_metadata = format!("title={}", lang_title);

        if needs_conversion {
            // WebM: map video, audio, existing subs from input 0, then new sub from input 1
            // All subtitles need to be webvtt for WebM container
            // Map streams explicitly: video, audio, then new subtitle first (so it's s:0), then existing subs
            cmd.args([
                "-i",
                video_path.to_str().unwrap_or(""),
                "-i",
                srt_path.to_str().unwrap_or(""),
                "-map", "0:v?",        // Video from original (optional - may not exist)
                "-map", "0:a?",        // Audio from original (optional - may not exist)
                "-map", "1:s",         // New subtitle FIRST (becomes s:0)
                "-map", "0:s?",        // Existing subtitles after (optional)
                "-c:v", "copy",
                "-c:a", "copy",
                "-c:s", subtitle_codec, // All subtitles to webvtt (required for WebM)
                // Metadata for the new subtitle stream (now at index s:0)
                "-metadata:s:s:0", &lang_metadata,
                "-metadata:s:s:0", &title_metadata,
                "-y",
                output_path.to_str().unwrap_or(""),
            ]);
        } else {
            // MKV/MP4: map all streams and add new subtitle
            // Existing subs can be copied, new SRT needs encoding to container format
            // Map streams explicitly: video, audio, then new subtitle first (so it's s:0), then existing subs
            cmd.args([
                "-i",
                video_path.to_str().unwrap_or(""),
                "-i",
                srt_path.to_str().unwrap_or(""),
                "-map", "0:v?",        // Video from original (optional)
                "-map", "0:a?",        // Audio from original (optional)
                "-map", "1:s",         // New subtitle FIRST (becomes s:0)
                "-map", "0:s?",        // Existing subtitles after (optional)
                "-c", "copy",          // Copy all streams by default
                "-c:s", subtitle_codec, // Encode all subtitles to container format
                // Metadata for the new subtitle stream (now at index s:0)
                "-metadata:s:s:0", &lang_metadata,
                "-metadata:s:s:0", &title_metadata,
                "-y",
                output_path.to_str().unwrap_or(""),
            ]);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);

        log::info!("Running ffmpeg for subtitle embedding...");

        // Spawn the process and monitor for cancellation
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to execute ffmpeg: {}", e))?;

        let mut cancel_rx_clone = cancel_rx.clone();

        // Wait for process completion or cancellation
        tokio::select! {
            result = child.wait() => {
                let status = result.map_err(|e| format!("Failed to wait for ffmpeg: {}", e))?;
                if !status.success() {
                    return Err("ffmpeg muxing failed".to_string());
                }
            }
            _ = cancel_rx_clone.changed() => {
                if *cancel_rx_clone.borrow() {
                    // Kill the process
                    let _ = child.kill().await;
                    // Clean up partial output
                    let _ = fs::remove_file(output_path).await;
                    return Err("Cancelled".to_string());
                }
            }
        }

        log::info!("ffmpeg muxing successful");

        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "embedding".to_string(),
                progress: 100.0,
                message: "Subtitles embedded".to_string(),
            })
            .await;

        Ok(output_path.to_path_buf())
    }

    /// Get speed multiplier for ETA calculation
    pub fn get_speed_multiplier(&self, engine_id: &str, model_id: &str, use_gpu: bool) -> f64 {
        if let Some(engine) = self.dispatcher.get_engine(engine_id) {
            let (gpu_speed, cpu_speed) = engine.speed_multiplier(model_id);
            if use_gpu {
                gpu_speed
            } else {
                cpu_speed
            }
        } else {
            5.0 // Default fallback
        }
    }
}

impl Default for TranscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}
