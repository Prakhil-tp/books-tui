"""Controller that wires input, navigation and rendering together."""
import curses
from dataclasses import dataclass, field

from .domain import Insight, ReadingRecord
from .layout import clamp_offset, geometry
from .navigation import Navigation

QUIT_KEYS = frozenset((ord("q"), 27, 3))
OPEN_KEYS = frozenset((curses.KEY_ENTER, 10, 13, ord("l")))


@dataclass(frozen=True)
class Services:
    library: object
    metadata: object
    history: object
    opener: object


@dataclass
class Ui:
    screen: object
    renderer: object
    navigation: Navigation


@dataclass
class Session:
    books: list = field(default_factory=list)
    records: dict = field(default_factory=dict)
    status: str = ""


class App:
    def __init__(self, services: Services, ui: Ui, session: Session | None = None):
        self._services = services
        self._ui = ui
        self._session = session or Session()

    def run(self) -> None:
        self._load()
        while True:
            self._draw()
            if self._handle(self._ui.screen.getkey()):
                return

    def _load(self) -> None:
        self._session.books = self._services.library.books()
        self._session.records = self._services.history.records()
        if not self._services.library.exists():
            self._session.status = f"{self._services.library.directory} does not exist"

    def _draw(self) -> None:
        width, height = self._ui.screen.size()
        geo = geometry(width, height, len(self._session.books))
        self._clamp_cursor()
        offset = clamp_offset(
            self._ui.navigation.offset(geo.visible),
            self._ui.navigation.cursor,
            geo.visible,
            len(self._session.books),
        )
        self._ui.renderer.render(
            self._session.books,
            self._ui.navigation.cursor,
            offset,
            self._session.status,
            self._insights(offset, geo.visible),
        )
        self._session.status = ""

    def _clamp_cursor(self) -> None:
        last = max(0, len(self._session.books) - 1)
        self._ui.navigation.cursor = min(self._ui.navigation.cursor, last)

    def _insights(self, offset: int, visible: int) -> dict:
        insights = {}
        end = min(offset + visible, len(self._session.books))
        for index in range(offset, end):
            book = self._session.books[index]
            insights[index] = Insight(
                metadata=self._services.metadata.metadata_for(book.path),
                record=self._session.records.get(str(book.path), ReadingRecord()),
            )
        return insights

    def _handle(self, key: int) -> bool:
        if key in QUIT_KEYS:
            return True
        if key == curses.KEY_RESIZE:
            return False
        if key in OPEN_KEYS:
            self._open_selected()
            return False
        self._navigate(key)
        return False

    def _navigate(self, key: int) -> None:
        count = len(self._session.books)
        commands = {
            ord("j"): lambda: self._ui.navigation.down(count),
            curses.KEY_DOWN: lambda: self._ui.navigation.down(count),
            ord("k"): self._ui.navigation.up,
            curses.KEY_UP: self._ui.navigation.up,
            ord("G"): lambda: self._ui.navigation.bottom(count),
            4: lambda: self._ui.navigation.half(self._visible(), True),
            21: lambda: self._ui.navigation.half(self._visible(), False),
        }
        command = commands.get(key)
        if command is not None:
            command()
        elif key == ord("g"):
            self._ui.navigation.press_g()
        else:
            self._ui.navigation.cancel()

    def _visible(self) -> int:
        width, height = self._ui.screen.size()
        return geometry(width, height, len(self._session.books)).visible

    def _open_selected(self) -> None:
        if not self._session.books:
            return
        book = self._session.books[self._ui.navigation.cursor]
        error = self._services.opener.open(book.path)
        self._session.records = self._services.history.records()
        if error:
            self._session.status = error
