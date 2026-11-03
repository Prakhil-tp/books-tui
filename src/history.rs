//! Reading progress: zathura's plain and sqlite histories, merged into a
//! books-tui-owned copy so progress survives zathura losing its own file.

use crate::library::BookPath;
use anyhow::Result;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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

/// Reads zathura's sqlite database (`database sqlite`). Any failure yields an
/// empty map; the file is opened read-only and never created.
pub fn load_sqlite(path: &Path) -> HashMap<BookPath, ReadingRecord> {
    query_sqlite(path).unwrap_or_default()
}

fn query_sqlite(path: &Path) -> rusqlite::Result<HashMap<BookPath, ReadingRecord>> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.busy_timeout(std::time::Duration::from_millis(200))?;
    // zathura stores `time` as DATETIME('now'), i.e. UTC "YYYY-MM-DD HH:MM:SS".
    let mut statement = connection
        .prepare("SELECT file, page, CAST(strftime('%s', time) AS INTEGER) FROM fileinfo")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            ReadingRecord {
                page: row.get::<_, Option<i64>>(1)?.unwrap_or(0).max(0) as u32,
                read_at: row.get::<_, Option<i64>>(2)?.unwrap_or(0),
            },
        ))
    })?;
    Ok(rows
        .filter_map(Result::ok)
        .map(|(file, record)| (BookPath::new(PathBuf::from(file)), record))
        .collect())
}

/// Merges `from` into `into`, keeping the most recently read record per book.
/// Returns whether `into` changed.
pub fn merge(
    into: &mut HashMap<BookPath, ReadingRecord>,
    from: HashMap<BookPath, ReadingRecord>,
) -> bool {
    let mut changed = false;
    for (path, record) in from {
        match into.get(&path) {
            Some(existing) if existing.read_at > record.read_at || *existing == record => {}
            _ => {
                into.insert(path, record);
                changed = true;
            }
        }
    }
    changed
}

/// books-tui's own durable copy of reading progress, written atomically.
pub struct ProgressStore {
    path: PathBuf,
}

impl ProgressStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> HashMap<BookPath, ReadingRecord> {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|text| serde_json::from_str::<HashMap<PathBuf, ReadingRecord>>(&text).ok())
            .map(|entries| {
                entries
                    .into_iter()
                    .map(|(path, record)| (BookPath::new(path), record))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn save(&self, records: &HashMap<BookPath, ReadingRecord>) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let entries: HashMap<&Path, &ReadingRecord> = records
            .iter()
            .map(|(path, record)| (path.as_path(), record))
            .collect();
        let temporary = self.path.with_extension("json.tmp");
        std::fs::write(&temporary, serde_json::to_string(&entries)?)?;
        std::fs::rename(&temporary, &self.path)?;
        Ok(())
    }
}

/// All progress sources, combined: own copy, then zathura plain, then sqlite.
pub struct ReadingHistory {
    plain: PathBuf,
    sqlite: PathBuf,
    store: ProgressStore,
}

impl ReadingHistory {
    pub fn new(plain: PathBuf, sqlite: PathBuf, store: PathBuf) -> Self {
        Self {
            plain,
            sqlite,
            store: ProgressStore::new(store),
        }
    }

    pub fn load(&self) -> HashMap<BookPath, ReadingRecord> {
        let mut records = self.store.load();
        let plain = merge(&mut records, load(&self.plain));
        let sqlite = merge(&mut records, load_sqlite(&self.sqlite));
        if plain || sqlite {
            // Best effort: failing to persist must not hide progress on screen.
            let _ = self.store.save(&records);
        }
        records
    }
}
