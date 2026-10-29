"""Launches the reader and resumes the interface afterwards."""
import subprocess

from .providers import CommandRunner


class ZathuraOpener:
    def __init__(self, screen, runner: CommandRunner):
        self._screen = screen
        self._runner = runner

    def open(self, path) -> str | None:
        self._screen.suspend()
        try:
            self._runner.run(["zathura", str(path)])
        except FileNotFoundError:
            return "zathura not found in PATH"
        except (OSError, subprocess.SubprocessError):
            return "could not launch zathura"
        finally:
            self._screen.resume()
        return None
