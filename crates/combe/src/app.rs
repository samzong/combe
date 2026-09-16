use crate::entry::{self, Entry};
use crate::{ghostty, quota_panel, sidebar, sidebar_panel, window};
use combe_catalog::home_dir;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationDelegate, NSApplicationTerminateReply, NSPasteboard,
    NSPasteboardTypeString,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSString, NSURL};
use std::path::Path;

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeAppDelegate"]
    #[ivars = ()]
    pub struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationShouldTerminateAfterLastWindowClosed:))]
        fn should_terminate_after_last_window_closed(&self, _app: &NSApplication) -> bool {
            false
        }

        #[unsafe(method(applicationShouldTerminate:))]
        fn should_terminate(&self, _app: &NSApplication) -> NSApplicationTerminateReply {
            let open = window::is_open();
            if !open || window::confirm_quit() {
                NSApplicationTerminateReply::TerminateNow
            } else {
                NSApplicationTerminateReply::TerminateCancel
            }
        }

        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn should_handle_reopen(&self, _app: &NSApplication, _has_visible_windows: bool) -> bool {
            window::reveal_window();
            true
        }

        #[unsafe(method(application:openURLs:))]
        fn open_urls(&self, _app: &NSApplication, urls: &NSArray<NSURL>) {
            open_urls(urls);
        }

        #[unsafe(method(applicationDidBecomeActive:))]
        fn did_become_active(&self, _notification: &AnyObject) {
            ghostty::set_focus(true);
            window::refresh_attention();
            sidebar_panel::refresh();
            quota_panel::refresh();
        }

        #[unsafe(method(applicationDidResignActive:))]
        fn did_resign_active(&self, _notification: &AnyObject) {
            ghostty::set_focus(false);
            window::deactivate_chrome();
        }
    }

    impl AppDelegate {
        #[unsafe(method(openTab:userData:error:))]
        fn open_tab_service(
            &self,
            pasteboard: &NSPasteboard,
            _user_data: Option<&NSString>,
            _error: *mut *mut NSString,
        ) {
            if let Some(text) = unsafe { pasteboard.stringForType(NSPasteboardTypeString) } {
                open_paths(&text.to_string());
            }
        }
    }
);

impl AppDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

pub fn install_delegate(mtm: MainThreadMarker, app: &NSApplication) -> Retained<AppDelegate> {
    let delegate = AppDelegate::new(mtm);
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    unsafe { app.setServicesProvider(Some(delegate.as_ref())) };
    delegate
}

fn open_urls(urls: &NSArray<NSURL>) {
    for url in urls {
        if let Some(entry) = resolve(&url) {
            accept(entry);
        }
    }
}

fn open_paths(text: &str) {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(entry) = entry::file(Path::new(line)) {
            accept(entry);
        }
    }
}

fn resolve(url: &NSURL) -> Option<Entry> {
    if url.isFileURL() {
        return entry::file(Path::new(&url.path()?.to_string()));
    }
    let scheme = url.scheme()?.to_string().to_ascii_lowercase();
    if scheme == entry::HOP_SCHEME {
        return entry::directory(Path::new(&url.path()?.to_string()));
    }
    let host = url.host()?.to_string();
    match scheme.as_str() {
        "ssh" => entry::ssh(
            url.user().map(|user| user.to_string()).as_deref(),
            &host,
            url.port()
                .and_then(|port| u16::try_from(port.as_i64()).ok()),
        ),
        "x-man-page" => {
            let path = url.path().map(|path| path.to_string());
            match path.as_deref().map(|path| path.trim_start_matches('/')) {
                Some(page) if !page.is_empty() => entry::man(Some(&host), page),
                _ => entry::man(None, &host),
            }
        }
        _ => None,
    }
}

fn accept(entry: Entry) {
    match entry {
        Entry::Workspace(path) => {
            sidebar::add(std::slice::from_ref(&path));
            let repos = sidebar::repos();
            let label = repos
                .iter()
                .flat_map(|repo| repo.rows.iter())
                .find(|row| row.path == path)
                .map(|row| row.label.clone())
                .unwrap_or_else(|| entry::name_of(&path));
            sidebar_panel::set_repos(repos);
            sidebar_panel::select(&path.to_string_lossy(), &label);
        }
        Entry::Hop(dir) => {
            let hop = entry::hop(&dir, &sidebar::rows(), home_dir().as_deref());
            window::new_tab(
                &hop.workspace.to_string_lossy(),
                &hop.cwd.to_string_lossy(),
                &hop.name,
                None,
            );
        }
        Entry::Run { cwd, name, input } => {
            if !window::confirm("Run this command in Combe?", &input, "Run") {
                return;
            }
            let cwd = cwd
                .map(|cwd| cwd.to_string_lossy().into_owned())
                .or_else(window::current_workspace)
                .unwrap_or_else(|| std::env::var("HOME").unwrap_or_else(|_| "/".to_owned()));
            window::new_tab(&cwd, &cwd, &name, Some(&input));
        }
    }
    window::reveal_window();
}
