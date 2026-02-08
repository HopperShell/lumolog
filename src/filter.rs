use crate::parser::ParsedLine;

/// Returns indices of lines matching the pattern (case-insensitive substring match).
pub fn filter_lines(lines: &[ParsedLine], pattern: &str) -> Vec<usize> {
    if pattern.is_empty() {
        return (0..lines.len()).collect();
    }

    let pattern_lower = pattern.to_lowercase();
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.raw.to_lowercase().contains(&pattern_lower))
        .map(|(i, _)| i)
        .collect()
}
