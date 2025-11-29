use clap::{Parser, Subcommand};

use yt_cli::commands;
use yt_cli::config::load_env;

#[derive(Parser)]
#[command(name = "yt-cli")]
#[command(about = "Download and transcribe videos using yt-dlp and AssemblyAI")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download and transcribe a video
    Transcribe {
        /// Video URL to transcribe
        url: String,
    },

    /// List available transcripts
    List {
        /// Filter by platform (youtube, vimeo, etc.)
        #[arg(short, long)]
        platform: Option<String>,

        /// Filter by channel name
        #[arg(short, long)]
        channel: Option<String>,
    },

    /// Read a transcript
    Read {
        /// Path to transcript directory
        path: String,

        /// Output as JSON with timestamps
        #[arg(short, long)]
        json: bool,
    },

    /// Search transcripts using full-text search
    Search {
        /// Search query
        query: String,

        /// Maximum results (default: 20)
        #[arg(short = 'n', long, default_value = "20")]
        limit: i32,
    },

    /// Show database statistics
    Stats,

    /// Initialize with AssemblyAI API key
    Init {
        /// AssemblyAI API key
        #[arg(short = 'k', long)]
        api_key: Option<String>,

        /// Overwrite existing config
        #[arg(short, long)]
        force: bool,
    },

    /// Reindex all transcripts in the database
    Reindex,
}

#[tokio::main]
async fn main() {
    // Load environment variables
    load_env();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Transcribe { url } => commands::transcribe::run(&url).await,
        Commands::List { platform, channel } => {
            commands::list::run(platform.as_deref(), channel.as_deref())
        }
        Commands::Read { path, json } => commands::read::run(&path, json),
        Commands::Search { query, limit } => commands::search::run(&query, limit),
        Commands::Stats => commands::stats::run(),
        Commands::Init { api_key, force } => commands::init::run(api_key, force),
        Commands::Reindex => commands::reindex::run(),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
