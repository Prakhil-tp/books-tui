"""Draws the minimal two-line reading list."""
import curses

from .domain import Insight
from .layout import MIN_HEIGHT, MIN_WIDTH, clamp_offset, geometry
from .presentation import Progress, format_relative, format_size, prettify

HINTS = (
    ("j/k", "move"),
    ("gg/G", "ends"),
    ("^d/^u", "half"),
    ("Enter/l", "open"),
    ("q", "quit"),
)

BAR_WIDTH = 6
PROGRESS_BLOCK = BAR_WIDTH + 5

LOGO = (
    " ██▄ █▀█ █▀█ █░█ ▄▀▀",
    " █▄█ █▄█ █▄█ █▀▄ ▄██",
)
LOGO_TOP = 1


class Renderer:
    def __init__(self, screen, palette):
        self._screen = screen
        self._palette = palette

    def render(self, books, cursor, offset, status, insights) -> int:
        width, height = self._screen.size()
        self._screen.clear()
        if width < MIN_WIDTH or height < MIN_HEIGHT:
            self._screen.put(0, 0, " terminal too small ", curses.A_REVERSE)
            self._screen.refresh()
            return offset
        geo = geometry(width, height, len(books))
        offset = clamp_offset(offset, cursor, geo.visible, len(books))
        self._header(geo, books)
        self._rows(geo, books, cursor, offset, insights)
        self._scrollbar(geo, len(books), offset)
        self._footer(geo, status)
        self._screen.refresh()
        return offset

    def _header(self, geo, books) -> None:
        palette = self._palette
        for row, line in enumerate(LOGO, start=LOGO_TOP):
            self._screen.put(row, 2, line, palette.amber() if row == LOGO_TOP else palette.amber_soft())
        self._put_summary(geo, books, palette.dim(), LOGO_TOP + len(LOGO) - 1)
        self._screen.put(LOGO_TOP + len(LOGO), 0, "─" * geo.width, palette.dim())

    def _put_summary(self, geo, books, attr, row) -> None:
        summary = f"{len(books)} volume{'s' if len(books) != 1 else ''} · {format_size(_total(books))}"
        if geo.width < 2 + len(LOGO[0]) + 4 + len(summary):
            return
        self._screen.put(row, geo.width - 2 - len(summary), summary, attr)

    def _rows(self, geo, books, cursor, offset, insights) -> None:
        if not books:
            message = "no PDFs found"
            y = geo.list_top + (geo.visible // 2) * geo.list_row_height
            self._screen.put(y, max(2, (geo.width - len(message)) // 2), message, self._palette.dim())
            return
        last = min(offset + geo.visible, len(books))
        for row, index in enumerate(range(offset, last)):
            y = geo.list_top + row * geo.list_row_height
            self._row(geo, books[index], insights.get(index, Insight()), index == cursor, y)

    def _row(self, geo, book, insight, selected, y) -> None:
        palette = self._palette
        if selected:
            self._screen.put(y, 0, "▌", palette.amber())
        title_attr = palette.amber() if selected else palette.text()
        limit = max(0, geo.content_right - PROGRESS_BLOCK - 3)
        self._screen.put(y, 2, prettify(book.path), title_attr, limit)
        self._progress(geo, insight, selected, y)
        self._meta(geo, insight, y + 1)

    def _progress(self, geo, insight, selected, y) -> None:
        palette = self._palette
        record = insight.record
        pages = insight.metadata.pages
        if record.page and pages:
            progress = Progress(record.page, pages)
            filled, empty = progress.bar(BAR_WIDTH)
            x = geo.content_right - PROGRESS_BLOCK + 1
            self._screen.put(y, x, "█" * filled, palette.amber() if selected else palette.dim())
            self._screen.put(y, x + filled, "░" * empty, palette.dim())
            self._screen.put(
                y,
                x + BAR_WIDTH + 1,
                f"{progress.percent():>3}%",
                palette.amber() if selected else palette.dim(),
            )
            return
        label = "unread" if not record.page else f"p.{record.page}"
        self._screen.put(y, geo.content_right - len(label) + 1, label, palette.dim())

    def _meta(self, geo, insight, y) -> None:
        metadata = insight.metadata
        record = insight.record
        parts = [metadata.author or "unknown author"]
        if metadata.pages:
            parts.append(f"{metadata.pages} pages")
        if metadata.year:
            parts.append(str(metadata.year))
        if record.page:
            parts.append(f"page {record.page}")
        if record.read_at:
            parts.append(f"read {format_relative(record.read_at)}")
        self._screen.put(
            y, 2, " · ".join(parts), self._palette.faint(), max(0, geo.content_right - 2)
        )

    def _scrollbar(self, geo, count, offset) -> None:
        if not geo.has_scrollbar:
            return
        for y in range(geo.list_top, geo.footer_y):
            self._screen.put(y, geo.scroll_x, "│", self._palette.dim())
        track = geo.footer_y - geo.list_top
        thumb = max(1, round(track * geo.visible / count))
        span = max(1, track - thumb)
        last = max(1, count - geo.visible)
        top = geo.list_top + int(offset / last * span)
        for step in range(thumb):
            self._screen.put(top + step, geo.scroll_x, "█", self._palette.amber())

    def _footer(self, geo, status) -> None:
        palette = self._palette
        if status:
            self._screen.put(geo.footer_y, 2, status, palette.amber())
            return
        x = 2
        for key, label in HINTS:
            if x + len(key) + len(label) + 2 >= geo.width - 2:
                return
            self._screen.put(geo.footer_y, x, key, palette.amber())
            x += len(key)
            self._screen.put(geo.footer_y, x, " " + label, palette.dim())
            x += len(label) + 3


def _total(books) -> int:
    return sum(book.size for book in books)
