use adw::prelude::*;

pub fn show_shortcuts_window(parent: &adw::ApplicationWindow) {
    const SHORTCUTS_UI: &str = r#"
<?xml version="1.0" encoding="UTF-8"?>
<interface>
  <object class="GtkShortcutsWindow" id="shortcuts_window">
    <property name="modal">true</property>
    <child>
      <object class="GtkShortcutsSection">
        <property name="section-name">shortcuts</property>
        <child>
          <object class="GtkShortcutsGroup">
            <property name="title">General</property>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;n</property><property name="title">New note</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;s</property><property name="title">Save</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;w</property><property name="title">Close tab</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;backslash</property><property name="title">Toggle sidebar</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;&lt;Shift&gt;f</property><property name="title">Search notes</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;&lt;Shift&gt;p</property><property name="title">Command palette</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;&lt;Shift&gt;j</property><property name="title">Zen mode</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;&lt;Shift&gt;t</property><property name="title">Daily note</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;&lt;Shift&gt;d</property><property name="title">Toggle theme</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">F1</property><property name="title">Help</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">F2</property><property name="title">Rename note</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">F11</property><property name="title">Fullscreen</property></object></child>
          </object>
        </child>
        <child>
          <object class="GtkShortcutsGroup">
            <property name="title">Formatting</property>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;b</property><property name="title">Bold</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;i</property><property name="title">Italic</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;u</property><property name="title">Underline</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;d</property><property name="title">Strikethrough</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;e</property><property name="title">Inline code</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;k</property><property name="title">Insert link</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;1</property><property name="title">Heading 1</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;2</property><property name="title">Heading 2</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;3</property><property name="title">Heading 3</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;&lt;Shift&gt;q</property><property name="title">Block quote</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;&lt;Shift&gt;l</property><property name="title">Bullet list</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;space</property><property name="title">Toggle checkbox</property></object></child>
          </object>
        </child>
        <child>
          <object class="GtkShortcutsGroup">
            <property name="title">Editing</property>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;z</property><property name="title">Undo</property></object></child>
            <child><object class="GtkShortcutsShortcut"><property name="accelerator">&lt;Ctrl&gt;&lt;Shift&gt;z</property><property name="title">Redo</property></object></child>
          </object>
        </child>
      </object>
    </child>
  </object>
</interface>
"#;

    let builder = gtk::Builder::from_string(SHORTCUTS_UI);
    let window: gtk::ShortcutsWindow = builder.object("shortcuts_window").unwrap();
    window.set_transient_for(Some(parent));
    window.present();
}

pub fn show_about_dialog(parent: &adw::ApplicationWindow) {
    let dialog = adw::AboutDialog::new();
    dialog.set_application_name("Pithos Notebook");
    dialog.set_version("0.1.0");
    dialog.set_developer_name("iamcarrasco");
    dialog.set_license_type(gtk::License::Gpl30);
    dialog.set_comments("A private, offline, encrypted Markdown notebook.");
    dialog.set_application_icon("com.pithos.notebook");
    dialog.set_website("https://github.com/iamcarrasco/Pithos-Notebook");
    dialog.set_developers(&["iamcarrasco"]);
    dialog.present(Some(parent));
}

pub fn show_help_dialog(parent: &adw::ApplicationWindow) {
    let nav_view = adw::NavigationView::new();

    // ── Main page ──────────────────────────────────────────────
    let main_page = build_help_main_page(&nav_view);
    nav_view.add(&main_page);

    let toolbar_view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&nav_view));

    let dialog = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("Help")
        .default_width(500)
        .default_height(560)
        .build();
    dialog.set_content(Some(&toolbar_view));
    dialog.present();
}

