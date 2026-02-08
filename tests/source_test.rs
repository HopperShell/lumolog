use std::io::Write;
use tempfile::NamedTempFile;

use lumolog::source::FileSource;

#[test]
fn test_file_source_reads_lines() {
    let source = FileSource::open("testdata/sample_plain.log").unwrap();
    let lines = source.lines();
    assert_eq!(lines.len(), 10);
    assert!(lines[0].contains("Application starting up"));
    assert!(lines[9].contains("Unhandled exception"));
}

#[test]
fn test_file_source_line_count() {
    let source = FileSource::open("testdata/sample_plain.log").unwrap();
    assert_eq!(source.line_count(), 10);
}

#[test]
fn test_file_source_missing_file() {
    let result = FileSource::open("nonexistent.log");
    assert!(result.is_err());
}

#[test]
fn test_large_file_line_count() {
    let mut file = NamedTempFile::new().unwrap();
    for i in 0..10_000 {
        writeln!(file, "2024-01-15 INFO Line number {}", i).unwrap();
    }
    let source = FileSource::open(file.path()).unwrap();
    assert_eq!(source.line_count(), 10_000);
}
