//! PDF metadata via `pdfinfo`, with a JSON cache.

use crate::library::BookPath;
use crate::ports::CommandRunner;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BookMetadata {
    pub author: String,
    pub pages: u32,
    pub year: u32,
}

pub trait MetadataReader: Send {
    /// `None` when the metadata could not be read (nothing should be cached).
    fn read(&self, path: &BookPath) -> Option<BookMetadata>;
}

pub struct PdfInfoReader<R: CommandRunner> {
    runner: R,
}

impl<R: CommandRunner> PdfInfoReader<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }
}

impl<R: CommandRunner + Send> MetadataReader for PdfInfoReader<R> {
    fn read(&self, path: &BookPath) -> Option<BookMetadata> {
        let path_text = path.as_path().to_string_lossy().into_owned();
        let output = self.runner.run(&["pdfinfo", &path_text]).ok()?;
        if !output.status.success() {
            return None;
        }
        Some(parse_pdfinfo(&String::from_utf8_lossy(&output.stdout)))
    }
}

pub fn parse_pdfinfo(text: &str) -> BookMetadata {
    let mut fields: HashMap<String, String> = HashMap::new();
    for line in text.lines() {
        if let Some((key, value)) = line.split_once(':') {
            fields.insert(key.trim().to_lowercase(), value.trim().to_string());
        }
    }
    BookMetadata {
        author: fields.get("author").cloned().unwrap_or_default(),
        pages: fields
            .get("pages")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        year: fields
            .get("creationdate")
            .and_then(|value| year_regex().find(value))
            .and_then(|found| found.as_str().parse().ok())
            .unwrap_or(0),
    }
}

fn year_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"\b(?:19|20)\d{2}\b").unwrap())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub stamp: (i64, u64),
    pub author: String,
    pub pages: u32,
    pub year: u32,
}

impl CacheEntry {
    fn metadata(&self) -> BookMetadata {
        BookMetadata {
            author: self.author.clone(),
            pages: self.pages,
            year: self.year,
        }
    }
}

pub struct JsonStore {
    path: PathBuf,
}

impl JsonStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> HashMap<String, CacheEntry> {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, entries: &HashMap<String, CacheEntry>) {
        let Some(parent) = self.path.parent() else {
            return;
        };
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
        let temporary = self.path.with_extension("json.tmp");
        let Ok(text) = serde_json::to_string(entries) else {
            return;
        };
        if std::fs::write(&temporary, text).is_err() {
            return;
        }
        let _ = std::fs::rename(&temporary, &self.path);
    }
}

pub struct MetadataCache<R: MetadataReader> {
    reader: R,
    store: JsonStore,
    entries: HashMap<String, CacheEntry>,
}

impl<R: MetadataReader> MetadataCache<R> {
    pub fn new(reader: R, store: JsonStore) -> Self {
        let entries = store.load();
        Self {
            reader,
            store,
            entries,
        }
    }

    pub fn metadata_for(&mut self, path: &BookPath) -> BookMetadata {
        let Some(stamp) = stamp(path.as_path()) else {
            return self.reader.read(path).unwrap_or_default();
        };
        let key = path.as_path().to_string_lossy().into_owned();
        if let Some(entry) = self.entries.get(&key) {
            if entry.stamp == stamp {
                return entry.metadata();
            }
        }
        let Some(metadata) = self.reader.read(path) else {
            return BookMetadata::default();
        };
        self.entries.insert(
            key,
            CacheEntry {
                stamp,
                author: metadata.author.clone(),
                pages: metadata.pages,
                year: metadata.year,
            },
        );
        self.store.save(&self.entries);
        metadata
    }
}

fn stamp(path: &Path) -> Option<(i64, u64)> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let seconds = modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    Some((seconds, metadata.len()))
}
