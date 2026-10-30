use anyhow::{bail, Context, Result};
use books_tui::app::{update, AppState, Effect, Event, Status};
use books_tui::config::{BooksDir, Config, FileConfigStore, FirstRun, ReaderCommand, StdinPrompt};
use books_tui::history;
use books_tui::install::{PathReadiness, ReaderInstaller, SystemRunner};
use books_tui::layout::rows_visible;
use books_tui::library::{self, BookPath};
use books_tui::metadata::{JsonStore, MetadataCache, PdfInfoReader};
use books_tui::opener::{open_book, TerminalControl};
use books_tui::ports::ConfigStore;
use books_tui::ui;
use books_tui::watcher;
use clap::Parser;
use crossterm::cursor::{Hide, Show};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{event as terminal_event, execute};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use std::io::{stdout, IsTerminal, Stdout};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(
    name = "books-tui",
    version,
    about = "Browse and open PDFs in your terminal"
)]
struct Args {
    #[arg(long)]
    list: bool,
    #[arg(long)]
    books_dir: Option<String>,
    #[arg(long)]
    config: Option<PathBuf>,
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("books-tui: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    let home = dirs::home_dir().context("cannot determine home directory")?;
    let config_path = args
        .config
        .clone()
        .unwrap_or_else(|| default_config_path(&home));
    let store = FileConfigStore::new(config_path, home.clone());
    let override_dir = args
        .books_dir
        .as_deref()
        .map(|value| BooksDir::parse(value, &home))
        .transpose()?;
    let config = resolve_config(&args, &store, &home, override_dir)?;
    if args.list {
        return list_books(&config);
    }
    ensure_reader(&config)?;
    run_tui(config, home)
}

fn default_config_path(home: &Path) -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| home.join(".config"))
        .join("books-tui")
        .join("config.toml")
}

fn resolve_config(
    args: &Args,
    store: &FileConfigStore,
    home: &Path,
    override_dir: Option<BooksDir>,
) -> Result<Config> {
    let loaded = store.load()?;
    if args.list {
        let mut config = match loaded {
            Some(config) => config,
            None => Config::new(
                BooksDir::parse("~/books", home)?,
                ReaderCommand::parse("zathura")?,
            ),
        };
        if let Some(directory) = override_dir {
            config.books_dir = directory;
        }
        return Ok(config);
    }
    let mut config = match loaded {
        Some(config) => config,
        None => {
            if !std::io::stdin().is_terminal() {
                bail!("no configuration found; run books-tui interactively once");
            }
            let prompt = StdinPrompt::new();
            FirstRun::new(store, &prompt, home, "zathura").run()?
        }
    };
    if let Some(directory) = override_dir {
        config.books_dir = directory;
    }
    Ok(config)
}

fn list_books(config: &Config) -> Result<()> {
    let directory = config.books_dir.as_path();
    if !directory.is_dir() {
        bail!("{} does not exist", directory.display());
    }
    for book in library::scan(directory) {
        println!("{}", book.path.as_path().display());
    }
    Ok(())
}

fn path_available(binary: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|directory| directory.join(binary).is_file())
}

fn ensure_reader(config: &Config) -> Result<()> {
    let runner = SystemRunner::default();
    let prompt = StdinPrompt::new();
    let available = |binary: &str| path_available(binary);
    let mut out = stdout();
    ReaderInstaller {
        readiness: &PathReadiness,
        prompt: &prompt,
        runner: &runner,
        available: &available,
        reader: &config.reader,
        package: "zathura",
        out: &mut out,
    }
    .ensure()?;
    Ok(())
}

fn cache_path(home: &Path) -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| home.join(".cache"))
        .join("books-tui")
        .join("metadata.json")
}

fn resolve_history_path(home: &Path) -> PathBuf {
    let xdg = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
    let candidate = history::history_path(home, xdg.as_deref());
    if candidate.exists() {
        return candidate;
    }
    #[cfg(target_os = "macos")]
    {
        let mac = home.join("Library/Application Support/zathura/history");
        if mac.exists() {
            return mac;
        }
    }
    candidate
}

