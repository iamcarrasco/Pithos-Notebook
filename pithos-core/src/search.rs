use crate::state::NoteItem;

/// Simple full-text search across note titles and content.
/// Returns true if the note matches the query (case-insensitive substring).
pub fn note_matches_query(note: &NoteItem, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let query_lower = query.to_lowercase();
    note.name.to_lowercase().contains(&query_lower)
        || note.content.to_lowercase().contains(&query_lower)
}
