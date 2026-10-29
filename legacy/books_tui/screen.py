"""Curses adapter implementing the screen used by the renderer."""
import curses

from .theme import enable_colors


class CursesScreen:
    def __init__(self, stdscr):
        self._std = stdscr

    def init(self) -> bool:
        curses.curs_set(0)
        self._std.keypad(True)
        return enable_colors()

    def size(self) -> tuple[int, int]:
        height, width = self._std.getmaxyx()
        return width, height

    def clear(self) -> None:
        self._std.erase()

    def put(self, y: int, x: int, text: str, attr: int = 0, limit: int | None = None) -> None:
        width = self.size()[0]
        if y < 0 or x < 0 or x >= width or not text:
            return
        allowed = width - x if limit is None else min(limit, width - x)
        if allowed <= 0:
            return
        try:
            self._std.addnstr(y, x, text, allowed, attr)
        except curses.error:
            pass

    def refresh(self) -> None:
        self._std.refresh()

    def getkey(self) -> int:
        return self._std.getch()

    def suspend(self) -> None:
        curses.def_prog_mode()
        curses.endwin()

    def resume(self) -> None:
        curses.reset_prog_mode()
        self._std.keypad(True)
        curses.curs_set(0)
        self._std.clear()
        self._std.refresh()
