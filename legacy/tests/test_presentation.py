import unittest
from datetime import datetime
from pathlib import Path

from books_tui.presentation import Progress, format_relative, format_size, prettify


class PrettifyTest(unittest.TestCase):
    def test_strips_author_and_dashes(self):
        name = Path("designing-data-intensive-applications-by-martin-kleppmann.pdf")
        self.assertEqual(prettify(name), "Designing Data Intensive Applications")

    def test_replaces_underscores(self):
        self.assertEqual(prettify(Path("clean_code.pdf")), "Clean Code")

    def test_keeps_title_when_no_author(self):
        self.assertEqual(
            prettify(Path("system-design-interview.pdf")), "System Design Interview"
        )


class FormatSizeTest(unittest.TestCase):
    def test_zero_bytes(self):
        self.assertEqual(format_size(0), "0B")

    def test_below_one_kilobyte(self):
        self.assertEqual(format_size(1023), "1023B")

    def test_kilobytes(self):
        self.assertEqual(format_size(1024), "1.0K")

    def test_megabytes(self):
        self.assertEqual(format_size(24 * 1024 * 1024), "24.0M")


class FormatRelativeTest(unittest.TestCase):
    NOW = datetime(2026, 10, 28, 12, 0, 0)

    def relative(self, seconds):
        return format_relative(self.NOW.timestamp() - seconds, self.NOW)

    def test_under_a_minute(self):
        self.assertEqual(self.relative(30), "just now")

    def test_minutes(self):
        self.assertEqual(self.relative(5 * 60), "5 min ago")

    def test_hours(self):
        self.assertEqual(self.relative(3 * 3600), "3 h ago")

    def test_yesterday(self):
        self.assertEqual(self.relative(26 * 3600), "yesterday")

    def test_days(self):
        self.assertEqual(self.relative(3 * 86400), "3 days ago")

    def test_weeks(self):
        self.assertEqual(self.relative(10 * 86400), "1 week ago")

    def test_old_dates_use_calendar(self):
        self.assertEqual(self.relative(90 * 86400), "30 Jul 2026")


class ProgressTest(unittest.TestCase):
    def test_percent_rounds(self):
        self.assertEqual(Progress(19, 613).percent(), 3)

    def test_bar_splits_filled_and_empty(self):
        self.assertEqual(Progress(50, 100).bar(10), (5, 5))

    def test_bar_is_empty_without_pages(self):
        self.assertEqual(Progress(10, 0).bar(10), (0, 10))

    def test_percent_is_clamped(self):
        self.assertEqual(Progress(700, 613).percent(), 100)


if __name__ == "__main__":
    unittest.main()
