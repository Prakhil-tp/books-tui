//! Ports at I/O boundaries.

use crate::config::{Config, ReaderCommand};
use anyhow::Result;
use std::process::{ExitStatus, Output};

pub trait ConfigStore {
    fn load(&self) -> Result<Option<Config>>;
    fn save(&self, config: &Config) -> Result<()>;
}

pub trait Prompt {
    fn ask(&self, question: &str, default: &str) -> Result<String>;

    /// Asks a yes/no question that defaults to no.
    fn confirm(&self, question: &str) -> Result<bool> {
        Ok(crate::config::is_yes(&self.ask(question, "n")?))
    }
}

pub trait Readiness {
    fn locate(&self, command: &ReaderCommand) -> bool;
}

pub trait CommandRunner {
    fn run(&self, argv: &[&str]) -> Result<Output>;
    fn run_interactive(&self, argv: &[&str]) -> Result<ExitStatus>;
}
