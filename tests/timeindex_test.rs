use lumolog::parser::{detect_format, parse_line};
use lumolog::timeindex::{
    bucket_range_to_time_range, build_time_index, compute_sparkline, filter_by_time_range,
    parse_timestamp,
};

fn make_parsed_lines(raw: &[&str]) -> Vec<lumolog::parser::ParsedLine> {
    let lines: Vec<String> = raw.iter().map(|s| s.to_string()).collect();
    let format = detect_format(&lines);
    lines.iter().map(|l| parse_line(l, format)).collect()
}

// --- Timestamp parsing tests ---

#[test]
fn test_parse_rfc3339_zulu() {
    let dt = parse_timestamp("2024-01-15T08:30:01Z").unwrap();
    assert_eq!(dt.to_string(), "2024-01-15 08:30:01");
}

#[test]
fn test_parse_rfc3339_with_offset() {
    let dt = parse_timestamp("2024-01-15T08:30:01+05:30").unwrap();
    // Stored as UTC: 08:30:01 - 05:30 = 03:00:01
    assert_eq!(dt.to_string(), "2024-01-15 03:00:01");
}

#[test]
fn test_parse_rfc3339_fractional_zulu() {
    let dt = parse_timestamp("2024-01-15T08:30:01.456Z").unwrap();
    assert!(dt.to_string().starts_with("2024-01-15 08:30:01"));
}

#[test]
fn test_parse_iso_no_offset() {
    let dt = parse_timestamp("2024-01-15T08:30:01").unwrap();
    assert_eq!(dt.to_string(), "2024-01-15 08:30:01");
}

#[test]
fn test_parse_space_separated() {
    let dt = parse_timestamp("2024-01-15 08:30:01").unwrap();
    assert_eq!(dt.to_string(), "2024-01-15 08:30:01");
}

#[test]
fn test_parse_space_separated_frac() {
    let dt = parse_timestamp("2024-01-15 08:30:01.999").unwrap();
    assert!(dt.to_string().starts_with("2024-01-15 08:30:01"));
}

#[test]
fn test_parse_python_comma_frac() {
    let dt = parse_timestamp("2024-01-15 08:30:01,123").unwrap();
    assert!(dt.to_string().starts_with("2024-01-15 08:30:01"));
}

#[test]
fn test_parse_epoch_millis() {
    let dt = parse_timestamp("1705307400000").unwrap();
    assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-01-15");
}

#[test]
fn test_parse_epoch_secs() {
    let dt = parse_timestamp("1705307400").unwrap();
    assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-01-15");
}

#[test]
fn test_parse_syslog_format() {
    let dt = parse_timestamp("Jan 15 08:30:00").unwrap();
    // Year will be current year, but month/day/time should match
    assert_eq!(dt.format("%m-%d %H:%M:%S").to_string(), "01-15 08:30:00");
}

#[test]
fn test_parse_klog_format() {
    let dt = parse_timestamp("0115 08:30:00.000000").unwrap();
    assert_eq!(dt.format("%m-%d %H:%M:%S").to_string(), "01-15 08:30:00");
}

#[test]
fn test_parse_apache_clf() {
    let dt = parse_timestamp("10/Oct/2024:13:55:36 +0000").unwrap();
    assert_eq!(dt.to_string(), "2024-10-10 13:55:36");
}

#[test]
fn test_parse_returns_none_for_garbage() {
    assert!(parse_timestamp("not a timestamp").is_none());
    assert!(parse_timestamp("").is_none());
    assert!(parse_timestamp("hello world 123").is_none());
}

// --- Time index building tests ---

#[test]
fn test_build_time_index_json() {
    let lines = make_parsed_lines(&[
        r#"{"timestamp":"2024-01-15T08:30:01Z","level":"info","message":"msg1"}"#,
        r#"{"timestamp":"2024-01-15T08:30:05Z","level":"info","message":"msg2"}"#,
        r#"{"timestamp":"2024-01-15T08:31:00Z","level":"warn","message":"msg3"}"#,
    ]);
    let index = build_time_index(&lines);
    assert!(index.has_timestamps());
    assert_eq!(index.len(), 3);
    assert!(index.timestamp_at(0).is_some());
    assert!(index.timestamp_at(1).is_some());
    assert!(index.timestamp_at(2).is_some());
    assert!(index.min_ts.unwrap() < index.max_ts.unwrap());
}

#[test]
fn test_forward_fill() {
    // Line 1 has timestamp, line 2 doesn't (invalid JSON for timestamp field), line 3 has timestamp
    let lines = make_parsed_lines(&[
        r#"{"timestamp":"2024-01-15T08:30:01Z","level":"info","message":"msg1"}"#,
        r#"{"level":"info","message":"msg2 no timestamp"}"#,
        r#"{"timestamp":"2024-01-15T08:31:00Z","level":"warn","message":"msg3"}"#,
    ]);
    let index = build_time_index(&lines);

    // Line 0: has its own timestamp
    let ts0 = index.timestamp_at(0).unwrap();
    // Line 1: forward-filled from line 0
    let ts1 = index.timestamp_at(1).unwrap();
    assert_eq!(ts0, ts1);
    // Line 2: has its own timestamp
    let ts2 = index.timestamp_at(2).unwrap();
    assert!(ts2 > ts0);
}

