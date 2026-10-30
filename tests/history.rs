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
