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
        Command {
            name: "Quit",
            keybinding: Some("q / Esc"),
            action: Quit,
        },
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
            name: "Cursor mode",
            keybinding: Some("Enter"),
            action: EnterCursorMode,
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
    ];
    COMMANDS
}
