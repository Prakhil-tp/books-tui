"""Adapters that supply metadata and reading history."""
import json
import re
import subprocess
from pathlib import Path
from typing import Protocol

from .domain import BookMetadata, ReadingRecord


class CommandRunner(Protocol):
    def run(self, argv: list[str]) -> subprocess.CompletedProcess:
        ...


class SubprocessRunner:
    def __init__(self, timeout: float | None = None):
        self._timeout = timeout

    def run(self, argv: list[str]) -> subprocess.CompletedProcess:
        if self._timeout is None:
            return subprocess.run(argv)
        return subprocess.run(
            argv, capture_output=True, text=True, timeout=self._timeout
        )


class PdfInfoReader:
    def __init__(self, runner: CommandRunner):
        self._runner = runner

    def read(self, path: Path) -> BookMetadata:
        try:
            result = self._runner.run(["pdfinfo", str(path)])
        except (OSError, subprocess.SubprocessError):
            return BookMetadata()
        if result.returncode != 0:
            return BookMetadata()
        return self._parse(result.stdout)

    @staticmethod
    def _parse(text: str) -> BookMetadata:
        fields = _fields(text)
        return BookMetadata(
            author=fields.get("author", ""),
            pages=_as_int(fields.get("pages", "")),
            year=_year(fields.get("creationdate", "")),
        )


class JsonStore:
    def __init__(self, path: Path):
        self._path = path

    def read(self) -> dict:
        try:
            return json.loads(self._path.read_text())
        except (OSError, ValueError):
            return {}

    def write(self, data: dict) -> None:
        try:
            self._path.parent.mkdir(parents=True, exist_ok=True)
            temp = self._path.with_suffix(".tmp")
            temp.write_text(json.dumps(data))
            temp.replace(self._path)
        except OSError:
            pass


class MetadataCache:
    def __init__(self, store: JsonStore, reader: PdfInfoReader):
        self._store = store
        self._reader = reader
        self._memory = {}

    def metadata_for(self, path: Path) -> BookMetadata:
        stamp = _stamp(path)
        if stamp is None:
            return self._reader.read(path)
        key = str(path)
        cached = self._memory.get(key)
        if cached and cached.get("stamp") == stamp:
            return _decode(cached)
        data = self._store.read()
        entry = data.get(key)
        if entry and entry.get("stamp") == stamp:
            self._memory[key] = entry
            return _decode(entry)
        metadata = self._reader.read(path)
        entry = _encode(metadata, stamp)
        self._memory[key] = entry
        data[key] = entry
        self._store.write(data)
        return metadata


class ZathuraHistory:
    def __init__(self, path: Path):
        self._path = path

    def records(self) -> dict[str, ReadingRecord]:
        try:
            text = self._path.read_text(errors="replace")
        except OSError:
            return {}
        return _parse_history(text)


def _fields(text: str) -> dict:
    fields = {}
    for line in text.splitlines():
        key, separator, value = line.partition(":")
        if separator:
            fields[key.strip().lower()] = value.strip()
    return fields


def _as_int(value) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return 0


def _year(value: str) -> int:
    match = re.search(r"\b(?:19|20)\d{2}\b", value)
    return int(match.group(0)) if match else 0


def _stamp(path: Path):
    try:
        stat = path.stat()
    except OSError:
        return None
    return [stat.st_mtime, stat.st_size]


def _encode(metadata: BookMetadata, stamp: list) -> dict:
    return {
        "stamp": stamp,
        "author": metadata.author,
        "pages": metadata.pages,
        "year": metadata.year,
    }


def _decode(entry: dict) -> BookMetadata:
    return BookMetadata(
        author=str(entry.get("author", "")),
        pages=_as_int(entry.get("pages", 0)),
        year=_as_int(entry.get("year", 0)),
    )


def _parse_history(text: str) -> dict[str, ReadingRecord]:
    return {section: _record(options) for section, options in _sections(text)}


def _sections(text: str):
    section = None
    options = {}
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("[") and line.endswith("]"):
            if section is not None:
                yield section, options
            section, options = line[1:-1], {}
        elif section is not None and "=" in line:
            key, _, value = line.partition("=")
            options[key.strip()] = value.strip()
    if section is not None:
        yield section, options


def _record(options: dict) -> ReadingRecord:
    return ReadingRecord(
        page=_as_int(options.get("page", 0)),
        read_at=_as_int(options.get("time", 0)),
    )
