use crate::state::NoteItem;

/// Case-insensitive substring check without allocating lowercased copies.
fn contains_case_insensitive(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let needle_lower: Vec<char> = needle.chars().flat_map(|c| c.to_lowercase()).collect();
    let needle_len = needle_lower.len();
    let hay_chars: Vec<char> = haystack.chars().flat_map(|c| c.to_lowercase()).collect();
    hay_chars.windows(needle_len).any(|w| w == needle_lower.as_slice())
}

/// Simple full-text search across note titles and content.
/// Returns true if the note matches the query (case-insensitive substring).
pub fn note_matches_query(note: &NoteItem, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    contains_case_insensitive(&note.name, query)
        || contains_case_insensitive(&note.content, query)
}
