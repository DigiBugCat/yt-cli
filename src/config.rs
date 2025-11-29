use std::path::PathBuf;
use std::sync::OnceLock;

use crate::error::{Error, Result};

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Get the base data directory (~/.yt-transcribe/)
pub fn data_dir() -> &'static PathBuf {
    DATA_DIR.get_or_init(|| {
        std::env::var("YT_TRANSCRIBE_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .expect("Could not determine home directory")
                    .join(".yt-transcribe")
            })
    })
}

/// Get the transcripts directory
pub fn transcripts_dir() -> PathBuf {
    data_dir().join("transcripts")
}

/// Get the downloads directory (temporary)
pub fn downloads_dir() -> PathBuf {
    data_dir().join(".downloads")
}

/// Get the database path
pub fn database_path() -> PathBuf {
    data_dir().join("transcripts.db")
}

/// Get the .env file path
pub fn env_file_path() -> PathBuf {
    data_dir().join(".env")
}

/// Load environment variables from the data directory's .env file
pub fn load_env() {
    let env_path = env_file_path();
    if env_path.exists() {
        let _ = dotenvy::from_path(&env_path);
    } else {
        // Try current directory as fallback
        let _ = dotenvy::dotenv();
    }
}

/// Get the AssemblyAI API key
pub fn assemblyai_api_key() -> Option<String> {
    std::env::var("ASSEMBLYAI_API_KEY").ok()
}

/// Validate that required configuration is present
pub fn validate_config() -> Result<()> {
    if assemblyai_api_key().is_none() {
        return Err(Error::ApiKeyMissing);
    }
    Ok(())
}

/// Create necessary directories if they don't exist
pub fn ensure_directories() -> Result<()> {
    std::fs::create_dir_all(data_dir())?;
    std::fs::create_dir_all(transcripts_dir())?;
    std::fs::create_dir_all(downloads_dir())?;
    Ok(())
}

/// Check if running in Docker mode (cookies mounted as volume)
pub fn is_docker_mode() -> bool {
    std::env::var("FIREFOX_COOKIES_PATH").is_ok()
}

/// Get yt-dlp arguments for Firefox cookies
pub fn firefox_cookies_args() -> Vec<String> {
    if let Ok(cookies_path) = std::env::var("FIREFOX_COOKIES_PATH") {
        // Docker mode: use mounted cookies file
        let path = PathBuf::from(&cookies_path);
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let cookies_file = entry.path().join("cookies.sqlite");
                    if cookies_file.exists() {
                        return vec![
                            "--cookies".to_string(),
                            cookies_file.to_string_lossy().to_string(),
                        ];
                    }
                }
            }
        }
        // Fallback
        let profile = std::env::var("FIREFOX_PROFILE").unwrap_or_else(|_| "default".to_string());
        vec![
            "--cookies".to_string(),
            format!("{}/{}/cookies.sqlite", cookies_path, profile),
        ]
    } else {
        // Local mode: let yt-dlp extract from browser
        vec!["--cookies-from-browser".to_string(), "firefox".to_string()]
    }
}
