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

    // Ensure terminal is restored even on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        ratatui::restore();
        original_hook(panic_info);
    }));

    let mut app = App::new(lines);

    if let Some(ref path) = cli.file {
        app.set_source_name(
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string())
        );
    }

    loop {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // When help is shown, any key dismisses it (except q which still quits)
                if app.show_help() {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.quit(),
                        _ => app.toggle_help(),
                    }
                } else {
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
                        KeyCode::Char('p') => app.toggle_pretty(),
                        KeyCode::Char('?') => app.toggle_help(),
                        _ => {}
                    }
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