// ── Main help page ────────────────────────────────────────────
fn build_help_main_page(
    nav_view: &adw::NavigationView,
) -> adw::NavigationPage {
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 16);
    vbox.set_margin_start(16);
    vbox.set_margin_end(16);
    vbox.set_margin_top(8);
    vbox.set_margin_bottom(16);

    // Brief intro
    let intro = gtk::Label::new(Some(
        "Pithos Notebook is a private, offline, encrypted Markdown notebook. \
         All your notes are stored in a single AES-256 encrypted vault file."
    ));
    intro.set_wrap(true);
    intro.set_xalign(0.0);
    intro.add_css_class("dim-label");
    vbox.append(&intro);

    // Getting Started section
    let section_getting_started = help_section("Getting Started");
    let list_gs = gtk::ListBox::new();
    list_gs.add_css_class("boxed-list");
    list_gs.set_selection_mode(gtk::SelectionMode::None);

    append_nav_row(&list_gs, "Vault &amp; Encryption", "security-high-symbolic", nav_view,
        build_vault_page);
    append_nav_row(&list_gs, "Creating Notes", "document-new-symbolic", nav_view,
        build_notes_page);

    vbox.append(&section_getting_started);
    vbox.append(&list_gs);

    // Editor section
    let section_editor = help_section("Editor");
    let list_ed = gtk::ListBox::new();
    list_ed.add_css_class("boxed-list");
    list_ed.set_selection_mode(gtk::SelectionMode::None);

    append_nav_row(&list_ed, "Formatting", "format-text-bold-symbolic", nav_view,
        build_formatting_page);
    append_nav_row(&list_ed, "Code Blocks", "utilities-terminal-symbolic", nav_view,
        build_code_blocks_page);
    append_nav_row(&list_ed, "Tables", "view-grid-symbolic", nav_view,
        build_tables_page);

    vbox.append(&section_editor);
    vbox.append(&list_ed);

    // Organization section
    let section_org = help_section("Organization");
    let list_org = gtk::ListBox::new();
    list_org.add_css_class("boxed-list");
    list_org.set_selection_mode(gtk::SelectionMode::None);

    append_nav_row(&list_org, "Folders &amp; Tags", "folder-symbolic", nav_view,
        build_folders_tags_page);
    append_nav_row(&list_org, "Search &amp; Daily Notes", "edit-find-symbolic", nav_view,
        build_search_page);

    vbox.append(&section_org);
    vbox.append(&list_org);

    // Advanced section
    let section_adv = help_section("Advanced");
    let list_adv = gtk::ListBox::new();
    list_adv.add_css_class("boxed-list");
    list_adv.set_selection_mode(gtk::SelectionMode::None);

    append_nav_row(&list_adv, "Snapshots &amp; History", "document-open-recent-symbolic", nav_view,
        build_snapshots_page);
    append_nav_row(&list_adv, "Import &amp; Export", "document-save-as-symbolic", nav_view,
        build_import_export_page);
    append_nav_row(&list_adv, "Backlinks", "emblem-symbolic-link-symbolic", nav_view,
        build_backlinks_page);

    vbox.append(&section_adv);
    vbox.append(&list_adv);

    // Keyboard Shortcuts row (opens the existing shortcuts dialog)
    let section_kbd = help_section("Reference");
    let list_kbd = gtk::ListBox::new();
    list_kbd.add_css_class("boxed-list");
    list_kbd.set_selection_mode(gtk::SelectionMode::None);

    append_nav_row(&list_kbd, "Keyboard Shortcuts", "preferences-desktop-keyboard-symbolic", nav_view,
        build_shortcuts_page);

    vbox.append(&section_kbd);
    vbox.append(&list_kbd);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&vbox)
        .build();

    adw::NavigationPage::builder()
        .title("Help")
        .child(&scroll)
        .build()
}

// ── Helpers ───────────────────────────────────────────────────

fn help_section(title: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(title));
    label.add_css_class("title-4");
    label.set_halign(gtk::Align::Start);
    label
}

fn append_nav_row(
    list: &gtk::ListBox,
    title: &str,
    icon: &str,
    nav_view: &adw::NavigationView,
    page_builder: fn() -> adw::NavigationPage,
) {
    let row = adw::ActionRow::builder()
        .title(title)
        .activatable(true)
        .build();
    row.add_prefix(&gtk::Image::from_icon_name(icon));
    row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
    {
        let nav_view = nav_view.clone();
        row.connect_activated(move |_| {
            let page = page_builder();
            nav_view.push(&page);
        });
    }
    list.append(&row);
}

fn help_page(title: &str, content: &gtk::Box) -> adw::NavigationPage {
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(content)
        .build();

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&adw::HeaderBar::new());
    toolbar_view.set_content(Some(&scroll));

    adw::NavigationPage::builder()
        .title(title)
        .child(&toolbar_view)
        .build()
}

