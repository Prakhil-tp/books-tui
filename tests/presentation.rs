use books_tui::presentation::{format_relative, format_size, prettify, truncate, Progress};
use chrono::{Local, TimeZone};
use std::path::Path;

#[test]
fn prettify_strips_author_and_dashes() {
    let path = Path::new("designing-data-intensive-applications-by-martin-kleppmann.pdf");
    assert_eq!(prettify(path), "Designing Data Intensive Applications");
}

#[test]
fn prettify_replaces_underscores_and_dots() {
    assert_eq!(prettify(Path::new("clean_code.v2.pdf")), "Clean Code V2");
}

#[test]
fn prettify_keeps_title_when_no_author() {
    assert_eq!(
        prettify(Path::new("system-design-interview.pdf")),
        "System Design Interview"
    );
}

#[test]
fn format_size_handles_units() {
    assert_eq!(format_size(0), "0B");
    assert_eq!(format_size(1023), "1023B");
    assert_eq!(format_size(1024), "1.0K");
    assert_eq!(format_size(24 * 1024 * 1024), "24.0M");
}

#[test]
fn format_relative_covers_each_bucket() {
    let now = Local.with_ymd_and_hms(2026, 10, 28, 12, 0, 0).unwrap();
    let ago = |seconds: i64| format_relative(now.timestamp() - seconds, now);

    assert_eq!(ago(30), "just now");
    assert_eq!(ago(5 * 60), "5 min ago");
    assert_eq!(ago(3 * 3600), "3 h ago");
    assert_eq!(ago(26 * 3600), "yesterday");
    assert_eq!(ago(3 * 86400), "3 days ago");
    assert_eq!(ago(10 * 86400), "1 week ago");
    assert_eq!(ago(90 * 86400), "30 Jul 2026");
}

#[test]
fn progress_rounds_and_clamps() {
    assert_eq!(Progress::new(19, 613).percent(), 3);
    assert_eq!(Progress::new(50, 100).bar(10), (5, 5));
    assert_eq!(Progress::new(10, 0).bar(10), (0, 10));
    assert_eq!(Progress::new(700, 613).percent(), 100);
}

#[test]
fn truncate_respects_width() {
    assert_eq!(truncate("abcdef", 3), "abc");
    assert_eq!(truncate("日本", 3), "日");
    assert_eq!(truncate("abc", 10), "abc");
}
