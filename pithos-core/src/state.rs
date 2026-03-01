use crate::vault::AssetMeta;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Manual,
    ModifiedDesc,
    ModifiedAsc,
    NameAsc,
    NameDesc,
    CreatedDesc,
    CreatedAsc,
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NoteVersion {
    pub ts: i64,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct NoteItem {
    pub id: String,
    pub name: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub versions: Vec<NoteVersion>,
    pub file_path: Option<PathBuf>,
    pub parent_id: Option<String>,
    pub pinned: bool,
}

impl NoteItem {
    pub fn new(id: String, name: String, content: String, tags: Vec<String>) -> Self {
        let now = unix_now();
        Self {
            id,
            name,
            content,
            tags,
            created_at: now,
            updated_at: now,
            versions: Vec::new(),
            file_path: None,
            parent_id: None,
            pinned: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FolderItem {
    pub id: String,
    pub name: String,
    pub expanded: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TrashItem {
    pub id: String,
    pub name: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: i64,
    pub parent_id: Option<String>,
    pub versions: Vec<NoteVersion>,
    pub pinned: bool,
}

pub struct NoteSummary {
    pub id: String,
    pub name: String,
    pub content_snippet: Option<String>,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub pinned: bool,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SidebarRowKind {
    Folder(String),
    Note(String),
}

// ---------------------------------------------------------------------------
// Welcome content
// ---------------------------------------------------------------------------

pub const DEFAULT_DOC: &str = r#"# Untitled

"#;

pub const PAGE_WELCOME: &str = r#"# Welcome to Pithos Notebook

Pithos Notebook is a private, offline markdown notebook built for professionals who produce sensitive technical documentation. Your notes are encrypted with AES-256-GCM and never leave your machine.

## Getting started

1. **Write** in the source editor with a live preview beside it
2. **Organize** notes into folders from the sidebar
3. **Tag** notes with the tag bar below the editor
4. **Search** across all notes with **Ctrl+Shift+F**
5. **Export** any note as Markdown or PDF from the menu

Your work is auto-saved every 30 seconds and encrypted with your vault passphrase.

## Tips

- Press **Ctrl+Shift+P** to open the command palette
- Toggle **Zen mode** with **Ctrl+Shift+J** for distraction-free writing
- Press **Ctrl+\\** to show or hide the sidebar
- Right-click notes in the sidebar for rename, move, and delete
- Drag images directly into the editor — they are encrypted too
- Use **Ctrl+1** through **Ctrl+6** for heading levels
- Switch between vaults using the vault menu in the header bar

See the other built-in notes for formatting examples and keyboard shortcuts.
"#;

pub const PAGE_FEATURES: &str = r#"# Keyboard shortcuts

## Editing

| Action | Shortcut |
| --- | --- |
| Bold | Ctrl+B |
| Italic | Ctrl+I |
| Underline | Ctrl+U |
| Strikethrough | Ctrl+D |
| Inline code | Ctrl+E |
| Link | Ctrl+K |
| Headings 1-6 | Ctrl+1 through Ctrl+6 |
| Block quote | Ctrl+Shift+Q |
| Bullet list | Ctrl+Shift+L |
| Toggle checkbox | Ctrl+Space |
| Undo | Ctrl+Z |
| Redo | Ctrl+Shift+Z |

## Navigation

| Action | Shortcut |
| --- | --- |
| New note | Ctrl+N |
| Save | Ctrl+S |
| Close tab | Ctrl+W |
| Search notes | Ctrl+Shift+F |
| Rename note | F2 |
| Command palette | Ctrl+Shift+P |
| Daily note | Ctrl+Shift+T |

## View

| Action | Shortcut |
| --- | --- |
| Toggle sidebar | Ctrl+\\ |
| Zen mode | Ctrl+Shift+J |
| Fullscreen | F11 |
| Toggle dark/light | Ctrl+Shift+D |

## Other features

- **Version history** — save named snapshots and restore earlier content from the menu
- **Tabs** — open multiple notes in tabs, drag to reorder, close with Ctrl+W
- **Export** — save any note as Markdown (.md) or PDF from the menu
- **Encryption** — AES-256-GCM with PBKDF2 key derivation (600k iterations)
- **Multi-vault** — work with multiple vaults for different clients or projects
"#;

pub const PAGE_FORMATTING: &str = r#"# Formatting examples

## Inline styles

**Bold**, *italic*, <u>underline</u>, ~~strikethrough~~, and `inline code`.

Combine styles: ***bold italic***, **~~bold strikethrough~~**.

## Lists

- Bullet item
  - Nested item
  - Another nested item
- Second item

1. First step
2. Second step
3. Third step

- [ ] To-do item
- [x] Completed item
- [ ] Another to-do

## Block quote

> Use block quotes to highlight important text or citations.

## Code blocks

Code blocks appear as cards with a language selector and delete button:

```python
def greet(name):
    return f"Hello, {name}!"

print(greet("World"))
```

```rust
fn main() {
    let items = vec!["notes", "folders", "tags"];
    for item in &items {
        println!("Pithos Notebook has: {item}");
    }
}
```

## Tables

| Feature | Status |
| --- | --- |
| Rich editing | Supported |
| Mermaid diagrams | Supported |
| Image embedding | Supported |
| Export | Markdown, PDF |

## Horizontal rule

---

## Images

Drag and drop an image file into the editor to embed it. Images are encrypted and stored inside your vault.
"#;

// ---------------------------------------------------------------------------
// DocState — application state
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct DocState {
    pub dirty: bool,
    pub suppress_sync: bool,
    pub saved_snapshot: String,
    pub last_snapshot: String,
    pub undo_stack: Vec<String>,
    pub redo_stack: Vec<String>,
    pub fullscreen: bool,
    pub notes: Vec<NoteItem>,
    pub active_note_id: String,
    pub open_tabs: Vec<String>,
    pub search_query: String,
    pub visible_row_items: Vec<SidebarRowKind>,
    pub next_note_seq: u64,
    pub sort_order: SortOrder,
    pub folders: Vec<FolderItem>,
    pub trash: Vec<TrashItem>,
    pub theme: String,
    pub active_folder_id: Option<String>,
    pub viewing_trash: bool,
    pub zen_mode: bool,
    pub sidebar_visible: bool,
    pub custom_templates: Vec<(String, String, String)>, // (name, content, tags_csv)
    pub disabled_templates: Vec<String>,                 // names of templates hidden from picker
    pub filter_tags: Vec<String>,
    pub tag_filter_and: bool,
    pub sidebar_width: i32,
    pub spellcheck_enabled: bool,

    pub last_undo_push: std::time::Instant,
    pub assets: HashMap<String, AssetMeta>,
    pub cached_key: Option<crate::crypto::CachedKey>,
}

impl Default for DocState {
    fn default() -> Self {
        let welcome = NoteItem::new(
            "note-1".to_string(),
            "Welcome".to_string(),
            PAGE_WELCOME.to_string(),
            vec!["guide".to_string()],
        );
        let features = NoteItem::new(
            "note-2".to_string(),
            "Keyboard Shortcuts".to_string(),
            PAGE_FEATURES.to_string(),
            vec!["guide".to_string()],
        );
        let formatting = NoteItem::new(
            "note-3".to_string(),
            "Formatting Examples".to_string(),
            PAGE_FORMATTING.to_string(),
            vec!["guide".to_string(), "examples".to_string()],
        );
        Self {
            dirty: false,
            suppress_sync: false,
            saved_snapshot: welcome.content.clone(),
            last_snapshot: welcome.content.clone(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            fullscreen: false,
            notes: vec![welcome.clone(), features, formatting],
            active_note_id: welcome.id.clone(),
            open_tabs: vec![welcome.id.clone()],
            search_query: String::new(),
            visible_row_items: vec![SidebarRowKind::Note(welcome.id)],
            next_note_seq: 4,
            sort_order: SortOrder::ModifiedDesc,
            folders: Vec::new(),
            trash: Vec::new(),
            theme: "system".to_string(),
            active_folder_id: None,
            viewing_trash: false,
            zen_mode: false,
            sidebar_visible: true,
            custom_templates: Vec::new(),
            disabled_templates: Vec::new(),
            filter_tags: Vec::new(),
            tag_filter_and: false,
            sidebar_width: 300,
            spellcheck_enabled: false,

            last_undo_push: std::time::Instant::now(),
            assets: HashMap::new(),
            cached_key: None,
        }
    }
}

pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl DocState {
    /// Moves a note to the trash and updates open tabs.
    /// Returns `Some(new_active_note_id)` if the active note was trashed and a switch is needed.
    pub fn move_note_to_trash(&mut self, note_id: &str) -> Option<String> {
        if self.notes.len() <= 1 {
            return None;
        }

        let idx = self.notes.iter().position(|n| n.id == note_id)?;

        let note = self.notes.remove(idx);

        self.trash.push(TrashItem {
            id: note.id.clone(),
            name: note.name,
            content: note.content,
            tags: note.tags,
            created_at: note.created_at,
            updated_at: note.updated_at,
            deleted_at: unix_now(),
            parent_id: note.parent_id,
            versions: note.versions,
            pinned: note.pinned,
        });

        self.open_tabs.retain(|id| id != note_id);

        if self.active_note_id == note_id {
            if !self.open_tabs.is_empty() {
                Some(self.open_tabs[0].clone())
            } else {
                let fallback = self.notes.first().map(|n| n.id.clone()).unwrap_or_default();
                if !fallback.is_empty() {
                    self.open_tabs.push(fallback.clone());
                    Some(fallback)
                } else {
                    None
                }
            }
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

pub fn find_note_index(notes: &[NoteItem], id: &str) -> Option<usize> {
    notes.iter().position(|note| note.id == id)
}

pub fn note_name_exists(
    notes: &[NoteItem],
    name: &str,
    folder_id: &Option<String>,
    exclude_id: Option<&str>,
) -> bool {
    let lower = name.to_lowercase();
    notes.iter().any(|n| {
        n.parent_id == *folder_id
            && n.name.to_lowercase() == lower
            && exclude_id.is_none_or(|eid| n.id != eid)
    })
}

pub fn deduplicate_note_name(notes: &[NoteItem], base: &str, folder_id: &Option<String>) -> String {
    if !note_name_exists(notes, base, folder_id, None) {
        return base.to_string();
    }
    for i in 2.. {
        let candidate = format!("{base} ({i})");
        if !note_name_exists(notes, &candidate, folder_id, None) {
            return candidate;
        }
    }
    unreachable!()
}

pub fn folder_name_exists(
    folders: &[FolderItem],
    name: &str,
    parent_id: &Option<String>,
    exclude_id: Option<&str>,
) -> bool {
    let lower = name.to_lowercase();
    folders.iter().any(|f| {
        f.parent_id == *parent_id
            && f.name.to_lowercase() == lower
            && exclude_id.is_none_or(|eid| f.id != eid)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_note_to_trash_active_tab() {
        let mut state = DocState::default();
        let initial_count = state.notes.len();
        state.notes.push(NoteItem::new(
            "note-extra".to_string(),
            "Extra Note".to_string(),
            "Content".to_string(),
            vec![],
        ));
        state.active_note_id = "note-extra".to_string();
        state.open_tabs = vec!["note-1".to_string(), "note-extra".to_string()];

        let switch = state.move_note_to_trash("note-extra");

        assert_eq!(switch, Some("note-1".to_string()));
        assert_eq!(state.notes.len(), initial_count);
        assert_eq!(state.trash.len(), 1);
        assert_eq!(state.open_tabs, vec!["note-1".to_string()]);
    }

    #[test]
    fn test_move_note_to_trash_last_note() {
        let mut state = DocState::default();
        state.notes.retain(|n| n.id == "note-1");
        assert_eq!(state.notes.len(), 1);

        let switch = state.move_note_to_trash("note-1");

        assert_eq!(switch, None);
        assert_eq!(state.notes.len(), 1);
        assert_eq!(state.trash.len(), 0);
    }
}
