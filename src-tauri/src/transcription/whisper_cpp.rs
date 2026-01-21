use super::{InstallProgress, TranscribeProgress, TranscriptionEngine, TranscriptionModel};
use crate::whisper_manager::WhisperManager;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

/// Whisper.cpp transcription engine (legacy)
pub struct WhisperCppEngine;

impl WhisperCppEngine {
    pub fn new() -> Self {
        Self
    }

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

    /// Extract audio from video file to WAV format suitable for whisper
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
}

impl Default for WhisperCppEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TranscriptionEngine for WhisperCppEngine {
    fn id(&self) -> &'static str {
        "whisper_cpp"
    }

    fn name(&self) -> &'static str {
        "Whisper.cpp"
    }

    fn description(&self) -> &'static str {
        "Portable, multilingual (2-8x realtime)"
    }

    fn gpu_required(&self) -> bool {
        false
    }

    async fn check_gpu_available(&self) -> bool {
        // whisper.cpp works on CPU, GPU is optional
        true
    }

    async fn is_available(&self) -> Result<bool, String> {
        let binary_path = WhisperManager::get_binary_path()?;
        Ok(binary_path.exists())
    }

    async fn available_models(&self) -> Vec<TranscriptionModel> {
        let models_dir = WhisperManager::get_models_dir().unwrap_or_default();

        vec![
            TranscriptionModel {
                id: "tiny".to_string(),
                name: "Tiny".to_string(),
                size: "75 MB".to_string(),
                installed: models_dir.join("ggml-tiny.bin").exists(),
                speed_gpu: 12.0,
                speed_cpu: 8.0,
            },
            TranscriptionModel {
                id: "base".to_string(),
                name: "Base".to_string(),
                size: "142 MB".to_string(),
                installed: models_dir.join("ggml-base.bin").exists(),
                speed_gpu: 8.0,
                speed_cpu: 5.0,
            },
            TranscriptionModel {
                id: "small".to_string(),
                name: "Small".to_string(),
                size: "466 MB".to_string(),
                installed: models_dir.join("ggml-small.bin").exists(),
                speed_gpu: 4.0,
                speed_cpu: 2.5,
            },
        ]
    }

    fn speed_multiplier(&self, model: &str) -> (f64, f64) {
        match model {
            "tiny" => (12.0, 8.0),
            "base" => (8.0, 5.0),
            "small" => (4.0, 2.5),
            _ => (5.0, 3.0),
        }
    }

    fn supported_languages(&self) -> Vec<&'static str> {
        vec![
            "auto", "en", "es", "fr", "de", "it", "pt", "ru", "ja", "zh", "ko", "ar",
            "hi", "nl", "pl", "tr", "vi", "th", "id", "uk", "cs", "el", "he", "hu",
            "no", "da", "fi", "sv", "ro", "sk", "bg", "hr", "lt", "lv", "sl", "et",
            "ms", "tl", "sw", "af", "bn", "ta", "te", "mr", "gu", "kn", "ml", "pa",
            "ur", "fa", "ne", "si", "my", "ka", "am", "km", "lo", "cy", "eu", "gl",
            "ca", "is", "mk", "sr", "sq", "az", "kk", "uz", "mn", "bo", "ps", "sd",
            "ha", "yo", "ig", "so", "sn", "su", "jw", "yi", "ht", "mi", "la", "mt",
        ]
    }

    async fn install(&self, progress_callback: Box<dyn Fn(InstallProgress) + Send + 'static>) -> Result<(), String> {
        WhisperManager::install(move |p| {
            progress_callback(InstallProgress {
                downloaded: p.downloaded,
                total: p.total,
                percentage: p.percentage,
                stage: p.stage,
            });
        })
        .await?;
        Ok(())
    }

    async fn download_model(&self, model: &str, progress_callback: Box<dyn Fn(InstallProgress) + Send + 'static>) -> Result<(), String> {
        WhisperManager::download_model(model, move |p| {
            progress_callback(InstallProgress {
                downloaded: p.downloaded,
                total: p.total,
                percentage: p.percentage,
                stage: p.stage,
            });
        })
        .await
    }

    async fn transcribe(
        &self,
        audio_path: &Path,
        model: &str,
        language: Option<&str>,
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

        // Create temp directory for intermediate files
        let temp_dir = audio_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(".zinc_whisper_temp");
        fs::create_dir_all(&temp_dir)
            .await
            .map_err(|e| format!("Failed to create temp directory: {}", e))?;

        // Check if input is already a WAV file
        let is_wav = audio_path
            .extension()
            .map(|e| e.to_str().unwrap_or("").eq_ignore_ascii_case("wav"))
            .unwrap_or(false);

        let wav_path = if is_wav {
            audio_path.to_path_buf()
        } else {
            let wav_path = temp_dir.join("audio.wav");
            Self::extract_audio(audio_path, &wav_path, &progress_tx).await?;
            wav_path
        };

        let srt_base = temp_dir.join("output");

        // Run whisper transcription
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
        log::info!("WAV path: {:?}", wav_path);
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
        let lang = language.unwrap_or("auto");
        if lang != "auto" {
            args.push("-l");
            args.push(lang);
        }

        args.push(wav_path.to_str().unwrap_or(""));

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

        // Return generated SRT path
        let generated_srt = temp_dir.join("output.srt");
        if !generated_srt.exists() {
            // Clean up temp files
            let _ = fs::remove_dir_all(&temp_dir).await;
            return Err("Whisper did not generate SRT file".to_string());
        }

        // Verify SRT has content
        let srt_content = fs::read_to_string(&generated_srt)
            .await
            .map_err(|e| format!("Failed to read SRT file: {}", e))?;
        if srt_content.trim().is_empty() {
            let _ = fs::remove_dir_all(&temp_dir).await;
            return Err("Transcription produced no text. The audio may be silent, corrupted, or in an unsupported format.".to_string());
        }

        // Move SRT to final location (next to audio file)
        let final_srt = audio_path.with_extension("srt");
        fs::rename(&generated_srt, &final_srt)
            .await
            .map_err(|e| format!("Failed to move SRT file: {}", e))?;

        // Clean up temp files
        let _ = fs::remove_dir_all(&temp_dir).await;

        Ok(final_srt)
    }
}
