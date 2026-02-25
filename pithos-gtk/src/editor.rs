use adw::prelude::*;
use sourceview5 as sourceview;
use std::{fs, path::PathBuf};
use pithos_core::state::*;
use crate::*;

// ---------------------------------------------------------------------------
// Signal wiring
// ---------------------------------------------------------------------------

pub fn wire_editor_signals(ctx: &EditorCtx, tag_entry: &gtk::Entry) {
    // Tag autocomplete popover
    let tag_popover = gtk::Popover::new();
    tag_popover.set_parent(tag_entry);
    tag_popover.set_autohide(false);
    tag_popover.add_css_class("menu");
    let tag_suggest_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    tag_suggest_box.set_margin_start(4);
    tag_suggest_box.set_margin_end(4);
    tag_suggest_box.set_margin_top(4);
    tag_suggest_box.set_margin_bottom(4);
    tag_popover.set_child(Some(&tag_suggest_box));

    {
        let ctx = ctx.clone();
        let tag_popover = tag_popover.clone();
        tag_entry.connect_activate(move |entry| {
            let tag = entry.text().trim().to_lowercase();
            if tag.is_empty() {
                return;
            }
            add_tag_to_active_note(&ctx, tag);
            entry.set_text("");
            tag_popover.popdown();
        });
    }
    {
        let ctx = ctx.clone();
        let tag_popover = tag_popover.clone();
        let tag_suggest_box = tag_suggest_box.clone();
        tag_entry.connect_changed(move |entry| {
            let input = entry.text().trim().to_lowercase();
            // Clear old suggestions
            while let Some(child) = tag_suggest_box.first_child() {
                tag_suggest_box.remove(&child);
            }
            if input.is_empty() {
                tag_popover.popdown();
                return;
            }
            // Collect all unique tags
            let state = ctx.state.borrow();
            let active_tags: Vec<String> = find_note_index(&state.notes, &state.active_note_id)
                .map(|i| state.notes[i].tags.clone())
                .unwrap_or_default();
            let mut all_tags: Vec<String> = state
                .notes
                .iter()
                .flat_map(|n| n.tags.iter().cloned())
                .collect();
            all_tags.sort();
            all_tags.dedup();
            drop(state);

            let suggestions: Vec<String> = all_tags
                .into_iter()
                .filter(|t| t.contains(&input) && !active_tags.contains(t))
                .take(5)
                .collect();

            if suggestions.is_empty() {
                tag_popover.popdown();
                return;
            }

            for suggestion in &suggestions {
                let btn = gtk::Button::with_label(suggestion);
                btn.add_css_class("flat");
                btn.set_halign(gtk::Align::Fill);
                let ctx = ctx.clone();
                let tag = suggestion.clone();
                let entry = entry.clone();
                let popover = tag_popover.clone();
                btn.connect_clicked(move |_| {
                    add_tag_to_active_note(&ctx, tag.clone());
                    entry.set_text("");
                    popover.popdown();
                });
                tag_suggest_box.append(&btn);
            }
            tag_popover.popup();
        });
    }
    // Source buffer change handler — debounced processing + preview update
    {
        let ctx = ctx.clone();
        let buffer = ctx.source_buffer.clone();
        buffer.connect_changed(move |_| {
            process_buffer_change_debounced(&ctx);
        });
    }
    // Drag-and-drop image files onto the source editor
    {
        let source_view = ctx.source_view.clone();
        let ctx = ctx.clone();
        let drop = gtk::DropTarget::new(gtk::gio::File::static_type(), gdk::DragAction::COPY);
        drop.connect_drop(move |_, value, _, _| {
            if let Ok(file) = value.get::<gtk::gio::File>() {
                if let Some(path) = file.path() {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg") {
                        if let Ok(bytes) = fs::read(&path) {
                            let filename = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("image")
                                .to_string();
                            let mime = mime_from_ext(&ext);
                            store_image_as_asset(&ctx, &bytes, &filename, &mime);
                            return true;
                        }
                    }
                }
            }
            false
        });
        source_view.add_controller(drop);
    }
    // Synchronized scroll: sync editor scroll position to preview
    {
        let ctx = ctx.clone();
        let adj = ctx.source_panel.vadjustment();
        adj.connect_value_changed(move |adj| {
            let range = adj.upper() - adj.page_size();
            if range > 0.0 {
                let fraction = adj.value() / range;
                crate::preview::sync_preview_scroll(&ctx, fraction);
            }
        });
    }
}

