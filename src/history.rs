//! Zathura reading history parser.

use crate::library::BookPath;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReadingRecord {
    pub page: u32,
    pub read_at: i64,
}

pub fn parse(text: &str) -> HashMap<BookPath, ReadingRecord> {
    let mut records = HashMap::new();
    let mut section: Option<String> = None;
    let mut options: HashMap<String, String> = HashMap::new();

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            if let Some(name) = section.take() {
                records.insert(BookPath::new(PathBuf::from(name)), record(&options));
            }
            section = Some(line[1..line.len() - 1].to_string());
            options.clear();
        } else if section.is_some() {
            if let Some((key, value)) = line.split_once('=') {
                options.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }
    if let Some(name) = section {
        records.insert(BookPath::new(PathBuf::from(name)), record(&options));
    }
    records
}

fn record(options: &HashMap<String, String>) -> ReadingRecord {
    ReadingRecord {
        page: options
            .get("page")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        read_at: options
            .get("time")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
    }
}

pub fn load(path: &Path) -> HashMap<BookPath, ReadingRecord> {
    std::fs::read_to_string(path)
        .map(|text| parse(&text))
        .unwrap_or_default()
}

pub fn history_path(home: &Path, xdg_data_home: Option<&Path>) -> PathBuf {
    match xdg_data_home {
        Some(directory) => directory.join("zathura").join("history"),
        None => home.join(".local/share/zathura/history"),
    }
}
