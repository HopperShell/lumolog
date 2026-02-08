mod app;
mod filter;
mod highlighter;
mod parser;
mod source;
mod ui;

use app::App;
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use source::FileSource;
use std::io::IsTerminal;
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
            if std::io::stdin().is_terminal() {
                eprintln!("Usage: lumolog <file> or pipe input via stdin");
                eprintln!("Example: cat app.log | lumolog");
                std::process::exit(1);
            }
            source::StdinSource::read_all()?.lines().to_vec()
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
                    _ if app.is_filter_mode() => match key.code {
                        KeyCode::Esc => {
                            if app.filter_pattern().is_empty() {
                                app.exit_filter_mode();
                            } else {
                                app.clear_filter();
                            }
                        }
                        KeyCode::Enter => app.exit_filter_mode(),
                        KeyCode::Backspace => app.filter_backspace(),
                        KeyCode::Char(c) => app.filter_input(c),
                        _ => {}
                    },
                    KeyCode::Char('q') | KeyCode::Esc => app.quit(),
                    KeyCode::Down | KeyCode::Char('j') => app.scroll_down(1),
                    KeyCode::Up | KeyCode::Char('k') => app.scroll_up(1),
                    KeyCode::PageDown | KeyCode::Char(' ') => app.page_down(),
                    KeyCode::PageUp => app.page_up(),
                    KeyCode::Char('g') => app.scroll_to_top(),
                    KeyCode::Char('G') => app.scroll_to_bottom(),
                    KeyCode::Char('/') => app.enter_filter_mode(),
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
