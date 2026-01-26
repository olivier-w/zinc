use super::{
    extract_audio_segment, format_srt_time, get_audio_duration, InstallProgress,
    TranscribeProgress, TranscriptionEngine, TranscriptionModel,
};
use crate::sherpa_manager::SherpaManager;
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Duration threshold for chunked transcription (5 minutes)
const CHUNK_DURATION_SECS: f64 = 300.0;
/// Overlap between chunks to avoid cutting mid-word (2 seconds)
const CHUNK_OVERLAP_SECS: f64 = 2.0;

/// Model download URLs from Hugging Face (GGML format)
const MODEL_URLS: &[(&str, &str, &str)] = &[
    (
        "tiny",
        "ggml-tiny.bin",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
    ),
    (
        "base",
        "ggml-base.bin",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
    ),
    (
        "small",
        "ggml-small.bin",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
    ),
    (
        "medium",
        "ggml-medium.bin",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
    ),
    (
        "large-v3",
        "ggml-large-v3.bin",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
    ),
];

/// Whisper-rs transcription engine using native Rust bindings with CUDA support
/// Provides fast GPU-accelerated transcription via whisper.cpp
pub struct WhisperRsEngine;

impl WhisperRsEngine {
    pub fn new() -> Self {
        Self
    }

    /// Get the models directory for whisper-rs
    fn get_models_dir() -> Result<PathBuf, String> {
        SherpaManager::get_models_dir("whisper-rs")
    }

