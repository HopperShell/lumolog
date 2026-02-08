use memmap2::Mmap;
use std::fs::File;
use std::io::{self, BufRead, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

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
