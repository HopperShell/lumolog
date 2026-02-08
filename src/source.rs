use memmap2::Mmap;
use std::fs::File;
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
