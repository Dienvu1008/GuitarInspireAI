//! Build information and one-time initialization.

use flutter_rust_bridge::frb;

/// Version of the Music Core linked into the app.
#[frb(sync)]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Called once by the generated Dart `RustLib.init()`.
#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
