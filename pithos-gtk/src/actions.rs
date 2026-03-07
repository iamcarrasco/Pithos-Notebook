use crate::*;
use adw::prelude::*;
use pithos_core::state::*;

// ---------------------------------------------------------------------------
// Content menu
// ---------------------------------------------------------------------------

#[allow(clippy::type_complexity)]
pub fn build_content_menu() -> gtk::gio::Menu {
    let menu = gtk::gio::Menu::new();

    let section1 = gtk::gio::Menu::new();
    section1.append(Some("Rename Note"), Some("win.rename-note"));
    section1.append(Some("Save Snapshot"), Some("win.save-snapshot"));
    section1.append(Some("Version History"), Some("win.version-history"));
    section1.append(Some("Move to Folder\u{2026}"), Some("win.move-to-folder"));
    section1.append(Some("Export\u{2026}"), Some("win.export"));
    menu.append_section(None, &section1);

    let section2 = gtk::gio::Menu::new();
    section2.append(Some("Zen Mode"), Some("win.zen-mode"));
    section2.append(Some("Fullscreen"), Some("win.fullscreen"));
    section2.append(Some("Settings"), Some("win.show-settings"));
    menu.append_section(None, &section2);

    let section3 = gtk::gio::Menu::new();
    section3.append(
        Some("Change Passphrase\u{2026}"),
        Some("win.change-passphrase"),
    );
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

// ---------------------------------------------------------------------------
// Sort order
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Action wiring
// ---------------------------------------------------------------------------

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
            let Some(val) = param.and_then(|p| p.get::<String>()) else {
                return;
            };
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
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| create_folder(&ctx, None));
    }
    window.add_action(&action);

    // New from template
    let action = SimpleAction::new("new-from-template", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| show_template_picker(&ctx));
    }
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
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| toggle_theme(&ctx));
    }
    window.add_action(&action);

    // --- Content menu actions ---

    // Rename note
    let action = SimpleAction::new("rename-note", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| rename_note_dialog(&ctx));
    }
    window.add_action(&action);

    // Save snapshot
    let action = SimpleAction::new("save-snapshot", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| save_manual_snapshot(&ctx));
    }
    window.add_action(&action);

    // Version history
    let action = SimpleAction::new("version-history", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| show_history_dialog(&ctx));
    }
    window.add_action(&action);

    // Move to folder
    let action = SimpleAction::new("move-to-folder", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| move_note_to_folder(&ctx));
    }
    window.add_action(&action);

    // Export
    let action = SimpleAction::new("export", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| export_document(&ctx));
    }
    window.add_action(&action);

    // Spellcheck toggle (stub — requires libspelling Rust bindings)
    let action = SimpleAction::new("toggle-spellcheck", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            let on = {
                let mut s = ctx.state.borrow_mut();
                s.spellcheck_enabled = !s.spellcheck_enabled;
                s.spellcheck_enabled
            };
            send_toast(
                &ctx,
                if on {
                    "Spellcheck on (requires libspelling)"
                } else {
                    "Spellcheck off"
                },
            );
        });
    }
    window.add_action(&action);

    // Zen mode
    let action = SimpleAction::new("zen-mode", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| toggle_zen_mode(&ctx));
    }
    window.add_action(&action);

    // Fullscreen
    let action = SimpleAction::new("fullscreen", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| toggle_fullscreen(&ctx));
    }
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
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| save_document(&ctx));
    }
    window.add_action(&action);

    // Import file (Ctrl+O)
    let action = SimpleAction::new("import-file", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| open_document(&ctx));
    }
    window.add_action(&action);

    // Delete note
    let action = SimpleAction::new("delete-note", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| delete_note(&ctx));
    }
    window.add_action(&action);

    // Empty trash
    let action = SimpleAction::new("empty-trash", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| empty_trash_action(&ctx));
    }
    window.add_action(&action);

    // Daily note
    let action = SimpleAction::new("daily-note", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| open_or_create_daily_note(&ctx));
    }
    window.add_action(&action);

    // Toggle sidebar
    let action = SimpleAction::new("toggle-sidebar", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| toggle_sidebar(&ctx));
    }
    window.add_action(&action);

    // --- Shortcut-only actions (no menu entry) ---

    // Save as (Ctrl+Shift+S)
    let action = SimpleAction::new("save-as", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| save_document_as(&ctx));
    }
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
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| undo(&ctx));
    }
    window.add_action(&action);

    // Redo (Ctrl+Shift+Z / Ctrl+Y)
    let action = SimpleAction::new("redo", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| redo(&ctx));
    }
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
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            md_wrap(&ctx.source_buffer, "**", "**", "bold text");
            process_buffer_change(&ctx);
        });
    }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-italic", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            md_wrap(&ctx.source_buffer, "*", "*", "italic text");
            process_buffer_change(&ctx);
        });
    }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-underline", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            md_wrap(&ctx.source_buffer, "<u>", "</u>", "underlined");
            process_buffer_change(&ctx);
        });
    }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-strike", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            md_wrap(&ctx.source_buffer, "~~", "~~", "strikethrough");
            process_buffer_change(&ctx);
        });
    }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-code", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            md_wrap(&ctx.source_buffer, "`", "`", "code");
            process_buffer_change(&ctx);
        });
    }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-link", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            insert_link(&ctx);
        });
    }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-h1", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            md_line_prefix(&ctx.source_buffer, "# ");
            process_buffer_change(&ctx);
        });
    }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-h2", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            md_line_prefix(&ctx.source_buffer, "## ");
            process_buffer_change(&ctx);
        });
    }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-h3", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            md_line_prefix(&ctx.source_buffer, "### ");
            process_buffer_change(&ctx);
        });
    }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-h4", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            md_line_prefix(&ctx.source_buffer, "#### ");
            process_buffer_change(&ctx);
        });
    }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-h5", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            md_line_prefix(&ctx.source_buffer, "##### ");
            process_buffer_change(&ctx);
        });
    }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-h6", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            md_line_prefix(&ctx.source_buffer, "###### ");
            process_buffer_change(&ctx);
        });
    }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-quote", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            md_line_prefix(&ctx.source_buffer, "> ");
            process_buffer_change(&ctx);
        });
    }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-bullet-list", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            md_line_prefix(&ctx.source_buffer, "- ");
            process_buffer_change(&ctx);
        });
    }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-ordered-list", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            md_line_prefix(&ctx.source_buffer, "1. ");
            process_buffer_change(&ctx);
        });
    }
    window.add_action(&action);

    let action = SimpleAction::new("fmt-task-list", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            md_line_prefix(&ctx.source_buffer, "- [ ] ");
            process_buffer_change(&ctx);
        });
    }
    window.add_action(&action);

    let action = SimpleAction::new("toggle-checkbox", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            toggle_checkbox_at_cursor(&ctx.source_buffer);
            process_buffer_change(&ctx);
        });
    }
    window.add_action(&action);

    // Command palette
    let action = SimpleAction::new("command-palette", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| show_command_palette(&ctx));
    }
    window.add_action(&action);

    // Find in editor (Ctrl+F)
    let action = SimpleAction::new("find-in-editor", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| show_find_bar(&ctx, false));
    }
    window.add_action(&action);

    // Find and replace (Ctrl+H)
    let action = SimpleAction::new("find-replace", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| show_find_bar(&ctx, true));
    }
    window.add_action(&action);

    // Find next
    let action = SimpleAction::new("find-next", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| find_next(&ctx));
    }
    window.add_action(&action);

    // Find previous
    let action = SimpleAction::new("find-prev", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| find_prev(&ctx));
    }
    window.add_action(&action);

    // Hide find bar
    let action = SimpleAction::new("hide-find", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| hide_find_bar(&ctx));
    }
    window.add_action(&action);

    // Replace one
    let action = SimpleAction::new("replace-one", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| replace_one(&ctx));
    }
    window.add_action(&action);

    // Replace all
    let action = SimpleAction::new("replace-all", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| replace_all(&ctx));
    }
    window.add_action(&action);

    // Table editing
    let action = SimpleAction::new("table-add-row", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| table_add_row(&ctx));
    }
    window.add_action(&action);

    let action = SimpleAction::new("table-add-column", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| table_add_column(&ctx));
    }
    window.add_action(&action);

    let action = SimpleAction::new("table-align", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| table_align(&ctx));
    }
    window.add_action(&action);

    // Lock vault — save, clear editor, go to unlock (keeps vault path)
    let action = SimpleAction::new("lock-vault", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            if !perform_vault_save_sync(&ctx) {
                return;
            }
            let vault_folder = ctx.vault_folder.borrow().clone();

            // Clear sensitive key material and stop background timers.
            stop_background_tasks(&ctx);
            *ctx.cached_key.borrow_mut() = None;
            {
                let mut state = ctx.state.borrow_mut();
                state.suppress_sync = true;
            }
            // Clear editor buffer content from memory.
            ctx.source_buffer.set_text("");

            ctx.window.set_content(gtk::Widget::NONE);
            show_unlock_vault_dialog(&ctx.window, vault_folder);
        });
    }
    window.add_action(&action);

    // Change vault — show vault switcher; only tears down editor if user picks a vault
    let action = SimpleAction::new("change-vault", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| {
            show_vault_switcher_dialog(&ctx);
        });
    }
    window.add_action(&action);

    // Change passphrase
    let action = SimpleAction::new("change-passphrase", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| show_change_passphrase_dialog(&ctx));
    }
    window.add_action(&action);

    // Open vault
    let action = SimpleAction::new("open-vault", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| show_open_vault_dialog(&ctx));
    }
    window.add_action(&action);

    // New vault
    let action = SimpleAction::new("new-vault", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| show_new_vault_dialog(&ctx));
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

    // Settings
    let action = SimpleAction::new("show-settings", None);
    {
        let ctx = ctx.clone();
        action.connect_activate(move |_, _| show_settings_dialog(&ctx));
    }
    window.add_action(&action);
}

