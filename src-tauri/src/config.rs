use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub output_dir: PathBuf,
    pub default_format: String,
    pub default_quality: String,
    pub theme: String,
    #[serde(default)]
    pub generate_subtitles: bool,
    #[serde(default = "default_whisper_model")]
    pub whisper_model: String,
    #[serde(default = "default_transcription_engine")]
    pub transcription_engine: String,
    #[serde(default = "default_transcription_model")]
    pub transcription_model: String,
    #[serde(default)]
    pub network_interface: Option<String>, // Stores IPv4 address or None for any interface
}

fn default_whisper_model() -> String {
    "base".to_string()
}

fn default_transcription_engine() -> String {
    "whisper_cpp".to_string()
}

fn default_transcription_model() -> String {
    "base".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        let output_dir = dirs::download_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."));

        Self {
            output_dir,
            default_format: "mp4".to_string(),
            default_quality: "best".to_string(),
            theme: "system".to_string(),
            generate_subtitles: false,
            whisper_model: default_whisper_model(),
            transcription_engine: default_transcription_engine(),
            transcription_model: default_transcription_model(),
            network_interface: None,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("zinc").join("config.json");
            if config_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&config_path) {
                    if let Ok(config) = serde_json::from_str(&content) {
                        return config;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let config_dir = dirs::config_dir()
            .ok_or("Could not find config directory")?
            .join("zinc");

        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;

        let config_path = config_dir.join("config.json");
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        std::fs::write(&config_path, content)
            .map_err(|e| format!("Failed to write config: {}", e))?;

        Ok(())
    }
}
