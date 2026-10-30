use books_tui::app::{AppState, FilterState};
use books_tui::config::{BooksDir, Config, ReaderCommand};
use books_tui::history::ReadingRecord;
use books_tui::library::{Book, BookPath};
use books_tui::metadata::BookMetadata;
use books_tui::ui;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::path::{Path, PathBuf};

fn state(titles: &[&str]) -> AppState {
    let books_dir = BooksDir::parse("/books", Path::new("/")).unwrap();
    let reader = ReaderCommand::parse("zathura").unwrap();
    let mut state = AppState::new(Config::new(books_dir, reader));
    let books = titles
        .iter()
        .map(|title| Book {
            path: BookPath::new(PathBuf::from(format!("/books/{title}.pdf"))),
            size: 1024,
        })
        .collect();
    state.replace_books(books);
    state
}

fn render(state: &AppState, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| ui::draw(frame, state)).unwrap();
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn draws_wordmark_summary_and_footer() {
    let state = state(&["a", "b", "c"]);
    let text = render(&state, 90, 26);
    assert!(text.contains("┏┓ ┏━┓┏━┓╻┏ ┏━┓"));
    assert!(text.contains("┗━┛┗━┛┗━┛╹ ╹┗━┛"));
    assert!(text.contains("3 volumes"));
    assert!(text.contains("3.0K"));
    assert!(text.contains("j/k"));
}

#[test]
fn marks_the_selected_row() {
    let state = state(&["a", "b", "c"]);
    assert!(render(&state, 90, 26).contains("▌"));
}

#[test]
fn shows_metadata_progress_and_history() {
    let mut state = state(&["book"]);
    let path = state.books[0].path.clone();
    state.metadata.insert(
        path.clone(),
        BookMetadata {
            author: "Martin Kleppmann".to_string(),
            pages: 613,
            year: 2017,
        },
    );
    state.records.insert(
        path,
        ReadingRecord {
            page: 19,
            read_at: 0,
        },
    );
    let text = render(&state, 90, 26);
    assert!(text.contains("Martin Kleppmann"));
    assert!(text.contains("613 pages"));
    assert!(text.contains("2017"));
    assert!(text.contains("page 19"));
    assert!(text.contains("3%"));
}

#[test]
fn shows_unread_for_books_without_progress() {
    let state = state(&["book"]);
    let text = render(&state, 90, 26);
    assert!(text.contains("unread"));
    assert!(text.contains("unknown author"));
}

#[test]
fn renders_the_empty_library_message() {
    let state = state(&[]);
    assert!(render(&state, 90, 20).contains("no PDFs found"));
}

#[test]
fn renders_the_too_small_message() {
    let state = state(&["a"]);
    assert!(render(&state, 20, 5).contains("terminal too small"));
}

fn column(text: &str, x: usize, rows: std::ops::Range<usize>) -> Vec<char> {
    text.lines()
        .enumerate()
        .filter(|(y, _)| rows.contains(y))
        .map(|(_, line)| line.chars().nth(x).unwrap())
        .collect()
}

#[test]
fn renders_a_scrollbar_for_long_lists() {
    let titles: Vec<String> = (0..40).map(|index| format!("book{index}")).collect();
    let refs: Vec<&str> = titles.iter().map(String::as_str).collect();
    let mut state = state(&refs);
    state.cursor = 39;
    state.offset = 35;
    let geo = books_tui::layout::geometry(90, 20, 40);
    let text = render(&state, 90, 20);
    let top = geo.list_top as usize;
    let cells = column(&text, geo.scroll_x as usize, top..19);
    assert!(cells.contains(&'█'));
    assert!(cells.contains(&'│'));
}

#[test]
fn short_lists_leave_the_scrollbar_column_blank() {
    let state = state(&["a", "b"]);
    let geo = books_tui::layout::geometry(90, 20, 2);
    let text = render(&state, 90, 20);
    // The first line of each row holds the right-aligned "unread" label; skip it.
    let x = geo.scroll_x as usize;
    let top = geo.list_top as usize;
    let mut cells = column(&text, x, top + 1..top + 3);
    cells.extend(column(&text, x, top + 4..19));
    assert!(cells.iter().all(|c| *c == ' '));
}

#[test]
fn summary_is_right_aligned_by_display_width() {
    let state = state(&["a", "b", "c"]);
    let text = render(&state, 90, 26);
    let line = text.lines().nth(1).unwrap();
    assert_eq!(line.chars().count(), 90);
    assert!(line.ends_with("3 volumes · 3.0K  "));
}

#[test]
fn monochrome_palette_keeps_modifiers_without_colors() {
    use ratatui::style::Modifier;
    let palette = books_tui::theme::Palette::with_color(false);
    for style in [
        palette.amber,
        palette.amber_soft,
        palette.dim,
        palette.faint,
        palette.text,
    ] {
        assert_eq!(style.fg, None);
        assert_eq!(style.bg, None);
    }
    assert!(palette.amber.add_modifier.contains(Modifier::BOLD));
    assert!(palette.amber_soft.add_modifier.contains(Modifier::BOLD));
    assert!(palette.dim.add_modifier.contains(Modifier::DIM));
    assert!(palette.faint.add_modifier.contains(Modifier::DIM));
    assert_eq!(palette.text, ratatui::style::Style::default());
    assert!(books_tui::theme::Palette::with_color(true)
        .amber
        .fg
        .is_some());
}

#[test]
fn status_replaces_the_footer_hints() {
    let mut state = state(&["a"]);
    state.set_status("zathura not found in PATH");
    let text = render(&state, 90, 26);
    assert!(text.contains("zathura not found in PATH"));
    assert!(!text.contains("j/k"));
}

#[test]
fn filter_mode_renders_a_search_bar() {
    let mut state = state(&["rust", "clean"]);
    state.filter = FilterState::Editing {
        query: "ru".to_string(),
    };
    state.rebuild_visible();
    let text = render(&state, 90, 26);
    assert!(text.contains("/ru"));
    assert!(text.contains("1 match"));
}

#[test]
fn normal_screen_snapshot() {
    let state = state(&[
        "designing-data-intensive-applications-by-martin-kleppmann",
        "clean-code",
    ]);
    insta::assert_snapshot!(render(&state, 90, 26));
}

fn numbered(count: usize) -> Vec<Book> {
    (0..count)
        .map(|n| Book {
            path: BookPath::new(PathBuf::from(format!("/books/book{n:02}.pdf"))),
            size: 10,
        })
        .collect()
}

#[test]
fn shrinking_the_library_while_scrolled_does_not_panic() {
    let mut state = state(&[]);
    state.replace_books(numbered(30));
    state.cursor = 29;
    state.offset = 24;
    state.replace_books(numbered(27));
    let text = render(&state, 80, 24);
    assert!(text.contains("Book 26") || text.contains("book26") || text.contains("Book26"));
}

#[test]
fn drawing_larger_than_the_last_update_does_not_panic() {
    let mut state = state(&[]);
    state.replace_books(numbered(30));
    state.cursor = 29;
    state.offset = 24;
    let text = render(&state, 80, 40);
    assert!(text.to_lowercase().contains("book29"));
}

#[test]
fn row_labels_keep_the_same_right_margin_as_the_summary() {
    let state = state(&["a", "b", "c"]);
    let text = render(&state, 90, 26);
    let top = books_tui::layout::geometry(90, 26, 3).list_top as usize;
    let row = text.lines().nth(top).unwrap();
    assert!(row.ends_with("unread  "), "row was {row:?}");
}
