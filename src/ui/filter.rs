//! Search bar rendered in the footer while editing.

use crate::app::AppState;
use crate::layout::Geometry;
use crate::theme::Palette;
use crate::ui::{put, put_right};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, state: &AppState, geo: &Geometry, palette: &Palette) {
    let Some(query) = state.filter.query() else {
        return;
    };
    let line = format!("/{query}█");
    put(
        frame,
        2,
        geo.footer_y,
        geo.width.saturating_sub(2),
        &line,
        palette.amber,
    );
    let matches = format!(
        "{} match{}",
        state.visible.len(),
        if state.visible.len() == 1 { "" } else { "es" }
    );
    put_right(
        frame,
        geo.width.saturating_sub(2),
        geo.footer_y,
        &matches,
        palette.dim,
    );
}
