use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::ytdlp_manager::InstallProgress;

const APP_IDENTIFIER: &str = "com.zinc.app";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum DenoStatus {
    #[serde(rename = "not_installed")]
    NotInstalled,
    #[serde(rename = "installed")]
    Installed { version: String, path: String },
    #[serde(rename = "error")]
    Error { message: String },
}

pub struct DenoManager;

impl DenoManager {
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

    /// Returns the full path to the deno binary
    pub fn get_binary_path() -> Result<PathBuf, String> {
        let bin_dir = Self::get_bin_dir()?;
        let binary_name = if cfg!(target_os = "windows") {
            "deno.exe"
        } else {
            "deno"
        };
        Ok(bin_dir.join(binary_name))
    }

    /// Get the installed version by running --version
    pub async fn get_installed_version() -> Result<String, String> {
        let binary_path = Self::get_binary_path()?;

        if !binary_path.exists() {
            return Err("Deno is not installed".to_string());
        }

        let mut cmd = Command::new(&binary_path);
        cmd.arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

        let output = cmd
            .output()
            .await
            .map_err(|e| format!("Failed to execute deno: {}", e))?;

        if !output.status.success() {
            return Err("Failed to get deno version".to_string());
        }

        // deno --version outputs multiple lines like "deno 1.40.0 ..."
        // We want the first line's version number
        let stdout = String::from_utf8_lossy(&output.stdout);
        let version = stdout
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("deno "))
            .map(|v| {
                // Take just the version number (stop at first space or end)
                v.split_whitespace().next().unwrap_or(v).to_string()
            })
            .unwrap_or_else(|| stdout.trim().to_string());

        Ok(version)
    }

    /// Get the current status of deno
    pub async fn check_status() -> DenoStatus {
        let binary_path = match Self::get_binary_path() {
            Ok(p) => p,
            Err(e) => return DenoStatus::Error { message: e },
        };

        if !binary_path.exists() {
            return DenoStatus::NotInstalled;
        }

        match Self::get_installed_version().await {
            Ok(version) => DenoStatus::Installed {
                version,
                path: binary_path.to_string_lossy().to_string(),
            },
            Err(e) => DenoStatus::Error { message: e },
        }
    }

    /// Get the download URL for the current platform
    fn get_download_url() -> String {
        let target = if cfg!(target_os = "windows") {
            "deno-x86_64-pc-windows-msvc.zip"
        } else if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                "deno-aarch64-apple-darwin.zip"
            } else {
                "deno-x86_64-apple-darwin.zip"
            }
        } else {
            "deno-x86_64-unknown-linux-gnu.zip"
        };
        format!(
            "https://github.com/denoland/deno/releases/latest/download/{}",
            target
        )
    }

    /// Install deno by downloading from GitHub
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
            .get(&download_url)
            .header("User-Agent", "Zinc-App")
            .send()
            .await
            .map_err(|e| format!("Failed to download Deno: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Download failed with status: {}",
                response.status()
            ));
        }

        let total_size = response.content_length();

        // Download to a temp zip file
        let zip_path = bin_dir.join("deno_download.zip");
        let mut file = fs::File::create(&zip_path)
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

        // Extract deno binary from zip
        let zip_path_clone = zip_path.clone();
        let binary_path_clone = binary_path.clone();
        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&zip_path_clone)
                .map_err(|e| format!("Failed to open zip file: {}", e))?;
            let mut archive =
                zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip: {}", e))?;

            let binary_name = if cfg!(target_os = "windows") {
                "deno.exe"
            } else {
                "deno"
            };

            let mut found = false;
            for i in 0..archive.len() {
                let mut entry = archive
                    .by_index(i)
                    .map_err(|e| format!("Failed to read zip entry: {}", e))?;
                let name = entry.name().to_string();
                if name == binary_name || name.ends_with(&format!("/{}", binary_name)) {
                    let mut outfile = std::fs::File::create(&binary_path_clone)
                        .map_err(|e| format!("Failed to create binary file: {}", e))?;
                    std::io::copy(&mut entry, &mut outfile)
                        .map_err(|e| format!("Failed to extract binary: {}", e))?;
                    found = true;
                    break;
                }
            }

            if !found {
                return Err(format!("Could not find {} in zip archive", binary_name));
            }

            Ok::<(), String>(())
        })
        .await
        .map_err(|e| format!("Extract task failed: {}", e))??;

        // Clean up zip file
        let _ = fs::remove_file(&zip_path).await;

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
}