fn help_paragraph(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_wrap(true);
    label.set_xalign(0.0);
    label
}

fn help_tip(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_wrap(true);
    label.set_xalign(0.0);
    label.add_css_class("dim-label");
    label.add_css_class("caption");
    label
}

fn help_content() -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 12);
    b.set_margin_start(16);
    b.set_margin_end(16);
    b.set_margin_top(8);
    b.set_margin_bottom(16);
    b
}

fn shortcut_list(items: &[(&str, &str)]) -> gtk::ListBox {
    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::None);
    for (accel, desc) in items {
        let row = adw::ActionRow::builder().title(*desc).build();
        let lbl = gtk::Label::new(Some(accel));
        lbl.add_css_class("dim-label");
        lbl.add_css_class("caption");
        row.add_suffix(&lbl);
        list.append(&row);
    }
    list
}

// ── Sub-pages ─────────────────────────────────────────────────

fn build_vault_page() -> adw::NavigationPage {
    let c = help_content();

    c.append(&help_paragraph(
        "Pithos Notebook stores all your notes in a single encrypted vault file. \
         When you first launch the app, you\u{2019}ll be asked to choose a folder \
         and set a password."
    ));

    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::None);
    for desc in [
        "Your password is used for AES-256 encryption via PBKDF2 key derivation",
        "The vault file is saved to your chosen folder as an encrypted .mdvault file",
        "Nobody can read your notes without the password \u{2014} not even with direct file access",
    ] {
        list.append(&adw::ActionRow::builder().title(desc).build());
    }
    c.append(&list);

    c.append(&help_section("Saving"));
    c.append(&help_paragraph(
        "Changes are auto-saved periodically. You can also save manually."
    ));
    c.append(&shortcut_list(&[
        ("Ctrl+S", "Save vault"),
        ("Ctrl+Shift+S", "Save As (export vault to new location)"),
    ]));

    help_page("Vault &amp; Encryption", &c)
}

fn build_notes_page() -> adw::NavigationPage {
    let c = help_content();

    c.append(&help_paragraph(
        "Notes are listed in the sidebar. Click any note to open it in the editor."
    ));

    c.append(&help_section("Creating & Managing"));
    c.append(&shortcut_list(&[
        ("Ctrl+N", "Create a new note"),
        ("F2", "Rename the active note"),
        ("Ctrl+W", "Close the current tab"),
        ("Ctrl+Shift+T", "Create or open today\u{2019}s daily note"),
    ]));

    c.append(&help_section("Context Menu"));
    c.append(&help_paragraph(
        "Right-click a note in the sidebar for more options:"
    ));

    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::None);
    for item in ["Rename", "Pin / Unpin to Top", "Move to Folder", "Move to Trash"] {
        list.append(&adw::ActionRow::builder().title(item).build());
    }
    c.append(&list);

    c.append(&help_section("Templates"));
    c.append(&help_paragraph(
        "Use the sidebar menu \u{2192} \u{201c}New from Template\u{201d} to create \
         a note from a preset template (e.g. meeting notes, journal entry)."
    ));

    help_page("Creating Notes", &c)
}

fn build_formatting_page() -> adw::NavigationPage {
    let c = help_content();

    c.append(&help_paragraph(
        "Use the toolbar or keyboard shortcuts to format text. \
         Write in the source editor on the left and see a live \
         preview on the right."
    ));

    c.append(&help_section("Inline Formatting"));
    c.append(&shortcut_list(&[
        ("Ctrl+B", "Bold"),
        ("Ctrl+I", "Italic"),
        ("Ctrl+U", "Underline"),
        ("Ctrl+D", "Strikethrough"),
        ("Ctrl+E", "Inline code"),
        ("Ctrl+K", "Insert link"),
    ]));

    c.append(&help_section("Block Types"));
    c.append(&shortcut_list(&[
        ("Ctrl+1\u{2026}6", "Heading 1 through 6"),
        ("Ctrl+Shift+Q", "Block quote"),
        ("Ctrl+Shift+L", "Bullet list"),
        ("Toolbar", "Ordered list"),
        ("Toolbar", "Task list"),
        ("Ctrl+Space", "Toggle checkbox"),
    ]));

    c.append(&help_section("Insertions"));
    c.append(&help_paragraph(
        "Use the toolbar buttons to insert horizontal rules, images, \
         tables, and code blocks."
    ));

    c.append(&help_section("Undo & Redo"));
    c.append(&shortcut_list(&[
        ("Ctrl+Z", "Undo"),
        ("Ctrl+Shift+Z", "Redo"),
    ]));

    help_page("Formatting", &c)
}

