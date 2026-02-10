use crate::parser::{LogLevel, ParsedLine};
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

/// Result of filtering log lines — carries the matching indices and whether
/// fuzzy matching was used (so the UI can indicate it).
pub struct FilterResult {
    pub indices: Vec<usize>,
    pub is_fuzzy: bool,
}

/// Returns indices of lines matching the pattern (case-insensitive substring match)
/// and at or above the minimum log level. Falls back to fuzzy matching when exact
/// substring match returns zero results.
pub fn filter_lines(
    lines: &[ParsedLine],
    pattern: &str,
    min_level: Option<LogLevel>,
) -> FilterResult {
    // Level-filtered candidates (indices into `lines`)
    let level_ok: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            if let Some(min) = min_level {
                if let Some(level) = line.level {
                    return level >= min;
                }
            }
            true
        })
        .map(|(i, _)| i)
        .collect();

    if pattern.is_empty() {
        return FilterResult {
            indices: level_ok,
            is_fuzzy: false,
        };
    }

    // Exact substring match (case-insensitive)
    let pattern_lower = pattern.to_lowercase();
    let exact: Vec<usize> = level_ok
        .iter()
        .copied()
        .filter(|&i| lines[i].raw.to_lowercase().contains(&pattern_lower))
        .collect();

    if !exact.is_empty() {
        return FilterResult {
            indices: exact,
            is_fuzzy: false,
        };
    }

    // Fuzzy fallback — only when exact match found nothing
    let mut matcher = Matcher::new(Config::DEFAULT);
    let pat = Pattern::new(
        pattern,
        CaseMatching::Ignore,
        Normalization::Smart,
        AtomKind::Fuzzy,
    );
    let mut buf = Vec::new();

    let fuzzy: Vec<usize> = level_ok
        .iter()
        .copied()
        .filter(|&i| {
            buf.clear();
            let haystack = Utf32Str::new(&lines[i].raw, &mut buf);
            pat.score(haystack, &mut matcher).is_some()
        })
        .collect();

    let is_fuzzy = !fuzzy.is_empty();
    FilterResult {
        indices: fuzzy,
        is_fuzzy,
    }
}
