use chrono::NaiveDateTime;
use std::cell::Cell;

use crate::parser::ParsedLine;

#[derive(Debug, Clone)]
pub struct TimeIndex {
    /// Per-line resolved timestamp (forward-filled from nearest preceding timestamped line)
    timestamps: Vec<Option<NaiveDateTime>>,
    pub min_ts: Option<NaiveDateTime>,
    pub max_ts: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct SparklineData {
    pub buckets: Vec<u64>,
    pub bucket_starts: Vec<NaiveDateTime>,
    pub bucket_duration_secs: i64,
    pub num_buckets: usize,
}

#[derive(Debug, Clone)]
pub struct TimeRange {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct TimeModeState {
    pub cursor_bucket: usize,
    pub range_start: Option<usize>,
    pub dragging: bool,
    pub drag_start: Option<usize>,
}

// "Winner sticks" cache: after first successful parse, try that format first.
thread_local! {
    static LAST_FORMAT_IDX: Cell<Option<usize>> = const { Cell::new(None) };
}

const FORMAT_STRINGS: &[&str] = &[
    "%Y-%m-%dT%H:%M:%S%.f%:z", // RFC 3339 with offset (colon)
    "%Y-%m-%dT%H:%M:%S%.f%#z", // RFC 3339 with offset (no colon)
    "%Y-%m-%dT%H:%M:%S%:z",    // RFC 3339 no frac, with offset
    "%Y-%m-%dT%H:%M:%S%#z",    // RFC 3339 no frac, offset no colon
    "%Y-%m-%dT%H:%M:%SZ",      // RFC 3339 Zulu
    "%Y-%m-%dT%H:%M:%S%.fZ",   // RFC 3339 Zulu with frac
    "%Y-%m-%dT%H:%M:%S%.f",    // ISO without offset
    "%Y-%m-%dT%H:%M:%S",       // ISO basic
    "%Y-%m-%d %H:%M:%S%.f",    // Space-separated with frac
    "%Y-%m-%d %H:%M:%S,%f",    // Python comma frac
    "%Y-%m-%d %H:%M:%S",       // Basic datetime
    "%d/%b/%Y:%H:%M:%S %z",    // Apache CLF
];

/// Syslog months for manual parsing
const SYSLOG_MONTHS: &[&str] = &[
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

pub fn parse_timestamp(raw: &str) -> Option<NaiveDateTime> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    // Try last successful format first ("winner sticks")
    let last_idx = LAST_FORMAT_IDX.get();
    if let Some(idx) = last_idx {
        if let Some(dt) = try_parse_with_format(raw, idx) {
            return Some(dt);
        }
    }

    // Try all format strings
    for (i, _fmt) in FORMAT_STRINGS.iter().enumerate() {
        if Some(i) == last_idx {
            continue; // Already tried
        }
        if let Some(dt) = try_parse_with_format(raw, i) {
            LAST_FORMAT_IDX.set(Some(i));
            return Some(dt);
        }
    }

    // Try epoch millis (1e12..1e14 range)
    if let Ok(n) = raw.parse::<i64>() {
        if (1_000_000_000_000..100_000_000_000_000).contains(&n) {
            let secs = n / 1000;
            let nsecs = ((n % 1000) * 1_000_000) as u32;
            if let Some(dt) = chrono::DateTime::from_timestamp(secs, nsecs) {
                return Some(dt.naive_utc());
            }
        }
        // Epoch secs (1e9..1e10 range)
        if (1_000_000_000..10_000_000_000).contains(&n) {
            if let Some(dt) = chrono::DateTime::from_timestamp(n, 0) {
                return Some(dt.naive_utc());
            }
        }
    }

    // Try epoch as float (e.g. "1705312200.123")
    if let Ok(f) = raw.parse::<f64>() {
        let n = f as i64;
        if (1_000_000_000..10_000_000_000).contains(&n) {
            let frac = ((f - n as f64) * 1_000_000_000.0) as u32;
            if let Some(dt) = chrono::DateTime::from_timestamp(n, frac) {
                return Some(dt.naive_utc());
            }
        }
    }

    // Syslog: "Jan 15 08:30:00" (no year → current year)
    if let Some(dt) = try_parse_syslog(raw) {
        return Some(dt);
    }

    // Klog: "0115 08:30:00.000000" (MMDD, no year → current year)
    if let Some(dt) = try_parse_klog(raw) {
        return Some(dt);
    }

    None
}

fn try_parse_with_format(raw: &str, idx: usize) -> Option<NaiveDateTime> {
    let fmt = FORMAT_STRINGS[idx];
    // Some formats include %z/%:z which produce DateTime<FixedOffset>
    if fmt.contains("%z") || fmt.contains("%:z") || fmt.contains("%#z") {
        if let Ok(dt) = chrono::DateTime::parse_from_str(raw, fmt) {
            return Some(dt.naive_utc());
        }
        // Try prefix match: timestamp may be followed by other content
        // For fixed-offset formats, try increasingly longer prefixes
        for end in (20..=raw.len().min(40)).rev() {
            if end > raw.len() {
                continue;
            }
            if let Ok(dt) = chrono::DateTime::parse_from_str(&raw[..end], fmt) {
                return Some(dt.naive_utc());
            }
        }
        return None;
    }
    // NaiveDateTime formats
    if let Ok(dt) = NaiveDateTime::parse_from_str(raw, fmt) {
        return Some(dt);
    }
    // Try prefix match for naive formats
    for end in (19..=raw.len().min(35)).rev() {
        if end > raw.len() {
            continue;
        }
        if let Ok(dt) = NaiveDateTime::parse_from_str(&raw[..end], fmt) {
            return Some(dt);
        }
    }
    None
}

fn try_parse_syslog(raw: &str) -> Option<NaiveDateTime> {
    // Format: "Jan 15 08:30:00" or "Jan  5 08:30:00"
    if raw.len() < 15 {
        return None;
    }
    let month_str = &raw[..3];
    let month = SYSLOG_MONTHS.iter().position(|&m| m == month_str)? as u32 + 1;

    let rest = raw[3..].trim_start();
    let space_pos = rest.find(' ')?;
    let day: u32 = rest[..space_pos].parse().ok()?;

    let time_str = rest[space_pos + 1..].trim_start();
    if time_str.len() < 8 {
        return None;
    }
    let hour: u32 = time_str[0..2].parse().ok()?;
    let min: u32 = time_str[3..5].parse().ok()?;
    let sec: u32 = time_str[6..8].parse().ok()?;

    use chrono::Datelike;
    let year = chrono::Local::now().year();
    let date = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
    let time = chrono::NaiveTime::from_hms_opt(hour, min, sec)?;
    Some(NaiveDateTime::new(date, time))
}

fn try_parse_klog(raw: &str) -> Option<NaiveDateTime> {
    // Format: "0115 08:30:00.000000" (MMDD HH:MM:SS.micros)
    if raw.len() < 15 {
        return None;
    }
    // Must start with 4 digits (MMDD)
    if !raw[..4].chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if raw.as_bytes().get(4) != Some(&b' ') {
        return None;
    }
    let month: u32 = raw[0..2].parse().ok()?;
    let day: u32 = raw[2..4].parse().ok()?;
    if month == 0 || month > 12 || day == 0 || day > 31 {
        return None;
    }

    let time_str = &raw[5..];
    if time_str.len() < 8 {
        return None;
    }
    let hour: u32 = time_str[0..2].parse().ok()?;
    let min: u32 = time_str[3..5].parse().ok()?;
    let sec: u32 = time_str[6..8].parse().ok()?;

    let micros = if time_str.len() > 9 && time_str.as_bytes()[8] == b'.' {
        let frac_str = &time_str[9..time_str.len().min(15)];
        let padded = format!("{:0<6}", frac_str);
        padded[..6].parse::<u32>().unwrap_or(0)
    } else {
        0
    };

    use chrono::Datelike;
    let year = chrono::Local::now().year();
    let date = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
    let time = chrono::NaiveTime::from_hms_micro_opt(hour, min, sec, micros)?;
    Some(NaiveDateTime::new(date, time))
}

pub fn build_time_index(lines: &[ParsedLine]) -> TimeIndex {
    let mut timestamps: Vec<Option<NaiveDateTime>> = Vec::with_capacity(lines.len());
    let mut min_ts: Option<NaiveDateTime> = None;
    let mut max_ts: Option<NaiveDateTime> = None;

    // Reset the format cache at the start of building an index
    LAST_FORMAT_IDX.set(None);

    for line in lines {
        let ts = line.timestamp.as_ref().and_then(|s| parse_timestamp(s));
        if let Some(t) = ts {
            match min_ts {
                None => min_ts = Some(t),
                Some(current_min) if t < current_min => min_ts = Some(t),
                _ => {}
            }
            match max_ts {
                None => max_ts = Some(t),
                Some(current_max) if t > current_max => max_ts = Some(t),
                _ => {}
            }
        }
        timestamps.push(ts);
    }

    // Forward-fill: lines without a timestamp inherit from the nearest preceding timestamped line
    let mut last_ts: Option<NaiveDateTime> = None;
    for ts in &mut timestamps {
        if ts.is_some() {
            last_ts = *ts;
        } else if last_ts.is_some() {
            *ts = last_ts;
        }
    }

    TimeIndex {
        timestamps,
        min_ts,
        max_ts,
    }
}

impl TimeIndex {
    pub fn has_timestamps(&self) -> bool {
        self.min_ts.is_some() && self.max_ts.is_some()
    }