fn build_code_blocks_page() -> adw::NavigationPage {
    let c = help_content();

    c.append(&help_paragraph(
        "Code blocks let you include source code with syntax-aware formatting."
    ));

    c.append(&help_section("Inserting a Code Block"));
    c.append(&help_paragraph(
        "Click the code block button in the toolbar and select a language. \
         A fenced code block will be inserted at the cursor position."
    ));

    c.append(&help_section("Editing"));
    c.append(&help_paragraph(
        "Type your code between the opening and closing fence markers (```). \
         The preview pane renders code blocks with syntax highlighting."
    ));

    c.append(&help_section("Supported Languages"));
    let lang_list = gtk::ListBox::new();
    lang_list.add_css_class("boxed-list");
    lang_list.set_selection_mode(gtk::SelectionMode::None);
    for lang in [
        "text", "javascript", "python", "rust", "bash",
        "html", "css", "json", "yaml", "go", "c", "cpp", "java", "sql",
    ] {
        lang_list.append(&adw::ActionRow::builder().title(lang).build());
    }
    c.append(&lang_list);

    c.append(&help_tip(
        "Tip: In Markdown source view, you can type fenced code blocks manually \
         with any language identifier."
    ));

    help_page("Code Blocks", &c)
}

fn build_tables_page() -> adw::NavigationPage {
    let c = help_content();

    c.append(&help_paragraph(
        "Tables can be inserted from the toolbar. A default 3\u{00d7}3 table \
         with a header row is created."
    ));

    c.append(&help_section("Editing Tables"));
    c.append(&help_paragraph(
        "Edit cell contents directly in the rich editor. \
         The table structure is preserved as Markdown pipe syntax."
    ));

    c.append(&help_tip(
        "Tip: Edit the raw pipe-delimited Markdown syntax directly in the \
         source editor. The preview pane will render the table."
    ));

    help_page("Tables", &c)
}

fn build_folders_tags_page() -> adw::NavigationPage {
    let c = help_content();

    c.append(&help_section("Folders"));
    c.append(&help_paragraph(
        "Organize notes into folders using the sidebar."
    ));

    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::None);
    for item in [
        "Create folders from the sidebar menu or by right-clicking in the sidebar",
        "Right-click a folder to create subfolders, rename, or delete",
        "Move notes into folders via right-click \u{2192} \u{201c}Move to Folder\u{201d}",
        "Click a folder in the sidebar to expand or collapse it",
    ] {
        list.append(&adw::ActionRow::builder().title(item).build());
    }
    c.append(&list);

    c.append(&help_section("Tags"));
    c.append(&help_paragraph(
        "Tags help you categorize and filter notes across folders."
    ));

    let tag_list = gtk::ListBox::new();
    tag_list.add_css_class("boxed-list");
    tag_list.set_selection_mode(gtk::SelectionMode::None);
    for item in [
        "Add tags using the tag bar at the bottom of the editor",
        "Type a tag name and press Enter to add it",
        "Click the \u{00d7} on a tag to remove it",
        "Use tag filters in the sidebar to show only matching notes",
    ] {
        tag_list.append(&adw::ActionRow::builder().title(item).build());
    }
    c.append(&tag_list);

    help_page("Folders &amp; Tags", &c)
}

fn build_search_page() -> adw::NavigationPage {
    let c = help_content();

    c.append(&help_section("Search"));
    c.append(&help_paragraph(
        "Quickly find notes by title using the search bar in the sidebar."
    ));
    c.append(&shortcut_list(&[
        ("Ctrl+Shift+F", "Focus the search bar"),
    ]));
    c.append(&help_paragraph(
        "Start typing to filter the notes list. The search matches note titles."
    ));

    c.append(&help_section("Daily Notes"));
    c.append(&help_paragraph(
        "Daily notes are date-stamped notes for journaling or quick capture."
    ));
    c.append(&shortcut_list(&[
        ("Ctrl+Shift+T", "Create or open today\u{2019}s daily note"),
    ]));
    c.append(&help_paragraph(
        "If a daily note for today already exists, it will be opened. \
         Otherwise a new one is created with today\u{2019}s date as the title."
    ));

    help_page("Search &amp; Daily Notes", &c)
}

