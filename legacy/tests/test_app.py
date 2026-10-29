import unittest
from pathlib import Path

from books_tui.app import App, Services, Session, Ui
from books_tui.domain import Book, BookMetadata
from books_tui.navigation import Navigation


class FakeScreen:
    def __init__(self, keys, size=(90, 26)):
        self._keys = list(keys)
        self._size = size

    def size(self):
        return self._size

    def getkey(self):
        return self._keys.pop(0)


class FakeRenderer:
    def __init__(self):
        self.calls = []

    def render(self, books, cursor, offset, status, insights):
        self.calls.append((books, cursor, offset, status, insights))
        return offset


class FakeLibrary:
    def __init__(self, books):
        self._books = books
        self.directory = Path("/books")

    def books(self):
        return list(self._books)

    def exists(self):
        return True


class FakeMetadata:
    def metadata_for(self, path):
        return BookMetadata(author="Author", pages=10, year=2020)


class FakeHistory:
    def records(self):
        return {}


class FakeOpener:
    def __init__(self):
        self.opened = []

    def open(self, path):
        self.opened.append(path)
        return None


def build(keys, count=3, size=(90, 26)):
    books = [Book(Path(f"/books/b{index}.pdf"), 100) for index in range(count)]
    screen = FakeScreen(keys, size)
    renderer = FakeRenderer()
    opener = FakeOpener()
    services = Services(FakeLibrary(books), FakeMetadata(), FakeHistory(), opener)
    navigation = Navigation()
    app = App(services, Ui(screen, renderer, navigation), Session())
    return app, renderer, opener, navigation


class AppTest(unittest.TestCase):
    def test_j_moves_the_cursor_down(self):
        app, renderer, _, navigation = build([ord("j"), ord("q")])
        app.run()
        self.assertEqual(navigation.cursor, 1)
        self.assertEqual(renderer.calls[-1][1], 1)

    def test_cursor_is_clamped_to_the_library(self):
        app, _, _, navigation = build([ord("j"), ord("j"), ord("q")], count=1)
        app.run()
        self.assertEqual(navigation.cursor, 0)

    def test_double_g_returns_to_the_top(self):
        app, _, _, navigation = build([ord("G"), ord("g"), ord("g"), ord("q")])
        app.run()
        self.assertEqual(navigation.cursor, 0)

    def test_enter_opens_the_selected_book(self):
        app, _, opener, _ = build([10, ord("q")])
        app.run()
        self.assertEqual(opener.opened, [Path("/books/b0.pdf")])

    def test_insights_are_limited_to_visible_rows(self):
        app, renderer, _, _ = build([ord("q")], count=20, size=(90, 20))
        app.run()
        self.assertEqual(set(renderer.calls[0][4]), set(range(5)))

    def test_insights_follow_the_scroll_position(self):
        app, renderer, _, _ = build([ord("G"), ord("q")], count=20, size=(90, 20))
        app.run()
        self.assertEqual(set(renderer.calls[1][4]), set(range(15, 20)))

    def test_quit_key_ends_the_loop(self):
        app, renderer, _, _ = build([ord("q")])
        app.run()
        self.assertEqual(len(renderer.calls), 1)


if __name__ == "__main__":
    unittest.main()