fn run_tui(config: Config, home: PathBuf) -> Result<()> {
    let mut terminal = TerminalSession::new()?;
    let books_dir = config.books_dir.as_path().to_path_buf();
    let history_path = resolve_history_path(&home);
    let mut state = AppState::new(config);
    state.replace_books(library::scan(&books_dir));
    state.records = history::load(&history_path);
    if !books_dir.is_dir() {
        state.set_persistent_status(missing_message(&books_dir));
    }

    let (events, receiver) = mpsc::channel();
    spawn_input(events.clone(), terminal.suspended.clone());
    let mut _watcher = watcher::start(books_dir.clone(), events.clone()).ok();
    let (metadata_sender, metadata_receiver) = mpsc::channel();
    let cache = MetadataCache::new(
        PdfInfoReader::new(SystemRunner::default()),
        JsonStore::new(cache_path(&home)),
    );
    spawn_metadata_worker(cache, metadata_receiver, events.clone());

    let rows = rows_visible(terminal.area().height);
    let requests = state.request_metadata(rows);
    if !requests.is_empty() {
        let _ = metadata_sender.send(requests);
    }

    let banner = missing_message(&books_dir);
    let mut dir_present = books_dir.is_dir();
    let mut last_dir_check = Instant::now();

    loop {
        let event = receiver
            .recv_timeout(Duration::from_millis(250))
            .unwrap_or(Event::Tick);
        let effects = update(&mut state, event, terminal.area());
        for effect in effects {
            match effect {
                Effect::Scan => {
                    state.replace_books(library::scan(&books_dir));
                    let rows = rows_visible(terminal.area().height);
                    let requests = state.request_metadata(rows);
                    if !requests.is_empty() {
                        let _ = metadata_sender.send(requests);
                    }
                }
                Effect::LoadHistory => {
                    state.records = history::load(&history_path);
                }
                Effect::LoadMetadata(paths) => {
                    let _ = metadata_sender.send(paths);
                }
                Effect::Open(path) => {
                    let runner = SystemRunner::default();
                    let opened = open_book(&mut terminal, &runner, &state.config.reader, &path);
                    discard_stale_keys(&receiver, &events);
                    match opened {
                        Ok(()) => {
                            state.records = history::load(&history_path);
                        }
                        Err(error) => state.set_status(format!("could not open book: {error}")),
                    }
                }
                Effect::Quit => {}
            }
        }
        match dir_transition(dir_present, &mut last_dir_check, &books_dir) {
            DirChange::Vanished => {
                dir_present = false;
                state.set_persistent_status(banner.clone());
                state.replace_books(Vec::new());
            }
            DirChange::Returned => {
                dir_present = true;
                if is_banner(&state.status, &banner) {
                    state.status = None;
                }
                state.replace_books(library::scan(&books_dir));
                _watcher = None;
                _watcher = watcher::start(books_dir.clone(), events.clone()).ok();
            }
            DirChange::StillMissing => {
                if !is_banner(&state.status, &banner) {
                    state.set_persistent_status(banner.clone());
                }
            }
            DirChange::Unchanged => {}
        }
        let requests = state.request_metadata(rows_visible(terminal.area().height));
        if !requests.is_empty() {
            let _ = metadata_sender.send(requests);
        }
        terminal.draw(&state)?;
        if state.should_quit {
            break;
        }
    }
    Ok(())
}

const DIR_CHECK_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, PartialEq, Eq)]
enum DirChange {
    Unchanged,
    Vanished,
    StillMissing,
    Returned,
}

fn missing_message(directory: &Path) -> String {
    format!("{} does not exist", directory.display())
}

fn is_banner(status: &Option<Status>, banner: &str) -> bool {
    status.as_ref().is_some_and(|status| status.text == banner)
}

/// Drops key events queued while the reader had the terminal, keeping the rest.
fn discard_stale_keys(receiver: &Receiver<Event>, sender: &Sender<Event>) {
    let kept: Vec<Event> = receiver
        .try_iter()
        .filter(|event| !matches!(event, Event::Key(_)))
        .collect();
    for event in kept {
        let _ = sender.send(event);
    }
}

fn classify_dir(was_present: bool, is_present: bool) -> DirChange {
    match (was_present, is_present) {
        (true, false) => DirChange::Vanished,
        (false, false) => DirChange::StillMissing,
        (false, true) => DirChange::Returned,
        _ => DirChange::Unchanged,
    }
}

