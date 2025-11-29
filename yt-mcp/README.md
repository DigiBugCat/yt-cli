# yt-transcribe

MCP server for downloading and transcribing videos using yt-dlp and AssemblyAI.

## Features

- Download videos from YouTube, Vimeo, and other platforms via yt-dlp
- Transcribe audio using AssemblyAI with speaker diarization
- Organized file storage: `transcripts/{platform}/{channel}/{video_title}/`
- Firefox cookies support for accessing private/age-restricted content
- Docker support

## Installation

```bash
# Clone and install
cd yt-mcp
uv sync
```

## Configuration

1. Copy `.env.example` to `.env`
2. Add your AssemblyAI API key:
   ```
   ASSEMBLYAI_API_KEY=your_key_here
   ```

## Usage

### Run locally

```bash
uv run fastmcp run src/yt_transcribe/server.py
```

### Run with Docker

```bash
# Edit docker-compose.yml to uncomment your OS's Firefox path
docker compose up
```

### Claude Desktop Configuration

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "yt-transcribe": {
      "command": "uv",
      "args": ["--directory", "/path/to/yt-mcp", "run", "fastmcp", "run", "src/yt_transcribe/server.py"],
      "env": {
        "ASSEMBLYAI_API_KEY": "your_key"
      }
    }
  }
}
```

## MCP Tools

- `transcribe_video(url)` - Download and transcribe a video
- `list_available_transcripts(platform?, channel?)` - List all transcripts
- `read_transcript(path)` - Read transcript text
- `get_transcript_details(path)` - Get detailed transcript with speakers/timestamps

## File Structure

```
transcripts/
└── youtube/
    └── ChannelName/
        └── VideoTitle/
            ├── audio.mp3
            ├── transcript.txt
            ├── transcript.json
            └── metadata.json
```
