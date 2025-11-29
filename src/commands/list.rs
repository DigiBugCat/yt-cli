use crate::error::Result;
use crate::storage::list_transcripts;

pub fn run(platform: Option<&str>, channel: Option<&str>) -> Result<()> {
    let transcripts = list_transcripts(platform, channel)?;

    if transcripts.is_empty() {
        println!("No transcripts found.");
        return Ok(());
    }

    println!("Found {} transcript(s):\n", transcripts.len());

    for t in transcripts {
        let mut line = format!("- {}/{}/{}", t.platform, t.channel, t.title);
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
