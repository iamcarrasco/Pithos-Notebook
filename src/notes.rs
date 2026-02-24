use adw::prelude::*;
use crate::state::*;
use crate::*;

// ---------------------------------------------------------------------------
// Note management
// ---------------------------------------------------------------------------

pub fn create_note(ctx: &EditorCtx, name: String, content: String, tags: Vec<String>) {
    let note_id = {
        let mut state = ctx.state.borrow_mut();
        let folder = state.active_folder_id.clone();
        let unique_name = deduplicate_note_name(&state.notes, &name, &folder);
        let note_id = format!("note-{}", state.next_note_seq);
        state.next_note_seq += 1;
        let mut note = NoteItem::new(note_id.clone(), unique_name, content, tags);
        note.parent_id = folder;
        state.notes.push(note);
        note_id
    };
    switch_to_note(ctx, &note_id);
    trigger_vault_save(ctx);
}

pub fn create_note_in_folder(
    ctx: &EditorCtx,
    name: String,
    content: String,
    tags: Vec<String>,
    folder_id: Option<String>,
) {
    let id = {
        let mut state = ctx.state.borrow_mut();
        let unique_name = deduplicate_note_name(&state.notes, &name, &folder_id);
        let id = format!("note-{}", state.next_note_seq);
        state.next_note_seq += 1;
        let mut note = NoteItem::new(id.clone(), unique_name, content, tags);
        note.parent_id = folder_id;
        state.notes.push(note);
        id
    };
    switch_to_note(ctx, &id);
    trigger_vault_save(ctx);
}

pub fn new_document(ctx: &EditorCtx) {
    create_note(
        ctx,
        "Untitled Note".to_string(),
        "# Untitled Note\n\n".to_string(),
        Vec::new(),
    );
}

pub fn close_tab(ctx: &EditorCtx, note_id: &str) {
    if ctx.state.borrow().open_tabs.len() <= 1 {
        return;
    }
    // Find the TabView page for this note and ask AdwTabView to close it.
    for i in 0..ctx.tab_view.n_pages() {
        let page = ctx.tab_view.nth_page(i);
        let page_id = page
            .child()
            .downcast_ref::<gtk::Label>()
            .map(|l| l.widget_name());
        if page_id.as_ref().map(|s| s.as_str()) == Some(note_id) {
            ctx.tab_view.close_page(&page);
            return;
        }
    }
}

pub fn switch_to_note(ctx: &EditorCtx, note_id: &str) {
    let current_id = ctx.state.borrow().active_note_id.clone();
    if current_id == note_id {
        {
            let mut state = ctx.state.borrow_mut();
            if !state.open_tabs.iter().any(|id| id == note_id) {
                state.open_tabs.push(note_id.to_string());
            }
        }
        refresh_tabs(ctx);
        refresh_note_list(ctx);
        refresh_tags(ctx);
        return;
    }

    let outgoing_markdown = current_markdown(ctx);
    let incoming_markdown = {
        let mut state = ctx.state.borrow_mut();

        if let Some(index) = find_note_index(&state.notes, &state.active_note_id) {
            if state.notes[index].content != outgoing_markdown {
                let previous_content = state.notes[index].content.clone();
                push_snapshot(&mut state.notes[index], previous_content);
                state.notes[index].content = outgoing_markdown.clone();
                state.notes[index].updated_at = unix_now();
            }
        }

        if !state.open_tabs.iter().any(|id| id == note_id) {
            state.open_tabs.push(note_id.to_string());
        }
        state.active_note_id = note_id.to_string();

        find_note_index(&state.notes, note_id)
            .map(|index| state.notes[index].content.clone())
            .unwrap_or_else(|| "# Untitled Note\n\n".to_string())
    };

    load_document(ctx, &incoming_markdown, None);
    refresh_tabs(ctx);
    refresh_note_list(ctx);
    refresh_tags(ctx);
}

