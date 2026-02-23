mod crypto;
mod vault;
mod state;
use state::*;
mod ui;
use ui::*;
mod signals;
pub use signals::*;

use adw::prelude::*;
use gtk::gdk;
use gtk::glib;
const APP_ID: &str = "com.pithos.notebook";
const MAX_UNDO_HISTORY: usize = 100;
const AUTO_SAVE_INTERVAL_SECS: u32 = 30;



// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    if let Err(err) = adw::init() {
        eprintln!("Failed to initialize Adwaita: {err}");
        std::process::exit(1);
    }
    install_css();

    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run();
}

// ---------------------------------------------------------------------------
// UI building — decomposed into focused helpers
// ---------------------------------------------------------------------------





