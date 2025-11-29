use crate::database::search_transcripts;
use crate::error::Result;

pub fn run(query: &str, limit: i32) -> Result<()> {
    let results = search_transcripts(query, limit)?;

    if results.is_empty() {
        println!("No results found for: {}", query);
        return Ok(());
    }

    println!("Found {} result(s) for '{}':\n", results.len(), query);

    for r in results {
        let duration = r.duration.unwrap_or(0);
        let mins = duration / 60;
        let secs = duration % 60;

        println!("- {}: {} ({}m {}s)", r.channel, r.title, mins, secs);
        println!("  Path: {}", r.path);
        if let Some(snippet) = r.snippet {
            println!("  Match: {}", snippet);
        }
        println!();
    }

    Ok(())
}
