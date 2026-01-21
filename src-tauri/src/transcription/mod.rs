mod engine;
mod whisper_cpp;
mod moonshine;
pub mod parakeet;

pub use engine::*;
pub use whisper_cpp::WhisperCppEngine;
pub use moonshine::MoonshineEngine;
pub use parakeet::ParakeetEngine;

use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Dispatcher for transcription engines
pub struct TranscriptionDispatcher {
    engines: Vec<Arc<dyn TranscriptionEngine>>,
}

impl TranscriptionDispatcher {
    pub fn new() -> Self {
        Self {
            engines: vec![
                Arc::new(MoonshineEngine::new()),
                Arc::new(ParakeetEngine::new()),
                Arc::new(WhisperCppEngine::new()),
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
        progress_tx: mpsc::Sender<TranscribeProgress>,
    ) -> Result<std::path::PathBuf, String> {
        let engine = self.get_engine(engine_id)
            .ok_or_else(|| format!("Engine '{}' not found", engine_id))?;

        engine.transcribe(audio_path, model, language, progress_tx).await
    }
}

impl Default for TranscriptionDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
