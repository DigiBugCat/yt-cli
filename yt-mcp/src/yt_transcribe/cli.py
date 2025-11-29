"""CLI for yt-transcribe."""

import argparse
import sys

from .config import DATA_DIR, ensure_directories, validate_config
from .downloader import download_audio
from .storage import (
    create_storage_path,
    get_platform_from_url,
    get_transcript,
    index_transcript,
    list_transcripts,
    move_audio_file,
    save_metadata,
    save_transcript,
)
from .transcriber import format_transcript, transcribe_audio


def cmd_transcribe(args):
    """Download and transcribe a video."""
    validate_config()
    ensure_directories()

    print(f"Downloading: {args.url}", file=sys.stderr)
    audio_file, metadata = download_audio(args.url)
    print(f"Downloaded: {metadata['title']}", file=sys.stderr)
    print(f"Channel: {metadata['channel']}", file=sys.stderr)
    print(f"Duration: {metadata['duration']}s", file=sys.stderr)

    print("\nTranscribing with AssemblyAI...", file=sys.stderr)
    transcript_data = transcribe_audio(audio_file)
    print("Transcription complete!", file=sys.stderr)

    # Create storage path
    platform = get_platform_from_url(args.url)
    storage_path = create_storage_path(
        platform=platform,
        channel=metadata["channel"],
        title=metadata["title"],
    )

    # Move audio and save files
    move_audio_file(audio_file, storage_path)
    formatted_text = format_transcript(transcript_data)
    save_transcript(storage_path, formatted_text, transcript_data)
    save_metadata(storage_path, metadata)

    # Index in database
    index_transcript(metadata, transcript_data, storage_path)
    print("Indexed in database.", file=sys.stderr)

    # Output result
    text = transcript_data.get("text", "")
    word_count = len(text.split())
    duration = transcript_data.get("audio_duration", 0)
    speaker_count = len(set(u["speaker"] for u in transcript_data.get("utterances", [])))

    print(f"""
Transcription complete!

Path: {storage_path}
Title: {metadata['title']}
Channel: {metadata['channel']}
Duration: {duration // 60}m {duration % 60}s
Words: {word_count}
Speakers: {speaker_count}

Preview (first 500 chars):
{text[:500]}{"..." if len(text) > 500 else ""}
""")


def cmd_list(args):
    """List available transcripts."""
    transcripts = list_transcripts(platform=args.platform, channel=args.channel)

    if not transcripts:
        print("No transcripts found.")
        return

    print(f"Found {len(transcripts)} transcript(s):\n")

    for t in transcripts:
        line = f"- {t['platform']}/{t['channel']}/{t['title']}"
        if t.get("duration"):
            mins = t["duration"] // 60
            secs = t["duration"] % 60
            line += f" ({mins}m {secs}s)"
        print(line)
        print(f"  Path: {t['path']}")


def cmd_read(args):
    """Read a transcript."""
    try:
        data = get_transcript(args.path)

        if args.json:
            import json
            print(json.dumps(data.get("structured", {}), indent=2))
        else:
            print(data.get("text", "No text content found."))
    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


def cmd_search(args):
    """Search transcripts using full-text search."""
    from .database import search_transcripts

    results = search_transcripts(args.query, limit=args.limit)

    if not results:
        print(f"No results found for: {args.query}")
        return

    print(f"Found {len(results)} result(s) for '{args.query}':\n")

    for r in results:
        duration = r.get("duration", 0)
        mins = duration // 60 if duration else 0
        secs = duration % 60 if duration else 0

        print(f"- {r['channel']}: {r['title']} ({mins}m {secs}s)")
        print(f"  Path: {r['path']}")
        if r.get("snippet"):
            print(f"  Match: {r['snippet']}")
        print()


def cmd_stats(args):
    """Show database statistics."""
    from .database import get_stats

    stats = get_stats()

    if not stats or not stats.get("total_transcripts"):
        print("No transcripts in database yet.")
        print(f"\nData directory: {DATA_DIR}")
        return

    total_duration = stats.get("total_duration", 0) or 0
    hours = total_duration // 3600
    mins = (total_duration % 3600) // 60

    print(f"Transcript Database Statistics")
    print(f"==============================")
    print(f"Total transcripts: {stats.get('total_transcripts', 0)}")
    print(f"Unique channels:   {stats.get('unique_channels', 0)}")
    print(f"Unique platforms:  {stats.get('unique_platforms', 0)}")
    print(f"Total duration:    {hours}h {mins}m")
    print(f"Total words:       {stats.get('total_words', 0):,}")
    print(f"\nData directory: {DATA_DIR}")