pub fn wire_keyboard_shortcuts(
    ctx: &EditorCtx,
    window: &adw::ApplicationWindow,
) {
    // Use set_accels_for_action — the standard GNOME pattern.
    let app = window
        .application()
        .expect("window must have an application");

    let shortcuts: &[(&[&str], &str)] = &[
        (&["<Ctrl>n"],          "win.new-note"),
        (&["<Ctrl>s"],          "win.save-vault"),
        (&["<Ctrl><Shift>s"],   "win.save-as"),
        (&["<Ctrl>o"],          "win.import-file"),
        (&["<Ctrl>w"],          "win.close-tab"),
        (&["<Ctrl>z"],          "win.undo"),
        (&["<Ctrl><Shift>z", "<Ctrl>y"], "win.redo"),
        (&["<Ctrl>backslash"],  "win.toggle-sidebar"),
        (&["<Ctrl><Shift>j"],   "win.zen-mode"),
        (&["<Ctrl><Shift>t"],   "win.daily-note"),
        (&["<Ctrl><Shift>f"],   "win.focus-search"),
        (&["<Ctrl><Shift>d"],   "win.toggle-theme"),
        (&["F2"],               "win.rename-note"),
        (&["F11"],              "win.fullscreen"),
        // Formatting shortcuts
        (&["<Ctrl>b"],          "win.fmt-bold"),
        (&["<Ctrl>i"],          "win.fmt-italic"),
        (&["<Ctrl>u"],          "win.fmt-underline"),
        (&["<Ctrl>d"],          "win.fmt-strike"),
        (&["<Ctrl>e"],          "win.fmt-code"),
        (&["<Ctrl>k"],          "win.fmt-link"),
        (&["<Ctrl>1"],          "win.fmt-h1"),
        (&["<Ctrl>2"],          "win.fmt-h2"),
        (&["<Ctrl>3"],          "win.fmt-h3"),
        (&["<Ctrl>4"],          "win.fmt-h4"),
        (&["<Ctrl>5"],          "win.fmt-h5"),
        (&["<Ctrl>6"],          "win.fmt-h6"),
        (&["<Ctrl><Shift>q"],   "win.fmt-quote"),
        (&["<Ctrl><Shift>l"],   "win.fmt-bullet-list"),
        (&["<Ctrl>space"],      "win.toggle-checkbox"),
        (&["<Ctrl><Shift>p"],   "win.command-palette"),
        (&["F1"],               "win.show-help"),
        (&["<Ctrl>f"],          "win.find-in-editor"),
        (&["<Ctrl>h"],          "win.find-replace"),
    ];

    for &(accels, action_name) in shortcuts {
        app.set_accels_for_action(action_name, accels);
    }

    // Source-view key handler: Escape (zen exit), Ctrl+V (paste image)
    {
        let source_view = ctx.source_view.clone();
        let ctx = ctx.clone();
        let key_ctl = gtk::EventControllerKey::new();
        key_ctl.connect_key_pressed(move |_, key, _, mods| {
            handle_source_view_keys(&ctx, key, mods)
        });
        source_view.add_controller(key_ctl);
    }
}

// ---------------------------------------------------------------------------
// Buffer sync / undo-redo
// ---------------------------------------------------------------------------

pub fn sync_markdown_and_status(ctx: &EditorCtx) -> String {
    let markdown = source_buffer_text(&ctx.source_buffer);
    update_status_full(ctx, &markdown);
    render_preview(ctx);
    markdown
}

pub fn process_buffer_change_debounced(ctx: &EditorCtx) {
    if ctx.state.borrow().suppress_sync {
        return;
    }
    // Cancel any pending debounced sync
    if let Some(id) = ctx.sync_timeout_id.take() {
        id.remove();
    }
    let timeout_cell = ctx.sync_timeout_id.clone();
    let ctx_inner = ctx.clone();
    let id = glib::timeout_add_local_once(
        std::time::Duration::from_millis(300),
        move || {
            timeout_cell.set(None);
            if !ctx_inner.state.borrow().suppress_sync {
                do_sync_and_undo(&ctx_inner);
            }
        },
    );
    ctx.sync_timeout_id.set(Some(id));
}

pub fn process_buffer_change(ctx: &EditorCtx) {
    // Cancel any pending debounced sync
    if let Some(id) = ctx.sync_timeout_id.take() {
        id.remove();
    }
    if ctx.state.borrow().suppress_sync {
        return;
    }
    // Explicit actions (toolbar, shortcuts) always checkpoint for undo
    do_sync_and_undo_checkpoint(ctx);
}

pub fn do_sync_and_undo(ctx: &EditorCtx) {
    do_sync_and_undo_inner(ctx, false);
}

pub fn do_sync_and_undo_checkpoint(ctx: &EditorCtx) {
    do_sync_and_undo_inner(ctx, true);
}

pub fn do_sync_and_undo_inner(ctx: &EditorCtx, force_checkpoint: bool) {
    let markdown = sync_markdown_and_status(ctx);
    update_active_note_content(ctx, &markdown);
    let mut should_refresh = false;
    let mut content_changed = false;

    {
        let mut state = ctx.state.borrow_mut();
        let previous = state.last_snapshot.clone();

        if markdown != previous {
            content_changed = true;
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(state.last_undo_push);
            if force_checkpoint || elapsed > std::time::Duration::from_millis(500) {
                if !previous.is_empty() {
                    state.undo_stack.push(previous);
                    if state.undo_stack.len() > MAX_UNDO_HISTORY {
                        state.undo_stack.remove(0);
                    }
                }
                state.last_undo_push = now;
            }
            state.last_snapshot = markdown.clone();
            state.redo_stack.clear();
        }

        let dirty_now = markdown != state.saved_snapshot;
        if dirty_now != state.dirty {
            state.dirty = dirty_now;
            should_refresh = true;
        }
    }

    if should_refresh {
        refresh_header(ctx);
    }
    if !ctx.state.borrow().search_query.trim().is_empty() {
        refresh_note_list(ctx);
    }
    if content_changed {
        trigger_vault_save(ctx);
    }
}

pub fn refresh_header(ctx: &EditorCtx) {
    let state = ctx.state.borrow();
    let file_name = find_note_index(&state.notes, &state.active_note_id)
        .map(|index| state.notes[index].name.clone())
        .unwrap_or_else(|| "Untitled".to_string());

    ctx.doc_label.set_label(&file_name);

    if state.dirty {
        ctx.dirty_label.set_visible(true);
        ctx.dirty_label.set_label("\u{25cf} Unsaved");
    } else {
        ctx.dirty_label.set_visible(false);
    }

    let title = if state.dirty {
        format!("*{file_name} \u{2014} Pithos Notebook")
    } else {
        format!("{file_name} \u{2014} Pithos Notebook")
    };
    ctx.window.set_title(Some(&title));
}

