use memmap2::Mmap;
use std::fs::File;
use std::io::{self, BufRead, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

pub struct FileSource {
    lines: Vec<String>,
}

impl FileSource {
    pub fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;

        if metadata.len() == 0 {
            return Ok(Self { lines: Vec::new() });
        }

        let mmap = unsafe { Mmap::map(&file)? };
        let content = std::str::from_utf8(&mmap)?;
        let lines: Vec<String> = content.lines().map(String::from).collect();
        Ok(Self { lines })
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    #[allow(dead_code)] // used by integration tests
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

pub struct StdinSource {
    lines: Vec<String>,
}

impl StdinSource {
    /// Read all available input from stdin (for non-streaming use).
    pub fn read_all() -> anyhow::Result<Self> {
        let stdin = io::stdin();
        let lines: Vec<String> = stdin.lock().lines().collect::<Result<_, _>>()?;
        Ok(Self { lines })
    }

    /// For testing: read from any reader.
    #[allow(dead_code)] // used by integration tests
    pub fn from_reader<R: Read>(reader: R) -> Self {
        let reader = io::BufReader::new(reader);
        let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
        Self { lines }
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }
}

pub struct FollowableSource {
    path: PathBuf,
    offset: u64,
}

impl FollowableSource {
    pub fn new<P: AsRef<Path>>(path: P, initial_offset: u64) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            offset: initial_offset,
        }
    }

    /// Read any new lines appended since the last read.
    /// Returns an empty vec if the file hasn't grown.
    pub fn read_new_lines(&mut self) -> anyhow::Result<Vec<String>> {
        let mut file = File::open(&self.path)?;
        let len = file.metadata()?.len();

        if len <= self.offset {
            return Ok(Vec::new());
        }

        file.seek(SeekFrom::Start(self.offset))?;

        let mut buf = Vec::with_capacity((len - self.offset) as usize);
        file.read_to_end(&mut buf)?;

        self.offset = len;

        let text = String::from_utf8_lossy(&buf);
        let lines: Vec<String> = text.lines().map(String::from).collect();

        Ok(lines)
    }
}

pub struct FollowableStdinSource {
    receiver: mpsc::Receiver<String>,
    closed: bool,
}

impl FollowableStdinSource {
    /// Spawn a background reader on the current stdin file descriptor.
    /// Dups stdin fd so it remains valid even after dup2 redirects fd 0 to /dev/tty.
    /// Must be called BEFORE any dup2 that redirects stdin.
    #[cfg(unix)]
    pub fn spawn_stdin() -> Self {
        use std::os::unix::io::FromRawFd;
        let fd = unsafe { libc::dup(libc::STDIN_FILENO) };
        assert!(fd >= 0, "Failed to dup stdin fd");
        let file = unsafe { File::from_raw_fd(fd) };
        Self::from_reader(file)
    }

    /// Create from any reader. Spawns a background thread that reads lines
    /// and sends them through an mpsc channel.
    pub fn from_reader<R: io::Read + Send + 'static>(reader: R) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let buf_reader = io::BufReader::new(reader);
            for line in buf_reader.lines() {
                match line {
                    Ok(l) => {
                        if tx.send(l).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            receiver: rx,
            closed: false,
        }
    }

    /// Collect initial lines with a timeout.
    /// Waits up to `timeout` for the first line, then drains all immediately
    /// available lines (with 10ms gaps to catch burst data).
    pub fn recv_initial(&mut self, timeout: Duration) -> Vec<String> {
        let mut lines = Vec::new();
        match self.receiver.recv_timeout(timeout) {
            Ok(first) => {
                lines.push(first);
                while let Ok(line) = self.receiver.recv_timeout(Duration::from_millis(10)) {
                    lines.push(line);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                self.closed = true;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        lines
    }

    /// Non-blocking drain of all available lines from the channel.
    pub fn read_new_lines(&mut self) -> Vec<String> {
        if self.closed {
            return Vec::new();
        }
        let mut lines = Vec::new();
        loop {
            match self.receiver.try_recv() {
                Ok(line) => lines.push(line),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.closed = true;
                    break;
                }
            }
        }
        lines
    }

    /// Returns true if the stdin pipe has been closed (EOF / writer dropped).
    pub fn is_closed(&self) -> bool {
        self.closed
    }
}
