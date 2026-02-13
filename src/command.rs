/// Every executable action in the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    ScrollDown,
    ScrollUp,
    ScrollLeft,
    ScrollRight,
    PageDown,
    PageUp,
    ScrollToTop,
    ScrollToBottom,
    OpenFilter,
    CycleLevelUp,
    CycleLevelDown,
    TogglePretty,
    ToggleWrap,
    EnterCursorMode,
    ToggleFollowPause,
    OpenCommandPalette,
    YankLine,
    YankAllFiltered,
    EnterTimeMode,
    ClearTimeRange,
    ToggleSparkline,
    TimeMarkStart,
    TimeMarkEndApply,
    TimePresetLast5m,
    TimePresetLast15m,
    TimePresetLast1h,
    TimePresetLast24h,
}

/// A command in the palette. `keybinding` is a display string for the help column.
pub struct Command {
    pub name: &'static str,
    pub keybinding: Option<&'static str>,
    pub action: Action,
}

/// The single source of truth for all commands. Both the command palette
/// and key dispatch derive from this list.
pub fn commands() -> &'static [Command] {
    use Action::*;
    static COMMANDS: &[Command] = &[
        // --- Actions (most useful at the top) ---
        Command {
            name: "Filter / search",
            keybinding: Some("/"),
            action: OpenFilter,
        },
        Command {
            name: "Level filter up",
            keybinding: Some("v"),
            action: CycleLevelUp,
        },
        Command {
            name: "Level filter down",
            keybinding: Some("V"),
            action: CycleLevelDown,
        },
        Command {
            name: "Cursor mode",
            keybinding: Some("Enter"),
            action: EnterCursorMode,
        },
        Command {
            name: "Yank line to clipboard",
            keybinding: Some("y (cursor)"),
            action: YankLine,
        },
        Command {
            name: "Yank all filtered lines",
            keybinding: Some("Y (cursor)"),
            action: YankAllFiltered,
        },
        Command {
            name: "Pretty-print JSON",
            keybinding: Some("p"),
            action: TogglePretty,
        },
        Command {
            name: "Toggle line wrap",
            keybinding: Some("w"),
            action: ToggleWrap,
        },
        Command {
            name: "Time range mode",
            keybinding: Some("t"),
            action: EnterTimeMode,
        },
        Command {
            name: "Time: mark start",
            keybinding: Some("[ (time)"),
            action: TimeMarkStart,
        },
        Command {
            name: "Time: mark end & apply",
            keybinding: Some("] (time)"),
            action: TimeMarkEndApply,
        },
        Command {
            name: "Time: last 5 minutes",
            keybinding: Some("1 (time)"),
            action: TimePresetLast5m,
        },
        Command {
            name: "Time: last 15 minutes",
            keybinding: Some("2 (time)"),
            action: TimePresetLast15m,
        },
        Command {
            name: "Time: last 1 hour",
            keybinding: Some("3 (time)"),
            action: TimePresetLast1h,
        },
        Command {
            name: "Time: last 24 hours",
            keybinding: Some("4 (time)"),
            action: TimePresetLast24h,
        },
        Command {
            name: "Clear time range",
            keybinding: Some("c (time)"),
            action: ClearTimeRange,
        },
        Command {
            name: "Toggle sparkline",
            keybinding: None,
            action: ToggleSparkline,
        },
        Command {
            name: "Pause / resume follow",
            keybinding: Some("Space"),
            action: ToggleFollowPause,
        },
        Command {
            name: "Command palette",
            keybinding: Some("?"),
            action: OpenCommandPalette,
        },
        Command {
            name: "Quit",
            keybinding: Some("q"),
            action: Quit,
        },
        // --- Navigation (less important, bottom) ---
        Command {
            name: "Scroll down",
            keybinding: Some("j / Down"),
            action: ScrollDown,
        },
        Command {
            name: "Scroll up",
            keybinding: Some("k / Up"),
            action: ScrollUp,
        },
        Command {
            name: "Scroll left",
            keybinding: Some("h / Left"),
            action: ScrollLeft,
        },
        Command {
            name: "Scroll right",
            keybinding: Some("l / Right"),
            action: ScrollRight,
        },
        Command {
            name: "Page down",
            keybinding: Some("Space / PgDn"),
            action: PageDown,
        },
        Command {
            name: "Page up",
            keybinding: Some("PgUp"),
            action: PageUp,
        },
        Command {
            name: "Go to top",
            keybinding: Some("g"),
            action: ScrollToTop,
        },
        Command {
            name: "Go to bottom",
            keybinding: Some("G"),
            action: ScrollToBottom,
        },
    ];
    COMMANDS
}
