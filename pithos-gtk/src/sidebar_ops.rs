use adw::prelude::*;
use std::collections::HashMap;
use pithos_core::state::*;
use pithos_core::search::note_matches_query;
use crate::*;

// ---------------------------------------------------------------------------
// Sidebar signal wiring
// ---------------------------------------------------------------------------

pub fn wire_sidebar_signals(
    ctx: &EditorCtx,
    search_entry: &gtk::SearchEntry,
    notes_list: &gtk::ListBox,
) {
    {
        let ctx = ctx.clone();
        search_entry.connect_search_changed(move |entry| {
            let query = entry.text().to_string();
            ctx.state.borrow_mut().search_query = query;

            // Cancel any pending debounce
            if let Some(source_id) = ctx.search_timeout_id.take() {
                source_id.remove();
            }

            let ctx_inner = ctx.clone();
            let source_id = glib::timeout_add_local_once(
                std::time::Duration::from_millis(150),
                move || {
                    ctx_inner.search_timeout_id.set(None);
                    refresh_note_list(&ctx_inner);
                },
            );
            ctx.search_timeout_id.set(Some(source_id));
        });
    }
    {
        let ctx = ctx.clone();
        notes_list.connect_row_activated(move |_, row| {
            let index = row.index();
            if index < 0 {
                return;
            }
            let viewing_trash = ctx.state.borrow().viewing_trash;
            if viewing_trash {
                return; // trash items have their own buttons
            }
            let item = {
                let state = ctx.state.borrow();
                state.visible_row_items.get(index as usize).cloned()
            };
            match item {
                Some(SidebarRowKind::Note(note_id)) => {
                    switch_to_note(&ctx, &note_id);
                }
                Some(SidebarRowKind::Folder(folder_id)) => {
                    {
                        let mut state = ctx.state.borrow_mut();
                        if let Some(f) = state.folders.iter_mut().find(|f| f.id == folder_id) {
                            f.expanded = !f.expanded;
                        }
                        state.active_folder_id = Some(folder_id);
                    }
                    refresh_note_list(&ctx);
                    trigger_vault_save(&ctx);
                }
                None => {}
            }
        });
    }
    // Context menu on note rows (right-click)
    {
        let ctx = ctx.clone();
        let click = gtk::GestureClick::builder()
            .button(3) // right-click
            .build();
        click.connect_released(move |gesture, _, x, y| {
            let viewing_trash = ctx.state.borrow().viewing_trash;
            if viewing_trash {
                return;
            }
            let Some(widget) = gesture.widget() else {
                return;
            };
            let Some(list_box) = widget.downcast_ref::<gtk::ListBox>() else {
                return;
            };
            let Some(row) = list_box.row_at_y(y as i32) else {
                return;
            };
            let index = row.index();
            if index < 0 {
                return;
            }
            let item = {
                let state = ctx.state.borrow();
                state.visible_row_items.get(index as usize).cloned()
            };
            match item {
                Some(SidebarRowKind::Note(note_id)) => {
                    show_note_context_menu(&ctx, &note_id, x, y, &widget);
                }
                Some(SidebarRowKind::Folder(folder_id)) => {
                    show_folder_context_menu(&ctx, &folder_id, x, y, &widget);
                }
                None => {}
            }
        });
        notes_list.add_controller(click);
    }
    // Drag-and-drop reorder / re-parent in sidebar tree
    {
        let ctx = ctx.clone();
        let drop = gtk::DropTarget::new(String::static_type(), gdk::DragAction::MOVE);
        drop.connect_drop(move |target, value, _, y| {
            let Ok(payload) = value.get::<String>() else {
                return false;
            };
            let Some(dragged) = parse_sidebar_drag_payload(&payload) else {
                return false;
            };

            let target_item = {
                let Some(widget) = target.widget() else {
                    return false;
                };
                let Some(list_box) = widget.downcast_ref::<gtk::ListBox>() else {
                    return false;
                };
                list_box
                    .row_at_y(y as i32)
                    .and_then(|row| {
                        let index = row.index();
                        if index < 0 {
                            None
                        } else {
                            Some(index as usize)
                        }
                    })
                    .and_then(|index| ctx.state.borrow().visible_row_items.get(index).cloned())
            };

            let changed = apply_sidebar_drop(&ctx, dragged, target_item);
            if changed {
                refresh_note_list(&ctx);
                refresh_tabs(&ctx);
                trigger_vault_save(&ctx);
            }
            changed
        });
        notes_list.add_controller(drop);
    }
}

// ---------------------------------------------------------------------------
// Drag-and-drop helpers
// ---------------------------------------------------------------------------

fn parse_sidebar_drag_payload(payload: &str) -> Option<SidebarRowKind> {
    if let Some(id) = payload.strip_prefix("note:") {
        return Some(SidebarRowKind::Note(id.to_string()));
    }
    if let Some(id) = payload.strip_prefix("folder:") {
        return Some(SidebarRowKind::Folder(id.to_string()));
    }
    None
}

