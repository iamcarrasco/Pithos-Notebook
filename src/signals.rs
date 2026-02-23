use adw::prelude::*;
use sourceview5::prelude::*;
use sourceview5 as sourceview;
use std::{cell::RefCell, collections::HashMap, rc::Rc, path::PathBuf, fs};
use crate::state::*;
use crate::crypto;
use crate::vault;
use crate::ui::types::*;
use crate::*;

#[allow(clippy::type_complexity)]
pub fn build_content_menu() -> gtk::gio::Menu {
    let menu = gtk::gio::Menu::new();

    let section1 = gtk::gio::Menu::new();
    section1.append(Some("Rename Note"), Some("win.rename-note"));
    section1.append(Some("Save Snapshot"), Some("win.save-snapshot"));
    section1.append(Some("View Backlinks"), Some("win.view-backlinks"));
    section1.append(Some("Version History"), Some("win.version-history"));
    section1.append(Some("Move to Folder\u{2026}"), Some("win.move-to-folder"));
    section1.append(Some("Export as Markdown\u{2026}"), Some("win.export-markdown"));
    section1.append(Some("Export as HTML\u{2026}"), Some("win.export-html"));
    menu.append_section(None, &section1);

    let section2 = gtk::gio::Menu::new();
    section2.append(Some("Zen Mode"), Some("win.zen-mode"));
    section2.append(Some("Fullscreen"), Some("win.fullscreen"));
    menu.append_section(None, &section2);

    let section3 = gtk::gio::Menu::new();
    section3.append(Some("Lock Vault"), Some("win.lock-vault"));
    section3.append(Some("Change Vault\u{2026}"), Some("win.change-vault"));
    menu.append_section(None, &section3);

    let section4 = gtk::gio::Menu::new();
    section4.append(Some("Help"), Some("win.show-help"));
    section4.append(Some("Keyboard Shortcuts"), Some("win.show-shortcuts"));
    section4.append(Some("About Pithos Notebook"), Some("win.show-about"));
    menu.append_section(None, &section4);

    menu
}

fn sort_order_action_key(order: SortOrder) -> &'static str {
    match order {
        SortOrder::Manual => "manual",
        SortOrder::ModifiedDesc => "modified-desc",
        SortOrder::ModifiedAsc => "modified-asc",
        SortOrder::NameAsc => "name-asc",
        SortOrder::NameDesc => "name-desc",
        SortOrder::CreatedDesc => "created-desc",
        SortOrder::CreatedAsc => "created-asc",
    }
}

pub fn wire_menu_actions(ctx: &EditorCtx) {
    use gtk::gio::SimpleAction;

    let window = &ctx.window;

    // --- Sidebar menu actions ---

    // Sort order (stateful string action)
    let current_sort = sort_order_action_key(ctx.state.borrow().sort_order).to_string();
    let sort_action = SimpleAction::new_stateful(
        "sort-order",
        Some(&String::static_variant_type()),
        &current_sort.to_variant(),
    );
    {
        let ctx = ctx.clone();
        sort_action.connect_activate(move |action, param| {
            let Some(val) = param.and_then(|p| p.get::<String>()) else { return };
            action.set_state(&val.to_variant());
            let order = match val.as_str() {
                "manual" => SortOrder::Manual,
                "modified-desc" => SortOrder::ModifiedDesc,
                "modified-asc" => SortOrder::ModifiedAsc,
                "name-asc" => SortOrder::NameAsc,
                "name-desc" => SortOrder::NameDesc,
                "created-desc" => SortOrder::CreatedDesc,
                "created-asc" => SortOrder::CreatedAsc,
                _ => SortOrder::Manual,
            };
            ctx.state.borrow_mut().sort_order = order;
            refresh_note_list(&ctx);
            trigger_vault_save(&ctx);
        });
    }
    window.add_action(&sort_action);

    // New folder
    let action = SimpleAction::new("new-folder", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| create_folder(&ctx, None)); }
    window.add_action(&action);

    // New from template
    let action = SimpleAction::new("new-from-template", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| show_template_picker(&ctx)); }
    window.add_action(&action);

    // View trash
    let action = SimpleAction::new("view-trash", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            let viewing = ctx.state.borrow().viewing_trash;
            if viewing {
                ctx.state.borrow_mut().viewing_trash = false;
                refresh_note_list(&ctx);
            } else {
                ctx.state.borrow_mut().viewing_trash = true;
                refresh_trash_view(&ctx);
            }
        });
    }
    window.add_action(&action);

    // Toggle theme
    let action = SimpleAction::new("toggle-theme", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| toggle_theme(&ctx)); }
    window.add_action(&action);

    // --- Content menu actions ---

    // Rename note
    let action = SimpleAction::new("rename-note", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| rename_note_dialog(&ctx)); }
    window.add_action(&action);

    // Save snapshot
    let action = SimpleAction::new("save-snapshot", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| save_manual_snapshot(&ctx)); }
    window.add_action(&action);

    // View backlinks
    let action = SimpleAction::new("view-backlinks", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| show_backlinks_dialog(&ctx)); }
    window.add_action(&action);

    // Version history
    let action = SimpleAction::new("version-history", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| show_history_dialog(&ctx)); }
    window.add_action(&action);

    // Move to folder
    let action = SimpleAction::new("move-to-folder", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| move_note_to_folder(&ctx)); }
    window.add_action(&action);

    // Export as markdown
    let action = SimpleAction::new("export-markdown", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| save_document_as(&ctx)); }
    window.add_action(&action);

    // Export as HTML
    let action = SimpleAction::new("export-html", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| export_as_html(&ctx)); }
    window.add_action(&action);

    // Zen mode
    let action = SimpleAction::new("zen-mode", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| toggle_zen_mode(&ctx)); }
    window.add_action(&action);

    // Fullscreen
    let action = SimpleAction::new("fullscreen", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| toggle_fullscreen(&ctx)); }
    window.add_action(&action);

    // --- Sidebar header button actions (not in menu) ---

    // New note (from the + button in sidebar header)
    let action = SimpleAction::new("new-note", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            create_note(
                &ctx,
                "Untitled Note".to_string(),
                "# Untitled Note\n\n".to_string(),
                Vec::new(),
            );
        });
    }
    window.add_action(&action);

    // Save vault (Ctrl+S)
    let action = SimpleAction::new("save-vault", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| save_document(&ctx)); }
    window.add_action(&action);

    // Import file (Ctrl+O)
    let action = SimpleAction::new("import-file", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| open_document(&ctx)); }
    window.add_action(&action);

    // Delete note
    let action = SimpleAction::new("delete-note", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| delete_note(&ctx)); }
    window.add_action(&action);

    // Empty trash
    let action = SimpleAction::new("empty-trash", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| empty_trash_action(&ctx)); }
    window.add_action(&action);

    // Daily note
    let action = SimpleAction::new("daily-note", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| open_or_create_daily_note(&ctx)); }
    window.add_action(&action);

    // Toggle sidebar
    let action = SimpleAction::new("toggle-sidebar", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| toggle_sidebar(&ctx)); }
    window.add_action(&action);

    // --- Shortcut-only actions (no menu entry) ---

    // Save as (Ctrl+Shift+S)
    let action = SimpleAction::new("save-as", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| save_document_as(&ctx)); }
    window.add_action(&action);

    // Close tab (Ctrl+W)
    let action = SimpleAction::new("close-tab", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            let note_id = ctx.state.borrow().active_note_id.clone();
            close_tab(&ctx, &note_id);
        });
    }
    window.add_action(&action);

    // Undo (Ctrl+Z)
    let action = SimpleAction::new("undo", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| undo(&ctx)); }
    window.add_action(&action);

    // Redo (Ctrl+Shift+Z / Ctrl+Y)
    let action = SimpleAction::new("redo", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| redo(&ctx)); }
    window.add_action(&action);

    // Focus search (Ctrl+Shift+F)
    let action = SimpleAction::new("focus-search", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            ctx.search_bar.set_search_mode(true);
            ctx.search_entry.grab_focus();
        });
    }
    window.add_action(&action);

    // --- Formatting actions ---

    let action = SimpleAction::new("fmt-bold", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        md_wrap(&ctx.source_buffer, "**", "**", "bold text");
        process_buffer_change(&ctx);
    }); }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-italic", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        md_wrap(&ctx.source_buffer, "*", "*", "italic text");
        process_buffer_change(&ctx);
    }); }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-underline", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        md_wrap(&ctx.source_buffer, "<u>", "</u>", "underlined");
        process_buffer_change(&ctx);
    }); }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-strike", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        md_wrap(&ctx.source_buffer, "~~", "~~", "strikethrough");
        process_buffer_change(&ctx);
    }); }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-code", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        md_wrap(&ctx.source_buffer, "`", "`", "code");
        process_buffer_change(&ctx);
    }); }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-link", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        insert_link(&ctx);
    }); }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-h1", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        md_line_prefix(&ctx.source_buffer, "# ");
        process_buffer_change(&ctx);
    }); }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-h2", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        md_line_prefix(&ctx.source_buffer, "## ");
        process_buffer_change(&ctx);
    }); }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-h3", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        md_line_prefix(&ctx.source_buffer, "### ");
        process_buffer_change(&ctx);
    }); }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-h4", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        md_line_prefix(&ctx.source_buffer, "#### ");
        process_buffer_change(&ctx);
    }); }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-h5", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        md_line_prefix(&ctx.source_buffer, "##### ");
        process_buffer_change(&ctx);
    }); }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-h6", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        md_line_prefix(&ctx.source_buffer, "###### ");
        process_buffer_change(&ctx);
    }); }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-quote", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        md_line_prefix(&ctx.source_buffer, "> ");
        process_buffer_change(&ctx);
    }); }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-bullet-list", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        md_line_prefix(&ctx.source_buffer, "- ");
        process_buffer_change(&ctx);
    }); }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-ordered-list", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        md_line_prefix(&ctx.source_buffer, "1. ");
        process_buffer_change(&ctx);
    }); }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-task-list", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        md_line_prefix(&ctx.source_buffer, "- [ ] ");
        process_buffer_change(&ctx);
    }); }
    window.add_action(&action);

    let action = SimpleAction::new("toggle-checkbox", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| {
        toggle_checkbox_at_cursor(&ctx.source_buffer);
        process_buffer_change(&ctx);
    }); }
    window.add_action(&action);

    // Command palette
    let action = SimpleAction::new("command-palette", None);
    { let ctx = ctx.clone(); action.connect_activate(move |_, _| show_command_palette(&ctx)); }
    window.add_action(&action);

    // Lock vault — save, clear editor, go to unlock (keeps vault path)
    let action = SimpleAction::new("lock-vault", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            perform_vault_save_sync(&ctx);
            let vault_folder = ctx.vault_folder.borrow().clone();
            ctx.window.set_content(gtk::Widget::NONE);
            show_unlock_vault_dialog(&ctx.window, vault_folder);
        });
    }
    window.add_action(&action);

    // Change vault — save, clear config, go to welcome (pick new/existing)
    let action = SimpleAction::new("change-vault", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            perform_vault_save_sync(&ctx);
            let _ = vault::save_config(&vault::AppConfig { vault_path: None });
            ctx.window.set_content(gtk::Widget::NONE);
            show_welcome_dialog(&ctx.window);
        });
    }
    window.add_action(&action);

    // Keyboard shortcuts window
    let action = SimpleAction::new("show-shortcuts", None);
    {
        let window = window.clone();
        action.connect_activate(move |_, _| {
            show_shortcuts_window(&window);
        });
    }
    window.add_action(&action);

    // About dialog
    let action = SimpleAction::new("show-about", None);
    {
        let window = window.clone();
        action.connect_activate(move |_, _| {
            show_about_dialog(&window);
        });
    }
    window.add_action(&action);

    // Help dialog
    let action = SimpleAction::new("show-help", None);
    {
        let window = window.clone();
        action.connect_activate(move |_, _| {
            show_help_dialog(&window);
        });
    }
    window.add_action(&action);
}





