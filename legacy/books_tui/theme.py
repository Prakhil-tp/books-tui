"""Terminal colour palette."""
import curses

PAIR_AMBER = 1
PAIR_DIM = 2
PAIR_AMBER_SOFT = 3
AMBER_256 = 179
AMBER_SOFT_256 = 137
DIM_256 = 244


class Palette:
    def __init__(self, colored: bool):
        self._colored = colored

    def amber(self) -> int:
        if self._colored:
            return curses.color_pair(PAIR_AMBER) | curses.A_BOLD
        return curses.A_BOLD

    def amber_soft(self) -> int:
        if self._colored:
            return curses.color_pair(PAIR_AMBER_SOFT) | curses.A_BOLD
        return curses.A_BOLD

    def dim(self) -> int:
        if self._colored:
            return curses.color_pair(PAIR_DIM)
        return curses.A_DIM

    def faint(self) -> int:
        return self.dim() | curses.A_DIM

    def text(self) -> int:
        return 0


def enable_colors() -> bool:
    if not curses.has_colors():
        return False
    curses.start_color()
    background = _background()
    curses.init_pair(PAIR_AMBER, _amber(), background)
    curses.init_pair(PAIR_AMBER_SOFT, _amber_soft(), background)
    curses.init_pair(PAIR_DIM, _dim(), background)
    return True


def _background() -> int:
    try:
        curses.use_default_colors()
        return -1
    except curses.error:
        return curses.COLOR_BLACK


def _amber() -> int:
    return AMBER_256 if curses.COLORS >= 256 else curses.COLOR_YELLOW


def _amber_soft() -> int:
    return AMBER_SOFT_256 if curses.COLORS >= 256 else curses.COLOR_YELLOW


def _dim() -> int:
    return DIM_256 if curses.COLORS >= 256 else curses.COLOR_WHITE
