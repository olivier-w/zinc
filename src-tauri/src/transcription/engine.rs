use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

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
            EngineInfo {
                id: self.id().to_string(),
                name: self.name().to_string(),
                description: self.description().to_string(),
                status: EngineStatus::Available,
                gpu_required: self.gpu_required(),
                gpu_available,
                languages: self.supported_languages().iter().map(|s| s.to_string()).collect(),
                models: self.available_models().await,
            }
        } else if self.gpu_required() && !gpu_available {
            EngineInfo {
                id: self.id().to_string(),
                name: self.name().to_string(),
                description: self.description().to_string(),
                status: EngineStatus::Unavailable { reason: "NVIDIA GPU required".to_string() },
                gpu_required: self.gpu_required(),
                gpu_available,
                languages: self.supported_languages().iter().map(|s| s.to_string()).collect(),
                models: self.available_models().await,
            }
        } else {
            EngineInfo {
                id: self.id().to_string(),
                name: self.name().to_string(),
                description: self.description().to_string(),
                status: EngineStatus::NotInstalled,
                gpu_required: self.gpu_required(),
                gpu_available,
                languages: self.supported_languages().iter().map(|s| s.to_string()).collect(),
                models: self.available_models().await,
            }
        };

        status
    }

    /// Install the engine (download binary/runtime)
    async fn install(&self, progress_callback: Box<dyn Fn(InstallProgress) + Send + 'static>) -> Result<(), String>;

    /// Download a model for this engine
    async fn download_model(&self, model: &str, progress_callback: Box<dyn Fn(InstallProgress) + Send + 'static>) -> Result<(), String>;

    /// Transcribe audio file to SRT
    async fn transcribe(
        &self,
        audio_path: &Path,
        model: &str,
        language: Option<&str>,
        progress_tx: mpsc::Sender<TranscribeProgress>,
    ) -> Result<PathBuf, String>;
}
