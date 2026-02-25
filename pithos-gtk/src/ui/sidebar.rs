use adw::prelude::*;

use crate::*; // For everything left in main.rs temporarily

pub fn build_sidebar() -> (adw::ToolbarView, adw::HeaderBar, gtk::SearchBar, gtk::SearchEntry, gtk::ListBox, gtk::Box) {
    let toolbar_view = adw::ToolbarView::new();

    // --- Sidebar header bar ---
    let header = adw::HeaderBar::new();
    let title_label = gtk::Label::new(Some("Notes"));
    title_label.add_css_class("title-4");
    header.set_title_widget(Some(&title_label));

    // Start: sidebar toggle
    let sidebar_toggle = gtk::Button::from_icon_name("sidebar-show-symbolic");
    sidebar_toggle.set_tooltip_text(Some("Toggle Sidebar (Ctrl+\\)"));
    sidebar_toggle.add_css_class("flat");
    sidebar_toggle.set_action_name(Some("win.toggle-sidebar"));
    set_accessible_label(&sidebar_toggle, "Toggle Sidebar");
    header.pack_start(&sidebar_toggle);

    // End: search toggle, new note, menu
    let search_toggle = gtk::ToggleButton::new();
    search_toggle.set_icon_name("edit-find-symbolic");
    search_toggle.set_tooltip_text(Some("Search Notes (Ctrl+Shift+F)"));
    search_toggle.add_css_class("flat");
    set_accessible_label(&search_toggle, "Search Notes");
    header.pack_end(&search_toggle);

    let new_note_btn = gtk::Button::from_icon_name("document-new-symbolic");
    new_note_btn.set_tooltip_text(Some("New Note (Ctrl+N)"));
    new_note_btn.add_css_class("flat");
    new_note_btn.set_action_name(Some("win.new-note"));
    set_accessible_label(&new_note_btn, "New Note");
    header.pack_end(&new_note_btn);

    let sidebar_menu_btn = gtk::MenuButton::new();
    sidebar_menu_btn.set_icon_name("open-menu-symbolic");
    sidebar_menu_btn.set_tooltip_text(Some("Sidebar Menu"));
    sidebar_menu_btn.add_css_class("flat");
    sidebar_menu_btn.set_menu_model(Some(&build_sidebar_menu()));
    set_accessible_label(&sidebar_menu_btn, "Sidebar Menu");
    header.pack_end(&sidebar_menu_btn);

    toolbar_view.add_top_bar(&header);

    // --- Search bar (toggleable) ---
    let search_entry = gtk::SearchEntry::new();
    search_entry.set_placeholder_text(Some("Search notes\u{2026}"));
    search_entry.set_hexpand(true);

    let search_bar = gtk::SearchBar::builder()
        .child(&search_entry)
        .search_mode_enabled(false)
        .show_close_button(true)
        .build();
    search_bar.connect_entry(&search_entry);

    // Bind toggle button to search bar
    search_toggle
        .bind_property("active", &search_bar, "search-mode-enabled")
        .bidirectional()
        .sync_create()
        .build();

    toolbar_view.add_top_bar(&search_bar);

    // --- Tag filter area (hidden by default) ---
    let tag_filter_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    tag_filter_box.set_margin_start(8);
    tag_filter_box.set_margin_end(8);
    tag_filter_box.set_margin_top(4);
    tag_filter_box.set_margin_bottom(4);
    tag_filter_box.set_visible(false);
    toolbar_view.add_top_bar(&tag_filter_box);

    // --- Notes list ---
    let notes_list = gtk::ListBox::new();
    notes_list.set_selection_mode(gtk::SelectionMode::Single);
    notes_list.set_activate_on_single_click(true);
    notes_list.add_css_class("navigation-sidebar");

    let notes_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(&notes_list)
        .build();

    toolbar_view.set_content(Some(&notes_scroll));

    (toolbar_view, header, search_bar, search_entry, notes_list, tag_filter_box)
}

pub fn build_sidebar_menu() -> gtk::gio::Menu {
    let menu = gtk::gio::Menu::new();

    // Sort submenu
    let sort_menu = gtk::gio::Menu::new();
    sort_menu.append(Some("Manual"), Some("win.sort-order::manual"));
    sort_menu.append(Some("Modified \u{2193}"), Some("win.sort-order::modified-desc"));
    sort_menu.append(Some("Modified \u{2191}"), Some("win.sort-order::modified-asc"));
    sort_menu.append(Some("Name A\u{2192}Z"), Some("win.sort-order::name-asc"));
    sort_menu.append(Some("Name Z\u{2192}A"), Some("win.sort-order::name-desc"));
    sort_menu.append(Some("Created \u{2193}"), Some("win.sort-order::created-desc"));
    sort_menu.append(Some("Created \u{2191}"), Some("win.sort-order::created-asc"));
    menu.append_submenu(Some("Sort by\u{2026}"), &sort_menu);

    let section1 = gtk::gio::Menu::new();
    section1.append(Some("New Folder"), Some("win.new-folder"));
    section1.append(Some("New from Template\u{2026}"), Some("win.new-from-template"));
    section1.append(Some("Daily Note"), Some("win.daily-note"));
    section1.append(Some("View Trash"), Some("win.view-trash"));
    menu.append_section(None, &section1);

    let section2 = gtk::gio::Menu::new();
    section2.append(Some("Toggle Theme"), Some("win.toggle-theme"));
    menu.append_section(None, &section2);

    menu
}
