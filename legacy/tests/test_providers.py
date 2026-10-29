import tempfile
import unittest
from pathlib import Path
from subprocess import CompletedProcess

from books_tui.domain import BookMetadata, ReadingRecord
from books_tui.providers import (
    JsonStore,
    MetadataCache,
    PdfInfoReader,
    ZathuraHistory,
)

PDFINFO = """Title:          System Design Interview
Author:         Alex Xu
CreationDate:   Sat Dec 19 09:59:24 2020 +04
Pages:          269
Page size:      595.28 x 841.89 pts (A4)
"""

HISTORY = """[/home/me/books/one.pdf]
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
"""


class FakeRunner:
    def __init__(self, stdout="", returncode=0):
        self.stdout = stdout
        self.returncode = returncode
        self.calls = []

    def run(self, argv):
        self.calls.append(argv)
        return CompletedProcess(argv, self.returncode, self.stdout, "")


class PdfInfoReaderTest(unittest.TestCase):
    def test_reads_author_pages_and_year(self):
        meta = PdfInfoReader(FakeRunner(PDFINFO)).read(Path("/tmp/book.pdf"))
        self.assertEqual(meta, BookMetadata(author="Alex Xu", pages=269, year=2020))

    def test_failed_command_yields_empty_metadata(self):
        meta = PdfInfoReader(FakeRunner(PDFINFO, returncode=1)).read(Path("/tmp/x.pdf"))
        self.assertEqual(meta, BookMetadata())

    def test_broken_output_is_tolerated(self):
        meta = PdfInfoReader(FakeRunner("Pages: many\n")).read(Path("/tmp/x.pdf"))
        self.assertEqual(meta, BookMetadata())


class CountingReader:
    def __init__(self, metadata):
        self.metadata = metadata
        self.calls = 0

    def read(self, path):
        self.calls += 1
        return self.metadata


class MetadataCacheTest(unittest.TestCase):
    def setUp(self):
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.directory = Path(self._temp.name)
        self.book = self.directory / "book.pdf"
        self.book.write_bytes(b"pdf")
        store = JsonStore(self.directory / "cache.json")
        self.reader = CountingReader(BookMetadata(author="Alex Xu", pages=269, year=2020))
        self.cache = MetadataCache(store, self.reader)

    def test_reads_and_then_reuses_cached_value(self):
        first = self.cache.metadata_for(self.book)
        second = self.cache.metadata_for(self.book)
        self.assertEqual(first, second)
        self.assertEqual(self.reader.calls, 1)

    def test_refreshes_when_file_changes(self):
        self.cache.metadata_for(self.book)
        self.book.write_bytes(b"pdf version two")
        self.cache.metadata_for(self.book)
        self.assertEqual(self.reader.calls, 2)

    def test_survives_corrupt_cache_file(self):
        (self.directory / "cache.json").write_text("not json")
        metadata = MetadataCache(
            JsonStore(self.directory / "cache.json"), self.reader
        ).metadata_for(self.book)
        self.assertEqual(metadata.author, "Alex Xu")


class ZathuraHistoryTest(unittest.TestCase):
    def setUp(self):
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.path = Path(self._temp.name) / "history"
        self.path.write_text(HISTORY)

    def test_parses_page_and_time_by_path(self):
        records = ZathuraHistory(self.path).records()
        self.assertEqual(
            records["/home/me/books/one.pdf"], ReadingRecord(page=19, read_at=1793193751)
        )
        self.assertEqual(
            records["/home/me/books/two.pdf"], ReadingRecord(page=3, read_at=1793100000)
        )

    def test_missing_history_yields_empty_mapping(self):
        self.assertEqual(ZathuraHistory(self.path.parent / "nope").records(), {})


if __name__ == "__main__":
    unittest.main()
