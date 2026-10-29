"""Pure presentation helpers: labels, sizes, relative dates, progress."""
import re
from datetime import datetime
from pathlib import Path


def prettify(path: Path) -> str:
    text = re.sub(r"[_.]+", " ", path.stem).replace("-", " ")
    title = re.split(r"\s+by\s+", text, maxsplit=1, flags=re.IGNORECASE)[0]
    return " ".join(word[:1].upper() + word[1:] for word in title.split())


def format_size(num: float) -> str:
    for unit in ("B", "K", "M", "G"):
        if num < 1024 or unit == "G":
            return f"{num:.0f}{unit}" if unit == "B" else f"{num:.1f}{unit}"
        num /= 1024
    return f"{num:.1f}G"


def format_relative(timestamp: float, now: datetime | None = None) -> str:
    moment = datetime.fromtimestamp(timestamp)
    seconds = ((now or datetime.now()) - moment).total_seconds()
    if seconds < 60:
        return "just now"
    if seconds < 3600:
        return f"{int(seconds // 60)} min ago"
    if seconds < 86400:
        return f"{int(seconds // 3600)} h ago"
    return _older(moment, now or datetime.now())


def _older(moment: datetime, now: datetime) -> str:
    days = (now.date() - moment.date()).days
    if days <= 1:
        return "yesterday"
    if days < 7:
        return f"{days} days ago"
    if days < 30:
        weeks = days // 7
        return f"{weeks} week{'s' if weeks > 1 else ''} ago"
    return moment.strftime("%d %b %Y")


class Progress:
    def __init__(self, page: int, pages: int):
        self._page = page
        self._pages = pages

    def percent(self) -> int:
        if not self._page or not self._pages:
            return 0
        return max(0, min(100, round(self._page / self._pages * 100)))

    def bar(self, width: int) -> tuple[int, int]:
        filled = max(0, min(width, round(width * self.percent() / 100)))
        return filled, width - filled
