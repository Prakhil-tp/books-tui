import curses
import time
import unittest
from pathlib import Path

from books_tui.domain import Book, BookMetadata, Insight, ReadingRecord
from books_tui.render import LOGO, Renderer
from books_tui.theme import Palette


class FakeScreen:
    def __init__(self, width, height):
        self.width = width
        self.height = height
        self.writes = {}
        self.attrs = {}

    def size(self):
        return self.width, self.height

    def clear(self):
        self.writes.clear()

    def put(self, y, x, text, attr=0, limit=None):
        self.writes[(y, x)] = text if limit is None else text[:limit]
        self.attrs[(y, x)] = attr

    def refresh(self):
        pass

    def text(self):
        return " ".join(self.writes.values())


def books(count):
    return [Book(Path(f"/books/book-{index}.pdf"), 1024) for index in range(count)]


def insight(author="Martin Kleppmann", pages=613, year=2017, page=19):
    return Insight(
        metadata=BookMetadata(author=author, pages=pages, year=year),
        record=ReadingRecord(page=page, read_at=int(time.time()) - 600),
    )


class RendererTest(unittest.TestCase):
    def render(self, width, height, count=3, cursor=0, insights=None, status=""):
        screen = FakeScreen(width, height)
        renderer = Renderer(screen, Palette(colored=False))
        offset = renderer.render(books(count), cursor, 0, status, insights or {})
        return screen, offset

    def test_draws_capital_wordmark_and_rule(self):
        screen, _ = self.render(90, 26)
        text = screen.text()
        self.assertIn(LOGO[0], text)
        self.assertIn(LOGO[1], text)
        self.assertIn("3 volumes", text)
        self.assertIn("─" * 90, text)

    def test_leaves_a_blank_top_margin(self):
        screen, _ = self.render(90, 26)
        self.assertGreaterEqual(min(y for y, _ in screen.writes), 1)

    def test_drops_summary_when_it_would_collide(self):
        screen, _ = self.render(40, 26)
        text = screen.text()
        self.assertIn(LOGO[0], text)
        self.assertNotIn("volumes", text)

    def test_meta_line_is_fainter_than_titles(self):
        screen, _ = self.render(90, 26)
        self.assertTrue(screen.attrs[(5, 2)] & curses.A_DIM)

    def test_marks_the_selected_row(self):
        screen, _ = self.render(90, 26, cursor=1)
        self.assertIn("▌", screen.text())

    def test_shows_insights_inline(self):
        screen, _ = self.render(90, 26, insights={0: insight()})
        text = screen.text()
        self.assertIn("Martin Kleppmann", text)
        self.assertIn("613 pages", text)
        self.assertIn("2017", text)
        self.assertIn("page 19", text)
        self.assertIn("read ", text)
        self.assertIn("3%", text)

    def test_shows_unread_label(self):
        screen, _ = self.render(90, 26, insights={0: Insight()})
        self.assertIn("unread", screen.text())
        self.assertIn("unknown author", screen.text())

    def test_renders_empty_library_message(self):
        screen = FakeScreen(90, 20)
        Renderer(screen, Palette(colored=False)).render([], 0, 0, "", {})
        self.assertIn("no PDFs found", screen.text())

    def test_scrollbar_appears_for_long_lists(self):
        screen, _ = self.render(90, 20, count=40, cursor=39)
        self.assertIn("█", screen.text())

    def test_keeps_offset_inside_the_list(self):
        _, offset = self.render(90, 20, count=40, cursor=39)
        self.assertEqual(offset, 35)

    def test_status_replaces_the_footer_hints(self):
        screen, _ = self.render(90, 26, status="zathura not found in PATH")
        self.assertIn("zathura not found", screen.text())


if __name__ == "__main__":
    unittest.main()
