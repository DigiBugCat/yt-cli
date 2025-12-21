use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::config::transcripts_dir;
use crate::downloader::VideoMetadata;
use crate::error::{Error, Result};
use crate::transcriber::TranscriptData;

/// Sanitize a string for use as a filename
pub fn sanitize_filename(name: &str, max_length: usize) -> String {
    let re = Regex::new(r#"[<>:"/\\|?*]"#).unwrap();
    let sanitized = re.replace_all(name, "_");

    let re_spaces = Regex::new(r"[\s_]+").unwrap();
    let sanitized = re_spaces.replace_all(&sanitized, "_");

    let sanitized = sanitized.trim_matches(|c| c == '_' || c == ' ');

    let result = if sanitized.len() > max_length {
        sanitized[..max_length].trim_end_matches('_').to_string()
    } else {
        sanitized.to_string()
    };

    if result.is_empty() {
        "untitled".to_string()
    } else {
        result
    }
}

/// Platform mapping from URL domains
static PLATFORM_MAP: &[(&str, &str)] = &[
    ("youtube.com", "youtube"),
    ("youtu.be", "youtube"),
    ("vimeo.com", "vimeo"),
    ("twitter.com", "twitter"),
    ("x.com", "twitter"),
    ("twitch.tv", "twitch"),
    ("dailymotion.com", "dailymotion"),
    ("facebook.com", "facebook"),
    ("fb.watch", "facebook"),
    ("instagram.com", "instagram"),
    ("tiktok.com", "tiktok"),
];

/// Detect the platform from a video URL
pub fn get_platform_from_url(url: &str) -> String {
    let url_lower = url.to_lowercase();

    // Remove www. prefix for matching
    let domain = url_lower
        .split("://")
        .nth(1)
        .unwrap_or(&url_lower)
        .split('/')
        .next()
        .unwrap_or("")
        .trim_start_matches("www.");

    for (pattern, platform) in PLATFORM_MAP {
        if domain.contains(pattern) {
            return platform.to_string();
        }
    }

    // Default to domain name without TLD
    domain
        .split('.')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// Create organized storage path for a video
/// Structure: transcripts/{platform}/{channel_id}/{video_id}/
pub fn create_storage_path(platform: &str, channel: &str, video_id: &str) -> Result<PathBuf> {
    let safe_channel = sanitize_filename(channel, 100);
    // Video ID is already safe (alphanumeric), but sanitize just in case
    let safe_video_id = sanitize_filename(video_id, 50);

    let storage_path = transcripts_dir().join(platform).join(&safe_channel).join(&safe_video_id);
    fs::create_dir_all(&storage_path)?;

    Ok(storage_path)
}

/// Save transcript in markdown and JSON formats
pub fn save_transcript(
    storage_path: &Path,
    markdown: &str,
    structured_data: &TranscriptData,
) -> Result<(PathBuf, PathBuf)> {
    let md_path = storage_path.join("transcript.md");
    let json_path = storage_path.join("transcript.json");

    fs::write(&md_path, markdown)?;
    fs::write(&json_path, serde_json::to_string_pretty(structured_data)?)?;

    Ok((md_path, json_path))
}

/// Save video metadata as JSON
pub fn save_metadata(storage_path: &Path, metadata: &VideoMetadata) -> Result<PathBuf> {
    let metadata_path = storage_path.join("metadata.json");
    fs::write(&metadata_path, serde_json::to_string_pretty(metadata)?)?;
    Ok(metadata_path)
}

/// Move audio file to storage directory
pub fn move_audio_file(source: &Path, storage_path: &Path) -> Result<PathBuf> {
    let dest = storage_path.join("audio.mp3");
    fs::rename(source, &dest)?;
    Ok(dest)
}

/// Transcript listing info
#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptInfo {
    pub path: String,
    pub title: String,
    pub channel: String,
    pub channel_handle: Option<String>,
    pub platform: String,
    pub duration: Option<i64>,
    pub upload_date: Option<String>,
    pub url: Option<String>,
}

/// List available transcripts
pub fn list_transcripts(
    platform: Option<&str>,
    channel: Option<&str>,
    handle: Option<&str>,
) -> Result<Vec<TranscriptInfo>> {
    let mut results = Vec::new();
    let base_dir = transcripts_dir();

    if !base_dir.exists() {
        return Ok(results);
    }

    // Determine search paths based on platform filter only
    let search_paths: Vec<PathBuf> = if let Some(p) = platform {
        vec![base_dir.join(p)]
    } else {
        vec![base_dir]
    };

    for search_path in search_paths {
        if !search_path.exists() {
            continue;
        }

        find_transcripts_recursive(&search_path, &mut results)?;
    }

    // Filter by channel display name
    if let Some(channel_filter) = channel {
        let filter_lower = channel_filter.to_lowercase();
        results.retain(|t| t.channel.to_lowercase().contains(&filter_lower));
    }

    // Filter by channel handle
    if let Some(handle_filter) = handle {
        let filter_lower = handle_filter.to_lowercase();
        results.retain(|t| {
            t.channel_handle
                .as_ref()
                .map(|h| h.to_lowercase().contains(&filter_lower))
                .unwrap_or(false)
        });
    }

    Ok(results)
}

fn find_transcripts_recursive(path: &Path, results: &mut Vec<TranscriptInfo>) -> Result<()> {
    if !path.is_dir() {
        return Ok(());
    }

    let transcript_file = path.join("transcript.json");
    if transcript_file.exists() {
        let metadata_file = path.join("metadata.json");

        let mut info = TranscriptInfo {
            path: path.to_string_lossy().to_string(),
            title: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
            channel: path
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            channel_handle: None,
            platform: path
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            duration: None,
            upload_date: None,
            url: None,
        };

        if metadata_file.exists() {
            if let Ok(content) = fs::read_to_string(&metadata_file) {
                if let Ok(metadata) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&content) {
                    info.duration = metadata.get("duration").and_then(|v| v.as_i64());
                    info.upload_date = metadata.get("upload_date").and_then(|v| v.as_str()).map(String::from);
                    info.url = metadata.get("url").and_then(|v| v.as_str()).map(String::from);
                    info.channel_handle = metadata.get("uploader_id").and_then(|v| v.as_str()).map(String::from);
                    // Also get channel name from metadata if available
                    if let Some(channel_name) = metadata.get("channel").and_then(|v| v.as_str()) {
                        info.channel = channel_name.to_string();
                    }
                }
            }
        }

        results.push(info);
        return Ok(());
    }

    // Recurse into subdirectories
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                find_transcripts_recursive(&entry.path(), results)?;
            }
        }
    }

    Ok(())
}

