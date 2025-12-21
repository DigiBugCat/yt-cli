use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::config::{database_path, ensure_directories};
use crate::error::Result;

/// Initialize database tables
fn init_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- Main transcripts table
        CREATE TABLE IF NOT EXISTS transcripts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            video_id TEXT UNIQUE,
            url TEXT,
            title TEXT,
            channel TEXT,
            channel_handle TEXT,
            channel_id TEXT,
            platform TEXT,
            duration INTEGER,
            upload_date TEXT,
            description TEXT,
            thumbnail TEXT,
            view_count INTEGER,
            like_count INTEGER,
            transcribed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            path TEXT,
            speaker_count INTEGER,
            word_count INTEGER,
            confidence REAL
        );

        -- Full-text search table
        CREATE VIRTUAL TABLE IF NOT EXISTS transcripts_fts USING fts5(
            title,
            channel,
            description,
            transcript_text
        );
        "#,
    )?;

    // Migration: Remove chapters columns from existing databases
    migrate_remove_chapters(conn)?;

    // Migration: Add channel_handle column
    migrate_add_channel_handle(conn)?;

    Ok(())
}

/// Migration to remove chapters-related columns from existing databases
fn migrate_remove_chapters(conn: &Connection) -> Result<()> {
    // Check if 'chapters' column exists in transcripts table
    let has_chapters_column: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('transcripts') WHERE name = 'chapters'")?
        .exists([])?;

    if has_chapters_column {
        // SQLite doesn't support DROP COLUMN in older versions, so we recreate the table
        conn.execute_batch(
            r#"
            -- Recreate transcripts table without chapters column
            CREATE TABLE transcripts_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                video_id TEXT UNIQUE,
                url TEXT,
                title TEXT,
                channel TEXT,
                channel_id TEXT,
                platform TEXT,
                duration INTEGER,
                upload_date TEXT,
                description TEXT,
                thumbnail TEXT,
                view_count INTEGER,
                like_count INTEGER,
                transcribed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                path TEXT,
                speaker_count INTEGER,
                word_count INTEGER,
                confidence REAL
            );

            INSERT INTO transcripts_new (id, video_id, url, title, channel, channel_id, platform,
                duration, upload_date, description, thumbnail, view_count, like_count,
                transcribed_at, path, speaker_count, word_count, confidence)
            SELECT id, video_id, url, title, channel, channel_id, platform,
                duration, upload_date, description, thumbnail, view_count, like_count,
                transcribed_at, path, speaker_count, word_count, confidence
            FROM transcripts;

            DROP TABLE transcripts;
            ALTER TABLE transcripts_new RENAME TO transcripts;

            -- Recreate FTS table without chapters_text
            DROP TABLE IF EXISTS transcripts_fts;
            CREATE VIRTUAL TABLE transcripts_fts USING fts5(
                title,
                channel,
                description,
                transcript_text
            );
            "#,
        )?;
    }

    Ok(())
}

/// Migration to add channel_handle column to existing databases
fn migrate_add_channel_handle(conn: &Connection) -> Result<()> {
    // Check if 'channel_handle' column exists
    let has_channel_handle: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('transcripts') WHERE name = 'channel_handle'")?
        .exists([])?;

    if !has_channel_handle {
        conn.execute("ALTER TABLE transcripts ADD COLUMN channel_handle TEXT", [])?;
    }

    Ok(())
}

/// Get a database connection
pub fn get_connection() -> Result<Connection> {
    ensure_directories()?;
    let conn = Connection::open(database_path())?;
    init_tables(&conn)?;
    Ok(conn)
}

/// Metadata for adding a transcript
pub struct TranscriptMetadata<'a> {
    pub video_id: &'a str,
    pub url: &'a str,
    pub title: &'a str,
    pub channel: &'a str,
    pub channel_handle: Option<&'a str>,
    pub channel_id: Option<&'a str>,
    pub platform: &'a str,
    pub duration: Option<i64>,
    pub upload_date: Option<&'a str>,
    pub description: Option<&'a str>,
    pub thumbnail: Option<&'a str>,
    pub view_count: Option<i64>,
    pub like_count: Option<i64>,
    pub path: &'a str,
    pub speaker_count: i32,
    pub word_count: i32,
    pub confidence: Option<f64>,
    pub transcript_text: &'a str,
}

/// Add a transcript to the database
pub fn add_transcript(meta: &TranscriptMetadata) -> Result<i64> {
    let conn = get_connection()?;

    // Insert or replace the transcript
    conn.execute(
        r#"
        INSERT OR REPLACE INTO transcripts
        (video_id, url, title, channel, channel_handle, channel_id, platform, duration, upload_date,
         description, thumbnail, view_count, like_count, path, speaker_count, word_count, confidence)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
        "#,
        params![
            meta.video_id, meta.url, meta.title, meta.channel, meta.channel_handle, meta.channel_id,
            meta.platform, meta.duration, meta.upload_date, meta.description,
            meta.thumbnail, meta.view_count, meta.like_count, meta.path,
            meta.speaker_count, meta.word_count, meta.confidence
        ],
    )?;

    let transcript_id = conn.last_insert_rowid();

    // Update FTS with transcript text
    conn.execute(
        r#"
        INSERT OR REPLACE INTO transcripts_fts(rowid, title, channel, description, transcript_text)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        params![transcript_id, meta.title, meta.channel, meta.description.unwrap_or(""), meta.transcript_text],
    )?;

    Ok(transcript_id)
}

