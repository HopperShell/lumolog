use memmap2::Mmap;
use std::fs::File;
use std::io::{self, BufRead, Read};
use std::path::Path;

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
    pub fn from_reader<R: Read>(reader: R) -> Self {
        let reader = io::BufReader::new(reader);
        let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
        Self { lines }
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }
}
