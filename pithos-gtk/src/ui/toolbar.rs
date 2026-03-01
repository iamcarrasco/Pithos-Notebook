use adw::prelude::*;
use std::{cell::Cell, rc::Rc};

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
    let strike_btn = icon_button(
        "format-text-strikethrough-symbolic",
        "Strikethrough (Ctrl+D)",
    );
    let code_btn = symbol_button("<>", "Inline Code (Ctrl+E)");

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

    let bullet_btn = icon_button("view-list-bullet-symbolic", "Bullet List");
    let ordered_btn = icon_button("view-list-ordered-symbolic", "Ordered List");
    let task_btn = icon_button("emblem-ok-symbolic", "Task List");

    let link_btn = icon_button("insert-link-symbolic", "Insert Link (Ctrl+K)");

    let table_menu_btn = gtk::MenuButton::new();
    table_menu_btn.set_icon_name("view-grid-symbolic");
    table_menu_btn.set_tooltip_text(Some("Table Actions"));
    set_accessible_label(&table_menu_btn, "Table Actions");
    table_menu_btn.add_css_class("flat");
    table_menu_btn.add_css_class("toolbar-icon");
    let table_popover = gtk::Popover::new();
    let table_popover_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    table_popover_box.set_margin_start(6);
    table_popover_box.set_margin_end(6);
    table_popover_box.set_margin_top(6);
    table_popover_box.set_margin_bottom(6);

    // --- Table size grid picker (Apostrophe-style) ---
    let table_grid = gtk::Grid::new();
    table_grid.set_column_spacing(2);
    table_grid.set_row_spacing(2);
    let table_grid_label = gtk::Label::new(None);
    table_grid_label.add_css_class("table-grid-label");
    table_grid_label.set_halign(gtk::Align::Center);
    table_grid_label.set_valign(gtk::Align::Center);
    table_grid_label.set_can_target(false);

    let table_hover: Rc<Cell<(i32, i32)>> = Rc::new(Cell::new((0, 0)));

    const GRID_ROWS: i32 = 5;
    const GRID_COLS: i32 = 6;
    for row in 0..GRID_ROWS {
        for col in 0..GRID_COLS {
            let cell = gtk::Button::new();
            cell.add_css_class("flat");
            cell.add_css_class("table-grid-cell");
            // Name encodes position for hover lookup
            cell.set_widget_name(&format!("tc-{}-{}", row + 1, col + 1));

            // Hover tracking
            let motion = gtk::EventControllerMotion::new();
            {
                let grid = table_grid.clone();
                let label = table_grid_label.clone();
                let hover = table_hover.clone();
                let r = row + 1;
                let c = col + 1;
                motion.connect_enter(move |_, _, _| {
                    hover.set((r, c));
                    // Highlight all cells from top-left to this one
                    let mut child = grid.first_child();
                    while let Some(w) = child {
                        let name = w.widget_name();
                        if let Some(rest) = name.strip_prefix("tc-") {
                            let parts: Vec<&str> = rest.split('-').collect();
                            if parts.len() == 2 {
                                let cr: i32 = parts[0].parse().unwrap_or(0);
                                let cc: i32 = parts[1].parse().unwrap_or(0);
                                if cr <= r && cc <= c {
                                    w.add_css_class("hovered");
                                } else {
                                    w.remove_css_class("hovered");
                                }
                            }
                        }
                        child = w.next_sibling();
                    }
                    label.set_label(&format!("{r} \u{00d7} {c}"));
                });
            }
            {
                let grid = table_grid.clone();
                let label = table_grid_label.clone();
                let hover = table_hover.clone();
                motion.connect_leave(move |_| {
                    hover.set((0, 0));
                    let mut child = grid.first_child();
                    while let Some(w) = child {
                        w.remove_css_class("hovered");
                        child = w.next_sibling();
                    }
                    label.set_label("");
                });
            }
            cell.add_controller(motion);
            table_grid.attach(&cell, col, row, 1, 1);
        }
    }

    let grid_overlay = gtk::Overlay::new();
    grid_overlay.set_child(Some(&table_grid));
    grid_overlay.add_overlay(&table_grid_label);
    table_popover_box.append(&grid_overlay);

    let table_sep = gtk::Separator::new(gtk::Orientation::Horizontal);
    table_popover_box.append(&table_sep);

    let table_add_row_btn = gtk::Button::with_label("Add Row");
    let table_add_col_btn = gtk::Button::with_label("Add Column");
    let table_align_btn = gtk::Button::with_label("Align Table");
    for button in [&table_add_row_btn, &table_add_col_btn, &table_align_btn] {
        button.add_css_class("flat");
        button.set_halign(gtk::Align::Fill);
        button.set_hexpand(true);
        table_popover_box.append(button);
    }
    table_popover.set_child(Some(&table_popover_box));
    table_menu_btn.set_popover(Some(&table_popover));

    let rule_btn = symbol_button("\u{2014}", "Insert Horizontal Rule");
    let fullscreen_btn = icon_button("view-fullscreen-symbolic", "Fullscreen (F11)");

    let image_btn = icon_button("insert-image-symbolic", "Insert Image Snippet");

    let code_block_menu_btn = gtk::MenuButton::new();
    code_block_menu_btn.set_icon_name("list-add-symbolic");
    code_block_menu_btn.set_tooltip_text(Some("Insert Code Block"));
    set_accessible_label(&code_block_menu_btn, "Insert Code Block");
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
    for button in [
        &bold_btn,
        &italic_btn,
        &underline_btn,
        &strike_btn,
        &code_btn,
    ] {
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
    insert_group.append(&link_btn);
    insert_group.append(&table_menu_btn);
    insert_group.append(&rule_btn);
    insert_group.append(&image_btn);
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
        table_menu: table_menu_btn,
        table_popover,
        table_hover,
        table_add_row: table_add_row_btn,
        table_add_col: table_add_col_btn,
        table_align: table_align_btn,
        rule: rule_btn,
        fullscreen: fullscreen_btn,
        image: image_btn,
        code_block_popover,
        code_languages: code_lang_buttons,
    };

    (toolbar, widgets)
}
