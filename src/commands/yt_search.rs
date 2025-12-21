use crate::downloader::{search_youtube, PlaylistEntry};
use crate::error::Result;

pub fn run(query: &str, limit: usize) -> Result<()> {
    eprintln!("Searching YouTube for: {}", query);

    let results = search_youtube(query, limit)?;

    if results.is_empty() {
        println!("No results found for: {}", query);
        return Ok(());
    }

    println!("Found {} result(s) for '{}':\n", results.len(), query);

    for (i, video) in results.iter().enumerate() {
        print_search_result(i + 1, video);
    }

    println!("To transcribe a video, run:");
    println!("  yt-cli transcribe <url>");

    Ok(())
}

fn print_search_result(index: usize, video: &PlaylistEntry) {
    // Title with channel
    let channel_str = video
        .channel
        .as_ref()
        .map(|c| format!(" - {}", c))
        .unwrap_or_default();

    let duration_str = video
        .duration
        .map(|d| {
            let mins = d / 60;
            let secs = d % 60;
            format!(" ({}:{:02})", mins, secs)
        })
        .unwrap_or_default();

    println!("{}. {}{}{}", index, video.title, channel_str, duration_str);

    // View count
    if let Some(views) = video.view_count {
        println!("   {}", format_view_count(views));
    }

    // URL
    println!("   {}", video.url);
    println!();
}

fn format_view_count(views: i64) -> String {
    if views >= 1_000_000 {
        format!("{:.1}M views", views as f64 / 1_000_000.0)
    } else if views >= 1_000 {
        format!("{:.1}K views", views as f64 / 1_000.0)
    } else {
        format!("{} views", views)
    }
}
