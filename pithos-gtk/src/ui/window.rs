use adw::prelude::*;
use sourceview5::prelude::*;
use sourceview5 as sourceview;
use std::{cell::Cell, cell::RefCell, rc::Rc};

use pithos_core::crypto;
use pithos_core::vault;
use pithos_core::state::*;
use crate::ui::types::*;
use crate::*; // For everything left in main.rs temporarily

pub fn build_ui(app: &adw::Application) {
    // Register the bundled app icon so GTK can find it
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let icon_search_paths: Vec<std::path::PathBuf> = [
        exe_dir.as_ref().map(|d| d.join("../data/icons")),
        exe_dir.as_ref().map(|d| d.join("../../data/icons")),
        Some(std::path::PathBuf::from("data/icons")),
    ]
    .into_iter()
    .flatten()
    .filter(|p| p.exists())
    .collect();

    if let Some(display) = gdk::Display::default() {
        let icon_theme = gtk::IconTheme::for_display(&display);
        for path in &icon_search_paths {
            icon_theme.add_search_path(path);
        }
    } else {
        eprintln!("No GTK display available while registering icon search paths");
    }

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Pithos Notebook")
        .default_width(1280)
        .default_height(860)
        .width_request(960)
        .height_request(400)
        .icon_name("com.pithos.notebook")
        .build();

    window.present();

    let config = vault::load_config();

    match config.vault_path {
        Some(ref path) if vault::vault_file_path(path).exists() => {
            let folder = path.clone();
            show_unlock_vault_dialog(&window, folder);
        }
        _ => {
            show_welcome_dialog(&window);
        }
    }
}

