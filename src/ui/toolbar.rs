use adw::prelude::*;

use crate::ui::types::*;
use crate::*; // For everything left in main.rs temporarily

pub fn build_toolbar() -> (gtk::Box, ToolbarWidgets) {
    let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    toolbar.add_css_class("toolbar-row");
    toolbar.set_margin_start(8);
    toolbar.set_margin_end(8);
    toolbar.set_margin_top(8);
    toolbar.set_margin_bottom(8);

    let undo_btn = icon_button("edit-undo-symbolic", "Undo (Ctrl+Z)");
    let redo_btn = icon_button("edit-redo-symbolic", "Redo (Ctrl+Shift+Z / Ctrl+Y)");

    let bold_btn = icon_button("format-text-bold-symbolic", "Bold (Ctrl+B)");
    let italic_btn = icon_button("format-text-italic-symbolic", "Italic (Ctrl+I)");
    let underline_btn = icon_button("format-text-underline-symbolic", "Underline (Ctrl+U)");
    let strike_btn = icon_button("format-text-strikethrough-symbolic", "Strikethrough (Ctrl+D)");
    let code_btn = symbol_button("<>", "Inline code (Ctrl+E)");

    let block_menu_btn = gtk::MenuButton::new();
    block_menu_btn.set_label("Block type");
    block_menu_btn.add_css_class("toolbar-pill");
    block_menu_btn.add_css_class("block-type");
    block_menu_btn.set_width_request(170);

    let block_popover = gtk::Popover::new();
    let block_popover_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    block_popover_box.set_margin_start(6);
    block_popover_box.set_margin_end(6);
    block_popover_box.set_margin_top(6);
    block_popover_box.set_margin_bottom(6);
    let block_paragraph_btn = gtk::Button::with_label("Paragraph");
    let block_h1_btn = gtk::Button::with_label("Heading 1");
    let block_h2_btn = gtk::Button::with_label("Heading 2");
    let block_h3_btn = gtk::Button::with_label("Heading 3");
    let block_h4_btn = gtk::Button::with_label("Heading 4");
    let block_h5_btn = gtk::Button::with_label("Heading 5");
    let block_h6_btn = gtk::Button::with_label("Heading 6");
    let block_quote_btn = gtk::Button::with_label("Quote");
    for button in [
        &block_paragraph_btn,
        &block_h1_btn,
        &block_h2_btn,
        &block_h3_btn,
        &block_h4_btn,
        &block_h5_btn,
        &block_h6_btn,
        &block_quote_btn,
    ] {
        button.add_css_class("flat");
        button.set_halign(gtk::Align::Fill);
        button.set_hexpand(true);
        block_popover_box.append(button);
    }
    block_popover.set_child(Some(&block_popover_box));
    block_menu_btn.set_popover(Some(&block_popover));

    let bullet_btn = icon_button("view-list-bullet-symbolic", "Bullet list");
    let ordered_btn = icon_button("view-list-ordered-symbolic", "Ordered list");
    let task_btn = icon_button("emblem-ok-symbolic", "Task list");

    let link_btn = icon_button("insert-link-symbolic", "Insert link (Ctrl+K)");
    let table_btn = icon_button("view-grid-symbolic", "Insert table");
    let rule_btn = symbol_button("\u{2014}", "Insert horizontal rule");
    let fullscreen_btn = icon_button("view-fullscreen-symbolic", "Fullscreen (F11)");

    let image_btn = icon_button("insert-image-symbolic", "Insert image snippet");

    let code_block_menu_btn = gtk::MenuButton::new();
    code_block_menu_btn.set_icon_name("list-add-symbolic");
    code_block_menu_btn.set_tooltip_text(Some("Insert code block"));
    set_accessible_label(&code_block_menu_btn, "Insert code block");
    code_block_menu_btn.add_css_class("flat");
    code_block_menu_btn.add_css_class("toolbar-icon");
    let code_block_popover = gtk::Popover::new();
    let code_block_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    code_block_box.set_margin_start(6);
    code_block_box.set_margin_end(6);
    code_block_box.set_margin_top(6);
    code_block_box.set_margin_bottom(6);

    let mut code_lang_buttons: Vec<(String, gtk::Button)> = Vec::new();
    for lang in CODE_LANGUAGES {
        let btn = gtk::Button::with_label(&format!("Code block: {lang}"));
        btn.add_css_class("flat");
        btn.set_halign(gtk::Align::Fill);
        btn.set_hexpand(true);
        code_block_box.append(&btn);
        code_lang_buttons.push((lang.to_string(), btn));
    }
    code_block_popover.set_child(Some(&code_block_box));
    code_block_menu_btn.set_popover(Some(&code_block_popover));

    // Group related buttons with .linked for pill-like segmented controls
    let undo_redo_group = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    undo_redo_group.add_css_class("linked");
    undo_redo_group.append(&undo_btn);
    undo_redo_group.append(&redo_btn);
    toolbar.append(&undo_redo_group);

    toolbar.append(&toolbar_separator());

    let format_group = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    format_group.add_css_class("linked");
    for button in [&bold_btn, &italic_btn, &underline_btn, &strike_btn, &code_btn] {
        format_group.append(button);
    }
    toolbar.append(&format_group);

    toolbar.append(&toolbar_separator());
    toolbar.append(&block_menu_btn);
    toolbar.append(&toolbar_separator());

    let list_group = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    list_group.add_css_class("linked");
    for button in [&bullet_btn, &ordered_btn, &task_btn] {
        list_group.append(button);
    }
    toolbar.append(&list_group);

    toolbar.append(&toolbar_separator());

    let insert_group = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    insert_group.add_css_class("linked");
    for button in [&link_btn, &table_btn, &rule_btn, &image_btn] {
        insert_group.append(button);
    }
    toolbar.append(&insert_group);

    toolbar.append(&code_block_menu_btn);
    toolbar.append(&toolbar_separator());
    toolbar.append(&fullscreen_btn);

    let widgets = ToolbarWidgets {
        undo: undo_btn,
        redo: redo_btn,
        bold: bold_btn,
        italic: italic_btn,
        underline: underline_btn,
        strike: strike_btn,
        code: code_btn,
        block_popover,
        block_paragraph: block_paragraph_btn,
        block_h1: block_h1_btn,
        block_h2: block_h2_btn,
        block_h3: block_h3_btn,
        block_h4: block_h4_btn,
        block_h5: block_h5_btn,
        block_h6: block_h6_btn,
        block_quote: block_quote_btn,
        bullet: bullet_btn,
        ordered: ordered_btn,
        task: task_btn,
        link: link_btn,
        table: table_btn,
        rule: rule_btn,
        fullscreen: fullscreen_btn,
        image: image_btn,
        code_block_popover,
        code_languages: code_lang_buttons,
    };

    (toolbar, widgets)
}
