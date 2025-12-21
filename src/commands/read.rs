use crate::commands::reindex::{find_video_on_disk, index_video_dir};
use crate::database::get_transcript_by_id;
use crate::error::{Error, Result};
use crate::storage::get_transcript;

/// Resolve a video ID or path to an actual transcript path
fn resolve_path(path_or_id: &str) -> Result<String> {
    // First, check if it's already a valid path
    let as_path = std::path::Path::new(path_or_id);
    if as_path.exists() {
        return Ok(path_or_id.to_string());
    }

    // Try to find it in the database by video ID
    if let Some(record) = get_transcript_by_id(path_or_id)? {
        return Ok(record.path);
    }

    // Not in database - try to find on disk and auto-index
    if let Some(video_dir) = find_video_on_disk(path_or_id) {
        eprintln!("Found on disk, indexing...");
        index_video_dir(&video_dir)?;
        return Ok(video_dir.to_string_lossy().to_string());
    }

    Err(Error::FileNotFound(format!(
        "No transcript found for '{}'",
        path_or_id
    )))
}

pub fn run(path_or_id: &str, json: bool) -> Result<()> {
    let path = resolve_path(path_or_id)?;
    let data = get_transcript(&path)?;

    if json {
        if let Some(structured) = data.structured {
            println!("{}", serde_json::to_string_pretty(&structured)?);
        } else {
            eprintln!("No structured data available.");
        }
    } else if let Some(text) = data.text {
        println!("{}", text);
    } else {
        eprintln!("No text content found.");
    }

    Ok(())
}
