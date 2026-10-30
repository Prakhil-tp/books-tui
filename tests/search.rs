use books_tui::search::rank;

fn items(titles: &[&str]) -> Vec<(usize, String)> {
    titles
        .iter()
        .enumerate()
        .map(|(index, title)| (index, title.to_string()))
        .collect()
}

#[test]
fn empty_query_keeps_original_order() {
    let items = items(&["Clean Code", "Rust Book", "Designing Data"]);
    assert_eq!(rank("", &items), vec![0, 1, 2]);
    assert_eq!(rank("   ", &items), vec![0, 1, 2]);
}

#[test]
fn matches_subsequences_case_insensitively() {
    let items = items(&["Clean Code", "Rust Book", "Designing Data"]);
    assert_eq!(rank("rust", &items), vec![1]);
    assert_eq!(rank("code", &items), vec![0]);
}

#[test]
fn ranks_prefix_matches_first() {
    let items = items(&["Trust Book", "Rust Book"]);
    assert_eq!(rank("rust book", &items), vec![1, 0]);
}

#[test]
fn excludes_non_matches() {
    let items = items(&["Clean Code", "Rust Book"]);
    assert!(rank("zzzz", &items).is_empty());
}

#[test]
fn ties_keep_filename_order() {
    let items = items(&["Book A", "Book B"]);
    assert_eq!(rank("book", &items), vec![0, 1]);
}