pub fn build_editor(
    window: &adw::ApplicationWindow,
    initial_state: DocState,
    vault_folder: String,
    cached_key: crypto::CachedKey,
) {
    // --- Build sidebar pane ---
    let (sidebar_toolbar_view, _sidebar_header, search_bar, search_entry, notes_list, tag_filter_box) =
        build_sidebar();

    // --- Build content pane ---
    let ContentPaneWidgets {
        toast_overlay,
        content_toolbar_view: _content_toolbar_view,
        content_header,
        doc_label,
        dirty_label,
        toolbar_scroll,
        tab_view,
        tab_bar,
        breadcrumbs,
        status_label: status,
        meta_label,
        status_bar: _status_bar,
        source_buffer,
        source_view,
        source_scroll,
        preview_webview,
        preview_scroll,
        split,
        tags_popover,
        tags_box,
        tag_entry,
        toolbar_widgets: tb,
        content_stack,
        find_bar,
        find_entry,
        replace_entry,
        replace_row,
        find_match_label,
        search_context,
        search_settings,
        vault_name_label,
    } = build_content_pane();

    // --- Assemble OverlaySplitView ---
    let split_view = adw::OverlaySplitView::new();
    split_view.set_sidebar(Some(&sidebar_toolbar_view));
    split_view.set_content(Some(&toast_overlay));
    split_view.set_show_sidebar(initial_state.sidebar_visible);
    split_view.set_min_sidebar_width(180.0);
    split_view.set_max_sidebar_width(400.0);

    // Adaptive: collapse sidebar into overlay before toolbar icons get clipped.
    // Toolbar needs ~960px, sidebar ~300px → collapse below ~1260px.
    match adw::BreakpointCondition::parse("max-width: 1260sp") {
        Ok(cond) => {
            let bp = adw::Breakpoint::new(cond);
            bp.add_setter(&split_view, "collapsed", Some(&true.to_value()));
            window.add_breakpoint(bp);
        }
        Err(err) => eprintln!("Failed to parse breakpoint condition: {err}"),
    }

    // Compute sidebar width fraction from saved pixel width.
    // Use the window's default width as the denominator to avoid exceeding available space.
    let default_width = window.default_width().max(800) as f64;
    let saved_width = (initial_state.sidebar_width as f64).clamp(150.0, default_width * 0.35);
    let fraction = (saved_width / default_width).clamp(0.15, 0.35);
    split_view.set_sidebar_width_fraction(fraction);

    // Apply theme from loaded state
    apply_theme(&initial_state.theme);
    apply_sourceview_theme(&source_view, is_dark_active());

    let state = Rc::new(RefCell::new(initial_state));

    let ctx = EditorCtx {
        window: window.clone(),
        notes_list: notes_list.clone(),
        search_entry: search_entry.clone(),
        source_buffer,
        source_view,
        source_panel: source_scroll,
        preview_webview,
        preview_panel: preview_scroll,
        split,
        status,
        doc_label,
        dirty_label,
        tags_box,
        tag_entry: tag_entry.clone(),
        state,
        vault_folder: Rc::new(RefCell::new(vault_folder)),
        cached_key: Rc::new(RefCell::new(Some(cached_key))),
        save_timeout_id: Rc::new(Cell::new(None)),
        save_generation: Rc::new(Cell::new(0)),
        saving: Rc::new(Cell::new(false)),
        close_requested: Rc::new(Cell::new(false)),
        split_view: split_view.clone(),
        toast_overlay,
        toolbar: toolbar_scroll.clone(),
        tab_view: tab_view.clone(),
        tab_bar,
        tags_popover,
        breadcrumbs,
        tag_filter_box,
        meta_label,
        search_bar,
        content_stack,
        sync_timeout_id: Rc::new(Cell::new(None)),
        search_timeout_id: Rc::new(Cell::new(None)),
        content_header,
        find_bar,
        find_entry,
        replace_entry,
        replace_row,
        find_match_label,
        search_context,
        search_settings,
        vault_name_label,
        last_save_completed: Rc::new(Cell::new(std::time::Instant::now())),
    };

    initialize_state(&ctx);

    let purged = purge_old_trash(&mut ctx.state.borrow_mut());

    wire_menu_actions(&ctx);
    wire_toolbar_signals(&ctx, &tb);
    wire_sidebar_signals(&ctx, &search_entry, &notes_list);
    wire_editor_signals(&ctx, &tag_entry);
    wire_keyboard_shortcuts(&ctx, window);
    wire_find_replace_signals(&ctx);

    // Fullscreen autohide: reveal header/toolbar when mouse is near the top edge
    {
        let ctx = ctx.clone();
        let motion = gtk::EventControllerMotion::new();
        motion.connect_motion(move |_, _, y| {
            if ctx.state.borrow().fullscreen {
                if y < 10.0 {
                    ctx.content_header.set_visible(true);
                    ctx.toolbar.set_visible(true);
                } else if y > 100.0 {
                    ctx.content_header.set_visible(false);
                    ctx.toolbar.set_visible(false);
                }
            }
        });
        split_view.add_controller(motion);
    }

    wire_close_request(&ctx);
    setup_auto_save(&ctx);
    watch_vault_file(&ctx);

    // Wire AdwTabView signals
    {
        let ctx2 = ctx.clone();
        tab_view.connect_selected_page_notify(move |tv| {
            if ctx2.state.borrow().suppress_sync {
                return;
            }
            if let Some(page) = tv.selected_page() {
                let child = page.child();
                if let Some(label) = child.downcast_ref::<gtk::Label>() {
                    let note_id = label.widget_name();
                    if !note_id.is_empty() {
                        let current = ctx2.state.borrow().active_note_id.clone();
                        if note_id.as_str() != current {
                            switch_to_note(&ctx2, &note_id);
                        }
                    }
                }
            }
        });
    }
    {
        let ctx2 = ctx.clone();
        tab_view.connect_close_page(move |tv, page| {
            // During programmatic close (suppress_sync=true), confirm immediately.
            if ctx2.state.borrow().suppress_sync {
                tv.close_page_finish(page, true);
                return glib::Propagation::Stop;
            }

            // User-initiated close (clicking × on tab) or close_tab (Ctrl+W).
            let note_id = page
                .child()
                .downcast_ref::<gtk::Label>()
                .map(|l| l.widget_name().to_string())
                .filter(|s| !s.is_empty());

            if let Some(note_id) = note_id {
                let needs_switch = {
                    let mut state = ctx2.state.borrow_mut();
                    if state.open_tabs.len() <= 1 {
                        tv.close_page_finish(page, false);
                        return glib::Propagation::Stop;
                    }
                    state.open_tabs.retain(|id| id != &note_id);
                    if state.active_note_id == note_id {
                        state.open_tabs.first().cloned()
                    } else {
                        None
                    }
                };

                // Suppress signals BEFORE close_page_finish — its internal
                // set_selected_page emits selected-page-notify which would
                // re-enter switch_to_note and cause a crash.
                ctx2.state.borrow_mut().suppress_sync = true;
                tv.close_page_finish(page, true);
                ctx2.state.borrow_mut().suppress_sync = false;

                if let Some(switch_id) = needs_switch {
                    switch_to_note(&ctx2, &switch_id);
                } else {
                    refresh_tabs(&ctx2);
                    refresh_note_list(&ctx2);
                }
            } else {
                ctx2.state.borrow_mut().suppress_sync = true;
                tv.close_page_finish(page, true);
                ctx2.state.borrow_mut().suppress_sync = false;
            }

            glib::Propagation::Stop
        });
    }
    {
        let ctx2 = ctx.clone();
        tab_view.connect_page_reordered(move |tv, _page, _position| {
            if ctx2.state.borrow().suppress_sync {
                return;
            }
            // Sync open_tabs order from TabView order
            let mut new_order = Vec::new();
            for i in 0..tv.n_pages() {
                let page = tv.nth_page(i);
                let child = page.child();
                if let Some(label) = child.downcast_ref::<gtk::Label>() {
                    let note_id = label.widget_name();
                    if !note_id.is_empty() {
                        new_order.push(note_id.to_string());
                    }
                }
            }
            ctx2.state.borrow_mut().open_tabs = new_order;
            trigger_vault_save(&ctx2);
        });
    }

    if purged > 0 {
        send_toast(&ctx, &format!("Purged {purged} old item(s) from trash"));
        trigger_vault_save(&ctx);
    }

    window.set_content(Some(&split_view));
}

