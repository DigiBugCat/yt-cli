use crate::downloader::{fetch_channel_videos, PlaylistEntry};
use crate::error::Result;

pub fn run(channel: &str, limit: usize) -> Result<()> {
    eprintln!("Fetching videos from channel...");

    let videos = fetch_channel_videos(channel, limit)?;

    if videos.is_empty() {
        println!("No videos found for channel: {}", channel);
        return Ok(());
    }

    println!("Found {} video(s):\n", videos.len());

    for (i, video) in videos.iter().enumerate() {
        print_video_entry(i + 1, video);
    }

    println!("To transcribe a video, run:");
    println!("  yt-cli transcribe <url>");

    Ok(())
}

fn print_video_entry(index: usize, video: &PlaylistEntry) {
    // Title line with duration
    let duration_str = video
        .duration
        .map(|d| {
            let mins = d / 60;
            let secs = d % 60;
            format!(" ({}:{:02})", mins, secs)
        })
        .unwrap_or_default();

    println!("{}. {}{}", index, video.title, duration_str);

    // View count and upload date
    let mut meta_parts = Vec::new();
    if let Some(views) = video.view_count {
        meta_parts.push(format_view_count(views));
    }
    if let Some(date) = &video.upload_date {
        meta_parts.push(format_upload_date(date));
    }
    if !meta_parts.is_empty() {
        println!("   {}", meta_parts.join(" | "));
    }

    // URL for easy copying
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

fn format_upload_date(date: &str) -> String {
    // yt-dlp returns YYYYMMDD format
    if date.len() == 8 {
        format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8])
    } else {
        date.to_string()
    }
}
