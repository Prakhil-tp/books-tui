//! Reader installation across Linux package managers and Homebrew.

use crate::config::ReaderCommand;
use crate::ports::{CommandRunner, Prompt, Readiness};
use anyhow::{bail, Result};
use std::io::Write;
use std::path::Path;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::time::Duration;
use wait_timeout::ChildExt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageManager {
    pub name: &'static str,
    pub binary: &'static str,
    pub description: &'static str,
    pub install_prefix: &'static [&'static str],
}

impl PackageManager {
    pub fn install_argv(&self, package: &str) -> Vec<String> {
        self.install_prefix
            .iter()
            .map(|part| part.to_string())
            .chain(std::iter::once(package.to_string()))
            .collect()
    }

    pub fn command(&self, package: &str) -> String {
        self.install_argv(package).join(" ")
    }
}

pub const MANAGERS: [PackageManager; 6] = [
    PackageManager {
        name: "apt",
        binary: "apt-get",
        description: "Debian/Ubuntu",
        install_prefix: &["sudo", "apt-get", "install", "-y"],
    },
    PackageManager {
        name: "dnf",
        binary: "dnf",
        description: "Fedora/RHEL",
        install_prefix: &["sudo", "dnf", "install", "-y"],
    },
    PackageManager {
        name: "pacman",
        binary: "pacman",
        description: "Arch",
        install_prefix: &["sudo", "pacman", "-S", "--noconfirm"],
    },
    PackageManager {
        name: "zypper",
        binary: "zypper",
        description: "openSUSE",
        install_prefix: &["sudo", "zypper", "install", "-y"],
    },
    PackageManager {
        name: "apk",
        binary: "apk",
        description: "Alpine",
        install_prefix: &["sudo", "apk", "add"],
    },
    PackageManager {
        name: "brew",
        binary: "brew",
        description: "macOS Homebrew",
        install_prefix: &["brew", "install"],
    },
];

pub fn detect(available: &dyn Fn(&str) -> bool) -> Option<&'static PackageManager> {
    MANAGERS.iter().find(|manager| available(manager.binary))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallOutcome {
    Present,
    Installed,
    Declined,
    Failed,
    NoManager,
}

pub struct ReaderInstaller<'a> {
    pub readiness: &'a dyn Readiness,
    pub prompt: &'a dyn Prompt,
    pub runner: &'a dyn CommandRunner,
    pub available: &'a dyn Fn(&str) -> bool,
    pub reader: &'a ReaderCommand,
    pub package: &'a str,
    pub out: &'a mut dyn Write,
}

impl ReaderInstaller<'_> {
    pub fn ensure(&mut self) -> Result<InstallOutcome> {
        if self.readiness.locate(self.reader) {
            return Ok(InstallOutcome::Present);
        }
        writeln!(
            self.out,
            "Reader \"{}\" was not found.",
            self.reader.as_str()
        )?;
        let Some(manager) = detect(self.available) else {
            writeln!(
                self.out,
                "No supported package manager found. Install {} manually.",
                self.package
            )?;
            return Ok(InstallOutcome::NoManager);
        };
        writeln!(
            self.out,
            "Detected: {} ({})",
            manager.name, manager.description
        )?;
        let answer = self.prompt.ask("Install it now?", "n")?;
        if !crate::config::is_yes(&answer) {
            writeln!(
                self.out,
                "Install it with: {}",
                manager.command(self.package)
            )?;
            return Ok(InstallOutcome::Declined);
        }
        let argv = manager.install_argv(self.package);
        let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        let success = self
            .runner
            .run_interactive(&argv_refs)
            .map(|status| status.success())
            .unwrap_or(false);
        if success && self.readiness.locate(self.reader) {
            writeln!(self.out, "Installed {}.", self.package)?;
            return Ok(InstallOutcome::Installed);
        }
        writeln!(
            self.out,
            "Install failed. Run it manually: {}",
            manager.command(self.package)
        )?;
        Ok(InstallOutcome::Failed)
    }
}

pub struct PathReadiness;

impl Readiness for PathReadiness {
    fn locate(&self, command: &ReaderCommand) -> bool {
        let name = command.as_str();
        if name.contains(std::path::MAIN_SEPARATOR) {
            return Path::new(name).is_file();
        }
        let Some(paths) = std::env::var_os("PATH") else {
            return false;
        };
        std::env::split_paths(&paths).any(|directory| directory.join(name).is_file())
    }
}

#[derive(Clone, Debug)]
pub struct SystemRunner {
    pub timeout: Duration,
}

impl Default for SystemRunner {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(15),
        }
    }
}

impl CommandRunner for SystemRunner {
    fn run(&self, argv: &[&str]) -> Result<Output> {
        if argv.is_empty() {
            bail!("command argv must not be empty");
        }
        let mut child = Command::new(argv[0])
            .args(&argv[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        match child.wait_timeout(self.timeout)? {
            Some(_) => Ok(child.wait_with_output()?),
            None => {
                let _ = child.kill();
                let _ = child.wait();
                bail!("command timed out: {}", argv.join(" "))
            }
        }
    }

    fn run_interactive(&self, argv: &[&str]) -> Result<ExitStatus> {
        if argv.is_empty() {
            bail!("command argv must not be empty");
        }
        Ok(Command::new(argv[0]).args(&argv[1..]).status()?)
    }
}
