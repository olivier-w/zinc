use super::{
    generate_srt_from_text, get_audio_duration, parse_json_text_field,
    InstallProgress, TranscribeProgress, TranscriptionEngine, TranscriptionModel,
};
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
        _style: &str,  // Moonshine doesn't support word-level timing, always uses sentence mode
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

        // Get sherpa-onnx binary
        let sherpa_binary = SherpaManager::get_binary_path()?;
        if !sherpa_binary.exists() {
            return Err("sherpa-onnx is not installed. Please install it first.".to_string());
        }

        // Generate output SRT path
        let srt_path = audio_path.with_extension("srt");

        // Get audio duration to determine if we need chunking
        let duration = get_audio_duration(audio_path).await.unwrap_or(60.0);

        // Moonshine has context length limits - chunk long audio into 30-second segments
        const CHUNK_DURATION: f64 = 30.0;

        let transcript = if duration > CHUNK_DURATION {
            // Split audio into chunks and transcribe each
            Self::transcribe_chunked(
                audio_path,
                &sherpa_binary,
                &preprocessor,
                &encoder,
                &uncached_decoder,
                &cached_decoder,
                &tokens,
                duration,
                CHUNK_DURATION,
                &progress_tx,
            ).await?
        } else {
            // Short audio - transcribe directly
            let _ = progress_tx
                .send(TranscribeProgress {
                    stage: "transcribing".to_string(),
                    progress: 10.0,
                    message: "Running transcription...".to_string(),
                })
                .await;

            Self::transcribe_single(
                audio_path,
                &sherpa_binary,
                &preprocessor,
                &encoder,
                &uncached_decoder,
                &cached_decoder,
                &tokens,
            ).await?
        };

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

        // Generate SRT file
        let srt_content = generate_srt_from_text(transcript, duration);
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
    /// Transcribe a single audio file (for short audio under chunk duration)
    async fn transcribe_single(
        audio_path: &Path,
        sherpa_binary: &Path,
        preprocessor: &Path,
        encoder: &Path,
        uncached_decoder: &Path,
        cached_decoder: &Path,
        tokens: &Path,
    ) -> Result<String, String> {
        let mut cmd = Command::new(sherpa_binary);
        cmd.args([
            &format!("--moonshine-preprocessor={}", preprocessor.to_str().unwrap()),
            &format!("--moonshine-encoder={}", encoder.to_str().unwrap()),
            &format!("--moonshine-uncached-decoder={}", uncached_decoder.to_str().unwrap()),
            &format!("--moonshine-cached-decoder={}", cached_decoder.to_str().unwrap()),
            &format!("--tokens={}", tokens.to_str().unwrap()),
            "--num-threads=4",
            audio_path.to_str().unwrap(),
        ]);

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);

        log::info!("Running sherpa-onnx-offline for Moonshine transcription");

        let output = cmd
            .output()
            .await
            .map_err(|e| format!("Failed to run sherpa-onnx: {}", e))?;

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);

        log::info!("sherpa-onnx stdout ({} bytes)", output.stdout.len());
        log::info!("sherpa-onnx stderr ({} bytes)", output.stderr.len());

        if !output.status.success() {
            return Err(format!(
                "sherpa-onnx transcription failed: {}",
                stderr_str.lines().next().unwrap_or("unknown error")
            ));
        }

        // Parse transcript from combined output
        let combined_output = format!("{}\n{}", stdout_str, stderr_str);
        Ok(parse_json_text_field(&combined_output))
    }

    /// Transcribe long audio by splitting into chunks with ffmpeg
    async fn transcribe_chunked(
        audio_path: &Path,
        sherpa_binary: &Path,
        preprocessor: &Path,
        encoder: &Path,
        uncached_decoder: &Path,
        cached_decoder: &Path,
        tokens: &Path,
        total_duration: f64,
        chunk_duration: f64,
        progress_tx: &mpsc::Sender<TranscribeProgress>,
    ) -> Result<String, String> {
        let num_chunks = (total_duration / chunk_duration).ceil() as usize;
        log::info!(
            "Splitting {:.1}s audio into {} chunks of {:.0}s each",
            total_duration,
            num_chunks,
            chunk_duration
        );

        // Create temp directory for chunks
        let temp_dir = audio_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(".zinc_moonshine_chunks");
        fs::create_dir_all(&temp_dir)
            .await
            .map_err(|e| format!("Failed to create temp directory: {}", e))?;

        let mut all_transcripts = Vec::new();

        for i in 0..num_chunks {
            let start_time = i as f64 * chunk_duration;
            let chunk_path = temp_dir.join(format!("chunk_{:03}.wav", i));

            let progress = 10.0 + (70.0 * i as f64 / num_chunks as f64);
            let _ = progress_tx
                .send(TranscribeProgress {
                    stage: "transcribing".to_string(),
                    progress,
                    message: format!("Processing chunk {}/{}...", i + 1, num_chunks),
                })
                .await;

            // Extract chunk using ffmpeg
            let mut ffmpeg_cmd = Command::new(if cfg!(target_os = "windows") {
                "ffmpeg.exe"
            } else {
                "ffmpeg"
            });

            ffmpeg_cmd.args([
                "-y",
                "-i", audio_path.to_str().unwrap(),
                "-ss", &format!("{:.3}", start_time),
                "-t", &format!("{:.3}", chunk_duration),
                "-acodec", "pcm_s16le",
                "-ar", "16000",
                "-ac", "1",
                chunk_path.to_str().unwrap(),
            ]);

            ffmpeg_cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

            #[cfg(target_os = "windows")]
            ffmpeg_cmd.creation_flags(0x08000000);

            let ffmpeg_output = ffmpeg_cmd
                .output()
                .await
                .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

            if !ffmpeg_output.status.success() {
                let _ = fs::remove_dir_all(&temp_dir).await;
                return Err(format!(
                    "ffmpeg chunk extraction failed: {}",
                    String::from_utf8_lossy(&ffmpeg_output.stderr)
                ));
            }

            // Transcribe this chunk
            let chunk_transcript = Self::transcribe_single(
                &chunk_path,
                sherpa_binary,
                preprocessor,
                encoder,
                uncached_decoder,
                cached_decoder,
                tokens,
            ).await;

            // Clean up chunk file immediately
            let _ = fs::remove_file(&chunk_path).await;

            match chunk_transcript {
                Ok(text) => {
                    let text = text.trim();
                    if !text.is_empty() {
                        log::info!("Chunk {}: '{}'", i + 1, &text.chars().take(50).collect::<String>());
                        all_transcripts.push(text.to_string());
                    }
                }
                Err(e) => {
                    log::warn!("Chunk {} failed: {}", i + 1, e);
                    // Continue with other chunks
                }
            }
        }

        // Clean up temp directory
        let _ = fs::remove_dir_all(&temp_dir).await;

        Ok(all_transcripts.join(" "))
    }

}
