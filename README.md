# yt-cli

Download and transcribe videos using yt-dlp and AssemblyAI with speaker diarization and auto-chapters.

## Features

- Download audio from YouTube and other platforms via yt-dlp
- Transcribe with AssemblyAI (speaker labels, punctuation, auto-chapters)
- Full-text search across all transcripts
- Organized storage by platform/channel/video
- Firefox cookie support for members-only content

## Requirements

- [yt-dlp](https://github.com/yt-dlp/yt-dlp): `brew install yt-dlp`
- [AssemblyAI API key](https://www.assemblyai.com/)

## Installation

```bash
cargo install --path .
```

## Setup

```bash
# Initialize with your AssemblyAI API key
yt-cli init -k YOUR_API_KEY

# Or set environment variable directly
export ASSEMBLYAI_API_KEY=YOUR_API_KEY

# Optionally customize data directory (default: ~/.yt-transcribe)
export YT_TRANSCRIBE_DATA_DIR=/path/to/data
```

## Usage

```bash
# Transcribe a video
yt-cli transcribe https://www.youtube.com/watch?v=VIDEO_ID

# List all transcripts
yt-cli list

# Filter by platform or channel
yt-cli list --platform youtube
yt-cli list --channel "Channel Name"

# Search transcripts
yt-cli search "search query"

# Read a transcript
yt-cli read /path/to/transcript

# Get transcript path for a URL
yt-cli get https://www.youtube.com/watch?v=VIDEO_ID

# Show statistics
yt-cli stats

# Reindex all transcripts
yt-cli reindex
```

## Storage

Transcripts are stored at `~/.yt-transcribe/`:

```
~/.yt-transcribe/
├── .env                    # API key
├── transcripts.db          # SQLite with FTS5 search
└── transcripts/
    └── {platform}/{channel}/{video_id}/
        ├── metadata.json
        ├── transcript.md
        ├── transcript.json
        └── audio.mp3
```

## License

MIT