pub fn initialize_state(ctx: &EditorCtx) {
    // Load the active note's content into the source buffer
    let active_content = {
        let state = ctx.state.borrow();
        find_note_index(&state.notes, &state.active_note_id)
            .map(|i| state.notes[i].content.clone())
            .unwrap_or_default()
    };
    {
        let mut state = ctx.state.borrow_mut();
        state.suppress_sync = true;
    }
    ctx.source_buffer.set_text(&active_content);
    {
        let mut state = ctx.state.borrow_mut();
        state.suppress_sync = false;
    }

    let initial_markdown = sync_markdown_and_status(ctx);
    {
        let mut state = ctx.state.borrow_mut();
        state.saved_snapshot = initial_markdown.clone();
        state.last_snapshot = initial_markdown;
        state.dirty = false;
        state.undo_stack.clear();
        state.redo_stack.clear();
    }
    refresh_header(ctx);
    refresh_tabs(ctx);
    refresh_note_list(ctx);
    refresh_tags(ctx);
}

// ---------------------------------------------------------------------------
// Signal wiring — decomposed by area
// ---------------------------------------------------------------------------

pub fn wire_toolbar_signals(ctx: &EditorCtx, tb: &ToolbarWidgets) {
    {
        let ctx = ctx.clone();
        tb.undo.connect_clicked(move |_| undo(&ctx));
    }
    {
        let ctx = ctx.clone();
        tb.redo.connect_clicked(move |_| redo(&ctx));
    }
    {
        let ctx = ctx.clone();
        tb.bold.connect_clicked(move |_| {
            md_wrap(&ctx.source_buffer, "**", "**", "bold text");
            process_buffer_change(&ctx);
        });
    }
    {
        let ctx = ctx.clone();
        tb.italic.connect_clicked(move |_| {
            md_wrap(&ctx.source_buffer, "*", "*", "italic text");
            process_buffer_change(&ctx);
        });
    }
    {
        let ctx = ctx.clone();
        tb.underline.connect_clicked(move |_| {
            md_wrap(&ctx.source_buffer, "<u>", "</u>", "underlined");
            process_buffer_change(&ctx);
        });
    }
    {
        let ctx = ctx.clone();
        tb.strike.connect_clicked(move |_| {
            md_wrap(&ctx.source_buffer, "~~", "~~", "strikethrough");
            process_buffer_change(&ctx);
        });
    }
    {
        let ctx = ctx.clone();
        tb.code.connect_clicked(move |_| {
            md_wrap(&ctx.source_buffer, "`", "`", "code");
            process_buffer_change(&ctx);
        });
    }
    {
        let ctx = ctx.clone();
        let popover = tb.block_popover.clone();
        tb.block_paragraph.connect_clicked(move |_| {
            // Strip any line prefix (revert to plain paragraph)
            let buffer = &ctx.source_buffer;
            let cursor = buffer.iter_at_offset(buffer.cursor_position());
            let line = cursor.line();
            let mut line_start = buffer.iter_at_line(line).unwrap_or(cursor);
            let mut line_end = line_start;
            if !line_end.ends_line() { line_end.forward_to_line_end(); }
            let line_text = buffer.text(&line_start, &line_end, true).to_string();
            let stripped = strip_line_prefix(&line_text);
            buffer.delete(&mut line_start, &mut line_end);
            let mut ins = buffer.iter_at_line(line).unwrap_or(buffer.start_iter());
            buffer.insert(&mut ins, stripped);
            process_buffer_change(&ctx);
            popover.popdown();
        });
    }
    {
        let ctx = ctx.clone();
        let popover = tb.block_popover.clone();
        tb.block_h1.connect_clicked(move |_| {
            md_line_prefix(&ctx.source_buffer, "# ");
            process_buffer_change(&ctx);
            popover.popdown();
        });
    }
    {
        let ctx = ctx.clone();
        let popover = tb.block_popover.clone();
        tb.block_h2.connect_clicked(move |_| {
            md_line_prefix(&ctx.source_buffer, "## ");
            process_buffer_change(&ctx);
            popover.popdown();
        });
    }
    {
        let ctx = ctx.clone();
        let popover = tb.block_popover.clone();
        tb.block_h3.connect_clicked(move |_| {
            md_line_prefix(&ctx.source_buffer, "### ");
            process_buffer_change(&ctx);
            popover.popdown();
        });
    }
    {
        let ctx = ctx.clone();
        let popover = tb.block_popover.clone();
        tb.block_h4.connect_clicked(move |_| {
            md_line_prefix(&ctx.source_buffer, "#### ");
            process_buffer_change(&ctx);
            popover.popdown();
        });
    }
    {
        let ctx = ctx.clone();
        let popover = tb.block_popover.clone();
        tb.block_h5.connect_clicked(move |_| {
            md_line_prefix(&ctx.source_buffer, "##### ");
            process_buffer_change(&ctx);
            popover.popdown();
        });
    }
    {
        let ctx = ctx.clone();
        let popover = tb.block_popover.clone();
        tb.block_h6.connect_clicked(move |_| {
            md_line_prefix(&ctx.source_buffer, "###### ");
            process_buffer_change(&ctx);
            popover.popdown();
        });
    }
    {
        let ctx = ctx.clone();
        let popover = tb.block_popover.clone();
        tb.block_quote.connect_clicked(move |_| {
            md_line_prefix(&ctx.source_buffer, "> ");
            process_buffer_change(&ctx);
            popover.popdown();
        });
    }
    {
        let ctx = ctx.clone();
        tb.bullet.connect_clicked(move |_| {
            md_line_prefix(&ctx.source_buffer, "- ");
            process_buffer_change(&ctx);
        });
    }
    {
        let ctx = ctx.clone();
        tb.ordered.connect_clicked(move |_| {
            md_line_prefix(&ctx.source_buffer, "1. ");
            process_buffer_change(&ctx);
        });
    }
    {
        let ctx = ctx.clone();
        tb.task.connect_clicked(move |_| {
            md_line_prefix(&ctx.source_buffer, "- [ ] ");
            process_buffer_change(&ctx);
        });
    }
    {
        let ctx = ctx.clone();
        tb.link.connect_clicked(move |_| {
            insert_link(&ctx);
        });
    }
    {
        let ctx = ctx.clone();
        tb.table.connect_clicked(move |_| {
            insert_table_snippet(&ctx.source_buffer);
            process_buffer_change(&ctx);
        });
    }
    {
        let ctx = ctx.clone();
        tb.rule.connect_clicked(move |_| {
            let buffer = &ctx.source_buffer;
            let mut iter = buffer.iter_at_offset(buffer.cursor_position());
            buffer.insert(&mut iter, "\n---\n");
            process_buffer_change(&ctx);
        });
    }
    {
        let ctx = ctx.clone();
        tb.fullscreen.connect_clicked(move |_| toggle_fullscreen(&ctx));
    }
    {
        let ctx = ctx.clone();
        tb.image.connect_clicked(move |_| insert_image_snippet(&ctx));
    }
    for (lang, btn) in &tb.code_languages {
        let ctx = ctx.clone();
        let popover = tb.code_block_popover.clone();
        let lang = lang.clone();
        btn.connect_clicked(move |_| {
            insert_code_block_snippet_with_language(&ctx.source_buffer, &lang);
            process_buffer_change(&ctx);
            popover.popdown();
        });
    }
}

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
    let move_btn = gtk::Button::with_label("Move to Folder...");
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

    let dialog = adw::AlertDialog::new(Some("Rename Folder"), Some("Enter a new name for the folder."));

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

pub fn delete_folder(ctx: &EditorCtx, folder_id: &str) {
    let dialog = adw::AlertDialog::new(
        Some("Delete Folder?"),
        Some("Notes inside this folder will be moved to the root level."),
    );

    dialog.add_response("cancel", "Cancel");
    dialog.add_response("delete", "Delete");
    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
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

}

