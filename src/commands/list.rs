use crate::error::Result;
use crate::storage::list_transcripts;

pub fn run(platform: Option<&str>, channel: Option<&str>, handle: Option<&str>) -> Result<()> {
    let transcripts = list_transcripts(platform, channel, handle)?;

    if transcripts.is_empty() {
        println!("No transcripts found.");
        return Ok(());
    }

    println!("Found {} transcript(s):\n", transcripts.len());

    for t in transcripts {
        // Show channel name with handle if different
        let channel_display = if let Some(ref handle) = t.channel_handle {
            if handle != &t.channel && !handle.is_empty() {
                format!("{} ({})", t.channel, handle)
            } else {
                t.channel.clone()
            }
        } else {
            t.channel.clone()
        };

        let mut line = format!("- {}/{}/{}", t.platform, channel_display, t.title);
        if let Some(duration) = t.duration {
            let mins = duration / 60;
            let secs = duration % 60;
            line.push_str(&format!(" ({}m {}s)", mins, secs));
        }
        println!("{}", line);
        println!("  Path: {}", t.path);
    }

    Ok(())
}
