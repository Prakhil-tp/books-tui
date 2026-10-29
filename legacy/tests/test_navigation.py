import unittest

from books_tui.navigation import Navigation


class NavigationTest(unittest.TestCase):
    def setUp(self):
        self.nav = Navigation()

    def test_moves_down_and_up_within_bounds(self):
        self.nav.down(10)
        self.assertEqual(self.nav.cursor, 1)
        self.nav.up()
        self.nav.up()
        self.assertEqual(self.nav.cursor, 0)

    def test_stops_at_last_book(self):
        self.nav.down(10)
        self.nav.bottom(10)
        self.assertEqual(self.nav.cursor, 9)

    def test_half_page_moves_by_half_the_visible_rows(self):
        self.nav.half(10, forward=True)
        self.assertEqual(self.nav.cursor, 5)
        self.nav.half(10, forward=False)
        self.assertEqual(self.nav.cursor, 0)

    def test_double_g_jumps_to_top(self):
        self.nav.bottom(10)
        self.assertFalse(self.nav.press_g())
        self.assertTrue(self.nav.press_g())
        self.assertEqual(self.nav.cursor, 0)

    def test_unrelated_key_cancels_pending_g(self):
        self.nav.press_g()
        self.nav.cancel()
        self.assertFalse(self.nav.press_g())

    def test_offset_follows_cursor(self):
        self.nav.bottom(30)
        self.assertEqual(self.nav.offset(10), 20)
        self.nav.up()
        self.assertEqual(self.nav.offset(10), 19)


if __name__ == "__main__":
    unittest.main()
