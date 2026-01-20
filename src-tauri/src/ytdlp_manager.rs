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
pub enum YtDlpStatus {
    #[serde(rename = "not_installed")]
    NotInstalled,
    #[serde(rename = "installed")]
    Installed { version: String, path: String },
    #[serde(rename = "update_available")]
    UpdateAvailable {
        current: String,
        latest: String,
        path: String,
    },
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percentage: f64,
}

pub struct YtDlpManager;

impl YtDlpManager {
    /// Returns the app's bin directory path
    pub fn get_bin_dir() -> Result<PathBuf, String> {
        let base_dir = if cfg!(target_os = "windows") {
            dirs::data_dir()
        } else if cfg!(target_os = "macos") {
            dirs::data_dir() // ~/Library/Application Support
        } else {
            dirs::data_local_dir() // ~/.local/share
        };

        base_dir
            .map(|p| p.join(APP_IDENTIFIER).join("bin"))
            .ok_or_else(|| "Could not determine app data directory".to_string())
    }

    /// Returns the full path to the yt-dlp binary
    pub fn get_binary_path() -> Result<PathBuf, String> {
        let bin_dir = Self::get_bin_dir()?;
        let binary_name = if cfg!(target_os = "windows") {
            "yt-dlp.exe"
        } else {
            "yt-dlp"
        };
        Ok(bin_dir.join(binary_name))
    }

    /// Get the installed version by running --version
    pub async fn get_installed_version() -> Result<String, String> {
        let binary_path = Self::get_binary_path()?;

        if !binary_path.exists() {
            return Err("yt-dlp is not installed".to_string());
        }

        let output = Command::new(&binary_path)
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to execute yt-dlp: {}", e))?;

        if !output.status.success() {
            return Err("Failed to get yt-dlp version".to_string());
        }

        let version = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();

        Ok(version)
    }

    /// Fetch the latest version from GitHub API
    pub async fn get_latest_version() -> Result<String, String> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest")
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

    /// Get the current status of yt-dlp
    pub async fn check_status() -> YtDlpStatus {
        let binary_path = match Self::get_binary_path() {
            Ok(p) => p,
            Err(e) => return YtDlpStatus::Error { message: e },
        };

        if !binary_path.exists() {
            return YtDlpStatus::NotInstalled;
        }

        let version = match Self::get_installed_version().await {
            Ok(v) => v,
            Err(e) => return YtDlpStatus::Error { message: e },
        };

        // Check for updates (don't fail if this fails)
        if let Ok(latest) = Self::get_latest_version().await {
            if version != latest {
                return YtDlpStatus::UpdateAvailable {
                    current: version,
                    latest,
                    path: binary_path.to_string_lossy().to_string(),
                };
            }
        }

        YtDlpStatus::Installed {
            version,
            path: binary_path.to_string_lossy().to_string(),
        }
    }

    /// Get the download URL for the current platform
    fn get_download_url() -> &'static str {
        if cfg!(target_os = "windows") {
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
        } else if cfg!(target_os = "macos") {
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
        } else {
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux"
        }
    }

    /// Install yt-dlp by downloading from GitHub
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

        let download_url = Self::get_download_url();

        let client = reqwest::Client::new();
        let response = client
            .get(download_url)
            .header("User-Agent", "Zinc-App")
            .send()
            .await
            .map_err(|e| format!("Failed to download yt-dlp: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Download failed with status: {}",
                response.status()
            ));
        }

        let total_size = response.content_length();

        // Use a temp file for atomic write
        let temp_path = binary_path.with_extension("tmp");
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
            });
        }

        file.flush()
            .await
            .map_err(|e| format!("Failed to flush file: {}", e))?;
        drop(file);

        // Rename temp file to final path
        fs::rename(&temp_path, &binary_path)
            .await
            .map_err(|e| format!("Failed to rename temp file: {}", e))?;

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

        // Verify installation
        let version = Self::get_installed_version().await?;

        Ok(version)
    }

    /// Update yt-dlp to the latest version
    pub async fn update<F>(progress_callback: F) -> Result<String, String>
    where
        F: Fn(InstallProgress) + Send + 'static,
    {
        // Simply re-download - the install function handles everything
        Self::install(progress_callback).await
    }
}
