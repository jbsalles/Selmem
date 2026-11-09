//! Reusable experiment I/O. Protocol-specific fixtures and decisions stay in binaries.
use crate::util::json::{self, Json};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub type Result<T> = std::result::Result<T, String>;

pub fn read_jsonl(path: &Path) -> Result<Vec<Json>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    fs::read_to_string(path)
        .map_err(|e| e.to_string())?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(json::parse)
        .collect()
}

pub fn write_bytes(path: &Path, content: impl AsRef<[u8]>) -> Result<()> {
    fs::write(path, content).map_err(|e| e.to_string())
}

pub fn append_json(path: &Path, row: &Json) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{}", row.encode()).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_journal_is_an_empty_collection() {
        assert!(
            read_jsonl(Path::new("/unlikely-selmem-journal-no-file-862739.jsonl"))
                .unwrap()
                .is_empty()
        );
    }
}