pub fn open_or_create_daily_note(ctx: &EditorCtx) {
    let date = if let Ok(now) = glib::DateTime::now_local() {
        now.format("%Y-%m-%d")
            .map(|value| value.to_string())
            .unwrap_or_else(|_| "Daily".to_string())
    } else {
        "Daily".to_string()
    };
    let note_name = format!("{date} - Daily Note");

    let existing = {
        let state = ctx.state.borrow();
        state
            .notes
            .iter()
            .find(|note| note.name == note_name)
            .map(|note| note.id.clone())
    };

    if let Some(note_id) = existing {
        switch_to_note(ctx, &note_id);
        return;
    }

    let content = format!("# {note_name}\n\n## Tasks\n\n- [ ] \n\n## Notes\n\n");
    create_note(ctx, note_name, content, vec!["daily".to_string()]);
}

// ---------------------------------------------------------------------------
// Tabs
// ---------------------------------------------------------------------------

pub fn refresh_tabs(ctx: &EditorCtx) {
    ctx.state.borrow_mut().suppress_sync = true;

    let (tabs, active_id, note_names) = {
        let state = ctx.state.borrow();
        let names: Vec<(String, String)> = state
            .notes
            .iter()
            .map(|n| (n.id.clone(), n.name.clone()))
            .collect();
        (
            state.open_tabs.clone(),
            state.active_note_id.clone(),
            names,
        )
    };

    let lookup_name = |id: &str| -> String {
        note_names
            .iter()
            .find(|(nid, _)| nid == id)
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| "Untitled".to_string())
    };

    // Helper: read the note ID stored on a page's child label widget name.
    let page_note_id = |page: &adw::TabPage| -> Option<String> {
        page.child()
            .downcast_ref::<gtk::Label>()
            .map(|l| l.widget_name().to_string())
            .filter(|s| !s.is_empty())
    };

    // --- Reuse existing page slots; add/remove only what changed. ---

    let desired = tabs.len() as i32;
    let current = ctx.tab_view.n_pages();

    // 1. Reuse slots that already exist — overwrite their note ID + title.
    for i in 0..desired.min(current) {
        let page = ctx.tab_view.nth_page(i);
        let tab_id = &tabs[i as usize];
        if let Some(label) = page.child().downcast_ref::<gtk::Label>() {
            label.set_widget_name(tab_id);
        }
        page.set_title(&lookup_name(tab_id));
    }

    // 2. If we need more pages than we have, append new ones.
    for i in current..desired {
        let tab_id = &tabs[i as usize];
        let dummy = gtk::Label::new(None);
        dummy.set_widget_name(tab_id);
        let page = ctx.tab_view.append(&dummy);
        page.set_title(&lookup_name(tab_id));
    }

    // 3. If we have more pages than needed, close excess from the end.
    //    suppress_sync is already true, so close-page handler confirms immediately.
    if desired < current {
        for _ in desired..current {
            let last = ctx.tab_view.nth_page(ctx.tab_view.n_pages() - 1);
            ctx.tab_view.close_page(&last);
        }
    }

    // 4. Select the page matching the active note.
    for i in 0..ctx.tab_view.n_pages() {
        let page = ctx.tab_view.nth_page(i);
        if page_note_id(&page).as_deref() == Some(&active_id) {
            ctx.tab_view.set_selected_page(&page);
            break;
        }
    }

    ctx.state.borrow_mut().suppress_sync = false;
}

// ---------------------------------------------------------------------------
// Tags
// ---------------------------------------------------------------------------

pub fn refresh_tags(ctx: &EditorCtx) {
    while let Some(child) = ctx.tags_box.first_child() {
        ctx.tags_box.remove(&child);
    }
    ctx.tag_entry.set_sensitive(true);
    ctx.tag_entry
        .set_placeholder_text(Some("add tag and press Enter"));

    let tags = {
        let state = ctx.state.borrow();
        find_note_index(&state.notes, &state.active_note_id)
            .map(|index| state.notes[index].tags.clone())
            .unwrap_or_default()
    };

    for tag in tags {
        let tag_btn = gtk::Button::with_label(&format!("#{tag} \u{00d7}"));
        tag_btn.add_css_class("flat");
        tag_btn.add_css_class("toolbar-pill");
        let ctx_for_click = ctx.clone();
        tag_btn.connect_clicked(move |_| remove_tag_from_active_note(&ctx_for_click, &tag));
        ctx.tags_box.insert(&tag_btn, -1);
    }
}

