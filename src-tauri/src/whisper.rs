// Legacy whisper implementation - mostly superseded by transcription system
// Only check_ffmpeg() is currently used

use crate::whisper_manager::WhisperManager;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::fs;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscribeProgress {
    pub stage: String,
    pub progress: f64,
    pub message: String,
}

#[allow(dead_code)]
pub struct Whisper;

impl Whisper {
    #[allow(dead_code)]
    fn get_command() -> PathBuf {
        // Try managed binary first
        if let Ok(path) = WhisperManager::get_binary_path() {
            if path.exists() {
                return path;
            }
        }
        // Fall back to system PATH
        PathBuf::from(if cfg!(target_os = "windows") {
            "whisper.exe"
        } else {
            "whisper"
        })
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

    /// Extract audio from video file to WAV format suitable for whisper
    #[allow(dead_code)]
    async fn extract_audio(
        video_path: &Path,
        output_wav: &Path,
        progress_tx: &mpsc::Sender<TranscribeProgress>,
    ) -> Result<(), String> {
        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "extracting".to_string(),
                progress: 0.0,
                message: "Extracting audio...".to_string(),
            })
            .await;

        let mut cmd = Command::new(if cfg!(target_os = "windows") {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });

        cmd.args([
            "-i",
            video_path.to_str().unwrap_or(""),
            "-ar",
            "16000", // 16kHz sample rate required by whisper
            "-ac",
            "1", // Mono
            "-c:a",
            "pcm_s16le", // 16-bit PCM
            "-y", // Overwrite output
            output_wav.to_str().unwrap_or(""),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);

        let output = cmd
            .output()
            .await
            .map_err(|e| format!("Failed to execute ffmpeg: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("ffmpeg error: {}", stderr));
        }

        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "extracting".to_string(),
                progress: 100.0,
                message: "Audio extracted".to_string(),
            })
            .await;

