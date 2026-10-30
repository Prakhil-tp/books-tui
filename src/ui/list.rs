//! Book rows, progress, metadata, scrollbar, and empty state.

use crate::app::AppState;
use crate::layout::{Geometry, ROW_HEIGHT};
use crate::library::BookPath;
use crate::presentation::{format_relative, prettify, Progress};
use crate::theme::Palette;
use crate::ui::{put, put_right, put_truncated};
use chrono::Local;
use ratatui::Frame;

const BAR_WIDTH: u32 = 6;
const PROGRESS_BLOCK: u16 = BAR_WIDTH as u16 + 5;

pub fn draw(frame: &mut Frame, state: &AppState, geo: &Geometry, palette: &Palette) {
    if state.visible.is_empty() {
        let message = if state.books.is_empty() {
            format!(
                "no PDFs found in {}",
                state.config.books_dir.as_path().display()
            )
        } else {
            "no books match the filter".to_string()
        };
        let y = geo.list_top + (geo.visible as u16 / 2) * ROW_HEIGHT;
        let x = ((geo.width as usize).saturating_sub(message.chars().count()) / 2).max(2) as u16;
        put_truncated(
            frame,
            x,
            y,
            geo.width.saturating_sub(x),
            &message,
            palette.dim,
        );
        return;
    }
    for row in 0..geo.visible {
        let index = state.offset + row;
        let Some(&book_index) = state.visible.get(index) else {
            break;
        };
        let book = &state.books[book_index];
        let y = geo.list_top + row as u16 * ROW_HEIGHT;
        let selected = index == state.cursor;
        if selected {
            put(frame, 0, y, 1, "▌", palette.amber);
        }
        let title_style = if selected {
            palette.amber
        } else {
            palette.text
        };
        let limit = geo.content_right.saturating_sub(PROGRESS_BLOCK + 3);
        put_truncated(
            frame,
            2,
            y,
            limit,
            &prettify(book.path.as_path()),
            title_style,
        );
        progress(frame, state, geo, &book.path, selected, y, palette);
        meta(frame, state, geo, &book.path, y + 1, palette);
    }
    scrollbar(frame, state, geo, palette);
}

fn progress(
    frame: &mut Frame,
    state: &AppState,
    geo: &Geometry,
    path: &BookPath,
    selected: bool,
    y: u16,
    palette: &Palette,
) {
    let record = state.records.get(path);
    let pages = state
        .metadata
        .get(path)
        .map(|metadata| metadata.pages)
        .unwrap_or(0);
    let page = record.map(|record| record.page).unwrap_or(0);
    let style = if selected { palette.amber } else { palette.dim };
    if page > 0 && pages > 0 {
        let progress = Progress::new(page, pages);
        let (filled, empty) = progress.bar(BAR_WIDTH);
        let (filled, empty) = (filled as u16, empty as u16);
        let x = geo.content_right.saturating_sub(PROGRESS_BLOCK) + 1;
        put(frame, x, y, filled, &"█".repeat(filled as usize), style);
        put(
            frame,
            x + filled,
            y,
            empty,
            &"░".repeat(empty as usize),
            palette.dim,
        );
        put(
            frame,
            x + BAR_WIDTH as u16 + 1,
            y,
            PROGRESS_BLOCK,
            &format!("{:>3}%", progress.percent()),
            style,
        );
        return;
    }
    let label = if page == 0 {
        "unread".to_string()
    } else {
        format!("p.{page}")
    };
    put_right(frame, geo.content_right + 1, y, &label, palette.dim);
}

fn meta(
    frame: &mut Frame,
    state: &AppState,
    geo: &Geometry,
    path: &BookPath,
    y: u16,
    palette: &Palette,
) {
    let metadata = state.metadata.get(path);
    let record = state.records.get(path);
    let mut parts = vec![metadata
        .map(|metadata| metadata.author.clone())
        .filter(|author| !author.is_empty())
        .unwrap_or_else(|| "unknown author".to_string())];
    if let Some(metadata) = metadata {
        if metadata.pages > 0 {
            parts.push(format!("{} pages", metadata.pages));
        }
        if metadata.year > 0 {
            parts.push(metadata.year.to_string());
        }
    }
    if let Some(record) = record {
        if record.page > 0 {
            parts.push(format!("page {}", record.page));
        }
        if record.read_at > 0 {
            parts.push(format!(
                "read {}",
                format_relative(record.read_at, Local::now())
            ));
        }
    }
    put_truncated(
        frame,
        2,
        y,
        geo.content_right.saturating_sub(2),
        &parts.join(" · "),
        palette.faint,
    );
}

fn scrollbar(frame: &mut Frame, state: &AppState, geo: &Geometry, palette: &Palette) {
    if !geo.has_scrollbar {
        return;
    }
    for y in geo.list_top..geo.footer_y {
        put(frame, geo.scroll_x, y, 1, "│", palette.dim);
    }
    let track = geo.footer_y.saturating_sub(geo.list_top) as usize;
    if track == 0 {
        return;
    }
    let count = state.visible.len();
    let thumb = (track * geo.visible / count).max(1).min(track);
    let span = track.saturating_sub(thumb);
    let last = count.saturating_sub(geo.visible).max(1);
    let top = geo.list_top as usize + state.offset * span / last;
    for step in 0..thumb {
        put(
            frame,
            geo.scroll_x,
            (top + step) as u16,
            1,
            "█",
            palette.amber,
        );
    }
}
