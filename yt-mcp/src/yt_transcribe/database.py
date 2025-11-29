"""SQLite database with full-text search for transcripts."""

import sqlite3
from pathlib import Path
from typing import Any

from .config import DATABASE_PATH, ensure_directories


def get_connection() -> sqlite3.Connection:
    """Get database connection, creating tables if needed."""
    ensure_directories()
    conn = sqlite3.connect(DATABASE_PATH)
    conn.row_factory = sqlite3.Row
    _init_tables(conn)
    return conn


def _init_tables(conn: sqlite3.Connection) -> None:
    """Initialize database tables."""
    conn.executescript("""
        -- Main transcripts table
        CREATE TABLE IF NOT EXISTS transcripts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            video_id TEXT UNIQUE,
            url TEXT,
            title TEXT,
            channel TEXT,
            platform TEXT,
            duration INTEGER,
            upload_date TEXT,
            transcribed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            path TEXT,
            speaker_count INTEGER,
            word_count INTEGER
        );

        -- Full-text search table (stores its own content)
        CREATE VIRTUAL TABLE IF NOT EXISTS transcripts_fts USING fts5(
            title,
            channel,
            transcript_text
        );
    """)
    conn.commit()


def add_transcript(
    video_id: str,
    url: str,
    title: str,
    channel: str,
    platform: str,
    duration: int,
    upload_date: str | None,
    path: str,
    speaker_count: int,
    word_count: int,
    transcript_text: str,
) -> int:
    """Add a transcript to the database.

    Returns the transcript ID.
    """
    conn = get_connection()
    cursor = conn.cursor()

    # Insert or replace the transcript
    cursor.execute("""
        INSERT OR REPLACE INTO transcripts
        (video_id, url, title, channel, platform, duration, upload_date, path, speaker_count, word_count)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, (video_id, url, title, channel, platform, duration, upload_date, path, speaker_count, word_count))

    transcript_id = cursor.lastrowid

    # Update FTS with transcript text
    cursor.execute("""
        INSERT OR REPLACE INTO transcripts_fts(rowid, title, channel, transcript_text)
        VALUES (?, ?, ?, ?)
    """, (transcript_id, title, channel, transcript_text))

    conn.commit()
    conn.close()

    return transcript_id


def search_transcripts(query: str, limit: int = 20) -> list[dict[str, Any]]:
    """Search transcripts using full-text search.

    Args:
        query: Search query
        limit: Maximum results to return

    Returns:
        List of matching transcripts with snippets
    """
    conn = get_connection()
    cursor = conn.cursor()

    # Escape special FTS5 characters and wrap in quotes for exact matching
    escaped_query = '"' + query.replace('"', '""') + '"'

    cursor.execute("""
        SELECT
            t.id,
            t.video_id,
            t.title,
            t.channel,
            t.platform,
            t.duration,
            t.path,
            snippet(transcripts_fts, 2, '>>> ', ' <<<', '...', 32) as snippet
        FROM transcripts_fts
        JOIN transcripts t ON transcripts_fts.rowid = t.id
        WHERE transcripts_fts MATCH ?
        ORDER BY rank
        LIMIT ?
    """, (escaped_query, limit))

    results = [dict(row) for row in cursor.fetchall()]
    conn.close()

    return results


def list_all_transcripts(
    platform: str | None = None,
    channel: str | None = None,
    limit: int = 100,
) -> list[dict[str, Any]]:
    """List all transcripts with optional filters."""
    conn = get_connection()
    cursor = conn.cursor()

    query = "SELECT * FROM transcripts WHERE 1=1"
    params = []

    if platform:
        query += " AND platform = ?"
        params.append(platform)

    if channel:
        query += " AND channel LIKE ?"
        params.append(f"%{channel}%")

    query += " ORDER BY transcribed_at DESC LIMIT ?"
    params.append(limit)

    cursor.execute(query, params)
    results = [dict(row) for row in cursor.fetchall()]
    conn.close()

    return results


def get_transcript_by_id(video_id: str) -> dict[str, Any] | None:
    """Get a transcript by video ID."""
    conn = get_connection()
    cursor = conn.cursor()

    cursor.execute("SELECT * FROM transcripts WHERE video_id = ?", (video_id,))
    row = cursor.fetchone()
    conn.close()

    return dict(row) if row else None


def get_transcript_by_path(path: str) -> dict[str, Any] | None:
    """Get a transcript by path."""
    conn = get_connection()
    cursor = conn.cursor()

    cursor.execute("SELECT * FROM transcripts WHERE path = ?", (path,))
    row = cursor.fetchone()
    conn.close()

    return dict(row) if row else None


def delete_transcript(video_id: str) -> bool:
    """Delete a transcript from the database."""
    conn = get_connection()
    cursor = conn.cursor()

    cursor.execute("DELETE FROM transcripts WHERE video_id = ?", (video_id,))
    deleted = cursor.rowcount > 0

    conn.commit()
    conn.close()

    return deleted


def get_stats() -> dict[str, Any]:
    """Get database statistics."""
    conn = get_connection()
    cursor = conn.cursor()

    cursor.execute("""
        SELECT
            COUNT(*) as total_transcripts,
            COUNT(DISTINCT channel) as unique_channels,
            COUNT(DISTINCT platform) as unique_platforms,
            SUM(duration) as total_duration,
            SUM(word_count) as total_words
        FROM transcripts
    """)

    row = cursor.fetchone()
    conn.close()

    return dict(row) if row else {}