fn move_vec_item<T>(items: &mut Vec<T>, from: usize, mut to: usize) {
    if from >= items.len() || to > items.len() || from == to {
        return;
    }
    let item = items.remove(from);
    if from < to {
        to = to.saturating_sub(1);
    }
    if to > items.len() {
        to = items.len();
    }
    items.insert(to, item);
}

fn would_create_folder_cycle(state: &DocState, folder_id: &str, new_parent_id: Option<&str>) -> bool {
    let mut current = new_parent_id.map(str::to_string);
    while let Some(parent) = current {
        if parent == folder_id {
            return true;
        }
        current = state
            .folders
            .iter()
            .find(|f| f.id == parent)
            .and_then(|f| f.parent_id.clone());
    }
    false
}

fn move_note_to_parent(
    state: &mut DocState,
    note_id: &str,
    new_parent_id: Option<String>,
    target_note_id: Option<&str>,
) -> bool {
    let Some(mut from_idx) = state.notes.iter().position(|n| n.id == note_id) else {
        return false;
    };

    let mut changed = false;
    if state.notes[from_idx].parent_id != new_parent_id {
        state.notes[from_idx].parent_id = new_parent_id.clone();
        state.notes[from_idx].updated_at = unix_now();
        changed = true;
    }

    if let Some(target_note_id) = target_note_id {
        if target_note_id != note_id {
            if let Some(target_idx) = state.notes.iter().position(|n| n.id == target_note_id) {
                move_vec_item(&mut state.notes, from_idx, target_idx);
                changed = true;
            }
        }
    } else {
        from_idx = state
            .notes
            .iter()
            .position(|n| n.id == note_id)
            .unwrap_or(from_idx);
        let sibling_tail = state
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.id != note_id && n.parent_id == new_parent_id)
            .map(|(i, _)| i)
            .next_back()
            .map_or(state.notes.len(), |i| i + 1);
        if from_idx != sibling_tail {
            move_vec_item(&mut state.notes, from_idx, sibling_tail);
            changed = true;
        }
    }

    changed
}

fn move_folder_to_parent(
    state: &mut DocState,
    folder_id: &str,
    new_parent_id: Option<String>,
    target_folder_id: Option<&str>,
) -> bool {
    if would_create_folder_cycle(state, folder_id, new_parent_id.as_deref()) {
        return false;
    }

    let Some(mut from_idx) = state.folders.iter().position(|f| f.id == folder_id) else {
        return false;
    };

    let mut changed = false;
    if state.folders[from_idx].parent_id != new_parent_id {
        state.folders[from_idx].parent_id = new_parent_id.clone();
        state.folders[from_idx].updated_at = unix_now();
        changed = true;
    }

    if let Some(target_folder_id) = target_folder_id {
        if target_folder_id != folder_id {
            if let Some(target_idx) = state.folders.iter().position(|f| f.id == target_folder_id) {
                move_vec_item(&mut state.folders, from_idx, target_idx);
                changed = true;
            }
        }
    } else {
        from_idx = state
            .folders
            .iter()
            .position(|f| f.id == folder_id)
            .unwrap_or(from_idx);
        let sibling_tail = state
            .folders
            .iter()
            .enumerate()
            .filter(|(_, f)| f.id != folder_id && f.parent_id == new_parent_id)
            .map(|(i, _)| i)
            .next_back()
            .map_or(state.folders.len(), |i| i + 1);
        if from_idx != sibling_tail {
            move_vec_item(&mut state.folders, from_idx, sibling_tail);
            changed = true;
        }
    }

    changed
}

fn apply_sidebar_drop(
    ctx: &EditorCtx,
    dragged: SidebarRowKind,
    target: Option<SidebarRowKind>,
) -> bool {
    let mut state = ctx.state.borrow_mut();

    if state.viewing_trash || !state.search_query.trim().is_empty() || !state.filter_tags.is_empty() {
        return false;
    }

    let changed = match dragged {
        SidebarRowKind::Note(note_id) => {
            let (new_parent, target_note) = match target {
                Some(SidebarRowKind::Folder(folder_id)) => (Some(folder_id), None),
                Some(SidebarRowKind::Note(target_note_id)) => {
                    let parent = state
                        .notes
                        .iter()
                        .find(|n| n.id == target_note_id)
                        .and_then(|n| n.parent_id.clone());
                    (parent, Some(target_note_id))
                }
                None => (None, None),
            };
            move_note_to_parent(&mut state, &note_id, new_parent, target_note.as_deref())
        }
        SidebarRowKind::Folder(folder_id) => {
            let (new_parent, target_folder) = match target {
                Some(SidebarRowKind::Folder(target_folder_id)) => {
                    (Some(target_folder_id.clone()), Some(target_folder_id))
                }
                Some(SidebarRowKind::Note(target_note_id)) => {
                    let parent = state
                        .notes
                        .iter()
                        .find(|n| n.id == target_note_id)
                        .and_then(|n| n.parent_id.clone());
                    (parent, None)
                }
                None => (None, None),
            };
            move_folder_to_parent(&mut state, &folder_id, new_parent, target_folder.as_deref())
        }
    };

    if changed {
        state.sort_order = SortOrder::Manual;
    }
    changed
}