pub fn load_document(ctx: &EditorCtx, markdown: &str, path: Option<PathBuf>) {
    {
        let mut state = ctx.state.borrow_mut();
        state.suppress_sync = true;
    }
    ctx.source_buffer.set_text(markdown);
    {
        let mut state = ctx.state.borrow_mut();
        state.suppress_sync = false;
    }

    let normalized = sync_markdown_and_status(ctx);

    {
        let mut state = ctx.state.borrow_mut();
        state.saved_snapshot = normalized.clone();
        state.last_snapshot = normalized;
        state.undo_stack.clear();
        state.redo_stack.clear();
        state.dirty = false;
        if let Some(index) = find_note_index(&state.notes, &state.active_note_id) {
            state.notes[index].content = state.saved_snapshot.clone();
            state.notes[index].updated_at = unix_now();
            state.notes[index].file_path = path;
        }
    }

    refresh_header(ctx);
    refresh_tabs(ctx);
    refresh_note_list(ctx);
    refresh_tags(ctx);
}

pub fn undo(ctx: &EditorCtx) {
    let target = {
        let mut state = ctx.state.borrow_mut();
        let Some(previous) = state.undo_stack.pop() else {
            return;
        };

        let current = state.last_snapshot.clone();
        if !current.is_empty() {
            state.redo_stack.push(current);
        }

        previous
    };

    apply_snapshot(ctx, &target);
}

pub fn redo(ctx: &EditorCtx) {
    let target = {
        let mut state = ctx.state.borrow_mut();
        let Some(next) = state.redo_stack.pop() else {
            return;
        };

        let current = state.last_snapshot.clone();
        if !current.is_empty() {
            state.undo_stack.push(current);
        }

        next
    };

    apply_snapshot(ctx, &target);
}

pub fn apply_snapshot(ctx: &EditorCtx, markdown: &str) {
    // Cancel any pending debounced sync to prevent stale snapshots from
    // firing after the undo/redo completes and corrupting the undo stack.
    if let Some(id) = ctx.sync_timeout_id.take() {
        id.remove();
    }
    // Also cancel any pending auto-save to prevent stale data from being written.
    if let Some(id) = ctx.save_timeout_id.take() {
        id.remove();
    }

    {
        let mut state = ctx.state.borrow_mut();
        state.suppress_sync = true;
    }

    ctx.source_buffer.set_text(markdown);

    {
        let mut state = ctx.state.borrow_mut();
        state.suppress_sync = false;
    }

    let normalized = sync_markdown_and_status(ctx);
    update_active_note_content(ctx, &normalized);

    {
        let mut state = ctx.state.borrow_mut();
        state.last_snapshot = normalized.clone();
        state.dirty = normalized != state.saved_snapshot;
    }

    refresh_header(ctx);
    refresh_note_list(ctx);
}

/// Smart list continuation: when pressing Enter inside a list item, auto-continue
/// with the matching prefix. If the item is empty (just the prefix), remove it instead.
fn handle_list_continuation(ctx: &EditorCtx) -> bool {
    use gtk::prelude::TextBufferExt;
    let buffer = &ctx.source_buffer;
    let cursor = buffer.iter_at_offset(buffer.cursor_position());
    let line = cursor.line();
    let line_start = match buffer.iter_at_line(line) {
        Some(it) => it,
        None => return false,
    };
    let line_text = buffer.text(&line_start, &cursor, true).to_string();

    // Capture leading whitespace
    let trimmed = line_text.trim_start_matches([' ', '\t']);
    let indent = &line_text[..line_text.len() - trimmed.len()];

    // Detect which list type and extract (next_prefix, is_content_empty)
    let result: Option<(String, bool)> = None
        // Task list: "- [ ] " / "- [x] " / "* [ ] " etc.
        .or_else(|| {
            let markers = ["- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] ", "* [X] ", "+ [ ] ", "+ [x] ", "+ [X] "];
            for m in markers {
                if let Some(rest) = trimmed.strip_prefix(m) {
                    return Some(("- [ ] ".to_string(), rest.is_empty()));
                }
            }
            None
        })
        // Ordered list: "1. " etc.
        .or_else(|| {
            let dot_pos = trimmed.find(". ")?;
            let num: u64 = trimmed[..dot_pos].parse().ok()?;
            let rest = &trimmed[dot_pos + 2..];
            Some((format!("{}. ", num + 1), rest.is_empty()))
        })
        // Unordered list: "- " / "* " / "+ "
        .or_else(|| {
            for prefix in ["- ", "* ", "+ "] {
                if let Some(rest) = trimmed.strip_prefix(prefix) {
                    return Some((prefix.to_string(), rest.is_empty()));
                }
            }
            None
        })
        // Blockquote: "> "
        .or_else(|| {
            trimmed.strip_prefix("> ").map(|rest| ("> ".to_string(), rest.is_empty()))
        });

    let Some((next_prefix, is_empty)) = result else { return false };

    if is_empty {
        // Empty list item — remove the prefix
        let mut s = buffer.iter_at_offset(line_start.offset());
        let mut e = buffer.iter_at_offset(cursor.offset());
        buffer.delete(&mut s, &mut e);
    } else {
        // Continue the list
        let mut ins = buffer.iter_at_offset(cursor.offset());
        buffer.insert(&mut ins, &format!("\n{indent}{next_prefix}"));
    }

    true
}

/// Source-view key handling: Escape (zen mode exit) and Ctrl+V (paste image).
pub fn handle_source_view_keys(
    ctx: &EditorCtx,
    key: gdk::Key,
    mods: gdk::ModifierType,
) -> glib::Propagation {
    // Ctrl+V with image on clipboard → store as vault asset
    if key == gdk::Key::v
        && mods.contains(gdk::ModifierType::CONTROL_MASK)
        && try_paste_image(ctx) {
            return glib::Propagation::Stop;
        }

    // Enter: smart list continuation (not with Shift)
    if (key == gdk::Key::Return || key == gdk::Key::KP_Enter)
        && !mods.contains(gdk::ModifierType::SHIFT_MASK)
        && handle_list_continuation(ctx)
    {
        return glib::Propagation::Stop;
    }

    // Escape hides find bar first, then exits zen mode
    if key == gdk::Key::Escape {
        if ctx.find_bar.is_visible() {
            hide_find_bar(ctx);
            return glib::Propagation::Stop;
        }
        if ctx.state.borrow().zen_mode {
            toggle_zen_mode(ctx);
            return glib::Propagation::Stop;
        }
    }

    glib::Propagation::Proceed
}