/// Transcript content
#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptContent {
    pub text: Option<String>,
    pub structured: Option<TranscriptData>,
}

/// Get transcript content from a path
pub fn get_transcript(path: &str) -> Result<TranscriptContent> {
    let path = PathBuf::from(path);

    let (text_file, json_file) = if path.is_dir() {
        // Prefer .md, fallback to .txt
        let md_file = path.join("transcript.md");
        let txt_file = path.join("transcript.txt");
        let text_file = if md_file.exists() { md_file } else { txt_file };
        (text_file, path.join("transcript.json"))
    } else if path.extension().map(|e| e == "md" || e == "txt").unwrap_or(false) {
        (path.clone(), path.with_extension("json"))
    } else {
        (path.with_extension("md"), path.clone())
    };

    let mut result = TranscriptContent {
        text: None,
        structured: None,
    };

    if text_file.exists() {
        result.text = Some(fs::read_to_string(&text_file)?);
    }

    if json_file.exists() {
        let content = fs::read_to_string(&json_file)?;
        result.structured = Some(serde_json::from_str(&content)?);
    }

    if result.text.is_none() && result.structured.is_none() {
        return Err(Error::FileNotFound(format!(
            "No transcript found at {}",
            path.display()
        )));
    }

    Ok(result)
}