/// Search result
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: i64,
    pub video_id: String,
    pub title: String,
    pub channel: String,
    pub platform: String,
    pub duration: Option<i64>,
    pub path: String,
    pub snippet: Option<String>,
}

/// Search transcripts using full-text search
pub fn search_transcripts(query: &str, limit: i32) -> Result<Vec<SearchResult>> {
    let conn = get_connection()?;

    // Escape special FTS5 characters and wrap in quotes
    let escaped_query = format!("\"{}\"", query.replace('"', "\"\""));

    let mut stmt = conn.prepare(
        r#"
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
        WHERE transcripts_fts MATCH ?1
        ORDER BY rank
        LIMIT ?2
        "#,
    )?;

    let results = stmt
        .query_map(params![escaped_query, limit], |row| {
            Ok(SearchResult {
                id: row.get(0)?,
                video_id: row.get(1)?,
                title: row.get(2)?,
                channel: row.get(3)?,
                platform: row.get(4)?,
                duration: row.get(5)?,
                path: row.get(6)?,
                snippet: row.get(7)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(results)
}

/// Transcript listing from database
#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptRecord {
    pub id: i64,
    pub video_id: String,
    pub url: Option<String>,
    pub title: String,
    pub channel: String,
    pub channel_handle: Option<String>,
    pub platform: String,
    pub duration: Option<i64>,
    pub upload_date: Option<String>,
    pub path: String,
    pub speaker_count: Option<i32>,
    pub word_count: Option<i32>,
}

/// List all transcripts with optional filters
pub fn list_all_transcripts(
    platform: Option<&str>,
    channel: Option<&str>,
    handle: Option<&str>,
    limit: i32,
) -> Result<Vec<TranscriptRecord>> {
    let conn = get_connection()?;

    let mut query = "SELECT id, video_id, url, title, channel, channel_handle, platform, duration, upload_date, path, speaker_count, word_count FROM transcripts WHERE 1=1".to_string();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(p) = platform {
        query.push_str(" AND platform = ?");
        params_vec.push(Box::new(p.to_string()));
    }

    if let Some(c) = channel {
        query.push_str(" AND channel LIKE ?");
        params_vec.push(Box::new(format!("%{}%", c)));
    }

    if let Some(h) = handle {
        query.push_str(" AND channel_handle LIKE ?");
        params_vec.push(Box::new(format!("%{}%", h)));
    }

    query.push_str(" ORDER BY transcribed_at DESC LIMIT ?");
    params_vec.push(Box::new(limit));

    let mut stmt = conn.prepare(&query)?;

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let results = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(TranscriptRecord {
                id: row.get(0)?,
                video_id: row.get(1)?,
                url: row.get(2)?,
                title: row.get(3)?,
                channel: row.get(4)?,
                channel_handle: row.get(5)?,
                platform: row.get(6)?,
                duration: row.get(7)?,
                upload_date: row.get(8)?,
                path: row.get(9)?,
                speaker_count: row.get(10)?,
                word_count: row.get(11)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(results)
}

/// Get a transcript by video ID
pub fn get_transcript_by_id(video_id: &str) -> Result<Option<TranscriptRecord>> {
    let conn = get_connection()?;

    let mut stmt = conn.prepare(
        "SELECT id, video_id, url, title, channel, channel_handle, platform, duration, upload_date, path, speaker_count, word_count FROM transcripts WHERE video_id = ?",
    )?;

    let mut rows = stmt.query(params![video_id])?;

    if let Some(row) = rows.next()? {
        Ok(Some(TranscriptRecord {
            id: row.get(0)?,
            video_id: row.get(1)?,
            url: row.get(2)?,
            title: row.get(3)?,
            channel: row.get(4)?,
            channel_handle: row.get(5)?,
            platform: row.get(6)?,
            duration: row.get(7)?,
            upload_date: row.get(8)?,
            path: row.get(9)?,
            speaker_count: row.get(10)?,
            word_count: row.get(11)?,
        }))
    } else {
        Ok(None)
    }
}

/// Database statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct Stats {
    pub total_transcripts: i64,
    pub unique_channels: i64,
    pub unique_platforms: i64,
    pub total_duration: Option<i64>,
    pub total_words: Option<i64>,
}

/// Get database statistics
pub fn get_stats() -> Result<Stats> {
    let conn = get_connection()?;

    let mut stmt = conn.prepare(
        r#"
        SELECT
            COUNT(*) as total_transcripts,
            COUNT(DISTINCT channel) as unique_channels,
            COUNT(DISTINCT platform) as unique_platforms,
            SUM(duration) as total_duration,
            SUM(word_count) as total_words
        FROM transcripts
        "#,
    )?;

    let stats = stmt.query_row([], |row| {
        Ok(Stats {
            total_transcripts: row.get(0)?,
            unique_channels: row.get(1)?,
            unique_platforms: row.get(2)?,
            total_duration: row.get(3)?,
            total_words: row.get(4)?,
        })
    })?;

    Ok(stats)
}

/// Delete a transcript from the database
pub fn delete_transcript(video_id: &str) -> Result<bool> {
    let conn = get_connection()?;

    let changes = conn.execute(
        "DELETE FROM transcripts WHERE video_id = ?",
        params![video_id],
    )?;

    Ok(changes > 0)
}
