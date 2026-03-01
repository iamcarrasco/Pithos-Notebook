use crate::*;
use adw::prelude::*;
use pithos_core::crypto;
use pithos_core::state::*;
use pithos_core::vault;
use std::cell::RefCell;
use std::rc::Rc;

/// Save app config, logging failures to stderr (non-critical — just vault path persistence).
fn save_config_or_log(config: &vault::AppConfig) {
    if let Err(e) = vault::save_config(config) {
        eprintln!("Failed to save config: {e}");
    }
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

    let subtitle = gtk::Label::new(Some("A private, offline, encrypted markdown notebook"));
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
                                save_config_or_log(&vault::AppConfig {
                                    vault_path: Some(folder.clone()),
                                    ..Default::default()
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

/// Vault switcher shown from an active editor session.
/// Dismissing the dialog simply returns to the editor — no teardown.
/// Only when the user commits to opening/creating a vault do we save + switch.
pub fn show_vault_switcher_dialog(ctx: &EditorCtx) {
    let dialog = adw::Window::builder()
        .transient_for(&ctx.window)
        .modal(true)
        .title("Change Vault")
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

    let title = gtk::Label::new(Some("Change Vault"));
    title.add_css_class("title-2");
    vbox.append(&title);

    let subtitle = gtk::Label::new(Some(
        "Open a different vault or create a new one.\nYour current vault will be saved and locked.",
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

    // Dismissing just closes the dialog — editor stays intact (no quit logic)

    // Helper: save + teardown the current editor session.
    // Returns false when save fails so the caller can abort switching.
    fn teardown(ctx: &EditorCtx) -> bool {
        if !perform_vault_save_sync(ctx) {
            return false;
        }
        save_config_or_log(&vault::AppConfig {
            vault_path: None,
            ..Default::default()
        });
        stop_background_tasks(ctx);
        *ctx.cached_key.borrow_mut() = None;
        {
            let mut state = ctx.state.borrow_mut();
            state.suppress_sync = true;
        }
        ctx.source_buffer.set_text("");
        ctx.window.set_content(gtk::Widget::NONE);
        true
    }

    // Create New Vault
    {
        let ctx = ctx.clone();
        let dialog = dialog.clone();
        create_btn.connect_clicked(move |_| {
            if !teardown(&ctx) {
                return;
            }
            transition_close(&dialog);
            show_create_vault_dialog(&ctx.window);
        });
    }

    // Open Existing Vault
    {
        let ctx = ctx.clone();
        let dialog = dialog.clone();
        open_btn.connect_clicked(move |_| {
            let chooser = gtk::FileDialog::builder()
                .title("Select Vault Folder")
                .accept_label("Select")
                .build();
            let ctx = ctx.clone();
            let dlg = dialog.clone();
            let dlg_for_parent = dlg.clone();
            chooser.select_folder(Some(&dlg_for_parent), gtk::gio::Cancellable::NONE, move |result| {
                match result {
                    Ok(file) => {
                        if let Some(path) = file.path() {
                            let folder = path.to_string_lossy().to_string();
                            if vault::vault_file_path(&folder).exists() {
                                if !teardown(&ctx) {
                                    return;
                                }
                                save_config_or_log(&vault::AppConfig {
                                    vault_path: Some(folder.clone()),
                                    ..Default::default()
                                });
                                transition_close(&dlg);
                                show_unlock_vault_dialog(&ctx.window, folder);
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
        "Choose a folder and set a passphrase to encrypt your notes",
    ));
    subtitle.add_css_class("dim-label");
    subtitle.set_wrap(true);
    vbox.append(&subtitle);

    let folder_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let folder_label = gtk::Label::new(Some("No folder selected"));
    folder_label.set_hexpand(true);
    folder_label.set_xalign(0.0);
    folder_label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    let folder_btn = gtk::Button::with_label("Choose Folder\u{2026}");
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

    let strength_label = gtk::Label::new(None);
    strength_label.set_xalign(0.0);
    strength_label.add_css_class("caption");
    strength_label.set_visible(false);
    vbox.append(&strength_label);

    // Update strength indicator as the user types
    {
        let strength_label = strength_label.clone();
        pass1.connect_changed(move |entry| {
            let text = entry.text();
            let len = text.len();
            if len == 0 {
                strength_label.set_visible(false);
                return;
            }
            strength_label.set_visible(true);

            let has_upper = text.chars().any(|c| c.is_uppercase());
            let has_lower = text.chars().any(|c| c.is_lowercase());
            let has_digit = text.chars().any(|c| c.is_ascii_digit());
            let has_special = text.chars().any(|c| !c.is_alphanumeric());
            let variety = [has_upper, has_lower, has_digit, has_special]
                .iter()
                .filter(|&&v| v)
                .count();

            // Remove previous strength classes
            for class in &["success", "warning", "error"] {
                strength_label.remove_css_class(class);
            }

            let (label, class) = if len < 8 {
                ("Too short \u{2014} minimum 8 characters", "error")
            } else if len < 12 || variety < 2 {
                ("Weak", "warning")
            } else if len < 16 || variety < 3 {
                ("Fair", "warning")
            } else {
                ("Strong", "success")
            };
            strength_label.set_label(label);
            strength_label.add_css_class(class);
        });
    }

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
            chooser.select_folder(
                Some(&dlg),
                gtk::gio::Cancellable::NONE,
                move |result: Result<gtk::gio::File, gtk::glib::Error>| match result {
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
                },
            );
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
            use zeroize::Zeroize;
            let create_btn = create_btn_inner.clone();
            let mut p1 = pass1.text().to_string();
            let mut p2 = pass2.text().to_string();
            let fp = folder_path.borrow().clone();

            if fp.is_none() {
                p1.zeroize();
                p2.zeroize();
                error_label.set_label("Please select a folder");
                error_label.set_visible(true);
                return;
            }
            if p1.len() < 8 {
                p1.zeroize();
                p2.zeroize();
                pass1.set_text("");
                pass2.set_text("");
                error_label.set_label("Passphrase must be at least 8 characters");
                error_label.set_visible(true);
                return;
            }
            if p1 != p2 {
                p1.zeroize();
                p2.zeroize();
                pass1.set_text("");
                pass2.set_text("");
                error_label.set_label("Passphrases do not match");
                error_label.set_visible(true);
                return;
            }
            p2.zeroize();
            pass2.set_text("");

            let Some(vault_folder) = fp else {
                error_label.set_label("Please select a folder");
                error_label.set_visible(true);
                return;
            };

            // If the selected folder already contains a vault, switch to unlock flow
            if vault::vault_file_path(&vault_folder).exists() {
                save_config_or_log(&vault::AppConfig {
                    vault_path: Some(vault_folder.clone()),
                    ..Default::default()
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
                        if let Err(e) = vault::write_vault_raw(&vault_folder_for_thread, &encrypted)
                        {
                            let _ = tx.send(Err(format!("Write failed: {e}")));
                        } else {
                            let _ = tx.send(Ok(cached_key));
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(format!("Encryption failed: {e}")));
                    }
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
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        return glib::ControlFlow::Continue
                    }
                    Err(_) => return glib::ControlFlow::Break,
                };
                match result {
                    Ok(cached_key) => {
                        save_config_or_log(&vault::AppConfig {
                            vault_path: Some(vault_folder_c.clone()),
                            ..Default::default()
                        });
                        gtk::prelude::GtkWindowExt::set_focus(&window, gtk::Widget::NONE);
                        transition_close(&dialog);
                        let window = window.clone();
                        let vault_folder_c = vault_folder_c.clone();
                        glib::timeout_add_local_once(
                            std::time::Duration::from_millis(50),
                            move || {
                                build_editor(
                                    &window,
                                    DocState::default(),
                                    vault_folder_c,
                                    cached_key,
                                );
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

    let subtitle = gtk::Label::new(Some("Enter your passphrase to decrypt your notes"));
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
            pass_entry.set_text(""); // clear passphrase from UI immediately
            if passphrase.is_empty() {
                error_label.set_label("Please enter your passphrase");
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

            let (tx, rx) =
                std::sync::mpsc::channel::<Result<(String, crypto::CachedKey), String>>();
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
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        return glib::ControlFlow::Continue
                    }
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
                            error_label.set_label("Wrong passphrase, try again");
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
            save_config_or_log(&vault::AppConfig {
                vault_path: None,
                ..Default::default()
            });
            transition_close(&dialog);
            show_welcome_dialog(&window);
        });
    }

    dialog.present();
    pass_entry.grab_focus();
}

// ---------------------------------------------------------------------------
// Command palette
// ---------------------------------------------------------------------------

pub struct CommandEntry {
    pub label: String,
    pub accel: String,
    pub action_name: String,
}

pub fn build_command_entries() -> Vec<CommandEntry> {
    vec![
        CommandEntry {
            label: "New Note".into(),
            accel: "Ctrl+N".into(),
            action_name: "win.new-note".into(),
        },
        CommandEntry {
            label: "Save".into(),
            accel: "Ctrl+S".into(),
            action_name: "win.save-vault".into(),
        },
        CommandEntry {
            label: "Save As\u{2026}".into(),
            accel: "Ctrl+Shift+S".into(),
            action_name: "win.save-as".into(),
        },
        CommandEntry {
            label: "Import File\u{2026}".into(),
            accel: "Ctrl+O".into(),
            action_name: "win.import-file".into(),
        },
        CommandEntry {
            label: "Rename Note".into(),
            accel: "F2".into(),
            action_name: "win.rename-note".into(),
        },
        CommandEntry {
            label: "Delete Note".into(),
            accel: "".into(),
            action_name: "win.delete-note".into(),
        },
        CommandEntry {
            label: "Close Tab".into(),
            accel: "Ctrl+W".into(),
            action_name: "win.close-tab".into(),
        },
        CommandEntry {
            label: "Undo".into(),
            accel: "Ctrl+Z".into(),
            action_name: "win.undo".into(),
        },
        CommandEntry {
            label: "Redo".into(),
            accel: "Ctrl+Shift+Z".into(),
            action_name: "win.redo".into(),
        },
        CommandEntry {
            label: "Toggle Sidebar".into(),
            accel: "Ctrl+\\".into(),
            action_name: "win.toggle-sidebar".into(),
        },
        CommandEntry {
            label: "Zen Mode".into(),
            accel: "Ctrl+Shift+J".into(),
            action_name: "win.zen-mode".into(),
        },
        CommandEntry {
            label: "Fullscreen".into(),
            accel: "F11".into(),
            action_name: "win.fullscreen".into(),
        },
        CommandEntry {
            label: "Toggle Theme".into(),
            accel: "Ctrl+Shift+D".into(),
            action_name: "win.toggle-theme".into(),
        },
        CommandEntry {
            label: "Focus Search".into(),
            accel: "Ctrl+Shift+F".into(),
            action_name: "win.focus-search".into(),
        },
        CommandEntry {
            label: "Daily Note".into(),
            accel: "Ctrl+Shift+T".into(),
            action_name: "win.daily-note".into(),
        },
        CommandEntry {
            label: "New Folder".into(),
            accel: "".into(),
            action_name: "win.new-folder".into(),
        },
        CommandEntry {
            label: "New from Template\u{2026}".into(),
            accel: "".into(),
            action_name: "win.new-from-template".into(),
        },
        CommandEntry {
            label: "View Trash".into(),
            accel: "".into(),
            action_name: "win.view-trash".into(),
        },
        CommandEntry {
            label: "Save Snapshot".into(),
            accel: "".into(),
            action_name: "win.save-snapshot".into(),
        },
        CommandEntry {
            label: "Version History".into(),
            accel: "".into(),
            action_name: "win.version-history".into(),
        },
        CommandEntry {
            label: "Move to Folder\u{2026}".into(),
            accel: "".into(),
            action_name: "win.move-to-folder".into(),
        },
        CommandEntry {
            label: "Export\u{2026}".into(),
            accel: "Ctrl+Shift+E".into(),
            action_name: "win.export".into(),
        },
        CommandEntry {
            label: "Bold".into(),
            accel: "Ctrl+B".into(),
            action_name: "win.fmt-bold".into(),
        },
        CommandEntry {
            label: "Italic".into(),
            accel: "Ctrl+I".into(),
            action_name: "win.fmt-italic".into(),
        },
        CommandEntry {
            label: "Underline".into(),
            accel: "Ctrl+U".into(),
            action_name: "win.fmt-underline".into(),
        },
        CommandEntry {
            label: "Strikethrough".into(),
            accel: "Ctrl+D".into(),
            action_name: "win.fmt-strike".into(),
        },
        CommandEntry {
            label: "Inline Code".into(),
            accel: "Ctrl+E".into(),
            action_name: "win.fmt-code".into(),
        },
        CommandEntry {
            label: "Insert Link".into(),
            accel: "Ctrl+K".into(),
            action_name: "win.fmt-link".into(),
        },
        CommandEntry {
            label: "Heading 1".into(),
            accel: "Ctrl+1".into(),
            action_name: "win.fmt-h1".into(),
        },
        CommandEntry {
            label: "Heading 2".into(),
            accel: "Ctrl+2".into(),
            action_name: "win.fmt-h2".into(),
        },
        CommandEntry {
            label: "Heading 3".into(),
            accel: "Ctrl+3".into(),
            action_name: "win.fmt-h3".into(),
        },
        CommandEntry {
            label: "Heading 4".into(),
            accel: "Ctrl+4".into(),
            action_name: "win.fmt-h4".into(),
        },
        CommandEntry {
            label: "Heading 5".into(),
            accel: "Ctrl+5".into(),
            action_name: "win.fmt-h5".into(),
        },
        CommandEntry {
            label: "Heading 6".into(),
            accel: "Ctrl+6".into(),
            action_name: "win.fmt-h6".into(),
        },
        CommandEntry {
            label: "Block Quote".into(),
            accel: "Ctrl+Shift+Q".into(),
            action_name: "win.fmt-quote".into(),
        },
        CommandEntry {
            label: "Bullet List".into(),
            accel: "Ctrl+Shift+L".into(),
            action_name: "win.fmt-bullet-list".into(),
        },
        CommandEntry {
            label: "Ordered List".into(),
            accel: "".into(),
            action_name: "win.fmt-ordered-list".into(),
        },
        CommandEntry {
            label: "Task List".into(),
            accel: "".into(),
            action_name: "win.fmt-task-list".into(),
        },
        CommandEntry {
            label: "Toggle Checkbox".into(),
            accel: "Ctrl+Space".into(),
            action_name: "win.toggle-checkbox".into(),
        },
        CommandEntry {
            label: "Change Passphrase\u{2026}".into(),
            accel: "".into(),
            action_name: "win.change-passphrase".into(),
        },
        CommandEntry {
            label: "Find\u{2026}".into(),
            accel: "Ctrl+F".into(),
            action_name: "win.find-in-editor".into(),
        },
        CommandEntry {
            label: "Find and Replace\u{2026}".into(),
            accel: "Ctrl+H".into(),
            action_name: "win.find-replace".into(),
        },
        CommandEntry {
            label: "Find Next".into(),
            accel: "".into(),
            action_name: "win.find-next".into(),
        },
        CommandEntry {
            label: "Find Previous".into(),
            accel: "".into(),
            action_name: "win.find-prev".into(),
        },
        CommandEntry {
            label: "Table: Add Row".into(),
            accel: "".into(),
            action_name: "win.table-add-row".into(),
        },
        CommandEntry {
            label: "Table: Add Column".into(),
            accel: "".into(),
            action_name: "win.table-add-column".into(),
        },
        CommandEntry {
            label: "Table: Align".into(),
            accel: "".into(),
            action_name: "win.table-align".into(),
        },
        CommandEntry {
            label: "Toggle Spellcheck".into(),
            accel: "".into(),
            action_name: "win.toggle-spellcheck".into(),
        },
        CommandEntry {
            label: "Open Vault\u{2026}".into(),
            accel: "".into(),
            action_name: "win.open-vault".into(),
        },
        CommandEntry {
            label: "New Vault\u{2026}".into(),
            accel: "".into(),
            action_name: "win.new-vault".into(),
        },
        CommandEntry {
            label: "Lock Vault".into(),
            accel: "".into(),
            action_name: "win.lock-vault".into(),
        },
        CommandEntry {
            label: "Help".into(),
            accel: "F1".into(),
            action_name: "win.show-help".into(),
        },
        CommandEntry {
            label: "Settings".into(),
            accel: "".into(),
            action_name: "win.show-settings".into(),
        },
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
        key_ctl.connect_key_pressed(move |_, key, _, _| match key {
            gdk::Key::Down => {
                if let Some(row) = list_ref.selected_row() {
                    let idx = row.index();
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
    if let Some(name) = action_name.strip_prefix("win.") {
        ActionGroupExt::activate_action(window, name, None);
    }
}

// ---------------------------------------------------------------------------
// Misc dialogs (history, rename)
// ---------------------------------------------------------------------------

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
            Some("No snapshots available yet"),
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

pub fn rename_note_dialog(ctx: &EditorCtx) {
    let current_name = {
        let state = ctx.state.borrow();
        find_note_index(&state.notes, &state.active_note_id)
            .map(|i| state.notes[i].name.clone())
            .unwrap_or_else(|| "Untitled".to_string())
    };

    let dialog = adw::AlertDialog::new(Some("Rename Note"), Some("Enter a new name for this note"));

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
        let new_name = entry.text().trim().to_string();
        dlg.set_extra_child(gtk::Widget::NONE);
        if response == "rename" && !new_name.is_empty() {
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
                if let Some(index) = find_note_index(&state.notes, &state.active_note_id) {
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
// Change passphrase dialog
// ---------------------------------------------------------------------------

pub fn show_change_passphrase_dialog(ctx: &EditorCtx) {
    let dialog = adw::Window::builder()
        .transient_for(&ctx.window)
        .modal(true)
        .title("Change Passphrase")
        .default_width(420)
        .default_height(360)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 12);
    vbox.set_margin_start(24);
    vbox.set_margin_end(24);
    vbox.set_margin_top(24);
    vbox.set_margin_bottom(24);

    let title = gtk::Label::new(Some("Change Vault Passphrase"));
    title.add_css_class("title-2");
    vbox.append(&title);

    let subtitle = gtk::Label::new(Some("Enter your current passphrase, then choose a new one"));
    subtitle.add_css_class("dim-label");
    subtitle.set_wrap(true);
    vbox.append(&subtitle);

    let current_pass = gtk::PasswordEntry::builder()
        .placeholder_text("Current passphrase")
        .show_peek_icon(true)
        .build();
    vbox.append(&current_pass);

    let new_pass1 = gtk::PasswordEntry::builder()
        .placeholder_text("New passphrase")
        .show_peek_icon(true)
        .build();
    vbox.append(&new_pass1);

    // Strength indicator for new passphrase
    let strength_label = gtk::Label::new(None);
    strength_label.set_xalign(0.0);
    strength_label.add_css_class("caption");
    strength_label.set_visible(false);
    vbox.append(&strength_label);

    {
        let strength_label = strength_label.clone();
        new_pass1.connect_changed(move |entry| {
            let text = entry.text();
            let len = text.len();
            if len == 0 {
                strength_label.set_visible(false);
                return;
            }
            strength_label.set_visible(true);

            let has_upper = text.chars().any(|c| c.is_uppercase());
            let has_lower = text.chars().any(|c| c.is_lowercase());
            let has_digit = text.chars().any(|c| c.is_ascii_digit());
            let has_special = text.chars().any(|c| !c.is_alphanumeric());
            let variety = [has_upper, has_lower, has_digit, has_special]
                .iter()
                .filter(|&&v| v)
                .count();

            for class in &["success", "warning", "error"] {
                strength_label.remove_css_class(class);
            }

            let (label, class) = if len < 8 {
                ("Too short \u{2014} minimum 8 characters", "error")
            } else if len < 12 || variety < 2 {
                ("Weak", "warning")
            } else if len < 16 || variety < 3 {
                ("Fair", "warning")
            } else {
                ("Strong", "success")
            };
            strength_label.set_label(label);
            strength_label.add_css_class(class);
        });
    }

    let new_pass2 = gtk::PasswordEntry::builder()
        .placeholder_text("Confirm new passphrase")
        .show_peek_icon(true)
        .build();
    vbox.append(&new_pass2);

    let error_label = gtk::Label::new(None);
    error_label.add_css_class("error");
    error_label.set_visible(false);
    vbox.append(&error_label);

    let change_btn = gtk::Button::with_label("Change Passphrase");
    change_btn.add_css_class("suggested-action");
    change_btn.add_css_class("pill");
    vbox.append(&change_btn);

    let toolbar = adw::HeaderBar::new();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&toolbar);
    content.append(&vbox);
    dialog.set_content(Some(&content));

    {
        let ctx = ctx.clone();
        let dialog = dialog.clone();
        let change_btn_inner = change_btn.clone();
        change_btn.connect_clicked(move |_| {
            let change_btn = change_btn_inner.clone();
            let old_pass = current_pass.text().to_string();
            let p1 = new_pass1.text().to_string();
            let p2 = new_pass2.text().to_string();

            if old_pass.is_empty() {
                error_label.set_label("Enter your current passphrase");
                error_label.set_visible(true);
                return;
            }
            if p1.len() < 8 {
                error_label.set_label("New passphrase must be at least 8 characters");
                error_label.set_visible(true);
                return;
            }
            if p1 != p2 {
                error_label.set_label("New passphrases do not match");
                error_label.set_visible(true);
                return;
            }

            let vault_folder = ctx.vault_folder.borrow().clone();
            if vault_folder.is_empty() {
                error_label.set_label("No vault is open");
                error_label.set_visible(true);
                return;
            }

            // Collect asset IDs to re-encrypt
            let asset_ids: Vec<String> = ctx.state.borrow().assets.keys().cloned().collect();

            change_btn.set_sensitive(false);
            error_label.set_label("Changing passphrase\u{2026}");
            error_label.remove_css_class("error");
            error_label.set_visible(true);

            // Refuse to start if an async save is already in flight — its thread
            // could write with the old key after we've re-encrypted.
            if ctx.saving.get() {
                error_label.add_css_class("error");
                error_label.set_label("A save is in progress — please try again in a moment");
                error_label.set_visible(true);
                change_btn.set_sensitive(true);
                return;
            }

            // Cancel any pending autosave and block new ones during re-encryption.
            if let Some(source_id) = ctx.save_timeout_id.take() {
                source_id.remove();
            }
            ctx.saving.set(true);

            let (tx, rx) = std::sync::mpsc::channel::<Result<crypto::CachedKey, String>>();
            let vault_folder_t = vault_folder.clone();
            std::thread::spawn(move || {
                use zeroize::Zeroize;
                let mut old_pass = old_pass;
                let mut p1 = p1;

                // Inner closure does the real work; passphrases are zeroized after.
                let result = (|| -> Result<crypto::CachedKey, String> {
                    // 1) Read and verify current passphrase (also gets plaintext + old key)
                    let raw = vault::read_vault_raw(&vault_folder_t)
                        .map_err(|e| format!("Read error: {e}"))?
                        .ok_or_else(|| "Vault file not found.".to_string())?;
                    let (plaintext, old_key) = crypto::decrypt_vault_returning_key(&raw, &old_pass)
                        .map_err(|_| "Wrong current passphrase.".to_string())?;

                    // 2) Derive new key
                    let new_key = crypto::CachedKey::derive(&p1);

                    // 3) Re-encrypt vault with new key
                    let reencrypted = crypto::encrypt_vault_fast(&plaintext, &new_key)
                        .map_err(|e| format!("Encryption failed: {e}"))?;

                    // 4) Re-encrypt ALL assets to memory first (transactional).
                    //    Only write to disk if every asset succeeds.
                    let assets_path = vault::assets_dir(&vault_folder_t);
                    let mut reencrypted_assets: Vec<(String, Vec<u8>)> = Vec::new();
                    for asset_id in &asset_ids {
                        if !vault::is_valid_asset_id(asset_id) {
                            return Err(format!("Invalid asset ID: {asset_id}"));
                        }
                        let path = assets_path.join(asset_id);
                        let data = std::fs::read(&path)
                            .map_err(|e| format!("Failed to read asset {asset_id}: {e}"))?;
                        let decrypted = crypto::decrypt_asset(&data, &old_key)
                            .map_err(|e| format!("Failed to decrypt asset {asset_id}: {e}"))?;
                        let encrypted_str = crypto::encrypt_asset(&decrypted, &new_key)
                            .map_err(|e| format!("Failed to re-encrypt asset {asset_id}: {e}"))?;
                        reencrypted_assets.push((asset_id.clone(), encrypted_str.into_bytes()));
                    }

                    // 5) Write new vault FIRST — if this fails, no assets
                    //    have been touched and everything stays on the old key.
                    vault::write_vault_raw(&vault_folder_t, &reencrypted)
                        .map_err(|e| format!("Write failed: {e}"))?;

                    // 6) Commit re-encrypted assets to disk.
                    //    The vault is already on the new key, so if an asset
                    //    write fails the user can retry with the new passphrase.
                    for (asset_id, data) in &reencrypted_assets {
                        vault::write_asset(&vault_folder_t, asset_id, data)
                            .map_err(|e| format!("Failed to write asset {asset_id}: {e}"))?;
                    }

                    Ok(new_key)
                })();

                // Zeroize passphrase strings regardless of success/failure.
                old_pass.zeroize();
                p1.zeroize();

                let _ = tx.send(result);
            });

            let ctx = ctx.clone();
            let dialog = dialog.clone();
            let error_label = error_label.clone();
            let change_btn = change_btn.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                let result = match rx.try_recv() {
                    Ok(r) => r,
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        return glib::ControlFlow::Continue
                    }
                    Err(_) => return glib::ControlFlow::Break,
                };
                // Re-enable autosave now that re-encryption is done.
                ctx.saving.set(false);
                match result {
                    Ok(new_key) => {
                        *ctx.cached_key.borrow_mut() = Some(new_key);
                        send_toast(&ctx, "Passphrase changed successfully");
                        dialog.close();
                    }
                    Err(e) => {
                        error_label.add_css_class("error");
                        error_label.set_label(&e);
                        change_btn.set_sensitive(true);
                    }
                }
                glib::ControlFlow::Break
            });
        });
    }

    dialog.present();
}

// ---------------------------------------------------------------------------
// Vault switching dialogs
// ---------------------------------------------------------------------------

pub fn show_open_vault_dialog(ctx: &EditorCtx) {
    let chooser = gtk::FileDialog::builder()
        .title("Open Vault Folder")
        .accept_label("Open")
        .build();

    let ctx = ctx.clone();
    let window = ctx.window.clone();
    chooser.select_folder(Some(&window), gtk::gio::Cancellable::NONE, move |result| {
        match result {
            Ok(file) => {
                if let Some(path) = file.path() {
                    let folder = path.to_string_lossy().to_string();
                    if vault::vault_file_path(&folder).exists() {
                        // Save current vault, lock, switch
                        if !perform_vault_save_sync(&ctx) {
                            return;
                        }
                        stop_background_tasks(&ctx);
                        *ctx.cached_key.borrow_mut() = None;
                        {
                            let mut state = ctx.state.borrow_mut();
                            state.suppress_sync = true;
                        }
                        ctx.source_buffer.set_text("");
                        save_config_or_log(&vault::AppConfig {
                            vault_path: Some(folder.clone()),
                            ..Default::default()
                        });
                        ctx.window.set_content(gtk::Widget::NONE);
                        show_unlock_vault_dialog(&ctx.window, folder);
                    } else {
                        show_error(
                            &ctx.window,
                            "No Vault Found",
                            "The selected folder does not contain a vault file",
                        );
                    }
                }
            }
            Err(e) => {
                if !e.matches(gtk::DialogError::Dismissed) {
                    send_toast(&ctx, &format!("Could not open folder picker: {e}"));
                }
            }
        }
    });
}

pub fn show_new_vault_dialog(ctx: &EditorCtx) {
    if !perform_vault_save_sync(ctx) {
        return;
    }
    stop_background_tasks(ctx);
    *ctx.cached_key.borrow_mut() = None;
    {
        let mut state = ctx.state.borrow_mut();
        state.suppress_sync = true;
    }
    ctx.source_buffer.set_text("");
    ctx.window.set_content(gtk::Widget::NONE);
    show_create_vault_dialog(&ctx.window);
}

// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Settings dialog
// ---------------------------------------------------------------------------

pub fn show_settings_dialog(ctx: &EditorCtx) {
    let dialog = adw::Window::builder()
        .transient_for(&ctx.window)
        .modal(true)
        .title("Settings")
        .default_width(520)
        .default_height(560)
        .build();

    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    outer.append(&header);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.set_margin_top(24);
    content.set_margin_bottom(24);

    let section_title = gtk::Label::new(Some("Templates"));
    section_title.add_css_class("title-3");
    section_title.set_xalign(0.0);
    content.append(&section_title);

    let section_desc =
        gtk::Label::new(Some("Choose which templates appear in the template picker"));
    section_desc.add_css_class("dim-label");
    section_desc.set_xalign(0.0);
    section_desc.set_wrap(true);
    content.append(&section_desc);

    let disabled = ctx.state.borrow().disabled_templates.clone();

    // --- Built-in Templates ---
    let builtin_heading = gtk::Label::new(Some("Built-in Templates"));
    builtin_heading.add_css_class("heading");
    builtin_heading.set_xalign(0.0);
    builtin_heading.set_margin_top(4);
    content.append(&builtin_heading);

    let builtin_list = gtk::ListBox::new();
    builtin_list.set_selection_mode(gtk::SelectionMode::None);
    builtin_list.add_css_class("boxed-list");

    for (name, _body, tags_csv) in builtin_templates() {
        let row = adw::SwitchRow::builder()
            .title(&name)
            .subtitle(tags_csv.replace(',', ", "))
            .active(!disabled.contains(&name))
            .build();
        {
            let ctx = ctx.clone();
            let name = name.clone();
            row.connect_active_notify(move |r| {
                let active = r.is_active();
                {
                    let mut state = ctx.state.borrow_mut();
                    if active {
                        state.disabled_templates.retain(|n| n != &name);
                    } else if !state.disabled_templates.contains(&name) {
                        state.disabled_templates.push(name.clone());
                    }
                }
                trigger_vault_save(&ctx);
            });
        }
        builtin_list.append(&row);
    }
    content.append(&builtin_list);

    // --- Custom Templates ---
    let custom_heading = gtk::Label::new(Some("Custom Templates"));
    custom_heading.add_css_class("heading");
    custom_heading.set_xalign(0.0);
    custom_heading.set_margin_top(4);
    content.append(&custom_heading);

    let custom_templates = ctx.state.borrow().custom_templates.clone();

    if custom_templates.is_empty() {
        let empty_label = gtk::Label::new(Some("No custom templates yet"));
        empty_label.add_css_class("dim-label");
        empty_label.set_xalign(0.0);
        content.append(&empty_label);
    } else {
        let custom_list = gtk::ListBox::new();
        custom_list.set_selection_mode(gtk::SelectionMode::None);
        custom_list.add_css_class("boxed-list");

        for (i, (name, _body, tags_csv)) in custom_templates.iter().enumerate() {
            let row = adw::ActionRow::builder()
                .title(name)
                .subtitle(tags_csv.replace(',', ", "))
                .build();

            let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            btn_box.set_valign(gtk::Align::Center);

            let switch = gtk::Switch::new();
            switch.set_valign(gtk::Align::Center);
            switch.set_active(!disabled.contains(name));
            {
                let ctx = ctx.clone();
                let name = name.clone();
                switch.connect_state_set(move |_, active| {
                    {
                        let mut state = ctx.state.borrow_mut();
                        if active {
                            state.disabled_templates.retain(|n| n != &name);
                        } else if !state.disabled_templates.contains(&name) {
                            state.disabled_templates.push(name.clone());
                        }
                    }
                    trigger_vault_save(&ctx);
                    glib::Propagation::Proceed
                });
            }
            btn_box.append(&switch);

            let edit_btn = gtk::Button::from_icon_name("document-edit-symbolic");
            edit_btn.add_css_class("flat");
            edit_btn.set_tooltip_text(Some("Edit Template"));
            edit_btn.set_valign(gtk::Align::Center);
            {
                let ctx = ctx.clone();
                let dialog = dialog.clone();
                edit_btn.connect_clicked(move |_| {
                    dialog.close();
                    show_template_editor_dialog(&ctx, Some(i));
                });
            }
            btn_box.append(&edit_btn);

            let del_btn = gtk::Button::from_icon_name("user-trash-symbolic");
            del_btn.add_css_class("flat");
            del_btn.set_tooltip_text(Some("Delete Template"));
            del_btn.set_valign(gtk::Align::Center);
            {
                let ctx = ctx.clone();
                let dialog = dialog.clone();
                let name = name.clone();
                del_btn.connect_clicked(move |_| {
                    // Delete immediately; offer undo via toast (HIG: prefer undo over confirm)
                    let deleted = {
                        let mut state = ctx.state.borrow_mut();
                        let pos = state
                            .custom_templates
                            .iter()
                            .position(|(n, _, _)| n == &name);
                        if let Some(idx) = pos {
                            let removed = state.custom_templates.remove(idx);
                            state.disabled_templates.retain(|n| n != &name);
                            Some((idx, removed))
                        } else {
                            None
                        }
                    };
                    if let Some((idx, template)) = deleted {
                        trigger_vault_save(&ctx);
                        dialog.close();

                        // Show undo toast
                        let toast = adw::Toast::new(&format!("\u{201c}{name}\u{201d} deleted"));
                        toast.set_button_label(Some("Undo"));
                        toast.set_timeout(5);
                        let ctx_undo = ctx.clone();
                        toast.connect_button_clicked(move |_| {
                            {
                                let mut state = ctx_undo.state.borrow_mut();
                                let insert_at = idx.min(state.custom_templates.len());
                                state.custom_templates.insert(insert_at, template.clone());
                            }
                            trigger_vault_save(&ctx_undo);
                        });
                        ctx.toast_overlay.add_toast(toast);

                        show_settings_dialog(&ctx);
                    }
                });
            }
            btn_box.append(&del_btn);

            row.add_suffix(&btn_box);
            custom_list.append(&row);
        }
        content.append(&custom_list);
    }

    // --- New Custom Template button ---
    let new_btn = gtk::Button::with_label("New Custom Template");
    new_btn.add_css_class("pill");
    new_btn.set_margin_top(4);
    new_btn.set_halign(gtk::Align::Start);
    {
        let ctx = ctx.clone();
        let dialog = dialog.clone();
        new_btn.connect_clicked(move |_| {
            dialog.close();
            show_template_editor_dialog(&ctx, None);
        });
    }
    content.append(&new_btn);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .child(&content)
        .build();
    outer.append(&scroll);

    dialog.set_content(Some(&outer));
    dialog.present();
}

// ---------------------------------------------------------------------------
// Template editor dialog (create / edit custom template)
// ---------------------------------------------------------------------------

fn show_template_editor_dialog(ctx: &EditorCtx, edit_index: Option<usize>) {
    let (initial_name, initial_content, initial_tags) = match edit_index {
        Some(i) => {
            let state = ctx.state.borrow();
            if i < state.custom_templates.len() {
                let t = &state.custom_templates[i];
                (t.0.clone(), t.1.clone(), t.2.clone())
            } else {
                return;
            }
        }
        None => (String::new(), String::new(), String::new()),
    };

    let is_edit = edit_index.is_some();
    let dialog = adw::Window::builder()
        .transient_for(&ctx.window)
        .modal(true)
        .title(if is_edit {
            "Edit Template"
        } else {
            "New Template"
        })
        .default_width(480)
        .default_height(520)
        .build();

    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    outer.append(&header);

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
    vbox.set_margin_start(24);
    vbox.set_margin_end(24);
    vbox.set_margin_top(24);
    vbox.set_margin_bottom(24);

    let name_label = gtk::Label::new(Some("Template Name"));
    name_label.set_xalign(0.0);
    name_label.add_css_class("heading");
    vbox.append(&name_label);

    let name_entry = gtk::Entry::new();
    name_entry.set_text(&initial_name);
    name_entry.set_placeholder_text(Some("e.g. Sprint Retro"));
    vbox.append(&name_entry);

    let tags_label = gtk::Label::new(Some("Tags (comma-separated)"));
    tags_label.set_xalign(0.0);
    tags_label.add_css_class("heading");
    tags_label.set_margin_top(4);
    vbox.append(&tags_label);

    let tags_entry = gtk::Entry::new();
    tags_entry.set_text(&initial_tags);
    tags_entry.set_placeholder_text(Some("e.g. meeting,retro"));
    vbox.append(&tags_entry);

    let content_label = gtk::Label::new(Some("Content"));
    content_label.set_xalign(0.0);
    content_label.add_css_class("heading");
    content_label.set_margin_top(4);
    vbox.append(&content_label);

    let text_view = gtk::TextView::new();
    text_view.set_wrap_mode(gtk::WrapMode::Word);
    text_view.set_top_margin(8);
    text_view.set_bottom_margin(8);
    text_view.set_left_margin(8);
    text_view.set_right_margin(8);
    text_view.set_monospace(true);
    text_view.buffer().set_text(&initial_content);

    let text_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .min_content_height(250)
        .child(&text_view)
        .build();
    text_scroll.add_css_class("card");
    vbox.append(&text_scroll);

    let error_label = gtk::Label::new(None);
    error_label.add_css_class("error");
    error_label.set_visible(false);
    vbox.append(&error_label);

    let save_btn = gtk::Button::with_label("Save");
    save_btn.add_css_class("suggested-action");
    save_btn.add_css_class("pill");
    save_btn.set_margin_top(4);
    {
        let ctx = ctx.clone();
        let dialog = dialog.clone();
        let name_entry = name_entry.clone();
        let tags_entry = tags_entry.clone();
        let text_view = text_view.clone();
        let error_label = error_label.clone();
        save_btn.connect_clicked(move |_| {
            let name = name_entry.text().trim().to_string();
            if name.is_empty() {
                error_label.set_label("Template name cannot be empty");
                error_label.set_visible(true);
                return;
            }

            let tags = tags_entry.text().trim().to_string();
            let buf = text_view.buffer();
            let content_text = buf
                .text(&buf.start_iter(), &buf.end_iter(), false)
                .to_string();

            {
                let mut state = ctx.state.borrow_mut();
                let dup = state
                    .custom_templates
                    .iter()
                    .enumerate()
                    .any(|(j, (n, _, _))| n == &name && edit_index != Some(j));
                if dup {
                    drop(state);
                    error_label.set_label("A custom template with that name already exists");
                    error_label.set_visible(true);
                    return;
                }

                match edit_index {
                    Some(i) if i < state.custom_templates.len() => {
                        state.custom_templates[i] = (name, content_text, tags);
                    }
                    _ => {
                        state.custom_templates.push((name, content_text, tags));
                    }
                }
            }
            trigger_vault_save(&ctx);
            dialog.close();
            show_settings_dialog(&ctx);
        });
    }
    vbox.append(&save_btn);

    outer.append(&vbox);
    dialog.set_content(Some(&outer));
    dialog.present();
    name_entry.grab_focus();
}
