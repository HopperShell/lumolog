mod app;
mod command;
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

fn dispatch_action(action: command::Action, app: &mut App) {
    use command::Action::*;
    match action {
        Quit => app.quit(),
        ScrollDown => app.scroll_down(1),
        ScrollUp => app.scroll_up(1),
        ScrollLeft => app.scroll_left(1),
        ScrollRight => app.scroll_right(1),
        PageDown => app.page_down(),
        PageUp => app.page_up(),
        ScrollToTop => app.scroll_to_top(),
        ScrollToBottom => app.scroll_to_bottom(),
        OpenFilter => app.enter_filter_mode(),
        CycleLevelUp => app.cycle_level_up(),
        CycleLevelDown => app.cycle_level_down(),
        TogglePretty => app.toggle_pretty(),
        ToggleWrap => app.toggle_wrap(),
        EnterCursorMode => app.enter_cursor_mode(),
        ToggleFollowPause => app.toggle_follow_pause(),
        OpenCommandPalette => app.open_palette(),
        YankLine => {
            if let Some(text) = app.cursor_line_raw() {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(text.to_string());
                    app.set_yank_flash();
                }
            }
        }
        YankAllFiltered => {
            let text = app.all_filtered_lines_raw();
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(text);
                app.set_yank_flash();
            }
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
                    if app.mode() == AppMode::CommandPalette {
                        match key.code {
                            KeyCode::Esc => app.close_palette(),
                            KeyCode::Up => app.palette_up(),
                            KeyCode::Down => app.palette_down(),
                            KeyCode::Enter => {
                                if let Some(action) = app.palette_execute() {
                                    dispatch_action(action, &mut app);
                                }
                            }
                            KeyCode::Backspace => app.palette_backspace(),
                            KeyCode::Char(c) => app.palette_type(c),
                            _ => {}
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
                    } else if app.is_cursor_mode() {
                        match key.code {
                            KeyCode::Down | KeyCode::Char('j') => app.cursor_down(1),
                            KeyCode::Up | KeyCode::Char('k') => app.cursor_up(1),
                            KeyCode::Char('y') => {
                                dispatch_action(command::Action::YankLine, &mut app)
                            }
                            KeyCode::Char('Y') => {
                                dispatch_action(command::Action::YankAllFiltered, &mut app)
                            }
                            KeyCode::Char('s') => app.filter_by_similar(),
                            KeyCode::Right | KeyCode::Char('l') => app.scroll_right(1),
                            KeyCode::Left | KeyCode::Char('h') => app.scroll_left(1),
                            KeyCode::Esc => app.exit_cursor_mode(),
                            KeyCode::Char('q') => app.quit(),
                            KeyCode::Char('?') => app.open_palette(),
                            _ => {}
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
                            KeyCode::Char('q') => app.quit(),
                            KeyCode::Esc => {
                                if app.is_similar_filter() {
                                    app.clear_similar();
                                } else {
                                    app.quit();
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => app.scroll_down(1),
                            KeyCode::Up | KeyCode::Char('k') => app.scroll_up(1),
                            KeyCode::Right | KeyCode::Char('l') => app.scroll_right(1),
                            KeyCode::Left | KeyCode::Char('h') => app.scroll_left(1),
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
                            KeyCode::Char('?') => app.open_palette(),
                            KeyCode::Enter => app.enter_cursor_mode(),
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
                                    app.open_context_menu(value, kind, (mouse.column, mouse.row));
                                }
                            }
                        } else if app.mode() != AppMode::Cursor {
                            if let Some((kind, value)) =
                                ui::token_at_position(&app, mouse.column, mouse.row, terminal_area)
                            {
                                app.open_context_menu(value, kind, (mouse.column, mouse.row));
                            }
                        }
                    }
                    MouseEventKind::ScrollDown => app.scroll_down(3),
                    MouseEventKind::ScrollUp => app.scroll_up(3),
                    MouseEventKind::ScrollLeft => app.scroll_left(3),
                    MouseEventKind::ScrollRight => app.scroll_right(3),
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
