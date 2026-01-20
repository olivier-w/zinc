use crate::ytdlp_manager::YtDlpManager;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, watch};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFormat {
    pub format_id: String,
    pub ext: String,
    pub resolution: Option<String>,
    pub filesize: Option<u64>,
    pub filesize_approx: Option<u64>,
    pub format_note: Option<String>,
    pub fps: Option<f64>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub abr: Option<f64>,
    pub vbr: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    pub channel: Option<String>,
    pub view_count: Option<u64>,
    pub upload_date: Option<String>,
    pub description: Option<String>,
    pub formats: Vec<VideoFormat>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub download_id: String,
    pub status: String,
    pub progress: f64,
    pub speed: Option<String>,
    pub eta: Option<String>,
    pub filename: Option<String>,
    pub total_bytes: Option<u64>,
    pub downloaded_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadOptions {
    pub format: String,
    pub output_dir: PathBuf,
    pub filename_template: Option<String>,
    pub container_format: Option<String>,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            format: "best".to_string(),
            output_dir: dirs::download_dir().unwrap_or_else(|| PathBuf::from(".")),
            filename_template: None,
            container_format: Some("mp4".to_string()),
        }
    }
}

fn try_capture_filename(regex: &Option<Regex>, line: &str) -> Option<String> {
    regex.as_ref()
        .and_then(|r| r.captures(line))
        .map(|caps| caps[1].to_string())
}

pub struct YtDlp;

impl YtDlp {
    fn get_command() -> PathBuf {
        // Try managed binary first
        if let Ok(path) = YtDlpManager::get_binary_path() {
            if path.exists() {
                return path;
            }
        }
        // Fall back to system PATH
        PathBuf::from(if cfg!(target_os = "windows") {
            "yt-dlp.exe"
        } else {
            "yt-dlp"
        })
    }

