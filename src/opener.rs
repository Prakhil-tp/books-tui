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
    std::iter::once(reader.program())
        .chain(reader.args())
        .map(str::to_string)
        .chain(std::iter::once(
            path.as_path().to_string_lossy().into_owned(),
        ))
        .collect()
}

pub fn open_book(
    terminal: &mut dyn TerminalControl,
    runner: &dyn CommandRunner,
    reader: &ReaderCommand,
    path: &BookPath,
) -> Result<()> {
    let argv = reader_argv(reader, path);
    let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
    if let Err(error) = terminal.suspend() {
        // A half-suspended terminal must still be restored.
        let _ = terminal.resume();
        return Err(error);
    }
    let result = runner.run_interactive(&argv_refs);
    let resumed = terminal.resume();
    result?;
    resumed?;
    Ok(())
}
