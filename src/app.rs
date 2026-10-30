//! Application state, events, effects, and the pure update function.

use crate::config::Config;
use crate::history::ReadingRecord;
use crate::layout::{clamp_offset, rows_visible};
use crate::library::{Book, BookPath};
use crate::metadata::BookMetadata;
use crate::presentation::prettify;
use crate::search;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use std::collections::{HashMap, HashSet};

const STATUS_TICKS: u8 = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilterState {
    Idle,
    Editing { query: String },
    Applied { query: String },
}

impl FilterState {
    pub fn query(&self) -> Option<&str> {
        match self {
            FilterState::Idle => None,
            FilterState::Editing { query } | FilterState::Applied { query } => Some(query),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub text: String,
    pub ticks_left: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Key(KeyEvent),
    Resize(u16, u16),
    FilesChanged,
    MetadataLoaded(Vec<(BookPath, BookMetadata)>),
    ReaderClosed,
    Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Scan,
    LoadHistory,
    LoadMetadata(Vec<BookPath>),
    Open(BookPath),
    Quit,
}

pub struct AppState {
    pub config: Config,
    pub books: Vec<Book>,
    pub visible: Vec<usize>,
    pub cursor: usize,
    pub offset: usize,
    pub filter: FilterState,
    pub status: Option<Status>,
    pub records: HashMap<BookPath, ReadingRecord>,
    pub metadata: HashMap<BookPath, BookMetadata>,
    pub requested: HashSet<BookPath>,
    pub pending_g: bool,
    pub should_quit: bool,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            books: Vec::new(),
            visible: Vec::new(),
            cursor: 0,
            offset: 0,
            filter: FilterState::Idle,
            status: None,
            records: HashMap::new(),
            metadata: HashMap::new(),
            requested: HashSet::new(),
            pending_g: false,
            should_quit: false,
        }
    }

    pub fn selected(&self) -> Option<&Book> {
        self.visible
            .get(self.cursor)
            .and_then(|index| self.books.get(*index))
    }

    pub fn replace_books(&mut self, books: Vec<Book>) {
        let selected = self.selected().map(|book| book.path.clone());
        self.books = books;
        self.rebuild_visible();
        if let Some(path) = selected {
            if let Some(position) = self
                .visible
                .iter()
                .position(|index| self.books[*index].path == path)
            {
                self.cursor = position;
            }
        }
        self.clamp_cursor();
        self.offset = self.offset.min(self.visible.len().saturating_sub(1));
    }

    /// Replaces the library and re-clamps the scroll offset for `rows` rows.
    pub fn refresh_books(&mut self, books: Vec<Book>, rows: usize) {
        self.replace_books(books);
        self.scroll_to_cursor(rows);
    }

    pub fn rebuild_visible(&mut self) {
        self.visible = match self.filter.query() {
            Some(query) if !query.trim().is_empty() => {
                let items: Vec<(usize, String)> = self
                    .books
                    .iter()
                    .enumerate()
                    .map(|(index, book)| (index, prettify(book.path.as_path())))
                    .collect();
                search::rank(query.trim(), &items)
            }
            _ => (0..self.books.len()).collect(),
        };
    }

    pub fn clamp_cursor(&mut self) {
        let last = self.visible.len().saturating_sub(1);
        self.cursor = self.cursor.min(last);
    }

    pub fn scroll_to_cursor(&mut self, rows: usize) {
        self.offset = clamp_offset(self.offset, self.cursor, rows, self.visible.len());
    }

    pub fn set_status(&mut self, text: impl Into<String>) {
        self.status = Some(Status {
            text: text.into(),
            ticks_left: Some(STATUS_TICKS),
        });
    }

    pub fn set_persistent_status(&mut self, text: impl Into<String>) {
        self.status = Some(Status {
            text: text.into(),
            ticks_left: None,
        });
    }

    pub fn request_metadata(&mut self, rows: usize) -> Vec<BookPath> {
        let end = (self.offset + rows).min(self.visible.len());
        let start = self.offset.min(end);
        let mut requests = Vec::new();
        for &index in &self.visible[start..end] {
            let path = &self.books[index].path;
            if !self.metadata.contains_key(path) && self.requested.insert(path.clone()) {
                requests.push(path.clone());
            }
        }
        requests
    }

    fn move_down(&mut self) {
        self.cursor = (self.cursor + 1).min(self.visible.len().saturating_sub(1));
        self.pending_g = false;
    }

    fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
        self.pending_g = false;
    }

    fn bottom(&mut self) {
        self.cursor = self.visible.len().saturating_sub(1);
        self.pending_g = false;
    }

    fn half(&mut self, rows: usize, forward: bool) {
        let step = (rows / 2).max(1);
        self.cursor = if forward {
            (self.cursor + step).min(self.visible.len().saturating_sub(1))
        } else {
            self.cursor.saturating_sub(step)
        };
        self.pending_g = false;
    }

    fn expire_status(&mut self) {
        let Some(status) = &mut self.status else {
            return;
        };
        let Some(left) = status.ticks_left else {
            return;
        };
        if left <= 1 {
            self.status = None;
        } else {
            status.ticks_left = Some(left - 1);
        }
    }
}

pub fn update(state: &mut AppState, event: Event, area: Rect) -> Vec<Effect> {
    match event {
        Event::Key(key) => {
            if key.kind == KeyEventKind::Release {
                return Vec::new();
            }
            match state.filter {
                FilterState::Editing { .. } => handle_filter_key(state, key, area),
                _ => handle_normal_key(state, key, area),
            }
        }
        Event::Resize(_, _) => {
            state.clamp_cursor();
            state.scroll_to_cursor(rows_visible(area.height));
            Vec::new()
        }
        Event::FilesChanged => vec![Effect::Scan],
        Event::MetadataLoaded(entries) => {
            for (path, metadata) in entries {
                state.requested.remove(&path);
                state.metadata.insert(path, metadata);
            }
            Vec::new()
        }
        Event::ReaderClosed => vec![Effect::LoadHistory],
        Event::Tick => {
            state.expire_status();
            Vec::new()
        }
    }
}

fn handle_normal_key(state: &mut AppState, key: KeyEvent, area: Rect) -> Vec<Effect> {
    let rows = rows_visible(area.height);
    match key.code {
        KeyCode::Char('q') if key.modifiers.is_empty() => {
            state.should_quit = true;
            return vec![Effect::Quit];
        }
        KeyCode::Esc => {
            state.should_quit = true;
            return vec![Effect::Quit];
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.should_quit = true;
            return vec![Effect::Quit];
        }
        KeyCode::Char('j') | KeyCode::Down => state.move_down(),
        KeyCode::Char('k') | KeyCode::Up => state.move_up(),
        KeyCode::Char('G') => state.bottom(),
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.half(rows, true)
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.half(rows, false)
        }
        KeyCode::Char('g') => {
            if state.pending_g {
                state.cursor = 0;
                state.pending_g = false;
            } else {
                state.pending_g = true;
            }
        }
        KeyCode::Char('/') => {
            let query = state.filter.query().map(str::to_string).unwrap_or_default();
            state.filter = FilterState::Editing { query };
            state.pending_g = false;
        }
        KeyCode::Enter | KeyCode::Char('l') => {
            state.pending_g = false;
            if let Some(book) = state.selected() {
                return vec![Effect::Open(book.path.clone())];
            }
            return Vec::new();
        }
        _ => state.pending_g = false,
    }
    state.scroll_to_cursor(rows);
    Vec::new()
}

fn handle_filter_key(state: &mut AppState, key: KeyEvent, area: Rect) -> Vec<Effect> {
    let rows = rows_visible(area.height);
    match key.code {
        KeyCode::Esc => {
            state.filter = FilterState::Idle;
            state.rebuild_visible();
            state.cursor = 0;
            state.offset = 0;
            return Vec::new();
        }
        KeyCode::Enter => {
            let query = state.filter.query().unwrap_or_default().to_string();
            state.filter = if query.trim().is_empty() {
                FilterState::Idle
            } else {
                FilterState::Applied { query }
            };
            return Vec::new();
        }
        KeyCode::Backspace => {
            if let FilterState::Editing { query } = &mut state.filter {
                query.pop();
            }
            state.rebuild_visible();
            state.cursor = 0;
            state.offset = 0;
        }
        KeyCode::Up => state.move_up(),
        KeyCode::Down => state.move_down(),
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => state.move_up(),
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => state.move_down(),
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            if let FilterState::Editing { query } = &mut state.filter {
                query.push(character);
            }
            state.rebuild_visible();
            state.cursor = 0;
            state.offset = 0;
        }
        _ => {}
    }
    state.scroll_to_cursor(rows);
    let requests = state.request_metadata(rows);
    if requests.is_empty() {
        Vec::new()
    } else {
        vec![Effect::LoadMetadata(requests)]
    }
}
