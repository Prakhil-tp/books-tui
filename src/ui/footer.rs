//! Footer hints and status messages.

use crate::app::AppState;
use crate::layout::Geometry;
use crate::theme::Palette;
use crate::ui::put;
use ratatui::Frame;

const HINTS: [(&str, &str); 6] = [
    ("j/k", "move"),
    ("gg/G", "ends"),
    ("^d/^u", "half"),
    ("/", "filter"),
    ("Enter/l", "open"),
    ("q", "quit"),
];

pub fn draw(frame: &mut Frame, state: &AppState, geo: &Geometry, palette: &Palette) {
    if let Some(status) = &state.status {
        put(
            frame,
            2,
            geo.footer_y,
            geo.width.saturating_sub(2),
            &status.text,
            palette.amber,
        );
        return;
    }
    let mut x = 2;
    for (key, label) in HINTS {
        let width = unicode_width::UnicodeWidthStr::width(key)
            + unicode_width::UnicodeWidthStr::width(label)
            + 2;
        if x as usize + width >= geo.width.saturating_sub(2) as usize {
            return;
        }
        put(frame, x, geo.footer_y, geo.width, key, palette.amber);
        x += unicode_width::UnicodeWidthStr::width(key) as u16;
        put(
            frame,
            x,
            geo.footer_y,
            geo.width,
            &format!(" {label}"),
            palette.dim,
        );
        x += unicode_width::UnicodeWidthStr::width(label) as u16 + 3;
    }
}
