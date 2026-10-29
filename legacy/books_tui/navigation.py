"""Vim-style cursor state, independent of the interface."""


class Navigation:
    def __init__(self):
        self.cursor = 0
        self._pending_g = False

    def down(self, count: int) -> None:
        self.cursor = min(self.cursor + 1, max(0, count - 1))
        self._pending_g = False

    def up(self) -> None:
        self.cursor = max(0, self.cursor - 1)
        self._pending_g = False

    def bottom(self, count: int) -> None:
        self.cursor = max(0, count - 1)
        self._pending_g = False

    def half(self, visible: int, forward: bool) -> None:
        step = max(1, visible // 2) * (1 if forward else -1)
        self.cursor = max(0, self.cursor + step)
        self._pending_g = False

    def press_g(self) -> bool:
        if self._pending_g:
            self.cursor = 0
            self._pending_g = False
            return True
        self._pending_g = True
        return False

    def cancel(self) -> None:
        self._pending_g = False

    def offset(self, visible: int) -> int:
        if self.cursor < visible:
            return 0
        return self.cursor - visible + 1
