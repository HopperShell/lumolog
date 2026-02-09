mod app;
mod filter;
mod highlighter;
mod parser;
mod source;
mod ui;

use app::App;
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use source::{FileSource, FollowableSource};
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "lumolog",
    version,
    about = "A terminal log viewer that makes logs readable"
)]
struct Cli {
    /// Log file to view. Omit to read from stdin.
    file: Option<PathBuf>,

    /// Follow the file for new lines (like tail -f). Requires a file argument.
    #[arg(short, long)]
    follow: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.follow && cli.file.is_none() {
        eprintln!("Error: --follow requires a file argument");
        std::process::exit(1);
    }

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

    // Set up follow source if --follow was requested
    let mut follow_source = if cli.follow {
        let path = cli.file.as_ref().unwrap();
        let initial_offset = std::fs::metadata(path)?.len();
        Some(FollowableSource::new(path, initial_offset))
    } else {
        None
    };

    let mut terminal = ratatui::init();

    // Ensure terminal is restored even on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        ratatui::restore();
        original_hook(panic_info);
    }));

    let mut app = App::new(lines);
    app.scroll_to_bottom();
    app.set_follow_mode(cli.follow);

    if let Some(ref path) = cli.file {
        app.set_source_name(
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string()),
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
                        KeyCode::Char(' ') if app.is_follow_mode() => app.toggle_follow_pause(),
                        KeyCode::PageDown | KeyCode::Char(' ') => app.page_down(),
                        KeyCode::PageUp => app.page_up(),
                        KeyCode::Char('g') => app.scroll_to_top(),
                        KeyCode::Char('G') => app.scroll_to_bottom(),
                        KeyCode::Char('/') => app.enter_filter_mode(),
                        KeyCode::Char('p') => app.toggle_pretty(),
                        KeyCode::Char('v') => app.cycle_level_up(),
                        KeyCode::Char('V') => app.cycle_level_down(),
                        KeyCode::Char('?') => app.toggle_help(),
                        _ => {}
                    }
                }
            }
        }

        // Poll for new lines in follow mode (unless paused)
        if !app.is_follow_paused() {
            if let Some(ref mut source) = follow_source {
                let new_lines = source.read_new_lines()?;
                if !new_lines.is_empty() {
                    app.append_lines(new_lines);
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
