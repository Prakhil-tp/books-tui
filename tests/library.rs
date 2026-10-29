use books_tui::library::scan;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn scan_returns_sorted_pdfs_only() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("b-book.pdf"), b"x").unwrap();
    fs::write(dir.path().join("a-book.PDF"), b"x").unwrap();
    fs::write(dir.path().join("notes.txt"), b"x").unwrap();

    let books = scan(dir.path());
    let names: Vec<&str> = books
        .iter()
        .map(|book| book.path.as_path().file_name().unwrap().to_str().unwrap())
        .collect();

    assert_eq!(names, vec!["a-book.PDF", "b-book.pdf"]);
}

#[test]
fn scan_reports_file_size() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("book.pdf"), b"12345").unwrap();
    assert_eq!(scan(dir.path())[0].size, 5);
}

#[test]
fn missing_directory_yields_no_books() {
    assert!(scan(Path::new("/definitely/missing/books")).is_empty());
}