// ---------------------------------------------------------------------------
// Toolbar signals
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
            if !line_end.ends_line() {
                line_end.forward_to_line_end();
            }
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
    // Table grid cell clicks — each cell reads hover pos to determine size
    {
        let ctx = ctx.clone();
        let hover = tb.table_hover.clone();
        let popover = tb.table_popover.clone();
        let mut child = tb.table_popover.first_child();
        // Walk all descendants to find buttons with "tc-" names
        fn wire_grid_cells(
            widget: &gtk::Widget,
            ctx: &EditorCtx,
            hover: &std::rc::Rc<std::cell::Cell<(i32, i32)>>,
            popover: &gtk::Popover,
        ) {
            if let Ok(btn) = widget.clone().downcast::<gtk::Button>() {
                let name = btn.widget_name();
                if name.starts_with("tc-") {
                    let ctx = ctx.clone();
                    let hover = hover.clone();
                    let popover = popover.clone();
                    btn.connect_clicked(move |_| {
                        let (rows, cols) = hover.get();
                        if rows > 0 && cols > 0 {
                            insert_table_with_size(&ctx.source_buffer, cols, rows);
                            process_buffer_change(&ctx);
                            popover.popdown();
                        }
                    });
                }
            }
            let mut c = widget.first_child();
            while let Some(w) = c {
                wire_grid_cells(&w, ctx, hover, popover);
                c = w.next_sibling();
            }
        }
        while let Some(w) = child {
            wire_grid_cells(&w, &ctx, &hover, &popover);
            child = w.next_sibling();
        }
    }
    {
        let ctx = ctx.clone();
        let popover = tb.table_popover.clone();
        tb.table_add_row.connect_clicked(move |_| {
            table_add_row(&ctx);
            popover.popdown();
        });
    }
    {
        let ctx = ctx.clone();
        let popover = tb.table_popover.clone();
        tb.table_add_col.connect_clicked(move |_| {
            table_add_column(&ctx);
            popover.popdown();
        });
    }
    {
        let ctx = ctx.clone();
        let popover = tb.table_popover.clone();
        tb.table_align.connect_clicked(move |_| {
            table_align(&ctx);
            popover.popdown();
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
        tb.fullscreen
            .connect_clicked(move |_| toggle_fullscreen(&ctx));
    }
    {
        let ctx = ctx.clone();
        tb.image
            .connect_clicked(move |_| insert_image_snippet(&ctx));
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
