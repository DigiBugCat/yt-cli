"""Audio transcription with AssemblyAI."""

from pathlib import Path
from typing import Any

import assemblyai as aai

from .config import ASSEMBLYAI_API_KEY


def transcribe_audio(audio_path: Path) -> dict[str, Any]:
    """Transcribe an audio file using AssemblyAI.

    Args:
        audio_path: Path to the audio file

    Returns:
        Dictionary containing transcript data
    """
    if not ASSEMBLYAI_API_KEY:
        raise ValueError("ASSEMBLYAI_API_KEY is not set")

    aai.settings.api_key = ASSEMBLYAI_API_KEY

    config = aai.TranscriptionConfig(
        speaker_labels=True,
        punctuate=True,
        format_text=True,
    )

    transcriber = aai.Transcriber(config=config)
    transcript = transcriber.transcribe(str(audio_path))

    if transcript.status == aai.TranscriptStatus.error:
        raise RuntimeError(f"Transcription failed: {transcript.error}")

    # Build structured data
    utterances = []
    if transcript.utterances:
        for utterance in transcript.utterances:
            utterances.append({
                "speaker": utterance.speaker,
                "text": utterance.text,
                "start": utterance.start,
                "end": utterance.end,
                "confidence": utterance.confidence,
            })

    words = []
    if transcript.words:
        for word in transcript.words:
            words.append({
                "text": word.text,
                "start": word.start,
                "end": word.end,
                "confidence": word.confidence,
                "speaker": getattr(word, "speaker", None),
            })

    return {
        "text": transcript.text,
        "utterances": utterances,
        "words": words,
        "confidence": transcript.confidence,
        "audio_duration": transcript.audio_duration,
        "id": transcript.id,
    }


def format_transcript(transcript_data: dict[str, Any]) -> str:
    """Format transcript data as readable text with speaker labels.

    Args:
        transcript_data: Transcript data from transcribe_audio()

    Returns:
        Formatted text transcript
    """
    utterances = transcript_data.get("utterances", [])

    if utterances:
        # Format with speaker labels
        lines = []
        for utterance in utterances:
            speaker = utterance.get("speaker", "?")
            text = utterance.get("text", "")
            lines.append(f"Speaker {speaker}: {text}")
        return "\n\n".join(lines)
    else:
        # No speaker diarization, return plain text
        return transcript_data.get("text", "")


def format_timestamp(ms: int) -> str:
    """Convert milliseconds to HH:MM:SS format.

    Args:
        ms: Time in milliseconds

    Returns:
        Formatted time string
    """
    seconds = ms // 1000
    minutes = seconds // 60
    hours = minutes // 60

    return f"{hours:02d}:{minutes % 60:02d}:{seconds % 60:02d}"


def format_transcript_with_timestamps(transcript_data: dict[str, Any]) -> str:
    """Format transcript with timestamps.

    Args:
        transcript_data: Transcript data from transcribe_audio()

    Returns:
        Formatted text with timestamps
    """
    utterances = transcript_data.get("utterances", [])

    if utterances:
        lines = []
        for utterance in utterances:
            speaker = utterance.get("speaker", "?")
            text = utterance.get("text", "")
            start = format_timestamp(utterance.get("start", 0))
            lines.append(f"[{start}] Speaker {speaker}: {text}")
        return "\n\n".join(lines)
    else:
        return transcript_data.get("text", "")
