use futures_util::StreamExt;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::transcription::InstallProgress;

const APP_IDENTIFIER: &str = "com.zinc.app";

/// Sherpa-onnx version to download (from k2-fsa releases)
const SHERPA_VERSION: &str = "v1.12.23";

pub struct SherpaManager;

impl SherpaManager {
    /// Returns the app's bin directory path for sherpa
    pub fn get_bin_dir() -> Result<PathBuf, String> {
        let base_dir = if cfg!(target_os = "windows") {
            dirs::data_dir()
        } else if cfg!(target_os = "macos") {
            dirs::data_dir()
        } else {
            dirs::data_local_dir()
        };

        base_dir
            .map(|p| p.join(APP_IDENTIFIER).join("bin").join("sherpa"))
            .ok_or_else(|| "Could not determine app data directory".to_string())
    }

    /// Returns the models directory path for a specific engine
    pub fn get_models_dir(engine: &str) -> Result<PathBuf, String> {
        let base_dir = if cfg!(target_os = "windows") {
            dirs::data_dir()
        } else if cfg!(target_os = "macos") {
            dirs::data_dir()
        } else {
            dirs::data_local_dir()
        };

        base_dir
            .map(|p| p.join(APP_IDENTIFIER).join("models").join(engine))
            .ok_or_else(|| "Could not determine app data directory".to_string())
    }

    /// Returns the full path to the sherpa-onnx-offline binary
    pub fn get_binary_path() -> Result<PathBuf, String> {
        let bin_dir = Self::get_bin_dir()?;
        let binary_name = if cfg!(target_os = "windows") {
            "sherpa-onnx-offline.exe"
        } else {
            "sherpa-onnx-offline"
        };
        Ok(bin_dir.join(binary_name))
    }

    /// Check if sherpa-onnx is installed
    pub async fn is_installed() -> bool {
        let binary_path = match Self::get_binary_path() {
            Ok(p) => p,
            Err(_) => return false,
        };
        binary_path.exists()
    }

    /// Get the installed version by running -h and checking it runs
    #[allow(dead_code)]
    pub async fn get_installed_version() -> Result<String, String> {
        let binary_path = Self::get_binary_path()?;

        if !binary_path.exists() {
            return Err("sherpa-onnx is not installed".to_string());
        }

        let mut cmd = Command::new(&binary_path);
        cmd.arg("-h")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

        let output = cmd.output().await;

        match output {
            Ok(_) => Ok(format!("sherpa-onnx {}", SHERPA_VERSION)),
            Err(e) => Err(format!("Failed to execute sherpa-onnx: {}", e)),
        }
    }

    /// Get the download URL for the current platform
    fn get_download_url() -> String {
        // All platforms use tar.bz2 format
        let asset_name = if cfg!(target_os = "windows") {
            // Use CUDA build for GPU acceleration support
            format!("sherpa-onnx-{}-win-x64-cuda.tar.bz2", SHERPA_VERSION)
        } else if cfg!(target_os = "macos") {
            // macOS uses universal2 builds (works on both Intel and Apple Silicon)
            format!("sherpa-onnx-{}-osx-universal2-shared.tar.bz2", SHERPA_VERSION)
        } else {
            format!("sherpa-onnx-{}-linux-x64-shared.tar.bz2", SHERPA_VERSION)
        };

        format!(
            "https://github.com/k2-fsa/sherpa-onnx/releases/download/{}/{}",
            SHERPA_VERSION, asset_name
        )
    }

    /// Install sherpa-onnx by downloading from GitHub
    pub async fn install(progress_callback: Box<dyn Fn(InstallProgress) + Send>) -> Result<String, String> {
        let bin_dir = Self::get_bin_dir()?;

        // Create bin directory if it doesn't exist
        fs::create_dir_all(&bin_dir)
            .await
            .map_err(|e| format!("Failed to create bin directory: {}", e))?;

        let download_url = Self::get_download_url();

        progress_callback(InstallProgress {
            downloaded: 0,
            total: None,
            percentage: 0.0,
            stage: "Downloading sherpa-onnx...".to_string(),
        });

        let client = reqwest::Client::new();
        let response = client
            .get(&download_url)
            .header("User-Agent", "Zinc-App")
            .send()
            .await
            .map_err(|e| format!("Failed to download sherpa-onnx: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Download failed with status: {}",
                response.status()
            ));
        }

