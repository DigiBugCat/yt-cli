"""Video downloading with yt-dlp (using system binary)."""

import json
import shutil
import subprocess
import uuid
from pathlib import Path
from typing import Any

from .config import DOWNLOADS_DIR, ensure_directories, get_firefox_cookies_args

# Find system yt-dlp binary
YTDLP_PATH = shutil.which("yt-dlp") or "/opt/homebrew/bin/yt-dlp"


def _run_ytdlp(args: list[str]) -> subprocess.CompletedProcess:
    """Run yt-dlp with the given arguments."""
    cmd = [YTDLP_PATH] + get_firefox_cookies_args() + args
    return subprocess.run(cmd, capture_output=True, text=True)


def extract_metadata(url: str) -> dict[str, Any]:
    """Extract video metadata without downloading.

    Args:
        url: Video URL

    Returns:
        Dictionary containing video metadata
    """
    result = _run_ytdlp([
        "--dump-json",
        "--no-download",
        url,
    ])

    if result.returncode != 0:
        raise ValueError(f"Could not extract metadata: {result.stderr}")

    info = json.loads(result.stdout)

    return {
        "id": info.get("id"),
        "title": info.get("title", "Unknown Title"),
        "channel": info.get("channel") or info.get("uploader") or "Unknown Channel",
        "uploader": info.get("uploader"),
        "uploader_id": info.get("uploader_id"),
        "duration": info.get("duration"),
        "upload_date": info.get("upload_date"),
        "description": info.get("description"),
        "view_count": info.get("view_count"),
        "like_count": info.get("like_count"),
        "thumbnail": info.get("thumbnail"),
        "url": url,
        "webpage_url": info.get("webpage_url", url),
        "extractor": info.get("extractor"),
    }


def download_audio(url: str) -> tuple[Path, dict[str, Any]]:
    """Download audio from a video URL.

    Args:
        url: Video URL

    Returns:
        Tuple of (audio_file_path, metadata)
    """
    ensure_directories()

    # Generate unique filename to avoid collisions
    output_id = str(uuid.uuid4())[:8]
    output_template = str(DOWNLOADS_DIR / f"{output_id}.%(ext)s")

    # Download with yt-dlp, also dump metadata
    result = _run_ytdlp([
        "-f", "bestaudio",
        "-x", "--audio-format", "mp3",
        "--print-json",
        "-o", output_template,
        url,
    ])

    if result.returncode != 0:
        raise ValueError(f"Could not download audio: {result.stderr}")

    # Parse metadata from output
    info = json.loads(result.stdout)

    # Find the downloaded file
    audio_file = DOWNLOADS_DIR / f"{output_id}.mp3"
    if not audio_file.exists():
        # Try to find any file with the output_id prefix
        for f in DOWNLOADS_DIR.glob(f"{output_id}.*"):
            audio_file = f
            break

    if not audio_file.exists():
        raise FileNotFoundError(f"Downloaded audio file not found for {url}")

    metadata = {
        "id": info.get("id"),
        "title": info.get("title", "Unknown Title"),
        "channel": info.get("channel") or info.get("uploader") or "Unknown Channel",
        "uploader": info.get("uploader"),
        "uploader_id": info.get("uploader_id"),
        "duration": info.get("duration"),
        "upload_date": info.get("upload_date"),
        "description": info.get("description"),
        "view_count": info.get("view_count"),
        "like_count": info.get("like_count"),
        "thumbnail": info.get("thumbnail"),
        "url": url,
        "webpage_url": info.get("webpage_url", url),
        "extractor": info.get("extractor"),
    }

    return audio_file, metadata