pub fn build_content_pane() -> ContentPaneWidgets {
    let content_toolbar_view = adw::ToolbarView::new();

    // --- Content header bar ---
    let content_header = adw::HeaderBar::new();

    // Start: sidebar toggle
    let content_sidebar_toggle = gtk::Button::from_icon_name("sidebar-show-symbolic");
    content_sidebar_toggle.set_tooltip_text(Some("Toggle Sidebar (Ctrl+\\)"));
    content_sidebar_toggle.add_css_class("flat");
    content_sidebar_toggle.set_action_name(Some("win.toggle-sidebar"));
    set_accessible_label(&content_sidebar_toggle, "Toggle Sidebar");
    content_header.pack_start(&content_sidebar_toggle);

    // Vault name label (shown in header bar after sidebar toggle)
    let vault_name_label = gtk::Label::new(None);
    vault_name_label.add_css_class("dim-label");
    content_header.pack_start(&vault_name_label);

    // Title widget: doc name + dirty indicator
    let title_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    title_box.set_halign(gtk::Align::Center);
    let doc_label = gtk::Label::new(None);
    doc_label.add_css_class("title-4");
    doc_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    let dirty_label = gtk::Label::new(Some("\u{25cf}"));
    dirty_label.add_css_class("dirty-indicator");
    dirty_label.set_visible(false);
    title_box.append(&dirty_label);
    title_box.append(&doc_label);
    content_header.set_title_widget(Some(&title_box));

    // End: theme toggle + primary menu
    let theme_toggle = gtk::Button::from_icon_name("weather-clear-night-symbolic");
    theme_toggle.set_tooltip_text(Some("Toggle Theme (Ctrl+Shift+D)"));
    theme_toggle.add_css_class("flat");
    theme_toggle.set_action_name(Some("win.toggle-theme"));
    set_accessible_label(&theme_toggle, "Toggle Theme");
    content_header.pack_end(&theme_toggle);

    let content_menu_btn = gtk::MenuButton::new();
    content_menu_btn.set_icon_name("open-menu-symbolic");
    content_menu_btn.set_tooltip_text(Some("Main Menu"));
    content_menu_btn.add_css_class("flat");
    set_accessible_label(&content_menu_btn, "Main Menu");
    content_menu_btn.set_menu_model(Some(&build_content_menu()));
    content_header.pack_end(&content_menu_btn);

    content_toolbar_view.add_top_bar(&content_header);

    // --- Formatting toolbar (scrollable so buttons are never clipped) ---
    let (toolbar, tb) = build_toolbar();
    let toolbar_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .child(&toolbar)
        .build();
    content_toolbar_view.add_top_bar(&toolbar_scroll);

    // --- Content body ---
    let content_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    // Breadcrumbs label (folder path)
    let breadcrumbs = gtk::Label::new(None);
    breadcrumbs.add_css_class("dim-label");
    breadcrumbs.add_css_class("caption");
    breadcrumbs.set_xalign(0.0);
    breadcrumbs.set_margin_start(12);
    breadcrumbs.set_margin_top(4);
    breadcrumbs.set_margin_bottom(0);
    breadcrumbs.set_ellipsize(gtk::pango::EllipsizeMode::End);
    content_box.append(&breadcrumbs);

    // Native tabs via AdwTabView + AdwTabBar
    let tab_view = adw::TabView::new();
    let tab_bar = adw::TabBar::new();
    tab_bar.set_view(Some(&tab_view));
    tab_bar.set_autohide(false);
    content_box.append(&tab_bar);

    // --- Find/replace bar (hidden by default) ---
    let find_bar = gtk::Box::new(gtk::Orientation::Vertical, 4);
    find_bar.set_margin_start(8);
    find_bar.set_margin_end(8);
    find_bar.set_margin_top(4);
    find_bar.set_margin_bottom(4);
    find_bar.set_visible(false);

    let find_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let find_entry = gtk::SearchEntry::new();
    find_entry.set_hexpand(true);
    find_entry.set_placeholder_text(Some("Find\u{2026}"));
    let find_match_label = gtk::Label::new(None);
    find_match_label.add_css_class("dim-label");
    find_match_label.add_css_class("caption");
    let find_prev_btn = gtk::Button::from_icon_name("go-up-symbolic");
    find_prev_btn.add_css_class("flat");
    find_prev_btn.set_tooltip_text(Some("Previous Match (Shift+Enter)"));
    find_prev_btn.set_action_name(Some("win.find-prev"));
    let find_next_btn = gtk::Button::from_icon_name("go-down-symbolic");
    find_next_btn.add_css_class("flat");
    find_next_btn.set_tooltip_text(Some("Next Match (Enter)"));
    find_next_btn.set_action_name(Some("win.find-next"));
    let find_close_btn = gtk::Button::from_icon_name("window-close-symbolic");
    find_close_btn.add_css_class("flat");
    find_close_btn.set_tooltip_text(Some("Close Find (Escape)"));
    find_close_btn.set_action_name(Some("win.hide-find"));
    let case_toggle = gtk::ToggleButton::new();
    case_toggle.set_label("Aa");
    case_toggle.add_css_class("flat");
    case_toggle.set_tooltip_text(Some("Case Sensitive"));
    let regex_toggle = gtk::ToggleButton::new();
    regex_toggle.set_label(".*");
    regex_toggle.add_css_class("flat");
    regex_toggle.set_tooltip_text(Some("Regular Expression"));
    find_row.append(&find_entry);
    find_row.append(&find_match_label);
    find_row.append(&case_toggle);
    find_row.append(&regex_toggle);
    find_row.append(&find_prev_btn);
    find_row.append(&find_next_btn);
    find_row.append(&find_close_btn);
    find_bar.append(&find_row);

    let replace_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    replace_row.set_visible(false);
    let replace_entry = gtk::Entry::new();
    replace_entry.set_hexpand(true);
    replace_entry.set_placeholder_text(Some("Replace\u{2026}"));
    let replace_btn = gtk::Button::with_label("Replace");
    replace_btn.add_css_class("flat");
    replace_btn.set_action_name(Some("win.replace-one"));
    let replace_all_btn = gtk::Button::with_label("Replace All");
    replace_all_btn.add_css_class("flat");
    replace_all_btn.set_action_name(Some("win.replace-all"));
    replace_row.append(&replace_entry);
    replace_row.append(&replace_btn);
    replace_row.append(&replace_all_btn);
    find_bar.append(&replace_row);

    content_box.append(&find_bar);

    // Editor panes: source editor (left) + WebView preview (right)
    let (source_buffer, source_view, source_scroll) = build_source_editor();
    let (preview_webview, preview_scroll) = build_preview_pane();

    // Search context for find/replace (attached to source buffer)
    let search_settings = sourceview::SearchSettings::builder()
        .wrap_around(true)
        .build();
    let search_context = sourceview::SearchContext::builder()
        .buffer(&source_buffer)
        .settings(&search_settings)
        .highlight(true)
        .build();

    // Wire case-sensitivity toggle
    {
        let settings = search_settings.clone();
        case_toggle.connect_toggled(move |btn| {
            settings.set_case_sensitive(btn.is_active());
        });
    }
    // Wire regex toggle
    {
        let settings = search_settings.clone();
        regex_toggle.connect_toggled(move |btn| {
            settings.set_regex_enabled(btn.is_active());
        });
    }

    let split = gtk::Paned::new(gtk::Orientation::Horizontal);
    split.set_wide_handle(true);
    split.set_position(600);
    split.set_start_child(Some(&source_scroll));
    split.set_end_child(Some(&preview_scroll));
    split.set_shrink_start_child(true);
    split.set_shrink_end_child(true);
    split.set_vexpand(true);
    content_box.append(&split);

    content_toolbar_view.set_content(Some(&content_box));

    // --- Tags popover (accessible from status bar) ---
    let (tags_popover, tags_box, tag_entry) = build_tags_popover();

    // --- Bottom bar: ActionBar with word count + tags button + metadata ---
    let status_bar = gtk::ActionBar::new();
    let status_label = gtk::Label::new(None);
    status_label.add_css_class("dim-label");
    status_label.add_css_class("caption");
    status_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    status_label.set_hexpand(true);
    status_label.set_xalign(0.0);
    status_bar.pack_start(&status_label);

    let tags_btn = gtk::MenuButton::new();
    tags_btn.set_icon_name("tag-symbolic");
    tags_btn.set_tooltip_text(Some("Tags"));
    tags_btn.add_css_class("flat");
    tags_btn.set_popover(Some(&tags_popover));
    set_accessible_label(&tags_btn, "Tags");
    status_bar.pack_end(&tags_btn);

    let meta_label = gtk::Label::new(None);
    meta_label.add_css_class("dim-label");
    meta_label.add_css_class("caption");
    meta_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    meta_label.set_xalign(1.0);
    status_bar.pack_end(&meta_label);

    content_toolbar_view.add_bottom_bar(&status_bar);

    // Empty state
    let empty_page = adw::StatusPage::builder()
        .icon_name("document-new-symbolic")
        .title("No Notes")
        .description("Create a new note to get started")
        .build();
    let empty_btn = gtk::Button::with_label("New Note");
    empty_btn.add_css_class("suggested-action");
    empty_btn.add_css_class("pill");
    empty_btn.set_halign(gtk::Align::Center);
    empty_btn.set_action_name(Some("win.new-note"));
    empty_page.set_child(Some(&empty_btn));

    // Stack to switch between editor and empty state
    let content_stack = gtk::Stack::new();
    content_stack.add_named(&content_toolbar_view, Some("editor"));
    content_stack.add_named(&empty_page, Some("empty"));
    content_stack.set_visible_child_name("editor");

    // Wrap in ToastOverlay
    let toast_overlay = adw::ToastOverlay::new();
    toast_overlay.set_child(Some(&content_stack));

    ContentPaneWidgets {
        toast_overlay,
        content_toolbar_view,
        content_header,
        doc_label,
        dirty_label,
        toolbar_scroll,
        tab_view,
        tab_bar,
        breadcrumbs,
        status_label,
        meta_label,
        status_bar,
        source_buffer,
        source_view,
        source_scroll,
        preview_webview,
        preview_scroll,
        split,
        tags_popover,
        tags_box,
        tag_entry,
        toolbar_widgets: tb,
        content_stack,
        find_bar,
        find_entry,
        replace_entry,
        replace_row,
        find_match_label,
        search_context,
        search_settings,
        vault_name_label,
    }
}

