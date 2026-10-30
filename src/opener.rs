//! Launch the configured reader and resume the interface afterwards.

use crate::config::ReaderCommand;
use crate::library::BookPath;
use crate::ports::CommandRunner;
use anyhow::Result;

pub trait TerminalControl {
    fn suspend(&mut self) -> Result<()>;
    fn resume(&mut self) -> Result<()>;
}

pub fn reader_argv(reader: &ReaderCommand, path: &BookPath) -> Vec<String> {
    vec![
        reader.as_str().to_string(),
        path.as_path().to_string_lossy().into_owned(),
    ]
}

pub fn open_book(
    terminal: &mut dyn TerminalControl,
    runner: &dyn CommandRunner,
    reader: &ReaderCommand,
    path: &BookPath,
) -> Result<()> {
    let argv = reader_argv(reader, path);
    let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
    terminal.suspend()?;
    let result = runner.run_interactive(&argv_refs);
    let resumed = terminal.resume();
    result?;
    resumed?;
    Ok(())
}
