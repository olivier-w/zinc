use crate::transcription::{
    EngineInfo, EngineStatus, InstallProgress, TranscribeProgress, TranscriptionDispatcher,
    TranscriptionModel,
};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::mpsc;

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
            .transcribe(file_path, model_id, language, style, progress_tx)
            .await
    }

    /// Extract audio from video file to 16kHz mono WAV format (required by most transcription engines)
    async fn extract_audio(
        video_path: &Path,
        progress_tx: &mpsc::Sender<TranscribeProgress>,
    ) -> Result<PathBuf, String> {
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

        let output = cmd
            .output()
            .await
            .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::error!("Audio extraction failed: {}", stderr);
            return Err(format!("Audio extraction failed: {}", stderr));
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
    ) -> Result<PathBuf, String> {
        log::info!(
            "process_video called for: {:?} with engine: {}, model: {}, style: {}",
            video_path,
            engine_id,
            model_id,
            style
        );

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
        let audio_path = Self::extract_audio(video_path, &progress_tx).await?;

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

        // Step 2: Embed subtitles
        log::info!("Starting subtitle embedding...");
        Self::embed_subtitles(video_path, &srt_path, &output_path, &progress_tx).await?;
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

    /// Embed SRT subtitles into video file
    async fn embed_subtitles(
        video_path: &Path,
        srt_path: &Path,
        output_path: &Path,
        progress_tx: &mpsc::Sender<TranscribeProgress>,
    ) -> Result<PathBuf, String> {
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

        let mut cmd = Command::new(if cfg!(target_os = "windows") {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });

        if needs_conversion {
            cmd.args([
                "-i",
                video_path.to_str().unwrap_or(""),
                "-i",
                srt_path.to_str().unwrap_or(""),
                "-c:v",
                "copy",
                "-c:a",
                "copy",
                "-c:s",
                subtitle_codec,
                "-metadata:s:s:0",
                "language=eng",
                "-y",
                output_path.to_str().unwrap_or(""),
            ]);
        } else {
            cmd.args([
                "-i",
                video_path.to_str().unwrap_or(""),
                "-i",
                srt_path.to_str().unwrap_or(""),
                "-c",
                "copy",
                "-c:s",
                subtitle_codec,
                "-metadata:s:s:0",
                "language=eng",
                "-y",
                output_path.to_str().unwrap_or(""),
            ]);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);

        log::info!("Running ffmpeg for subtitle embedding...");

        let output = cmd
            .output()
            .await
            .map_err(|e| format!("Failed to execute ffmpeg: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::error!("ffmpeg muxing failed: {}", stderr);
            return Err(format!("ffmpeg muxing error: {}", stderr));
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
