use std::collections::HashSet;

use crate::config::{ensure_directories, validate_config};
use crate::database::add_transcript;
use crate::downloader::download_audio;
use crate::error::Result;
use crate::storage::{create_storage_path, get_platform_from_url, move_audio_file, save_metadata, save_transcript};
use crate::transcriber::{format_transcript, AssemblyAI};

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

    // Create storage path
    let platform = get_platform_from_url(url);
    let storage_path = create_storage_path(&platform, &metadata.channel, &metadata.title)?;

    // Move audio and save files
    move_audio_file(&audio_file, &storage_path)?;
    let formatted_text = format_transcript(&transcript_data);
    save_transcript(&storage_path, &formatted_text, &transcript_data)?;
    save_metadata(&storage_path, &metadata)?;

    // Index in database
    let speaker_count = transcript_data
        .utterances
        .iter()
        .map(|u| &u.speaker)
        .collect::<HashSet<_>>()
        .len() as i32;
    let word_count = transcript_data.text.split_whitespace().count() as i32;

    add_transcript(
        &metadata.id,
        url,
        &metadata.title,
        &metadata.channel,
        &platform,
        metadata.duration,
        metadata.upload_date.as_deref(),
        &storage_path.to_string_lossy(),
        speaker_count,
        word_count,
        &transcript_data.text,
    )?;
    eprintln!("Indexed in database.");

    // Output result
    let duration = transcript_data.audio_duration.unwrap_or(0);
    let mins = duration / 60;
    let secs = duration % 60;

    println!(
        r#"
Transcription complete!

Path: {}
Title: {}
Channel: {}
Duration: {}m {}s
Words: {}
Speakers: {}

Preview (first 500 chars):
{}{}"#,
        storage_path.display(),
        metadata.title,
        metadata.channel,
        mins,
        secs,
        word_count,
        speaker_count,
        &transcript_data.text[..transcript_data.text.len().min(500)],
        if transcript_data.text.len() > 500 { "..." } else { "" }
    );

    Ok(())
}