pub fn add_tag_to_active_note(ctx: &EditorCtx, tag: String) {
    {
        let mut state = ctx.state.borrow_mut();
        if let Some(index) = find_note_index(&state.notes, &state.active_note_id) {
            if !state.notes[index]
                .tags
                .iter()
                .any(|existing| existing == &tag)
            {
                state.notes[index].tags.push(tag);
                state.notes[index].updated_at = unix_now();
            }
        }
    }
    refresh_note_list(ctx);
    refresh_tags(ctx);
    trigger_vault_save(ctx);
}

pub fn remove_tag_from_active_note(ctx: &EditorCtx, tag: &str) {
    {
        let mut state = ctx.state.borrow_mut();
        if let Some(index) = find_note_index(&state.notes, &state.active_note_id) {
            state.notes[index].tags.retain(|existing| existing != tag);
            state.notes[index].updated_at = unix_now();
        }
    }
    refresh_note_list(ctx);
    refresh_tags(ctx);
    trigger_vault_save(ctx);
}

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

pub fn save_manual_snapshot(ctx: &EditorCtx) {
    let markdown = current_markdown(ctx);
    {
        let mut state = ctx.state.borrow_mut();
        if let Some(index) = find_note_index(&state.notes, &state.active_note_id) {
            push_snapshot(&mut state.notes[index], markdown.clone());
            state.notes[index].content = markdown;
            state.notes[index].updated_at = unix_now();
        }
    }
    trigger_vault_save(ctx);
    send_toast(ctx, "Snapshot saved");
}

pub fn restore_snapshot(ctx: &EditorCtx, version_idx: usize) {
    let restored = {
        let mut state = ctx.state.borrow_mut();
        if let Some(note_idx) = find_note_index(&state.notes, &state.active_note_id) {
            if let Some(version) = state.notes[note_idx].versions.get(version_idx).cloned() {
                state.notes[note_idx].content = version.content.clone();
                state.notes[note_idx].updated_at = unix_now();
                Some(version.content)
            } else {
                None
            }
        } else {
            None
        }
    };

    if let Some(markdown) = restored {
        load_document(ctx, &markdown, None);
        refresh_note_list(ctx);
        refresh_tabs(ctx);
        refresh_tags(ctx);
    }
}

pub fn restore_latest_snapshot(ctx: &EditorCtx) {
    let latest = {
        let state = ctx.state.borrow();
        find_note_index(&state.notes, &state.active_note_id)
            .and_then(|note_idx| state.notes[note_idx].versions.len().checked_sub(1))
    };
    if let Some(version_idx) = latest {
        restore_snapshot(ctx, version_idx);
    }
}

// ---------------------------------------------------------------------------
// Note deletion / trash
// ---------------------------------------------------------------------------

pub fn delete_note(ctx: &EditorCtx) {
    let note_count = ctx.state.borrow().notes.len();
    if note_count <= 1 {
        show_info(
            &ctx.window,
            "Cannot delete",
            "You must have at least one note.",
        );
        return;
    }

    let (note_name, note_id) = {
        let state = ctx.state.borrow();
        let name = find_note_index(&state.notes, &state.active_note_id)
            .map(|i| state.notes[i].name.clone())
            .unwrap_or_else(|| "Untitled".to_string());
        (name, state.active_note_id.clone())
    };

    let dialog = adw::AlertDialog::new(
        Some(&format!("Move \"{note_name}\" to Trash?")),
        Some("You can restore it later from the Trash."),
    );
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("trash", "Move to Trash");
    dialog.set_response_appearance("trash", adw::ResponseAppearance::Destructive);
    dialog.set_close_response("cancel");

    let window = ctx.window.clone();
    let ctx = ctx.clone();
    dialog.connect_response(None, move |_, response| {
        if response == "trash" {
            move_to_trash(&ctx, &note_id);
        }
    });
    dialog.present(Some(&window));
}

