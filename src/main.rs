mod app;
mod source;
mod ui;

use app::App;
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use source::FileSource;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "lumolog", version, about = "A terminal log viewer that makes logs readable")]
struct Cli {
    /// Log file to view. Omit to read from stdin.
    file: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let lines = match &cli.file {
        Some(path) => {
            if !path.exists() {
                eprintln!("Error: file not found: {}", path.display());
                std::process::exit(1);
            }
            let source = FileSource::open(path)?;
            source.lines().to_vec()
        }
        None => {
            eprintln!("stdin support coming soon. Please provide a file.");
            std::process::exit(1);
        }
    };

    let mut terminal = ratatui::init();
    let mut app = App::new(lines);

    loop {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.quit(),
                    KeyCode::Down | KeyCode::Char('j') => app.scroll_down(1),
                    KeyCode::Up | KeyCode::Char('k') => app.scroll_up(1),
                    KeyCode::PageDown | KeyCode::Char(' ') => app.page_down(),
                    KeyCode::PageUp => app.page_up(),
                    KeyCode::Char('g') => app.scroll_to_top(),
                    KeyCode::Char('G') => app.scroll_to_bottom(),
                    _ => {}
                }
            }
        }

        if app.should_quit() {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}
