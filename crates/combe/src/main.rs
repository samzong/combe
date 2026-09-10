mod chrome_view;
mod cli;
mod entry;
mod find_bar;
mod ghostty;
mod habits;
mod overview;
mod quota;
mod quota_panel;
mod sidebar;
mod split;
mod surface;
mod tabs;
mod window;

use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_foundation::MainThreadMarker;
use std::process::ExitCode;

fn resources_dir() -> String {
    let bundled = std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.parent()?.join("Resources/ghostty")))
        .filter(|dir| dir.is_dir());
    match bundled {
        Some(dir) => dir.to_string_lossy().into_owned(),
        None => ghostty_sys::resources_dir().to_owned(),
    }
}

fn main() -> ExitCode {
    if let Some(code) = cli::run() {
        return code;
    }

    unsafe { std::env::set_var("GHOSTTY_RESOURCES_DIR", resources_dir()) };

    let mtm = MainThreadMarker::new().expect("combe must run on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

    ghostty::init();

    let _delegate = window::install_delegate(mtm, &app);
    window::install_menu(mtm, &app);
    window::open(mtm);

    app.activate();
    app.run();

    ExitCode::SUCCESS
}
