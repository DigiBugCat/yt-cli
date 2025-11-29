use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::config::{ensure_directories, transcripts_dir};
use crate::database::add_transcript;
use crate::error::Result;
use crate::transcriber::TranscriptData;

pub fn run() -> Result<()> {
    ensure_directories()?;

    let transcripts_path = transcripts_dir();
    if !transcripts_path.exists() {
        println!("No transcripts directory found.");
        return Ok(());
    }

    let mut count = 0;

    reindex_recursive(&transcripts_path, &mut count)?;

    println!("\nReindexed {} transcript(s).", count);

    Ok(())
}

fn reindex_recursive(path: &Path, count: &mut i32) -> Result<()> {
    if !path.is_dir() {
        return Ok(());
    }

    let transcript_json = path.join("transcript.json");
    if transcript_json.exists() {
        if let Err(e) = reindex_single(path, &transcript_json) {
            eprintln!("Error indexing {}: {}", path.display(), e);
        } else {
            *count += 1;
            println!("Indexed: {}", path.file_name().unwrap_or_default().to_string_lossy());
        }
        return Ok(());
    }

    // Recurse into subdirectories
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                reindex_recursive(&entry.path(), count)?;
            }
        }
    }

    Ok(())
}

fn reindex_single(video_dir: &Path, transcript_json: &Path) -> Result<()> {
    let metadata_file = video_dir.join("metadata.json");

    // Read transcript
    let transcript_content = fs::read_to_string(transcript_json)?;
    let transcript_data: TranscriptData = serde_json::from_str(&transcript_content)?;

    // Read metadata if available
    let metadata: HashMap<String, serde_json::Value> = if metadata_file.exists() {
        let content = fs::read_to_string(&metadata_file)?;
        serde_json::from_str(&content)?
    } else {
        HashMap::new()
    };

    let text = &transcript_data.text;
    let speaker_count = transcript_data
        .utterances
        .iter()
        .map(|u| &u.speaker)
        .collect::<HashSet<_>>()
        .len() as i32;
    let word_count = text.split_whitespace().count() as i32;

    // Get platform from path structure
    let transcripts_dir = crate::config::transcripts_dir();
    let rel_path = video_dir.strip_prefix(&transcripts_dir).unwrap_or(video_dir);
    let parts: Vec<_> = rel_path.components().collect();

    let platform = parts
        .first()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let channel = parts
        .get(1)
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let video_id = metadata
        .get("id")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| video_dir.file_name().unwrap_or_default().to_string_lossy().to_string());

    let url = metadata
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let title = metadata
        .get("title")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| video_dir.file_name().unwrap_or_default().to_string_lossy().to_string());

    let channel_from_meta = metadata
        .get("channel")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or(channel);

    let duration = metadata.get("duration").and_then(|v| v.as_i64());

    let upload_date = metadata.get("upload_date").and_then(|v| v.as_str());

    add_transcript(
        &video_id,
        url,
        &title,
        &channel_from_meta,
        &platform,
        duration,
        upload_date,
        &video_dir.to_string_lossy(),
        speaker_count,
        word_count,
        text,
    )?;

    Ok(())
}
