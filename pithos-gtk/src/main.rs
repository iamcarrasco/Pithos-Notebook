mod ui;
pub use ui::*;
mod signals;
pub use signals::*;
mod preview;
pub use preview::*;
mod editor;
pub use editor::*;
mod persistence;
pub use persistence::*;
mod notes;
pub use notes::*;
mod sidebar_ops;
pub use sidebar_ops::*;
mod actions;
pub use actions::*;
mod app_dialogs;
pub use app_dialogs::*;

use adw::prelude::*;
use gtk::gdk;
use gtk::glib;
const APP_ID: &str = "com.pithos.notebook";
const MAX_UNDO_HISTORY: usize = 100;
const AUTO_SAVE_INTERVAL_SECS: u32 = 30;

fn main() {
    if let Err(err) = adw::init() {
        eprintln!("Failed to initialize Adwaita: {err}");
        std::process::exit(1);
    }
    install_css();

    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        // If a window already exists, just present it (single-instance).
        if let Some(window) = app.active_window() {
            window.present();
            return;
        }
        build_ui(app);
    });
    app.run();
}
