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
