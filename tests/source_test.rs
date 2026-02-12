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

use lumolog::source::StdinSource;
use std::io::Cursor;

#[test]
fn test_stdin_source_reads_lines() {
    let input = "line 1\nline 2\nline 3\n";
    let cursor = Cursor::new(input);
    let source = StdinSource::from_reader(cursor);
    let lines = source.lines();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "line 1");
}

use lumolog::source::FollowableStdinSource;
use std::time::Duration;

#[test]
fn test_followable_stdin_from_reader() {
    let input = "line 1\nline 2\nline 3\n";
    let cursor = Cursor::new(input);
    let mut src = FollowableStdinSource::from_reader(cursor);
    let initial = src.recv_initial(Duration::from_millis(100));
    assert_eq!(initial, vec!["line 1", "line 2", "line 3"]);
}

#[test]
fn test_followable_stdin_read_new_lines() {
    let input = "line 1\nline 2\n";
    let cursor = Cursor::new(input);
    let mut src = FollowableStdinSource::from_reader(cursor);
    std::thread::sleep(Duration::from_millis(50));
    let lines = src.read_new_lines();
    assert_eq!(lines, vec!["line 1", "line 2"]);
    let empty = src.read_new_lines();
    assert!(empty.is_empty());
}

#[test]
fn test_followable_stdin_detects_closed() {
    let input = "done\n";
    let cursor = Cursor::new(input);
    let mut src = FollowableStdinSource::from_reader(cursor);
    std::thread::sleep(Duration::from_millis(50));
    let _ = src.read_new_lines();
    // Reader exhausted → sender dropped → channel disconnected
    let _ = src.read_new_lines();
    assert!(src.is_closed());
}

#[cfg(unix)]
#[test]
fn test_followable_stdin_streams_from_pipe() {
    use std::io::Write;
    use std::os::unix::io::FromRawFd;

    let (read_fd, write_fd) = {
        let mut fds = [0i32; 2];
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
        (fds[0], fds[1])
    };

    let reader = unsafe { std::fs::File::from_raw_fd(read_fd) };
    let mut writer = unsafe { std::fs::File::from_raw_fd(write_fd) };

    let mut src = FollowableStdinSource::from_reader(reader);

    writeln!(writer, "first").unwrap();
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(src.read_new_lines(), vec!["first"]);

    writeln!(writer, "second").unwrap();
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(src.read_new_lines(), vec!["second"]);

    drop(writer);
    std::thread::sleep(Duration::from_millis(50));
    let _ = src.read_new_lines(); // drain + detect disconnect
    assert!(src.is_closed());
}
