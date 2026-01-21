use super::{InstallProgress, TranscribeProgress, TranscriptionEngine, TranscriptionModel};
use crate::sherpa_manager::SherpaManager;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::mpsc;

/// Model download URLs from sherpa-onnx releases
const MOONSHINE_TINY_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-moonshine-tiny-en-int8.tar.bz2";
const MOONSHINE_BASE_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-moonshine-base-en-int8.tar.bz2";

/// Moonshine transcription engine using sherpa-onnx CLI
/// Fast, edge-optimized engine using ONNX Runtime
pub struct MoonshineEngine;

impl MoonshineEngine {
    pub fn new() -> Self {
        Self
    }

    /// Get the model directory name for a model ID
    fn get_model_dir_name(model: &str) -> &'static str {
        match model {
            "tiny" => "sherpa-onnx-moonshine-tiny-en-int8",
            "base" => "sherpa-onnx-moonshine-base-en-int8",
            _ => "sherpa-onnx-moonshine-tiny-en-int8",
        }
    }

    /// Get the download URL for a model
    fn get_model_url(model: &str) -> &'static str {
        match model {
            "tiny" => MOONSHINE_TINY_URL,
            "base" => MOONSHINE_BASE_URL,
            _ => MOONSHINE_TINY_URL,
        }
    }

    /// Get the models directory for Moonshine
    fn get_models_dir() -> Result<PathBuf, String> {
        SherpaManager::get_models_dir("moonshine")
    }

    /// Check if a model is installed
    fn is_model_installed(model: &str) -> bool {
        if let Ok(models_dir) = Self::get_models_dir() {
            let model_dir = models_dir.join(Self::get_model_dir_name(model));
            // Check for the tokens file as indicator that model is complete
            model_dir.join("tokens.txt").exists()
        } else {
            false
        }
    }

    /// Get the model configuration paths
    fn get_model_paths(model: &str) -> Result<(PathBuf, PathBuf, PathBuf, PathBuf, PathBuf), String> {
        let models_dir = Self::get_models_dir()?;
        let model_dir = models_dir.join(Self::get_model_dir_name(model));

        if !model_dir.exists() {
            return Err(format!("Model '{}' is not installed", model));
        }

        Ok((
            model_dir.join("preprocess.onnx"),
            model_dir.join("encode.int8.onnx"),
            model_dir.join("uncached_decode.int8.onnx"),
            model_dir.join("cached_decode.int8.onnx"),
            model_dir.join("tokens.txt"),
        ))
    }
}

