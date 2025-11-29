use crate::error::Result;
use crate::storage::get_transcript;

pub fn run(path: &str, json: bool) -> Result<()> {
    let data = get_transcript(path)?;

    if json {
        if let Some(structured) = data.structured {
            println!("{}", serde_json::to_string_pretty(&structured)?);
        } else {
            eprintln!("No structured data available.");
        }
    } else if let Some(text) = data.text {
        println!("{}", text);
    } else {
        eprintln!("No text content found.");
    }

    Ok(())
}
