use adw::prelude::*;
use sourceview5::prelude::*;
use sourceview5 as sourceview;
use pithos_core::state::*;
use crate::*;

// ---------------------------------------------------------------------------
// Shared utility functions used by multiple modules
// ---------------------------------------------------------------------------

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
// Widget helpers
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

// ---------------------------------------------------------------------------
// Dialogs
// ---------------------------------------------------------------------------

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
// Window state
// ---------------------------------------------------------------------------

pub fn toggle_fullscreen(ctx: &EditorCtx) {
    let fullscreen = {
        let mut state = ctx.state.borrow_mut();
        state.fullscreen = !state.fullscreen;
        state.fullscreen
    };

    if fullscreen {
        ctx.window.fullscreen();
        // Auto-hide header + toolbar in fullscreen
        ctx.content_header.set_visible(false);
        ctx.toolbar.set_visible(false);
    } else {
        ctx.window.unfullscreen();
        ctx.content_header.set_visible(true);
        ctx.toolbar.set_visible(true);
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

// ---------------------------------------------------------------------------
// Toast / status
// ---------------------------------------------------------------------------

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