// ---------------------------------------------------------------------------
// Context menus
// ---------------------------------------------------------------------------

pub fn show_note_context_menu(
    ctx: &EditorCtx,
    note_id: &str,
    x: f64,
    y: f64,
    widget: &gtk::Widget,
) {
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
    vbox.set_margin_start(6);
    vbox.set_margin_end(6);
    vbox.set_margin_top(6);
    vbox.set_margin_bottom(6);

    let rename_btn = gtk::Button::with_label("Rename");
    rename_btn.add_css_class("flat");
    rename_btn.set_halign(gtk::Align::Fill);
    let delete_btn = gtk::Button::with_label("Move to Trash");
    delete_btn.add_css_class("flat");
    delete_btn.set_halign(gtk::Align::Fill);
    let move_btn = gtk::Button::with_label("Move to Folder\u{2026}");
    move_btn.add_css_class("flat");
    move_btn.set_halign(gtk::Align::Fill);
    let pin_btn = {
        let state = ctx.state.borrow();
        let pinned = find_note_index(&state.notes, note_id)
            .map(|i| state.notes[i].pinned)
            .unwrap_or(false);
        if pinned {
            gtk::Button::with_label("Unpin")
        } else {
            gtk::Button::with_label("Pin to Top")
        }
    };
    pin_btn.add_css_class("flat");
    pin_btn.set_halign(gtk::Align::Fill);

    vbox.append(&rename_btn);
    vbox.append(&pin_btn);
    vbox.append(&move_btn);
    vbox.append(&delete_btn);

    let popover = gtk::Popover::new();
    popover.set_child(Some(&vbox));
    popover.set_parent(widget);
    popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
    popover.connect_closed(|p| p.unparent());

    {
        let ctx = ctx.clone();
        let popover = popover.clone();
        let note_id = note_id.to_string();
        rename_btn.connect_clicked(move |_| {
            popover.popdown();
            // Switch to the note first then rename
            switch_to_note(&ctx, &note_id);
            rename_note_dialog(&ctx);
        });
    }
    {
        let ctx = ctx.clone();
        let popover = popover.clone();
        let note_id = note_id.to_string();
        delete_btn.connect_clicked(move |_| {
            popover.popdown();
            move_to_trash(&ctx, &note_id);
        });
    }
    {
        let ctx = ctx.clone();
        let popover = popover.clone();
        let note_id = note_id.to_string();
        move_btn.connect_clicked(move |_| {
            popover.popdown();
            switch_to_note(&ctx, &note_id);
            move_note_to_folder(&ctx);
        });
    }
    {
        let ctx = ctx.clone();
        let popover = popover.clone();
        let note_id = note_id.to_string();
        pin_btn.connect_clicked(move |_| {
            popover.popdown();
            {
                let mut state = ctx.state.borrow_mut();
                if let Some(i) = find_note_index(&state.notes, &note_id) {
                    state.notes[i].pinned = !state.notes[i].pinned;
                    state.notes[i].updated_at = unix_now();
                }
            }
            refresh_note_list(&ctx);
            trigger_vault_save(&ctx);
        });
    }

    popover.popup();
}

pub fn show_folder_context_menu(
    ctx: &EditorCtx,
    folder_id: &str,
    x: f64,
    y: f64,
    widget: &gtk::Widget,
) {
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
    vbox.set_margin_start(6);
    vbox.set_margin_end(6);
    vbox.set_margin_top(6);
    vbox.set_margin_bottom(6);

    let rename_btn = gtk::Button::with_label("Rename Folder");
    rename_btn.add_css_class("flat");
    rename_btn.set_halign(gtk::Align::Fill);

    let new_note_btn = gtk::Button::with_label("New Note Here");
    new_note_btn.add_css_class("flat");
    new_note_btn.set_halign(gtk::Align::Fill);

    let new_subfolder_btn = gtk::Button::with_label("New Subfolder");
    new_subfolder_btn.add_css_class("flat");
    new_subfolder_btn.set_halign(gtk::Align::Fill);

    let delete_btn = gtk::Button::with_label("Delete Folder");
    delete_btn.add_css_class("flat");
    delete_btn.add_css_class("destructive-action");
    delete_btn.set_halign(gtk::Align::Fill);

    vbox.append(&rename_btn);
    vbox.append(&new_note_btn);
    vbox.append(&new_subfolder_btn);
    vbox.append(&delete_btn);

    let popover = gtk::Popover::new();
    popover.set_child(Some(&vbox));
    popover.set_parent(widget);
    popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
    popover.connect_closed(|p| p.unparent());

    {
        let ctx = ctx.clone();
        let popover = popover.clone();
        let folder_id = folder_id.to_string();
        rename_btn.connect_clicked(move |_| {
            popover.popdown();
            rename_folder_dialog(&ctx, &folder_id);
        });
    }
    {
        let ctx = ctx.clone();
        let popover = popover.clone();
        let folder_id = folder_id.to_string();
        new_note_btn.connect_clicked(move |_| {
            popover.popdown();
            let name = {
                let state = ctx.state.borrow();
                format!("Note {}", state.next_note_seq)
            };
            create_note_in_folder(&ctx, name, format!("# {}\n\n", "Untitled"), Vec::new(), Some(folder_id.clone()));
        });
    }
    {
        let ctx = ctx.clone();
        let popover = popover.clone();
        let folder_id = folder_id.to_string();
        new_subfolder_btn.connect_clicked(move |_| {
            popover.popdown();
            create_folder(&ctx, Some(folder_id.clone()));
        });
    }
    {
        let ctx = ctx.clone();
        let popover = popover.clone();
        let folder_id = folder_id.to_string();
        delete_btn.connect_clicked(move |_| {
            popover.popdown();
            delete_folder(&ctx, &folder_id);
        });
    }

    popover.popup();
}