fn build_snapshots_page() -> adw::NavigationPage {
    let c = help_content();

    c.append(&help_section("Snapshots"));
    c.append(&help_paragraph(
        "Snapshots let you save a named version of your note at any point in time. \
         Use the primary menu (hamburger) \u{2192} \u{201c}Save Snapshot\u{201d} to create one."
    ));

    c.append(&help_section("Version History"));
    c.append(&help_paragraph(
        "View all saved snapshots for the current note via the primary menu \
         \u{2192} \u{201c}Version History\u{201d}. You can restore any previous \
         snapshot to replace the current content."
    ));

    c.append(&help_tip(
        "Tip: Snapshots are stored inside the encrypted vault, so they\u{2019}re \
         just as secure as your notes."
    ));

    help_page("Snapshots &amp; History", &c)
}

fn build_import_export_page() -> adw::NavigationPage {
    let c = help_content();

    c.append(&help_section("Import"));
    c.append(&help_paragraph(
        "Import a Markdown file as a new note in your vault."
    ));
    c.append(&shortcut_list(&[
        ("Ctrl+O", "Import a .md file"),
    ]));

    c.append(&help_section("Export"));
    c.append(&help_paragraph(
        "Export the current note to a file on disk. Available from the primary menu."
    ));

    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::None);
    list.append(&adw::ActionRow::builder()
        .title("Export as Markdown")
        .subtitle("Saves the note as a .md file")
        .build());
    list.append(&adw::ActionRow::builder()
        .title("Export as HTML")
        .subtitle("Converts the note to HTML and saves it")
        .build());
    c.append(&list);

    help_page("Import &amp; Export", &c)
}

fn build_backlinks_page() -> adw::NavigationPage {
    let c = help_content();

    c.append(&help_paragraph(
        "Backlinks show which other notes in your vault link to the current note."
    ));

    c.append(&help_section("Viewing Backlinks"));
    c.append(&help_paragraph(
        "Open the primary menu (hamburger) \u{2192} \u{201c}View Backlinks\u{201d} \
         to see a list of notes that reference the current note."
    ));

    c.append(&help_tip(
        "Tip: To create a link to another note, use Ctrl+K and type the note\u{2019}s name."
    ));

    help_page("Backlinks", &c)
}

fn build_shortcuts_page() -> adw::NavigationPage {
    let c = help_content();

    let shortcuts = [
        ("General", &[
            ("Ctrl+N", "New note"),
            ("Ctrl+S", "Save"),
            ("Ctrl+W", "Close tab"),
            ("Ctrl+\\", "Toggle sidebar"),
            ("Ctrl+Shift+F", "Search notes"),
            ("Ctrl+Shift+P", "Command palette"),
            ("Ctrl+Shift+J", "Zen mode"),
            ("Ctrl+Shift+T", "Daily note"),
            ("Ctrl+Shift+D", "Toggle theme"),
            ("F1", "Help"),
            ("F2", "Rename note"),
            ("F11", "Fullscreen"),
        ] as &[(&str, &str)]),
        ("Formatting", &[
            ("Ctrl+B", "Bold"),
            ("Ctrl+I", "Italic"),
            ("Ctrl+U", "Underline"),
            ("Ctrl+D", "Strikethrough"),
            ("Ctrl+E", "Inline code"),
            ("Ctrl+K", "Insert link"),
            ("Ctrl+1\u{2026}6", "Heading 1\u{2026}6"),
            ("Ctrl+Shift+Q", "Block quote"),
            ("Ctrl+Shift+L", "Bullet list"),
            ("Ctrl+Space", "Toggle checkbox"),
        ]),
        ("Editing", &[
            ("Ctrl+Z", "Undo"),
            ("Ctrl+Shift+Z", "Redo"),
        ]),
    ];

    for (group_title, entries) in shortcuts {
        c.append(&help_section(group_title));
        c.append(&shortcut_list(entries));
    }

    help_page("Keyboard Shortcuts", &c)
}
