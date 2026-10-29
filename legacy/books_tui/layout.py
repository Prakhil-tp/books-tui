"""Screen geometry; keeps the layout math out of the renderer."""
from dataclasses import dataclass

ROW_HEIGHT = 3
MIN_WIDTH = 34
MIN_HEIGHT = 8
HEADER_HEIGHT = 4


@dataclass(frozen=True)
class Geometry:
    width: int
    height: int
    list_top: int
    list_row_height: int
    visible: int
    scroll_x: int
    content_right: int
    has_scrollbar: bool
    footer_y: int


def geometry(width: int, height: int, book_count: int) -> Geometry:
    visible = max(1, (height - HEADER_HEIGHT - 1) // ROW_HEIGHT)
    has_scrollbar = book_count > visible
    return Geometry(
        width=width,
        height=height,
        list_top=HEADER_HEIGHT,
        list_row_height=ROW_HEIGHT,
        visible=visible,
        scroll_x=width - 2,
        content_right=width - 4 if has_scrollbar else width - 1,
        has_scrollbar=has_scrollbar,
        footer_y=height - 1,
    )


def clamp_offset(offset: int, cursor: int, visible: int, count: int) -> int:
    if count <= 0:
        return 0
    last = max(0, count - visible)
    offset = max(0, min(offset, last))
    if cursor < offset:
        return max(0, min(cursor, last))
    if cursor >= offset + visible:
        return min(last, cursor - visible + 1)
    return offset
