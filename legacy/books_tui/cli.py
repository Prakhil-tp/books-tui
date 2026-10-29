"""Command line entry point."""
import curses
import locale
import sys
from pathlib import Path

from .app import App, Services, Session, Ui
from .domain import Library
from .navigation import Navigation
from .opener import ZathuraOpener
from .providers import (
    JsonStore,
    MetadataCache,
    PdfInfoReader,
    SubprocessRunner,
    ZathuraHistory,
)
from .render import Renderer
from .screen import CursesScreen
from .theme import Palette

BOOKS_DIR = Path.home() / "books"
HISTORY_FILE = Path.home() / ".local" / "share" / "zathura" / "history"
CACHE_FILE = Path.home() / ".cache" / "books-tui" / "metadata.json"


def main(argv=None) -> int:
    locale.setlocale(locale.LC_ALL, "")
    arguments = sys.argv[1:] if argv is None else argv
    if "--list" in arguments:
        return _list_books()
    if not sys.stdin.isatty():
        print("books-tui requires an interactive terminal", file=sys.stderr)
        return 1
    curses.wrapper(_run)
    return 0


def _run(stdscr) -> None:
    screen = CursesScreen(stdscr)
    colored = screen.init()
    renderer = Renderer(screen, Palette(colored))
    App(_services(screen), Ui(screen, renderer, Navigation()), Session()).run()


def _services(screen) -> Services:
    return Services(
        library=Library(BOOKS_DIR),
        metadata=MetadataCache(
            JsonStore(CACHE_FILE), PdfInfoReader(SubprocessRunner(timeout=15))
        ),
        history=ZathuraHistory(HISTORY_FILE),
        opener=ZathuraOpener(screen, SubprocessRunner()),
    )


def _list_books() -> int:
    library = Library(BOOKS_DIR)
    if not library.exists():
        print(f"{BOOKS_DIR} does not exist", file=sys.stderr)
        return 1
    for book in library.books():
        print(book.path)
    return 0