pub fn move_to_trash(ctx: &EditorCtx, note_id: &str) {
    {
        let state = ctx.state.borrow();
        if state.notes.len() <= 1 {
            drop(state);
            show_info(&ctx.window, "Cannot Delete", "You must keep at least one note.");
            return;
        }
    }

    let note_name = {
        let state = ctx.state.borrow();
        state
            .notes
            .iter()
            .find(|n| n.id == note_id)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| "this note".into())
    };

    let dialog = adw::AlertDialog::builder()
        .heading("Move to Trash?")
        .body(format!(
            "\u{201c}{note_name}\u{201d} will be moved to the trash. You can restore it later."
        ))
        .build();
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("trash", "Move to Trash");
    dialog.set_response_appearance("trash", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");

    let window = ctx.window.clone();
    let ctx = ctx.clone();
    let note_id = note_id.to_string();
    dialog.choose(
        Some(&window),
        None::<&gtk::gio::Cancellable>,
        move |response| {
            if response == "trash" {
                do_move_to_trash(&ctx, &note_id);
            }
        },
    );
}

fn do_move_to_trash(ctx: &EditorCtx, note_id: &str) {
    let switch_to_opt = {
        let mut state = ctx.state.borrow_mut();
        state.move_note_to_trash(note_id)
    };

    if let Some(switch_to) = switch_to_opt {
        switch_to_note(ctx, &switch_to);
    } else {
        refresh_tabs(ctx);
        refresh_note_list(ctx);
    }
    perform_vault_save_async(ctx, false);
}

pub fn restore_from_trash(ctx: &EditorCtx, trash_id: &str) {
    let note_id = {
        let mut state = ctx.state.borrow_mut();
        let Some(idx) = state.trash.iter().position(|t| t.id == trash_id) else {
            return;
        };
        let item = state.trash.remove(idx);
        // Check if original parent folder still exists
        let parent_id = if item.parent_id.as_ref().is_some_and(|pid| state.folders.iter().any(|f| &f.id == pid)) {
            item.parent_id
        } else {
            None
        };
        let note = NoteItem {
            id: item.id.clone(),
            name: item.name,
            content: item.content,
            tags: item.tags,
            created_at: item.created_at,
            updated_at: unix_now(),
            versions: item.versions,
            file_path: None,
            parent_id,
            pinned: item.pinned,
        };
        state.notes.push(note);
        state.viewing_trash = false;
        item.id
    };
    switch_to_note(ctx, &note_id);
    trigger_vault_save(ctx);
}

pub fn delete_permanently(ctx: &EditorCtx, trash_id: &str) {
    ctx.state
        .borrow_mut()
        .trash
        .retain(|t| t.id != trash_id);
    refresh_trash_view(ctx);
    trigger_vault_save(ctx);
}

pub fn empty_trash_action(ctx: &EditorCtx) {
    let count = ctx.state.borrow().trash.len();
    if count == 0 {
        return;
    }

    let dialog = adw::AlertDialog::new(
        Some("Empty Trash?"),
        Some(&format!(
            "This will permanently delete {count} item(s). This cannot be undone."
        )),
    );
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("empty", "Empty Trash");
    dialog.set_response_appearance("empty", adw::ResponseAppearance::Destructive);
    dialog.set_close_response("cancel");

    let window = ctx.window.clone();
    let ctx = ctx.clone();
    dialog.connect_response(None, move |_, response| {
        if response == "empty" {
            ctx.state.borrow_mut().trash.clear();
            refresh_trash_view(&ctx);
            trigger_vault_save(&ctx);
        }
    });
    dialog.present(Some(&window));
}

