//! Fuzzy matching and ranking.

use nucleo_matcher::{Config, Matcher, Utf32Str};

pub fn rank(query: &str, items: &[(usize, String)]) -> Vec<usize> {
    if query.trim().is_empty() {
        return items.iter().map(|(index, _)| *index).collect();
    }
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut needle_buffer = Vec::new();
    let needle = Utf32Str::new(query, &mut needle_buffer);
    let mut scored: Vec<(u16, usize)> = Vec::new();
    for (index, title) in items {
        let mut haystack_buffer = Vec::new();
        let haystack = Utf32Str::new(title, &mut haystack_buffer);
        if let Some(score) = matcher.fuzzy_match(haystack, needle) {
            scored.push((score, *index));
        }
    }
    scored.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(&right.1)));
    scored.into_iter().map(|(_, index)| index).collect()
}
