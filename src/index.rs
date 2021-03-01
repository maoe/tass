use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::Range;
use std::path::Path;

/// Records the locations of all newlines in a file.
pub struct Index {
    offset: u64,
    newlines: Vec<u64>,
    file: BufReader<File>,
    watch_for_updates: bool,
}

impl Index {
    pub fn new(path: &Path) -> anyhow::Result<Index> {
        let mut ret = Index {
            offset: 0,
            file: BufReader::new(File::open(path)?),
            newlines: vec![],
            watch_for_updates: true,
        };
        ret.update()?;
        Ok(ret)
    }

    pub fn stop_watching(&mut self) {
        self.watch_for_updates = false;
    }

    /// Reads the file, starting at EOF the last time this function was
    /// called, up to the current EOF, adding line-break offsets to `newlines`.
    pub fn update(&mut self) -> anyhow::Result<()> {
        if !self.watch_for_updates {
            return Ok(());
        }
        let n_lines_start = self.len();
        loop {
            if self.len() - n_lines_start > 1_000_000 {
                self.up_to_date = false;
                return Ok(());
            }
            let buf = self.file.fill_buf()?;
            if buf.is_empty() {
                return Ok(());
            }
            if let Some(x) = memchr::memchr(b'\n', buf) {
                self.newlines.push(self.offset + x as u64);
                self.offset += x as u64 + 1;
                self.file.consume(x + 1);
            } else {
                let x = buf.len();
                self.offset += x as u64;
                self.file.consume(x);
            }
        }
    }

    /// Gives a byte-range which doesn't include the newline
    pub fn line2range(&self, line: usize) -> Range<u64> {
        let lhs = if line == 0 {
            0
        } else {
            self.newlines[line - 1] as u64 + 1
        };
        let rhs = self.newlines[line] as u64;
        lhs..rhs
    }

    pub fn line2pos(&self, mut line: usize) -> csv::Position {
        line += 1;
        let mut pos = csv::Position::new();
        pos.set_line(line as u64)
            .set_byte(self.line2range(line).start)
            .set_record(0);
        pos
    }

    pub fn len(&self) -> usize {
        self.newlines.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::*;

    #[test]
    fn test() {
        let mut f = NamedTempFile::new().unwrap();
        let s = b"foo,bar\n1,2\n3,4\n";
        f.write_all(s).unwrap();
        let lines = Index::new(f.path()).unwrap();
        assert_eq!(lines.len(), 3);
        // line2range never includes the newline char, hence the non-contiguous
        // ranges
        assert_eq!(lines.line2range(0), 0..7);
        assert_eq!(lines.line2range(1), 8..11);
        assert_eq!(lines.line2range(2), 12..15);
        assert_eq!(s.len(), 16);
    }
}