pub fn refresh_trash_view(ctx: &EditorCtx) {
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

    let trash = ctx.state.borrow().trash.clone();
    if trash.is_empty() {
        let empty = gtk::Label::new(Some("Trash is empty"));
        empty.add_css_class("dim-label");
        empty.set_margin_top(24);
        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&empty));
        row.set_selectable(false);
        row.set_activatable(false);
        ctx.notes_list.append(&row);
        return;
    }

    for item in &trash {
        let row = gtk::ListBoxRow::new();
        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row_box.set_margin_start(8);
        row_box.set_margin_end(8);
        row_box.set_margin_top(6);
        row_box.set_margin_bottom(6);

        let info = gtk::Box::new(gtk::Orientation::Vertical, 2);
        info.set_hexpand(true);
        let title = gtk::Label::new(Some(&item.name));
        title.set_xalign(0.0);
        title.add_css_class("note-row-title");
        let date_label = gtk::Label::new(Some(&format!("Deleted {}", format_ts(item.deleted_at))));
        date_label.set_xalign(0.0);
        date_label.add_css_class("dim-label");
        date_label.add_css_class("caption");
        info.append(&title);
        info.append(&date_label);

        let restore_btn = gtk::Button::with_label("Restore");
        restore_btn.add_css_class("flat");
        restore_btn.add_css_class("toolbar-pill");
        let del_btn = gtk::Button::with_label("Delete");
        del_btn.add_css_class("flat");
        del_btn.add_css_class("destructive-action");

        {
            let ctx = ctx.clone();
            let id = item.id.clone();
            restore_btn.connect_clicked(move |_| restore_from_trash(&ctx, &id));
        }
        {
            let ctx = ctx.clone();
            let id = item.id.clone();
            del_btn.connect_clicked(move |_| delete_permanently(&ctx, &id));
        }

        row_box.append(&info);
        row_box.append(&restore_btn);
        row_box.append(&del_btn);
        row.set_child(Some(&row_box));
        row.set_selectable(false);
        row.set_activatable(false);
        ctx.notes_list.append(&row);
    }
}

// ---------------------------------------------------------------------------
// Folder operations
// ---------------------------------------------------------------------------

pub fn create_folder(ctx: &EditorCtx, parent_id: Option<String>) {
    let dialog = adw::AlertDialog::new(
        Some("New Folder"),
        Some("Enter a name for the folder."),
    );

    let entry = gtk::Entry::new();
    entry.set_text("New Folder");
    entry.set_activates_default(true);
    dialog.set_extra_child(Some(&entry));

    dialog.add_response("cancel", "Cancel");
    dialog.add_response("create", "Create");
    dialog.set_response_appearance("create", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("create"));
    dialog.set_close_response("cancel");

    let window = ctx.window.clone();
    let ctx = ctx.clone();
    dialog.connect_response(None, move |dlg, response| {
        let name = entry.text().trim().to_string();
        dlg.set_extra_child(gtk::Widget::NONE);
        if response == "create"
            && !name.is_empty() {
                let mut state = ctx.state.borrow_mut();
                if folder_name_exists(&state.folders, &name, &parent_id, None) {
                    drop(state);
                    send_toast(&ctx, "A folder with that name already exists here");
                    return;
                }
                let id = format!("folder-{}", state.next_note_seq);
                state.next_note_seq += 1;
                state.folders.push(FolderItem {
                    id,
                    name,
                    expanded: true,
                    created_at: unix_now(),
                    updated_at: unix_now(),
                    parent_id: parent_id.clone(),
                });
                drop(state);
                refresh_note_list(&ctx);
                trigger_vault_save(&ctx);
            }
    });
    dialog.present(Some(&window));
}

