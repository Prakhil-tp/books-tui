use books_tui::layout::{clamp_offset, geometry, rows_visible};

#[test]
fn header_leaves_room_for_books() {
    let geo = geometry(90, 26, 3);
    assert_eq!(geo.list_top, 4);
    assert_eq!(geo.visible, 7);
    assert_eq!(geo.footer_y, 25);
}

#[test]
fn small_terminal_still_shows_a_book() {
    let geo = geometry(60, 8, 3);
    assert_eq!(geo.list_top, 4);
    assert_eq!(geo.visible, 1);
    assert_eq!(rows_visible(8), 1);
}

#[test]
fn scrollbar_reserves_content_space() {
    let scrolling = geometry(90, 26, 40);
    let plain = geometry(90, 26, 3);
    assert!(scrolling.has_scrollbar);
    assert!(!plain.has_scrollbar);
    assert!(scrolling.content_right < plain.content_right);
    assert_eq!(scrolling.scroll_x, 88);
}

#[test]
fn clamps_cursor_at_the_bottom() {
    assert_eq!(clamp_offset(0, 39, 6, 40), 34);
}

#[test]
fn never_scrolls_past_the_last_page() {
    assert_eq!(clamp_offset(0, 39, 6, 10), 4);
}

#[test]
fn follows_cursor_when_moving_up() {
    assert_eq!(clamp_offset(20, 15, 6, 40), 15);
}

#[test]
fn stays_at_top_for_small_lists() {
    assert_eq!(clamp_offset(0, 2, 6, 3), 0);
}