// ---------------------------------------------------------------------------
// Folder dialogs
// ---------------------------------------------------------------------------

pub fn rename_folder_dialog(ctx: &EditorCtx, folder_id: &str) {
    let current_name = {
        let state = ctx.state.borrow();
        state
            .folders
            .iter()
            .find(|f| f.id == folder_id)
            .map(|f| f.name.clone())
            .unwrap_or_default()
    };

    let dialog = adw::AlertDialog::new(Some("Rename Folder"), Some("Enter a new name for the folder"));

    let entry = gtk::Entry::new();
    entry.set_text(&current_name);
    entry.set_activates_default(true);
    dialog.set_extra_child(Some(&entry));

    dialog.add_response("cancel", "Cancel");
    dialog.add_response("rename", "Rename");
    dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("rename"));
    dialog.set_close_response("cancel");

    let window = ctx.window.clone();
    let ctx = ctx.clone();
    let folder_id = folder_id.to_string();
    dialog.connect_response(None, move |dlg, response| {
        let new_name = entry.text().trim().to_string();
        dlg.set_extra_child(gtk::Widget::NONE);
        if response == "rename"
            && !new_name.is_empty() {
                let conflict = {
                    let state = ctx.state.borrow();
                    let parent = state
                        .folders
                        .iter()
                        .find(|f| f.id == folder_id)
                        .and_then(|f| f.parent_id.clone());
                    folder_name_exists(&state.folders, &new_name, &parent, Some(&folder_id))
                };
                if conflict {
                    send_toast(&ctx, "A folder with that name already exists here");
                    return;
                }
                {
                    let mut state = ctx.state.borrow_mut();
                    if let Some(f) = state.folders.iter_mut().find(|f| f.id == folder_id) {
                        f.name = new_name;
                        f.updated_at = unix_now();
                    }
                }
                refresh_note_list(&ctx);
                trigger_vault_save(&ctx);
            }
    });
    dialog.present(Some(&window));
}

pub fn delete_folder(ctx: &EditorCtx, folder_id: &str) {
    let dialog = adw::AlertDialog::new(
        Some("Delete Folder?"),
        Some("Notes inside this folder will be moved to the root level"),
    );

    dialog.add_response("cancel", "Cancel");
    dialog.add_response("delete", "Delete");
    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");

    let window = ctx.window.clone();
    let ctx = ctx.clone();
    let folder_id = folder_id.to_string();
    dialog.connect_response(None, move |_, response| {
        if response == "delete" {
            {
                let mut state = ctx.state.borrow_mut();
                for note in state.notes.iter_mut() {
                    if note.parent_id.as_deref() == Some(&folder_id) {
                        note.parent_id = None;
                    }
                }
                let parent_of_deleted = state
                    .folders
                    .iter()
                    .find(|f| f.id == folder_id)
                    .and_then(|f| f.parent_id.clone());
                for folder in state.folders.iter_mut() {
                    if folder.parent_id.as_deref() == Some(&folder_id) {
                        folder.parent_id = parent_of_deleted.clone();
                    }
                }
                state.folders.retain(|f| f.id != folder_id);
                if state.active_folder_id.as_deref() == Some(&folder_id) {
                    state.active_folder_id = None;
                }
            }
            refresh_note_list(&ctx);
            trigger_vault_save(&ctx);
        }
    });
    dialog.present(Some(&window));
}

// ---------------------------------------------------------------------------
// Note list rendering
// ---------------------------------------------------------------------------

/// Unified sort comparator — used by both owned slices and reference slices.
fn note_sort_cmp<T: NoteSortable>(a: &T, b: &T, sort_order: SortOrder) -> std::cmp::Ordering {
    match sort_order {
        SortOrder::Manual => std::cmp::Ordering::Equal,
        SortOrder::ModifiedDesc => b.updated_at().cmp(&a.updated_at()),
        SortOrder::ModifiedAsc => a.updated_at().cmp(&b.updated_at()),
        SortOrder::NameAsc => a.name().to_lowercase().cmp(&b.name().to_lowercase()),
        SortOrder::NameDesc => b.name().to_lowercase().cmp(&a.name().to_lowercase()),
        SortOrder::CreatedDesc => b.created_at().cmp(&a.created_at()),
        SortOrder::CreatedAsc => a.created_at().cmp(&b.created_at()),
    }
}