/// Re-checks the books directory at most every `DIR_CHECK_INTERVAL`.
fn dir_transition(was_present: bool, last_check: &mut Instant, directory: &Path) -> DirChange {
    if last_check.elapsed() < DIR_CHECK_INTERVAL {
        return DirChange::Unchanged;
    }
    *last_check = Instant::now();
    classify_dir(was_present, directory.is_dir())
}

/// Reads terminal input on its own thread. While `suspended` is set (a reader
/// owns the terminal) it leaves input alone so keystrokes are not stolen.
fn spawn_input(sender: Sender<Event>, suspended: Arc<AtomicBool>) {
    thread::spawn(move || loop {
        if suspended.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(50));
            continue;
        }
        match terminal_event::poll(Duration::from_millis(50)) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(_) => break,
        }
        let event = match terminal_event::read() {
            Ok(terminal_event::Event::Key(key)) => Event::Key(key),
            Ok(terminal_event::Event::Resize(width, height)) => Event::Resize(width, height),
            Ok(_) => continue,
            Err(_) => break,
        };
        if sender.send(event).is_err() {
            break;
        }
    });
}

fn spawn_metadata_worker(
    mut cache: MetadataCache<PdfInfoReader<SystemRunner>>,
    receiver: Receiver<Vec<BookPath>>,
    sender: Sender<Event>,
) {
    thread::spawn(move || {
        while let Ok(paths) = receiver.recv() {
            let loaded = paths
                .into_iter()
                .map(|path| {
                    let metadata = cache.metadata_for(&path);
                    (path, metadata)
                })
                .collect();
            if sender.send(Event::MetadataLoaded(loaded)).is_err() {
                break;
            }
        }
    });
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    suspended: Arc<AtomicBool>,
}

impl TerminalSession {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut out = stdout();
        execute!(out, EnterAlternateScreen, Hide)?;
        let backend = CrosstermBackend::new(out);
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal,
            suspended: Arc::new(AtomicBool::new(false)),
        })
    }

    fn area(&self) -> Rect {
        match self.terminal.size() {
            Ok(size) => Rect::new(0, 0, size.width, size.height),
            Err(_) => Rect::new(0, 0, 80, 24),
        }
    }

    fn draw(&mut self, state: &AppState) -> Result<()> {
        self.terminal.draw(|frame| ui::draw(frame, state))?;
        Ok(())
    }
}

impl TerminalControl for TerminalSession {
    fn suspend(&mut self) -> Result<()> {
        self.suspended.store(true, Ordering::SeqCst);
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen, Show)?;
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        execute!(self.terminal.backend_mut(), EnterAlternateScreen, Hide)?;
        enable_raw_mode()?;
        self.terminal.clear()?;
        self.suspended.store(false, Ordering::SeqCst);
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen, Show);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_directory_transitions() {
        assert_eq!(classify_dir(true, false), DirChange::Vanished);
        assert_eq!(classify_dir(false, true), DirChange::Returned);
        assert_eq!(classify_dir(true, true), DirChange::Unchanged);
        assert_eq!(classify_dir(false, false), DirChange::StillMissing);
        assert_eq!(classify_dir(true, true), DirChange::Unchanged);
    }

    #[test]
    fn banner_is_recognised_only_by_exact_text() {
        let status = |text: &str| {
            Some(Status {
                text: text.to_string(),
                ticks_left: None,
            })
        };
        assert!(is_banner(&status("/b does not exist"), "/b does not exist"));
        assert!(!is_banner(
            &status("could not open book"),
            "/b does not exist"
        ));
        assert!(!is_banner(&None, "/b does not exist"));
    }

    #[test]
    fn stale_keys_are_dropped_and_other_events_kept() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let (sender, receiver) = mpsc::channel();
        sender
            .send(Event::Key(KeyEvent::new(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
            )))
            .unwrap();
        sender.send(Event::FilesChanged).unwrap();
        sender.send(Event::Tick).unwrap();
        discard_stale_keys(&receiver, &sender);
        let rest: Vec<Event> = receiver.try_iter().collect();
        assert_eq!(rest, vec![Event::FilesChanged, Event::Tick]);
    }

    #[test]
    fn missing_message_names_the_directory() {
        assert_eq!(
            missing_message(Path::new("/x/books")),
            "/x/books does not exist"
        );
    }
}