        let total_size = response.content_length();
        let mut downloaded: u64 = 0;

        // All platforms use tar.bz2 format
        let temp_archive = bin_dir.join("sherpa-onnx.tar.bz2");

        let mut file = fs::File::create(&temp_archive)
            .await
            .map_err(|e| format!("Failed to create temp file: {}", e))?;

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
                stage: "Downloading sherpa-onnx...".to_string(),
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

        // Extract the archive (all platforms use tar.bz2)
        Self::extract_tar_bz2(&temp_archive, &bin_dir).await?;

        // Clean up archive file
        let _ = fs::remove_file(&temp_archive).await;

        // Set executable permission on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let binary_path = Self::get_binary_path()?;
            if binary_path.exists() {
                let mut perms = fs::metadata(&binary_path)
                    .await
                    .map_err(|e| format!("Failed to get file metadata: {}", e))?
                    .permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&binary_path, perms)
                    .await
                    .map_err(|e| format!("Failed to set executable permission: {}", e))?;
            }
        }

        Ok(SHERPA_VERSION.to_string())
    }

    /// Extract zip file (Windows) - kept for potential future use
    #[allow(dead_code)]
    async fn extract_zip(zip_path: &PathBuf, bin_dir: &PathBuf) -> Result<(), String> {
        let zip_data = fs::read(zip_path)
            .await
            .map_err(|e| format!("Failed to read zip file: {}", e))?;

        let bin_dir = bin_dir.clone();

        tokio::task::spawn_blocking(move || {
            use std::io::{Cursor, Read};

            let reader = Cursor::new(zip_data);
            let mut archive = zip::ZipArchive::new(reader)
                .map_err(|e| format!("Failed to open zip archive: {}", e))?;

            // Extract all files, stripping the top-level directory
            for i in 0..archive.len() {
                let mut file = archive
                    .by_index(i)
                    .map_err(|e| format!("Failed to read zip entry: {}", e))?;

                let name = file.name().to_string();
                if name.contains("__MACOSX") || file.is_dir() {
                    continue;
                }

                // Strip the top-level directory (e.g., "sherpa-onnx-v1.12.23-win-x64/")
                let parts: Vec<&str> = name.split('/').collect();
                if parts.len() < 2 {
                    continue;
                }

                let relative_path = parts[1..].join("/");
                if relative_path.is_empty() {
                    continue;
                }

                // Only extract bin files (executables and DLLs)
                if relative_path.starts_with("bin/") {
                    let file_name = relative_path.strip_prefix("bin/").unwrap_or(&relative_path);
                    let output_path = bin_dir.join(file_name);

                    // Create parent directories if needed
                    if let Some(parent) = output_path.parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| format!("Failed to create directory: {}", e))?;
                    }

                    let mut contents = Vec::new();
                    file.read_to_end(&mut contents)
                        .map_err(|e| format!("Failed to read file from zip: {}", e))?;

                    std::fs::write(&output_path, contents)
                        .map_err(|e| format!("Failed to write {}: {}", file_name, e))?;
                }
            }

            Ok(())
        })
        .await
        .map_err(|e| format!("Extraction task failed: {}", e))?
    }

    /// Extract tar.bz2 file using Rust libraries (works on all platforms)
    async fn extract_tar_bz2(archive_path: &PathBuf, dest_dir: &PathBuf) -> Result<(), String> {
        log::info!("Extracting {:?} to {:?}", archive_path, dest_dir);

        let archive_path = archive_path.clone();
        let dest_dir = dest_dir.clone();

        tokio::task::spawn_blocking(move || {
            use bzip2::read::BzDecoder;
            use std::fs::File;
            use std::io::BufReader;
            use tar::Archive;

            let file = File::open(&archive_path)
                .map_err(|e| format!("Failed to open archive: {}", e))?;
            let reader = BufReader::new(file);
            let decompressor = BzDecoder::new(reader);
            let mut archive = Archive::new(decompressor);

            // Extract files, stripping the top-level directory and bin/ subdirectory
            for entry in archive.entries().map_err(|e| format!("Failed to read archive entries: {}", e))? {
                let mut entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
                let path = entry.path().map_err(|e| format!("Failed to get entry path: {}", e))?;
                let path_str = path.to_string_lossy();

                // Skip directories
                if entry.header().entry_type().is_dir() {
                    continue;
                }

                // Parse the path: sherpa-onnx-v1.11.0-win-x64-shared/bin/file.ext
                let components: Vec<&str> = path_str.split('/').collect();

                // We want files from the bin/ directory
                // Structure is: top_dir/bin/filename or top_dir/lib/filename
                if components.len() >= 3 && (components[1] == "bin" || components[1] == "lib") {
                    let filename = components[2..].join("/");
                    if filename.is_empty() {
                        continue;
                    }

                    let output_path = dest_dir.join(&filename);

                    // Create parent directories if needed
                    if let Some(parent) = output_path.parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| format!("Failed to create directory: {}", e))?;
                    }

                    log::debug!("Extracting: {} -> {:?}", path_str, output_path);

                    entry.unpack(&output_path)
                        .map_err(|e| format!("Failed to extract {}: {}", filename, e))?;
                }
            }

            Ok(())
        })
        .await
        .map_err(|e| format!("Extraction task failed: {}", e))?
    }

    /// Download a model package (tar.bz2) for a specific engine
    pub async fn download_model(
        engine: &str,
        model_url: &str,
        model_dir_name: &str,
        progress_callback: Box<dyn Fn(InstallProgress) + Send>,
    ) -> Result<PathBuf, String> {
        let models_dir = Self::get_models_dir(engine)?;
        fs::create_dir_all(&models_dir)
            .await
            .map_err(|e| format!("Failed to create models directory: {}", e))?;

        let model_dir = models_dir.join(model_dir_name);

        // Check if already installed
        if model_dir.exists() && model_dir.join("tokens.txt").exists() {
            return Ok(model_dir);
        }

        progress_callback(InstallProgress {
            downloaded: 0,
            total: None,
            percentage: 0.0,
            stage: format!("Downloading {} model...", engine),
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

        let archive_path = models_dir.join(format!("{}.tar.bz2", model_dir_name));
        let mut file = fs::File::create(&archive_path)
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
                stage: format!("Downloading {} model...", engine),
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
            stage: "Extracting model files...".to_string(),
        });

        // Extract the archive
        Self::extract_model_tar_bz2(&archive_path, &models_dir).await?;

        // Clean up the archive
        let _ = fs::remove_file(&archive_path).await;

        Ok(model_dir)
    }

    /// Extract model tar.bz2 archive using Rust libraries
    async fn extract_model_tar_bz2(archive_path: &PathBuf, dest_dir: &PathBuf) -> Result<(), String> {
        log::info!("Extracting model {:?} to {:?}", archive_path, dest_dir);

        let archive_path = archive_path.clone();
        let dest_dir = dest_dir.clone();

        tokio::task::spawn_blocking(move || {
            use bzip2::read::BzDecoder;
            use std::fs::File;
            use std::io::BufReader;
            use tar::Archive;

            let file = File::open(&archive_path)
                .map_err(|e| format!("Failed to open archive: {}", e))?;
            let reader = BufReader::new(file);
            let decompressor = BzDecoder::new(reader);
            let mut archive = Archive::new(decompressor);

            // Extract all files to destination
            archive.unpack(&dest_dir)
                .map_err(|e| format!("Failed to extract archive: {}", e))?;

            log::info!("Model extraction complete");
            Ok(())
        })
        .await
        .map_err(|e| format!("Extraction task failed: {}", e))?
    }
}
