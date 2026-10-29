"""Domain model for the book library."""
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Book:
    path: Path
    size: int


@dataclass(frozen=True)
class BookMetadata:
    author: str = ""
    pages: int = 0
    year: int = 0


@dataclass(frozen=True)
class ReadingRecord:
    page: int = 0
    read_at: int = 0


@dataclass(frozen=True)
class Insight:
    metadata: BookMetadata = BookMetadata()
    record: ReadingRecord = ReadingRecord()


class Library:
    def __init__(self, directory: Path):
        self._directory = directory

    @property
    def directory(self) -> Path:
        return self._directory

    def exists(self) -> bool:
        return self._directory.is_dir()

    def books(self) -> list[Book]:
        return [self._as_book(path) for path in self._pdf_paths()]

    def _pdf_paths(self) -> list[Path]:
        try:
            entries = list(self._directory.iterdir())
        except OSError:
            return []
        pdfs = [path for path in entries if self._is_pdf(path)]
        return sorted(pdfs, key=lambda path: path.name.lower())

    @staticmethod
    def _is_pdf(path: Path) -> bool:
        return path.is_file() and path.suffix.lower() == ".pdf"

    @staticmethod
    def _as_book(path: Path) -> Book:
        try:
            return Book(path, path.stat().st_size)
        except OSError:
            return Book(path, 0)
