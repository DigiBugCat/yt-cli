use crate::config::data_dir;
use crate::database::get_stats;
use crate::error::Result;

pub fn run() -> Result<()> {
    let stats = get_stats()?;

    if stats.total_transcripts == 0 {
        println!("No transcripts in database yet.");
        println!("\nData directory: {}", data_dir().display());
        return Ok(());
    }

    let total_duration = stats.total_duration.unwrap_or(0);
    let hours = total_duration / 3600;
    let mins = (total_duration % 3600) / 60;

    println!("Transcript Database Statistics");
    println!("==============================");
    println!("Total transcripts: {}", stats.total_transcripts);
    println!("Unique channels:   {}", stats.unique_channels);
    println!("Unique platforms:  {}", stats.unique_platforms);
    println!("Total duration:    {}h {}m", hours, mins);
    println!("Total words:       {}", stats.total_words.unwrap_or(0));
    println!("\nData directory: {}", data_dir().display());

    Ok(())
}
