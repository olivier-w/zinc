mod engine;
mod moonshine;
mod whisper_rs_engine;

pub use engine::*;
pub use moonshine::MoonshineEngine;
pub use whisper_rs_engine::WhisperRsEngine;

use std::path::Path;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};

/// Dispatcher for transcription engines
pub struct TranscriptionDispatcher {
    engines: Vec<Arc<dyn TranscriptionEngine>>,
}

impl TranscriptionDispatcher {
    pub fn new() -> Self {
        Self {
            engines: vec![
                Arc::new(WhisperRsEngine::new()),  // Primary GPU engine
                Arc::new(MoonshineEngine::new()),  // CPU fallback
            ],
        }
    }

    /// Get all available engines
    #[allow(dead_code)]
    pub fn engines(&self) -> &[Arc<dyn TranscriptionEngine>] {
        &self.engines
    }

    /// Get engine by ID
    pub fn get_engine(&self, id: &str) -> Option<Arc<dyn TranscriptionEngine>> {
        self.engines.iter().find(|e| e.id() == id).cloned()
    }

    /// Get engine info for all engines
    pub async fn get_engine_infos(&self) -> Vec<EngineInfo> {
        let mut infos = Vec::new();
        for engine in &self.engines {
            infos.push(engine.get_info().await);
        }
        infos
    }

    /// Transcribe using the specified engine
    #[allow(dead_code)]
    pub async fn transcribe(
        &self,
        engine_id: &str,
        audio_path: &Path,
        model: &str,
        language: Option<&str>,
        style: &str,
        progress_tx: mpsc::Sender<TranscribeProgress>,
        cancel_rx: watch::Receiver<bool>,
    ) -> Result<std::path::PathBuf, String> {
        let engine = self.get_engine(engine_id)
            .ok_or_else(|| format!("Engine '{}' not found", engine_id))?;

        engine.transcribe(audio_path, model, language, style, progress_tx, cancel_rx).await
    }
}

impl Default for TranscriptionDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
