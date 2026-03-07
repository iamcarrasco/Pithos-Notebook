use crate::*;
use adw::prelude::*;
use pithos_core::crypto;
use pithos_core::state::*;
use pithos_core::vault;
use std::{fs, io::Write, path::PathBuf};

const TYPST_TEMPLATE: &str = include_str!("../../data/typst_template.typ");

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
    dialog.open(
        Some(&window),
        gtk::gio::Cancellable::NONE,
        move |result: Result<gtk::gio::File, gtk::glib::Error>| match result {
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
        },
    );
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
    dialog.save(
        Some(&window),
        gtk::gio::Cancellable::NONE,
        move |result: Result<gtk::gio::File, gtk::glib::Error>| match result {
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
        },
    );
}

const EXPORT_FORMATS: &[(&str, &str)] = &[
    ("Markdown", "md"),
    ("HTML", "html"),
    ("PDF", "pdf"),
    ("Word", "docx"),
    ("LaTeX", "tex"),
    ("EPUB", "epub"),
];

pub fn export_document(ctx: &EditorCtx) {
    let dialog = adw::AlertDialog::new(Some("Export"), Some("Choose a format"));

    let dropdown = gtk::DropDown::from_strings(
        &EXPORT_FORMATS
            .iter()
            .map(|(label, _)| *label)
            .collect::<Vec<_>>(),
    );
    dialog.set_extra_child(Some(&dropdown));

    dialog.add_response("cancel", "Cancel");
    dialog.add_response("export", "Export");
    dialog.set_response_appearance("export", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("export"));
    dialog.set_close_response("cancel");

    let ctx = ctx.clone();
    let window = ctx.window.clone();
    let window_for_present = window.clone();
    dialog.connect_response(None, move |dlg, response| {
        dlg.set_extra_child(gtk::Widget::NONE);
        if response != "export" {
            return;
        }

        let idx = dropdown.selected() as usize;
        let (_, ext) = EXPORT_FORMATS[idx];

        let markdown = current_markdown(&ctx);
        let note_name = {
            let state = ctx.state.borrow();
            find_note_index(&state.notes, &state.active_note_id)
                .map(|i| state.notes[i].name.clone())
                .unwrap_or_else(|| "Untitled".to_string())
        };

        let file_dialog = gtk::FileDialog::builder()
            .title("Export")
            .accept_label("Export")
            .build();

        let suggested = format!("{note_name}.{ext}");
        file_dialog.set_initial_name(Some(&suggested));

        let filter = gtk::FileFilter::new();
        filter.set_name(Some(ext));
        filter.add_pattern(&format!("*.{ext}"));
        let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        file_dialog.set_filters(Some(&filters));

        let ctx = ctx.clone();
        let ext = ext.to_string();
        let window = window.clone();
        file_dialog.save(
            Some(&window),
            gtk::gio::Cancellable::NONE,
            move |result: Result<gtk::gio::File, gtk::glib::Error>| match result {
                Ok(file) => {
                    if let Some(mut path) = file.path() {
                        if path.extension().is_none() {
                            path.set_extension(&ext);
                        }
                        if ext == "md" || ext == "markdown" {
                            match fs::write(&path, &markdown) {
                                Ok(_) => send_toast(&ctx, "Exported as Markdown"),
                                Err(e) => show_error(
                                    &ctx.window,
                                    "Export Failed",
                                    &format!("Could not write file: {e}"),
                                ),
                            }
                        } else {
                            run_pandoc_export(&ctx, &markdown, &path, &note_name);
                        }
                    } else {
                        show_error(
                            &ctx.window,
                            "Export Failed",
                            "This export location is not a local path.",
                        );
                    }
                }
                Err(e) => {
                    if !e.matches(gtk::DialogError::Dismissed) {
                        send_toast(&ctx, &format!("Could not open export dialog: {e}"));
                    }
                }
            },
        );
    });
    dialog.present(Some(&window_for_present));
}

