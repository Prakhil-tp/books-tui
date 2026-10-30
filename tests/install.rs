use books_tui::config::ReaderCommand;
use books_tui::install::{detect, InstallOutcome, PathReadiness, ReaderInstaller, SystemRunner};
use books_tui::ports::{CommandRunner, Prompt, Readiness};
use std::cell::Cell;
use std::os::unix::process::ExitStatusExt;
use std::process::{ExitStatus, Output};

struct FakeReadiness {
    calls: Cell<usize>,
    present_initially: bool,
    present_after_install: bool,
}

impl Readiness for FakeReadiness {
    fn locate(&self, _command: &ReaderCommand) -> bool {
        self.calls.set(self.calls.get() + 1);
        if self.calls.get() == 1 {
            self.present_initially
        } else {
            self.present_after_install
        }
    }
}

struct FakePrompt(&'static str);

impl Prompt for FakePrompt {
    fn ask(&self, _question: &str, _default: &str) -> anyhow::Result<String> {
        Ok(self.0.to_string())
    }
}

struct FakeRunner {
    calls: Cell<usize>,
    succeed: bool,
}

impl CommandRunner for FakeRunner {
    fn run(&self, _argv: &[&str]) -> anyhow::Result<Output> {
        self.calls.set(self.calls.get() + 1);
        let code = if self.succeed { 0 } else { 1 };
        Ok(Output {
            status: ExitStatus::from_raw(code),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }
    fn run_interactive(&self, _argv: &[&str]) -> anyhow::Result<ExitStatus> {
        self.calls.set(self.calls.get() + 1);
        let code = if self.succeed { 0 } else { 1 };
        Ok(ExitStatus::from_raw(code))
    }
}

fn install(
    readiness: &FakeReadiness,
    prompt: &FakePrompt,
    runner: &FakeRunner,
    available: &dyn Fn(&str) -> bool,
    out: &mut Vec<u8>,
) -> InstallOutcome {
    let reader = ReaderCommand::parse("zathura").unwrap();
    ReaderInstaller {
        readiness,
        prompt,
        runner,
        available,
        reader: &reader,
        package: "zathura",
        out,
    }
    .ensure()
    .unwrap()
}

#[test]
fn detects_package_managers_in_order() {
    let apt = detect(&|binary| binary == "apt-get").unwrap();
    assert_eq!(apt.name, "apt");
    let brew = detect(&|binary| binary == "brew").unwrap();
    assert_eq!(brew.name, "brew");
    assert!(detect(&|_| false).is_none());
}

#[test]
fn builds_install_commands() {
    let apt = detect(&|binary| binary == "apt-get").unwrap();
    assert_eq!(
        apt.install_argv("zathura"),
        vec!["sudo", "apt-get", "install", "-y", "zathura"]
    );
    let brew = detect(&|binary| binary == "brew").unwrap();
    assert_eq!(
        brew.install_argv("zathura"),
        vec!["brew", "install", "zathura"]
    );
}

#[test]
fn skips_install_when_reader_is_present() {
    let readiness = FakeReadiness {
        calls: Cell::new(0),
        present_initially: true,
        present_after_install: false,
    };
    let runner = FakeRunner {
        calls: Cell::new(0),
        succeed: true,
    };
    let outcome = install(
        &readiness,
        &FakePrompt("n"),
        &runner,
        &|binary| binary == "apt-get",
        &mut Vec::new(),
    );
    assert_eq!(outcome, InstallOutcome::Present);
    assert_eq!(runner.calls.get(), 0);
}

#[test]
fn declining_prints_the_command_without_running_it() {
    let readiness = FakeReadiness {
        calls: Cell::new(0),
        present_initially: false,
        present_after_install: false,
    };
    let runner = FakeRunner {
        calls: Cell::new(0),
        succeed: true,
    };
    let mut out = Vec::new();
    let outcome = install(
        &readiness,
        &FakePrompt("n"),
        &runner,
        &|binary| binary == "apt-get",
        &mut out,
    );
    assert_eq!(outcome, InstallOutcome::Declined);
    assert_eq!(runner.calls.get(), 0);
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("sudo apt-get install -y zathura"));
}

#[test]
fn installs_and_rechecks_the_reader() {
    let readiness = FakeReadiness {
        calls: Cell::new(0),
        present_initially: false,
        present_after_install: true,
    };
    let runner = FakeRunner {
        calls: Cell::new(0),
        succeed: true,
    };
    let outcome = install(
        &readiness,
        &FakePrompt("y"),
        &runner,
        &|binary| binary == "apt-get",
        &mut Vec::new(),
    );
    assert_eq!(outcome, InstallOutcome::Installed);
    assert_eq!(runner.calls.get(), 1);
}

#[test]
fn failed_install_reports_failure_and_command() {
    let readiness = FakeReadiness {
        calls: Cell::new(0),
        present_initially: false,
        present_after_install: false,
    };
    let runner = FakeRunner {
        calls: Cell::new(0),
        succeed: false,
    };
    let mut out = Vec::new();
    let outcome = install(
        &readiness,
        &FakePrompt("y"),
        &runner,
        &|binary| binary == "apt-get",
        &mut out,
    );
    assert_eq!(outcome, InstallOutcome::Failed);
    assert!(String::from_utf8(out)
        .unwrap()
        .contains("sudo apt-get install -y zathura"));
}

#[test]
fn no_manager_only_reports_instructions() {
    let readiness = FakeReadiness {
        calls: Cell::new(0),
        present_initially: false,
        present_after_install: false,
    };
    let runner = FakeRunner {
        calls: Cell::new(0),
        succeed: true,
    };
    let mut out = Vec::new();
    let outcome = install(&readiness, &FakePrompt("y"), &runner, &|_| false, &mut out);
    assert_eq!(outcome, InstallOutcome::NoManager);
    assert!(String::from_utf8(out)
        .unwrap()
        .contains("No supported package manager"));
    assert_eq!(runner.calls.get(), 0);
}

#[test]
fn path_readiness_finds_common_binaries() {
    let readiness = PathReadiness;
    let sh = ReaderCommand::parse("sh").unwrap();
    assert!(readiness.locate(&sh));
    let missing = ReaderCommand::parse("definitely-not-a-real-binary-xyz").unwrap();
    assert!(!readiness.locate(&missing));
}

#[test]
fn system_runner_reports_missing_binaries() {
    let runner = SystemRunner::default();
    assert!(runner.run(&["definitely-not-a-real-binary-xyz"]).is_err());
}

#[test]
fn system_runner_rejects_empty_argv() {
    let runner = SystemRunner::default();
    assert!(runner.run(&[]).is_err());
    assert!(runner.run_interactive(&[]).is_err());
}

#[test]
fn system_runner_runs_commands_successfully() {
    let runner = SystemRunner::default();
    let output = runner.run(&["sh", "-c", "exit 0"]).unwrap();
    assert!(output.status.success());
    let status = runner.run_interactive(&["sh", "-c", "exit 0"]).unwrap();
    assert!(status.success());
}

#[test]
fn path_readiness_checks_only_the_program_token() {
    let dir = tempfile::tempdir().unwrap();
    let binary = dir.path().join("fakereader");
    std::fs::write(&binary, b"").unwrap();
    let with_args = ReaderCommand::parse(&format!("{} --fork", binary.display())).unwrap();
    assert!(PathReadiness.locate(&with_args));
    let missing = ReaderCommand::parse("/nonexistent/reader --fork").unwrap();
    assert!(!PathReadiness.locate(&missing));
}

#[test]
fn installable_package_is_offered_only_for_zathura() {
    use books_tui::install::installable_package;
    let zathura = ReaderCommand::parse("zathura --fork").unwrap();
    assert_eq!(installable_package(&zathura), Some("zathura"));
    let other = ReaderCommand::parse("evince").unwrap();
    assert_eq!(installable_package(&other), None);
}

struct RecordingPrompt(std::cell::RefCell<Vec<String>>);

impl Prompt for RecordingPrompt {
    fn ask(&self, question: &str, _default: &str) -> anyhow::Result<String> {
        self.0.borrow_mut().push(question.to_string());
        Ok("n".to_string())
    }
}

#[test]
fn install_prompt_names_the_package() {
    let readiness = FakeReadiness {
        calls: Cell::new(0),
        present_initially: false,
        present_after_install: false,
    };
    let runner = FakeRunner {
        calls: Cell::new(0),
        succeed: true,
    };
    let prompt = RecordingPrompt(Default::default());
    let reader = ReaderCommand::parse("zathura").unwrap();
    ReaderInstaller {
        readiness: &readiness,
        prompt: &prompt,
        runner: &runner,
        available: &|binary| binary == "apt-get",
        reader: &reader,
        package: "zathura",
        out: &mut Vec::new(),
    }
    .ensure()
    .unwrap();
    assert_eq!(*prompt.0.borrow(), vec!["Install zathura now?"]);
}
