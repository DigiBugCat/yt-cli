"""Configuration management for yt-transcribe."""

import os
from pathlib import Path

from dotenv import load_dotenv

# Base directory - use ~/.yt-transcribe/ for global storage
DATA_DIR = Path(os.getenv("YT_TRANSCRIBE_DATA_DIR", Path.home() / ".yt-transcribe"))

# Load .env file from data directory if it exists
env_file = DATA_DIR / ".env"
if env_file.exists():
    load_dotenv(env_file)
else:
    load_dotenv()  # Try current directory

# AssemblyAI API Key
ASSEMBLYAI_API_KEY = os.getenv("ASSEMBLYAI_API_KEY")

# Storage paths
TRANSCRIPTS_DIR = DATA_DIR / "transcripts"
DOWNLOADS_DIR = DATA_DIR / ".downloads"
DATABASE_PATH = DATA_DIR / "transcripts.db"

# Firefox cookies configuration
# If FIREFOX_COOKIES_PATH is set, we're running in Docker mode
# and should use the mounted cookies file directly
FIREFOX_COOKIES_PATH = os.getenv("FIREFOX_COOKIES_PATH")
FIREFOX_PROFILE = os.getenv("FIREFOX_PROFILE", "default")


def is_docker_mode() -> bool:
    """Check if running in Docker mode (cookies mounted as volume)."""
    return FIREFOX_COOKIES_PATH is not None


def get_firefox_cookies_args() -> list[str]:
    """Get yt-dlp arguments for Firefox cookies.

    Returns appropriate args based on whether running locally or in Docker.
    """
    if is_docker_mode():
        # Docker mode: use mounted cookies file
        cookies_path = Path(FIREFOX_COOKIES_PATH)
        # Find the cookies.sqlite file in the profile directory
        for profile_dir in cookies_path.iterdir():
            if profile_dir.is_dir():
                cookies_file = profile_dir / "cookies.sqlite"
                if cookies_file.exists():
                    return ["--cookies", str(cookies_file)]
        # Fallback: try default profile pattern
        return ["--cookies", str(cookies_path / f"{FIREFOX_PROFILE}" / "cookies.sqlite")]
    else:
        # Local mode: let yt-dlp extract from browser
        return ["--cookies-from-browser", "firefox"]


def ensure_directories() -> None:
    """Create necessary directories if they don't exist."""
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    TRANSCRIPTS_DIR.mkdir(parents=True, exist_ok=True)
    DOWNLOADS_DIR.mkdir(parents=True, exist_ok=True)


def validate_config() -> None:
    """Validate required configuration."""
    if not ASSEMBLYAI_API_KEY:
        raise ValueError(
            "ASSEMBLYAI_API_KEY is required. "
            "Set it in .env file or as environment variable."
        )