pub fn wire_keyboard_shortcuts(
    ctx: &EditorCtx,
    window: &adw::ApplicationWindow,
) {
    // Use set_accels_for_action — the standard GNOME pattern.
    // This integrates with GtkShortcutsWindow and respects focus handling
    // (accelerators won't fire from dialog entries in separate windows).
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
// Small UI helpers
// ---------------------------------------------------------------------------

pub fn icon_button(icon_name: &str, tooltip: &str) -> gtk::Button {
    let image = gtk::Image::from_icon_name(icon_name);
    image.set_pixel_size(21);

    let button = gtk::Button::new();
    button.set_child(Some(&image));
    button.set_tooltip_text(Some(tooltip));
    set_accessible_label(&button, tooltip);
    button.add_css_class("flat");
    button.add_css_class("toolbar-icon");
    button
}

pub fn symbol_button(label: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.set_tooltip_text(Some(tooltip));
    set_accessible_label(&button, tooltip);
    button.add_css_class("flat");
    button.add_css_class("toolbar-symbol");
    button
}

pub fn toolbar_separator() -> gtk::Separator {
    let separator = gtk::Separator::new(gtk::Orientation::Vertical);
    separator.set_margin_start(8);
    separator.set_margin_end(8);
    separator.add_css_class("toolbar-separator");
    separator
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

pub fn set_accessible_label(widget: &impl gtk::prelude::AccessibleExt, label: &str) {
    widget.update_property(&[gtk::accessible::Property::Label(label)]);
}

pub fn format_ts(ts: i64) -> String {
    if let Ok(dt) = glib::DateTime::from_unix_local(ts) {
        if let Ok(s) = dt.format("%Y-%m-%d %H:%M") {
            return s.to_string();
        }
    }
    ts.to_string()
}

pub fn find_note_index(notes: &[NoteItem], id: &str) -> Option<usize> {
    notes.iter().position(|note| note.id == id)
}

/// Returns true if a note with `name` (case-insensitive) already exists in `folder_id`,
/// optionally excluding the note with `exclude_id` (for renames).
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

/// Returns a unique note name by appending " (2)", " (3)", etc. if needed.
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

/// Returns true if a folder with `name` (case-insensitive) already exists under `parent_id`,
/// optionally excluding the folder with `exclude_id` (for renames).
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
// Tabs & note list — optimized to avoid cloning full note content
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

pub fn apply_note_sort(visible: &mut [NoteSummary], sort_order: SortOrder) {
    match sort_order {
        SortOrder::Manual => {}
        SortOrder::ModifiedDesc => visible.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
        SortOrder::ModifiedAsc => visible.sort_by(|a, b| a.updated_at.cmp(&b.updated_at)),
        SortOrder::NameAsc => {
            visible.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        }
        SortOrder::NameDesc => {
            visible.sort_by(|a, b| b.name.to_lowercase().cmp(&a.name.to_lowercase()))
        }
        SortOrder::CreatedDesc => visible.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        SortOrder::CreatedAsc => visible.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
    }
    // Pinned notes always come first (stable sort preserves order within each group)
    visible.sort_by_key(|n| if n.pinned { 0 } else { 1 });
}

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

    let (mut visible, active_id, sort_order, search_query, folders, filter_active) = {
        let state = ctx.state.borrow();
        let query = state.search_query.to_lowercase();
        let filter_tags = state.filter_tags.clone();
        let tag_filter_and = state.tag_filter_and;
        let filter_active = !query.trim().is_empty() || !filter_tags.is_empty();

        let visible: Vec<NoteSummary> = state
            .notes
            .iter()
            .filter(|note| {
                let matches_search = query.trim().is_empty()
                    || note.name.to_lowercase().contains(&query)
                    || note.content.to_lowercase().contains(&query);
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
                content_snippet: make_search_snippet(&note.content, &query),
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
            query,
            state.folders.clone(),
            filter_active,
        )
    };

    apply_note_sort(&mut visible, sort_order);

    let mut row_items: Vec<SidebarRowKind> = Vec::new();

    if filter_active {
        // Flat mode during search — no folder hierarchy
        for note in &visible {
            let row = build_note_row(ctx, note, &active_id, &search_query, 0);
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

pub fn apply_note_sort_refs(notes: &mut Vec<&NoteSummary>, sort_order: SortOrder) {
    match sort_order {
        SortOrder::Manual => {}
        SortOrder::ModifiedDesc => notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
        SortOrder::ModifiedAsc => notes.sort_by(|a, b| a.updated_at.cmp(&b.updated_at)),
        SortOrder::NameAsc => {
            notes.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        }
        SortOrder::NameDesc => {
            notes.sort_by(|a, b| b.name.to_lowercase().cmp(&a.name.to_lowercase()))
        }
        SortOrder::CreatedDesc => notes.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        SortOrder::CreatedAsc => notes.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
    }
    notes.sort_by_key(|n| if n.pinned { 0 } else { 1 });
}

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
    mode_btn.set_tooltip_text(Some("Toggle AND/OR filter mode"));
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
        remove_btn.set_tooltip_text(Some("Remove tag from filter"));
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
    clear_btn.set_tooltip_text(Some("Clear all tag filters"));
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

pub fn close_tab(ctx: &EditorCtx, note_id: &str) {
    if ctx.state.borrow().open_tabs.len() <= 1 {
        return;
    }
    // Find the TabView page for this note and ask AdwTabView to close it.
    // The close-page signal handler will update state and switch notes.
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
// Dialogs
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

// ---------------------------------------------------------------------------
// Command palette
// ---------------------------------------------------------------------------

pub struct CommandEntry {
    label: String,
    accel: String,
    action_name: String,
}

pub fn build_command_entries() -> Vec<CommandEntry> {
    vec![
        CommandEntry { label: "New Note".into(), accel: "Ctrl+N".into(), action_name: "win.new-note".into() },
        CommandEntry { label: "Save".into(), accel: "Ctrl+S".into(), action_name: "win.save-vault".into() },
        CommandEntry { label: "Save As\u{2026}".into(), accel: "Ctrl+Shift+S".into(), action_name: "win.save-as".into() },
        CommandEntry { label: "Import File\u{2026}".into(), accel: "Ctrl+O".into(), action_name: "win.import-file".into() },
        CommandEntry { label: "Rename Note".into(), accel: "F2".into(), action_name: "win.rename-note".into() },
        CommandEntry { label: "Delete Note".into(), accel: "".into(), action_name: "win.delete-note".into() },
        CommandEntry { label: "Close Tab".into(), accel: "Ctrl+W".into(), action_name: "win.close-tab".into() },
        CommandEntry { label: "Undo".into(), accel: "Ctrl+Z".into(), action_name: "win.undo".into() },
        CommandEntry { label: "Redo".into(), accel: "Ctrl+Shift+Z".into(), action_name: "win.redo".into() },
        CommandEntry { label: "Toggle Sidebar".into(), accel: "Ctrl+\\".into(), action_name: "win.toggle-sidebar".into() },
        CommandEntry { label: "Zen Mode".into(), accel: "Ctrl+Shift+J".into(), action_name: "win.zen-mode".into() },
        CommandEntry { label: "Fullscreen".into(), accel: "F11".into(), action_name: "win.fullscreen".into() },
        CommandEntry { label: "Toggle Theme".into(), accel: "Ctrl+Shift+D".into(), action_name: "win.toggle-theme".into() },
        CommandEntry { label: "Focus Search".into(), accel: "Ctrl+Shift+F".into(), action_name: "win.focus-search".into() },
        CommandEntry { label: "Daily Note".into(), accel: "Ctrl+Shift+T".into(), action_name: "win.daily-note".into() },
        CommandEntry { label: "New Folder".into(), accel: "".into(), action_name: "win.new-folder".into() },
        CommandEntry { label: "New from Template\u{2026}".into(), accel: "".into(), action_name: "win.new-from-template".into() },
        CommandEntry { label: "View Trash".into(), accel: "".into(), action_name: "win.view-trash".into() },
        CommandEntry { label: "Save Snapshot".into(), accel: "".into(), action_name: "win.save-snapshot".into() },
        CommandEntry { label: "View Backlinks".into(), accel: "".into(), action_name: "win.view-backlinks".into() },
        CommandEntry { label: "Version History".into(), accel: "".into(), action_name: "win.version-history".into() },
        CommandEntry { label: "Move to Folder\u{2026}".into(), accel: "".into(), action_name: "win.move-to-folder".into() },
        CommandEntry { label: "Export as Markdown\u{2026}".into(), accel: "".into(), action_name: "win.export-markdown".into() },
        CommandEntry { label: "Export as HTML\u{2026}".into(), accel: "".into(), action_name: "win.export-html".into() },
        CommandEntry { label: "Bold".into(), accel: "Ctrl+B".into(), action_name: "win.fmt-bold".into() },
        CommandEntry { label: "Italic".into(), accel: "Ctrl+I".into(), action_name: "win.fmt-italic".into() },
        CommandEntry { label: "Underline".into(), accel: "Ctrl+U".into(), action_name: "win.fmt-underline".into() },
        CommandEntry { label: "Strikethrough".into(), accel: "Ctrl+D".into(), action_name: "win.fmt-strike".into() },
        CommandEntry { label: "Inline Code".into(), accel: "Ctrl+E".into(), action_name: "win.fmt-code".into() },
        CommandEntry { label: "Insert Link".into(), accel: "Ctrl+K".into(), action_name: "win.fmt-link".into() },
        CommandEntry { label: "Heading 1".into(), accel: "Ctrl+1".into(), action_name: "win.fmt-h1".into() },
        CommandEntry { label: "Heading 2".into(), accel: "Ctrl+2".into(), action_name: "win.fmt-h2".into() },
        CommandEntry { label: "Heading 3".into(), accel: "Ctrl+3".into(), action_name: "win.fmt-h3".into() },
        CommandEntry { label: "Heading 4".into(), accel: "Ctrl+4".into(), action_name: "win.fmt-h4".into() },
        CommandEntry { label: "Heading 5".into(), accel: "Ctrl+5".into(), action_name: "win.fmt-h5".into() },
        CommandEntry { label: "Heading 6".into(), accel: "Ctrl+6".into(), action_name: "win.fmt-h6".into() },
        CommandEntry { label: "Block Quote".into(), accel: "Ctrl+Shift+Q".into(), action_name: "win.fmt-quote".into() },
        CommandEntry { label: "Bullet List".into(), accel: "Ctrl+Shift+L".into(), action_name: "win.fmt-bullet-list".into() },
        CommandEntry { label: "Ordered List".into(), accel: "".into(), action_name: "win.fmt-ordered-list".into() },
        CommandEntry { label: "Task List".into(), accel: "".into(), action_name: "win.fmt-task-list".into() },
        CommandEntry { label: "Toggle Checkbox".into(), accel: "Ctrl+Space".into(), action_name: "win.toggle-checkbox".into() },
        CommandEntry { label: "Help".into(), accel: "F1".into(), action_name: "win.show-help".into() },
    ]
}

pub fn show_command_palette(ctx: &EditorCtx) {
    let entries = build_command_entries();

    let dialog = adw::Window::builder()
        .transient_for(&ctx.window)
        .modal(true)
        .title("Command Palette")
        .default_width(480)
        .default_height(420)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let header = adw::HeaderBar::new();
    header.set_show_title(false);
    vbox.append(&header);

    let search_entry = gtk::SearchEntry::new();
    search_entry.set_placeholder_text(Some("Type a command\u{2026}"));
    search_entry.set_margin_start(12);
    search_entry.set_margin_end(12);
    search_entry.set_margin_bottom(8);
    vbox.append(&search_entry);

    let list_box = gtk::ListBox::new();
    list_box.set_selection_mode(gtk::SelectionMode::Single);
    list_box.add_css_class("navigation-sidebar");

    for entry in &entries {
        let row = gtk::ListBoxRow::new();
        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row_box.set_margin_start(12);
        row_box.set_margin_end(12);
        row_box.set_margin_top(6);
        row_box.set_margin_bottom(6);

        let label = gtk::Label::new(Some(&entry.label));
        label.set_xalign(0.0);
        label.set_hexpand(true);
        row_box.append(&label);

        if !entry.accel.is_empty() {
            let accel_label = gtk::Label::new(Some(&entry.accel));
            accel_label.add_css_class("dim-label");
            accel_label.add_css_class("caption");
            row_box.append(&accel_label);
        }

        row.set_child(Some(&row_box));
        // Store action name in widget name for lookup (not tooltip)
        row.set_widget_name(&entry.action_name);
        list_box.append(&row);
    }

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&list_box)
        .build();
    vbox.append(&scroll);

    dialog.set_content(Some(&vbox));

    // Filter rows on search
    let list_ref = list_box.clone();
    search_entry.connect_search_changed(move |entry| {
        let query = entry.text().to_string().to_lowercase();
        let mut idx = 0;
        while let Some(row) = list_ref.row_at_index(idx) {
            let matches = if query.is_empty() {
                true
            } else if let Some(child) = row.child() {
                if let Some(row_box) = child.downcast_ref::<gtk::Box>() {
                    if let Some(first) = row_box.first_child() {
                        if let Some(lbl) = first.downcast_ref::<gtk::Label>() {
                            lbl.text().to_lowercase().contains(&query)
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                } else {
                    true
                }
            } else {
                true
            };
            row.set_visible(matches);
            idx += 1;
        }
        // Select first visible row
        let mut i = 0;
        while let Some(row) = list_ref.row_at_index(i) {
            if row.is_visible() {
                list_ref.select_row(Some(&row));
                break;
            }
            i += 1;
        }
    });

    // Activate on Enter
    {
        let dialog_ref = dialog.clone();
        let window_ref = ctx.window.clone();
        let list_ref = list_box.clone();
        search_entry.connect_activate(move |_| {
            if let Some(row) = list_ref.selected_row() {
                let action_name = row.widget_name();
                if !action_name.is_empty() {
                    dialog_ref.close();
                    activate_action_by_name(&window_ref, &action_name);
                }
            }
        });
    }

    // Activate on row click
    {
        let dialog_ref = dialog.clone();
        let window_ref = ctx.window.clone();
        list_box.connect_row_activated(move |_, row| {
            let action_name = row.widget_name();
            if !action_name.is_empty() {
                dialog_ref.close();
                activate_action_by_name(&window_ref, &action_name);
            }
        });
    }

    // Navigate list with arrow keys from search entry
    {
        let list_ref = list_box.clone();
        let key_ctl = gtk::EventControllerKey::new();
        key_ctl.connect_key_pressed(move |_, key, _, _| {
            match key {
                gdk::Key::Down => {
                    if let Some(row) = list_ref.selected_row() {
                        let idx = row.index();
                        // Find next visible row
                        let mut next = idx + 1;
                        while let Some(r) = list_ref.row_at_index(next) {
                            if r.is_visible() {
                                list_ref.select_row(Some(&r));
                                break;
                            }
                            next += 1;
                        }
                    }
                    glib::Propagation::Stop
                }
                gdk::Key::Up => {
                    if let Some(row) = list_ref.selected_row() {
                        let idx = row.index();
                        let mut prev = idx - 1;
                        while prev >= 0 {
                            if let Some(r) = list_ref.row_at_index(prev) {
                                if r.is_visible() {
                                    list_ref.select_row(Some(&r));
                                    break;
                                }
                            }
                            prev -= 1;
                        }
                    }
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        search_entry.add_controller(key_ctl);
    }

    // Select first row initially
    if let Some(first) = list_box.row_at_index(0) {
        list_box.select_row(Some(&first));
    }

    let dialog_for_close = dialog.clone();
    dialog.connect_close_request(move |_| {
        gtk::prelude::GtkWindowExt::set_focus(&dialog_for_close, gtk::Widget::NONE);
        dialog_for_close.set_content(gtk::Widget::NONE);
        glib::Propagation::Proceed
    });

    dialog.present();
    search_entry.grab_focus();
}

pub fn activate_action_by_name(window: &adw::ApplicationWindow, action_name: &str) {
    use gtk::gio::prelude::ActionGroupExt;
    // action_name is like "win.foo" — strip prefix
    if let Some(name) = action_name.strip_prefix("win.") {
        ActionGroupExt::activate_action(window, name, None);
    }
}

pub fn show_backlinks_dialog(ctx: &EditorCtx) {
    let (_active_name, backlinks) = {
        let state = ctx.state.borrow();
        let name = find_note_index(&state.notes, &state.active_note_id)
            .map(|i| state.notes[i].name.clone())
            .unwrap_or_else(|| "Untitled".to_string());
        let token = format!("[[{name}]]").to_lowercase();
        let links: Vec<String> = state
            .notes
            .iter()
            .filter(|note| {
                note.id != state.active_note_id
                    && note.content.to_lowercase().contains(&token)
            })
            .map(|note| note.name.clone())
            .collect();
        (name, links)
    };

    let message = if backlinks.is_empty() {
        "No backlinks found for the active note.".to_string()
    } else {
        backlinks
            .iter()
            .enumerate()
            .map(|(idx, name)| format!("{}. {}", idx + 1, name))
            .collect::<Vec<_>>()
            .join("\n")
    };
    show_info(&ctx.window, "Backlinks", &message);
}

pub fn show_history_dialog(ctx: &EditorCtx) {
    let (versions, note_name) = {
        let state = ctx.state.borrow();
        let name = find_note_index(&state.notes, &state.active_note_id)
            .map(|i| state.notes[i].name.clone())
            .unwrap_or_else(|| "Untitled".to_string());
        let versions = find_note_index(&state.notes, &state.active_note_id)
            .map(|index| state.notes[index].versions.clone())
            .unwrap_or_default();
        (versions, name)
    };

    if versions.is_empty() {
        let dialog = adw::AlertDialog::new(
            Some(&format!("Version history: {note_name}")),
            Some("No snapshots available yet."),
        );
        dialog.add_response("close", "Close");
        dialog.set_close_response("close");
        dialog.present(Some(&ctx.window));
    } else {
        let mut text = String::new();
        let history_rows: Vec<(usize, String)> = versions
            .iter()
            .enumerate()
            .rev()
            .take(5)
            .map(|(idx, version)| (idx, format_ts(version.ts)))
            .collect();
        for (idx, (_, ts)) in history_rows.iter().enumerate() {
            text.push_str(&format!("{}. {ts}\n", idx + 1));
        }
        let dialog = adw::AlertDialog::new(
            Some(&format!("Version history: {note_name}")),
            Some(text.trim_end()),
        );
        dialog.add_response("close", "Close");
        for (display_idx, (version_idx, ts)) in history_rows.iter().enumerate() {
            let response = format!("restore-{version_idx}");
            let label = if display_idx == 0 {
                format!("Restore latest ({ts})")
            } else {
                format!("Restore {ts}")
            };
            dialog.add_response(&response, &label);
            if display_idx == 0 {
                dialog.set_response_appearance(&response, adw::ResponseAppearance::Suggested);
            }
        }
        dialog.set_close_response("close");

        let window = ctx.window.clone();
        let ctx = ctx.clone();
        dialog.connect_response(None, move |_, response| {
            if let Some(idx_str) = response.strip_prefix("restore-") {
                if let Ok(version_idx) = idx_str.parse::<usize>() {
                    restore_snapshot(&ctx, version_idx);
                }
            }
        });
        dialog.present(Some(&window));
    }
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

pub fn show_info(parent: &adw::ApplicationWindow, title: &str, message: &str) {
    let dialog = adw::AlertDialog::new(Some(title), Some(message));
    dialog.add_response("ok", "OK");
    dialog.set_close_response("ok");
    dialog.present(Some(parent));
}

pub fn show_error(parent: &adw::ApplicationWindow, title: &str, message: &str) {
    let dialog = adw::AlertDialog::new(Some(title), Some(message));
    dialog.add_response("ok", "OK");
    dialog.set_close_response("ok");
    dialog.present(Some(parent));
}

// ---------------------------------------------------------------------------
// Block type / formatting helpers
// ---------------------------------------------------------------------------

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

    // Escape exits zen mode
    if key == gdk::Key::Escape && ctx.state.borrow().zen_mode {
        toggle_zen_mode(ctx);
        return glib::Propagation::Stop;
    }

    glib::Propagation::Proceed
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

pub fn new_document(ctx: &EditorCtx) {
    create_note(
        ctx,
        "Untitled Note".to_string(),
        "# Untitled Note\n\n".to_string(),
        Vec::new(),
    );
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

pub fn toggle_fullscreen(ctx: &EditorCtx) {
    let fullscreen = {
        let mut state = ctx.state.borrow_mut();
        state.fullscreen = !state.fullscreen;
        state.fullscreen
    };

    if fullscreen {
        ctx.window.fullscreen();
    } else {
        ctx.window.unfullscreen();
    }
}

pub fn toggle_sidebar(ctx: &EditorCtx) {
    let show = !ctx.split_view.shows_sidebar();
    ctx.split_view.set_show_sidebar(show);
    ctx.state.borrow_mut().sidebar_visible = show;
}

pub fn toggle_zen_mode(ctx: &EditorCtx) {
    let zen = {
        let mut state = ctx.state.borrow_mut();
        state.zen_mode = !state.zen_mode;
        state.zen_mode
    };
    // In zen mode: hide sidebar, toolbar, tabs, breadcrumbs, preview pane
    ctx.split_view.set_show_sidebar(!zen);
    ctx.toolbar.set_visible(!zen);
    ctx.tab_bar.set_visible(!zen);
    ctx.breadcrumbs.set_visible(!zen);
    // In zen mode, hide the preview side pane
    if zen {
        ctx.preview_panel.set_visible(false);
        ctx.split.set_position(ctx.split.allocated_width());
    } else {
        ctx.preview_panel.set_visible(true);
        ctx.split.set_position(700);
        // Restore sidebar state
        let sidebar_visible = ctx.state.borrow().sidebar_visible;
        ctx.split_view.set_show_sidebar(sidebar_visible);
    }
}

pub fn send_toast(ctx: &EditorCtx, message: &str) {
    let toast = adw::Toast::new(message);
    toast.set_timeout(2);
    ctx.toast_overlay.add_toast(toast);
}

pub fn set_save_status(ctx: &EditorCtx, status: &str) {
    // Show save feedback as a toast notification
    if status == "Saved" || status.contains("Error") {
        send_toast(ctx, status);
    }
    // "Saving…" is transient — no toast needed
}

// ---------------------------------------------------------------------------
// File I/O
// ---------------------------------------------------------------------------

pub fn open_document(ctx: &EditorCtx) {
    let dialog = gtk::FileDialog::builder()
        .title("Open Markdown File")
        .accept_label("Open")
        .build();

    let filter = gtk::FileFilter::new();
    filter.set_name(Some("Markdown files"));
    filter.add_pattern("*.md");
    filter.add_pattern("*.markdown");
    filter.add_pattern("*.txt");
    let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    dialog.set_filters(Some(&filters));

    let ctx = ctx.clone();
    let window = ctx.window.clone();
    dialog.open(Some(&window), gtk::gio::Cancellable::NONE, move |result: Result<gtk::gio::File, gtk::glib::Error>| {
        match result {
            Ok(file) => {
                if let Some(path) = file.path() {
                    match fs::read_to_string(&path) {
                        Ok(text) => {
                            let name = path
                                .file_stem()
                                .and_then(|name| name.to_str())
                                .unwrap_or("Imported Note")
                                .to_string();
                            create_note(&ctx, name, text, vec!["imported".to_string()]);
                        }
                        Err(err) => show_error(
                            &ctx.window,
                            "Open failed",
                            &format!("Could not read file:\n{err}"),
                        ),
                    }
                } else {
                    show_error(
                        &ctx.window,
                        "Open failed",
                        "This file location is not a local path.",
                    );
                }
            }
            Err(e) => {
                if !e.matches(gtk::DialogError::Dismissed) {
                    send_toast(&ctx, &format!("Could not open file picker: {e}"));
                }
            }
        }
    });
}

pub fn save_document(ctx: &EditorCtx) {
    // Dirty flag is only cleared after successful write
    perform_vault_save_async(ctx, true);
}

pub fn save_document_as(ctx: &EditorCtx) {
    let dialog = gtk::FileDialog::builder()
        .title("Save Markdown File")
        .accept_label("Save")
        .build();

    let suggested = {
        let state = ctx.state.borrow();
        find_note_index(&state.notes, &state.active_note_id)
            .and_then(|i| state.notes[i].file_path.as_ref())
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled.md")
            .to_string()
    };
    dialog.set_initial_name(Some(&suggested));

    let filter = gtk::FileFilter::new();
    filter.set_name(Some("Markdown files"));
    filter.add_pattern("*.md");
    filter.add_pattern("*.markdown");
    filter.add_pattern("*.txt");
    let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    dialog.set_filters(Some(&filters));

    let ctx = ctx.clone();
    let window = ctx.window.clone();
    dialog.save(Some(&window), gtk::gio::Cancellable::NONE, move |result: Result<gtk::gio::File, gtk::glib::Error>| {
        match result {
            Ok(file) => {
                if let Some(mut path) = file.path() {
                    if path.extension().is_none() {
                        path.set_extension("md");
                    }
                    write_document_to_path(&ctx, path);
                } else {
                    show_error(
                        &ctx.window,
                        "Save failed",
                        "This file location is not a local path.",
                    );
                }
            }
            Err(e) => {
                if !e.matches(gtk::DialogError::Dismissed) {
                    send_toast(&ctx, &format!("Could not open save dialog: {e}"));
                }
            }
        }
    });
}

pub fn export_as_html(ctx: &EditorCtx) {
    let markdown = current_markdown(ctx);
    let note_name = {
        let state = ctx.state.borrow();
        find_note_index(&state.notes, &state.active_note_id)
            .map(|i| state.notes[i].name.clone())
            .unwrap_or_else(|| "Untitled".to_string())
    };

    // Convert markdown to HTML using pulldown-cmark
    let parser = pulldown_cmark::Parser::new(&markdown);
    let mut html_body = String::new();
    pulldown_cmark::html::push_html(&mut html_body, parser);

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
body {{ font-family: system-ui, -apple-system, sans-serif; max-width: 48em; margin: 2em auto; padding: 0 1em; line-height: 1.6; color: #222; }}
h1, h2, h3 {{ margin-top: 1.5em; }}
code {{ background: #f4f4f4; padding: 0.15em 0.3em; border-radius: 3px; font-size: 0.9em; }}
pre {{ background: #f4f4f4; padding: 1em; border-radius: 6px; overflow-x: auto; }}
pre code {{ background: none; padding: 0; }}
blockquote {{ border-left: 3px solid #ddd; margin-left: 0; padding-left: 1em; color: #555; }}
table {{ border-collapse: collapse; width: 100%; }}
th, td {{ border: 1px solid #ddd; padding: 0.5em 0.75em; text-align: left; }}
th {{ background: #f8f8f8; }}
img {{ max-width: 100%; }}
</style>
</head>
<body>
{body}
</body>
</html>"#,
        title = note_name.replace('<', "&lt;").replace('>', "&gt;"),
        body = html_body,
    );

    let dialog = gtk::FileDialog::builder()
        .title("Export as HTML")
        .accept_label("Export")
        .build();

    let suggested = format!("{}.html", note_name);
    dialog.set_initial_name(Some(&suggested));

    let filter = gtk::FileFilter::new();
    filter.set_name(Some("HTML files"));
    filter.add_pattern("*.html");
    filter.add_pattern("*.htm");
    let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    dialog.set_filters(Some(&filters));

    let ctx = ctx.clone();
    let window = ctx.window.clone();
    dialog.save(Some(&window), gtk::gio::Cancellable::NONE, move |result: Result<gtk::gio::File, gtk::glib::Error>| {
        match result {
            Ok(file) => {
                if let Some(mut path) = file.path() {
                    if path.extension().is_none() {
                        path.set_extension("html");
                    }
                    match fs::write(&path, &html) {
                        Ok(_) => send_toast(&ctx, "Exported as HTML"),
                        Err(e) => show_error(
                            &ctx.window,
                            "Export Failed",
                            &format!("Could not write HTML: {e}"),
                        ),
                    }
                }
            }
            Err(e) => {
                if !e.matches(gtk::DialogError::Dismissed) {
                    send_toast(&ctx, &format!("Could not open export dialog: {e}"));
                }
            }
        }
    });
}

pub fn write_document_to_path(ctx: &EditorCtx, path: PathBuf) {
    let markdown = current_markdown(ctx);
    match fs::write(&path, &markdown) {
        Ok(_) => {
            {
                let mut state = ctx.state.borrow_mut();
                if let Some(index) = find_note_index(&state.notes, &state.active_note_id) {
                    if let Some(file_name) = path.file_stem().and_then(|stem| stem.to_str()) {
                        state.notes[index].name = file_name.to_string();
                    }
                    state.notes[index].content = markdown.clone();
                    state.notes[index].updated_at = unix_now();
                    state.notes[index].file_path = Some(path);
                }
                state.saved_snapshot = markdown.clone();
                state.last_snapshot = markdown;
                state.dirty = false;
            }
            refresh_header(ctx);
            refresh_tabs(ctx);
            refresh_note_list(ctx);
        }
        Err(err) => show_error(
            &ctx.window,
            "Save failed",
            &format!("Could not write file:\n{err}"),
        ),
    }
}

pub fn current_markdown(ctx: &EditorCtx) -> String {
    sync_markdown_and_status(ctx)
}

pub fn source_buffer_text(buffer: &sourceview::Buffer) -> String {
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    buffer.text(&start, &end, true).to_string()
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
            let tmp = std::env::temp_dir().join("pithos-paste.png");
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
            match crypto::encrypt_asset(&data_owned, key) {
                Ok(encrypted) => encrypted.into_bytes(),
                Err(e) => {
                    let _ = tx.send(Err(format!("Asset encryption failed: {e}")));
                    return;
                }
            }
        } else {
            data_owned
        };

        if let Err(e) = vault::write_asset(&vault_folder, &asset_id_thread, &write_data) {
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
                let meta = vault::AssetMeta {
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

// ---------------------------------------------------------------------------
// Note deletion
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

// ---------------------------------------------------------------------------
// Note renaming
// ---------------------------------------------------------------------------

pub fn rename_note_dialog(ctx: &EditorCtx) {
    let current_name = {
        let state = ctx.state.borrow();
        find_note_index(&state.notes, &state.active_note_id)
            .map(|i| state.notes[i].name.clone())
            .unwrap_or_else(|| "Untitled".to_string())
    };

    let dialog = adw::AlertDialog::new(
        Some("Rename Note"),
        Some("Enter a new name for this note."),
    );

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
    dialog.connect_response(None, move |dlg, response| {
        // Remove entry before dialog closes to avoid GtkText focus-out warning
        let new_name = entry.text().trim().to_string();
        dlg.set_extra_child(gtk::Widget::NONE);
        if response == "rename"
            && !new_name.is_empty() {
                let conflict = {
                    let state = ctx.state.borrow();
                    let active = &state.active_note_id;
                    find_note_index(&state.notes, active).is_some_and(|i| {
                        note_name_exists(
                            &state.notes,
                            &new_name,
                            &state.notes[i].parent_id,
                            Some(active),
                        )
                    })
                };
                if conflict {
                    send_toast(&ctx, "A note with that name already exists in this folder");
                    return;
                }
                {
                    let mut state = ctx.state.borrow_mut();
                    if let Some(index) =
                        find_note_index(&state.notes, &state.active_note_id)
                    {
                        state.notes[index].name = new_name;
                        state.notes[index].updated_at = unix_now();
                    }
                }
                refresh_header(&ctx);
                refresh_tabs(&ctx);
                refresh_note_list(&ctx);
                trigger_vault_save(&ctx);
            }
    });
    dialog.present(Some(&window));
}

// ---------------------------------------------------------------------------
// Unsaved changes warning on close
// ---------------------------------------------------------------------------

pub fn wire_close_request(ctx: &EditorCtx) {
    let ctx = ctx.clone();
    let win = ctx.window.clone();
    win.connect_close_request(move |_window| {
        gtk::prelude::GtkWindowExt::set_focus(_window, gtk::Widget::NONE);
        let has_vault = ctx.cached_key.borrow().is_some() && !ctx.vault_folder.borrow().is_empty();
        if !has_vault {
            // No vault configured — just close
            return glib::Propagation::Proceed;
        }

        // Route close through the same save generation queue used by autosave/manual saves.
        ctx.close_requested.set(true);
        // Cancel pending debounce timers
        if let Some(source_id) = ctx.save_timeout_id.take() {
            source_id.remove();
        }
        if let Some(sync_id) = ctx.sync_timeout_id.take() {
            sync_id.remove();
        }
        perform_vault_save_async(&ctx, false);
        glib::Propagation::Stop
    });
}

fn show_close_save_failed_dialog(window: &adw::ApplicationWindow, error: &str) {
    let dialog = adw::AlertDialog::new(
        Some("Save Failed"),
        Some(&format!(
            "Your changes could not be saved:\n\n{error}\n\nClose anyway and lose unsaved changes?"
        )),
    );
    dialog.add_response("cancel", "Keep Open");
    dialog.add_response("close", "Close Without Saving");
    dialog.set_response_appearance("close", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");

    let window_for_close = window.clone();
    let window_for_present = window.clone();
    dialog.connect_response(None, move |_, response| {
        if response == "close" {
            window_for_close.destroy();
        }
    });
    dialog.present(Some(&window_for_present));
}

// ---------------------------------------------------------------------------
// Auto-save
// ---------------------------------------------------------------------------

pub fn setup_auto_save(ctx: &EditorCtx) {
    let ctx = ctx.clone();
    glib::timeout_add_seconds_local(AUTO_SAVE_INTERVAL_SECS, move || {
        auto_save_tick(&ctx);
        glib::ControlFlow::Continue
    });
}

pub fn auto_save_tick(ctx: &EditorCtx) {
    // Flush the source buffer into the note before saving,
    // in case the debounced buffer-change hasn't fired yet.
    let markdown = source_buffer_text(&ctx.source_buffer);
    update_active_note_content(ctx, &markdown);

    // Push a snapshot to version history
    {
        let mut state = ctx.state.borrow_mut();
        if let Some(index) = find_note_index(&state.notes, &state.active_note_id) {
            let content = state.notes[index].content.clone();
            push_snapshot(&mut state.notes[index], content);
        }
    }
    perform_vault_save_async(ctx, false);
}

// ---------------------------------------------------------------------------
// Vault startup dialogs
// ---------------------------------------------------------------------------

// Shared flag to prevent the app from closing during programmatic dialog transitions.
thread_local! {
    static DIALOG_TRANSITIONING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn close_app_on_dialog_dismiss(dialog: &adw::Window, window: &adw::ApplicationWindow) {
    let window = window.clone();
    dialog.connect_close_request(move |_| {
        if DIALOG_TRANSITIONING.get() {
            return glib::Propagation::Proceed;
        }
        // User closed the dialog (X button / Escape) — quit the app
        // if the editor was never loaded.
        if window.content().is_none() {
            window.close();
        }
        glib::Propagation::Proceed
    });
}

/// Close a startup dialog as part of a flow transition (won't quit the app).
fn transition_close(dialog: &adw::Window) {
    DIALOG_TRANSITIONING.set(true);
    dialog.close();
    DIALOG_TRANSITIONING.set(false);
}

pub fn show_welcome_dialog(window: &adw::ApplicationWindow) {
    let dialog = adw::Window::builder()
        .transient_for(window)
        .modal(true)
        .title("Pithos Notebook")
        .default_width(460)
        .default_height(320)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 16);
    vbox.set_margin_start(32);
    vbox.set_margin_end(32);
    vbox.set_margin_top(32);
    vbox.set_margin_bottom(32);
    vbox.set_valign(gtk::Align::Center);

    let icon = gtk::Image::from_icon_name("com.pithos.notebook");
    icon.set_pixel_size(64);
    vbox.append(&icon);

    let title = gtk::Label::new(Some("Welcome to Pithos Notebook"));
    title.add_css_class("title-2");
    vbox.append(&title);

    let subtitle = gtk::Label::new(Some(
        "A private, offline, encrypted markdown notebook.",
    ));
    subtitle.add_css_class("dim-label");
    subtitle.set_wrap(true);
    subtitle.set_justify(gtk::Justification::Center);
    vbox.append(&subtitle);

    let btn_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    btn_box.set_margin_top(8);

    let create_btn = gtk::Button::with_label("Create New Vault");
    create_btn.add_css_class("suggested-action");
    create_btn.add_css_class("pill");
    btn_box.append(&create_btn);

    let open_btn = gtk::Button::with_label("Open Existing Vault");
    open_btn.add_css_class("pill");
    btn_box.append(&open_btn);

    vbox.append(&btn_box);

    let toolbar = adw::HeaderBar::new();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&toolbar);
    content.append(&vbox);
    dialog.set_content(Some(&content));

    close_app_on_dialog_dismiss(&dialog, window);

    // Create New Vault
    {
        let window = window.clone();
        let dialog = dialog.clone();
        create_btn.connect_clicked(move |_| {
            transition_close(&dialog);
            show_create_vault_dialog(&window);
        });
    }

    // Open Existing Vault
    {
        let window = window.clone();
        let dialog = dialog.clone();
        open_btn.connect_clicked(move |_| {
            let chooser = gtk::FileDialog::builder()
                .title("Select Vault Folder")
                .accept_label("Select")
                .build();
            let win = window.clone();
            let dlg = dialog.clone();
            let dlg_for_parent = dlg.clone();
            chooser.select_folder(Some(&dlg_for_parent), gtk::gio::Cancellable::NONE, move |result| {
                match result {
                    Ok(file) => {
                        if let Some(path) = file.path() {
                            let folder = path.to_string_lossy().to_string();
                            if vault::vault_file_path(&folder).exists() {
                                let _ = vault::save_config(&vault::AppConfig {
                                    vault_path: Some(folder.clone()),
                                });
                                transition_close(&dlg);
                                show_unlock_vault_dialog(&win, folder);
                            } else {
                                let alert = adw::AlertDialog::new(
                                    Some("No Vault Found"),
                                    Some("The selected folder does not contain a vault file. Please choose a different folder or create a new vault."),
                                );
                                alert.add_response("ok", "OK");
                                alert.set_close_response("ok");
                                alert.present(Some(&dlg));
                            }
                        }
                    }
                    Err(e) => {
                        if !e.matches(gtk::DialogError::Dismissed) {
                            let alert = adw::AlertDialog::new(
                                Some("Folder Selection Failed"),
                                Some(&format!("Could not open folder picker: {e}")),
                            );
                            alert.add_response("ok", "OK");
                            alert.set_close_response("ok");
                            alert.present(Some(&dlg));
                        }
                    }
                }
            });
        });
    }

    dialog.present();
}

pub fn show_create_vault_dialog(window: &adw::ApplicationWindow) {
    let dialog = adw::Window::builder()
        .transient_for(window)
        .modal(true)
        .title("Create Vault")
        .default_width(460)
        .default_height(340)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 12);
    vbox.set_margin_start(24);
    vbox.set_margin_end(24);
    vbox.set_margin_top(24);
    vbox.set_margin_bottom(24);

    let title = gtk::Label::new(Some("Create a New Vault"));
    title.add_css_class("title-2");
    vbox.append(&title);

    let subtitle = gtk::Label::new(Some(
        "Choose a folder and set a passphrase to encrypt your notes.",
    ));
    subtitle.add_css_class("dim-label");
    subtitle.set_wrap(true);
    vbox.append(&subtitle);

    let folder_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let folder_label = gtk::Label::new(Some("No folder selected"));
    folder_label.set_hexpand(true);
    folder_label.set_xalign(0.0);
    folder_label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    let folder_btn = gtk::Button::with_label("Choose Folder...");
    folder_btn.add_css_class("toolbar-pill");
    folder_row.append(&folder_label);
    folder_row.append(&folder_btn);
    vbox.append(&folder_row);

    let folder_path: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let pass1 = gtk::PasswordEntry::builder()
        .placeholder_text("Set passphrase")
        .show_peek_icon(true)
        .build();
    vbox.append(&pass1);

    let pass2 = gtk::PasswordEntry::builder()
        .placeholder_text("Confirm passphrase")
        .show_peek_icon(true)
        .build();
    vbox.append(&pass2);

    let error_label = gtk::Label::new(None);
    error_label.add_css_class("error");
    error_label.set_visible(false);
    vbox.append(&error_label);

    let create_btn = gtk::Button::with_label("Create Vault");
    create_btn.add_css_class("suggested-action");
    create_btn.add_css_class("pill");
    vbox.append(&create_btn);

    let toolbar = adw::HeaderBar::new();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&toolbar);
    content.append(&vbox);
    dialog.set_content(Some(&content));

    close_app_on_dialog_dismiss(&dialog, window);

    // Folder chooser
    {
        let dialog_ref = dialog.clone();
        let folder_label = folder_label.clone();
        let folder_path = folder_path.clone();
        folder_btn.connect_clicked(move |_| {
            let chooser = gtk::FileDialog::builder()
                .title("Select Vault Folder")
                .accept_label("Select")
                .build();
            let fl = folder_label.clone();
            let fp = folder_path.clone();
            let dlg = dialog_ref.clone();
            chooser.select_folder(Some(&dlg), gtk::gio::Cancellable::NONE, move |result: Result<gtk::gio::File, gtk::glib::Error>| {
                match result {
                    Ok(file) => {
                        if let Some(path) = file.path() {
                            let p = path.to_string_lossy().to_string();
                            fl.set_label(&p);
                            *fp.borrow_mut() = Some(p);
                        }
                    }
                    Err(e) => {
                        if !e.matches(gtk::DialogError::Dismissed) {
                            eprintln!("Folder selection failed: {e}");
                        }
                    }
                }
            });
        });
    }

    // Create action
    {
        let window = window.clone();
        let dialog = dialog.clone();
        let folder_path = folder_path.clone();
        let pass1 = pass1.clone();
        let pass2 = pass2.clone();
        let error_label = error_label.clone();
        let create_btn_inner = create_btn.clone();
        create_btn.connect_clicked(move |_| {
            let create_btn = create_btn_inner.clone();
            let p1 = pass1.text().to_string();
            let p2 = pass2.text().to_string();
            let fp = folder_path.borrow().clone();

            if fp.is_none() {
                error_label.set_label("Please select a folder.");
                error_label.set_visible(true);
                return;
            }
            if p1.len() < 8 {
                error_label.set_label("Passphrase must be at least 8 characters.");
                error_label.set_visible(true);
                return;
            }
            if p1 != p2 {
                error_label.set_label("Passphrases do not match.");
                error_label.set_visible(true);
                return;
            }

            let Some(vault_folder) = fp else {
                error_label.set_label("Please select a folder.");
                error_label.set_visible(true);
                return;
            };

            // If the selected folder already contains a vault, switch to unlock flow
            if vault::vault_file_path(&vault_folder).exists() {
                let _ = vault::save_config(&vault::AppConfig {
                    vault_path: Some(vault_folder.clone()),
                });
                transition_close(&dialog);
                show_unlock_vault_dialog(&window, vault_folder);
                return;
            }

            let default_state = DocState::default();
            let vault_data = vault::doc_state_to_vault(&default_state);
            let json = match serde_json::to_string_pretty(&vault_data) {
                Ok(j) => j,
                Err(e) => {
                    error_label.set_label(&format!("Serialization failed: {e}"));
                    error_label.add_css_class("error");
                    error_label.set_visible(true);
                    return;
                }
            };

            // Show spinner while deriving key (PBKDF2 is expensive)
            create_btn.set_sensitive(false);
            error_label.set_label("Creating vault\u{2026}");
            error_label.remove_css_class("error");
            error_label.set_visible(true);

            let (tx, rx) = std::sync::mpsc::channel::<Result<crypto::CachedKey, String>>();
            let vault_folder_for_thread = vault_folder.clone();
            std::thread::spawn(move || {
                let cached_key = crypto::CachedKey::derive(&p1);
                match crypto::encrypt_vault_fast(&json, &cached_key) {
                    Ok(encrypted) => {
                        if let Err(e) = vault::write_vault_raw(&vault_folder_for_thread, &encrypted) {
                            let _ = tx.send(Err(format!("Write failed: {e}")));
                        } else {
                            let _ = tx.send(Ok(cached_key));
                        }
                    }
                    Err(e) => { let _ = tx.send(Err(format!("Encryption failed: {e}"))); }
                }
            });

            let window = window.clone();
            let dialog = dialog.clone();
            let vault_folder_c = vault_folder.clone();
            let error_label = error_label.clone();
            let create_btn = create_btn.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                let result = match rx.try_recv() {
                    Ok(r) => r,
                    Err(std::sync::mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
                    Err(_) => return glib::ControlFlow::Break,
                };
                match result {
                    Ok(cached_key) => {
                        let _ = vault::save_config(&vault::AppConfig {
                            vault_path: Some(vault_folder_c.clone()),
                        });
                        gtk::prelude::GtkWindowExt::set_focus(&window, gtk::Widget::NONE);
                        transition_close(&dialog);
                        let window = window.clone();
                        let vault_folder_c = vault_folder_c.clone();
                        glib::timeout_add_local_once(
                            std::time::Duration::from_millis(50),
                            move || {
                                build_editor(&window, DocState::default(), vault_folder_c, cached_key);
                            },
                        );
                    }
                    Err(e) => {
                        error_label.add_css_class("error");
                        error_label.set_label(&e);
                        create_btn.set_sensitive(true);
                    }
                }
                glib::ControlFlow::Break
            });
        });
    }

    dialog.present();
}

pub fn show_unlock_vault_dialog(window: &adw::ApplicationWindow, vault_folder: String) {
    let dialog = adw::Window::builder()
        .transient_for(window)
        .modal(true)
        .title("Unlock Vault")
        .default_width(400)
        .default_height(260)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 12);
    vbox.set_margin_start(24);
    vbox.set_margin_end(24);
    vbox.set_margin_top(24);
    vbox.set_margin_bottom(24);

    let title = gtk::Label::new(Some("Unlock Your Vault"));
    title.add_css_class("title-2");
    vbox.append(&title);

    let subtitle = gtk::Label::new(Some("Enter your passphrase to decrypt your notes."));
    subtitle.add_css_class("dim-label");
    subtitle.set_wrap(true);
    vbox.append(&subtitle);

    let pass_entry = gtk::PasswordEntry::builder()
        .placeholder_text("Passphrase")
        .show_peek_icon(true)
        .build();
    vbox.append(&pass_entry);

    let error_label = gtk::Label::new(None);
    error_label.add_css_class("error");
    error_label.set_visible(false);
    vbox.append(&error_label);

    let btn_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let unlock_btn = gtk::Button::with_label("Unlock");
    unlock_btn.add_css_class("suggested-action");
    unlock_btn.add_css_class("pill");
    unlock_btn.set_hexpand(true);
    let change_btn = gtk::Button::with_label("Change Vault");
    change_btn.add_css_class("flat");
    btn_row.append(&unlock_btn);
    btn_row.append(&change_btn);
    vbox.append(&btn_row);

    let toolbar = adw::HeaderBar::new();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&toolbar);
    content.append(&vbox);
    dialog.set_content(Some(&content));

    close_app_on_dialog_dismiss(&dialog, window);

    // Unlock action
    {
        let window = window.clone();
        let dialog = dialog.clone();
        let vault_folder = vault_folder.clone();
        let pass_entry_for_activate = pass_entry.clone();
        let pass_entry = pass_entry.clone();
        let error_label = error_label.clone();
        let unlock_btn = unlock_btn.clone();
        let unlock_btn_for_connect = unlock_btn.clone();
        let do_unlock = move || {
            let passphrase = pass_entry.text().to_string();
            if passphrase.is_empty() {
                error_label.set_label("Please enter your passphrase.");
                error_label.set_visible(true);
                return;
            }

            let vault_folder_thread = vault_folder.clone();

            // Show spinner while decrypting (PBKDF2 is expensive)
            unlock_btn.set_sensitive(false);
            pass_entry.set_sensitive(false);
            error_label.set_label("Unlocking\u{2026}");
            error_label.remove_css_class("error");
            error_label.set_visible(true);

            let (tx, rx) = std::sync::mpsc::channel::<Result<(String, crypto::CachedKey), String>>();
            std::thread::spawn(move || {
                let raw = match vault::read_vault_raw(&vault_folder_thread) {
                    Ok(Some(data)) => data,
                    Ok(None) => {
                        let _ = tx.send(Err("Vault file not found.".to_string()));
                        return;
                    }
                    Err(e) => {
                        let _ = tx.send(Err(format!("Read error: {e}")));
                        return;
                    }
                };

                match crypto::decrypt_vault_returning_key(&raw, &passphrase) {
                    Ok((json, cached_key)) => {
                        let _ = tx.send(Ok((json, cached_key)));
                    }
                    Err(_) => {
                        let _ = tx.send(Err("wrong_passphrase".to_string()));
                    }
                }
            });

            let window = window.clone();
            let dialog = dialog.clone();
            let vault_folder = vault_folder.clone();
            let error_label = error_label.clone();
            let pass_entry = pass_entry.clone();
            let unlock_btn = unlock_btn.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                let result = match rx.try_recv() {
                    Ok(r) => r,
                    Err(std::sync::mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
                    Err(_) => return glib::ControlFlow::Break,
                };
                match result {
                    Ok((json, cached_key)) => {
                        let vault_data: vault::VaultData = match serde_json::from_str(&json) {
                            Ok(v) => v,
                            Err(e) => {
                                error_label.add_css_class("error");
                                error_label.set_label(&format!("Corrupt vault: {e}"));
                                unlock_btn.set_sensitive(true);
                                pass_entry.set_sensitive(true);
                                return glib::ControlFlow::Break;
                            }
                        };
                        let state = vault::vault_to_doc_state(vault_data);
                        gtk::prelude::GtkWindowExt::set_focus(&window, gtk::Widget::NONE);
                        transition_close(&dialog);
                        // Defer editor build so the dialog is fully destroyed and
                        // GtkText focus events are processed before creating the
                        // WebView — prevents segfault in confined environments.
                        let window = window.clone();
                        let vault_folder = vault_folder.clone();
                        glib::timeout_add_local_once(
                            std::time::Duration::from_millis(50),
                            move || {
                                build_editor(&window, state, vault_folder, cached_key);
                            },
                        );
                    }
                    Err(e) => {
                        error_label.add_css_class("error");
                        if e == "wrong_passphrase" {
                            error_label.set_label("Wrong passphrase. Try again.");
                            pass_entry.set_text("");
                            pass_entry.grab_focus();
                        } else {
                            error_label.set_label(&e);
                        }
                        unlock_btn.set_sensitive(true);
                        pass_entry.set_sensitive(true);
                    }
                }
                glib::ControlFlow::Break
            });
        };

        let do_unlock_click = do_unlock.clone();
        unlock_btn_for_connect.connect_clicked(move |_| do_unlock_click());

        let do_unlock_enter = do_unlock;
        pass_entry_for_activate.connect_activate(move |_| do_unlock_enter());
    }

    // Change vault
    {
        let window = window.clone();
        let dialog = dialog.clone();
        change_btn.connect_clicked(move |_| {
            let _ = vault::save_config(&vault::AppConfig { vault_path: None });
            transition_close(&dialog);
            show_welcome_dialog(&window);
        });
    }

    dialog.present();
    pass_entry.grab_focus();
}

// ---------------------------------------------------------------------------
// Theme
// ---------------------------------------------------------------------------

pub fn apply_theme(theme: &str) {
    let style_manager = adw::StyleManager::default();
    let scheme = match theme {
        "light" => adw::ColorScheme::ForceLight,
        "dark" => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    };
    style_manager.set_color_scheme(scheme);
}

pub fn apply_sourceview_theme(view: &sourceview::View, dark: bool) {
    let scheme_manager = sourceview::StyleSchemeManager::default();
    let scheme_id = if dark { "Adwaita-dark" } else { "Adwaita" };
    if let Some(scheme) = scheme_manager.scheme(scheme_id) {
        if let Some(buffer) = view.buffer().downcast_ref::<sourceview::Buffer>() {
            buffer.set_style_scheme(Some(&scheme));
        }
    }
}

pub fn is_dark_active() -> bool {
    adw::StyleManager::default().is_dark()
}

pub fn toggle_theme(ctx: &EditorCtx) {
    let new_theme = {
        let state = ctx.state.borrow();
        match state.theme.as_str() {
            "system" => "light",
            "light" => "dark",
            _ => "system",
        }
        .to_string()
    };
    apply_theme(&new_theme);
    apply_sourceview_theme(&ctx.source_view, is_dark_active());
    render_preview(ctx);
    ctx.state.borrow_mut().theme = new_theme;
    trigger_vault_save(ctx);
}

// ---------------------------------------------------------------------------
// Vault persistence
// ---------------------------------------------------------------------------

pub fn trigger_vault_save(ctx: &EditorCtx) {
    if let Some(source_id) = ctx.save_timeout_id.take() {
        source_id.remove();
    }

    let ctx_clone = ctx.clone();
    let source_id = glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
        ctx_clone.save_timeout_id.set(None);
        perform_vault_save_async(&ctx_clone, false);
    });

    ctx.save_timeout_id.set(Some(source_id));
}

/// Collect vault data on the main thread (cheap), return everything needed for I/O.
pub fn prepare_vault_save(ctx: &EditorCtx) -> Option<(vault::VaultData, crypto::CachedKey, String)> {
    let markdown = current_markdown(ctx);
    update_active_note_content(ctx, &markdown);

    // Persist sidebar width (convert fraction to approximate pixel width)
    {
        let mut state = ctx.state.borrow_mut();
        let fraction = ctx.split_view.sidebar_width_fraction();
        let total = ctx.split_view.allocated_width() as f64;
        state.sidebar_width = (fraction * total).round() as i32;
    }

    let vault_data = vault::doc_state_to_vault(&ctx.state.borrow());
    let key = ctx.cached_key.borrow().clone()?;
    let vault_folder = ctx.vault_folder.borrow().clone();
    if vault_folder.is_empty() {
        return None;
    }
    Some((vault_data, key, vault_folder))
}

/// Perform serialization + encryption + write (expensive, blocking).
pub fn vault_save_blocking(
    vault_data: vault::VaultData,
    key: &crypto::CachedKey,
    vault_folder: &str,
) -> Result<(), String> {
    use zeroize::Zeroize;
    let mut json = serde_json::to_string_pretty(&vault_data)
        .map_err(|e| format!("Serialization: {e}"))?;
    let result = crypto::encrypt_vault_fast(&json, key)
        .map_err(|e| format!("Encryption: {e}"));
    json.zeroize(); // Wipe plaintext vault contents from memory
    let encrypted = result?;
    vault::write_vault_raw(vault_folder, &encrypted)
        .map_err(|e| format!("Write: {e}"))?;
    Ok(())
}

/// Synchronous vault save — used only for close-request where we must block.
pub fn perform_vault_save_sync(ctx: &EditorCtx) -> bool {
    let Some((vault_data, key, vault_folder)) = prepare_vault_save(ctx) else {
        return false;
    };
    match vault_save_blocking(vault_data, &key, &vault_folder) {
        Ok(()) => {
            send_toast(ctx, "Saved");
            true
        }
        Err(e) => {
            show_error(&ctx.window, "Save Failed", &format!("Vault save failed: {e}"));
            false
        }
    }
}

/// Async vault save — runs serialization + encryption + write on a background thread.
/// Uses generation IDs to prevent stale saves from overwriting newer data.
/// Only marks document clean after the write succeeds.
pub fn perform_vault_save_async(ctx: &EditorCtx, toast: bool) {
    // If a save is already in flight, bump generation so it re-saves when done
    if ctx.saving.get() {
        ctx.save_generation.set(ctx.save_generation.get() + 1);
        return;
    }

    let snapshot = current_markdown(ctx);
    let Some((vault_data, key, vault_folder)) = prepare_vault_save(ctx) else {
        if toast {
            show_error(
                &ctx.window,
                "Save Failed",
                "Vault is not unlocked or no vault folder is configured.",
            );
        } else {
            show_error(
                &ctx.window,
                "Save Failed",
                "Vault is not unlocked or no vault folder is configured.",
            );
        }
        return;
    };

    let gen = ctx.save_generation.get() + 1;
    ctx.save_generation.set(gen);
    ctx.saving.set(true);

    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();

    std::thread::spawn(move || {
        let result = vault_save_blocking(vault_data, &key, &vault_folder);
        let _ = tx.send(result);
    });

    let ctx = ctx.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        match rx.try_recv() {
            Ok(Ok(())) => {
                ctx.saving.set(false);
                // Only mark clean if no new edits happened during save
                let current = current_markdown(&ctx);
                {
                    let mut state = ctx.state.borrow_mut();
                    state.saved_snapshot = snapshot.clone();
                    if current == snapshot {
                        state.dirty = false;
                    }
                }
                refresh_header(&ctx);
                if toast && !ctx.close_requested.get() {
                    send_toast(&ctx, "Saved");
                }
                // A newer save was requested while we were writing — re-save
                if ctx.save_generation.get() > gen {
                    perform_vault_save_async(&ctx, false);
                    return glib::ControlFlow::Break;
                }
                if ctx.close_requested.replace(false) {
                    ctx.window.destroy();
                }
                glib::ControlFlow::Break
            }
            Ok(Err(e)) => {
                ctx.saving.set(false);
                eprintln!("Vault save failed: {e}");
                if ctx.close_requested.replace(false) {
                    show_close_save_failed_dialog(&ctx.window, &e);
                } else {
                    show_error(&ctx.window, "Save Failed", &e);
                }
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                ctx.saving.set(false);
                if ctx.close_requested.replace(false) {
                    show_close_save_failed_dialog(&ctx.window, "Save thread disconnected");
                } else {
                    show_error(&ctx.window, "Save Failed", "Save thread disconnected");
                }
                glib::ControlFlow::Break
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Trash operations
// ---------------------------------------------------------------------------

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
// CSS
// ---------------------------------------------------------------------------

pub fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(include_str!("style.css"));

    let Some(display) = gdk::Display::default() else {
        return;
    };

    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

// ---------------------------------------------------------------------------
// Markdown syntax helpers (source-first editing)
// ---------------------------------------------------------------------------

fn md_wrap(buffer: &sourceview::Buffer, prefix: &str, suffix: &str, placeholder: &str) {
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

fn md_line_prefix(buffer: &sourceview::Buffer, prefix: &str) {
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

fn strip_line_prefix(text: &str) -> &str {
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

fn toggle_checkbox_at_cursor(buffer: &sourceview::Buffer) {
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

fn render_preview(ctx: &EditorCtx) {
    use webkit6::prelude::WebViewExt;
    let markdown = source_buffer_text(&ctx.source_buffer);
    let dark = is_dark_active();
    // Set the WebView background to match the theme immediately,
    // preventing a flash of the wrong color while HTML loads.
    let bg = if dark {
        gtk::gdk::RGBA::new(0.118, 0.118, 0.118, 1.0) // #1e1e1e
    } else {
        gtk::gdk::RGBA::new(0.98, 0.98, 0.98, 1.0)    // #fafafa
    };
    ctx.preview_webview.set_background_color(&bg);
    let mut html = build_preview_html(&markdown, dark);

    // Replace vault:// asset URLs with inline data: URLs so images render in preview
    resolve_vault_assets(&mut html, ctx);

    ctx.preview_webview.load_html(&html, None);
}

/// Replaces `src="vault://asset_id"` in HTML with `src="data:mime;base64,..."`.
fn resolve_vault_assets(html: &mut String, ctx: &EditorCtx) {
    let prefix = "src=\"vault://";
    while let Some(start) = html.find(prefix) {
        let attr_start = start + "src=\"".len(); // start of vault://...
        let Some(quote_end) = html[attr_start..].find('"') else { break };
        let vault_url = html[attr_start..attr_start + quote_end].to_string();
        let Some(asset_id) = vault_url.strip_prefix("vault://") else { break };

        let data_url = resolve_single_asset(asset_id, ctx);
        let replacement = format!("src=\"{}\"", data_url);
        html.replace_range(start..attr_start + quote_end + 1, &replacement);
    }
}

fn resolve_single_asset(asset_id: &str, ctx: &EditorCtx) -> String {
    let vault_folder = ctx.vault_folder.borrow().clone();
    let mime_type = ctx
        .state
        .borrow()
        .assets
        .get(asset_id)
        .map(|m| m.mime_type.clone())
        .unwrap_or_else(|| "image/png".to_string());

    let asset_path = vault::assets_dir(&vault_folder).join(asset_id);
    let Ok(raw_data) = fs::read(&asset_path) else {
        return String::new();
    };

    let cached_key_ref = ctx.cached_key.borrow();
    let Some(cached_key) = cached_key_ref.as_ref() else {
        return String::new();
    };

    match crypto::decrypt_asset(&raw_data, cached_key) {
        Ok(decrypted) => {
            let b64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &decrypted,
            );
            format!("data:{mime_type};base64,{b64}")
        }
        Err(_) => String::new(),
    }
}

const MERMAID_JS: &str = include_str!("../data/mermaid.min.js");

fn build_preview_html(markdown: &str, dark: bool) -> String {
    use pulldown_cmark::{Parser, Options, html};

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);
    let mut body = String::new();
    html::push_html(&mut body, parser);

    // Convert mermaid code blocks: <pre><code class="language-mermaid">...</code></pre>
    // → <pre class="mermaid">...</pre>
    let has_mermaid = body.contains("language-mermaid");
    // Replace only mermaid code blocks, leaving other <pre><code> blocks intact
    if has_mermaid {
        let open_tag = r#"<pre><code class="language-mermaid">"#;
        let close_tag = "</code></pre>";
        while let Some(start) = body.find(open_tag) {
            if let Some(rel_end) = body[start + open_tag.len()..].find(close_tag) {
                let content_start = start + open_tag.len();
                let content_end = content_start + rel_end;
                let content = body[content_start..content_end].to_string();
                let replacement = format!(r#"<pre class="mermaid">{content}</pre>"#);
                body.replace_range(start..content_end + close_tag.len(), &replacement);
            } else {
                break;
            }
        }
    }

    let (bg, fg, code_bg, border, link_color, heading_color) = if dark {
        // Match Adwaita-dark sourceview scheme (#1e1e1e background)
        ("#1e1e1e", "#d4d4d4", "#2a2a2a", "#3c3c3c", "#78aeed", "#e0e0e0")
    } else {
        // Match Adwaita light sourceview scheme
        ("#fafafa", "#2e2e2e", "#f0f0f0", "#d5d5d5", "#1c71d8", "#1e1e1e")
    };

    // CSP: allow inline scripts only when mermaid blocks are present
    let script_src = if has_mermaid { "'unsafe-inline'" } else { "'none'" };

    let mermaid_theme = if dark { "dark" } else { "default" };
    let mermaid_script = if has_mermaid {
        format!(
            r#"<script>{MERMAID_JS}</script>
<script>mermaid.initialize({{ startOnLoad: true, theme: '{mermaid_theme}' }});</script>"#
        )
    } else {
        String::new()
    };

    format!(r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src data:; script-src {script_src};">
<style>
body {{
    font-family: -apple-system, 'Cantarell', 'Segoe UI', sans-serif;
    font-size: 15px;
    line-height: 1.7;
    color: {fg};
    background: {bg};
    padding: 16px 24px;
    max-width: 800px;
    margin: 0 auto;
}}
h1, h2, h3, h4, h5, h6 {{ color: {heading_color}; margin-top: 1.4em; margin-bottom: 0.5em; }}
h1 {{ font-size: 2em; border-bottom: 1px solid {border}; padding-bottom: 0.3em; }}
h2 {{ font-size: 1.5em; border-bottom: 1px solid {border}; padding-bottom: 0.3em; }}
h3 {{ font-size: 1.25em; }}
a {{ color: {link_color}; text-decoration: none; }}
a:hover {{ text-decoration: underline; }}
code {{ background: {code_bg}; padding: 2px 6px; border-radius: 4px; font-size: 0.9em; }}
pre {{ background: {code_bg}; padding: 12px 16px; border-radius: 8px; overflow-x: auto; border: 1px solid {border}; }}
pre code {{ background: none; padding: 0; }}
pre.mermaid {{ background: transparent; border: none; padding: 8px 0; text-align: center; }}
blockquote {{ border-left: 3px solid {link_color}; margin: 1em 0; padding: 0.5em 1em; color: {fg}; opacity: 0.85; }}
table {{ border-collapse: collapse; width: 100%; margin: 1em 0; }}
th, td {{ border: 1px solid {border}; padding: 8px 12px; text-align: left; }}
th {{ background: {code_bg}; font-weight: 600; }}
hr {{ border: none; border-top: 1px solid {border}; margin: 2em 0; }}
img {{ max-width: 100%; height: auto; border-radius: 4px; }}
ul, ol {{ padding-left: 1.5em; }}
li {{ margin: 0.3em 0; }}
input[type="checkbox"] {{ margin-right: 0.5em; }}
</style>
</head>
<body>
{body}
{mermaid_script}
</body>
</html>"#)
}
