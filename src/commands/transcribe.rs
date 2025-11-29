use std::collections::HashSet;

use crate::config::{ensure_directories, validate_config};
use crate::database::{add_transcript, TranscriptMetadata};
use crate::downloader::download_audio;
use crate::error::Result;
use crate::storage::{create_storage_path, get_platform_from_url, move_audio_file, save_metadata, save_transcript};
use crate::transcriber::{format_transcript_markdown, AssemblyAI};

pub async fn run(url: &str) -> Result<()> {
    validate_config()?;
    ensure_directories()?;

    eprintln!("Downloading: {}", url);
    let (audio_file, metadata) = download_audio(url)?;
    eprintln!("Downloaded: {}", metadata.title);
    eprintln!("Channel: {}", metadata.channel);
    if let Some(duration) = metadata.duration {
        eprintln!("Duration: {}s", duration);
    }

    eprintln!("\nTranscribing with AssemblyAI...");
    let assemblyai = AssemblyAI::new()?;
    let transcript_data = assemblyai.transcribe(&audio_file).await?;
    eprintln!("Transcription complete!");

    // Create storage path using video ID
    let platform = get_platform_from_url(url);
    let storage_path = create_storage_path(&platform, &metadata.channel, &metadata.id)?;

    // Move audio and save files
    move_audio_file(&audio_file, &storage_path)?;
    let markdown = format_transcript_markdown(&transcript_data);
    save_transcript(&storage_path, &markdown, &transcript_data)?;
    save_metadata(&storage_path, &metadata)?;

    // Index in database with full metadata
    let speaker_count = transcript_data
        .utterances
        .iter()
        .map(|u| &u.speaker)
        .collect::<HashSet<_>>()
        .len() as i32;
    let word_count = transcript_data.text.split_whitespace().count() as i32;

    // Serialize chapters for database storage
    let chapters_json = serde_json::to_string(&transcript_data.chapters).ok();
    let chapters_text: String = transcript_data
        .chapters
        .iter()
        .map(|c| format!("{} {}", c.headline, c.summary))
        .collect::<Vec<_>>()
        .join(" ");

    add_transcript(&TranscriptMetadata {
        video_id: &metadata.id,
        url,
        title: &metadata.title,
        channel: &metadata.channel,
        channel_id: metadata.uploader_id.as_deref(),
        platform: &platform,
        duration: metadata.duration,
        upload_date: metadata.upload_date.as_deref(),
        description: metadata.description.as_deref(),
        thumbnail: metadata.thumbnail.as_deref(),
        view_count: metadata.view_count,
        like_count: metadata.like_count,
        path: &storage_path.to_string_lossy(),
        speaker_count,
        word_count,
        confidence: transcript_data.confidence,
        chapters_json: chapters_json.as_deref(),
        chapters_text: &chapters_text,
        transcript_text: &transcript_data.text,
    })?;
    eprintln!("Indexed in database.");

    // Output result
    let duration = transcript_data.audio_duration.unwrap_or(0);
    let mins = duration / 60;
    let secs = duration % 60;
    let chapter_count = transcript_data.chapters.len();

    println!(
        r#"
Transcription complete!

Path: {}
Video ID: {}
Title: {}
Channel: {}
Duration: {}m {}s
Words: {}
Speakers: {}
Chapters: {}

Preview (first 500 chars):
{}{}"#,
        storage_path.display(),
        metadata.id,
        metadata.title,
        metadata.channel,
        mins,
        secs,
        word_count,
        speaker_count,
        chapter_count,
        &transcript_data.text[..transcript_data.text.len().min(500)],
        if transcript_data.text.len() > 500 { "..." } else { "" }
    );

    Ok(())
}
