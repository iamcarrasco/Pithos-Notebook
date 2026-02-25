use adw::prelude::*;
use std::{cell::Cell, fs, path::PathBuf, rc::Rc};
use pithos_core::state::*;
use pithos_core::crypto;
use pithos_core::vault;
use crate::*;

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
                            "Open Failed",
                            &format!("Could not read file:\n{err}"),
                        ),
                    }
                } else {
                    show_error(
                        &ctx.window,
                        "Open Failed",
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
                        "Save Failed",
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
            "Save Failed",
            &format!("Could not write file:\n{err}"),
        ),
    }
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
    vault::backup_vault(vault_folder);
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
        show_error(
            &ctx.window,
            "Save Failed",
            "Vault is not unlocked or no vault folder is configured.",
        );
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
// Auto-save
// ---------------------------------------------------------------------------

pub fn setup_auto_save(ctx: &EditorCtx) {
    let ctx = ctx.clone();
    glib::timeout_add_seconds_local(AUTO_SAVE_INTERVAL_SECS, move || {
        auto_save_tick(&ctx);
        glib::ControlFlow::Continue
    });
}

/// Watch vault.json for external changes and notify the user.
pub fn watch_vault_file(ctx: &EditorCtx) {
    let vault_folder = ctx.vault_folder.borrow().clone();
    let vault_path = format!("{vault_folder}/vault.json");
    let file = gtk::gio::File::for_path(&vault_path);
    let monitor = match file.monitor_file(gtk::gio::FileMonitorFlags::NONE, None::<&gtk::gio::Cancellable>) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to watch vault file: {e}");
            return;
        }
    };

    let ctx = ctx.clone();
    let last_own_save = Rc::new(Cell::new(std::time::Instant::now()));
    // Record our save times so we can distinguish own vs external changes
    let save_marker = last_own_save.clone();
    let save_gen = ctx.save_generation.clone();
    // Poll save_generation changes to update the marker
    {
        let save_gen = save_gen.clone();
        let save_marker = save_marker.clone();
        let mut prev_gen = save_gen.get();
        glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
            let cur = save_gen.get();
            if cur != prev_gen {
                prev_gen = cur;
                save_marker.set(std::time::Instant::now());
            }
            glib::ControlFlow::Continue
        });
    }
    monitor.connect_changed(move |_, _, _, event| {
        if event == gtk::gio::FileMonitorEvent::ChangesDoneHint {
            // Ignore changes within 3 seconds of our own save
            if last_own_save.get().elapsed() < std::time::Duration::from_secs(3) { return; }
            send_toast(&ctx, "Vault changed externally");
        }
    });

    // Keep monitor alive by leaking it (app has one vault per lifetime)
    // This is acceptable since monitors are lightweight and we want them for the app's duration
    std::mem::forget(monitor);
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
// Close request
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

pub fn show_close_save_failed_dialog(window: &adw::ApplicationWindow, error: &str) {
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
