# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Run Commands

```bash
# Build
cargo build
cargo build --release

# Run CLI
cargo run -- <command>
cargo run -- transcribe <url>
cargo run -- list
cargo run -- search "query"

# Check/lint
cargo check
cargo clippy
```

## Architecture

yt-cli is a Rust CLI tool that downloads videos using yt-dlp and transcribes them using AssemblyAI's API with speaker diarization and auto-chapters.

### Core Modules

- **main.rs**: CLI entry point using clap with subcommands (transcribe, list, read, search, stats, init, reindex, get)
- **transcriber.rs**: AssemblyAI client - uploads audio, polls for completion, returns structured transcript data with utterances, words, and chapters
- **downloader.rs**: Wraps yt-dlp to extract metadata and download audio as MP3. Supports Firefox cookies for members-only content
- **database.rs**: SQLite storage with FTS5 full-text search on transcript content
- **storage.rs**: File organization at `~/.yt-transcribe/transcripts/{platform}/{channel}/{video_id}/`
- **config.rs**: Environment and path configuration. Data stored in `~/.yt-transcribe/`

### Data Flow

1. `transcribe` command: URL → yt-dlp (metadata + audio) → AssemblyAI upload → poll completion → save markdown/JSON + index in SQLite
2. `search` command: FTS5 query on indexed transcript text, descriptions, and chapter headlines

### External Dependencies

- **yt-dlp**: Must be installed (`brew install yt-dlp`)
- **AssemblyAI API**: Requires API key via `yt-cli init` or `ASSEMBLYAI_API_KEY` env var

### File Storage Structure

```
~/.yt-transcribe/
├── .env                          # API key
├── transcripts.db                # SQLite with FTS5
├── .downloads/                   # Temporary audio files
└── transcripts/{platform}/{channel}/{video_id}/
    ├── metadata.json
    ├── transcript.md
    ├── transcript.json
    └── audio.mp3
```
