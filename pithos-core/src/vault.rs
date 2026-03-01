use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::state::*;

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
    pub disabled_templates: Vec<String>,
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
    #[serde(default)]
    pub recent_vaults: Vec<String>,
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
    let json = serde_json::to_string_pretty(config).map_err(io::Error::other)?;
    atomic_write(&config_path(), json.as_bytes())
}

/// Add a vault path to the recent vaults list (max 5, most recent first).
pub fn add_recent_vault(config: &mut AppConfig, path: &str) {
    config.recent_vaults.retain(|p| p != path);
    config.recent_vaults.insert(0, path.to_string());
    config.recent_vaults.truncate(5);
}

// ---------------------------------------------------------------------------
// Vault file I/O
// ---------------------------------------------------------------------------

pub fn vault_file_path(vault_folder: &str) -> PathBuf {
    Path::new(vault_folder).join("vault.json")
}

/// Copy vault.json to vault.json.bak before each save (best-effort).
pub fn backup_vault(vault_folder: &str) {
    let src = vault_file_path(vault_folder);
    if src.exists() {
        let _ = fs::copy(&src, src.with_extension("json.bak"));
    }
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

pub fn atomic_write(target: &Path, data: &[u8]) -> io::Result<()> {
    let tmp = target.with_extension("tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, target).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

pub fn assets_dir(vault_folder: &str) -> PathBuf {
    Path::new(vault_folder).join("assets")
}

/// Validate that an asset ID is safe for use as a filename.
pub fn is_valid_asset_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 128 {
        return false;
    }
    if id.contains('/') || id.contains('\\') || id == "." || id == ".." || id.starts_with('.') {
        return false;
    }
    id.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

pub fn write_asset(vault_folder: &str, asset_id: &str, data: &[u8]) -> io::Result<()> {
    if !is_valid_asset_id(asset_id) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid asset ID",
        ));
    }
    let dir = assets_dir(vault_folder);
    fs::create_dir_all(&dir)?;
    atomic_write(&dir.join(asset_id), data)
}

// ---------------------------------------------------------------------------
// Conversion helpers: internal flat model <-> tree vault format
// ---------------------------------------------------------------------------

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
        disabled_templates: state.disabled_templates.clone(),
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
            let versions = vault
                .note_versions
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
            TrashItem {
                id: item.id.clone(),
                name: item.name.clone(),
                content: item.content.clone().unwrap_or_default(),
                tags: item.tags.clone().unwrap_or_default(),
                created_at: item.created_at,
                updated_at: item.updated_at,
                deleted_at: item.deleted_at.unwrap_or(item.updated_at),
                parent_id: None,
                versions,
                pinned: item.pinned.unwrap_or(false),
            }
        })
        .collect();

    let sort_order = parse_sort_order(&vault.sort_by, &vault.sort_direction);

    let active_id = if !vault.active_id.is_empty() && notes.iter().any(|n| n.id == vault.active_id)
    {
        vault.active_id.clone()
    } else {
        notes.first().map(|n| n.id.clone()).unwrap_or_default()
    };

    let open_tabs = if vault.open_tabs.is_empty() {
        vec![active_id.clone()]
    } else {
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

    // Re-populate empty built-in guide notes
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

    let max_existing = notes
        .iter()
        .map(|n| &n.id)
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
        custom_templates: vault
            .custom_templates
            .into_iter()
            .map(|t| (t.name, t.content, t.tags))
            .collect(),
        disabled_templates: vault.disabled_templates,
        filter_tags: Vec::new(),
        tag_filter_and: false,
        sidebar_width: if vault.sidebar_width > 0 {
            vault.sidebar_width
        } else {
            300
        },
        spellcheck_enabled: false,

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
    use std::collections::HashMap;

    #[test]
    fn test_vault_roundtrip() {
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

        let vault = VaultData {
            tree,
            trash: vec![],
            active_id: "n1".to_string(),
            open_tabs: vec!["n1".to_string()],
            theme: "dark".to_string(),
            sort_by: "name".to_string(),
            sort_direction: "asc".to_string(),
            note_versions: HashMap::new(),
            next_note_seq: 10,
            custom_templates: vec![],
            disabled_templates: vec![],
            sidebar_width: 250,
            assets: HashMap::new(),
        };

        let state = vault_to_doc_state(vault);
        assert_eq!(state.folders.len(), 1);
        assert_eq!(state.notes.len(), 1);
        assert_eq!(state.notes[0].id, "n1");
        assert_eq!(state.notes[0].parent_id, Some("f1".to_string()));
        assert_eq!(state.sort_order, SortOrder::NameAsc);
    }
}