pub fn move_note_to_folder(ctx: &EditorCtx) {
    let (folders, active_id, current_parent) = {
        let state = ctx.state.borrow();
        let folders: Vec<(String, String)> = state
            .folders
            .iter()
            .map(|f| (f.id.clone(), f.name.clone()))
            .collect();
        let parent = find_note_index(&state.notes, &state.active_note_id)
            .and_then(|i| state.notes[i].parent_id.clone());
        (folders, state.active_note_id.clone(), parent)
    };

    if folders.is_empty() {
        show_info(
            &ctx.window,
            "No Folders",
            "Create a folder first to move notes into.",
        );
        return;
    }

    let dialog = adw::AlertDialog::new(
        Some("Move to Folder"),
        Some("Select a destination folder."),
    );

    // Build a ListBox with folder choices
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.add_css_class("boxed-list");

    // Root option
    let root_row = adw::ActionRow::builder()
        .title("Root (top level)")
        .build();
    if current_parent.is_none() {
        root_row.set_subtitle("current");
    }
    list.append(&root_row);

    for (id, name) in &folders {
        let row = adw::ActionRow::builder()
            .title(name)
            .build();
        if Some(id) == current_parent.as_ref() {
            row.set_subtitle("current");
        }
        list.append(&row);
    }
    list.select_row(list.row_at_index(0).as_ref());

    dialog.set_extra_child(Some(&list));

    dialog.add_response("cancel", "Cancel");
    dialog.add_response("move", "Move");
    dialog.set_response_appearance("move", adw::ResponseAppearance::Suggested);
    dialog.set_close_response("cancel");

    let window = ctx.window.clone();
    let ctx = ctx.clone();
    dialog.connect_response(None, move |_, response| {
        if response == "move" {
            let selected_idx = list.selected_row().map(|r| r.index()).unwrap_or(-1);
            if selected_idx < 0 {
                return;
            }
            let target = if selected_idx == 0 {
                None
            } else {
                folders.get((selected_idx - 1) as usize).map(|(id, _)| id.clone())
            };
            {
                let mut state = ctx.state.borrow_mut();
                if let Some(i) = find_note_index(&state.notes, &active_id) {
                    let name = &state.notes[i].name;
                    if note_name_exists(&state.notes, name, &target, Some(&active_id)) {
                        let msg = format!(
                            "A note named \"{}\" already exists in the destination folder",
                            name
                        );
                        drop(state);
                        send_toast(&ctx, &msg);
                        return;
                    }
                    state.notes[i].parent_id = target;
                    state.notes[i].updated_at = unix_now();
                }
            }
            refresh_note_list(&ctx);
            trigger_vault_save(&ctx);
        }
    });
    dialog.present(Some(&window));
}

// ---------------------------------------------------------------------------
// Templates
// ---------------------------------------------------------------------------

pub fn builtin_templates() -> Vec<(String, String, String)> {
    vec![
        ("Meeting Notes".into(), "# Meeting Notes\n\n**Date**: \n**Attendees**:\n\n## Agenda\n\n1. \n\n## Discussion\n\n- \n\n## Action Items\n\n- [ ] \n".into(), "meeting".into()),
        ("TODO List".into(), "# TODO List\n\n## High Priority\n\n- [ ] \n\n## Medium Priority\n\n- [ ] \n\n## Completed\n\n- [x] \n".into(), "todo".into()),
        ("Journal Entry".into(), "# Journal Entry\n\n## How am I feeling?\n\n\n## What happened today?\n\n\n## What am I grateful for?\n\n1. \n2. \n3. \n".into(), "journal".into()),
        ("Project Plan".into(), "# Project Plan\n\n## Overview\n\n\n## Goals\n\n- \n\n## Timeline\n\n| Phase | Start | End | Status |\n| --- | --- | --- | --- |\n| Planning |  |  | In Progress |\n".into(), "project".into()),
    ]
}

