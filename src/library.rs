//! PDF library scanning.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BookPath(PathBuf);

impl BookPath {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Book {
    pub path: BookPath,
    pub size: u64,
}

pub fn scan(directory: &Path) -> Vec<Book> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut books: Vec<Book> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_pdf(path))
        .map(to_book)
        .collect();
    books.sort_by_key(|book| {
        book.path
            .as_path()
            .file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    });
    books
}

fn is_pdf(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
}

fn to_book(path: PathBuf) -> Book {
    let size = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
    Book {
        path: BookPath::new(path),
        size,
    }
}