    pub async fn check_installed() -> bool {
        Command::new(Self::get_command())
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub async fn get_video_info(url: &str) -> Result<VideoInfo, String> {
        let output = Command::new(Self::get_command())
            .args([
                "--dump-json",
                "--no-download",
                "--no-warnings",
                "--no-playlist",
                url,
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to execute yt-dlp: {}. Is yt-dlp installed?", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("yt-dlp error: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse video info: {}", e))?;

        let formats = json["formats"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|f| {
                        Some(VideoFormat {
                            format_id: f["format_id"].as_str()?.to_string(),
                            ext: f["ext"].as_str().unwrap_or("unknown").to_string(),
                            resolution: f["resolution"].as_str().map(|s| s.to_string()),
                            filesize: f["filesize"].as_u64(),
                            filesize_approx: f["filesize_approx"].as_u64(),
                            format_note: f["format_note"].as_str().map(|s| s.to_string()),
                            fps: f["fps"].as_f64(),
                            vcodec: f["vcodec"].as_str().map(|s| s.to_string()),
                            acodec: f["acodec"].as_str().map(|s| s.to_string()),
                            abr: f["abr"].as_f64(),
                            vbr: f["vbr"].as_f64(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(VideoInfo {
            id: json["id"].as_str().unwrap_or("unknown").to_string(),
            title: json["title"].as_str().unwrap_or("Unknown Title").to_string(),
            thumbnail: json["thumbnail"].as_str().map(|s| s.to_string()),
            duration: json["duration"].as_f64(),
            channel: json["channel"].as_str().or(json["uploader"].as_str()).map(|s| s.to_string()),
            view_count: json["view_count"].as_u64(),
            upload_date: json["upload_date"].as_str().map(|s| s.to_string()),
            description: json["description"].as_str().map(|s| s.to_string()),
            formats,
            url: url.to_string(),
        })
    }

    pub async fn start_download(
        url: &str,
        options: DownloadOptions,
        progress_tx: mpsc::Sender<DownloadProgress>,
        download_id: String,
        cancel_rx: watch::Receiver<bool>,
    ) -> Result<PathBuf, String> {
        let output_template = options
            .filename_template
            .unwrap_or_else(|| "%(title)s.%(ext)s".to_string());

        let output_path = options.output_dir.join(&output_template);

        let mut cmd = Command::new(Self::get_command());
        cmd.args([
            "--newline",
            "--progress",
            "--no-warnings",
            "--no-playlist",
            "--restrict-filenames",
            "-f",
            &options.format,
            "-o",
            output_path.to_str().unwrap_or("%(title)s.%(ext)s"),
        ]);

        // Set container format for merged output (video+audio)
        if let Some(ref container) = options.container_format {
            cmd.args(["--merge-output-format", container]);
        }

        cmd.arg(url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to start download: {}", e))?;

        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
        let mut reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let progress_regex = Regex::new(
            r"\[download\]\s+(\d+\.?\d*)%\s+of\s+~?\s*(\d+\.?\d*\w+)\s+at\s+(\d+\.?\d*\w+/s)\s+ETA\s+(\d+:\d+)"
        ).ok();

        // Patterns to capture the final output file path
        let download_regex = Regex::new(r"\[download\]\s+Destination:\s+(.+)").ok();
        let already_downloaded_regex = Regex::new(r"\[download\]\s+(.+)\s+has already been downloaded").ok();
        let merger_regex = Regex::new(r#"\[Merger\]\s+Merging formats into "(.+)""#).ok();

        let mut final_filename: Option<String> = None;
        let mut cancel_rx = cancel_rx;

        loop {
            tokio::select! {
                // Check for cancellation
                _ = cancel_rx.changed() => {
                    if *cancel_rx.borrow() {
                        // Kill the child process
                        let _ = child.kill().await;
                        return Err("Download cancelled".to_string());
                    }
                }
                // Read next line from stdout
                line_result = reader.next_line() => {
                    match line_result {
                        Ok(Some(line)) => {
                            // Capture output filename from various yt-dlp output patterns
                            if let Some(filename) = try_capture_filename(&download_regex, &line)
                                .or_else(|| try_capture_filename(&merger_regex, &line))
                                .or_else(|| try_capture_filename(&already_downloaded_regex, &line))
                            {
                                final_filename = Some(filename);
                            }

                            if let Some(ref regex) = progress_regex {
                                if let Some(caps) = regex.captures(&line) {
                                    let progress: f64 = caps[1].parse().unwrap_or(0.0);
                                    let _ = progress_tx
                                        .send(DownloadProgress {
                                            download_id: download_id.clone(),
                                            status: "downloading".to_string(),
                                            progress,
                                            speed: Some(caps[3].to_string()),
                                            eta: Some(caps[4].to_string()),
                                            filename: final_filename.clone(),
                                            total_bytes: None,
                                            downloaded_bytes: None,
                                        })
                                        .await;
                                }
                            }

                            if line.contains("[download] 100%") {
                                let _ = progress_tx
                                    .send(DownloadProgress {
                                        download_id: download_id.clone(),
                                        status: "completed".to_string(),
                                        progress: 100.0,
                                        speed: None,
                                        eta: None,
                                        filename: final_filename.clone(),
                                        total_bytes: None,
                                        downloaded_bytes: None,
                                    })
                                    .await;
                            }
                        }
                        Ok(None) => break, // EOF
                        Err(_) => break,
                    }
                }
            }
        }

        let status = child
            .wait()
            .await
            .map_err(|e| format!("Failed to wait for download: {}", e))?;

        if !status.success() {
            // Collect stderr for error details
            let mut error_lines = Vec::new();
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                if !line.is_empty() {
                    error_lines.push(line);
                }
            }
            let error_msg = if error_lines.is_empty() {
                "Download failed with unknown error".to_string()
            } else {
                // Get the last few meaningful lines
                let len = error_lines.len();
                error_lines.into_iter().skip(len.saturating_sub(3)).collect::<Vec<_>>().join(" | ")
            };
            return Err(error_msg);
        }

        Ok(final_filename
            .map(PathBuf::from)
            .unwrap_or_else(|| options.output_dir))
    }

    pub fn get_format_presets() -> HashMap<String, String> {
        let mut presets = HashMap::new();
        presets.insert("best".to_string(), "bestvideo+bestaudio/best".to_string());
        presets.insert("1080p".to_string(), "bestvideo[height<=1080]+bestaudio/best[height<=1080]".to_string());
        presets.insert("720p".to_string(), "bestvideo[height<=720]+bestaudio/best[height<=720]".to_string());
        presets.insert("480p".to_string(), "bestvideo[height<=480]+bestaudio/best[height<=480]".to_string());
        presets.insert("audio".to_string(), "bestaudio/best".to_string());
        presets.insert("mp3".to_string(), "bestaudio/best".to_string());
        presets
    }
}
