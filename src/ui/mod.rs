//! UI rendering.

pub mod filter;
pub mod footer;
pub mod header;
pub mod list;

use crate::app::{AppState, FilterState};
use crate::layout::{geometry, MIN_HEIGHT, MIN_WIDTH};
use crate::presentation::truncate;
use crate::theme::Palette;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn draw(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let message = Paragraph::new(Line::from(" terminal too small "))
            .style(Style::default().add_modifier(Modifier::REVERSED));
        frame.render_widget(message, area);
        return;
    }
    let geo = geometry(area.width, area.height, state.visible.len());
    let palette = Palette::new();
    header::draw(frame, state, &geo, &palette);
    list::draw(frame, state, &geo, &palette);
    if matches!(state.filter, FilterState::Editing { .. }) {
        filter::draw(frame, state, &geo, &palette);
    } else {
        footer::draw(frame, state, &geo, &palette);
    }
}

pub fn put(frame: &mut Frame, x: u16, y: u16, width: u16, text: &str, style: Style) {
    if width == 0 {
        return;
    }
    let area = Rect::new(x, y, width, 1);
    frame.render_widget(Paragraph::new(text.to_string()).style(style), area);
}

pub fn put_right(frame: &mut Frame, right: u16, y: u16, text: &str, style: Style) {
    let width = unicode_width::UnicodeWidthStr::width(text) as u16;
    let x = right.saturating_sub(width);
    put(frame, x, y, width, text, style);
}

pub fn put_truncated(frame: &mut Frame, x: u16, y: u16, width: u16, text: &str, style: Style) {
    let truncated = truncate(text, width as usize);
    put(frame, x, y, width, &truncated, style);
}
