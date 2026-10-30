use books_tui::library::BookPath;
use books_tui::metadata::{BookMetadata, JsonStore, MetadataCache, MetadataReader, PdfInfoReader};
use books_tui::ports::CommandRunner;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{ExitStatus, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::tempdir;

const PDFINFO: &str = "Title:          System Design Interview
Author:         Alex Xu
CreationDate:   Sat Dec 19 09:59:24 2020 +04
Pages:          269
Page size:      595.28 x 841.89 pts (A4)
";

struct FakeRunner {
    output: String,
    success: bool,
}

impl CommandRunner for FakeRunner {
    fn run(&self, _argv: &[&str]) -> anyhow::Result<Output> {
        Ok(Output {
            status: ExitStatus::from_raw(if self.success { 0 } else { 1 }),
            stdout: self.output.as_bytes().to_vec(),
            stderr: Vec::new(),
        })
    }
    fn run_interactive(&self, _argv: &[&str]) -> anyhow::Result<ExitStatus> {
        Ok(ExitStatus::from_raw(0))
    }
}

fn book_path(path: &Path) -> BookPath {
    BookPath::new(path.to_path_buf())
}

#[test]
fn reads_author_pages_and_year() {
    let reader = PdfInfoReader::new(FakeRunner {
        output: PDFINFO.to_string(),
        success: true,
    });
    let metadata = reader.read(&book_path(Path::new("/tmp/book.pdf")));
    assert_eq!(
        metadata,
        BookMetadata {
            author: "Alex Xu".to_string(),
            pages: 269,
            year: 2020,
        }
    );
}

#[test]
fn failed_or_broken_output_yields_defaults() {
    let failed = PdfInfoReader::new(FakeRunner {
        output: PDFINFO.to_string(),
        success: false,
    });
    assert_eq!(
        failed.read(&book_path(Path::new("/tmp/book.pdf"))),
        BookMetadata::default()
    );
    let broken = PdfInfoReader::new(FakeRunner {
        output: "Pages: many\n".to_string(),
        success: true,
    });
    assert_eq!(
        broken.read(&book_path(Path::new("/tmp/book.pdf"))),
        BookMetadata::default()
    );
}

struct CountingReader {
    calls: Arc<AtomicUsize>,
    metadata: BookMetadata,
}

impl MetadataReader for CountingReader {
    fn read(&self, _path: &BookPath) -> BookMetadata {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.metadata.clone()
    }
}

#[test]
fn caches_by_stamp_and_refreshes_when_the_file_changes() {
    let temp = tempdir().unwrap();
    let book = temp.path().join("book.pdf");
    std::fs::write(&book, b"pdf").unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let reader = CountingReader {
        calls: calls.clone(),
        metadata: BookMetadata {
            author: "Alex Xu".to_string(),
            pages: 269,
            year: 2020,
        },
    };
    let store = JsonStore::new(temp.path().join("cache.json"));
    let mut cache = MetadataCache::new(reader, store);

    let first = cache.metadata_for(&book_path(&book));
    let second = cache.metadata_for(&book_path(&book));
    assert_eq!(first, second);
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    std::fs::write(&book, b"pdf version two").unwrap();
    cache.metadata_for(&book_path(&book));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn survives_a_corrupt_cache_file() {
    let temp = tempdir().unwrap();
    let book = temp.path().join("book.pdf");
    std::fs::write(&book, b"pdf").unwrap();
    let cache_path = temp.path().join("cache.json");
    std::fs::write(&cache_path, "not json").unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let reader = CountingReader {
        calls: calls.clone(),
        metadata: BookMetadata {
            author: "Alex Xu".to_string(),
            pages: 269,
            year: 2020,
        },
    };
    let mut cache = MetadataCache::new(reader, JsonStore::new(cache_path));
    assert_eq!(
        cache.metadata_for(&book_path(&book)).author,
        "Alex Xu".to_string()
    );
}

#[test]
fn reuses_the_on_disk_cache_on_a_new_instance() {
    let temp = tempdir().unwrap();
    let book = temp.path().join("book.pdf");
    std::fs::write(&book, b"pdf").unwrap();
    let cache_path = temp.path().join("cache.json");
    let calls = Arc::new(AtomicUsize::new(0));
    let metadata = BookMetadata {
        author: "Alex Xu".to_string(),
        pages: 269,
        year: 2020,
    };
    {
        let mut cache = MetadataCache::new(
            CountingReader {
                calls: calls.clone(),
                metadata: metadata.clone(),
            },
            JsonStore::new(cache_path.clone()),
        );
        cache.metadata_for(&book_path(&book));
    }
    let mut cache = MetadataCache::new(
        CountingReader {
            calls: calls.clone(),
            metadata: metadata.clone(),
        },
        JsonStore::new(cache_path),
    );
    assert_eq!(cache.metadata_for(&book_path(&book)), metadata);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