// ---------------------------------------------------------------------------
// Markdown syntax helpers (source-first editing)
// ---------------------------------------------------------------------------

pub fn md_wrap(buffer: &sourceview::Buffer, prefix: &str, suffix: &str, placeholder: &str) {
    if let Some((start, end)) = buffer.selection_bounds() {
        let selected = buffer.text(&start, &end, true).to_string();
        let mut s = buffer.iter_at_offset(start.offset());
        let mut e = buffer.iter_at_offset(end.offset());
        buffer.delete(&mut s, &mut e);
        let text = format!("{prefix}{selected}{suffix}");
        let mut ins = buffer.iter_at_offset(start.offset());
        buffer.insert(&mut ins, &text);
    } else {
        let mut iter = buffer.iter_at_offset(buffer.cursor_position());
        let text = format!("{prefix}{placeholder}{suffix}");
        let offset = iter.offset() + prefix.chars().count() as i32;
        buffer.insert(&mut iter, &text);
        // Select the placeholder
        let sel_start = buffer.iter_at_offset(offset);
        let sel_end = buffer.iter_at_offset(offset + placeholder.chars().count() as i32);
        buffer.select_range(&sel_start, &sel_end);
    }
}

pub fn md_line_prefix(buffer: &sourceview::Buffer, prefix: &str) {
    let cursor = buffer.iter_at_offset(buffer.cursor_position());
    let line = cursor.line();
    let mut line_start = buffer.iter_at_line(line).unwrap_or(cursor);
    let mut line_end = line_start;
    if !line_end.ends_line() {
        line_end.forward_to_line_end();
    }
    let line_text = buffer.text(&line_start, &line_end, true).to_string();

    if line_text.starts_with(prefix) {
        // Remove prefix
        let mut prefix_end = buffer.iter_at_offset(line_start.offset() + prefix.chars().count() as i32);
        buffer.delete(&mut line_start, &mut prefix_end);
    } else {
        // Strip any existing heading/list prefix first, then add new one
        let stripped = strip_line_prefix(&line_text);
        let mut start = buffer.iter_at_line(line).unwrap_or(cursor);
        let mut end = start;
        if !end.ends_line() {
            end.forward_to_line_end();
        }
        buffer.delete(&mut start, &mut end);
        let new_text = format!("{prefix}{stripped}");
        let mut ins = buffer.iter_at_line(line).unwrap_or(buffer.start_iter());
        buffer.insert(&mut ins, &new_text);
    }
}

pub fn strip_line_prefix(text: &str) -> &str {
    let t = text.trim_start();
    // Headings
    if let Some(rest) = t.strip_prefix("######") { return rest.strip_prefix(' ').unwrap_or(rest); }
    if let Some(rest) = t.strip_prefix("#####") { return rest.strip_prefix(' ').unwrap_or(rest); }
    if let Some(rest) = t.strip_prefix("####") { return rest.strip_prefix(' ').unwrap_or(rest); }
    if let Some(rest) = t.strip_prefix("###") { return rest.strip_prefix(' ').unwrap_or(rest); }
    if let Some(rest) = t.strip_prefix("##") { return rest.strip_prefix(' ').unwrap_or(rest); }
    if let Some(rest) = t.strip_prefix('#') { return rest.strip_prefix(' ').unwrap_or(rest); }
    // Task list
    if let Some(rest) = t.strip_prefix("- [x] ").or_else(|| t.strip_prefix("- [ ] ")) { return rest; }
    // Bullet/ordered list
    if let Some(rest) = t.strip_prefix("- ") { return rest; }
    if let Some(rest) = t.strip_prefix("> ") { return rest; }
    // Ordered list like "1. "
    if let Some(dot_pos) = t.find(". ") {
        if t[..dot_pos].chars().all(|c| c.is_ascii_digit()) && dot_pos <= 3 {
            return &t[dot_pos + 2..];
        }
    }
    t
}

pub fn toggle_checkbox_at_cursor(buffer: &sourceview::Buffer) {
    let cursor = buffer.iter_at_offset(buffer.cursor_position());
    let line = cursor.line();
    let line_start = buffer.iter_at_line(line).unwrap_or(cursor);
    let mut line_end = line_start;
    if !line_end.ends_line() {
        line_end.forward_to_line_end();
    }
    let line_text = buffer.text(&line_start, &line_end, true).to_string();
    let trimmed = line_text.trim_start();

    if trimmed.starts_with("- [ ] ") {
        let prefix_offset = line_text.len() - trimmed.len();
        let check_start = line_start.offset() + prefix_offset as i32 + 2; // skip "- "
        let mut cs = buffer.iter_at_offset(check_start);
        let mut ce = buffer.iter_at_offset(check_start + 3); // "[ ]"
        buffer.delete(&mut cs, &mut ce);
        let mut ins = buffer.iter_at_offset(check_start);
        buffer.insert(&mut ins, "[x]");
    } else if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
        let prefix_offset = line_text.len() - trimmed.len();
        let check_start = line_start.offset() + prefix_offset as i32 + 2; // skip "- "
        let mut cs = buffer.iter_at_offset(check_start);
        let mut ce = buffer.iter_at_offset(check_start + 3); // "[x]"
        buffer.delete(&mut cs, &mut ce);
        let mut ins = buffer.iter_at_offset(check_start);
        buffer.insert(&mut ins, "[ ]");
    }
}

// ---------------------------------------------------------------------------
// Snippet insertion
// ---------------------------------------------------------------------------