pub fn build_source_editor() -> (sourceview::Buffer, sourceview::View, gtk::ScrolledWindow) {
    let buffer = sourceview::Buffer::new(None);
    buffer.set_highlight_syntax(true);
    buffer.set_highlight_matching_brackets(true);

    let language_manager = sourceview::LanguageManager::new();
    if let Some(language) = language_manager.language("markdown") {
        buffer.set_language(Some(&language));
    }

    let view = sourceview::View::with_buffer(&buffer);
    view.set_editable(true);
    view.set_cursor_visible(true);
    view.set_monospace(true);
    view.set_show_line_numbers(true);
    view.set_tab_width(4);
    view.set_indent_width(4);
    view.set_auto_indent(true);
    view.set_wrap_mode(gtk::WrapMode::WordChar);
    view.set_left_margin(8);
    view.set_right_margin(8);
    view.set_top_margin(8);
    view.set_bottom_margin(8);
    view.set_vexpand(true);
    view.set_hexpand(true);
    view.add_css_class("source-pane");

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(&view)
        .build();

    (buffer, view, scroll)
}

pub fn build_preview_pane() -> (webkit6::WebView, gtk::Box) {
    use webkit6::prelude::*;

    // Use an ephemeral network session so no cookies, cache, or local storage
    // persist to disk — the preview is a disposable render surface.
    let ephemeral_session = webkit6::NetworkSession::new_ephemeral();
    let webview = webkit6::WebView::builder()
        .network_session(&ephemeral_session)
        .build();

    // Security: disable features we don't need
    let settings = webkit6::prelude::WebViewExt::settings(&webview);
    if let Some(settings) = settings {
        settings.set_enable_javascript(true); // needed for mermaid.js
        settings.set_enable_developer_extras(false);
        settings.set_allow_file_access_from_file_urls(false);
        settings.set_allow_universal_access_from_file_urls(false);
    }

    // Block user-initiated navigation (link clicks) but allow programmatic loads
    // (load_html uses NavigationType::Other which must be permitted).
    webview.connect_decide_policy(|_webview, decision, decision_type| {
        if decision_type == webkit6::PolicyDecisionType::NavigationAction {
            if let Some(nav_decision) = decision.downcast_ref::<webkit6::NavigationPolicyDecision>() {
                let nav_action = nav_decision.navigation_action();
                if let Some(mut action) = nav_action {
                    if action.navigation_type() != webkit6::NavigationType::Other {
                        decision.ignore();
                        return true;
                    }
                }
            }
        }
        false
    });

    webview.set_vexpand(true);
    webview.set_hexpand(true);

    // WebView handles its own scrolling internally, so we just wrap
    // it in a simple Box for layout purposes (no ScrolledWindow needed).
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    container.set_hexpand(true);
    container.set_vexpand(true);
    container.append(&webview);

    (webview, container)
}

pub fn build_tags_popover() -> (gtk::Popover, gtk::FlowBox, gtk::Entry) {
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);

    let label = gtk::Label::new(Some("Tags"));
    label.add_css_class("title-4");
    label.set_halign(gtk::Align::Start);
    vbox.append(&label);

    let flow = gtk::FlowBox::new();
    flow.set_selection_mode(gtk::SelectionMode::None);
    flow.set_row_spacing(4);
    flow.set_column_spacing(4);
    flow.set_max_children_per_line(4);
    flow.set_hexpand(true);
    vbox.append(&flow);

    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some("Add tag and press Enter\u{2026}"));
    entry.set_width_chars(24);
    vbox.append(&entry);

    let popover = gtk::Popover::new();
    popover.set_child(Some(&vbox));

    (popover, flow, entry)
}
