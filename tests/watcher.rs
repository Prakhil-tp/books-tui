use books_tui::app::Event;
use books_tui::watcher::{is_pdf_path, is_relevant, start};
use notify::event::{CreateKind, EventKind};
use notify::Event as NotifyEvent;
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn recognizes_pdf_paths_case_insensitively() {
    assert!(is_pdf_path(Path::new("book.pdf")));
    assert!(is_pdf_path(Path::new("BOOK.PDF")));
    assert!(!is_pdf_path(Path::new("book.epub")));
}

#[test]
fn relevance_requires_a_pdf_path() {
    let pdf = NotifyEvent::new(EventKind::Create(CreateKind::File))
        .add_path(Path::new("/books/new.pdf").to_path_buf());
    let text = NotifyEvent::new(EventKind::Create(CreateKind::File))
        .add_path(Path::new("/books/notes.txt").to_path_buf());
    assert!(is_relevant(&pdf));
    assert!(!is_relevant(&text));
}

#[test]
fn reports_added_pdfs_within_five_seconds() {
    let directory = tempdir().unwrap();
    let (sender, receiver) = mpsc::channel();
    let _handle = start(directory.path().to_path_buf(), sender).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    fs::write(directory.path().join("new.pdf"), b"pdf").unwrap();
    assert_eq!(
        receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
        Event::FilesChanged
    );
}