pub fn insert_link(ctx: &EditorCtx) {
    let buffer = &ctx.source_buffer;

    let initial_label = if let Some((start, end)) = buffer.selection_bounds() {
        let selected = buffer.text(&start, &end, true).to_string();
        if selected.trim().is_empty() {
            "link text".to_string()
        } else {
            selected
        }
    } else {
        "link text".to_string()
    };

    show_link_dialog(ctx, &initial_label, "https://");
}

pub fn show_link_dialog(
    ctx: &EditorCtx,
    initial_label: &str,
    initial_url: &str,
) {
    let dialog = adw::AlertDialog::new(Some("Insert Link"), None);

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
    vbox.set_margin_start(4);
    vbox.set_margin_end(4);

    let label_entry = gtk::Entry::new();
    label_entry.set_placeholder_text(Some("Link text"));
    label_entry.set_text(initial_label);
    vbox.append(&gtk::Label::builder().label("Text").xalign(0.0).build());
    vbox.append(&label_entry);

    let url_entry = gtk::Entry::new();
    url_entry.set_placeholder_text(Some("https://example.com or [[Note Name]]"));
    url_entry.set_text(initial_url);
    url_entry.set_activates_default(true);
    vbox.append(&gtk::Label::builder().label("URL").xalign(0.0).build());
    vbox.append(&url_entry);

    dialog.set_extra_child(Some(&vbox));

    dialog.add_response("cancel", "Cancel");
    dialog.add_response("apply", "Insert");
    dialog.set_response_appearance("apply", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("apply"));
    dialog.set_close_response("cancel");

    let window = ctx.window.clone();
    let ctx = ctx.clone();
    dialog.connect_response(None, move |dlg, response| {
        let label_text = label_entry.text().trim().to_string();
        let url_text = url_entry.text().trim().to_string();
        dlg.set_extra_child(gtk::Widget::NONE);

        if response == "apply" && !label_text.is_empty() && !url_text.is_empty() {
            let buffer = &ctx.source_buffer;

            // Handle wiki links: [[Note Name]] → wiki:Note Name
            let is_wiki = url_text.starts_with("[[") && url_text.ends_with("]]");
            let stored_url = if is_wiki {
                format!("wiki:{}", &url_text[2..url_text.len() - 2])
            } else {
                url_text.clone()
            };

            // Delete selection if any
            if let Some((start, end)) = buffer.selection_bounds() {
                if start.offset() != end.offset() {
                    let mut s = buffer.iter_at_offset(start.offset());
                    let mut e = buffer.iter_at_offset(end.offset());
                    buffer.delete(&mut s, &mut e);
                }
            }

            // Insert markdown link syntax
            let snippet = if is_wiki {
                format!("[[{}]]", &url_text[2..url_text.len() - 2])
            } else {
                format!("[{label_text}]({stored_url})")
            };
            let mut iter = buffer.iter_at_offset(buffer.cursor_position());
            buffer.insert(&mut iter, &snippet);
            process_buffer_change(&ctx);
        }
    });
    dialog.present(Some(&window));
}

pub fn navigate_to_wiki_link(ctx: &EditorCtx, name: &str) {
    let found_id = {
        let state = ctx.state.borrow();
        state
            .notes
            .iter()
            .find(|n| n.name.eq_ignore_ascii_case(name))
            .map(|n| n.id.clone())
    };

    if let Some(id) = found_id {
        switch_to_note(ctx, &id);
    } else {
        create_note(
            ctx,
            name.to_string(),
            format!("# {name}\n\n"),
            Vec::new(),
        );
    }
}

pub fn insert_table_snippet(buffer: &sourceview::Buffer) {
    let snippet = "| Column 1 | Column 2 |\n| --- | --- |\n| | |\n";
    let mut iter = buffer.iter_at_offset(buffer.cursor_position());
    buffer.insert(&mut iter, snippet);
}

pub fn insert_table_with_size(buffer: &sourceview::Buffer, cols: i32, rows: i32) {
    let mut s = String::new();
    // Header row
    s.push('|');
    for c in 1..=cols {
        s.push_str(&format!(" Column {c} |"));
    }
    s.push('\n');
    // Separator row
    s.push('|');
    for _ in 0..cols {
        s.push_str(" --- |");
    }
    s.push('\n');
    // Data rows (rows includes the header, so data rows = rows - 1)
    for _ in 1..rows {
        s.push('|');
        for _ in 0..cols {
            s.push_str("  |");
        }
        s.push('\n');
    }
    let mut iter = buffer.iter_at_offset(buffer.cursor_position());
    buffer.insert(&mut iter, &s);
}

// ---------------------------------------------------------------------------
// Table editing helpers
// ---------------------------------------------------------------------------

struct ParsedTable {
    start_line: i32,
    end_line: i32,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

fn parse_table_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

fn is_separator_row(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.contains('|') {
        return false;
    }
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    inner.split('|').all(|c| {
        let c = c.trim();
        c.chars().all(|ch| ch == '-' || ch == ':') && c.contains('-')
    })
}

fn is_table_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.contains('|')
}

fn detect_table_at_cursor(buffer: &sourceview::Buffer) -> Option<ParsedTable> {
    let cursor = buffer.iter_at_offset(buffer.cursor_position());
    let cursor_line = cursor.line();
    let total_lines = buffer.line_count();

    // Get current line text
    let get_line_text = |line: i32| -> String {
        if line < 0 || line >= total_lines {
            return String::new();
        }
        let start = buffer.iter_at_line(line).unwrap_or(buffer.start_iter());
        let mut end = start;
        if !end.ends_line() {
            end.forward_to_line_end();
        }
        buffer.text(&start, &end, true).to_string()
    };

    // Check if cursor is on a table line
    if !is_table_line(&get_line_text(cursor_line)) {
        return None;
    }

    // Scan upward to find first table line
    let mut start_line = cursor_line;
    while start_line > 0 && is_table_line(&get_line_text(start_line - 1)) {
        start_line -= 1;
    }

    // Scan downward to find last table line
    let mut end_line = cursor_line;
    while end_line < total_lines - 1 && is_table_line(&get_line_text(end_line + 1)) {
        end_line += 1;
    }

    // Need at least 2 lines (header + separator)
    if end_line - start_line < 1 {
        return None;
    }

    let header_text = get_line_text(start_line);
    let sep_text = get_line_text(start_line + 1);

    if !is_separator_row(&sep_text) {
        return None;
    }

    let headers = parse_table_row(&header_text);
    let mut rows = Vec::new();
    for line in (start_line + 2)..=end_line {
        rows.push(parse_table_row(&get_line_text(line)));
    }

    Some(ParsedTable {
        start_line,
        end_line,
        headers,
        rows,
    })
}

