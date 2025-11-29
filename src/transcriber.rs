use std::path::Path;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::assemblyai_api_key;
use crate::error::{Error, Result};

const ASSEMBLYAI_BASE_URL: &str = "https://api.assemblyai.com/v2";

/// Utterance from speaker diarization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Utterance {
    pub speaker: String,
    pub text: String,
    pub start: i64,
    pub end: i64,
    pub confidence: Option<f64>,
}

/// Word-level data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Word {
    pub text: String,
    pub start: i64,
    pub end: i64,
    pub confidence: Option<f64>,
    pub speaker: Option<String>,
}

/// Full transcript data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptData {
    pub id: String,
    pub text: String,
    pub utterances: Vec<Utterance>,
    pub words: Vec<Word>,
    pub confidence: Option<f64>,
    pub audio_duration: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct UploadResponse {
    upload_url: String,
}

#[derive(Debug, Serialize)]
struct TranscriptRequest {
    audio_url: String,
    speaker_labels: bool,
    punctuate: bool,
    format_text: bool,
}

#[derive(Debug, Deserialize)]
struct TranscriptResponse {
    id: String,
    status: String,
    text: Option<String>,
    utterances: Option<Vec<ApiUtterance>>,
    words: Option<Vec<ApiWord>>,
    confidence: Option<f64>,
    audio_duration: Option<i64>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiUtterance {
    speaker: String,
    text: String,
    start: i64,
    end: i64,
    confidence: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ApiWord {
    text: String,
    start: i64,
    end: i64,
    confidence: Option<f64>,
    speaker: Option<String>,
}

/// AssemblyAI client
pub struct AssemblyAI {
    client: Client,
    api_key: String,
}

impl AssemblyAI {
    pub fn new() -> Result<Self> {
        let api_key = assemblyai_api_key().ok_or(Error::ApiKeyMissing)?;

        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()?;

        Ok(Self { client, api_key })
    }

    /// Upload an audio file and return the upload URL
    async fn upload_file(&self, path: &Path) -> Result<String> {
        let data = tokio::fs::read(path).await?;

        let response = self
            .client
            .post(format!("{}/upload", ASSEMBLYAI_BASE_URL))
            .header("Authorization", &self.api_key)
            .header("Content-Type", "application/octet-stream")
            .body(data)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Transcription(format!(
                "Upload failed ({}): {}",
                status, text
            )));
        }

        let upload: UploadResponse = response.json().await?;
        Ok(upload.upload_url)
    }

    /// Start a transcription job
    async fn create_transcript(&self, audio_url: &str) -> Result<String> {
        let request = TranscriptRequest {
            audio_url: audio_url.to_string(),
            speaker_labels: true,
            punctuate: true,
            format_text: true,
        };

        let response = self
            .client
            .post(format!("{}/transcript", ASSEMBLYAI_BASE_URL))
            .header("Authorization", &self.api_key)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Transcription(format!(
                "Create transcript failed ({}): {}",
                status, text
            )));
        }

        let transcript: TranscriptResponse = response.json().await?;
        Ok(transcript.id)
    }

    /// Poll for transcript completion
    async fn poll_transcript(&self, transcript_id: &str) -> Result<TranscriptData> {
        loop {
            let response = self
                .client
                .get(format!("{}/transcript/{}", ASSEMBLYAI_BASE_URL, transcript_id))
                .header("Authorization", &self.api_key)
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                return Err(Error::Transcription(format!(
                    "Poll failed ({}): {}",
                    status, text
                )));
            }

            let transcript: TranscriptResponse = response.json().await?;

            match transcript.status.as_str() {
                "completed" => {
                    let utterances = transcript
                        .utterances
                        .unwrap_or_default()
                        .into_iter()
                        .map(|u| Utterance {
                            speaker: u.speaker,
                            text: u.text,
                            start: u.start,
                            end: u.end,
                            confidence: u.confidence,
                        })
                        .collect();

                    let words = transcript
                        .words
                        .unwrap_or_default()
                        .into_iter()
                        .map(|w| Word {
                            text: w.text,
                            start: w.start,
                            end: w.end,
                            confidence: w.confidence,
                            speaker: w.speaker,
                        })
                        .collect();

                    return Ok(TranscriptData {
                        id: transcript.id,
                        text: transcript.text.unwrap_or_default(),
                        utterances,
                        words,
                        confidence: transcript.confidence,
                        audio_duration: transcript.audio_duration,
                    });
                }
                "error" => {
                    return Err(Error::Transcription(
                        transcript.error.unwrap_or_else(|| "Unknown error".to_string()),
                    ));
                }
                _ => {
                    // Still processing, wait and retry
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }
        }
    }

    /// Transcribe an audio file
    pub async fn transcribe(&self, audio_path: &Path) -> Result<TranscriptData> {
        // Upload the file
        let upload_url = self.upload_file(audio_path).await?;

        // Create transcript
        let transcript_id = self.create_transcript(&upload_url).await?;

        // Poll for completion
        self.poll_transcript(&transcript_id).await
    }
}

/// Format transcript data as readable text with speaker labels
pub fn format_transcript(data: &TranscriptData) -> String {
    if data.utterances.is_empty() {
        return data.text.clone();
    }

    data.utterances
        .iter()
        .map(|u| format!("Speaker {}: {}", u.speaker, u.text))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Format timestamp from milliseconds to HH:MM:SS
pub fn format_timestamp(ms: i64) -> String {
    let seconds = ms / 1000;
    let minutes = seconds / 60;
    let hours = minutes / 60;

    format!("{:02}:{:02}:{:02}", hours, minutes % 60, seconds % 60)
}

/// Format transcript with timestamps
pub fn format_transcript_with_timestamps(data: &TranscriptData) -> String {
    if data.utterances.is_empty() {
        return data.text.clone();
    }

    data.utterances
        .iter()
        .map(|u| {
            format!(
                "[{}] Speaker {}: {}",
                format_timestamp(u.start),
                u.speaker,
                u.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
