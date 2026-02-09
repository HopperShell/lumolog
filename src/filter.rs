use crate::parser::{LogLevel, ParsedLine};

/// Returns indices of lines matching the pattern (case-insensitive substring match)
/// and at or above the minimum log level.
pub fn filter_lines(
    lines: &[ParsedLine],
    pattern: &str,
    min_level: Option<LogLevel>,
) -> Vec<usize> {
    let pattern_lower = if pattern.is_empty() {
        None
    } else {
        Some(pattern.to_lowercase())
    };

    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            // Text filter
            if let Some(ref pat) = pattern_lower {
                if !line.raw.to_lowercase().contains(pat.as_str()) {
                    return false;
                }
            }

            // Level filter: lines with no parsed level are always shown
            if let Some(min) = min_level {
                if let Some(level) = line.level {
                    if level < min {
                        return false;
                    }
                }
            }

            true
        })
        .map(|(i, _)| i)
        .collect()
}