fn table_to_markdown(table: &ParsedTable) -> String {
    let col_count = table.headers.len();

    // Calculate max width per column
    let mut widths: Vec<usize> = table.headers.iter().map(|h| h.len().max(3)).collect();
    for row in &table.rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let mut result = String::new();

    // Header row
    result.push('|');
    for (i, h) in table.headers.iter().enumerate() {
        let w = widths.get(i).copied().unwrap_or(3);
        result.push_str(&format!(" {:<w$} |", h, w = w));
    }
    result.push('\n');

    // Separator row
    result.push('|');
    for w in &widths {
        result.push_str(&format!(" {} |", "-".repeat(*w)));
    }
    result.push('\n');

    // Data rows
    for row in &table.rows {
        result.push('|');
        for i in 0..col_count {
            let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
            let w = widths.get(i).copied().unwrap_or(3);
            result.push_str(&format!(" {:<w$} |", cell, w = w));
        }
        result.push('\n');
    }

    result
}

fn replace_table_in_buffer(buffer: &sourceview::Buffer, table: &ParsedTable, new_md: &str) {
    let mut start = buffer.iter_at_line(table.start_line).unwrap_or(buffer.start_iter());
    let mut end = buffer.iter_at_line(table.end_line).unwrap_or(buffer.end_iter());
    if !end.ends_line() {
        end.forward_to_line_end();
    }
    // Include trailing newline if present
    if !end.is_end() {
        end.forward_char();
    }
    buffer.delete(&mut start, &mut end);
    let mut ins = buffer.iter_at_line(table.start_line).unwrap_or(buffer.start_iter());
    buffer.insert(&mut ins, new_md);
}

pub fn table_add_row(ctx: &EditorCtx) {
    let Some(mut table) = detect_table_at_cursor(&ctx.source_buffer) else {
        send_toast(ctx, "Place cursor inside a table first");
        return;
    };
    let col_count = table.headers.len();
    table.rows.push(vec![String::new(); col_count]);
    let md = table_to_markdown(&table);
    replace_table_in_buffer(&ctx.source_buffer, &table, &md);
    process_buffer_change(ctx);
}

pub fn table_add_column(ctx: &EditorCtx) {
    let Some(mut table) = detect_table_at_cursor(&ctx.source_buffer) else {
        send_toast(ctx, "Place cursor inside a table first");
        return;
    };
    table.headers.push(format!("Column {}", table.headers.len() + 1));
    for row in &mut table.rows {
        row.push(String::new());
    }
    let md = table_to_markdown(&table);
    replace_table_in_buffer(&ctx.source_buffer, &table, &md);
    process_buffer_change(ctx);
}

pub fn table_align(ctx: &EditorCtx) {
    let Some(table) = detect_table_at_cursor(&ctx.source_buffer) else {
        send_toast(ctx, "Place cursor inside a table first");
        return;
    };
    let md = table_to_markdown(&table);
    replace_table_in_buffer(&ctx.source_buffer, &table, &md);
    process_buffer_change(ctx);
}

pub fn insert_image_snippet(ctx: &EditorCtx) {
    let dialog = gtk::FileDialog::builder()
        .title("Insert Image")
        .accept_label("Insert")
        .build();

    let filter = gtk::FileFilter::new();
    filter.set_name(Some("Images"));
    filter.add_mime_type("image/*");
    filter.add_pattern("*.png");
    filter.add_pattern("*.jpg");
    filter.add_pattern("*.jpeg");
    filter.add_pattern("*.webp");
    filter.add_pattern("*.gif");
    filter.add_pattern("*.svg");
    let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    dialog.set_filters(Some(&filters));

    let ctx = ctx.clone();
    let window = ctx.window.clone();
    dialog.open(Some(&window), gtk::gio::Cancellable::NONE, move |result: Result<gtk::gio::File, gtk::glib::Error>| {
        if let Ok(file) = result {
            if let Some(path) = file.path() {
                let filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("image")
                    .to_string();
                match fs::read(&path) {
                    Ok(bytes) => {
                        let mime = mime_from_ext(
                            path.extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or(""),
                        );
                        store_image_as_asset(&ctx, &bytes, &filename, &mime);
                    }
                    Err(e) => {
                        show_error(
                            &ctx.window,
                            "Insert image failed",
                            &format!("Could not read file: {e}"),
                        );
                    }
                }
            }
        }
    });
}

pub fn try_paste_image(ctx: &EditorCtx) -> bool {
    let Some(display) = gdk::Display::default() else {
        return false;
    };
    let clipboard = display.clipboard();
    let formats = clipboard.formats();

    let has_image = formats.contain_mime_type("image/png")
        || formats.contain_mime_type("image/jpeg")
        || formats.contain_mime_type("image/gif")
        || formats.contain_mime_type("image/webp");

    if !has_image {
        return false;
    }

    let ctx = ctx.clone();
    clipboard.read_texture_async(gtk::gio::Cancellable::NONE, move |result| {
        if let Ok(Some(texture)) = result {
            // Use a randomized temp filename to avoid symlink/race attacks.
            let random_name = format!("pithos-paste-{}.png", generate_asset_id());
            let tmp = std::env::temp_dir().join(random_name);
            if texture.save_to_png(&tmp).is_ok() {
                if let Ok(bytes) = fs::read(&tmp) {
                    store_image_as_asset(&ctx, &bytes, "pasted-image.png", "image/png");
                }
                let _ = fs::remove_file(&tmp);
            }
        }
    });

    true
}

