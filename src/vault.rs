use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fs, io};

// ---------------------------------------------------------------------------
// Vault data model (camelCase JSON — web-app compatible)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct VaultData {
    #[serde(default)]
    pub tree: Vec<TreeItem>,
    #[serde(default)]
    pub trash: Vec<TreeItem>,
    #[serde(default)]
    pub active_id: String,
    #[serde(default)]
    pub open_tabs: Vec<String>,
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub sort_by: String,
    #[serde(default)]
    pub sort_direction: String,
    #[serde(default)]
    pub note_versions: HashMap<String, Vec<VersionEntry>>,
    #[serde(default)]
    pub next_note_seq: u64,
    #[serde(default)]
    pub custom_templates: Vec<CustomTemplate>,
    #[serde(default)]
    pub sidebar_width: i32,
    #[serde(default)]
    pub assets: HashMap<String, AssetMeta>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssetMeta {
    pub id: String,
    pub original_name: String,
    pub mime_type: String,
    pub size: u64,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TreeItem {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TreeItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expanded: Option<bool>,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VersionEntry {
    pub ts: i64,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CustomTemplate {
    pub name: String,
    pub content: String,
    #[serde(default)]
    pub tags: String,
}

// ---------------------------------------------------------------------------
// App config  (~/.config/pithos-notebook/config.json)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub vault_path: Option<String>,
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("pithos-notebook")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn load_config() -> AppConfig {
    fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(config: &AppConfig) -> io::Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    let json = serde_json::to_string_pretty(config)
        .map_err(io::Error::other)?;
    atomic_write(&config_path(), json.as_bytes())
}

// ---------------------------------------------------------------------------
// Vault file I/O
// ---------------------------------------------------------------------------

pub fn vault_file_path(vault_folder: &str) -> PathBuf {
    Path::new(vault_folder).join("vault.json")
}

pub fn read_vault_raw(vault_folder: &str) -> io::Result<Option<String>> {
    let path = vault_file_path(vault_folder);
    if !path.exists() {
        return Ok(None);
    }
    fs::read_to_string(&path).map(Some)
}

pub fn write_vault_raw(vault_folder: &str, data: &str) -> io::Result<()> {
    fs::create_dir_all(vault_folder)?;
    atomic_write(&vault_file_path(vault_folder), data.as_bytes())
}

fn atomic_write(target: &Path, data: &[u8]) -> io::Result<()> {
    let tmp = target.with_extension("tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, target).map_err(|e| {
        let _ = fs::remove_file(&tmp); // Best-effort cleanup of temp file
        e
    })
}

pub fn assets_dir(vault_folder: &str) -> PathBuf {
    Path::new(vault_folder).join("assets")
}

pub fn write_asset(vault_folder: &str, asset_id: &str, data: &[u8]) -> io::Result<()> {
    let dir = assets_dir(vault_folder);
    fs::create_dir_all(&dir)?;
    atomic_write(&dir.join(asset_id), data)
}


// ---------------------------------------------------------------------------
// Conversion helpers: internal flat model <-> tree vault format
// ---------------------------------------------------------------------------

use crate::{DocState, FolderItem, NoteItem, NoteVersion, SortOrder, TrashItem};

pub fn sort_order_to_strings(order: SortOrder) -> (String, String) {
    match order {
        SortOrder::Manual => ("manual".into(), "manual".into()),
        SortOrder::ModifiedDesc => ("modified".into(), "desc".into()),
        SortOrder::ModifiedAsc => ("modified".into(), "asc".into()),
        SortOrder::NameAsc => ("name".into(), "asc".into()),
        SortOrder::NameDesc => ("name".into(), "desc".into()),
        SortOrder::CreatedDesc => ("created".into(), "desc".into()),
        SortOrder::CreatedAsc => ("created".into(), "asc".into()),
    }
}

pub fn parse_sort_order(sort_by: &str, direction: &str) -> SortOrder {
    match (sort_by, direction) {
        ("manual", _) => SortOrder::Manual,
        ("modified", "desc") => SortOrder::ModifiedDesc,
        ("modified", "asc") => SortOrder::ModifiedAsc,
        ("name", "asc") => SortOrder::NameAsc,
        ("name", "desc") => SortOrder::NameDesc,
        ("created", "desc") => SortOrder::CreatedDesc,
        ("created", "asc") => SortOrder::CreatedAsc,
        _ => SortOrder::ModifiedDesc,
    }
}

/// Convert the flat in-memory DocState into the tree-based vault format.
pub fn doc_state_to_vault(state: &DocState) -> VaultData {
    // Group notes and folders by parent_id for recursive tree building
    let mut notes_by_parent: HashMap<Option<String>, Vec<&NoteItem>> = HashMap::new();
    for note in &state.notes {
        notes_by_parent
            .entry(note.parent_id.clone())
            .or_default()
            .push(note);
    }
    let mut folders_by_parent: HashMap<Option<String>, Vec<&FolderItem>> = HashMap::new();
    for folder in &state.folders {
        folders_by_parent
            .entry(folder.parent_id.clone())
            .or_default()
            .push(folder);
    }
    let root_items = build_vault_tree_level(None, &notes_by_parent, &folders_by_parent);

    // Trash items
    let trash: Vec<TreeItem> = state
        .trash
        .iter()
        .map(|t| TreeItem {
            id: t.id.clone(),
            name: t.name.clone(),
            item_type: "note".to_string(),
            content: Some(t.content.clone()),
            children: None,
            expanded: None,
            created_at: t.created_at,
            updated_at: t.updated_at,
            deleted: Some(true),
            deleted_at: Some(t.deleted_at),
            tags: if t.tags.is_empty() {
                None
            } else {
                Some(t.tags.clone())
            },
            pinned: if t.pinned { Some(true) } else { None },
        })
        .collect();

    // Version history (include trashed note versions too)
    let mut note_versions: HashMap<String, Vec<VersionEntry>> = HashMap::new();
    for note in &state.notes {
        if !note.versions.is_empty() {
            let entries = note
                .versions
                .iter()
                .map(|v| VersionEntry {
                    ts: v.ts,
                    content: v.content.clone(),
                })
                .collect();
            note_versions.insert(note.id.clone(), entries);
        }
    }
    for trash_item in &state.trash {
        if !trash_item.versions.is_empty() {
            let entries = trash_item
                .versions
                .iter()
                .map(|v| VersionEntry {
                    ts: v.ts,
                    content: v.content.clone(),
                })
                .collect();
            note_versions.insert(trash_item.id.clone(), entries);
        }
    }

    let (sort_by, sort_direction) = sort_order_to_strings(state.sort_order);

    let custom_templates: Vec<CustomTemplate> = state
        .custom_templates
        .iter()
        .map(|(name, content, tags)| CustomTemplate {
            name: name.clone(),
            content: content.clone(),
            tags: tags.clone(),
        })
        .collect();

    VaultData {
        tree: root_items,
        trash,
        active_id: state.active_note_id.clone(),
        open_tabs: state.open_tabs.clone(),
        theme: state.theme.clone(),
        sort_by,
        sort_direction,
        note_versions,
        next_note_seq: state.next_note_seq,
        custom_templates,
        sidebar_width: state.sidebar_width,
        assets: state.assets.clone(),
    }
}

fn note_to_tree_item(note: &NoteItem) -> TreeItem {
    TreeItem {
        id: note.id.clone(),
        name: note.name.clone(),
        item_type: "note".to_string(),
        content: Some(note.content.clone()),
        children: None,
        expanded: None,
        created_at: note.created_at,
        updated_at: note.updated_at,
        deleted: None,
        deleted_at: None,
        tags: if note.tags.is_empty() {
            None
        } else {
            Some(note.tags.clone())
        },
        pinned: if note.pinned { Some(true) } else { None },
    }
}

fn build_vault_tree_level(
    parent_id: Option<&str>,
    notes_by_parent: &HashMap<Option<String>, Vec<&NoteItem>>,
    folders_by_parent: &HashMap<Option<String>, Vec<&FolderItem>>,
) -> Vec<TreeItem> {
    let mut items = Vec::new();
    let key = parent_id.map(|s| s.to_string());

    // Folders first
    if let Some(child_folders) = folders_by_parent.get(&key) {
        for folder in child_folders {
            let children =
                build_vault_tree_level(Some(&folder.id), notes_by_parent, folders_by_parent);
            items.push(TreeItem {
                id: folder.id.clone(),
                name: folder.name.clone(),
                item_type: "folder".to_string(),
                content: None,
                children: Some(children),
                expanded: Some(folder.expanded),
                created_at: folder.created_at,
                updated_at: folder.updated_at,
                deleted: None,
                deleted_at: None,
                tags: None,
                pinned: None,
            });
        }
    }

    // Notes at this level
    if let Some(child_notes) = notes_by_parent.get(&key) {
        for note in child_notes {
            items.push(note_to_tree_item(note));
        }
    }

    items
}

/// Convert tree-based vault format back into flat in-memory DocState.
pub fn vault_to_doc_state(vault: VaultData) -> DocState {
    let mut notes: Vec<NoteItem> = Vec::new();
    let mut folders: Vec<FolderItem> = Vec::new();

    flatten_tree(
        &vault.tree,
        None,
        &mut notes,
        &mut folders,
        &vault.note_versions,
    );

    let trash: Vec<TrashItem> = vault
        .trash
        .iter()
        .map(|item| {
            let versions = vault.note_versions.get(&item.id)
                .map(|entries| entries.iter().map(|e| NoteVersion { ts: e.ts, content: e.content.clone() }).collect())
                .unwrap_or_default();
            TrashItem {
                id: item.id.clone(),
                name: item.name.clone(),
                content: item.content.clone().unwrap_or_default(),
                tags: item.tags.clone().unwrap_or_default(),
                created_at: item.created_at,
                updated_at: item.updated_at,
                deleted_at: item.deleted_at.unwrap_or(item.updated_at),
                parent_id: None, // parent folder may be gone; restore checks
                versions,
                pinned: item.pinned.unwrap_or(false),
            }
        })
        .collect();

    let sort_order = parse_sort_order(&vault.sort_by, &vault.sort_direction);

    // Validate active_id
    let active_id = if !vault.active_id.is_empty()
        && notes.iter().any(|n| n.id == vault.active_id)
    {
        vault.active_id.clone()
    } else {
        notes.first().map(|n| n.id.clone()).unwrap_or_default()
    };

    let open_tabs = if vault.open_tabs.is_empty() {
        vec![active_id.clone()]
    } else {
        // Filter tabs to only existing notes
        let valid: Vec<String> = vault
            .open_tabs
            .into_iter()
            .filter(|id| notes.iter().any(|n| n.id == *id))
            .collect();
        if valid.is_empty() {
            vec![active_id.clone()]
        } else {
            valid
        }
    };

    // Re-populate empty built-in guide notes so existing vaults get updated content
    use crate::state::{PAGE_WELCOME, PAGE_FEATURES, PAGE_FORMATTING};
    let guide_defaults: &[(&str, &str)] = &[
        ("note-1", PAGE_WELCOME),
        ("note-2", PAGE_FEATURES),
        ("note-3", PAGE_FORMATTING),
    ];
    for (id, default_content) in guide_defaults {
        if let Some(note) = notes.iter_mut().find(|n| n.id == *id) {
            if note.content.trim().is_empty() {
                note.content = default_content.to_string();
            }
        }
    }

    let saved_content = notes
        .iter()
        .find(|n| n.id == active_id)
        .map(|n| n.content.clone())
        .unwrap_or_default();

    let theme = if vault.theme.is_empty() {
        "system".to_string()
    } else {
        vault.theme
    };

    // Ensure seq is higher than any existing note/trash ID to avoid collisions
    let max_existing = notes.iter().map(|n| &n.id)
        .chain(trash.iter().map(|t| &t.id))
        .filter_map(|id| id.strip_prefix("note-").and_then(|s| s.parse::<u64>().ok()))
        .max()
        .unwrap_or(0);
    let next_note_seq = vault.next_note_seq.max(max_existing + 1).max(1);

    DocState {
        dirty: false,
        suppress_sync: false,
        saved_snapshot: saved_content.clone(),
        last_snapshot: saved_content,
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
        fullscreen: false,
        notes,
        folders,
        trash,
        active_note_id: active_id,
        open_tabs,
        search_query: String::new(),
        visible_row_items: Vec::new(),
        next_note_seq,
        sort_order,
        theme,
        active_folder_id: None,
        viewing_trash: false,
        zen_mode: false,
        sidebar_visible: true,
        custom_templates: vault.custom_templates.into_iter().map(|t| (t.name, t.content, t.tags)).collect(),
        filter_tags: Vec::new(),
        tag_filter_and: false,
        sidebar_width: if vault.sidebar_width > 0 { vault.sidebar_width } else { 300 },

        last_undo_push: std::time::Instant::now(),
        assets: vault.assets,
        cached_key: None,
    }
}

fn flatten_tree(
    items: &[TreeItem],
    parent_id: Option<String>,
    notes: &mut Vec<NoteItem>,
    folders: &mut Vec<FolderItem>,
    versions_map: &HashMap<String, Vec<VersionEntry>>,
) {
    for item in items {
        if item.item_type == "folder" {
            folders.push(FolderItem {
                id: item.id.clone(),
                name: item.name.clone(),
                expanded: item.expanded.unwrap_or(true),
                created_at: item.created_at,
                updated_at: item.updated_at,
                parent_id: parent_id.clone(),
            });
            if let Some(children) = &item.children {
                flatten_tree(
                    children,
                    Some(item.id.clone()),
                    notes,
                    folders,
                    versions_map,
                );
            }
        } else {
            let versions = versions_map
                .get(&item.id)
                .map(|entries| {
                    entries
                        .iter()
                        .map(|e| NoteVersion {
                            ts: e.ts,
                            content: e.content.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            notes.push(NoteItem {
                id: item.id.clone(),
                name: item.name.clone(),
                content: item.content.clone().unwrap_or_default(),
                tags: item.tags.clone().unwrap_or_default(),
                created_at: item.created_at,
                updated_at: item.updated_at,
                versions,
                file_path: None,
                parent_id: parent_id.clone(),
                pinned: item.pinned.unwrap_or(false),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocState, NoteItem, FolderItem, SortOrder};
    use std::collections::HashMap;

    #[test]
    fn test_doc_state_to_vault() {
        let mut state = DocState::default();
        state.notes.push(NoteItem::new(
            "test-note-1".to_string(),
            "Test Note".to_string(),
            "Hello World".to_string(),
            vec!["test".to_string()],
        ));
        state.folders.push(FolderItem {
            id: "folder-1".to_string(),
            name: "Test Folder".to_string(),
            expanded: true,
            created_at: 1000,
            updated_at: 1000,
            parent_id: None,
        });
        state.notes.push(NoteItem {
            id: "note-in-folder".to_string(),
            name: "Child Note".to_string(),
            content: "Child Data".to_string(),
            tags: vec![],
            created_at: 2000,
            updated_at: 2000,
            versions: vec![],
            file_path: None,
            parent_id: Some("folder-1".to_string()),
            pinned: true,
        });
        
        state.active_note_id = "test-note-1".to_string();
        state.open_tabs = vec!["test-note-1".to_string()];

        let vault = doc_state_to_vault(&state);

        // Check root items
        assert_eq!(vault.tree.len(), 5); // 3 default notes + 1 test folder + 1 test note
        
        // Find folder and check children
        let folder = vault.tree.iter().find(|i| i.id == "folder-1").unwrap();
        assert_eq!(folder.item_type, "folder");
        let children = folder.children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id, "note-in-folder");
        assert_eq!(children[0].pinned, Some(true));

        assert_eq!(vault.active_id, "test-note-1");
        assert_eq!(vault.open_tabs.len(), 1);
    }

    #[test]
    fn test_vault_to_doc_state() {
        let tree = vec![TreeItem {
            id: "f1".to_string(),
            name: "Folder 1".to_string(),
            item_type: "folder".to_string(),
            content: None,
            children: Some(vec![TreeItem {
                id: "n1".to_string(),
                name: "Note 1".to_string(),
                item_type: "note".to_string(),
                content: Some("Note 1 Content".to_string()),
                children: None,
                expanded: None,
                created_at: 123,
                updated_at: 456,
                deleted: None,
                deleted_at: None,
                tags: Some(vec!["t1".to_string()]),
                pinned: Some(true),
            }]),
            expanded: Some(true),
            created_at: 100,
            updated_at: 100,
            deleted: None,
            deleted_at: None,
            tags: None,
            pinned: None,
        }];

        let trash = vec![TreeItem {
            id: "t1".to_string(),
            name: "Trash 1".to_string(),
            item_type: "note".to_string(),
            content: Some("Trash Content".to_string()),
            children: None,
            expanded: None,
            created_at: 789,
            updated_at: 789,
            deleted: Some(true),
            deleted_at: Some(800),
            tags: None,
            pinned: None,
        }];

        let mut note_versions = HashMap::new();
        note_versions.insert("n1".to_string(), vec![VersionEntry { ts: 100, content: "Old".to_string() }]);

        let vault = VaultData {
            tree,
            trash,
            active_id: "n1".to_string(),
            open_tabs: vec!["n1".to_string()],
            theme: "dark".to_string(),
            sort_by: "name".to_string(),
            sort_direction: "asc".to_string(),
            note_versions,
            next_note_seq: 10,
            custom_templates: vec![],
            sidebar_width: 250,
            assets: HashMap::new(),
        };

        let state = vault_to_doc_state(vault);
        
        assert_eq!(state.folders.len(), 1);
        assert_eq!(state.folders[0].id, "f1");

        assert_eq!(state.notes.len(), 1);
        assert_eq!(state.notes[0].id, "n1");
        assert_eq!(state.notes[0].parent_id, Some("f1".to_string()));
        assert_eq!(state.notes[0].content, "Note 1 Content");
        assert_eq!(state.notes[0].tags, vec!["t1".to_string()]);
        assert!(state.notes[0].pinned);
        assert_eq!(state.notes[0].versions.len(), 1);

        assert_eq!(state.trash.len(), 1);
        assert_eq!(state.trash[0].id, "t1");
        assert_eq!(state.trash[0].deleted_at, 800);

        assert_eq!(state.active_note_id, "n1");
        assert_eq!(state.theme, "dark");
        assert_eq!(state.sort_order, SortOrder::NameAsc);
        assert_eq!(state.sidebar_width, 250);
        assert_eq!(state.next_note_seq, 10);
    }
}
