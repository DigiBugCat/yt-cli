use crate::database::get_transcript_by_id;
use crate::error::{Error, Result};
use crate::storage::get_platform_from_url;

/// Extract video ID from URL
fn extract_video_id(url: &str) -> Option<String> {
    let url_lower = url.to_lowercase();

    // YouTube: various formats
    if url_lower.contains("youtube.com") || url_lower.contains("youtu.be") {
        // youtube.com/watch?v=VIDEO_ID
        if let Some(pos) = url.find("v=") {
            let start = pos + 2;
            let end = url[start..].find('&').map(|i| start + i).unwrap_or(url.len());
            return Some(url[start..end].to_string());
        }
        // youtu.be/VIDEO_ID
        if url_lower.contains("youtu.be/") {
            if let Some(pos) = url.find("youtu.be/") {
                let start = pos + 9;
                let end = url[start..].find('?').map(|i| start + i).unwrap_or(url.len());
                return Some(url[start..end].to_string());
            }
        }
    }

    // For other platforms, try to get the last path segment
    let path = url.split('?').next().unwrap_or(url);
    path.split('/').filter(|s| !s.is_empty()).last().map(String::from)
}

pub fn run(url: &str) -> Result<()> {
    let video_id = extract_video_id(url)
        .ok_or_else(|| Error::Config("Could not extract video ID from URL".to_string()))?;

    // Check database for existing transcript
    if let Some(record) = get_transcript_by_id(&video_id)? {
        println!("{}", record.path);
        return Ok(());
    }

    // Also try checking by constructing the expected path
    let platform = get_platform_from_url(url);
    let transcripts_dir = crate::config::transcripts_dir();

    // Search for the video ID in the transcripts directory
    if let Ok(entries) = std::fs::read_dir(transcripts_dir.join(&platform)) {
        for channel_entry in entries.flatten() {
            let video_path = channel_entry.path().join(&video_id);
            if video_path.join("transcript.md").exists() || video_path.join("transcript.txt").exists() {
                println!("{}", video_path.display());
                return Ok(());
            }
        }
    }

    Err(Error::FileNotFound(format!(
        "No transcript found for video ID: {}",
        video_id
    )))
}