/// Locate the typst binary, checking PATH and common install locations.
fn which_typst() -> Option<PathBuf> {
    // Check PATH via `which`
    if let Ok(out) = std::process::Command::new("which").arg("typst").output() {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() {
                return Some(PathBuf::from(p));
            }
        }
    }
    // Common locations
    for dir in &[".local/bin", ".cargo/bin"] {
        if let Some(home) = std::env::var_os("HOME") {
            let p = PathBuf::from(home).join(dir).join("typst");
            if p.exists() {
                return Some(p);
            }
        }
    }
    // System paths
    for p in &["/usr/local/bin/typst", "/usr/bin/typst", "/snap/bin/typst"] {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn write_unique_temp_file(prefix: &str, ext: &str, data: &[u8]) -> std::io::Result<PathBuf> {
    use std::io::ErrorKind;

    let tmp_dir = std::env::temp_dir();
    for _ in 0..32 {
        let nonce = rand::random::<u64>();
        let path = tmp_dir.join(format!(
            "{prefix}-{}-{nonce:016x}.{ext}",
            std::process::id()
        ));
        let mut file = match fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
        {
            Ok(file) => file,
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        if let Err(e) = file.write_all(data) {
            let _ = fs::remove_file(&path);
            return Err(e);
        }
        return Ok(path);
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "Could not allocate a unique temporary file",
    ))
}

fn run_pandoc_export(ctx: &EditorCtx, markdown: &str, output: &std::path::Path, title: &str) {
    let input_path = match write_unique_temp_file("pithos-export", "md", markdown.as_bytes()) {
        Ok(path) => path,
        Err(e) => {
            show_error(
                &ctx.window,
                "Export Failed",
                &format!("Could not write temp file: {e}"),
            );
            return;
        }
    };
    let output = output.to_path_buf();
    let input = input_path.clone();
    let title = title.to_string();

    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();

    let is_pdf = output
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"));

    std::thread::spawn(move || {
        let mut cmd = std::process::Command::new("pandoc");
        cmd.arg("-s")
            .arg("--metadata")
            .arg(format!("title={title}"))
            .arg("-f")
            .arg("markdown");

        let mut template_path: Option<PathBuf> = None;
        if is_pdf {
            // Write the bundled typst template to a temp file for pandoc to use.
            let temp_template_path = match write_unique_temp_file(
                "pithos-typst-template",
                "typ",
                TYPST_TEMPLATE.as_bytes(),
            ) {
                Ok(path) => path,
                Err(e) => {
                    let _ = fs::remove_file(&input);
                    let _ = tx.send(Err(format!("Could not write temp template file: {e}")));
                    return;
                }
            };
            template_path = Some(temp_template_path.clone());
            let typst_path = which_typst().unwrap_or_else(|| "typst".into());
            cmd.arg(format!("--pdf-engine={}", typst_path.display()));
            // Typst treats import paths that start with '/' as project-root relative.
            // Force root to filesystem '/' so '/tmp/...' resolves correctly.
            cmd.arg("--pdf-engine-opt=--root=/");
            cmd.arg("-V")
                .arg(format!("template={}", temp_template_path.display()));
            cmd.arg("--variable=papersize:a4");
        }

        cmd.arg("-o").arg(&output).arg(&input);

        let result = cmd.output();

        let _ = fs::remove_file(&input);
        if let Some(path) = template_path {
            let _ = fs::remove_file(path);
        }

        match result {
            Ok(out) if out.status.success() => {
                let _ = tx.send(Ok(()));
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                let _ = tx.send(Err(stderr));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let _ = tx.send(Err(
                    "Pandoc is not installed.\n\nInstall it with:\n  sudo apt install pandoc"
                        .to_string(),
                ));
            }
            Err(e) => {
                let _ = tx.send(Err(format!("Failed to run pandoc: {e}")));
            }
        }
    });

    let ctx = ctx.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        match rx.try_recv() {
            Ok(Ok(())) => {
                send_toast(&ctx, "Exported");
                glib::ControlFlow::Break
            }
            Ok(Err(e)) => {
                show_error(&ctx.window, "Export Failed", &e);
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                show_error(&ctx.window, "Export Failed", "Export thread disconnected");
                glib::ControlFlow::Break
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
    let source_id =
        glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
            ctx_clone.save_timeout_id.set(None);
            perform_vault_save_async(&ctx_clone, false);
        });

    ctx.save_timeout_id.set(Some(source_id));
}

/// Collect vault data on the main thread (cheap), return everything needed for I/O.
pub fn prepare_vault_save(
    ctx: &EditorCtx,
) -> Option<(vault::VaultData, crypto::CachedKey, String)> {
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
    let mut json =
        serde_json::to_string_pretty(&vault_data).map_err(|e| format!("Serialization: {e}"))?;
    let result = crypto::encrypt_vault_fast(&json, key).map_err(|e| format!("Encryption: {e}"));
    json.zeroize(); // Wipe plaintext vault contents from memory
    let encrypted = result?;
    vault::write_vault_raw(vault_folder, &encrypted).map_err(|e| format!("Write: {e}"))?;
    Ok(())
}

/// Synchronous vault save — used only for vault-switch flows where we must block.
/// If an async save is in flight, bump the generation so the async callback
/// won't overwrite our (newer) sync write, then proceed with the sync save.
pub fn perform_vault_save_sync(ctx: &EditorCtx) -> bool {
    // Invalidate any in-flight async save so its completion callback won't
    // re-save or mark the document clean with stale data.
    if ctx.saving.get() {
        ctx.save_generation.set(ctx.save_generation.get().wrapping_add(2));
    }
    let Some((vault_data, key, vault_folder)) = prepare_vault_save(ctx) else {
        return false;
    };
    match vault_save_blocking(vault_data, &key, &vault_folder) {
        Ok(()) => {
            send_toast(ctx, "Saved");
            true
        }
        Err(e) => {
            show_error(
                &ctx.window,
                "Save Failed",
                &format!("Vault save failed: {e}"),
            );
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
                ctx.last_save_completed.set(std::time::Instant::now());
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

pub fn stop_background_tasks(ctx: &EditorCtx) {
    if let Some(source_id) = ctx.save_timeout_id.take() {
        source_id.remove();
    }
    if let Some(source_id) = ctx.sync_timeout_id.take() {
        source_id.remove();
    }
    if let Some(source_id) = ctx.search_timeout_id.take() {
        source_id.remove();
    }
    if let Some(source_id) = ctx.auto_save_timeout_id.take() {
        source_id.remove();
    }
    if let Some(monitor) = ctx.vault_file_monitor.borrow_mut().take() {
        monitor.cancel();
    }
}

pub fn setup_auto_save(ctx: &EditorCtx) {
    if let Some(source_id) = ctx.auto_save_timeout_id.take() {
        source_id.remove();
    }

    let ctx_clone = ctx.clone();
    let source_id = glib::timeout_add_seconds_local(AUTO_SAVE_INTERVAL_SECS, move || {
        auto_save_tick(&ctx_clone);
        glib::ControlFlow::Continue
    });

    ctx.auto_save_timeout_id.set(Some(source_id));
}

/// Watch vault.json for external changes and notify the user.
pub fn watch_vault_file(ctx: &EditorCtx) {
    if let Some(monitor) = ctx.vault_file_monitor.borrow_mut().take() {
        monitor.cancel();
    }

    let vault_folder = ctx.vault_folder.borrow().clone();
    let vault_path = format!("{vault_folder}/vault.json");
    let file = gtk::gio::File::for_path(&vault_path);
    let monitor = match file.monitor_file(
        gtk::gio::FileMonitorFlags::NONE,
        None::<&gtk::gio::Cancellable>,
    ) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to watch vault file: {e}");
            return;
        }
    };

    let saving = ctx.saving.clone();
    let last_save_completed = ctx.last_save_completed.clone();
    let toast_overlay = ctx.toast_overlay.clone();
    monitor.connect_changed(move |_, _, _, event| {
        if event == gtk::gio::FileMonitorEvent::ChangesDoneHint {
            // Ignore changes within 3 seconds of our own save completing,
            // or while a save is still in flight
            if saving.get() {
                return;
            }
            if last_save_completed.get().elapsed() < std::time::Duration::from_secs(3) {
                return;
            }
            let toast = adw::Toast::new("Vault changed externally");
            toast.set_timeout(2);
            toast_overlay.add_toast(toast);
        }
    });

    *ctx.vault_file_monitor.borrow_mut() = Some(monitor);
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
            pithos_core::notes::push_snapshot(&mut state.notes[index], content);
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