trait NoteSortable {
    fn name(&self) -> &str;
    fn updated_at(&self) -> i64;
    fn created_at(&self) -> i64;
}

impl NoteSortable for NoteSummary {
    fn name(&self) -> &str { &self.name }
    fn updated_at(&self) -> i64 { self.updated_at }
    fn created_at(&self) -> i64 { self.created_at }
}

impl NoteSortable for &NoteSummary {
    fn name(&self) -> &str { &self.name }
    fn updated_at(&self) -> i64 { self.updated_at }
    fn created_at(&self) -> i64 { self.created_at }
}

pub fn apply_note_sort(visible: &mut [NoteSummary], sort_order: SortOrder) {
    visible.sort_by(|a, b| note_sort_cmp(a, b, sort_order));
    // Pinned notes always come first (stable sort preserves order within each group)
    visible.sort_by_key(|n| if n.pinned { 0 } else { 1 });
}

pub fn apply_note_sort_refs(notes: &mut Vec<&NoteSummary>, sort_order: SortOrder) {
    notes.sort_by(|a, b| note_sort_cmp(a, b, sort_order));
    notes.sort_by_key(|n| if n.pinned { 0 } else { 1 });
}

pub fn refresh_note_list(ctx: &EditorCtx) {
    // Collect rows first to avoid issues with non-row children (e.g. popovers)
    let mut rows = Vec::new();
    let mut child = ctx.notes_list.first_child();
    while let Some(c) = child {
        let next = c.next_sibling();
        if c.downcast_ref::<gtk::ListBoxRow>().is_some() {
            rows.push(c);
        }
        child = next;
    }
    for row in rows {
        ctx.notes_list.remove(&row);
    }

    let (mut visible, active_id, sort_order, search_query, highlight_term, folders, filter_active) = {
        let state = ctx.state.borrow();
        let raw_query = state.search_query.clone();
        let filter_tags = state.filter_tags.clone();
        let tag_filter_and = state.tag_filter_and;
        let has_search = !raw_query.trim().is_empty();
        let filter_active = has_search || !filter_tags.is_empty();

        let highlight_term = if has_search {
            raw_query.trim().to_lowercase()
        } else {
            String::new()
        };

        let visible: Vec<NoteSummary> = state
            .notes
            .iter()
            .filter(|note| {
                let matches_search = if has_search {
                    note_matches_query(note, &raw_query)
                } else {
                    true
                };
                let matches_tags = if filter_tags.is_empty() {
                    true
                } else if tag_filter_and {
                    filter_tags.iter().all(|ft| note.tags.contains(ft))
                } else {
                    filter_tags.iter().any(|ft| note.tags.contains(ft))
                };
                matches_search && matches_tags
            })
            .map(|note| NoteSummary {
                id: note.id.clone(),
                name: note.name.clone(),
                content_snippet: make_search_snippet(&note.content, &highlight_term),
                tags: note.tags.clone(),
                created_at: note.created_at,
                updated_at: note.updated_at,
                pinned: note.pinned,
                parent_id: note.parent_id.clone(),
            })
            .collect();

        (
            visible,
            state.active_note_id.clone(),
            state.sort_order,
            raw_query,
            highlight_term,
            state.folders.clone(),
            filter_active,
        )
    };

    apply_note_sort(&mut visible, sort_order);

    let mut row_items: Vec<SidebarRowKind> = Vec::new();

    if filter_active {
        // Flat mode during search — no folder hierarchy
        for note in &visible {
            let row = build_note_row(ctx, note, &active_id, &highlight_term, 0);
            ctx.notes_list.append(&row);
            row_items.push(SidebarRowKind::Note(note.id.clone()));
        }
    } else {
        // Tree mode — group notes and folders by parent_id
        let mut notes_by_parent: HashMap<Option<String>, Vec<&NoteSummary>> = HashMap::new();
        for note in &visible {
            notes_by_parent
                .entry(note.parent_id.clone())
                .or_default()
                .push(note);
        }
        // Sort notes within each group
        for group in notes_by_parent.values_mut() {
            apply_note_sort_refs(group, sort_order);
        }

        let mut folders_by_parent: HashMap<Option<String>, Vec<&FolderItem>> = HashMap::new();
        for folder in &folders {
            folders_by_parent
                .entry(folder.parent_id.clone())
                .or_default()
                .push(folder);
        }
        // Sort folders alphabetically unless manual ordering is active.
        if sort_order != SortOrder::Manual {
            for group in folders_by_parent.values_mut() {
                group.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }
        }

        render_tree_level(
            ctx,
            None,
            0,
            &folders_by_parent,
            &notes_by_parent,
            &active_id,
            &search_query,
            &mut row_items,
        );
    }

    ctx.state.borrow_mut().visible_row_items = row_items;

    // Toggle empty state
    let has_notes = !ctx.state.borrow().notes.is_empty();
    ctx.content_stack.set_visible_child_name(if has_notes { "editor" } else { "empty" });
}

