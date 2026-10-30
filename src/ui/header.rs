//! Header wordmark, summary, and rule.

use crate::app::AppState;
use crate::layout::Geometry;
use crate::presentation::format_size;
use crate::theme::Palette;
use crate::ui::{put, put_right};
use ratatui::Frame;

const LOGO: [&str; 3] = [" ┏┓ ┏━┓┏━┓╻┏ ┏━┓", " ┣┻┓┃ ┃┃ ┃┣┻┓┗━┓", " ┗━┛┗━┛┗━┛╹ ╹┗━┛"];

pub fn draw(frame: &mut Frame, state: &AppState, geo: &Geometry, palette: &Palette) {
    for (offset, line) in LOGO.iter().enumerate() {
        let style = if offset + 1 < LOGO.len() {
            palette.amber
        } else {
            palette.amber_soft
        };
        put(frame, 2, 1 + offset as u16, geo.width, line, style);
    }
    let total: u64 = state.books.iter().map(|book| book.size).sum();
    let summary = match state.filter.query() {
        Some(query) if !query.trim().is_empty() => {
            format!("{} of {} matches", state.visible.len(), state.books.len())
        }
        _ => format!(
            "{} volume{} · {}",
            state.books.len(),
            if state.books.len() == 1 { "" } else { "s" },
            format_size(total)
        ),
    };
    if geo.width as usize
        > 2 + LOGO[0].chars().count() + 4 + unicode_width::UnicodeWidthStr::width(summary.as_str())
    {
        put_right(frame, geo.width.saturating_sub(2), 1, &summary, palette.dim);
    }
    put(
        frame,
        0,
        geo.list_top - 1,
        geo.width,
        &"─".repeat(geo.width as usize),
        palette.dim,
    );
}