pub fn mime_from_ext(ext: &str) -> String {
    match ext.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
    .to_string()
}

pub fn store_image_as_asset(ctx: &EditorCtx, data: &[u8], filename: &str, mime: &str) {
    let asset_id = generate_asset_id();
    let vault_folder = ctx.vault_folder.borrow().clone();
    let cached_key = ctx.cached_key.borrow().clone();
    let data_size = data.len() as u64;
    let filename_owned = filename.to_string();
    let mime_owned = mime.to_string();
    let note_id = ctx.state.borrow().active_note_id.clone();

    // Write asset file FIRST (async), then insert metadata + snippet on success
    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let data_owned = data.to_vec();
    let asset_id_thread = asset_id.clone();

    std::thread::spawn(move || {
        let write_data = if let Some(ref key) = cached_key {
            match pithos_core::crypto::encrypt_asset(&data_owned, key) {
                Ok(encrypted) => encrypted.into_bytes(),
                Err(e) => {
                    let _ = tx.send(Err(format!("Asset encryption failed: {e}")));
                    return;
                }
            }
        } else {
            data_owned
        };

        if let Err(e) = pithos_core::vault::write_asset(&vault_folder, &asset_id_thread, &write_data) {
            let _ = tx.send(Err(format!("Asset write failed: {e}")));
        } else {
            let _ = tx.send(Ok(()));
        }
    });

    // Poll for write result; only commit metadata + snippet on success
    let ctx = ctx.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        match rx.try_recv() {
            Ok(Ok(())) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;
                let meta = pithos_core::vault::AssetMeta {
                    id: asset_id.clone(),
                    original_name: filename_owned.clone(),
                    mime_type: mime_owned.clone(),
                    size: data_size,
                    created_at: now,
                };
                ctx.state.borrow_mut().assets.insert(asset_id.clone(), meta);

                // Only insert into the buffer if we're still on the same note
                if ctx.state.borrow().active_note_id == note_id {
                    let safe_alt = filename_owned.replace(']', "\\]");
                    let image_url = format!("vault://{asset_id}");
                    let snippet = format!("![{safe_alt}]({image_url})");
                    let mut iter = ctx
                        .source_buffer
                        .iter_at_offset(ctx.source_buffer.cursor_position());
                    ctx.source_buffer.insert(&mut iter, &snippet);
                    process_buffer_change(&ctx);
                }
                glib::ControlFlow::Break
            }
            Ok(Err(e)) => {
                show_error(&ctx.window, "Asset save failed", &e);
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
        }
    });
}

