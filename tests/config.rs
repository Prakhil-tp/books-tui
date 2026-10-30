use books_tui::config::{BooksDir, Config, FileConfigStore, FirstRun, ReaderCommand};
use books_tui::ports::{ConfigStore, Prompt};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn config(dir: &Path) -> Config {
    Config::new(
        BooksDir::parse(dir.to_str().unwrap(), dir).unwrap(),
        ReaderCommand::parse("zathura").unwrap(),
    )
}

#[test]
fn books_dir_expands_tilde_and_rejects_empty() {
    let home = Path::new("/home/tester");
    let expanded = BooksDir::parse("~/books", home).unwrap();
    assert_eq!(expanded.as_path(), Path::new("/home/tester/books"));
    assert!(BooksDir::parse("   ", home).is_err());
}

#[test]
fn reader_command_trims_and_rejects_empty() {
    assert_eq!(
        ReaderCommand::parse("  zathura ").unwrap().as_str(),
        "zathura"
    );
    assert!(ReaderCommand::parse("").is_err());
}

#[test]
fn file_store_round_trips_config() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("books-tui/config.toml");
    let store = FileConfigStore::new(path.clone(), PathBuf::from("/home/tester"));
    let original = config(temp.path());

    store.save(&original).unwrap();
    assert_eq!(store.load().unwrap(), Some(original));
    assert!(path.exists());
}

#[test]
fn file_store_expands_tilde_when_loading() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("config.toml");
    std::fs::write(&path, "books_dir = \"~/books\"\nreader = \"zathura\"\n").unwrap();
    let store = FileConfigStore::new(path, temp.path().join("home"));
    let loaded = store.load().unwrap().unwrap();
    assert_eq!(
        loaded.books_dir.as_path(),
        temp.path().join("home").join("books").as_path()
    );
}

#[test]
fn file_store_returns_none_when_missing() {
    let temp = tempdir().unwrap();
    let store = FileConfigStore::new(temp.path().join("missing.toml"), temp.path().into());
    assert_eq!(store.load().unwrap(), None);
}

struct FakeStore {
    saved: RefCell<Option<Config>>,
}

impl ConfigStore for FakeStore {
    fn load(&self) -> anyhow::Result<Option<Config>> {
        Ok(self.saved.borrow().clone())
    }
    fn save(&self, config: &Config) -> anyhow::Result<()> {
        *self.saved.borrow_mut() = Some(config.clone());
        Ok(())
    }
}

struct FakePrompt {
    answers: RefCell<Vec<String>>,
    asked: RefCell<Vec<String>>,
}

impl FakePrompt {
    fn new(answers: &[&str]) -> Self {
        Self {
            answers: RefCell::new(answers.iter().map(|a| a.to_string()).collect()),
            asked: RefCell::new(Vec::new()),
        }
    }
}

impl Prompt for FakePrompt {
    fn ask(&self, question: &str, default: &str) -> anyhow::Result<String> {
        self.asked
            .borrow_mut()
            .push(format!("{question}|{default}"));
        Ok(self.answers.borrow_mut().remove(0))
    }
}

#[test]
fn first_run_defaults_to_home_books_and_persists() {
    let temp = tempdir().unwrap();
    std::fs::create_dir(temp.path().join("books")).unwrap();
    let store = FakeStore {
        saved: RefCell::new(None),
    };
    let prompt = FakePrompt::new(&[""]);
    let config = FirstRun::new(&store, &prompt, temp.path(), "zathura")
        .run()
        .unwrap();

    assert_eq!(config.books_dir.as_path(), temp.path().join("books"));
    assert_eq!(config.reader.as_str(), "zathura");
    assert_eq!(store.saved.borrow().clone(), Some(config));
    assert!(prompt.asked.borrow()[0].ends_with("|~/books"));
}

#[test]
fn first_run_creates_a_missing_directory_after_confirmation() {
    let temp = tempdir().unwrap();
    let store = FakeStore {
        saved: RefCell::new(None),
    };
    let prompt = FakePrompt::new(&["~/new-books", "y"]);
    let config = FirstRun::new(&store, &prompt, temp.path(), "zathura")
        .run()
        .unwrap();

    assert_eq!(config.books_dir.as_path(), temp.path().join("new-books"));
    assert!(config.books_dir.as_path().is_dir());
}

#[test]
fn first_run_reprompts_when_creation_is_declined() {
    let temp = tempdir().unwrap();
    std::fs::create_dir(temp.path().join("books")).unwrap();
    let store = FakeStore {
        saved: RefCell::new(None),
    };
    let prompt = FakePrompt::new(&["~/nope", "n", "~/books"]);
    let config = FirstRun::new(&store, &prompt, temp.path(), "zathura")
        .run()
        .unwrap();

    assert_eq!(config.books_dir.as_path(), temp.path().join("books"));
}

#[test]
fn reader_command_splits_program_and_arguments_on_whitespace() {
    let reader = ReaderCommand::parse("  zathura   --fork -x ").unwrap();
    assert_eq!(reader.program(), "zathura");
    assert_eq!(reader.args(), vec!["--fork", "-x"]);
    let bare = ReaderCommand::parse("zathura").unwrap();
    assert_eq!(bare.program(), "zathura");
    assert!(bare.args().is_empty());
}