def cmd_reindex(args):
    """Reindex all transcripts in the database."""
    import json
    from pathlib import Path

    from .config import TRANSCRIPTS_DIR
    from .database import add_transcript

    ensure_directories()

    if not TRANSCRIPTS_DIR.exists():
        print("No transcripts directory found.")
        return

    count = 0
    for transcript_json in TRANSCRIPTS_DIR.rglob("transcript.json"):
        video_dir = transcript_json.parent
        metadata_file = video_dir / "metadata.json"

        try:
            with open(transcript_json, encoding="utf-8") as f:
                transcript_data = json.load(f)

            metadata = {}
            if metadata_file.exists():
                with open(metadata_file, encoding="utf-8") as f:
                    metadata = json.load(f)

            text = transcript_data.get("text", "")
            utterances = transcript_data.get("utterances", [])
            speaker_count = len(set(u["speaker"] for u in utterances)) if utterances else 0
            word_count = len(text.split())

            # Get platform from path structure
            rel_path = video_dir.relative_to(TRANSCRIPTS_DIR)
            parts = rel_path.parts
            platform = parts[0] if len(parts) >= 1 else "unknown"
            channel = parts[1] if len(parts) >= 2 else "Unknown"

            add_transcript(
                video_id=metadata.get("id", video_dir.name),
                url=metadata.get("url", ""),
                title=metadata.get("title", video_dir.name),
                channel=metadata.get("channel", channel),
                platform=platform,
                duration=metadata.get("duration", 0),
                upload_date=metadata.get("upload_date"),
                path=str(video_dir),
                speaker_count=speaker_count,
                word_count=word_count,
                transcript_text=text,
            )
            count += 1
            print(f"Indexed: {video_dir.name}")

        except (json.JSONDecodeError, OSError) as e:
            print(f"Error indexing {video_dir}: {e}", file=sys.stderr)

    print(f"\nReindexed {count} transcript(s).")


def cmd_init(args):
    """Initialize yt-transcribe with API key."""
    ensure_directories()

    env_file = DATA_DIR / ".env"

    if env_file.exists() and not args.force:
        print(f"Config already exists at {env_file}")
        print("Use --force to overwrite.")
        return

    api_key = args.api_key or input("Enter your AssemblyAI API key: ").strip()

    if not api_key:
        print("Error: API key is required.", file=sys.stderr)
        sys.exit(1)

    env_file.write_text(f"ASSEMBLYAI_API_KEY={api_key}\n")
    print(f"Config saved to {env_file}")
    print(f"Data directory: {DATA_DIR}")


def main():
    parser = argparse.ArgumentParser(
        prog="yt-transcribe",
        description="Download and transcribe videos using yt-dlp and AssemblyAI",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    # transcribe command
    p_transcribe = subparsers.add_parser(
        "transcribe",
        help="Download and transcribe a video",
    )
    p_transcribe.add_argument("url", help="Video URL to transcribe")
    p_transcribe.set_defaults(func=cmd_transcribe)

    # list command
    p_list = subparsers.add_parser(
        "list",
        help="List available transcripts",
    )
    p_list.add_argument("--platform", "-p", help="Filter by platform")
    p_list.add_argument("--channel", "-c", help="Filter by channel")
    p_list.set_defaults(func=cmd_list)

    # read command
    p_read = subparsers.add_parser(
        "read",
        help="Read a transcript",
    )
    p_read.add_argument("path", help="Path to transcript directory")
    p_read.add_argument("--json", "-j", action="store_true", help="Output as JSON with timestamps")
    p_read.set_defaults(func=cmd_read)

    # search command
    p_search = subparsers.add_parser(
        "search",
        help="Search transcripts using full-text search",
    )
    p_search.add_argument("query", help="Search query")
    p_search.add_argument("--limit", "-n", type=int, default=20, help="Max results (default: 20)")
    p_search.set_defaults(func=cmd_search)

    # stats command
    p_stats = subparsers.add_parser(
        "stats",
        help="Show database statistics",
    )
    p_stats.set_defaults(func=cmd_stats)

    # init command
    p_init = subparsers.add_parser(
        "init",
        help="Initialize with AssemblyAI API key",
    )
    p_init.add_argument("--api-key", "-k", help="AssemblyAI API key")
    p_init.add_argument("--force", "-f", action="store_true", help="Overwrite existing config")
    p_init.set_defaults(func=cmd_init)

    # reindex command
    p_reindex = subparsers.add_parser(
        "reindex",
        help="Reindex all transcripts in the database",
    )
    p_reindex.set_defaults(func=cmd_reindex)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
