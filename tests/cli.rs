use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn list_prints_pdf_paths() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.pdf"), b"x").unwrap();
    fs::write(dir.path().join("b.txt"), b"x").unwrap();

    let output = Command::cargo_bin("books-tui")
        .unwrap()
        .args(["--list", "--books-dir", dir.path().to_str().unwrap()])
        .env("HOME", dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim().lines().count(), 1);
    assert!(stdout.contains("a.pdf"));
}

#[test]
fn list_reports_a_missing_directory() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("missing");
    let output = Command::cargo_bin("books-tui")
        .unwrap()
        .args(["--list", "--books-dir", missing.to_str().unwrap()])
        .env("HOME", dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("does not exist"));
}

#[test]
fn list_does_not_create_a_config_file() {
    let dir = tempdir().unwrap();
    let books = dir.path().join("books");
    let config_home = dir.path().join("config");
    fs::create_dir(&books).unwrap();
    fs::write(books.join("a.pdf"), b"x").unwrap();

    Command::cargo_bin("books-tui")
        .unwrap()
        .args(["--list", "--books-dir", books.to_str().unwrap()])
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", &config_home)
        .assert()
        .success();

    assert!(!config_home.join("books-tui/config.toml").exists());
}

#[test]
fn version_is_printed() {
    let output = Command::cargo_bin("books-tui")
        .unwrap()
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("books-tui"));
}

#[test]
fn list_honours_books_dir_from_a_config_file() {
    let dir = tempdir().unwrap();
    let books = dir.path().join("shelf");
    fs::create_dir(&books).unwrap();
    fs::write(books.join("from-config.pdf"), b"x").unwrap();
    let config = dir.path().join("custom.toml");
    fs::write(
        &config,
        format!("books_dir = {:?}\nreader = \"zathura\"\n", books),
    )
    .unwrap();

    let output = Command::cargo_bin("books-tui")
        .unwrap()
        .args(["--list", "--config", config.to_str().unwrap()])
        .env("HOME", dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("from-config.pdf"));
}

// The binary only runs first-run for the interactive TUI (it refuses without a
// terminal), so the temp-HOME first-run flow is driven through the library with
// the real file store and scripted answers: no TTY and no TUI needed.
#[test]
fn first_run_writes_a_toml_config_under_xdg_config_home() {
    use books_tui::config::{resolve_config_path, FileConfigStore, FirstRun};
    use books_tui::ports::Prompt;
    use std::cell::RefCell;

    struct Scripted(RefCell<Vec<&'static str>>);
    impl Prompt for Scripted {
        fn ask(&self, _question: &str, _default: &str) -> anyhow::Result<String> {
            Ok(self.0.borrow_mut().remove(0).to_string())
        }
    }

    let home = tempdir().unwrap();
    let xdg = home.path().join("xdgconf");
    let path = resolve_config_path(Some(xdg.as_os_str()), None, home.path());
    assert_eq!(path, xdg.join("books-tui/config.toml"));
    let store = FileConfigStore::new(path.clone(), home.path().to_path_buf());
    let prompt = Scripted(RefCell::new(vec!["~/library", "y"]));

    FirstRun::new(&store, &prompt, home.path(), "zathura")
        .run()
        .unwrap();

    let text = fs::read_to_string(&path).unwrap();
    let parsed: toml::Table = toml::from_str(&text).unwrap();
    let expected = home.path().join("library");
    assert_eq!(
        parsed["books_dir"].as_str().unwrap(),
        expected.to_str().unwrap()
    );
    assert_eq!(parsed["reader"].as_str().unwrap(), "zathura");
    assert!(expected.is_dir());
}