    /// Get the model file name for a model ID
    fn get_model_filename(model: &str) -> &'static str {
        MODEL_URLS
            .iter()
            .find(|(id, _, _)| *id == model)
            .map(|(_, filename, _)| *filename)
            .unwrap_or("ggml-base.bin")
    }

    /// Get the download URL for a model
    fn get_model_url(model: &str) -> &'static str {
        MODEL_URLS
            .iter()
            .find(|(id, _, _)| *id == model)
            .map(|(_, _, url)| *url)
            .unwrap_or(MODEL_URLS[1].2) // Default to base
    }

    /// Get the full path to a model file
    fn get_model_path(model: &str) -> Result<PathBuf, String> {
        let models_dir = Self::get_models_dir()?;
        Ok(models_dir.join(Self::get_model_filename(model)))
    }

    /// Check if a model is installed
    fn is_model_installed(model: &str) -> bool {
        if let Ok(path) = Self::get_model_path(model) {
            path.exists()
        } else {
            false
        }
    }

    /// Check if CUDA is available by checking for nvidia-smi
    fn check_cuda_available() -> bool {
        // Check for NVIDIA GPU via nvidia-smi
        std::process::Command::new("nvidia-smi")
            .arg("--query-gpu=name")
            .arg("--format=csv,noheader")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Load audio file as f32 samples at 16kHz mono
    async fn load_audio(audio_path: &Path) -> Result<Vec<f32>, String> {
        let audio_path = audio_path.to_path_buf();

        tokio::task::spawn_blocking(move || {
            let reader = hound::WavReader::open(&audio_path)
                .map_err(|e| format!("Failed to open audio file: {}", e))?;

            let spec = reader.spec();
            let sample_rate = spec.sample_rate;
            let channels = spec.channels as usize;

            // Read samples based on format
            let samples: Vec<f32> = match spec.sample_format {
                hound::SampleFormat::Int => {
                    let bits = spec.bits_per_sample;
                    let max_val = (1i32 << (bits - 1)) as f32;
                    reader
                        .into_samples::<i32>()
                        .filter_map(|s| s.ok())
                        .map(|s| s as f32 / max_val)
                        .collect()
                }
                hound::SampleFormat::Float => reader
                    .into_samples::<f32>()
                    .filter_map(|s| s.ok())
                    .collect(),
            };

            // Convert to mono if stereo
            let mono_samples: Vec<f32> = if channels > 1 {
                samples
                    .chunks(channels)
                    .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                    .collect()
            } else {
                samples
            };

            // Resample to 16kHz if needed (whisper requires 16kHz)
            let final_samples = if sample_rate != 16000 {
                // Simple linear interpolation resampling
                let ratio = sample_rate as f64 / 16000.0;
                let new_len = (mono_samples.len() as f64 / ratio) as usize;
                let mut resampled = Vec::with_capacity(new_len);

                for i in 0..new_len {
                    let src_idx = i as f64 * ratio;
                    let idx_floor = src_idx.floor() as usize;
                    let idx_ceil = (idx_floor + 1).min(mono_samples.len() - 1);
                    let frac = src_idx - idx_floor as f64;

                    let sample = mono_samples[idx_floor] * (1.0 - frac as f32)
                        + mono_samples[idx_ceil] * frac as f32;
                    resampled.push(sample);
                }

                resampled
            } else {
                mono_samples
            };

            Ok(final_samples)
        })
        .await
        .map_err(|e| format!("Audio loading task failed: {}", e))?
    }

    /// Generate SRT content from whisper segments with timestamps
    fn generate_srt_from_segments(segments: Vec<(i64, i64, String)>) -> String {
        let mut srt = String::new();

        for (i, (start_ms, end_ms, text)) in segments.iter().enumerate() {
            let start_secs = *start_ms as f64 / 1000.0;
            let end_secs = *end_ms as f64 / 1000.0;

            srt.push_str(&format!(
                "{}\n{} --> {}\n{}\n\n",
                i + 1,
                format_srt_time(start_secs),
                format_srt_time(end_secs),
                text.trim()
            ));
        }

        srt
    }

    /// Transcribe audio in chunks for long files
    /// This prevents memory issues and maintains accurate timestamps
    async fn transcribe_chunked(
        &self,
        audio_path: &Path,
        model: &str,
        language: Option<&str>,
        style: &str,
        duration: f64,
        progress_tx: mpsc::Sender<TranscribeProgress>,
    ) -> Result<PathBuf, String> {
        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "preparing".to_string(),
                progress: 0.0,
                message: "Preparing chunked transcription...".to_string(),
            })
            .await;

        // Get model path
        let model_path = Self::get_model_path(model)?;
        if !model_path.exists() {
            return Err(format!(
                "Model '{}' is not installed. Please download it first.",
                model
            ));
        }

        // Calculate number of chunks
        // Each chunk is CHUNK_DURATION_SECS with CHUNK_OVERLAP_SECS overlap
        let effective_chunk_duration = CHUNK_DURATION_SECS - CHUNK_OVERLAP_SECS;
        let num_chunks = ((duration - CHUNK_OVERLAP_SECS) / effective_chunk_duration).ceil() as usize;
        let num_chunks = num_chunks.max(1);

        log::info!(
            "Chunked transcription: {:.1}s audio -> {} chunks of {:.0}s (with {:.0}s overlap)",
            duration,
            num_chunks,
            CHUNK_DURATION_SECS,
            CHUNK_OVERLAP_SECS
        );

        // Create temp directory for chunk files
        let temp_dir = std::env::temp_dir().join(format!("zinc_whisper_chunks_{}", std::process::id()));
        fs::create_dir_all(&temp_dir)
            .await
            .map_err(|e| format!("Failed to create temp directory: {}", e))?;

        let mut all_segments: Vec<(i64, i64, String)> = Vec::new();

        // Process each chunk
        for chunk_idx in 0..num_chunks {
            let chunk_start = chunk_idx as f64 * effective_chunk_duration;
            let chunk_duration = if chunk_idx == num_chunks - 1 {
                // Last chunk: extend to end of audio
                duration - chunk_start
            } else {
                CHUNK_DURATION_SECS
            };

            // Skip if chunk would be too short
            if chunk_duration < 0.5 {
                continue;
            }

            let chunk_progress_base = (chunk_idx as f64 / num_chunks as f64) * 90.0 + 5.0;

            let _ = progress_tx
                .send(TranscribeProgress {
                    stage: "transcribing".to_string(),
                    progress: chunk_progress_base,
                    message: format!(
                        "Processing chunk {}/{} ({:.0}s - {:.0}s)...",
                        chunk_idx + 1,
                        num_chunks,
                        chunk_start,
                        chunk_start + chunk_duration
                    ),
                })
                .await;

            // Extract chunk audio
            let chunk_path = temp_dir.join(format!("chunk_{}.wav", chunk_idx));
            extract_audio_segment(audio_path, &chunk_path, chunk_start, chunk_duration).await?;

            // Load chunk audio
            let audio_samples = Self::load_audio(&chunk_path).await?;

            log::info!(
                "Chunk {}/{}: {} samples ({:.1}s) from offset {:.1}s",
                chunk_idx + 1,
                num_chunks,
                audio_samples.len(),
                chunk_duration,
                chunk_start
            );

            // Run transcription on this chunk
            let model_path_clone = model_path.clone();
            let language = language.map(|s| s.to_string());
            let style = style.to_string();
            let progress_tx_clone = progress_tx.clone();
            let chunk_offset_ms = (chunk_start * 1000.0) as i64;

            let chunk_segments = tokio::task::spawn_blocking(move || {
                // Create whisper context with GPU enabled
                let mut ctx_params = WhisperContextParameters::default();
                ctx_params.use_gpu(true);
                ctx_params.gpu_device(0);

                let ctx = WhisperContext::new_with_params(
                    model_path_clone.to_str().unwrap(),
                    ctx_params,
                )
                .map_err(|e| format!("Failed to load Whisper model: {}", e))?;

                // Create full params for transcription
                let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

                // Set language
                if let Some(lang) = &language {
                    params.set_language(Some(lang));
                } else {
                    params.set_language(Some("auto"));
                }

                // Enable timestamps
                params.set_token_timestamps(true);

                // Set segment length based on style
                if style == "word" {
                    params.set_max_len(1);
                }

                // Set thread count
                let num_threads = std::thread::available_parallelism()
                    .map(|p| p.get().min(8))
                    .unwrap_or(4) as i32;
                params.set_n_threads(num_threads);

                // Suppress non-speech tokens
                params.set_suppress_blank(true);
                params.set_suppress_nst(true);

                // Progress callback (optional, updates within chunk)
                let _progress_tx_inner = progress_tx_clone;
                // Note: We don't set individual chunk progress callbacks to avoid flooding

                // Create state and run inference
                let mut state = ctx
                    .create_state()
                    .map_err(|e| format!("Failed to create Whisper state: {}", e))?;

                state
                    .full(params, &audio_samples)
                    .map_err(|e| format!("Transcription failed: {}", e))?;

                // Extract segments with timestamps, adjusting for chunk offset
                let num_segments = state.full_n_segments();
                let mut segments: Vec<(i64, i64, String)> = Vec::new();

                for i in 0..num_segments {
                    if let Some(segment) = state.get_segment(i) {
                        let text = segment
                            .to_str_lossy()
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        let start = segment.start_timestamp();
                        let end = segment.end_timestamp();

                        // Convert centiseconds to milliseconds and add chunk offset
                        let start_ms = start * 10 + chunk_offset_ms;
                        let end_ms = end * 10 + chunk_offset_ms;

                        if !text.trim().is_empty() {
                            segments.push((start_ms, end_ms, text));
                        }
                    }
                }

                Ok::<Vec<(i64, i64, String)>, String>(segments)
            })
            .await
            .map_err(|e| format!("Chunk transcription task failed: {}", e))??;

            // Clean up chunk file immediately
            let _ = fs::remove_file(&chunk_path).await;

            // Merge segments, handling overlap deduplication
            if !all_segments.is_empty() && !chunk_segments.is_empty() {
                // Find the overlap boundary (where previous chunk ends in overlap region)
                let overlap_start_ms = (chunk_start * 1000.0) as i64;

                // Remove segments from previous chunk that fall entirely in overlap region
                // (they'll be replaced by more accurate segments from current chunk)
                all_segments.retain(|&(start, end, _)| {
                    // Keep if segment ends before overlap starts, or starts before overlap
                    end <= overlap_start_ms || start < overlap_start_ms
                });

                // Filter new segments to avoid duplicates in overlap region
                for seg in chunk_segments {
                    let (start_ms, _end_ms, _) = seg;
                    // Only add if segment starts after overlap region, or if we have no segments there
                    if start_ms >= overlap_start_ms {
                        all_segments.push(seg);
                    }
                }
            } else {
                all_segments.extend(chunk_segments);
            }
        }

        // Clean up temp directory
        let _ = fs::remove_dir_all(&temp_dir).await;

        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "transcribing".to_string(),
                progress: 95.0,
                message: "Generating subtitles...".to_string(),
            })
            .await;

        // Check if we got any transcription
        if all_segments.is_empty() {
            return Err(
                "Transcription produced no text. The audio may be silent or corrupted.".to_string(),
            );
        }

        // Sort segments by start time (should already be sorted, but ensure it)
        all_segments.sort_by_key(|(start, _, _)| *start);

        // Generate SRT file
        let srt_content = Self::generate_srt_from_segments(all_segments);
        let srt_path = audio_path.with_extension("srt");

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