    pub fn timestamp_at(&self, idx: usize) -> Option<NaiveDateTime> {
        self.timestamps.get(idx).copied().flatten()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.timestamps.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.timestamps.is_empty()
    }

    /// Extend the index with new lines (for follow mode).
    pub fn append(&mut self, lines: &[ParsedLine]) {
        let mut last_ts = self.timestamps.last().copied().flatten();
        for line in lines {
            let ts = line.timestamp.as_ref().and_then(|s| parse_timestamp(s));
            let resolved = ts.or(last_ts);
            if let Some(t) = ts {
                match self.min_ts {
                    None => self.min_ts = Some(t),
                    Some(current_min) if t < current_min => self.min_ts = Some(t),
                    _ => {}
                }
                match self.max_ts {
                    None => self.max_ts = Some(t),
                    Some(current_max) if t > current_max => self.max_ts = Some(t),
                    _ => {}
                }
            }
            if resolved.is_some() {
                last_ts = resolved;
            }
            self.timestamps.push(resolved);
        }
    }
}

pub fn compute_sparkline(index: &TimeIndex, num_buckets: usize) -> Option<SparklineData> {
    let min_ts = index.min_ts?;
    let max_ts = index.max_ts?;

    if num_buckets == 0 {
        return None;
    }

    let total_duration = (max_ts - min_ts).num_seconds().max(1);
    let bucket_duration_secs = (total_duration as f64 / num_buckets as f64).ceil() as i64;
    let bucket_duration_secs = bucket_duration_secs.max(1);

    let mut buckets = vec![0u64; num_buckets];
    let mut bucket_starts = Vec::with_capacity(num_buckets);

    for i in 0..num_buckets {
        let offset = chrono::Duration::seconds(bucket_duration_secs * i as i64);
        bucket_starts.push(min_ts + offset);
    }

    for t in index.timestamps.iter().flatten() {
        let offset_secs = (*t - min_ts).num_seconds().max(0);
        let bucket_idx = (offset_secs / bucket_duration_secs) as usize;
        let bucket_idx = bucket_idx.min(num_buckets - 1);
        buckets[bucket_idx] += 1;
    }

    Some(SparklineData {
        buckets,
        bucket_starts,
        bucket_duration_secs,
        num_buckets,
    })
}

pub fn bucket_range_to_time_range(
    sparkline: &SparklineData,
    start: usize,
    end: usize,
) -> Option<TimeRange> {
    if start > end || start >= sparkline.num_buckets {
        return None;
    }
    let end = end.min(sparkline.num_buckets - 1);
    let start_time = sparkline.bucket_starts[start];
    let end_time =
        sparkline.bucket_starts[end] + chrono::Duration::seconds(sparkline.bucket_duration_secs);
    Some(TimeRange {
        start: start_time,
        end: end_time,
    })
}

pub fn filter_by_time_range(index: &TimeIndex, range: &TimeRange, indices: &[usize]) -> Vec<usize> {
    indices
        .iter()
        .copied()
        .filter(|&i| {
            if let Some(ts) = index.timestamp_at(i) {
                ts >= range.start && ts <= range.end
            } else {
                false
            }
        })
        .collect()
}

/// Format a NaiveDateTime for display in the sparkline.
/// Uses "HH:MM" for same-day, "MM-DD HH:MM" for multi-day.
pub fn format_sparkline_time(dt: NaiveDateTime, multi_day: bool) -> String {
    if multi_day {
        dt.format("%m-%d %H:%M").to_string()
    } else {
        dt.format("%H:%M").to_string()
    }
}

/// Check if a time range spans more than a single calendar day.
pub fn is_multi_day(min_ts: NaiveDateTime, max_ts: NaiveDateTime) -> bool {
    min_ts.date() != max_ts.date()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rfc3339_z() {
        let dt = parse_timestamp("2024-01-15T08:30:01Z").unwrap();
        assert_eq!(dt.to_string(), "2024-01-15 08:30:01");
    }

    #[test]
    fn test_parse_rfc3339_offset() {
        let dt = parse_timestamp("2024-01-15T08:30:01+00:00").unwrap();
        assert_eq!(dt.to_string(), "2024-01-15 08:30:01");
    }

    #[test]
    fn test_parse_rfc3339_frac() {
        let dt = parse_timestamp("2024-01-15T08:30:01.123Z").unwrap();
        assert_eq!(
            dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
            "2024-01-15 08:30:01.123"
        );
    }

    #[test]
    fn test_parse_iso_no_offset() {
        let dt = parse_timestamp("2024-01-15T08:30:01.123").unwrap();
        assert_eq!(
            dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
            "2024-01-15 08:30:01.123"
        );
    }

    #[test]
    fn test_parse_space_separated() {
        let dt = parse_timestamp("2024-01-15 08:30:01").unwrap();
        assert_eq!(dt.to_string(), "2024-01-15 08:30:01");
    }

    #[test]
    fn test_parse_python_comma() {
        let dt = parse_timestamp("2024-01-15 08:30:01,123").unwrap();
        assert!(dt.to_string().starts_with("2024-01-15 08:30:01"));
    }

    #[test]
    fn test_parse_epoch_millis() {
        // 2024-01-15T08:30:00Z = 1705307400 * 1000 = 1705307400000
        let dt = parse_timestamp("1705307400000").unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-01-15");
    }

    #[test]
    fn test_parse_epoch_secs() {
        let dt = parse_timestamp("1705307400").unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-01-15");
    }

    #[test]
    fn test_parse_empty_returns_none() {
        assert!(parse_timestamp("").is_none());
        assert!(parse_timestamp("   ").is_none());
    }

    #[test]
    fn test_parse_garbage_returns_none() {
        assert!(parse_timestamp("not a timestamp").is_none());
    }
}