#[test]
fn test_no_timestamps_means_no_index() {
    let lines = make_parsed_lines(&["plain line 1", "plain line 2", "plain line 3"]);
    let index = build_time_index(&lines);
    assert!(!index.has_timestamps());
}

// --- Sparkline computation tests ---

#[test]
fn test_sparkline_bucket_counts_sum_to_total() {
    let lines = make_parsed_lines(&[
        r#"{"timestamp":"2024-01-15T08:30:01Z","level":"info","message":"a"}"#,
        r#"{"timestamp":"2024-01-15T08:30:02Z","level":"info","message":"b"}"#,
        r#"{"timestamp":"2024-01-15T08:30:03Z","level":"info","message":"c"}"#,
        r#"{"timestamp":"2024-01-15T08:31:00Z","level":"warn","message":"d"}"#,
        r#"{"timestamp":"2024-01-15T08:32:00Z","level":"error","message":"e"}"#,
    ]);
    let index = build_time_index(&lines);
    let sparkline = compute_sparkline(&index, 10).unwrap();

    let total: u64 = sparkline.buckets.iter().sum();
    assert_eq!(total, 5);
    assert_eq!(sparkline.num_buckets, 10);
}

#[test]
fn test_sparkline_single_bucket() {
    let lines = make_parsed_lines(&[
        r#"{"timestamp":"2024-01-15T08:30:01Z","level":"info","message":"a"}"#,
        r#"{"timestamp":"2024-01-15T08:30:02Z","level":"info","message":"b"}"#,
    ]);
    let index = build_time_index(&lines);
    let sparkline = compute_sparkline(&index, 1).unwrap();
    assert_eq!(sparkline.buckets[0], 2);
}

#[test]
fn test_sparkline_no_timestamps_returns_none() {
    let lines = make_parsed_lines(&["plain1", "plain2"]);
    let index = build_time_index(&lines);
    assert!(compute_sparkline(&index, 10).is_none());
}

// --- Time range filtering tests ---

#[test]
fn test_filter_by_time_range() {
    let lines = make_parsed_lines(&[
        r#"{"timestamp":"2024-01-15T08:00:00Z","level":"info","message":"early"}"#,
        r#"{"timestamp":"2024-01-15T09:00:00Z","level":"info","message":"mid"}"#,
        r#"{"timestamp":"2024-01-15T10:00:00Z","level":"warn","message":"late"}"#,
    ]);
    let index = build_time_index(&lines);

    // Filter to only the 09:00 line
    let mid_ts = index.timestamp_at(1).unwrap();
    let range = lumolog::timeindex::TimeRange {
        start: mid_ts,
        end: mid_ts,
    };
    let all_indices: Vec<usize> = (0..3).collect();
    let filtered = filter_by_time_range(&index, &range, &all_indices);
    assert_eq!(filtered, vec![1]);
}

#[test]
fn test_bucket_range_to_time_range() {
    let lines = make_parsed_lines(&[
        r#"{"timestamp":"2024-01-15T08:00:00Z","level":"info","message":"a"}"#,
        r#"{"timestamp":"2024-01-15T10:00:00Z","level":"info","message":"b"}"#,
    ]);
    let index = build_time_index(&lines);
    let sparkline = compute_sparkline(&index, 10).unwrap();

    let range = bucket_range_to_time_range(&sparkline, 0, 0).unwrap();
    assert!(range.start <= range.end);
    assert_eq!(range.start, sparkline.bucket_starts[0]);
}

// --- Time index append tests ---

#[test]
fn test_time_index_append() {
    let lines = make_parsed_lines(&[
        r#"{"timestamp":"2024-01-15T08:00:00Z","level":"info","message":"a"}"#,
    ]);
    let mut index = build_time_index(&lines);
    assert_eq!(index.len(), 1);

    let new_lines = make_parsed_lines(&[
        r#"{"timestamp":"2024-01-15T09:00:00Z","level":"info","message":"b"}"#,
    ]);
    index.append(&new_lines);
    assert_eq!(index.len(), 2);
    assert!(index.timestamp_at(1).unwrap() > index.timestamp_at(0).unwrap());
}

// --- Integration test with real sample data ---

#[test]
fn test_build_from_sample_json_log() {
    let content = std::fs::read_to_string("testdata/sample_json.log").unwrap();
    let raw: Vec<String> = content.lines().map(String::from).collect();
    let format = detect_format(&raw);
    let parsed: Vec<lumolog::parser::ParsedLine> =
        raw.iter().map(|l| parse_line(l, format)).collect();
    let index = build_time_index(&parsed);

    assert!(index.has_timestamps());
    // sample_json.log has 5 lines, all with timestamps
    assert_eq!(index.len(), parsed.len());

    let sparkline = compute_sparkline(&index, 20).unwrap();
    let total: u64 = sparkline.buckets.iter().sum();
    assert_eq!(total as usize, parsed.len());
}
