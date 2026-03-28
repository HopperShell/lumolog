# AI Analyze Mode

## Summary

Add an analyze mode where users send their currently filtered log lines to the AI with a free-form question. The AI reads the actual log content and responds with analysis — identifying issues, patterns, correlations, root causes, etc. The response appears in a scrollable overlay panel.

## User Experience

### Entry

- Press `A` (shift-a) in Normal mode (only available when AI is connected)
- An input bar appears at the bottom, styled distinctly from the filter and ask bars: `analyze: ` in cyan+bold
- Type a question (e.g., "what's causing these errors?", "any patterns here?", "summarize what happened")
- Press Enter to submit, Esc to cancel

### Processing

- The input bar disappears, status bar shows "AI analyzing..." in yellow
- All currently filtered log lines (raw text) are sent to the AI along with the user's question
- The request runs in a background thread — the UI remains responsive

### Response Display

- When the response arrives, an overlay panel appears centered over the main log view
- The panel has a bordered box with title "AI Analysis"
- Content is word-wrapped to fit the panel width
- If the response is longer than the panel height, it is scrollable with j/k or arrow keys
- Press Esc to dismiss the overlay and return to Normal mode

### Error Handling

- If the AI request fails or returns unparseable content, show the error in the status bar briefly and remain in Normal mode
- No overlay appears on error

## Architecture

### Changes to Existing Modules

**`src/ai.rs`:**
- Add `build_analyze_prompt(user_question: &str, log_lines: &[String]) -> (String, String)` — returns (system_prompt, user_message). The system prompt instructs the AI to analyze log data. The user message contains the log lines followed by the user's question.

**`src/app.rs`:**
- Add `Analyze` variant to `AppMode`
- Add fields: `analyze_input: String`, `analyze_response: Option<String>`, `analyze_scroll: usize`
- Add methods: `enter_analyze_mode()`, `exit_analyze_mode()`, `analyze_type(c)`, `analyze_backspace()`, `set_analyze_response(text)`, `clear_analyze_response()`, `analyze_scroll_down()`, `analyze_scroll_up()`, `analyze_input()`, `analyze_response()`
- `enter_analyze_mode()` guards on `ai_connected` (same as ask mode)

**`src/command.rs`:**
- Add `Action::EnterAnalyzeMode`
- Add command entry: name "AI analyze", keybinding "A", action `EnterAnalyzeMode`

**`src/main.rs`:**
- Add `Analyze` mode key handling in event loop:
  - Text input (same pattern as Ask/Filter modes)
  - On Enter: collect `app.all_filtered_lines_raw()`, build analyze prompt, spawn background thread
  - On Esc: exit analyze mode
- Add analyze overlay key handling (when `analyze_response` is Some):
  - j/Down: scroll down
  - k/Up: scroll up
  - Esc: clear response and return to Normal
- Add analyze response polling (same channel pattern as ask mode)
- Add `EnterAnalyzeMode` to `dispatch_action`
- Add `A` keybinding in Normal mode (guarded on `is_ai_connected`)

**`src/ui.rs`:**
- Analyze input bar: render in filter_area when mode is Analyze, styled `analyze: ` in cyan+bold
- Update `filter_height` to include Analyze mode
- Analyze overlay panel: render when `analyze_response` is Some
  - Centered box, ~80% of screen width, ~70% of screen height
  - Title: "AI Analysis"
  - Word-wrapped text content
  - Scroll offset support
  - Rendered over (on top of) the main log view using `Clear` widget + bordered `Paragraph`

### Prompt Design

System prompt:
```
You are a log analysis expert. The user will show you log lines and ask a question about them.
Analyze the logs carefully and provide a clear, concise response. Focus on:
- Patterns and trends you observe
- Errors, anomalies, or concerning behavior
- Correlations between events
- Potential root causes if errors are present
- A brief summary of what the logs show

Be specific — reference actual log content, timestamps, and error messages in your analysis.
Keep your response concise and actionable.
```

User message:
```
Here are {N} log lines:

{all filtered log lines, one per line}

Question: {user's question}
```

### Reuse

- Same `ai::query_ai()` function and background thread pattern as ask mode
- Same `mpsc` channel for receiving results
- Differentiate ask vs analyze responses via an enum wrapper in the channel:
  ```rust
  enum AiResponse {
      Filter(Result<String, String>),
      Analyze(Result<String, String>),
  }
  ```

## What This Is Not

- Not a chat. Each analyze request is independent — no conversation history.
- Not streaming. The full response appears at once when complete.
- The AI sees raw log lines as text. It does not get structured/parsed data.

## Testing

- Unit tests for `build_analyze_prompt`: verify it includes log lines and question
- Unit tests for overlay scroll math (content height vs viewport)
- Existing AI tests continue to pass unchanged
