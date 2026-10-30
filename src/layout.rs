//! Screen geometry and scroll math.

pub const ROW_HEIGHT: u16 = 3;
pub const HEADER_HEIGHT: u16 = 4;
pub const MIN_WIDTH: u16 = 34;
pub const MIN_HEIGHT: u16 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    pub width: u16,
    pub height: u16,
    pub list_top: u16,
    pub visible: usize,
    pub scroll_x: u16,
    pub content_right: u16,
    pub has_scrollbar: bool,
    pub footer_y: u16,
}

pub fn rows_visible(height: u16) -> usize {
    (height.saturating_sub(HEADER_HEIGHT + 1) / ROW_HEIGHT).max(1) as usize
}

pub fn geometry(width: u16, height: u16, count: usize) -> Geometry {
    let visible = rows_visible(height);
    let has_scrollbar = count > visible;
    Geometry {
        width,
        height,
        list_top: HEADER_HEIGHT,
        visible,
        scroll_x: width.saturating_sub(2),
        content_right: if has_scrollbar {
            width.saturating_sub(4)
        } else {
            width.saturating_sub(1)
        },
        has_scrollbar,
        footer_y: height.saturating_sub(1),
    }
}

pub fn clamp_offset(offset: usize, cursor: usize, visible: usize, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    let last = count.saturating_sub(visible);
    let offset = offset.min(last);
    if cursor < offset {
        return cursor.min(last);
    }
    if cursor >= offset + visible {
        return (cursor + 1).saturating_sub(visible).min(last);
    }
    offset
}
