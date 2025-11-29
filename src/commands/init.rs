use std::io::{self, Write};

use crate::config::{data_dir, ensure_directories, env_file_path};
use crate::error::Result;

pub fn run(api_key: Option<String>, force: bool) -> Result<()> {
    ensure_directories()?;

    let env_file = env_file_path();

    if env_file.exists() && !force {
        println!("Config already exists at {}", env_file.display());
        println!("Use --force to overwrite.");
        return Ok(());
    }

    let api_key = if let Some(key) = api_key {
        key
    } else {
        print!("Enter your AssemblyAI API key: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };

    if api_key.is_empty() {
        eprintln!("Error: API key is required.");
        std::process::exit(1);
    }

    std::fs::write(&env_file, format!("ASSEMBLYAI_API_KEY={}\n", api_key))?;

    println!("Config saved to {}", env_file.display());
    println!("Data directory: {}", data_dir().display());

    Ok(())
}
