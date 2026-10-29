import unittest

from books_tui.layout import clamp_offset, geometry


class GeometryTest(unittest.TestCase):
    def test_header_leaves_room_for_books(self):
        geo = geometry(90, 26, 3)
        self.assertEqual(geo.list_top, 4)
        self.assertEqual(geo.list_row_height, 3)
        self.assertEqual(geo.visible, 7)
        self.assertEqual(geo.footer_y, 25)

    def test_small_terminal_still_shows_a_book(self):
        geo = geometry(60, 8, 3)
        self.assertEqual(geo.list_top, 4)
        self.assertEqual(geo.visible, 1)

    def test_scrollbar_reserves_content_space(self):
        scrolling = geometry(90, 26, 40)
        plain = geometry(90, 26, 3)
        self.assertTrue(scrolling.has_scrollbar)
        self.assertLess(scrolling.content_right, plain.content_right)
        self.assertEqual(scrolling.scroll_x, 88)


class ClampOffsetTest(unittest.TestCase):
    def test_keeps_cursor_visible_at_the_bottom(self):
        self.assertEqual(clamp_offset(0, 39, 6, 40), 34)

    def test_never_scrolls_past_the_last_page(self):
        self.assertEqual(clamp_offset(0, 39, 6, 10), 4)

    def test_follows_cursor_when_moving_up(self):
        self.assertEqual(clamp_offset(20, 15, 6, 40), 15)

    def test_stays_at_top_for_small_lists(self):
        self.assertEqual(clamp_offset(0, 2, 6, 3), 0)


if __name__ == "__main__":
    unittest.main()
