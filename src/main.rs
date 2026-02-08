use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "lumolog", version, about = "A terminal log viewer that makes logs readable")]
struct Cli {
    /// Log file to view. Omit to read from stdin.
    file: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.file {
        Some(path) => {
            if !path.exists() {
                eprintln!("Error: file not found: {}", path.display());
                std::process::exit(1);
            }
            println!("Would open: {}", path.display());
        }
        None => {
            println!("Would read from stdin");
        }
    }

    Ok(())
}
