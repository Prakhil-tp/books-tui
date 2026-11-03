use books_tui::history::{history_path, load, parse};
use books_tui::library::BookPath;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

const HISTORY: &str = "[/home/me/books/one.pdf]
page=19
offset=0
time=1793193751
jumplist=

[hashed-entry-not-a-path]
page=4
time=1793000000

[/home/me/books/two.pdf]
page=3
time=1793100000
";

fn key(path: &str) -> BookPath {
    BookPath::new(PathBuf::from(path))
}

#[test]
fn parses_page_and_time_by_path() {
    let records = parse(HISTORY);
    assert_eq!(records[&key("/home/me/books/one.pdf")].page, 19);
    assert_eq!(records[&key("/home/me/books/one.pdf")].read_at, 1793193751);
    assert_eq!(records[&key("/home/me/books/two.pdf")].page, 3);
}

#[test]
fn tolerates_unknown_options_and_missing_values() {
    let records = parse("[a.pdf]\noffset=3\npage=many\ntime=\n");
    assert_eq!(records[&key("a.pdf")].page, 0);
    assert_eq!(records[&key("a.pdf")].read_at, 0);
}

#[test]
fn missing_history_file_yields_an_empty_map() {
    assert!(load(Path::new("/definitely/missing/history")).is_empty());
}

#[test]
fn loads_history_from_disk() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("history");
    std::fs::write(&path, HISTORY).unwrap();
    assert_eq!(load(&path).len(), 3);
}

#[test]
fn history_path_prefers_xdg_data_home() {
    let home = Path::new("/home/me");
    let xdg = Path::new("/xdg");
    assert_eq!(
        history_path(home, Some(xdg)),
        PathBuf::from("/xdg/zathura/history")
    );
    assert_eq!(
        history_path(home, None),
        PathBuf::from("/home/me/.local/share/zathura/history")
    );
}

// --- zathura sqlite backend -------------------------------------------------

use books_tui::history::{load_sqlite, merge, ProgressStore, ReadingHistory, ReadingRecord};
use std::collections::HashMap;

const ZATHURA_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS fileinfo (file TEXT PRIMARY KEY,page INTEGER,offset INTEGER,zoom FLOAT,rotation INTEGER,pages_per_row INTEGER,first_page_column TEXT,position_x FLOAT,position_y FLOAT,time TIMESTAMP,page_right_to_left INTEGER,sha256 BLOB);";

fn write_sqlite(path: &Path, rows: &[(&str, u32, &str)]) {
    let connection = rusqlite::Connection::open(path).unwrap();
    connection.execute_batch(ZATHURA_SCHEMA).unwrap();
    for (file, page, time) in rows {
        connection
            .execute(
                "REPLACE INTO fileinfo (file, page, time) VALUES (?1, ?2, ?3)",
                rusqlite::params![file, page, time],
            )
            .unwrap();
    }
}

fn rec(page: u32, read_at: i64) -> ReadingRecord {
    ReadingRecord { page, read_at }
}

#[test]
fn reads_zathura_sqlite_fileinfo() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bookmarks.sqlite");
    write_sqlite(&path, &[("/books/a.pdf", 42, "2026-11-02 13:54:10")]);
    let records = load_sqlite(&path);
    // 2026-11-02 13:54:10 UTC
    assert_eq!(records[&key("/books/a.pdf")], rec(42, 1793627650));
}

#[test]
fn missing_or_foreign_sqlite_yields_an_empty_map() {
    let dir = tempdir().unwrap();
    assert!(load_sqlite(&dir.path().join("missing.sqlite")).is_empty());
    let other = dir.path().join("other.sqlite");
    rusqlite::Connection::open(&other)
        .unwrap()
        .execute_batch("CREATE TABLE t (x INTEGER);")
        .unwrap();
    assert!(load_sqlite(&other).is_empty());
    assert!(!dir.path().join("missing.sqlite").exists());
}

// --- merging and books-tui's own copy ----------------------------------------

#[test]
fn merge_keeps_the_most_recent_record_per_book() {
    let mut into = HashMap::from([(key("a"), rec(10, 200)), (key("b"), rec(5, 100))]);
    let from = HashMap::from([
        (key("a"), rec(3, 100)),
        (key("b"), rec(9, 300)),
        (key("c"), rec(1, 50)),
    ]);
    assert!(merge(&mut into, from));
    assert_eq!(into[&key("a")], rec(10, 200));
    assert_eq!(into[&key("b")], rec(9, 300));
    assert_eq!(into[&key("c")], rec(1, 50));
    assert!(!merge(&mut into, HashMap::from([(key("a"), rec(10, 200))])));
}

#[test]
fn progress_store_round_trips() {
    let dir = tempdir().unwrap();
    let store = ProgressStore::new(dir.path().join("nested/progress.json"));
    assert!(store.load().is_empty());
    let records = HashMap::from([(key("/books/a.pdf"), rec(7, 123))]);
    store.save(&records).unwrap();
    assert_eq!(store.load(), records);
}

#[test]
fn progress_survives_zathura_truncating_its_history() {
    let dir = tempdir().unwrap();
    let plain = dir.path().join("history");
    let sqlite = dir.path().join("bookmarks.sqlite");
    let own = dir.path().join("own/progress.json");
    std::fs::write(&plain, HISTORY).unwrap();

    let first = ReadingHistory::new(plain.clone(), sqlite.clone(), own.clone()).load();
    assert_eq!(first[&key("/home/me/books/one.pdf")].page, 19);

    // zathura killed mid-write: history left empty.
    std::fs::write(&plain, "").unwrap();
    let second = ReadingHistory::new(plain, sqlite, own).load();
    assert_eq!(second, first);
}

#[test]
fn combines_plain_and_sqlite_sources() {
    let dir = tempdir().unwrap();
    let plain = dir.path().join("history");
    let sqlite = dir.path().join("bookmarks.sqlite");
    std::fs::write(&plain, "[/books/a.pdf]\npage=2\ntime=100\n").unwrap();
    // 1970-01-01 00:03:20 UTC == 200
    write_sqlite(
        &sqlite,
        &[
            ("/books/a.pdf", 8, "1970-01-01 00:03:20"),
            ("/books/b.pdf", 4, "1970-01-01 00:01:40"),
        ],
    );
    let records = ReadingHistory::new(plain, sqlite, dir.path().join("p.json")).load();
    assert_eq!(records[&key("/books/a.pdf")], rec(8, 200));
    assert_eq!(records[&key("/books/b.pdf")], rec(4, 100));
}
