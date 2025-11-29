use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::config::{downloads_dir, ensure_directories, firefox_cookies_args};
use crate::error::{Error, Result};

/// Video metadata extracted from yt-dlp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub uploader: Option<String>,
    pub uploader_id: Option<String>,
    pub duration: Option<i64>,
    pub upload_date: Option<String>,
    pub description: Option<String>,
    pub view_count: Option<i64>,
    pub like_count: Option<i64>,
    pub thumbnail: Option<String>,
    pub url: String,
    pub webpage_url: Option<String>,
    pub extractor: Option<String>,
}

/// Raw yt-dlp JSON output
#[derive(Debug, Deserialize)]
struct YtDlpOutput {
    id: Option<String>,
    title: Option<String>,
    channel: Option<String>,
    uploader: Option<String>,
    uploader_id: Option<String>,
    duration: Option<i64>,
    upload_date: Option<String>,
    description: Option<String>,
    view_count: Option<i64>,
    like_count: Option<i64>,
    thumbnail: Option<String>,
    webpage_url: Option<String>,
    extractor: Option<String>,
}

impl YtDlpOutput {
    fn into_metadata(self, url: &str) -> VideoMetadata {
        VideoMetadata {
            id: self.id.unwrap_or_default(),
            title: self.title.unwrap_or_else(|| "Unknown Title".to_string()),
            channel: self
                .channel
                .or(self.uploader.clone())
                .unwrap_or_else(|| "Unknown Channel".to_string()),
            uploader: self.uploader,
            uploader_id: self.uploader_id,
            duration: self.duration,
            upload_date: self.upload_date,
            description: self.description,
            view_count: self.view_count,
            like_count: self.like_count,
            thumbnail: self.thumbnail,
            url: url.to_string(),
            webpage_url: self.webpage_url,
            extractor: self.extractor,
        }
    }
}

/// Find the yt-dlp binary
fn find_ytdlp() -> Result<PathBuf> {
    // Try common locations
    let paths = [
        "/opt/homebrew/bin/yt-dlp",
        "/usr/local/bin/yt-dlp",
        "/usr/bin/yt-dlp",
    ];

    for path in paths {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }

    // Try PATH
    if let Ok(output) = Command::new("which").arg("yt-dlp").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(PathBuf::from(path));
            }
        }
    }

    Err(Error::Download(
        "yt-dlp not found. Install it with: brew install yt-dlp".to_string(),
    ))
}

/// Run yt-dlp with the given arguments
fn run_ytdlp(args: &[&str]) -> Result<String> {
    let ytdlp = find_ytdlp()?;
    let cookies_args = firefox_cookies_args();

    let mut cmd = Command::new(&ytdlp);
    for arg in &cookies_args {
        cmd.arg(arg);
    }
    for arg in args {
        cmd.arg(arg);
    }

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Download(stderr.to_string()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Extract video metadata without downloading
pub fn extract_metadata(url: &str) -> Result<VideoMetadata> {
    let output = run_ytdlp(&["--dump-json", "--no-download", url])?;
    let yt_output: YtDlpOutput = serde_json::from_str(&output)?;
    Ok(yt_output.into_metadata(url))
}

/// Download audio from a video URL
pub fn download_audio(url: &str) -> Result<(PathBuf, VideoMetadata)> {
    ensure_directories()?;

    let output_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let output_template = downloads_dir().join(format!("{}.%(ext)s", output_id));

    let output = run_ytdlp(&[
        "-f",
        "bestaudio",
        "-x",
        "--audio-format",
        "mp3",
        "--print-json",
        "-o",
        output_template.to_str().unwrap(),
        url,
    ])?;

    let yt_output: YtDlpOutput = serde_json::from_str(&output)?;
    let metadata = yt_output.into_metadata(url);

    // Find the downloaded file
    let audio_file = downloads_dir().join(format!("{}.mp3", output_id));
    if audio_file.exists() {
        return Ok((audio_file, metadata));
    }

    // Try to find any file with the output_id prefix
    if let Ok(entries) = std::fs::read_dir(downloads_dir()) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&output_id) {
                return Ok((entry.path(), metadata));
            }
        }
    }

    Err(Error::Download(format!(
        "Downloaded audio file not found for {}",
        url
    )))
}
