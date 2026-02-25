use crate::state::*;

// ---------------------------------------------------------------------------
// Snapshots
// ---------------------------------------------------------------------------

pub fn push_snapshot(note: &mut NoteItem, content: String) {
    if content.trim().is_empty() {
        return;
    }
    if note
        .versions
        .last()
        .is_some_and(|version| version.content == content)
    {
        return;
    }
    note.versions.push(NoteVersion {
        ts: unix_now(),
        content,
    });
    if note.versions.len() > 10 {
        let overflow = note.versions.len() - 10;
        note.versions.drain(0..overflow);
    }
}

// ---------------------------------------------------------------------------
// Trash auto-purge
// ---------------------------------------------------------------------------

const TRASH_PURGE_DAYS: i64 = 30;

/// Remove trash items older than 30 days. Returns the number of items purged.
pub fn purge_old_trash(state: &mut DocState) -> usize {
    let cutoff = unix_now() - (TRASH_PURGE_DAYS * 24 * 60 * 60);
    let before = state.trash.len();
    state.trash.retain(|item| item.deleted_at >= cutoff);
    before - state.trash.len()
}
