use super::{InstallProgress, TranscribeProgress, TranscriptionEngine, TranscriptionModel};
use crate::sherpa_manager::SherpaManager;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::mpsc;

/// Model download URL from sherpa-onnx releases (int8 quantized for smaller size)
/// v3 supports 25 European languages
const PARAKEET_V3_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2";

/// Parakeet TDT transcription engine using sherpa-onnx CLI
/// Ultra-fast engine optimized for NVIDIA GPUs
pub struct ParakeetEngine;

impl ParakeetEngine {
    pub fn new() -> Self {
        Self
    }

    /// Get the model directory name for a model ID
    fn get_model_dir_name(model: &str) -> &'static str {
        match model {
            "0.6b-v3" | "0.6b" => "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8",
            _ => "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8",
        }
    }

    /// Get the download URL for a model
    fn get_model_url(model: &str) -> &'static str {
        match model {
            "0.6b-v3" | "0.6b" => PARAKEET_V3_URL,
            _ => PARAKEET_V3_URL,
        }
    }

    /// Get the models directory for Parakeet
    fn get_models_dir() -> Result<PathBuf, String> {
        SherpaManager::get_models_dir("parakeet")
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
    fn get_model_paths(model: &str) -> Result<(PathBuf, PathBuf, PathBuf, PathBuf), String> {
        let models_dir = Self::get_models_dir()?;
        let model_dir = models_dir.join(Self::get_model_dir_name(model));

        if !model_dir.exists() {
            return Err(format!("Model '{}' is not installed", model));
        }

        Ok((
            model_dir.join("encoder.int8.onnx"),
            model_dir.join("decoder.int8.onnx"),
            model_dir.join("joiner.int8.onnx"),
            model_dir.join("tokens.txt"),
        ))
    }

    /// Check for NVIDIA GPU with CUDA support
    async fn check_nvidia_gpu() -> bool {
        #[cfg(target_os = "windows")]
        {
            let mut cmd = Command::new("nvidia-smi");
            cmd.arg("--query-gpu=name")
                .arg("--format=csv,noheader")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(0x08000000);

            cmd.status()
                .await
                .map(|s| s.success())
                .unwrap_or(false)
        }

        #[cfg(not(target_os = "windows"))]
        {
            let cmd = Command::new("nvidia-smi")
                .arg("--query-gpu=name")
                .arg("--format=csv,noheader")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .status()
                .await;

            cmd.map(|s| s.success()).unwrap_or(false)
        }
    }

    /// Check if Python is available
    async fn check_python() -> bool {
        let mut cmd = Command::new("python");
        cmd.arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);

        cmd.status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Check if sherpa-onnx Python package is installed
    async fn check_sherpa_onnx_installed() -> bool {
        let mut cmd = Command::new("python");
        cmd.args(["-c", "import sherpa_onnx; print(sherpa_onnx.__version__)"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);

        cmd.status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Check if CUDA DLLs are in place for sherpa-onnx
    async fn check_cuda_dlls_ready() -> bool {
        #[cfg(target_os = "windows")]
        {
            let mut cmd = Command::new("python");
            cmd.args(["-c", r#"
import os
import sherpa_onnx
lib_path = os.path.join(os.path.dirname(sherpa_onnx.__file__), 'lib')
cudnn_dll = os.path.join(lib_path, 'cudnn64_9.dll')
cublas_dll = os.path.join(lib_path, 'cublasLt64_12.dll')
print('ok' if os.path.exists(cudnn_dll) and os.path.exists(cublas_dll) else 'missing')
"#])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(0x08000000);

            if let Ok(output) = cmd.output().await {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.trim() == "ok"
            } else {
                false
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On non-Windows, CUDA is typically installed system-wide
            true
        }
    }

    /// Check overall GPU setup status
    pub async fn check_gpu_setup_status() -> Result<crate::commands::ParakeetGpuStatus, String> {
        Ok(crate::commands::ParakeetGpuStatus {
            python_available: Self::check_python().await,
            sherpa_onnx_installed: Self::check_sherpa_onnx_installed().await,
            cuda_dlls_ready: Self::check_cuda_dlls_ready().await,
            gpu_available: Self::check_nvidia_gpu().await,
        })
    }

    /// Set up GPU support for Parakeet (installs Python packages and copies CUDA DLLs)
    pub async fn setup_gpu(
        progress_callback: Box<dyn Fn(InstallProgress) + Send + 'static>,
    ) -> Result<(), String> {
        // Step 1: Check Python
        progress_callback(InstallProgress {
            downloaded: 0,
            total: None,
            percentage: 0.0,
            stage: "Checking Python installation...".to_string(),
        });

        if !Self::check_python().await {
            return Err("Python is not installed. Please install Python 3.10+ first.".to_string());
        }

        // Step 2: Install sherpa-onnx with CUDA support
        progress_callback(InstallProgress {
            downloaded: 0,
            total: None,
            percentage: 10.0,
            stage: "Installing sherpa-onnx with CUDA support...".to_string(),
        });

        let mut cmd = Command::new("pip");
        cmd.args([
            "install",
            "sherpa-onnx==1.12.23+cuda12.cudnn9",
            "-f",
            "https://k2-fsa.github.io/sherpa/onnx/cuda.html",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);

        let output = cmd.output().await.map_err(|e| format!("Failed to run pip: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to install sherpa-onnx: {}", stderr));
        }

        // Step 3: Install click dependency
        progress_callback(InstallProgress {
            downloaded: 0,
            total: None,
            percentage: 30.0,
            stage: "Installing dependencies...".to_string(),
        });

        let mut cmd = Command::new("pip");
        cmd.args(["install", "click"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);

        let _ = cmd.output().await;

        // Step 4: Install NVIDIA CUDA libraries
        progress_callback(InstallProgress {
            downloaded: 0,
            total: None,
            percentage: 40.0,
            stage: "Installing CUDA runtime libraries...".to_string(),
        });

        let cuda_packages = [
            "nvidia-cuda-runtime-cu12",
            "nvidia-cudnn-cu12==9.1.0.70",
            "nvidia-cublas-cu12",
            "nvidia-cufft-cu12",
            "nvidia-cusparse-cu12",
            "nvidia-cusolver-cu12",
        ];

        for (i, package) in cuda_packages.iter().enumerate() {
            progress_callback(InstallProgress {
                downloaded: 0,
                total: None,
                percentage: 40.0 + (i as f64 * 8.0),
                stage: format!("Installing {}...", package),
            });

            let mut cmd = Command::new("pip");
            cmd.args(["install", package])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            #[cfg(target_os = "windows")]
            cmd.creation_flags(0x08000000);

            let _ = cmd.output().await;
        }

        // Step 5: Copy CUDA DLLs to sherpa-onnx lib folder (Windows only)
        #[cfg(target_os = "windows")]
        {
            progress_callback(InstallProgress {
                downloaded: 0,
                total: None,
                percentage: 90.0,
                stage: "Configuring CUDA DLLs...".to_string(),
            });

            let mut cmd = Command::new("python");
            cmd.args(["-c", r#"
import os
import shutil
import sherpa_onnx

lib_path = os.path.join(os.path.dirname(sherpa_onnx.__file__), 'lib')
site_packages = os.path.dirname(os.path.dirname(sherpa_onnx.__file__))
nvidia_path = os.path.join(site_packages, 'nvidia')

if os.path.exists(nvidia_path):
    for subdir in ['cuda_runtime', 'cudnn', 'cublas', 'cufft', 'cusparse', 'cusolver', 'nvjitlink', 'cuda_nvrtc']:
        bin_path = os.path.join(nvidia_path, subdir, 'bin')
        if os.path.exists(bin_path):
            for f in os.listdir(bin_path):
                if f.endswith('.dll'):
                    src = os.path.join(bin_path, f)
                    dst = os.path.join(lib_path, f)
                    if not os.path.exists(dst):
                        shutil.copy2(src, dst)
                        print(f'Copied {f}')
print('Done')
"#])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(0x08000000);

            let output = cmd.output().await.map_err(|e| format!("Failed to copy DLLs: {}", e))?;
            log::info!("DLL copy output: {}", String::from_utf8_lossy(&output.stdout));
        }

        // Step 6: Copy Python script to app bin folder
        progress_callback(InstallProgress {
            downloaded: 0,
            total: None,
            percentage: 95.0,
            stage: "Finalizing setup...".to_string(),
        });

        // The script should be bundled with the app, copy it to the sherpa folder
        let bin_dir = SherpaManager::get_bin_dir()?;
        fs::create_dir_all(&bin_dir).await.ok();

        let script_content = include_str!("../../resources/transcribe_parakeet.py");
        let script_path = bin_dir.join("transcribe_parakeet.py");
        fs::write(&script_path, script_content)
            .await
            .map_err(|e| format!("Failed to write Python script: {}", e))?;

        progress_callback(InstallProgress {
            downloaded: 0,
            total: None,
            percentage: 100.0,
            stage: "GPU setup complete!".to_string(),
        });

        Ok(())
    }
}

impl Default for ParakeetEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TranscriptionEngine for ParakeetEngine {
    fn id(&self) -> &'static str {
        "parakeet"
    }

    fn name(&self) -> &'static str {
        "Parakeet TDT"
    }

    fn description(&self) -> &'static str {
        "Fast GPU engine (~12x realtime)"
    }

    fn gpu_required(&self) -> bool {
        // Parakeet works on CPU too, but GPU is much faster
        false
    }

    async fn check_gpu_available(&self) -> bool {
        Self::check_nvidia_gpu().await
    }

    async fn is_available(&self) -> Result<bool, String> {
        // Check if Python script exists AND at least one model is installed
        let script_path = SherpaManager::get_bin_dir()
            .map(|p| p.join("transcribe_parakeet.py"))
            .map(|p| p.exists())
            .unwrap_or(false);
        let has_model = Self::is_model_installed("0.6b");
        Ok(script_path && has_model)
    }

    async fn available_models(&self) -> Vec<TranscriptionModel> {
        let has_gpu = Self::check_nvidia_gpu().await;
        vec![TranscriptionModel {
            id: "0.6b".to_string(),
            name: "0.6B v3 (int8)".to_string(),
            size: "700 MB".to_string(),
            installed: Self::is_model_installed("0.6b"),
            speed_gpu: if has_gpu { 12.0 } else { 5.0 },
            speed_cpu: 5.0,
        }]
    }

    fn speed_multiplier(&self, _model: &str) -> (f64, f64) {
        (12.0, 5.0)
    }

    fn supported_languages(&self) -> Vec<&'static str> {
        // v3 supports 25 European languages
        vec![
            "en", "de", "es", "fr", "it", "pt", "nl", "pl", "ru", "uk",
            "cs", "da", "fi", "el", "hu", "no", "ro", "sk", "sl", "sv",
            "bg", "ca", "hr", "lt", "lv",
        ]
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

        SherpaManager::download_model("parakeet", url, model_dir_name, progress_callback).await?;

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
                message: "Loading Parakeet model...".to_string(),
            })
            .await;

        // Get model paths
        let (encoder, decoder, joiner, tokens) = Self::get_model_paths(model)?;

        // Verify all files exist
        for (name, path) in [
            ("encoder", &encoder),
            ("decoder", &decoder),
            ("joiner", &joiner),
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

        // Use Python script for CUDA-accelerated transcription
        // The script handles CUDA DLL loading and falls back to CPU if needed
        // Always write the latest script to ensure updates are applied
        let bin_dir = SherpaManager::get_bin_dir()?;
        let script_path = bin_dir.join("transcribe_parakeet.py");
        let script_content = include_str!("../../resources/transcribe_parakeet.py");
        fs::write(&script_path, script_content)
            .await
            .map_err(|e| format!("Failed to write Python script: {}", e))?;

        // Use forward slashes for paths on Windows for compatibility
        let encoder_str = encoder.to_str().unwrap().replace('\\', "/");
        let decoder_str = decoder.to_str().unwrap().replace('\\', "/");
        let joiner_str = joiner.to_str().unwrap().replace('\\', "/");
        let tokens_str = tokens.to_str().unwrap().replace('\\', "/");
        let audio_str = audio_path.to_str().unwrap().replace('\\', "/");

        // Determine provider - try CUDA first if GPU available
        let provider = if Self::check_nvidia_gpu().await {
            "cuda"
        } else {
            "cpu"
        };

        let mut cmd = Command::new("python");
        cmd.args([
            script_path.to_str().unwrap(),
            &format!("--encoder={}", encoder_str),
            &format!("--decoder={}", decoder_str),
            &format!("--joiner={}", joiner_str),
            &format!("--tokens={}", tokens_str),
            &format!("--provider={}", provider),
            "--num-threads=4",
            &audio_str,
        ]);

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

        log::info!("Running Python transcription script with provider: {}", provider);

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

        let (transcript, timestamps, tokens) = if let Some(json_str) = json_line {
            log::info!("Found JSON output: {}...", &json_str.chars().take(200).collect::<String>());
            Self::parse_sherpa_json(json_str)
        } else {
            log::warn!("No JSON output found in stdout");
            (String::new(), Vec::new(), Vec::new())
        };

        // If no transcript and process failed, return error
        if transcript.is_empty() && !output.status.success() {
            return Err(format!(
                "Transcription failed (exit code {:?}): {}",
                output.status.code(),
                stderr_str.lines().last().unwrap_or("unknown error")
            ));
        }

        log::info!("Final transcript ({} chars), {} timestamps, {} tokens",
            transcript.len(), timestamps.len(), tokens.len());

        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "transcribing".to_string(),
                progress: 80.0,
                message: "Generating subtitles...".to_string(),
            })
            .await;

        // Generate SRT file using actual timestamps if available
        let srt_content = if !timestamps.is_empty() && !tokens.is_empty() {
            Self::generate_srt_with_timestamps(&tokens, &timestamps)
        } else {
            // Fallback to duration-based splitting
            let duration = Self::get_audio_duration(audio_path).await.unwrap_or(60.0);
            Self::generate_srt(&transcript.trim(), duration)
        };

        fs::write(&srt_path, &srt_content)
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

impl ParakeetEngine {
    /// Parse sherpa-onnx JSON output to extract text, timestamps, and tokens
    fn parse_sherpa_json(json_str: &str) -> (String, Vec<f64>, Vec<String>) {
        let mut text = String::new();
        let mut timestamps: Vec<f64> = Vec::new();
        let mut tokens: Vec<String> = Vec::new();

        // Extract "text" field
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
                    text = string_content[..end_pos].to_string();
                }
            }
        }

        // Extract "timestamps" array
        if let Some(ts_start) = json_str.find("\"timestamps\":") {
            let after_ts = &json_str[ts_start + 13..];
            if let Some(bracket_start) = after_ts.find('[') {
                let array_content = &after_ts[bracket_start + 1..];
                if let Some(bracket_end) = array_content.find(']') {
                    let nums_str = &array_content[..bracket_end];
                    for num in nums_str.split(',') {
                        if let Ok(ts) = num.trim().parse::<f64>() {
                            timestamps.push(ts);
                        }
                    }
                }
            }
        }

        // Extract "tokens" array
        if let Some(tok_start) = json_str.find("\"tokens\":") {
            let after_tok = &json_str[tok_start + 9..];
            if let Some(bracket_start) = after_tok.find('[') {
                let array_content = &after_tok[bracket_start + 1..];
                if let Some(bracket_end) = array_content.find(']') {
                    let toks_str = &array_content[..bracket_end];
                    // Parse tokens - they are quoted strings
                    let mut in_string = false;
                    let mut current_token = String::new();
                    let mut escaped = false;
                    for c in toks_str.chars() {
                        if escaped {
                            current_token.push(c);
                            escaped = false;
                            continue;
                        }
                        if c == '\\' {
                            escaped = true;
                            continue;
                        }
                        if c == '"' {
                            if in_string {
                                tokens.push(current_token.clone());
                                current_token.clear();
                            }
                            in_string = !in_string;
                            continue;
                        }
                        if in_string {
                            current_token.push(c);
                        }
                    }
                }
            }
        }

        (text, timestamps, tokens)
    }

    /// Generate SRT content using actual timestamps from sherpa-onnx
    fn generate_srt_with_timestamps(tokens: &[String], timestamps: &[f64]) -> String {
        if tokens.is_empty() || timestamps.is_empty() {
            return String::new();
        }

        let mut srt = String::new();
        let mut subtitle_num = 1;

        // Group tokens into subtitle segments (roughly 8-12 words per segment)
        let mut segment_start_idx = 0;
        let mut current_segment = String::new();
        let mut word_count = 0;

        for (i, token) in tokens.iter().enumerate() {
            current_segment.push_str(token);

            // Count words (tokens starting with space are usually word boundaries)
            if token.starts_with(' ') || i == 0 {
                word_count += 1;
            }

            // Create subtitle segment every 8-12 words or at sentence boundaries
            let is_sentence_end = token.ends_with('.') || token.ends_with('!') || token.ends_with('?') || token.ends_with(',');
            let should_break = (word_count >= 8 && is_sentence_end) || word_count >= 12 || i == tokens.len() - 1;

            if should_break && !current_segment.trim().is_empty() {
                let start_time = timestamps.get(segment_start_idx).copied().unwrap_or(0.0);
                // End time is start of next segment or last timestamp + small buffer
                let end_time = if i + 1 < timestamps.len() {
                    timestamps[i + 1]
                } else {
                    timestamps.get(i).copied().unwrap_or(start_time) + 0.5
                };

                srt.push_str(&format!(
                    "{}\n{} --> {}\n{}\n\n",
                    subtitle_num,
                    Self::format_srt_time(start_time),
                    Self::format_srt_time(end_time),
                    current_segment.trim()
                ));

                subtitle_num += 1;
                segment_start_idx = i + 1;
                current_segment.clear();
                word_count = 0;
            }
        }

        srt
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
    fn generate_srt(text: &str, duration_secs: f64) -> String {
        let text = text.trim();
        if text.is_empty() {
            return String::new();
        }

        // Split into sentences
        let sentences: Vec<&str> = text
            .split(|c| c == '.' || c == '!' || c == '?')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if sentences.is_empty() {
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
