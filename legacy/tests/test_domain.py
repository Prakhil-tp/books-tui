import tempfile
import unittest
from pathlib import Path

from books_tui.domain import Book, Library


class LibraryTest(unittest.TestCase):
    def setUp(self):
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.directory = Path(self._temp.name)

    def write(self, name, content=b"x"):
        path = self.directory / name
        path.write_bytes(content)
        return path

    def test_returns_sorted_pdfs_only(self):
        self.write("b-book.pdf")
        self.write("a-book.PDF")
        self.write("notes.txt")
        books = Library(self.directory).books()
        self.assertEqual([book.path.name for book in books], ["a-book.PDF", "b-book.pdf"])

    def test_reports_file_size(self):
        self.write("book.pdf", b"12345")
        self.assertEqual(Library(self.directory).books()[0].size, 5)

    def test_missing_directory_yields_no_books(self):
        self.assertEqual(Library(self.directory / "missing").books(), [])

    def test_books_compare_by_value(self):
        path = Path("/tmp/book.pdf")
        self.assertEqual(Book(path, 1), Book(path, 1))


if __name__ == "__main__":
    unittest.main()