        Ok(())
    }

    /// Embed SRT subtitles into video file
    #[allow(dead_code)]
    pub async fn embed_subtitles(
        video_path: &Path,
        srt_path: &Path,
        output_path: &Path,
        progress_tx: &mpsc::Sender<TranscribeProgress>,
    ) -> Result<PathBuf, String> {
        log::info!("embed_subtitles called: video={:?}, srt={:?}, output={:?}", video_path, srt_path, output_path);

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
            "webm" => ("webvtt", true), // Need to convert SRT to WebVTT
            "mkv" => ("srt", false),
            _ => ("mov_text", false), // For MP4 and others
        };

        let mut cmd = Command::new(if cfg!(target_os = "windows") {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });

        if needs_conversion {
            // For WebM, we need to convert SRT to WebVTT during muxing
            cmd.args([
                "-i",
                video_path.to_str().unwrap_or(""),
                "-i",
                srt_path.to_str().unwrap_or(""),
                "-c:v",
                "copy", // Copy video stream
                "-c:a",
                "copy", // Copy audio stream
                "-c:s",
                subtitle_codec,
                "-metadata:s:s:0",
                "language=eng",
                "-y", // Overwrite output
                output_path.to_str().unwrap_or(""),
            ]);
        } else {
            cmd.args([
                "-i",
                video_path.to_str().unwrap_or(""),
                "-i",
                srt_path.to_str().unwrap_or(""),
                "-c",
                "copy", // Copy video and audio streams
                "-c:s",
                subtitle_codec,
                "-metadata:s:s:0",
                "language=eng",
                "-y", // Overwrite output
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

    /// Transcribe audio and generate SRT file
    #[allow(dead_code)]
    pub async fn transcribe(
        video_path: &Path,
        output_srt_path: &Path,
        model: &str,
        language: &str, // "auto" or language code like "en", "es"
        progress_tx: mpsc::Sender<TranscribeProgress>,
    ) -> Result<PathBuf, String> {
        // Verify model exists
        let model_path = WhisperManager::get_model_path(model)?;
        if !model_path.exists() {
            return Err(format!(
                "Model '{}' not found. Please download it first.",
                model
            ));
        }

        // Check ffmpeg availability
        if !Self::check_ffmpeg().await {
            return Err("ffmpeg is required for subtitle generation but was not found. Please install ffmpeg.".to_string());
        }

        // Create temp directory for intermediate files
        let temp_dir = video_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(".zinc_temp");
        fs::create_dir_all(&temp_dir)
            .await
            .map_err(|e| format!("Failed to create temp directory: {}", e))?;

        let audio_path = temp_dir.join("audio.wav");
        let srt_base = temp_dir.join("output");

        // Step 1: Extract audio
        Self::extract_audio(video_path, &audio_path, &progress_tx).await?;

        // Step 2: Run whisper transcription
        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "transcribing".to_string(),
                progress: 0.0,
                message: "Transcribing audio...".to_string(),
            })
            .await;

        let whisper_cmd = Self::get_command();
        log::info!("Whisper command path: {:?}", whisper_cmd);
        log::info!("Model path: {:?}", model_path);
        log::info!("Audio path: {:?}", audio_path);
        log::info!("Output base: {:?}", srt_base);

        let mut cmd = Command::new(&whisper_cmd);
        let mut args = vec![
            "-m",
            model_path.to_str().unwrap_or(""),
            "-osrt", // Output SRT format
            "-of",
            srt_base.to_str().unwrap_or("output"),
        ];

        // Add language flag if not "auto"
        if language != "auto" {
            args.push("-l");
            args.push(language);
        }

        args.push(audio_path.to_str().unwrap_or(""));

        cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to start whisper: {}", e))?;

        log::info!("Whisper process spawned");

        let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let mut stderr_reader = BufReader::new(stderr).lines();
        let mut stdout_reader = BufReader::new(stdout).lines();

        // Collect stderr output for error reporting
        let stderr_lines = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
        let stderr_lines_clone = stderr_lines.clone();

        // Monitor whisper output for progress
        let progress_tx_clone = progress_tx.clone();
        tokio::spawn(async move {
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                log::debug!("whisper stderr: {}", line);
                stderr_lines_clone.lock().await.push(line.clone());
                // whisper.cpp outputs progress like: "whisper_print_progress_callback: progress = 42%"
                if line.contains("progress") {
                    if let Some(pct_str) = line.split('=').last() {
                        if let Ok(pct) = pct_str.trim().trim_end_matches('%').parse::<f64>() {
                            let _ = progress_tx_clone
                                .send(TranscribeProgress {
                                    stage: "transcribing".to_string(),
                                    progress: pct,
                                    message: format!("Transcribing... {}%", pct as i32),
                                })
                                .await;
                        }
                    }
                }
            }
        });

        // Also capture stdout
        tokio::spawn(async move {
            while let Ok(Some(line)) = stdout_reader.next_line().await {
                log::debug!("whisper stdout: {}", line);
            }
        });

        let status = child
            .wait()
            .await
            .map_err(|e| format!("Failed to wait for whisper: {}", e))?;

        if !status.success() {
            let stderr_output = stderr_lines.lock().await.join("\n");
            log::error!("Whisper transcription failed with status: {:?}", status);
            log::error!("Whisper stderr output: {}", stderr_output);
            // Clean up temp files
            let _ = fs::remove_dir_all(&temp_dir).await;
            return Err(format!("Whisper transcription failed: {}", stderr_output));
        }

        log::info!("Whisper completed successfully");

        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "transcribing".to_string(),
                progress: 100.0,
                message: "Transcription complete".to_string(),
            })
            .await;

        // Move generated SRT to final location
        let generated_srt = temp_dir.join("output.srt");
        if !generated_srt.exists() {
            // Clean up temp files
            let _ = fs::remove_dir_all(&temp_dir).await;
            return Err("Whisper did not generate SRT file".to_string());
        }

        fs::rename(&generated_srt, output_srt_path)
            .await
            .map_err(|e| format!("Failed to move SRT file: {}", e))?;

        // Clean up temp files
        let _ = fs::remove_dir_all(&temp_dir).await;

        Ok(output_srt_path.to_path_buf())
    }

    /// Full pipeline: transcribe video and embed subtitles
    #[allow(dead_code)]
    pub async fn process_video(
        video_path: &Path,
        model: &str,
        language: &str, // "auto" or language code like "en", "es"
        progress_tx: mpsc::Sender<TranscribeProgress>,
    ) -> Result<PathBuf, String> {
        log::info!("process_video called for: {:?}", video_path);

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

        // Step 1: Transcribe
        log::info!("Starting transcription with model: {}, language: {}", model, language);
        Self::transcribe(video_path, &srt_path, model, language, progress_tx.clone()).await?;
        log::info!("Transcription complete, SRT exists: {}", srt_path.exists());

        // Step 2: Embed subtitles
        log::info!("Starting subtitle embedding...");
        Self::embed_subtitles(video_path, &srt_path, &output_path, &progress_tx).await?;
        log::info!("Embedding complete, output exists: {}", output_path.exists());

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
}