pub fn show_template_picker(ctx: &EditorCtx) {
    let dialog = adw::Window::builder()
        .transient_for(&ctx.window)
        .modal(true)
        .title("Create from Template")
        .default_width(360)
        .default_height(400)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    vbox.append(&header);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 4);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_margin_top(8);
    content.set_margin_bottom(12);

    // Blank note button
    let blank_btn = gtk::Button::with_label("Blank Note");
    blank_btn.add_css_class("flat");
    blank_btn.set_halign(gtk::Align::Fill);
    {
        let ctx = ctx.clone();
        let dialog = dialog.clone();
        blank_btn.connect_clicked(move |_| {
            create_note(&ctx, "Untitled Note".into(), "# Untitled Note\n\n".into(), Vec::new());
            dialog.close();
        });
    }
    content.append(&blank_btn);

    let sep1 = gtk::Separator::new(gtk::Orientation::Horizontal);
    sep1.set_margin_top(4);
    sep1.set_margin_bottom(4);
    content.append(&sep1);

    let builtin_label = gtk::Label::new(Some("Built-in Templates"));
    builtin_label.add_css_class("heading");
    builtin_label.set_xalign(0.0);
    content.append(&builtin_label);

    for (name, body, tags_csv) in builtin_templates() {
        let btn = gtk::Button::with_label(&name);
        btn.add_css_class("flat");
        btn.set_halign(gtk::Align::Fill);
        let ctx = ctx.clone();
        let dialog = dialog.clone();
        btn.connect_clicked(move |_| {
            let tags: Vec<String> = tags_csv.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            create_note(&ctx, name.clone(), body.clone(), tags);
            dialog.close();
        });
        content.append(&btn);
    }

    // Custom templates
    let custom_templates = ctx.state.borrow().custom_templates.clone();
    if !custom_templates.is_empty() {
        let sep2 = gtk::Separator::new(gtk::Orientation::Horizontal);
        sep2.set_margin_top(4);
        sep2.set_margin_bottom(4);
        content.append(&sep2);

        let custom_label = gtk::Label::new(Some("Custom Templates"));
        custom_label.add_css_class("heading");
        custom_label.set_xalign(0.0);
        content.append(&custom_label);

        for (i, (name, body, tags_csv)) in custom_templates.iter().enumerate() {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            let btn = gtk::Button::with_label(name);
            btn.add_css_class("flat");
            btn.set_hexpand(true);
            {
                let ctx = ctx.clone();
                let dialog = dialog.clone();
                let name = name.clone();
                let body = body.clone();
                let tags_csv = tags_csv.clone();
                btn.connect_clicked(move |_| {
                    let tags: Vec<String> = tags_csv.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                    create_note(&ctx, name.clone(), body.clone(), tags);
                    dialog.close();
                });
            }
            let del_btn = gtk::Button::from_icon_name("edit-delete-symbolic");
            del_btn.add_css_class("flat");
            del_btn.add_css_class("destructive-action");
            del_btn.set_tooltip_text(Some("Delete this template"));
            {
                let ctx = ctx.clone();
                let dialog = dialog.clone();
                del_btn.connect_clicked(move |_| {
                    {
                        let mut state = ctx.state.borrow_mut();
                        if i < state.custom_templates.len() {
                            state.custom_templates.remove(i);
                        }
                    }
                    trigger_vault_save(&ctx);
                    dialog.close();
                    show_template_picker(&ctx);
                });
            }
            row.append(&btn);
            row.append(&del_btn);
            content.append(&row);
        }
    }

    // "Save current note as template" button
    let sep3 = gtk::Separator::new(gtk::Orientation::Horizontal);
    sep3.set_margin_top(8);
    sep3.set_margin_bottom(4);
    content.append(&sep3);

    let save_template_btn = gtk::Button::with_label("Save Current Note as Template");
    save_template_btn.add_css_class("suggested-action");
    save_template_btn.add_css_class("pill");
    {
        let ctx = ctx.clone();
        let dialog = dialog.clone();
        save_template_btn.connect_clicked(move |_| {
            dialog.close();
            save_note_as_template(&ctx);
        });
    }
    content.append(&save_template_btn);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .child(&content)
        .build();
    vbox.append(&scroll);

    dialog.set_content(Some(&vbox));
    dialog.present();
}

pub fn save_note_as_template(ctx: &EditorCtx) {
    let (name, content, tags) = {
        let state = ctx.state.borrow();
        find_note_index(&state.notes, &state.active_note_id)
            .map(|i| {
                let note = &state.notes[i];
                (note.name.clone(), note.content.clone(), note.tags.join(","))
            })
            .unwrap_or_else(|| ("Untitled".into(), String::new(), String::new()))
    };

    let dialog = adw::AlertDialog::new(
        Some("Save as Template"),
        Some(&format!("Save \"{name}\" as a reusable template?")),
    );
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("save", "Save");
    dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
    dialog.set_close_response("cancel");

    let window = ctx.window.clone();
    let ctx = ctx.clone();
    dialog.connect_response(None, move |_, response| {
        if response == "save" {
            {
                let mut state = ctx.state.borrow_mut();
                state
                    .custom_templates
                    .push((name.clone(), content.clone(), tags.clone()));
            }
            trigger_vault_save(&ctx);
        }
    });
    dialog.present(Some(&window));
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