impl Default for MoonshineEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TranscriptionEngine for MoonshineEngine {
    fn id(&self) -> &'static str {
        "moonshine"
    }

    fn name(&self) -> &'static str {
        "Moonshine"
    }

    fn description(&self) -> &'static str {
        "Fast ONNX engine (5-15x realtime)"
    }

    fn gpu_required(&self) -> bool {
        false
    }

    async fn check_gpu_available(&self) -> bool {
        true // Moonshine works on CPU and GPU
    }

    async fn is_available(&self) -> Result<bool, String> {
        // Check if sherpa-onnx is installed AND at least one model is installed
        let sherpa_installed = SherpaManager::is_installed().await;
        let has_model = Self::is_model_installed("tiny") || Self::is_model_installed("base");
        Ok(sherpa_installed && has_model)
    }

    async fn available_models(&self) -> Vec<TranscriptionModel> {
        vec![
            TranscriptionModel {
                id: "tiny".to_string(),
                name: "Tiny (int8)".to_string(),
                size: "35 MB".to_string(),
                installed: Self::is_model_installed("tiny"),
                speed_gpu: 50.0,
                speed_cpu: 15.0,
            },
            TranscriptionModel {
                id: "base".to_string(),
                name: "Base (int8)".to_string(),
                size: "70 MB".to_string(),
                installed: Self::is_model_installed("base"),
                speed_gpu: 30.0,
                speed_cpu: 10.0,
            },
        ]
    }

    fn speed_multiplier(&self, model: &str) -> (f64, f64) {
        match model {
            "tiny" => (50.0, 15.0),
            "base" => (30.0, 10.0),
            _ => (30.0, 10.0),
        }
    }

    fn supported_languages(&self) -> Vec<&'static str> {
        // Moonshine currently only supports English
        vec!["en"]
    }

    async fn install(
        &self,
        progress_callback: Box<dyn Fn(InstallProgress) + Send + 'static>,
    ) -> Result<(), String> {
        // Install sherpa-onnx runtime
        if !SherpaManager::is_installed().await {
            SherpaManager::install(progress_callback).await?;
        }
        Ok(())
    }

    async fn download_model(
        &self,
        model: &str,
        progress_callback: Box<dyn Fn(InstallProgress) + Send + 'static>,
    ) -> Result<(), String> {
        // Auto-install sherpa-onnx if not installed
        if !SherpaManager::is_installed().await {
            log::info!("sherpa-onnx not installed, installing automatically...");
            // For the install step, we create a no-op callback since we can't share the callback
            // The main download will still show progress
            SherpaManager::install(Box::new(move |progress| {
                log::info!("Installing sherpa-onnx: {}% - {}", progress.percentage as i32, progress.stage);
            })).await?;
        }

        let url = Self::get_model_url(model);
        let model_dir_name = Self::get_model_dir_name(model);

        SherpaManager::download_model("moonshine", url, model_dir_name, progress_callback).await?;

        Ok(())
    }

    async fn transcribe(
        &self,
        audio_path: &Path,
        model: &str,
        _language: Option<&str>,
        progress_tx: mpsc::Sender<TranscribeProgress>,
    ) -> Result<PathBuf, String> {
        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "preparing".to_string(),
                progress: 0.0,
                message: "Loading Moonshine model...".to_string(),
            })
            .await;

        // Get model paths
        let (preprocessor, encoder, uncached_decoder, cached_decoder, tokens) =
            Self::get_model_paths(model)?;

        // Verify all files exist
        for (name, path) in [
            ("preprocessor", &preprocessor),
            ("encoder", &encoder),
            ("uncached_decoder", &uncached_decoder),
            ("cached_decoder", &cached_decoder),
            ("tokens", &tokens),
        ] {
            if !path.exists() {
                return Err(format!(
                    "Model file '{}' not found at {:?}. Please download the model first.",
                    name, path
                ));
            }
        }

        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "transcribing".to_string(),
                progress: 10.0,
                message: "Running transcription...".to_string(),
            })
            .await;

        // Generate output SRT path
        let srt_path = audio_path.with_extension("srt");

        // Use Python script for transcription (handles audio chunking for long files)
        // The Moonshine model has context length limits and produces empty output on long audio
        // when processed in one shot. The Python script chunks audio into 30-second segments.
        let bin_dir = SherpaManager::get_bin_dir()?;
        let script_path = bin_dir.join("transcribe_moonshine.py");
        let script_content = include_str!("../../resources/transcribe_moonshine.py");
        fs::write(&script_path, script_content)
            .await
            .map_err(|e| format!("Failed to write Python script: {}", e))?;

        // Use forward slashes for paths on Windows for compatibility
        let preprocessor_str = preprocessor.to_str().unwrap().replace('\\', "/");
        let encoder_str = encoder.to_str().unwrap().replace('\\', "/");
        let uncached_decoder_str = uncached_decoder.to_str().unwrap().replace('\\', "/");
        let cached_decoder_str = cached_decoder.to_str().unwrap().replace('\\', "/");
        let tokens_str = tokens.to_str().unwrap().replace('\\', "/");
        let audio_str = audio_path.to_str().unwrap().replace('\\', "/");

        let mut cmd = Command::new("python");
        cmd.args([
            script_path.to_str().unwrap(),
            &format!("--preprocessor={}", preprocessor_str),
            &format!("--encoder={}", encoder_str),
            &format!("--uncached-decoder={}", uncached_decoder_str),
            &format!("--cached-decoder={}", cached_decoder_str),
            &format!("--tokens={}", tokens_str),
            "--num-threads=4",
            &audio_str,
        ]);

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

        log::info!("Running Python Moonshine transcription script");

        let output = cmd
            .output()
            .await
            .map_err(|e| format!("Failed to run Python transcription script: {}", e))?;

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);

        log::info!("Python script exit status: {:?}", output.status);
        log::info!("Python script stdout ({} bytes): {}", output.stdout.len(), &stdout_str.chars().take(500).collect::<String>());
        log::info!("Python script stderr ({} bytes): {}", output.stderr.len(), &stderr_str.chars().take(500).collect::<String>());

        // Python script outputs JSON to stdout
        // Find the JSON line (should be the last non-empty line)
        let mut json_line: Option<&str> = None;
        for line in stdout_str.lines().rev() {
            let line = line.trim();
            if line.starts_with('{') && line.contains("\"text\":") {
                json_line = Some(line);
                break;
            }
        }

        let transcript = if let Some(json_str) = json_line {
            log::info!("Found JSON output: {}...", &json_str.chars().take(200).collect::<String>());
            Self::parse_json_text(json_str)
        } else {
            log::warn!("No JSON output found in stdout");
            String::new()
        };

        // If process failed, return error with details
        if !output.status.success() {
            return Err(format!(
                "Transcription failed (exit code {:?}): {}",
                output.status.code(),
                stderr_str.lines().last().unwrap_or("unknown error")
            ));
        }

        // If no transcript produced, return error
        let transcript = transcript.trim();
        if transcript.is_empty() {
            return Err("Transcription produced no text. The audio may be silent, corrupted, or in an unsupported format.".to_string());
        }

        log::info!("Final transcript ({} chars): '{}'", transcript.len(), &transcript.chars().take(200).collect::<String>());

        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "transcribing".to_string(),
                progress: 80.0,
                message: "Generating subtitles...".to_string(),
            })
            .await;

        // Get audio duration for SRT timing
        let duration = Self::get_audio_duration(audio_path).await.unwrap_or(60.0);

        // Generate SRT file
        let srt_content = Self::generate_srt(transcript, duration);
        fs::write(&srt_path, srt_content)
            .await
            .map_err(|e| format!("Failed to write SRT file: {}", e))?;

        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "complete".to_string(),
                progress: 100.0,
                message: "Transcription complete".to_string(),
            })
            .await;

        Ok(srt_path)
    }
}

