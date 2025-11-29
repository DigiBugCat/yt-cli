"""File storage and organization for transcripts."""

import json
import re
import shutil
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

from .config import TRANSCRIPTS_DIR


def sanitize_filename(name: str, max_length: int = 100) -> str:
    """Clean a string for use as a filename.

    Args:
        name: The string to sanitize
        max_length: Maximum length of the filename

    Returns:
        A filesystem-safe string
    """
    # Replace problematic characters with underscores
    sanitized = re.sub(r'[<>:"/\\|?*]', "_", name)
    # Replace multiple spaces/underscores with single underscore
    sanitized = re.sub(r"[\s_]+", "_", sanitized)
    # Remove leading/trailing underscores and spaces
    sanitized = sanitized.strip("_ ")
    # Truncate to max length
    if len(sanitized) > max_length:
        sanitized = sanitized[:max_length].rstrip("_")
    return sanitized or "untitled"


def get_platform_from_url(url: str) -> str:
    """Detect the platform from a video URL.

    Args:
        url: The video URL

    Returns:
        Platform name (youtube, vimeo, twitter, etc.)
    """
    parsed = urlparse(url)
    domain = parsed.netloc.lower()

    # Remove www. prefix
    if domain.startswith("www."):
        domain = domain[4:]

    # Map domains to platform names
    platform_map = {
        "youtube.com": "youtube",
        "youtu.be": "youtube",
        "vimeo.com": "vimeo",
        "twitter.com": "twitter",
        "x.com": "twitter",
        "twitch.tv": "twitch",
        "dailymotion.com": "dailymotion",
        "facebook.com": "facebook",
        "fb.watch": "facebook",
        "instagram.com": "instagram",
        "tiktok.com": "tiktok",
    }

    for key, platform in platform_map.items():
        if key in domain:
            return platform

    # Default to domain name without TLD
    return domain.split(".")[0] or "unknown"


def create_storage_path(
    platform: str,
    channel: str,
    title: str,
) -> Path:
    """Generate organized storage path for a video.

    Args:
        platform: Platform name (youtube, vimeo, etc.)
        channel: Channel/uploader name
        title: Video title

    Returns:
        Path to the storage directory
    """
    safe_channel = sanitize_filename(channel)
    safe_title = sanitize_filename(title)

    storage_path = TRANSCRIPTS_DIR / platform / safe_channel / safe_title
    storage_path.mkdir(parents=True, exist_ok=True)

    return storage_path


def save_transcript(
    storage_path: Path,
    text: str,
    structured_data: dict[str, Any],
) -> tuple[Path, Path]:
    """Save transcript in both text and JSON formats.

    Args:
        storage_path: Directory to save files
        text: Plain text transcript (with speaker labels)
        structured_data: Full transcript data with timestamps, speakers, etc.

    Returns:
        Tuple of (text_path, json_path)
    """
    text_path = storage_path / "transcript.txt"
    json_path = storage_path / "transcript.json"

    text_path.write_text(text, encoding="utf-8")

    with open(json_path, "w", encoding="utf-8") as f:
        json.dump(structured_data, f, indent=2, ensure_ascii=False)

    return text_path, json_path


def save_metadata(storage_path: Path, metadata: dict[str, Any]) -> Path:
    """Save video metadata as JSON.

    Args:
        storage_path: Directory to save file
        metadata: Video metadata dictionary

    Returns:
        Path to metadata file
    """
    metadata_path = storage_path / "metadata.json"

    with open(metadata_path, "w", encoding="utf-8") as f:
        json.dump(metadata, f, indent=2, ensure_ascii=False)

    return metadata_path


def move_audio_file(source: Path, storage_path: Path) -> Path:
    """Move audio file to storage directory.

    Args:
        source: Source audio file path
        storage_path: Destination directory

    Returns:
        Path to moved audio file
    """
    dest = storage_path / "audio.mp3"
    shutil.move(str(source), str(dest))
    return dest


def list_transcripts(
    platform: str | None = None,
    channel: str | None = None,
) -> list[dict[str, Any]]:
    """List available transcripts.

    Args:
        platform: Optional platform filter
        channel: Optional channel filter

    Returns:
        List of transcript info dictionaries
    """
    results = []

    if not TRANSCRIPTS_DIR.exists():
        return results

    # Determine search path based on filters
    if platform and channel:
        search_paths = [TRANSCRIPTS_DIR / platform / sanitize_filename(channel)]
    elif platform:
        search_paths = [TRANSCRIPTS_DIR / platform]
    else:
        search_paths = [TRANSCRIPTS_DIR]

    for search_path in search_paths:
        if not search_path.exists():
            continue

        # Find all transcript.json files
        for transcript_file in search_path.rglob("transcript.json"):
            video_dir = transcript_file.parent
            metadata_file = video_dir / "metadata.json"

            info = {
                "path": str(video_dir),
                "title": video_dir.name,
                "channel": video_dir.parent.name,
                "platform": video_dir.parent.parent.name,
            }

            # Add metadata if available
            if metadata_file.exists():
                try:
                    with open(metadata_file) as f:
                        metadata = json.load(f)
                    info["duration"] = metadata.get("duration")
                    info["upload_date"] = metadata.get("upload_date")
                    info["url"] = metadata.get("url")
                except (json.JSONDecodeError, OSError):
                    pass

            results.append(info)

    return results


def index_transcript(
    metadata: dict[str, Any],
    transcript_data: dict[str, Any],
    storage_path: Path,
) -> int:
    """Index a transcript in the database.

    Args:
        metadata: Video metadata
        transcript_data: Transcript data with text and utterances
        storage_path: Path where transcript is stored

    Returns:
        Database ID of the transcript
    """
    from .database import add_transcript

    text = transcript_data.get("text", "")
    utterances = transcript_data.get("utterances", [])
    speaker_count = len(set(u["speaker"] for u in utterances)) if utterances else 0
    word_count = len(text.split())

    return add_transcript(
        video_id=metadata.get("id", ""),
        url=metadata.get("url", ""),
        title=metadata.get("title", "Unknown"),
        channel=metadata.get("channel", "Unknown"),
        platform=get_platform_from_url(metadata.get("url", "")),
        duration=metadata.get("duration", 0),
        upload_date=metadata.get("upload_date"),
        path=str(storage_path),
        speaker_count=speaker_count,
        word_count=word_count,
        transcript_text=text,
    )


def get_transcript(path: str) -> dict[str, Any]:
    """Get transcript content from a path.

    Args:
        path: Path to transcript directory or file

    Returns:
        Dictionary with transcript text and structured data
    """
    path = Path(path)

    # If path is a directory, look for transcript files
    if path.is_dir():
        text_file = path / "transcript.txt"
        json_file = path / "transcript.json"
    else:
        # Assume it's a file path
        if path.suffix == ".txt":
            text_file = path
            json_file = path.with_suffix(".json")
        else:
            json_file = path
            text_file = path.with_suffix(".txt")

    result = {}

    if text_file.exists():
        result["text"] = text_file.read_text(encoding="utf-8")

    if json_file.exists():
        with open(json_file, encoding="utf-8") as f:
            result["structured"] = json.load(f)

    if not result:
        raise FileNotFoundError(f"No transcript found at {path}")

    return result
