//! Configuration value objects, file storage, and first-run setup.

use crate::ports::{ConfigStore, Prompt};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const DEFAULT_BOOKS_DIR: &str = "~/books";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BooksDir(PathBuf);

impl BooksDir {
    pub fn parse(input: &str, home: &Path) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            bail!("books directory cannot be empty");
        }
        let expanded = expand_home(trimmed, home);
        let path = if expanded.is_absolute() {
            expanded
        } else {
            std::env::current_dir()?.join(expanded)
        };
        Ok(Self(path))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

pub fn expand_home(input: &str, home: &Path) -> PathBuf {
    if input == "~" {
        return home.to_path_buf();
    }
    match input.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None => PathBuf::from(input),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderCommand(String);

impl ReaderCommand {
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            bail!("reader command cannot be empty");
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The executable: the first whitespace-separated token.
    pub fn program(&self) -> &str {
        self.0.split_ascii_whitespace().next().unwrap_or_default()
    }

    /// Extra arguments that precede the book path.
    pub fn args(&self) -> Vec<&str> {
        self.0.split_ascii_whitespace().skip(1).collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub books_dir: BooksDir,
    pub reader: ReaderCommand,
}

impl Config {
    pub fn new(books_dir: BooksDir, reader: ReaderCommand) -> Self {
        Self { books_dir, reader }
    }
}

#[derive(Serialize, Deserialize)]
struct ConfigFile {
    books_dir: String,
    reader: String,
}

pub struct FileConfigStore {
    path: PathBuf,
    home: PathBuf,
}

impl FileConfigStore {
    pub fn new(path: PathBuf, home: PathBuf) -> Self {
        Self { path, home }
    }
}

impl ConfigStore for FileConfigStore {
    fn load(&self) -> Result<Option<Config>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&self.path)
            .with_context(|| format!("reading {}", self.path.display()))?;
        let raw: ConfigFile =
            toml::from_str(&text).with_context(|| format!("parsing {}", self.path.display()))?;
        Ok(Some(Config::new(
            BooksDir::parse(&raw.books_dir, &self.home)?,
            ReaderCommand::parse(&raw.reader)?,
        )))
    }

    fn save(&self, config: &Config) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = ConfigFile {
            books_dir: config.books_dir.as_path().to_string_lossy().into_owned(),
            reader: config.reader.as_str().to_string(),
        };
        let temporary = self.path.with_extension("toml.tmp");
        std::fs::write(&temporary, toml::to_string(&raw)?)?;
        std::fs::rename(&temporary, &self.path)?;
        Ok(())
    }
}

pub struct StdinPrompt;

impl StdinPrompt {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StdinPrompt {
    fn default() -> Self {
        Self::new()
    }
}

impl StdinPrompt {
    fn read(prompt: &str) -> Result<String> {
        print!("{prompt}");
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        Ok(answer.trim().to_string())
    }
}

impl Prompt for StdinPrompt {
    fn ask(&self, question: &str, default: &str) -> Result<String> {
        let answer = Self::read(&format!("{question} [{default}]: "))?;
        Ok(if answer.is_empty() {
            default.to_string()
        } else {
            answer
        })
    }

    fn confirm(&self, question: &str) -> Result<bool> {
        Ok(is_yes(&Self::read(&format!("{question} [y/N] "))?))
    }
}

/// Config file location: `$XDG_CONFIG_HOME` (if non-empty and absolute), then
/// the platform config directory, then `~/.config`.
pub fn resolve_config_path(
    xdg_config_home: Option<&std::ffi::OsStr>,
    system_config_dir: Option<PathBuf>,
    home: &Path,
) -> PathBuf {
    let base = xdg_config_home
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or(system_config_dir)
        .unwrap_or_else(|| home.join(".config"));
    base.join("books-tui").join("config.toml")
}

pub struct FirstRun<'a> {
    store: &'a dyn ConfigStore,
    prompt: &'a dyn Prompt,
    home: &'a Path,
    default_reader: &'a str,
}

impl<'a> FirstRun<'a> {
    pub fn new(
        store: &'a dyn ConfigStore,
        prompt: &'a dyn Prompt,
        home: &'a Path,
        default_reader: &'a str,
    ) -> Self {
        Self {
            store,
            prompt,
            home,
            default_reader,
        }
    }

    pub fn run(&self) -> Result<Config> {
        println!("Welcome to books-tui.");
        loop {
            let answer = self.prompt.ask("Book directory", DEFAULT_BOOKS_DIR)?;
            let answer = answer.trim();
            let answer = if answer.is_empty() {
                DEFAULT_BOOKS_DIR
            } else {
                answer
            };
            let books_dir = BooksDir::parse(answer, self.home)?;
            if books_dir.as_path().is_dir() {
                return self.persist(books_dir);
            }
            if self
                .prompt
                .confirm("Directory does not exist. Create it?")?
            {
                std::fs::create_dir_all(books_dir.as_path())?;
                return self.persist(books_dir);
            }
        }
    }

    fn persist(&self, books_dir: BooksDir) -> Result<Config> {
        let config = Config::new(books_dir, ReaderCommand::parse(self.default_reader)?);
        self.store.save(&config)?;
        Ok(config)
    }
}

pub fn is_yes(answer: &str) -> bool {
    matches!(answer.trim().to_lowercase().as_str(), "y" | "yes")
}
