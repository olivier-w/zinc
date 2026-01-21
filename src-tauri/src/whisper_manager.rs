use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

const APP_IDENTIFIER: &str = "com.zinc.app";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum WhisperStatus {
    #[serde(rename = "not_installed")]
    NotInstalled,
    #[serde(rename = "installed")]
    Installed { version: String, path: String },
    #[serde(rename = "model_missing")]
    ModelMissing { version: String, path: String },
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhisperModel {
    pub id: String,
    pub name: String,
    pub size: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percentage: f64,
    pub stage: String,
}

pub struct WhisperManager;

impl WhisperManager {
    /// Returns the app's bin directory path
    pub fn get_bin_dir() -> Result<PathBuf, String> {
        let base_dir = if cfg!(target_os = "windows") {
            dirs::data_dir()
        } else if cfg!(target_os = "macos") {
            dirs::data_dir()
        } else {
            dirs::data_local_dir()
        };

        base_dir
            .map(|p| p.join(APP_IDENTIFIER).join("bin"))
            .ok_or_else(|| "Could not determine app data directory".to_string())
    }

    /// Returns the models directory path
    pub fn get_models_dir() -> Result<PathBuf, String> {
        let base_dir = if cfg!(target_os = "windows") {
            dirs::data_dir()
        } else if cfg!(target_os = "macos") {
            dirs::data_dir()
        } else {
            dirs::data_local_dir()
        };

        base_dir
            .map(|p| p.join(APP_IDENTIFIER).join("models"))
            .ok_or_else(|| "Could not determine app data directory".to_string())
    }

    /// Returns the full path to the whisper binary
    pub fn get_binary_path() -> Result<PathBuf, String> {
        let bin_dir = Self::get_bin_dir()?;
        let binary_name = if cfg!(target_os = "windows") {
            "whisper.exe"
        } else {
            "whisper"
        };
        Ok(bin_dir.join(binary_name))
    }

    /// Returns the path to a model file
    pub fn get_model_path(model: &str) -> Result<PathBuf, String> {
        let models_dir = Self::get_models_dir()?;
        Ok(models_dir.join(format!("ggml-{}.bin", model)))
    }

    /// Get the installed version by running --help and parsing output
    pub async fn get_installed_version() -> Result<String, String> {
        let binary_path = Self::get_binary_path()?;

        if !binary_path.exists() {
            return Err("whisper is not installed".to_string());
        }

        let mut cmd = Command::new(&binary_path);
        cmd.arg("--help")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

        let output = cmd
            .output()
            .await
            .map_err(|e| format!("Failed to execute whisper: {}", e))?;

        // whisper.cpp doesn't have a --version flag, so we just check it runs
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Look for version pattern like "v1.2.3" or just confirm it runs
        for line in stdout.lines() {
            let line = line.trim();
            // Look for a version number pattern (v followed by digits and dots)
            if let Some(pos) = line.find(" v") {
                let version_start = pos + 1;
                if let Some(version_part) = line.get(version_start..) {
                    // Extract just the version (e.g., "v1.7.5")
                    let version: String = version_part
                        .chars()
                        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == 'v')
                        .collect();
                    if version.starts_with('v') && version.len() > 1 {
                        return Ok(format!("whisper.cpp {}", version));
                    }
                }
            }
        }

        // If binary runs successfully, just return a simple status
        Ok("whisper.cpp (installed)".to_string())
    }

    /// Fetch the latest version tag from GitHub API
    pub async fn get_latest_version() -> Result<String, String> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.github.com/repos/ggml-org/whisper.cpp/releases/latest")
            .header("User-Agent", "Zinc-App")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch latest version: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "GitHub API returned status: {}",
                response.status()
            ));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse GitHub response: {}", e))?;

        json["tag_name"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Could not find tag_name in GitHub response".to_string())
    }

    /// Check if any model is installed
    pub async fn has_any_model() -> bool {
        let models_dir = match Self::get_models_dir() {
            Ok(d) => d,
            Err(_) => return false,
        };

        for model in ["tiny", "base", "small"] {
            let path = models_dir.join(format!("ggml-{}.bin", model));
            if path.exists() {
                return true;
            }
        }
        false
    }

    /// Get the current status of whisper
    pub async fn check_status() -> WhisperStatus {
        let binary_path = match Self::get_binary_path() {
            Ok(p) => p,
            Err(e) => return WhisperStatus::Error { message: e },
        };

        if !binary_path.exists() {
            return WhisperStatus::NotInstalled;
        }

        let version = match Self::get_installed_version().await {
            Ok(v) => v,
            Err(e) => return WhisperStatus::Error { message: e },
        };

        // Check if at least one model is installed
        if !Self::has_any_model().await {
            return WhisperStatus::ModelMissing {
                version,
                path: binary_path.to_string_lossy().to_string(),
            };
        }

        WhisperStatus::Installed {
            version,
            path: binary_path.to_string_lossy().to_string(),
        }
    }

    /// Get available models with install status
    pub async fn get_available_models() -> Vec<WhisperModel> {
        let models_dir = Self::get_models_dir().unwrap_or_default();

        vec![
            WhisperModel {
                id: "tiny".to_string(),
                name: "Tiny".to_string(),
                size: "75 MB".to_string(),
                installed: models_dir.join("ggml-tiny.bin").exists(),
            },
            WhisperModel {
                id: "base".to_string(),
                name: "Base".to_string(),
                size: "142 MB".to_string(),
                installed: models_dir.join("ggml-base.bin").exists(),
            },
            WhisperModel {
                id: "small".to_string(),
                name: "Small".to_string(),
                size: "466 MB".to_string(),
                installed: models_dir.join("ggml-small.bin").exists(),
            },
        ]
    }

    /// Get the download URL for the current platform
    fn get_download_url(version: &str) -> String {
        let asset_name = if cfg!(target_os = "windows") {
            "whisper-bin-x64.zip"
        } else if cfg!(target_os = "macos") {
            "whisper-bin-universal.zip"
        } else {
            "whisper-bin-x64.zip"
        };

        format!(
            "https://github.com/ggml-org/whisper.cpp/releases/download/{}/{}",
            version, asset_name
        )
    }

    /// Install whisper by downloading from GitHub
    pub async fn install<F>(progress_callback: F) -> Result<String, String>
    where
        F: Fn(InstallProgress) + Send + 'static,
    {
        let bin_dir = Self::get_bin_dir()?;
        let binary_path = Self::get_binary_path()?;

        // Create bin directory if it doesn't exist
        fs::create_dir_all(&bin_dir)
            .await
            .map_err(|e| format!("Failed to create bin directory: {}", e))?;

        // Get latest version
        let version = Self::get_latest_version().await?;
        let download_url = Self::get_download_url(&version);

        progress_callback(InstallProgress {
            downloaded: 0,
            total: None,
            percentage: 0.0,
            stage: "Downloading whisper...".to_string(),
        });

        let client = reqwest::Client::new();
        let response = client
            .get(&download_url)
            .header("User-Agent", "Zinc-App")
            .send()
            .await
            .map_err(|e| format!("Failed to download whisper: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Download failed with status: {}",
                response.status()
            ));
        }

        let total_size = response.content_length();

        // Download to temp zip file
        let temp_zip = bin_dir.join("whisper.zip");
        let mut file = fs::File::create(&temp_zip)
            .await
            .map_err(|e| format!("Failed to create temp file: {}", e))?;

        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to write file: {}", e))?;

            downloaded += chunk.len() as u64;

            let percentage = total_size
                .map(|t| (downloaded as f64 / t as f64) * 100.0)
                .unwrap_or(0.0);

            progress_callback(InstallProgress {
                downloaded,
                total: total_size,
                percentage,
                stage: "Downloading whisper...".to_string(),
            });
        }

        file.flush()
            .await
            .map_err(|e| format!("Failed to flush file: {}", e))?;
        drop(file);

        progress_callback(InstallProgress {
            downloaded,
            total: total_size,
            percentage: 100.0,
            stage: "Extracting...".to_string(),
        });

        // Extract the zip file
        Self::extract_zip(&temp_zip, &bin_dir, &binary_path).await?;

        // Clean up zip file
        let _ = fs::remove_file(&temp_zip).await;

        // Set executable permission on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&binary_path)
                .await
                .map_err(|e| format!("Failed to get file metadata: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&binary_path, perms)
                .await
                .map_err(|e| format!("Failed to set executable permission: {}", e))?;
        }

        Ok(version)
    }

    /// Extract zip file - extracts whisper-cli and required DLLs
    async fn extract_zip(
        zip_path: &PathBuf,
        bin_dir: &PathBuf,
        target_path: &PathBuf,
    ) -> Result<(), String> {
        // Read zip file
        let zip_data = fs::read(zip_path)
            .await
            .map_err(|e| format!("Failed to read zip file: {}", e))?;

        // Use blocking task for zip extraction
        let target_path = target_path.clone();
        let bin_dir = bin_dir.clone();

        tokio::task::spawn_blocking(move || {
            use std::io::{Cursor, Read};

            let reader = Cursor::new(zip_data);
            let mut archive = zip::ZipArchive::new(reader)
                .map_err(|e| format!("Failed to open zip archive: {}", e))?;

            // Files to extract on Windows:
            // - whisper-cli.exe -> whisper.exe (main binary)
            // - whisper.dll, ggml.dll, ggml-base.dll, ggml-cpu.dll (required DLLs)
            #[cfg(target_os = "windows")]
            let files_to_extract: Vec<(&str, Option<&str>)> = vec![
                ("whisper-cli.exe", Some("whisper.exe")), // Rename to whisper.exe
                ("whisper.dll", None),
                ("ggml.dll", None),
                ("ggml-base.dll", None),
                ("ggml-cpu.dll", None),
            ];

            // On macOS/Linux, just extract the whisper-cli binary
            #[cfg(not(target_os = "windows"))]
            let files_to_extract: Vec<(&str, Option<&str>)> = vec![
                ("whisper-cli", Some("whisper")), // Rename to whisper
            ];

            let mut extracted_main = false;

            for i in 0..archive.len() {
                let mut file = archive
                    .by_index(i)
                    .map_err(|e| format!("Failed to read zip entry: {}", e))?;

                let name = file.name().to_string();
                if name.contains("__MACOSX") {
                    continue;
                }

                // Check if this file should be extracted
                for (search_name, rename_to) in &files_to_extract {
                    if name.ends_with(search_name) {
                        let mut contents = Vec::new();
                        file.read_to_end(&mut contents)
                            .map_err(|e| format!("Failed to read file from zip: {}", e))?;

                        let output_path = if let Some(new_name) = rename_to {
                            if *search_name == "whisper-cli.exe" || *search_name == "whisper-cli" {
                                // This is the main binary - use the target_path
                                target_path.clone()
                            } else {
                                bin_dir.join(new_name)
                            }
                        } else {
                            bin_dir.join(*search_name)
                        };

                        std::fs::write(&output_path, contents)
                            .map_err(|e| format!("Failed to write {}: {}", search_name, e))?;

                        if *search_name == "whisper-cli.exe" || *search_name == "whisper-cli" {
                            extracted_main = true;
                        }

                        break;
                    }
                }
            }

            if !extracted_main {
                // List what we found for debugging
                let mut found_files = Vec::new();
                for i in 0..archive.len() {
                    if let Ok(file) = archive.by_index(i) {
                        found_files.push(file.name().to_string());
                    }
                }

                return Err(format!(
                    "Could not find whisper-cli binary in zip. Found: {:?}",
                    found_files
                ));
            }

            Ok(())
        })
        .await
        .map_err(|e| format!("Extraction task failed: {}", e))?
    }

    /// Download a model file from Hugging Face
    pub async fn download_model<F>(model: &str, progress_callback: F) -> Result<(), String>
    where
        F: Fn(InstallProgress) + Send + 'static,
    {
        let valid_models = ["tiny", "base", "small"];
        if !valid_models.contains(&model) {
            return Err(format!("Invalid model: {}. Valid models: {:?}", model, valid_models));
        }

        let models_dir = Self::get_models_dir()?;
        let model_path = Self::get_model_path(model)?;

        // Create models directory if it doesn't exist
        fs::create_dir_all(&models_dir)
            .await
            .map_err(|e| format!("Failed to create models directory: {}", e))?;

        let download_url = format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
            model
        );

        progress_callback(InstallProgress {
            downloaded: 0,
            total: None,
            percentage: 0.0,
            stage: format!("Downloading {} model...", model),
        });

        let client = reqwest::Client::new();
        let response = client
            .get(&download_url)
            .header("User-Agent", "Zinc-App")
            .send()
            .await
            .map_err(|e| format!("Failed to download model: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Download failed with status: {}",
                response.status()
            ));
        }

        let total_size = response.content_length();

        // Use a temp file for atomic write
        let temp_path = model_path.with_extension("tmp");
        let mut file = fs::File::create(&temp_path)
            .await
            .map_err(|e| format!("Failed to create temp file: {}", e))?;

        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to write file: {}", e))?;

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

        Ok(())
    }
}
