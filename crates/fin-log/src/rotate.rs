use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// `name.log` plus up to `keep` older files `name.1.log` (newest) to `name.<keep>.log`.
pub struct RotatingFile {
    dir: PathBuf,
    name: String,
    max_bytes: u64,
    keep: usize,
    file: File,
    size: u64,
}

impl RotatingFile {
    pub fn open(dir: &Path, name: &str, max_bytes: u64, keep: usize) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let path = dir.join(format!("{name}.log"));
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let size = file.metadata()?.len();
        Ok(Self {
            dir: dir.to_path_buf(),
            name: name.to_string(),
            max_bytes,
            keep: keep.max(1),
            file,
            size,
        })
    }

    fn path(&self, n: usize) -> PathBuf {
        if n == 0 {
            self.dir.join(format!("{}.log", self.name))
        } else {
            self.dir.join(format!("{}.{n}.log", self.name))
        }
    }

    fn rotate(&mut self) -> io::Result<()> {
        let _ = fs::remove_file(self.path(self.keep));
        for n in (0..self.keep).rev() {
            if self.path(n).exists() {
                fs::rename(self.path(n), self.path(n + 1))?;
            }
        }
        self.file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.path(0))?;
        self.size = 0;
        Ok(())
    }

    pub fn append(&mut self, line: &str) -> io::Result<()> {
        let bytes = line.len() as u64 + 1;
        if self.size > 0 && self.size + bytes > self.max_bytes {
            self.rotate()?;
        }
        writeln!(self.file, "{line}")?;
        self.size += bytes;
        Ok(())
    }

    /// The last `max_lines` lines, oldest first, reading back through older files as needed.
    pub fn tail(&self, max_lines: usize) -> String {
        let mut lines: Vec<String> = vec![];
        for n in 0..=self.keep {
            if lines.len() >= max_lines {
                break;
            }
            if let Ok(text) = fs::read_to_string(self.path(n)) {
                let mut older: Vec<String> = text.lines().map(String::from).collect();
                older.extend(lines);
                lines = older;
            }
        }
        let start = lines.len().saturating_sub(max_lines);
        lines[start..].join("\n")
    }
}
