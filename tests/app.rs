use books_tui::app::{update, AppState, Effect, Event, FilterState};
use books_tui::config::{BooksDir, Config, ReaderCommand};
use books_tui::library::{Book, BookPath};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use std::path::{Path, PathBuf};

fn state(titles: &[&str]) -> AppState {
    let books_dir = BooksDir::parse("/books", Path::new("/")).unwrap();
    let reader = ReaderCommand::parse("zathura").unwrap();
    let mut state = AppState::new(Config::new(books_dir, reader));
    let books = titles
        .iter()
        .map(|title| Book {
            path: BookPath::new(PathBuf::from(format!("/books/{title}.pdf"))),
            size: 100,
        })
        .collect();
    state.replace_books(books);
    state
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn ctrl(character: char) -> Event {
    Event::Key(KeyEvent::new(
        KeyCode::Char(character),
        KeyModifiers::CONTROL,
    ))
}

fn area() -> Rect {
    Rect::new(0, 0, 90, 26)
}

#[test]
fn j_moves_the_cursor_down_and_k_moves_up() {
    let mut state = state(&["a", "b", "c"]);
    update(&mut state, key(KeyCode::Char('j')), area());
    assert_eq!(state.cursor, 1);
    update(&mut state, key(KeyCode::Char('k')), area());
    assert_eq!(state.cursor, 0);
}

#[test]
fn cursor_is_clamped_to_the_library() {
    let mut state = state(&["a"]);
    update(&mut state, key(KeyCode::Char('j')), area());
    assert_eq!(state.cursor, 0);
    update(&mut state, key(KeyCode::Char('G')), area());
    assert_eq!(state.cursor, 0);
}

#[test]
fn double_g_returns_to_the_top() {
    let mut state = state(&["a", "b", "c"]);
    update(&mut state, key(KeyCode::Char('G')), area());
    assert_eq!(state.cursor, 2);
    update(&mut state, key(KeyCode::Char('g')), area());
    update(&mut state, key(KeyCode::Char('g')), area());
    assert_eq!(state.cursor, 0);
}

#[test]
fn half_page_moves_by_half_the_visible_rows() {
    let mut state = state(&["a", "b", "c", "d", "e", "f", "g", "h", "i"]);
    update(&mut state, ctrl('d'), area());
    assert_eq!(state.cursor, 3);
    update(&mut state, ctrl('u'), area());
    assert_eq!(state.cursor, 0);
}

#[test]
fn enter_opens_the_selected_book() {
    let mut state = state(&["a", "b"]);
    let effects = update(&mut state, key(KeyCode::Enter), area());
    assert!(matches!(
        effects.as_slice(),
        [Effect::Open(path)] if path.as_path() == Path::new("/books/a.pdf")
    ));
}

#[test]
fn quit_keys_set_the_flag() {
    for event in [key(KeyCode::Char('q')), key(KeyCode::Esc)] {
        let mut state = state(&["a"]);
        let effects = update(&mut state, event, area());
        assert!(state.should_quit);
        assert_eq!(effects, vec![Effect::Quit]);
    }
}

#[test]
fn slash_starts_filtering_and_reopening_keeps_the_query() {
    let mut state = state(&["a", "b"]);
    update(&mut state, key(KeyCode::Char('/')), area());
    assert_eq!(
        state.filter,
        FilterState::Editing {
            query: String::new()
        }
    );
    update(&mut state, key(KeyCode::Char('a')), area());
    update(&mut state, key(KeyCode::Enter), area());
    update(&mut state, key(KeyCode::Char('/')), area());
    assert_eq!(
        state.filter,
        FilterState::Editing {
            query: "a".to_string()
        }
    );
}

#[test]
fn typing_filters_and_escape_cancels() {
    let mut state = state(&["clean-code", "rust-book"]);
    update(&mut state, key(KeyCode::Char('/')), area());
    for character in "rust".chars() {
        update(&mut state, key(KeyCode::Char(character)), area());
    }
    assert_eq!(state.visible.len(), 1);
    assert_eq!(state.visible[0], 1);

    update(&mut state, key(KeyCode::Esc), area());
    assert_eq!(state.filter, FilterState::Idle);
    assert_eq!(state.visible.len(), 2);
}

#[test]
fn q_is_typed_while_filtering() {
    let mut state = state(&["q-book", "other"]);
    update(&mut state, key(KeyCode::Char('/')), area());
    update(&mut state, key(KeyCode::Char('q')), area());
    assert!(!state.should_quit);
    assert_eq!(
        state.filter,
        FilterState::Editing {
            query: "q".to_string()
        }
    );
}

#[test]
fn backspace_shortens_the_query() {
    let mut state = state(&["abc", "abd"]);
    update(&mut state, key(KeyCode::Char('/')), area());
    update(&mut state, key(KeyCode::Char('a')), area());
    update(&mut state, key(KeyCode::Char('b')), area());
    update(&mut state, key(KeyCode::Backspace), area());
    assert_eq!(
        state.filter,
        FilterState::Editing {
            query: "a".to_string()
        }
    );
}

#[test]
fn resize_reclamps_the_offset() {
    let mut state = state(&["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"]);
    update(&mut state, key(KeyCode::Char('G')), area());
    update(&mut state, Event::Resize(90, 8), Rect::new(0, 0, 90, 8));
    assert_eq!(state.offset, state.cursor);
}

#[test]
fn metadata_requests_only_cover_visible_uncached_rows() {
    let mut state = state(&["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"]);
    let requests = state.request_metadata(3);
    assert_eq!(requests.len(), 3);
    assert_eq!(state.request_metadata(3), Vec::new());
}

#[test]
fn metadata_load_updates_the_state() {
    let mut state = state(&["a"]);
    let path = state.books[0].path.clone();
    update(
        &mut state,
        Event::MetadataLoaded(vec![(
            path.clone(),
            books_tui::metadata::BookMetadata {
                author: "Author".to_string(),
                pages: 10,
                year: 2020,
            },
        )]),
        area(),
    );
    assert_eq!(state.metadata[&path].author, "Author");
}