impl Default for WhisperRsEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TranscriptionEngine for WhisperRsEngine {
    fn id(&self) -> &'static str {
        "whisper_rs"
    }

    fn name(&self) -> &'static str {
        "Whisper (GPU)"
    }

    fn description(&self) -> &'static str {
        "Fast native GPU engine (16-32x realtime)"
    }

    fn gpu_required(&self) -> bool {
        false // Can run on CPU too, just faster with GPU
    }

    async fn check_gpu_available(&self) -> bool {
        Self::check_cuda_available()
    }

    async fn is_available(&self) -> Result<bool, String> {
        // Available if at least one model is installed
        Ok(MODEL_URLS.iter().any(|(id, _, _)| Self::is_model_installed(id)))
    }

    async fn available_models(&self) -> Vec<TranscriptionModel> {
        vec![
            TranscriptionModel {
                id: "tiny".to_string(),
                name: "Tiny".to_string(),
                size: "75 MB".to_string(),
                installed: Self::is_model_installed("tiny"),
                speed_gpu: 32.0,
                speed_cpu: 8.0,
            },
            TranscriptionModel {
                id: "base".to_string(),
                name: "Base".to_string(),
                size: "142 MB".to_string(),
                installed: Self::is_model_installed("base"),
                speed_gpu: 16.0,
                speed_cpu: 4.0,
            },
            TranscriptionModel {
                id: "small".to_string(),
                name: "Small".to_string(),
                size: "466 MB".to_string(),
                installed: Self::is_model_installed("small"),
                speed_gpu: 6.0,
                speed_cpu: 2.0,
            },
            TranscriptionModel {
                id: "medium".to_string(),
                name: "Medium".to_string(),
                size: "1.5 GB".to_string(),
                installed: Self::is_model_installed("medium"),
                speed_gpu: 2.0,
                speed_cpu: 0.5,
            },
            TranscriptionModel {
                id: "large-v3".to_string(),
                name: "Large v3".to_string(),
                size: "3.1 GB".to_string(),
                installed: Self::is_model_installed("large-v3"),
                speed_gpu: 1.0,
                speed_cpu: 0.2,
            },
        ]
    }

    fn speed_multiplier(&self, model: &str) -> (f64, f64) {
        match model {
            "tiny" => (32.0, 8.0),
            "base" => (16.0, 4.0),
            "small" => (6.0, 2.0),
            "medium" => (2.0, 0.5),
            "large-v3" => (1.0, 0.2),
            _ => (16.0, 4.0),
        }
    }

    fn supported_languages(&self) -> Vec<&'static str> {
        vec![
            "en", "zh", "de", "es", "ru", "ko", "fr", "ja", "pt", "tr", "pl", "ca", "nl", "ar",
            "sv", "it", "id", "hi", "fi", "vi", "he", "uk", "el", "ms", "cs", "ro", "da", "hu",
            "ta", "no", "th", "ur", "hr", "bg", "lt", "la", "mi", "ml", "cy", "sk", "te", "fa",
            "lv", "bn", "sr", "az", "sl", "kn", "et", "mk", "br", "eu", "is", "hy", "ne", "mn",
            "bs", "kk", "sq", "sw", "gl", "mr", "pa", "si", "km", "sn", "yo", "so", "af", "oc",
            "ka", "be", "tg", "sd", "gu", "am", "yi", "lo", "uz", "fo", "ht", "ps", "tk", "nn",
            "mt", "sa", "lb", "my", "bo", "tl", "mg", "as", "tt", "haw", "ln", "ha", "ba", "jw",
            "su",
        ]
    }

    async fn install(
        &self,
        _progress_callback: Box<dyn Fn(InstallProgress) + Send + 'static>,
    ) -> Result<(), String> {
        // whisper-rs is built into the binary, no separate installation needed
        // Just ensure the models directory exists
        let models_dir = Self::get_models_dir()?;
        fs::create_dir_all(&models_dir)
            .await
            .map_err(|e| format!("Failed to create models directory: {}", e))?;
        Ok(())
    }

    async fn download_model(
        &self,
        model: &str,
        progress_callback: Box<dyn Fn(InstallProgress) + Send + 'static>,
    ) -> Result<(), String> {
        let models_dir = Self::get_models_dir()?;
        fs::create_dir_all(&models_dir)
            .await
            .map_err(|e| format!("Failed to create models directory: {}", e))?;

        let model_url = Self::get_model_url(model);
        let model_path = Self::get_model_path(model)?;

        // Check if already downloaded
        if model_path.exists() {
            log::info!("Model {} already exists at {:?}", model, model_path);
            return Ok(());
        }

        progress_callback(InstallProgress {
            downloaded: 0,
            total: None,
            percentage: 0.0,
            stage: format!("Downloading {} model...", model),
        });

        let client = reqwest::Client::new();
        let response = client
            .get(model_url)
            .header("User-Agent", "Zinc-App")
            .send()
            .await
            .map_err(|e| format!("Failed to start download: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Download failed with status: {}",
                response.status()
            ));
        }

        let total_size = response.content_length();
        let mut downloaded: u64 = 0;

        // Download to a temp file first
        let temp_path = model_path.with_extension("bin.tmp");
        let mut file = fs::File::create(&temp_path)
            .await
            .map_err(|e| format!("Failed to create file: {}", e))?;

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Write error: {}", e))?;

            downloaded += chunk.len() as u64;
            let percentage = total_size
                .map(|t| (downloaded as f64 / t as f64) * 100.0)
                .unwrap_or(0.0);

            progress_callback(InstallProgress {
                downloaded,
                total: total_size,
                percentage,
                stage: format!("Downloading {} model...", model),
            });
        }

        file.flush()
            .await
            .map_err(|e| format!("Failed to flush file: {}", e))?;
        drop(file);

        // Rename temp file to final path
        fs::rename(&temp_path, &model_path)
            .await
            .map_err(|e| format!("Failed to rename temp file: {}", e))?;

        log::info!("Model {} downloaded successfully to {:?}", model, model_path);
        Ok(())
    }

    async fn transcribe(
        &self,
        audio_path: &Path,
        model: &str,
        language: Option<&str>,
        style: &str,
        progress_tx: mpsc::Sender<TranscribeProgress>,
    ) -> Result<PathBuf, String> {
        // Check audio duration first to decide on chunked vs single-shot transcription
        let duration = get_audio_duration(audio_path).await.unwrap_or(60.0);

        // Use chunked transcription for long audio files
        if duration > CHUNK_DURATION_SECS {
            log::info!(
                "Audio duration {:.1}s exceeds chunk threshold {:.0}s, using chunked transcription",
                duration,
                CHUNK_DURATION_SECS
            );
            return self
                .transcribe_chunked(audio_path, model, language, style, duration, progress_tx)
                .await;
        }

        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "preparing".to_string(),
                progress: 0.0,
                message: "Loading Whisper model...".to_string(),
            })
            .await;

        // Get model path
        let model_path = Self::get_model_path(model)?;
        if !model_path.exists() {
            return Err(format!(
                "Model '{}' is not installed. Please download it first.",
                model
            ));
        }

        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "preparing".to_string(),
                progress: 5.0,
                message: "Loading audio...".to_string(),
            })
            .await;

        // Load audio (for short files, load all at once)
        let audio_samples = Self::load_audio(audio_path).await?;

        log::info!(
            "Loaded {} samples ({:.1}s) from {:?}",
            audio_samples.len(),
            duration,
            audio_path
        );

        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "transcribing".to_string(),
                progress: 10.0,
                message: "Initializing Whisper...".to_string(),
            })
            .await;

        // Run transcription in a blocking task since whisper-rs is synchronous
        let model_path_clone = model_path.clone();
        let language = language.map(|s| s.to_string());
        let style = style.to_string();
        let progress_tx_clone = progress_tx.clone();

        let segments = tokio::task::spawn_blocking(move || {
            // Create whisper context with GPU enabled
            let mut ctx_params = WhisperContextParameters::default();
            ctx_params.use_gpu(true);
            ctx_params.gpu_device(0); // Use first GPU

            println!("=== WHISPER-RS: Loading model with GPU enabled ===");

            let ctx = WhisperContext::new_with_params(
                model_path_clone.to_str().unwrap(),
                ctx_params,
            )
            .map_err(|e| format!("Failed to load Whisper model: {}", e))?;

            println!("=== WHISPER-RS: Model loaded successfully ===");

            // Create full params for transcription
            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

            // Set language if specified
            if let Some(lang) = &language {
                params.set_language(Some(lang));
            } else {
                params.set_language(Some("auto"));
            }

            // Enable timestamps
            params.set_token_timestamps(true);

            // Set segment length based on style:
            // "word" = one word per subtitle (karaoke-style timing)
            // "sentence" = natural phrase groupings (like movie subtitles)
            if style == "word" {
                params.set_max_len(1); // One word per segment
            }
            // For "sentence" mode, don't set max_len - whisper naturally segments by phrases

            // Set thread count based on CPU cores
            let num_threads = std::thread::available_parallelism()
                .map(|p| p.get().min(8))
                .unwrap_or(4) as i32;
            params.set_n_threads(num_threads);

            // Suppress non-speech tokens
            params.set_suppress_blank(true);
            params.set_suppress_nst(true);

            // Set up progress callback
            let progress_tx_inner = progress_tx_clone.clone();
            params.set_progress_callback_safe(move |progress| {
                let pct = 10.0 + (progress as f64 * 0.8); // 10% to 90%
                let _ = progress_tx_inner.blocking_send(TranscribeProgress {
                    stage: "transcribing".to_string(),
                    progress: pct,
                    message: format!("Transcribing... {}%", progress),
                });
            });

            // Create state and run inference
            let mut state = ctx.create_state()
                .map_err(|e| format!("Failed to create Whisper state: {}", e))?;

            state.full(params, &audio_samples)
                .map_err(|e| format!("Transcription failed: {}", e))?;

            // Extract segments with timestamps
            let num_segments = state.full_n_segments();

            let mut segments: Vec<(i64, i64, String)> = Vec::new();

            for i in 0..num_segments {
                if let Some(segment) = state.get_segment(i) {
                    let text = segment.to_str_lossy()
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let start = segment.start_timestamp();
                    let end = segment.end_timestamp();

                    // whisper-rs returns times in centiseconds (1/100 sec), convert to milliseconds
                    let start_ms = start * 10;
                    let end_ms = end * 10;

                    if !text.trim().is_empty() {
                        segments.push((start_ms, end_ms, text));
                    }
                }
            }

            Ok::<Vec<(i64, i64, String)>, String>(segments)
        })
        .await
        .map_err(|e| format!("Transcription task failed: {}", e))??;

        let _ = progress_tx
            .send(TranscribeProgress {
                stage: "transcribing".to_string(),
                progress: 90.0,
                message: "Generating subtitles...".to_string(),
            })
            .await;

        // Check if we got any transcription
        if segments.is_empty() {
            return Err(
                "Transcription produced no text. The audio may be silent or corrupted.".to_string(),
            );
        }

        // Generate SRT file
        let srt_content = Self::generate_srt_from_segments(segments);
        let srt_path = audio_path.with_extension("srt");

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