// ---------------------------------------------------------------------------
// Search highlighting
// ---------------------------------------------------------------------------

pub fn highlight_search(text: &str, query: &str) -> String {
    if query.is_empty() {
        return glib::markup_escape_text(text).to_string();
    }
    let lower = text.to_lowercase();
    // Map each byte offset in `lower` back to the corresponding byte offset in `text`.
    let mut lower_to_orig: Vec<usize> = Vec::with_capacity(lower.len() + 1);
    for (orig_byte, orig_ch) in text.char_indices() {
        let lc: String = orig_ch.to_lowercase().collect();
        for _ in 0..lc.len() {
            lower_to_orig.push(orig_byte);
        }
    }
    lower_to_orig.push(text.len());

    let lower_query = query.to_lowercase();
    let mut result = String::new();
    let mut pos = 0usize; // byte position in `lower`
    let mut orig_pos = 0usize; // byte position in `text`
    while let Some(idx) = lower[pos..].find(&lower_query) {
        let abs = pos + idx;
        let match_end = abs + lower_query.len();
        let orig_start = lower_to_orig[abs];
        let orig_end = lower_to_orig[match_end.min(lower_to_orig.len() - 1)];
        result.push_str(&glib::markup_escape_text(&text[orig_pos..orig_start]));
        result.push_str("<b>");
        result.push_str(&glib::markup_escape_text(&text[orig_start..orig_end]));
        result.push_str("</b>");
        orig_pos = orig_end;
        pos = match_end;
    }
    result.push_str(&glib::markup_escape_text(&text[orig_pos..]));
    result
}

pub fn make_search_snippet(content: &str, query: &str) -> Option<String> {
    let query = query.trim();
    if query.is_empty() {
        return None;
    }

    let lower = content.to_lowercase();
    let lower_query = query.to_lowercase();
    let match_pos = lower.find(&lower_query)?;

    // Show context around first match while preserving UTF-8 boundaries.
    let mut start = match_pos.saturating_sub(64);
    while start < content.len() && !content.is_char_boundary(start) {
        start += 1;
    }

    let mut end = (match_pos + lower_query.len() + 96).min(content.len());
    while end > start && !content.is_char_boundary(end) {
        end -= 1;
    }

    let mut snippet = content[start..end].replace('\n', " ");
    snippet = snippet.split_whitespace().collect::<Vec<_>>().join(" ");
    if snippet.is_empty() {
        return None;
    }

    if start > 0 {
        snippet.insert(0, '\u{2026}');
    }
    if end < content.len() {
        snippet.push('\u{2026}');
    }

    Some(snippet)
}

// ---------------------------------------------------------------------------
// Row builders
// ---------------------------------------------------------------------------

pub fn build_note_row(
    ctx: &EditorCtx,
    note: &NoteSummary,
    active_id: &str,
    search_query: &str,
    depth: u32,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_widget_name(&format!("note:{}", note.id));
    let row_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let indent = 8 + (depth.min(5) * 20) as i32;
    row_box.set_margin_start(indent);
    row_box.set_margin_end(8);
    row_box.set_margin_top(6);
    row_box.set_margin_bottom(6);

    let title_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);

    // Document icon for notes inside folders
    if depth > 0 {
        let doc_icon = gtk::Image::from_icon_name("document-text-symbolic");
        doc_icon.set_pixel_size(14);
        doc_icon.set_opacity(0.5);
        title_row.append(&doc_icon);
    }

    if note.pinned {
        let pin_icon = gtk::Image::from_icon_name("view-pin-symbolic");
        pin_icon.set_pixel_size(12);
        pin_icon.set_opacity(0.5);
        title_row.append(&pin_icon);
    }
    let title = gtk::Label::new(None);
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("note-row-title");
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    if note.id == active_id {
        title.add_css_class("heading");
    }
    if !search_query.is_empty() {
        let highlighted = highlight_search(&note.name, search_query);
        title.set_markup(&highlighted);
    } else {
        title.set_text(&note.name);
    }
    title_row.append(&title);
    row_box.append(&title_row);

    let subtitle = gtk::Label::new(None);
    subtitle.set_xalign(0.0);
    subtitle.add_css_class("dim-label");
    subtitle.add_css_class("caption");
    subtitle.set_ellipsize(gtk::pango::EllipsizeMode::End);
    subtitle.set_wrap(false);
    if !search_query.is_empty() {
        if let Some(snippet) = note.content_snippet.as_ref() {
            let highlighted = highlight_search(snippet, search_query);
            subtitle.set_markup(&highlighted);
        } else {
            let meta_text = format!(
                "Updated {}  \u{2022}  Created {}",
                format_ts(note.updated_at),
                format_ts(note.created_at)
            );
            subtitle.set_text(&meta_text);
        }
    } else {
        let meta_text = format!(
            "Updated {}  \u{2022}  Created {}",
            format_ts(note.updated_at),
            format_ts(note.created_at)
        );
        subtitle.set_text(&meta_text);
    }
    row_box.append(&subtitle);

    if !note.tags.is_empty() {
        let tags_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        for tag in note.tags.iter().take(4) {
            let tag_btn = gtk::Button::with_label(&format!("#{tag}"));
            tag_btn.add_css_class("flat");
            tag_btn.add_css_class("caption");
            tag_btn.set_tooltip_text(Some(&format!("Filter by #{tag}")));
            let ctx = ctx.clone();
            let tag = tag.clone();
            tag_btn.connect_clicked(move |_| {
                toggle_tag_filter(&ctx, &tag);
            });
            tags_row.append(&tag_btn);
        }
        row_box.append(&tags_row);
    }

    row.set_child(Some(&row_box));
    let drag_payload = format!("note:{}", note.id);
    let drag = gtk::DragSource::builder()
        .actions(gdk::DragAction::MOVE)
        .build();
    drag.connect_prepare(move |_, _, _| {
        Some(gdk::ContentProvider::for_value(&drag_payload.to_value()))
    });
    row.add_controller(drag);
    row
}