impl MoonshineEngine {
    /// Parse the "text" field from JSON output
    fn parse_json_text(json_str: &str) -> String {
        if let Some(text_start) = json_str.find("\"text\":") {
            let after_text = &json_str[text_start + 7..];
            if let Some(quote_start) = after_text.find('"') {
                let string_content = &after_text[quote_start + 1..];
                let mut end_pos = 0;
                let mut escaped = false;
                for (i, c) in string_content.char_indices() {
                    if escaped {
                        escaped = false;
                        continue;
                    }
                    if c == '\\' {
                        escaped = true;
                        continue;
                    }
                    if c == '"' {
                        end_pos = i;
                        break;
                    }
                }
                if end_pos > 0 {
                    return string_content[..end_pos].to_string();
                }
            }
        }
        String::new()
    }

    /// Get audio duration using ffprobe
    async fn get_audio_duration(audio_path: &Path) -> Option<f64> {
        let mut cmd = Command::new(if cfg!(target_os = "windows") {
            "ffprobe.exe"
        } else {
            "ffprobe"
        });

        cmd.args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            audio_path.to_str()?,
        ]);

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);

        let output = cmd.output().await.ok()?;
        let duration_str = String::from_utf8_lossy(&output.stdout);
        duration_str.trim().parse().ok()
    }

    /// Generate SRT content from transcription text
    /// Note: Caller must ensure text is not empty
    fn generate_srt(text: &str, duration_secs: f64) -> String {
        // Split into sentences or chunks
        let sentences: Vec<&str> = text
            .split(|c| c == '.' || c == '!' || c == '?')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if sentences.is_empty() {
            // Single block for the entire text
            let end_time = Self::format_srt_time(duration_secs);
            return format!("1\n00:00:00,000 --> {}\n{}\n\n", end_time, text);
        }

        let time_per_sentence = duration_secs / sentences.len() as f64;
        let mut srt = String::new();

        for (i, sentence) in sentences.iter().enumerate() {
            let start_time = i as f64 * time_per_sentence;
            let end_time = (i + 1) as f64 * time_per_sentence;

            srt.push_str(&format!(
                "{}\n{} --> {}\n{}.\n\n",
                i + 1,
                Self::format_srt_time(start_time),
                Self::format_srt_time(end_time),
                sentence
            ));
        }

        srt
    }

    fn format_srt_time(seconds: f64) -> String {
        let hours = (seconds / 3600.0) as u32;
        let minutes = ((seconds % 3600.0) / 60.0) as u32;
        let secs = (seconds % 60.0) as u32;
        let millis = ((seconds % 1.0) * 1000.0) as u32;
        format!("{:02}:{:02}:{:02},{:03}", hours, minutes, secs, millis)
    }
}