pub fn generate_asset_id() -> String {
    let mut bytes = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn insert_code_block_snippet_with_language(buffer: &sourceview::Buffer, language: &str) {
    let snippet = format!("```{language}\n\n```\n");
    let mut iter = buffer.iter_at_offset(buffer.cursor_position());
    let offset = iter.offset();
    buffer.insert(&mut iter, &snippet);
    // Place cursor inside the code block (after ```lang\n)
    let cursor_offset = offset + language.chars().count() as i32 + 4; // 3 backticks + newline
    let cursor_iter = buffer.iter_at_offset(cursor_offset);
    buffer.place_cursor(&cursor_iter);
}

pub fn replace_range(
    buffer: &gtk::TextBuffer,
    start: &gtk::TextIter,
    end: &gtk::TextIter,
    replacement: &str,
) {
    let anchor = start.offset();
    let mut start_mut = *start;
    let mut end_mut = *end;
    buffer.delete(&mut start_mut, &mut end_mut);
    let mut iter = buffer.iter_at_offset(anchor);
    buffer.insert(&mut iter, replacement);
}

pub fn update_status(label: &gtk::Label, text: &str) {
    let words = text.split_whitespace().count();
    let chars = text.chars().count();
    let lines = text.lines().count().max(1);
    let read_min = words.div_ceil(200); // ~200 wpm reading speed
    label.set_label(&format!(
        "{words} words  \u{2022}  {chars} chars  \u{2022}  {lines} lines  \u{2022}  ~{read_min} min read"
    ));
}

pub fn update_status_full(ctx: &EditorCtx, text: &str) {
    update_status(&ctx.status, text);

    let state = ctx.state.borrow();
    if let Some(idx) = find_note_index(&state.notes, &state.active_note_id) {
        let note = &state.notes[idx];
        let created = format_ts(note.created_at);
        let modified = format_ts(note.updated_at);

        // Breadcrumbs: folder path
        let folder_name = note
            .parent_id
            .as_ref()
            .and_then(|pid| state.folders.iter().find(|f| &f.id == pid))
            .map(|f| format!("{} / ", f.name))
            .unwrap_or_default();
        ctx.breadcrumbs
            .set_label(&format!("{folder_name}{}", note.name));

        // Meta label in ActionBar: created/modified
        ctx.meta_label.set_label(&format!(
            "Created {created}  \u{2022}  Modified {modified}"
        ));
    }
}

pub fn source_buffer_text(buffer: &sourceview::Buffer) -> String {
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    buffer.text(&start, &end, true).to_string()
}

pub fn current_markdown(ctx: &EditorCtx) -> String {
    sync_markdown_and_status(ctx)
}

pub fn update_active_note_content(ctx: &EditorCtx, markdown: &str) {
    let mut state = ctx.state.borrow_mut();
    if let Some(index) = find_note_index(&state.notes, &state.active_note_id) {
        if state.notes[index].content != markdown {
            state.notes[index].content = markdown.to_string();
            state.notes[index].updated_at = unix_now();
        }
    }
}

// ---------------------------------------------------------------------------
// Find / replace
// ---------------------------------------------------------------------------

pub fn show_find_bar(ctx: &EditorCtx, with_replace: bool) {
    use sourceview5::prelude::*;
    ctx.find_bar.set_visible(true);
    ctx.replace_row.set_visible(with_replace);
    ctx.find_entry.grab_focus();
    // If text is selected, use it as the search term
    if let Some((start, end)) = ctx.source_buffer.selection_bounds() {
        let selected = ctx.source_buffer.text(&start, &end, true).to_string();
        if !selected.is_empty() && !selected.contains('\n') {
            ctx.find_entry.set_text(&selected);
        }
    }
    // Trigger initial search from current entry text
    let text = ctx.find_entry.text().to_string();
    if !text.is_empty() {
        ctx.search_settings.set_search_text(Some(&text));
    }
    update_find_match_label(ctx);
}

pub fn hide_find_bar(ctx: &EditorCtx) {
    use sourceview5::prelude::*;
    ctx.find_bar.set_visible(false);
    ctx.search_settings.set_search_text(None);
    ctx.search_context.set_highlight(false);
    ctx.source_view.grab_focus();
}

pub fn find_next(ctx: &EditorCtx) {
    let cursor = ctx.source_buffer.iter_at_offset(ctx.source_buffer.cursor_position());
    if let Some((mut start, end, _wrapped)) = ctx.search_context.forward(&cursor) {
        // If the match is right at the cursor, move forward past it to find the next one
        if start.offset() == cursor.offset() {
            if let Some((s2, e2, _)) = ctx.search_context.forward(&end) {
                ctx.source_buffer.select_range(&s2, &e2);
                ctx.source_view.scroll_to_iter(&mut s2.clone(), 0.1, false, 0.0, 0.0);
                update_find_match_label(ctx);
                return;
            }
        }
        ctx.source_buffer.select_range(&start, &end);
        ctx.source_view.scroll_to_iter(&mut start, 0.1, false, 0.0, 0.0);
    }
    update_find_match_label(ctx);
}

pub fn find_prev(ctx: &EditorCtx) {
    let cursor = ctx.source_buffer.iter_at_offset(ctx.source_buffer.cursor_position());
    if let Some((mut start, end, _wrapped)) = ctx.search_context.backward(&cursor) {
        ctx.source_buffer.select_range(&start, &end);
        ctx.source_view.scroll_to_iter(&mut start, 0.1, false, 0.0, 0.0);
    }
    update_find_match_label(ctx);
}

pub fn replace_one(ctx: &EditorCtx) {
    let replacement = ctx.replace_entry.text().to_string();
    if let Some((mut start, mut end)) = ctx.source_buffer.selection_bounds() {
        let _ = ctx.search_context.replace(&mut start, &mut end, &replacement);
        process_buffer_change(ctx);
        find_next(ctx);
    }
}

pub fn replace_all(ctx: &EditorCtx) {
    let replacement = ctx.replace_entry.text().to_string();
    let mut count = 0;
    let mut iter = ctx.source_buffer.start_iter();
    while let Some((mut start, mut end, _)) = ctx.search_context.forward(&iter) {
        if ctx.search_context.replace(&mut start, &mut end, &replacement).is_ok() {
            count += 1;
            iter = start; // after replacement, start is moved past the replacement
        } else {
            break;
        }
    }
    if count > 0 {
        process_buffer_change(ctx);
        send_toast(ctx, &format!("Replaced {count} occurrence(s)"));
    }
    update_find_match_label(ctx);
}

fn update_find_match_label(ctx: &EditorCtx) {
    let count = ctx.search_context.occurrences_count();
    if count < 0 {
        ctx.find_match_label.set_label(""); // still counting
    } else if count == 0 {
        ctx.find_match_label.set_label("No matches");
    } else {
        ctx.find_match_label.set_label(&format!("{count} match(es)"));
    }
}

pub fn wire_find_replace_signals(ctx: &EditorCtx) {
    use sourceview5::prelude::*;
    // Update search as user types
    {
        let find_entry = ctx.find_entry.clone();
        let ctx = ctx.clone();
        find_entry.connect_search_changed(move |entry| {
            let text = entry.text().to_string();
            if text.is_empty() {
                ctx.search_settings.set_search_text(None);
            } else {
                ctx.search_settings.set_search_text(Some(&text));
                ctx.search_context.set_highlight(true);
            }
            update_find_match_label(&ctx);
        });
    }
    // Enter → find next, Shift+Enter → find prev
    {
        let find_entry = ctx.find_entry.clone();
        let ctx = ctx.clone();
        let key_ctrl = gtk::EventControllerKey::new();
        key_ctrl.connect_key_pressed(move |_, key, _, mods| {
            if key == gdk::Key::Return || key == gdk::Key::KP_Enter {
                if mods.contains(gdk::ModifierType::SHIFT_MASK) {
                    find_prev(&ctx);
                } else {
                    find_next(&ctx);
                }
                return glib::Propagation::Stop;
            }
            if key == gdk::Key::Escape {
                hide_find_bar(&ctx);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        find_entry.add_controller(key_ctrl);
    }
    // Escape from replace entry
    {
        let replace_entry = ctx.replace_entry.clone();
        let ctx = ctx.clone();
        let key_ctrl = gtk::EventControllerKey::new();
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                hide_find_bar(&ctx);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        replace_entry.add_controller(key_ctrl);
    }
    // Update match count when occurrences change
    {
        let search_context = ctx.search_context.clone();
        let ctx = ctx.clone();
        search_context.connect_occurrences_count_notify(move |_| {
            update_find_match_label(&ctx);
        });
    }
}
