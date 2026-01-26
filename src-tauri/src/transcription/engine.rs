use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::mpsc;

/// Extract a segment of audio using ffmpeg
/// Returns the path to the extracted segment (16kHz mono WAV)
pub async fn extract_audio_segment(
    input_path: &Path,
    output_path: &Path,
    start_secs: f64,
    duration_secs: f64,
) -> Result<(), String> {
    let input_str = input_path
        .to_str()
        .ok_or("Invalid input path encoding")?;
    let output_str = output_path
        .to_str()
        .ok_or("Invalid output path encoding")?;

    let mut cmd = Command::new(if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    });

    cmd.args([
        "-y",                           // Overwrite output
        "-ss", &format!("{:.3}", start_secs), // Seek to start (before input for faster seeking)
        "-i", input_str,                // Input file
        "-t", &format!("{:.3}", duration_secs), // Duration
        "-vn",                          // No video
        "-acodec", "pcm_s16le",         // 16-bit PCM
        "-ar", "16000",                 // 16kHz sample rate
        "-ac", "1",                     // Mono
        output_str,
    ]);

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffmpeg segment extraction failed: {}", stderr));
    }

    Ok(())
}

/// Format seconds as SRT timestamp (HH:MM:SS,mmm)
pub fn format_srt_time(seconds: f64) -> String {
    let hours = (seconds / 3600.0) as u32;
    let minutes = ((seconds % 3600.0) / 60.0) as u32;
    let secs = (seconds % 60.0) as u32;
    let millis = ((seconds % 1.0) * 1000.0) as u32;
    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, secs, millis)
}

/// Generate SRT content from transcription text by splitting on sentence boundaries
pub fn generate_srt_from_text(text: &str, duration_secs: f64) -> String {
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
        let end_time = format_srt_time(duration_secs);
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
            format_srt_time(start_time),
            format_srt_time(end_time),
            sentence
        ));
    }

    srt
}

/// Get audio duration using ffprobe
pub async fn get_audio_duration(audio_path: &Path) -> Option<f64> {
    let audio_str = audio_path.to_str()?;

    let mut cmd = Command::new(if cfg!(target_os = "windows") {
        "ffprobe.exe"
    } else {
        "ffprobe"
    });

    cmd.args([
        "-v", "error",
        "-show_entries", "format=duration",
        "-of", "default=noprint_wrappers=1:nokey=1",
        audio_str,
    ]);

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);

    let output = cmd.output().await.ok()?;
    let duration_str = String::from_utf8_lossy(&output.stdout);
    duration_str.trim().parse().ok()
}

/// Parse the "text" field from sherpa-onnx JSON output
pub fn parse_json_text_field(json_str: &str) -> String {
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

/// Progress update during transcription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscribeProgress {
    pub stage: String,
    pub progress: f64,
    pub message: String,
}

/// Information about a transcription model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionModel {
    pub id: String,
    pub name: String,
    pub size: String,
    pub installed: bool,
    pub speed_gpu: f64,  // Speed multiplier with GPU
    pub speed_cpu: f64,  // Speed multiplier with CPU
}

/// Status of an engine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EngineStatus {
    Available,
    NotInstalled,
    Unavailable { reason: String },
}

/// Information about a transcription engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: EngineStatus,
    pub gpu_required: bool,
    pub gpu_available: bool,
    pub languages: Vec<String>,
    pub models: Vec<TranscriptionModel>,
}

/// Download progress for engines/models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percentage: f64,
    pub stage: String,
}

/// Trait for transcription engines
#[async_trait::async_trait]
pub trait TranscriptionEngine: Send + Sync {
    /// Get the engine's unique identifier
    fn id(&self) -> &'static str;

    /// Get the engine's display name
    fn name(&self) -> &'static str;

    /// Get a short description of the engine
    fn description(&self) -> &'static str;

    /// Check if GPU is required for this engine
    fn gpu_required(&self) -> bool;

    /// Check if a compatible GPU is available
    async fn check_gpu_available(&self) -> bool;

    /// Check if the engine is available (binary/runtime installed)
    async fn is_available(&self) -> Result<bool, String>;

    /// Get available models for this engine
    async fn available_models(&self) -> Vec<TranscriptionModel>;

    /// Get speed multiplier for a given model
    /// Returns (gpu_speed, cpu_speed) - higher is faster
    fn speed_multiplier(&self, model: &str) -> (f64, f64);

    /// Get supported language codes
    fn supported_languages(&self) -> Vec<&'static str>;

    /// Get full engine info
    async fn get_info(&self) -> EngineInfo {
        let is_available = self.is_available().await.unwrap_or(false);
        let gpu_available = self.check_gpu_available().await;

        let status = if is_available {
            EngineStatus::Available
        } else if self.gpu_required() && !gpu_available {
            EngineStatus::Unavailable { reason: "NVIDIA GPU required".to_string() }
        } else {
            EngineStatus::NotInstalled
        };

        EngineInfo {
            id: self.id().to_string(),
            name: self.name().to_string(),
            description: self.description().to_string(),
            status,
            gpu_required: self.gpu_required(),
            gpu_available,
            languages: self.supported_languages().iter().map(|s| s.to_string()).collect(),
            models: self.available_models().await,
        }
    }

    /// Install the engine (download binary/runtime)
    async fn install(&self, progress_callback: Box<dyn Fn(InstallProgress) + Send + 'static>) -> Result<(), String>;

    /// Download a model for this engine
    async fn download_model(&self, model: &str, progress_callback: Box<dyn Fn(InstallProgress) + Send + 'static>) -> Result<(), String>;

    /// Transcribe audio file to SRT
    /// style: "word" for one word per subtitle (karaoke-style), "sentence" for natural phrase groupings
    async fn transcribe(
        &self,
        audio_path: &Path,
        model: &str,
        language: Option<&str>,
        style: &str,
        progress_tx: mpsc::Sender<TranscribeProgress>,
    ) -> Result<PathBuf, String>;
}