pub fn build_folder_row(folder: &FolderItem, depth: u32, note_count: usize) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_widget_name(&format!("folder:{}", folder.id));
    let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let indent = 8 + (depth.min(5) * 20) as i32;
    row_box.set_margin_start(indent);
    row_box.set_margin_end(8);
    row_box.set_margin_top(4);
    row_box.set_margin_bottom(4);

    // Chevron icon
    let chevron_name = if folder.expanded {
        "pan-down-symbolic"
    } else {
        "pan-end-symbolic"
    };
    let chevron = gtk::Image::from_icon_name(chevron_name);
    chevron.set_pixel_size(16);
    chevron.add_css_class("folder-row-chevron");
    row_box.append(&chevron);

    // Folder icon
    let folder_icon = gtk::Image::from_icon_name("folder-symbolic");
    folder_icon.set_pixel_size(16);
    folder_icon.add_css_class("folder-row-icon");
    row_box.append(&folder_icon);

    // Folder name
    let name_label = gtk::Label::new(Some(&folder.name));
    name_label.set_xalign(0.0);
    name_label.set_hexpand(true);
    name_label.add_css_class("folder-row");
    name_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    row_box.append(&name_label);

    // Note count
    if note_count > 0 {
        let count_label = gtk::Label::new(Some(&note_count.to_string()));
        count_label.add_css_class("folder-row-count");
        count_label.add_css_class("dim-label");
        count_label.add_css_class("caption");
        row_box.append(&count_label);
    }

    row.set_child(Some(&row_box));
    let drag_payload = format!("folder:{}", folder.id);
    let drag = gtk::DragSource::builder()
        .actions(gdk::DragAction::MOVE)
        .build();
    drag.connect_prepare(move |_, _, _| {
        Some(gdk::ContentProvider::for_value(&drag_payload.to_value()))
    });
    row.add_controller(drag);
    row
}

pub fn count_descendant_notes(
    folder_id: &str,
    folders_by_parent: &HashMap<Option<String>, Vec<&FolderItem>>,
    notes_by_parent: &HashMap<Option<String>, Vec<&NoteSummary>>,
) -> usize {
    let key = Some(folder_id.to_string());
    let mut count = notes_by_parent.get(&key).map_or(0, |v| v.len());
    if let Some(child_folders) = folders_by_parent.get(&key) {
        for cf in child_folders {
            count += count_descendant_notes(&cf.id, folders_by_parent, notes_by_parent);
        }
    }
    count
}

#[allow(clippy::too_many_arguments)]
pub fn render_tree_level(
    ctx: &EditorCtx,
    parent_id: Option<&str>,
    depth: u32,
    folders_by_parent: &HashMap<Option<String>, Vec<&FolderItem>>,
    notes_by_parent: &HashMap<Option<String>, Vec<&NoteSummary>>,
    active_id: &str,
    search_query: &str,
    row_items: &mut Vec<SidebarRowKind>,
) {
    let key = parent_id.map(|s| s.to_string());

    // Render folders first at this level
    if let Some(child_folders) = folders_by_parent.get(&key) {
        for folder in child_folders {
            let note_count =
                count_descendant_notes(&folder.id, folders_by_parent, notes_by_parent);
            let row = build_folder_row(folder, depth, note_count);
            ctx.notes_list.append(&row);
            row_items.push(SidebarRowKind::Folder(folder.id.clone()));

            if folder.expanded {
                render_tree_level(
                    ctx,
                    Some(&folder.id),
                    depth + 1,
                    folders_by_parent,
                    notes_by_parent,
                    active_id,
                    search_query,
                    row_items,
                );
            }
        }
    }

    // Then render notes at this level
    if let Some(child_notes) = notes_by_parent.get(&key) {
        for note in child_notes {
            let row = build_note_row(ctx, note, active_id, search_query, depth);
            ctx.notes_list.append(&row);
            row_items.push(SidebarRowKind::Note(note.id.clone()));
        }
    }
}

// ---------------------------------------------------------------------------
// Tag filter bar
// ---------------------------------------------------------------------------

