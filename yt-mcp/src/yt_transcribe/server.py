"""FastMCP server for video transcription."""

from fastmcp import FastMCP

from .config import ensure_directories, validate_config
from .downloader import download_audio
from .storage import (
    create_storage_path,
    get_platform_from_url,
    get_transcript,
    list_transcripts,
    move_audio_file,
    save_metadata,
    save_transcript,
)
from .transcriber import format_transcript, transcribe_audio

# Initialize FastMCP server
mcp = FastMCP("yt-transcribe")


@mcp.tool()
def transcribe_video(url: str) -> str:
    """Download a video and transcribe it using AssemblyAI.

    Downloads the audio from the provided URL, transcribes it with speaker
    diarization, and saves everything to an organized folder structure.

    Args:
        url: The video URL (YouTube, Vimeo, etc.)

    Returns:
        Path to the transcript folder and a summary of the transcript.
    """
    # Validate config on first use
    validate_config()
    ensure_directories()

    # Download audio
    audio_file, metadata = download_audio(url)

    # Transcribe
    transcript_data = transcribe_audio(audio_file)

    # Create storage path
    platform = get_platform_from_url(url)
    storage_path = create_storage_path(
        platform=platform,
        channel=metadata["channel"],
        title=metadata["title"],
    )

    # Move audio file to storage
    move_audio_file(audio_file, storage_path)

    # Format and save transcript
    formatted_text = format_transcript(transcript_data)
    save_transcript(storage_path, formatted_text, transcript_data)

    # Save metadata
    save_metadata(storage_path, metadata)

    # Generate summary
    text = transcript_data.get("text", "")
    word_count = len(text.split())
    duration = transcript_data.get("audio_duration", 0)
    speaker_count = len(set(u["speaker"] for u in transcript_data.get("utterances", [])))

    summary = f"""Transcription complete!

Path: {storage_path}
Title: {metadata['title']}
Channel: {metadata['channel']}
Duration: {duration // 60}m {duration % 60}s
Words: {word_count}
Speakers: {speaker_count}

Preview (first 500 chars):
{text[:500]}{"..." if len(text) > 500 else ""}"""

    return summary


@mcp.tool()
def list_available_transcripts(
    platform: str | None = None,
    channel: str | None = None,
) -> str:
    """List all available transcripts.

    Args:
        platform: Optional filter by platform (youtube, vimeo, etc.)
        channel: Optional filter by channel name

    Returns:
        List of available transcripts with their metadata.
    """
    transcripts = list_transcripts(platform=platform, channel=channel)

    if not transcripts:
        return "No transcripts found."

    lines = [f"Found {len(transcripts)} transcript(s):\n"]

    for t in transcripts:
        line = f"- {t['platform']}/{t['channel']}/{t['title']}"
        if t.get("duration"):
            mins = t["duration"] // 60
            secs = t["duration"] % 60
            line += f" ({mins}m {secs}s)"
        lines.append(line)

    return "\n".join(lines)


@mcp.tool()
def read_transcript(path: str) -> str:
    """Read a transcript from disk.

    Args:
        path: Path to the transcript directory

    Returns:
        The transcript text content.
    """
    try:
        data = get_transcript(path)
        return data.get("text", "No text content found.")
    except FileNotFoundError as e:
        return f"Error: {e}"


@mcp.tool()
def get_transcript_details(path: str) -> str:
    """Get detailed transcript data including speaker information.

    Args:
        path: Path to the transcript directory

    Returns:
        Detailed transcript information with speakers and timestamps.
    """
    try:
        data = get_transcript(path)
        structured = data.get("structured", {})

        lines = []

        # Basic info
        if structured.get("audio_duration"):
            duration = structured["audio_duration"]
            lines.append(f"Duration: {duration // 60}m {duration % 60}s")

        if structured.get("confidence"):
            lines.append(f"Confidence: {structured['confidence']:.1%}")

        # Utterances with speakers
        utterances = structured.get("utterances", [])
        if utterances:
            lines.append(f"\nSpeakers: {len(set(u['speaker'] for u in utterances))}")
            lines.append("\n--- Transcript ---\n")

            for u in utterances:
                speaker = u.get("speaker", "?")
                text = u.get("text", "")
                start_ms = u.get("start", 0)
                start_sec = start_ms // 1000
                mins = start_sec // 60
                secs = start_sec % 60
                lines.append(f"[{mins:02d}:{secs:02d}] Speaker {speaker}: {text}\n")
        else:
            lines.append("\n--- Transcript ---\n")
            lines.append(data.get("text", "No content"))

        return "\n".join(lines)
    except FileNotFoundError as e:
        return f"Error: {e}"


# Run the server
if __name__ == "__main__":
    mcp.run()
