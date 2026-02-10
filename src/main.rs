mod app;
mod filter;
mod highlighter;
mod parser;
mod source;
mod ui;

use app::{App, AppMode, MenuAction};
use clap::Parser;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton,
    MouseEventKind,
};
use crossterm::execute;
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

fn execute_action(action: MenuAction, value: String, app: &mut App) {
    match action {
        MenuAction::FilterByValue => {
            app.set_filter(value);
        }
        MenuAction::LookupAbuseIPDB => {
            let url = format!("https://www.abuseipdb.com/check/{}", value);
            let _ = open::that(url);
        }
        MenuAction::OpenInBrowser => {
            let _ = open::that(&value);
        }
    }
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
    execute!(std::io::stdout(), EnableMouseCapture)?;

    // Ensure terminal is restored even on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = execute!(std::io::stdout(), DisableMouseCapture);
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

        let terminal_area: ratatui::layout::Rect = terminal.size()?.into();

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    // When help is shown, any key dismisses it (except q which still quits)
                    if app.show_help() {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => app.quit(),
                            _ => app.toggle_help(),
                        }
                    } else if app.mode() == AppMode::ContextMenu {
                        match key.code {
                            KeyCode::Up | KeyCode::Char('k') => app.menu_up(),
                            KeyCode::Down | KeyCode::Char('j') => app.menu_down(),
                            KeyCode::Enter => {
                                if let Some((action, value)) = app.execute_menu_action() {
                                    execute_action(action, value, &mut app);
                                }
                            }
                            KeyCode::Esc => app.close_context_menu(),
                            _ => app.close_context_menu(),
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
                            KeyCode::Char('w') => app.toggle_wrap(),
                            KeyCode::Char('v') => app.cycle_level_up(),
                            KeyCode::Char('V') => app.cycle_level_down(),
                            KeyCode::Char('?') => app.toggle_help(),
                            _ => {}
                        }
                    }
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        if app.mode() == AppMode::ContextMenu {
                            if let Some(index) = ui::menu_item_at_position(
                                &app,
                                mouse.column,
                                mouse.row,
                                terminal_area,
                            ) {
                                // Clicked a menu item — execute it
                                if let Some((action, value)) = app.execute_menu_item(index) {
                                    execute_action(action, value, &mut app);
                                }
                            } else {
                                // Clicked outside menu — close and check for new token
                                app.close_context_menu();
                                if let Some((kind, value)) = ui::token_at_position(
                                    &app,
                                    mouse.column,
                                    mouse.row,
                                    terminal_area,
                                ) {
                                    app.open_context_menu(
                                        value,
                                        kind,
                                        (mouse.column, mouse.row),
                                    );
                                }
                            }
                        } else if app.mode() == AppMode::Normal {
                            if let Some((kind, value)) =
                                ui::token_at_position(&app, mouse.column, mouse.row, terminal_area)
                            {
                                app.open_context_menu(value, kind, (mouse.column, mouse.row));
                            }
                        }
                    }
                    MouseEventKind::ScrollDown => app.scroll_down(3),
                    MouseEventKind::ScrollUp => app.scroll_up(3),
                    _ => {}
                },
                _ => {}
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

    execute!(std::io::stdout(), DisableMouseCapture)?;
    ratatui::restore();
    Ok(())
}