pub fn refresh_tag_filter_bar(ctx: &EditorCtx) {
    while let Some(child) = ctx.tag_filter_box.first_child() {
        ctx.tag_filter_box.remove(&child);
    }
    let (filter_tags, tag_filter_and) = {
        let state = ctx.state.borrow();
        (state.filter_tags.clone(), state.tag_filter_and)
    };
    if filter_tags.is_empty() {
        ctx.tag_filter_box.set_visible(false);
        return;
    }
    ctx.tag_filter_box.set_visible(true);

    let filter_icon = gtk::Image::from_icon_name("edit-find-symbolic");
    filter_icon.set_pixel_size(14);
    filter_icon.set_opacity(0.6);
    ctx.tag_filter_box.append(&filter_icon);

    // AND/OR toggle
    let mode_btn = gtk::Button::with_label(if tag_filter_and { "AND" } else { "OR" });
    mode_btn.add_css_class("flat");
    mode_btn.add_css_class("caption");
    mode_btn.set_tooltip_text(Some("Toggle AND/OR Filter Mode"));
    {
        let ctx = ctx.clone();
        mode_btn.connect_clicked(move |_| {
            {
                let mut state = ctx.state.borrow_mut();
                state.tag_filter_and = !state.tag_filter_and;
            }
            refresh_tag_filter_bar(&ctx);
            refresh_note_list(&ctx);
        });
    }
    ctx.tag_filter_box.append(&mode_btn);

    for tag in &filter_tags {
        let chip = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        chip.add_css_class("tag-chip");
        let lbl = gtk::Label::new(Some(&format!("#{tag}")));
        lbl.add_css_class("caption");
        chip.append(&lbl);
        let remove_btn = gtk::Button::from_icon_name("window-close-symbolic");
        remove_btn.add_css_class("flat");
        remove_btn.add_css_class("circular");
        remove_btn.set_valign(gtk::Align::Center);
        remove_btn.set_tooltip_text(Some("Remove Tag From Filter"));
        set_accessible_label(&remove_btn, "Remove tag from filter");
        {
            let ctx = ctx.clone();
            let tag = tag.clone();
            remove_btn.connect_clicked(move |_| {
                {
                    let mut state = ctx.state.borrow_mut();
                    state.filter_tags.retain(|t| t != &tag);
                }
                refresh_tag_filter_bar(&ctx);
                refresh_note_list(&ctx);
            });
        }
        chip.append(&remove_btn);
        ctx.tag_filter_box.append(&chip);
    }

    // Clear all button
    let clear_btn = gtk::Button::from_icon_name("edit-clear-symbolic");
    clear_btn.add_css_class("flat");
    clear_btn.set_tooltip_text(Some("Clear All Tag Filters"));
    set_accessible_label(&clear_btn, "Clear all tag filters");
    {
        let ctx = ctx.clone();
        clear_btn.connect_clicked(move |_| {
            {
                let mut state = ctx.state.borrow_mut();
                state.filter_tags.clear();
            }
            refresh_tag_filter_bar(&ctx);
            refresh_note_list(&ctx);
        });
    }
    ctx.tag_filter_box.append(&clear_btn);
}

pub fn toggle_tag_filter(ctx: &EditorCtx, tag: &str) {
    {
        let mut state = ctx.state.borrow_mut();
        if state.filter_tags.contains(&tag.to_string()) {
            state.filter_tags.retain(|t| t != tag);
        } else {
            state.filter_tags.push(tag.to_string());
        }
    }
    refresh_tag_filter_bar(ctx);
    refresh_note_list(ctx);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_note(name: &str, content: &str) -> NoteItem {
        NoteItem {
            id: "test".to_string(),
            name: name.to_string(),
            content: content.to_string(),
            tags: vec![],
            created_at: 0,
            updated_at: 0,
            versions: vec![],
            file_path: None,
            parent_id: None,
            pinned: false,
        }
    }

    #[test]
    fn test_single_word_search() {
        assert!(note_matches_query(&make_note("Rust notes", ""), "rust"));
        assert!(note_matches_query(&make_note("", "learning rust"), "rust"));
        assert!(!note_matches_query(&make_note("python notes", "no match"), "rust"));
    }

    #[test]
    fn test_case_insensitive() {
        assert!(note_matches_query(&make_note("RUST", ""), "rust"));
        assert!(note_matches_query(&make_note("rust", ""), "RUST"));
    }

    #[test]
    fn test_empty_query() {
        assert!(note_matches_query(&make_note("anything", "anything"), ""));
    }

    #[test]
    fn test_content_match() {
        assert!(note_matches_query(&make_note("title", "memory safety is great"), "memory safety"));
    }

    #[test]
    fn test_highlight_search_basic() {
        let result = highlight_search("Hello World", "world");
        assert!(result.contains("<b>World</b>"));
    }

    #[test]
    fn test_make_search_snippet() {
        let snippet = make_search_snippet("some long content with rust keyword here", "rust");
        assert!(snippet.is_some());
        assert!(snippet.unwrap().contains("rust"));
    }
}
