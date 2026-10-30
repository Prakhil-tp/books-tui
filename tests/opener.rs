use books_tui::config::ReaderCommand;
use books_tui::library::BookPath;
use books_tui::opener::{open_book, reader_argv, TerminalControl};
use books_tui::ports::CommandRunner;
use std::cell::RefCell;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{ExitStatus, Output};

#[derive(Default)]
struct FakeTerminal {
    calls: RefCell<Vec<&'static str>>,
}

impl TerminalControl for FakeTerminal {
    fn suspend(&mut self) -> anyhow::Result<()> {
        self.calls.borrow_mut().push("suspend");
        Ok(())
    }
    fn resume(&mut self) -> anyhow::Result<()> {
        self.calls.borrow_mut().push("resume");
        Ok(())
    }
}

struct FakeRunner {
    argv: RefCell<Vec<Vec<String>>>,
    fail: bool,
}

impl CommandRunner for FakeRunner {
    fn run(&self, _argv: &[&str]) -> anyhow::Result<Output> {
        unimplemented!("opener only uses run_interactive")
    }
    fn run_interactive(&self, argv: &[&str]) -> anyhow::Result<ExitStatus> {
        self.argv
            .borrow_mut()
            .push(argv.iter().map(|part| part.to_string()).collect());
        if self.fail {
            anyhow::bail!("reader failed");
        }
        Ok(ExitStatus::from_raw(0))
    }
}

#[test]
fn builds_reader_argv() {
    let reader = ReaderCommand::parse("zathura").unwrap();
    let path = BookPath::new(PathBuf::from("/books/a.pdf"));
    assert_eq!(reader_argv(&reader, &path), vec!["zathura", "/books/a.pdf"]);
}

#[test]
fn suspends_and_resumes_around_the_reader() {
    let reader = ReaderCommand::parse("zathura").unwrap();
    let path = BookPath::new(PathBuf::from("/books/a.pdf"));
    let mut terminal = FakeTerminal::default();
    let runner = FakeRunner {
        argv: RefCell::new(Vec::new()),
        fail: false,
    };

    open_book(&mut terminal, &runner, &reader, &path).unwrap();

    assert_eq!(*terminal.calls.borrow(), vec!["suspend", "resume"]);
    assert_eq!(runner.argv.borrow()[0], vec!["zathura", "/books/a.pdf"]);
}

#[test]
fn resumes_and_returns_the_error_when_the_reader_fails() {
    let reader = ReaderCommand::parse("zathura").unwrap();
    let path = BookPath::new(PathBuf::from("/books/a.pdf"));
    let mut terminal = FakeTerminal::default();
    let runner = FakeRunner {
        argv: RefCell::new(Vec::new()),
        fail: true,
    };

    assert!(open_book(&mut terminal, &runner, &reader, &path).is_err());
    assert_eq!(*terminal.calls.borrow(), vec!["suspend", "resume"]);
}
