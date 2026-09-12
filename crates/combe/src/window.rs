use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::ffi::c_void;
use std::path::{Path, PathBuf};

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{ClassType, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAccessibility, NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSAnimationContext,
    NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSApplication,
    NSApplicationDelegate, NSApplicationTerminateReply, NSAutoresizingMaskOptions,
    NSBackingStoreType, NSButton, NSColor, NSEvent, NSEventModifierFlags, NSEventType,
    NSPasteboard, NSPasteboardTypeString, NSResponder, NSSplitView, NSSplitViewDelegate,
    NSSplitViewDividerStyle, NSText, NSView, NSWindow, NSWindowButton, NSWindowDelegate,
    NSWindowStyleMask, NSWindowTitleVisibility, NSWorkspace,
};
use objc2_core_foundation::CGFloat;
use objc2_foundation::{
    MainThreadMarker, NSArray, NSInteger, NSNotification, NSPoint, NSRect, NSSize, NSString, NSURL,
};
use objc2_quartz_core::CAMediaTimingFunction;

use crate::chrome_view::{self, ClickView};
use crate::entry::{self, Entry};
use crate::ghostty;
use crate::habits;
use crate::menu;
use crate::overview::Overview;
use crate::quota_panel;
use crate::split;
use crate::surface::SurfaceView;
use crate::tab_bar::TabBar;
use crate::tabs::Tabs;
use crate::{sidebar, sidebar_panel};

const TOP_BAR_HEIGHT: f64 = 60.0;
const TRAFFIC_INSET: f64 = 84.0;
const FULLSCREEN_INSET: f64 = 12.0;
const INSET: f64 = 12.0;
const WINDOW_RADIUS: f64 = 34.0;
const DIVIDER_GRAB: f64 = 4.0;

const FILL: NSAutoresizingMaskOptions = NSAutoresizingMaskOptions(
    NSAutoresizingMaskOptions::ViewWidthSizable.0 | NSAutoresizingMaskOptions::ViewHeightSizable.0,
);

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static ANIMATE_CHROME: Cell<bool> = const { Cell::new(false) };
    static ATTENTION_QUEUED: Cell<bool> = const { Cell::new(false) };
}

struct State {
    window: Retained<NSWindow>,
    _window_delegate: Retained<WindowDelegate>,
    split: Retained<SplitView>,
    _split_delegate: Retained<SplitDelegate>,
    sidebar_pane: Retained<NSView>,
    overview: Option<Retained<Overview>>,
    overview_return: Option<Retained<NSResponder>>,
    overview_tab: Option<u64>,
    overview_button: Option<Retained<NSButton>>,
    tab_bar: TabBar,
    content: Retained<NSView>,
    tabs: Tabs,
}

pub enum TabTarget {
    Previous,
    Next,
    Last,
    Index(usize),
}

define_class!(
    #[unsafe(super(NSWindow))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeWindow"]
    #[ivars = ()]
    struct Window;

    impl Window {
        #[unsafe(method(sendEvent:))]
        fn send_event(&self, event: &NSEvent) {
            if handle_overview_event(event) { return; }
            let quota = quota_panel::handle_event(event);
            let sidebar = sidebar_panel::handle_event(event);
            if !quota && !sidebar { let _: () = unsafe { msg_send![super(self), sendEvent: event] }; }
            sidebar_panel::after_event(event);
        }
    }
);

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
            let open = STATE.with(|state| {
                state
                    .borrow()
                    .as_ref()
                    .is_some_and(|state| state.window.isVisible() || state.window.isMiniaturized())
            });
            if !open || confirm_quit() {
                NSApplicationTerminateReply::TerminateNow
            } else {
                NSApplicationTerminateReply::TerminateCancel
            }
        }

        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn should_handle_reopen(&self, _app: &NSApplication, _has_visible_windows: bool) -> bool {
            reveal_window();
            true
        }

        #[unsafe(method(application:openURLs:))]
        fn open_urls(&self, _app: &NSApplication, urls: &NSArray<NSURL>) {
            open_urls(urls);
        }

        #[unsafe(method(applicationDidBecomeActive:))]
        fn did_become_active(&self, _notification: &AnyObject) {
            ghostty::set_focus(true);
            refresh_attention();
            sidebar_panel::refresh();
            quota_panel::refresh();
        }

        #[unsafe(method(applicationDidResignActive:))]
        fn did_resign_active(&self, _notification: &AnyObject) {
            ghostty::set_focus(false);
            deactivate_chrome();
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

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeWindowDelegate"]
    #[ivars = ()]
    struct WindowDelegate;

    unsafe impl NSObjectProtocol for WindowDelegate {}

    unsafe impl NSWindowDelegate for WindowDelegate {
        #[unsafe(method(windowDidBecomeKey:))]
        fn did_become_key(&self, _note: &NSNotification) {
            refresh_attention();
        }
        #[unsafe(method(windowDidResignKey:))]
        fn did_resign_key(&self, _note: &NSNotification) {
            deactivate_chrome();
        }

        #[unsafe(method(windowShouldClose:))]
        fn window_should_close(&self, _sender: &NSWindow) -> bool {
            confirm_quit()
        }
    }
);

pub fn install_delegate(mtm: MainThreadMarker, app: &NSApplication) -> Retained<AppDelegate> {
    let delegate = AppDelegate::new(mtm);
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    unsafe { app.setServicesProvider(Some(delegate.as_ref())) };
    delegate
}

pub fn open(mtm: MainThreadMarker) {
    let frame = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(habits::WINDOW_WIDTH, habits::WINDOW_HEIGHT),
    );
    let style = NSWindowStyleMask::Titled
        | NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable
        | NSWindowStyleMask::FullSizeContentView;
    let window = unsafe {
        let allocated = Window::alloc(mtm).set_ivars(());
        let window: Retained<Window> = msg_send![super(allocated), initWithContentRect: frame, styleMask: style, backing: NSBackingStoreType::Buffered, defer: false];
        Retained::into_super(window)
    };
    window.setTitle(&NSString::from_str("Combe"));
    window.setAcceptsMouseMovedEvents(true);
    window.setTitlebarAppearsTransparent(true);
    window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
    window.setOpaque(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    unsafe { window.setReleasedWhenClosed(false) };

    let split = SplitView::alloc(mtm).set_ivars(());
    let split: Retained<SplitView> = unsafe { msg_send![super(split), initWithFrame: frame] };
    split.setVertical(true);
    split.setDividerStyle(NSSplitViewDividerStyle::Thin);
    split.setAutoresizingMask(FILL);

    let sidebar_frame = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(habits::SIDEBAR_WIDTH + 2.0 * INSET, frame.size.height),
    );
    let sidebar_pane = NSView::initWithFrame(NSView::alloc(mtm), sidebar_frame);
    sidebar_pane.setAutoresizingMask(NSAutoresizingMaskOptions::ViewHeightSizable);

    split.addSubview(&sidebar_pane);

    let right_frame = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(
            frame.size.width - habits::SIDEBAR_WIDTH - 2.0 * INSET,
            frame.size.height,
        ),
    );
    let right = NSView::initWithFrame(NSView::alloc(mtm), right_frame);
    right.setAutoresizingMask(FILL);

    let tab_bar = TabBar::new(
        mtm,
        NSRect::new(
            NSPoint::new(0.0, right_frame.size.height - TOP_BAR_HEIGHT),
            NSSize::new(right_frame.size.width, TOP_BAR_HEIGHT),
        ),
        activate_tab,
        request_close_tab,
        new_current_tab,
    );

    let status_h = quota_panel::mount(
        mtm,
        &right,
        right_frame.size.width,
        quota_window_live,
        layout_chrome,
        focus_active,
    );

    let content = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(
            NSPoint::new(0.0, status_h),
            NSSize::new(
                right_frame.size.width,
                (right_frame.size.height - TOP_BAR_HEIGHT - status_h).max(0.0),
            ),
        ),
    );
    content.setAutoresizingMask(FILL);
    right.addSubview(&content);

    split.addSubview(&right);

    let root = ChromeView::alloc(mtm).set_ivars(());
    let root: Retained<ChromeView> = unsafe { msg_send![super(root), initWithFrame: frame] };
    root.setAutoresizingMask(FILL);
    root.setWantsLayer(true);
    if let Some(layer) = root.layer() {
        layer.setMasksToBounds(true);
        let _: () = unsafe { msg_send![&*layer, setCornerRadius: WINDOW_RADIUS] };
    }
    root.addSubview(&split);
    root.addSubview(tab_bar.view());
    let overview_button = chrome_view::icon_button(
        mtm,
        "square.grid.2x2",
        &menu::commands(mtm),
        sel!(toggleTabOverview:),
        NSRect::default(),
    );
    if let Some(button) = overview_button.as_ref() {
        button.setAccessibilityLabel(Some(&NSString::from_str("Tab Overview")));
        button.setToolTip(Some(&NSString::from_str("Tab Overview (⌘⇧\\)")));
        root.addSubview(button);
    }
    sidebar_panel::mount(
        mtm,
        &window,
        &root,
        open_worktree,
        || dismiss_overview(false),
        sidebar_changed,
        focus_active,
    );
    window.setContentView(Some(&root));

    let split_delegate = SplitDelegate::alloc(mtm).set_ivars(());
    let split_delegate: Retained<SplitDelegate> = unsafe { msg_send![super(split_delegate), init] };
    split.setDelegate(Some(ProtocolObject::from_ref(&*split_delegate)));
    split.setPosition_ofDividerAtIndex(habits::SIDEBAR_WIDTH + 2.0 * INSET, 0);

    let window_delegate = WindowDelegate::alloc(mtm).set_ivars(());
    let window_delegate: Retained<WindowDelegate> =
        unsafe { msg_send![super(window_delegate), init] };
    window.setDelegate(Some(ProtocolObject::from_ref(&*window_delegate)));

    STATE.with(|state| {
        *state.borrow_mut() = Some(State {
            window: window.clone(),
            _window_delegate: window_delegate,
            split: split.clone(),
            _split_delegate: split_delegate,
            sidebar_pane: sidebar_pane.clone(),
            overview: None,
            overview_return: None,
            overview_tab: None,
            overview_button,
            tab_bar,
            content: content.clone(),
            tabs: Tabs::default(),
        })
    });

    sync_appearance();
    if !habits::SIDEBAR_VISIBLE {
        sidebar_panel::toggle();
    }

    window.center();
    window.makeKeyAndOrderFront(None);

    sidebar_panel::set_repos(sidebar::repos());
    quota_panel::start();
    if let Some(first) = sidebar_panel::first_row() {
        sidebar_panel::select(&first.0, &first.1);
    }
}

fn hex(value: &str) -> Retained<NSColor> {
    let rgb = u32::from_str_radix(value, 16).unwrap_or(0);
    NSColor::colorWithSRGBRed_green_blue_alpha(
        ((rgb >> 16) & 0xff) as f64 / 255.0,
        ((rgb >> 8) & 0xff) as f64 / 255.0,
        (rgb & 0xff) as f64 / 255.0,
        1.0,
    )
}

pub(crate) fn new_current_tab() {
    dismiss_overview(false);
    let target = STATE
        .with(|state| {
            let state = state.borrow();
            let state = state.as_ref()?;
            let workspace = state.tabs.current()?.to_owned();
            let name = state
                .tabs
                .visible()
                .next()
                .map(|tab| tab.name.clone())
                .unwrap_or_else(|| leaf_name(&workspace));
            Some((workspace, name))
        })
        .or_else(sidebar_panel::first_row);
    if let Some((path, name)) = target {
        new_tab(&path, &name, None);
    }
}

fn leaf_name(path: &str) -> String {
    PathBuf::from(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
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
    let host = url.host()?.to_string();
    match url.scheme()?.to_string().to_ascii_lowercase().as_str() {
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
        Entry::Run { cwd, name, input } => {
            if !confirm("Run this command in Combe?", &input, "Run") {
                return;
            }
            let cwd = cwd
                .map(|cwd| cwd.to_string_lossy().into_owned())
                .or_else(|| {
                    STATE.with(|state| state.borrow().as_ref()?.tabs.current().map(str::to_owned))
                })
                .unwrap_or_else(|| std::env::var("HOME").unwrap_or_else(|_| "/".to_owned()));
            new_tab(&cwd, &name, Some(&input));
        }
    }
    reveal_window();
}

fn open_worktree(path: &str, name: &str) {
    let existing = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut()?;
        state.tabs.enter(path)
    });
    match existing {
        Some(id) => activate_tab(id),
        None => new_tab(path, name, None),
    }
}

fn new_tab(path: &str, name: &str, input: Option<&str>) {
    let mtm = MainThreadMarker::new().expect("main thread");
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        let root = split::root(mtm, state.content.bounds(), path, input);
        state.content.addSubview(&root);
        state.tabs.push(path.to_owned(), name.to_owned(), root);
    });
    sync_tabs();
    sidebar_panel::refresh();
    focus_active();
}

fn activate_tab(id: u64) {
    dismiss_overview(false);
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        state.tabs.set_active(id);
    });
    sync_tabs();
    sidebar_panel::rebuild();
    let guides = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .and_then(|state| state.tabs.active())
            .map(|tab| split::surfaces(&tab.root))
            .unwrap_or_default()
    });
    if guides
        .iter()
        .filter(|view| !view.isHiddenOrHasHiddenAncestor())
        .count()
        > 1
    {
        for view in &guides {
            view.guide_attention();
        }
    }
    for view in guides {
        if view.needs_attention() {
            view.set_attention(false);
            crate::notification::acknowledge(view.notification_id());
        }
    }
    focus_active();
}

fn request_close_tab(id: u64) {
    dismiss_overview(false);
    let plan = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let tab = state.tabs.get(id)?;
        let last = state.tabs.siblings(id) <= 1;
        let current = state.tabs.current() == Some(tab.workspace.as_str());
        let other = if last && current {
            state.tabs.other(&tab.workspace)
        } else {
            None
        };
        Some((last, current, other))
    });
    let Some((last, current, other)) = plan else {
        return;
    };
    if last && current && other.is_none() {
        close_window();
        return;
    }
    if !confirm_close_tab(id) {
        return;
    }
    close_tab(id);
    if let Some(other) = other {
        activate_tab(other);
    } else if last {
        sidebar_panel::rebuild();
    }
}

fn confirm_close_tab(id: u64) -> bool {
    let running = STATE.with(|state| {
        let state = state.borrow();
        let Some(tab) = state.as_ref().and_then(|state| state.tabs.get(id)) else {
            return false;
        };
        split::surfaces(&tab.root)
            .iter()
            .any(|view| view.needs_confirm_quit())
    });
    !running
        || confirm(
            "Close this tab?",
            "A process is still running in one of its panes.",
            "Close",
        )
}

fn confirm_quit() -> bool {
    !ghostty::needs_confirm_quit()
        || confirm(
            "Quit Combe?",
            "A process is still running in one of the terminals.",
            "Quit",
        )
}

fn confirm(message: &str, informative: &str, action: &str) -> bool {
    let mtm = MainThreadMarker::new().expect("main thread");
    let alert = NSAlert::new(mtm);
    alert.setAlertStyle(NSAlertStyle::Warning);
    alert.setMessageText(&NSString::from_str(message));
    alert.setInformativeText(&NSString::from_str(informative));
    alert.addButtonWithTitle(&NSString::from_str(action));
    alert.addButtonWithTitle(&NSString::from_str("Cancel"));
    alert.runModal() == NSAlertFirstButtonReturn
}

fn reveal_window() {
    STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else { return };
        if state.window.isMiniaturized() {
            state.window.deminiaturize(None);
        }
        state.window.makeKeyAndOrderFront(None);
    });
}

fn close_window() {
    let window = STATE.with(|state| state.borrow().as_ref().map(|state| state.window.clone()));
    if let Some(window) = window {
        window.performClose(None);
    }
}

pub(crate) fn close_all_windows() {
    let mtm = MainThreadMarker::new().expect("main thread");
    let app = NSApplication::sharedApplication(mtm);
    let listed = app.windows();
    let windows: Vec<Retained<NSWindow>> = (0..listed.count())
        .map(|index| listed.objectAtIndex(index))
        .collect();
    for window in windows {
        if window.styleMask().contains(NSWindowStyleMask::Titled) {
            window.performClose(None);
        }
    }
}

fn close_tab(id: u64) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        let Some(tab) = state.tabs.remove(id) else {
            return;
        };
        for view in split::surfaces(&tab.root) {
            view.close();
        }
        tab.root.removeFromSuperview();
    });
    sync_tabs();
    focus_active();
}

pub(crate) fn toggle_split_zoom() {
    let Some(view) = focused_surface() else {
        return;
    };
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        let Some(id) = state.tabs.active_id() else {
            return;
        };
        let Some(tab) = state.tabs.get_mut(id) else {
            return;
        };
        if let Some(zoom) = tab.zoom.take() {
            zoom.restore();
        } else {
            tab.zoom = split::Zoom::new(&tab.root, &view);
        }
    });
    sync_tabs();
    if let Some(window) = view.window() {
        window.makeFirstResponder(Some(&*view));
    }
}

fn restore_zoom(view: &SurfaceView) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        let id = state
            .tabs
            .items()
            .iter()
            .find(|tab| {
                split::surfaces(&tab.root)
                    .iter()
                    .any(|leaf| std::ptr::eq(&**leaf, view))
            })
            .map(|tab| tab.id);
        if let Some(tab) = id.and_then(|id| state.tabs.get_mut(id))
            && let Some(zoom) = tab.zoom.take()
        {
            zoom.restore();
        }
    });
    sync_tabs();
}

pub(crate) fn close_focused() {
    let Some(view) = focused_surface() else {
        return;
    };
    let active = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let tab = state.tabs.active()?;
        Some((tab.id, split::surfaces(&tab.root).len()))
    });
    let Some((id, leaves)) = active else { return };
    if leaves <= 1 {
        request_close_tab(id);
        return;
    }
    if view.needs_confirm_quit()
        && !confirm(
            "Close this pane?",
            "A process is still running in it.",
            "Close",
        )
    {
        return;
    }
    restore_zoom(&view);
    split::close(&view);
    focus_active();
}

pub(crate) fn divide(vertical: bool) {
    let mtm = MainThreadMarker::new().expect("main thread");
    let Some(view) = focused_surface() else {
        return;
    };
    restore_zoom(&view);
    let cwd = view.cwd();
    let Some(fresh) = split::divide(mtm, &view, vertical, &cwd) else {
        return;
    };
    if let Some(window) = fresh.window() {
        window.makeFirstResponder(Some(&*fresh));
    }
}

pub(crate) fn move_pane_to_new_tab() {
    let mtm = MainThreadMarker::new().expect("main thread");
    let Some(view) = focused_surface() else {
        return;
    };
    let source = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let tab = state.tabs.active()?;
        (split::surfaces(&tab.root).len() > 1).then(|| (tab.workspace.clone(), tab.name.clone()))
    });
    let Some((workspace, name)) = source else {
        return;
    };
    restore_zoom(&view);
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        let root = split::adopt(mtm, state.content.bounds(), &view);
        state.content.addSubview(&root);
        state.tabs.push(workspace, name, root);
    });
    sync_tabs();
    focus_active();
}

pub fn goto_tab(target: TabTarget) -> bool {
    let next = STATE.with(|state| {
        let state = state.borrow();
        let tabs = &state.as_ref()?.tabs;
        match target {
            TabTarget::Previous => tabs.step(-1),
            TabTarget::Next => tabs.step(1),
            TabTarget::Last => tabs.visible().last().map(|tab| tab.id),
            TabTarget::Index(index) => tabs.visible().nth(index.checked_sub(1)?).map(|tab| tab.id),
        }
    });
    let Some(id) = next else { return false };
    activate_tab(id);
    true
}

pub(crate) fn surface_action(action: &str) {
    if let Some(view) = focused_surface() {
        view.binding_action(action);
    }
}

pub(crate) fn clipboard_action(action: &str, native: objc2::runtime::Sel) {
    let editing = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .and_then(|state| state.window.firstResponder())
            .is_some_and(|responder| responder.isKindOfClass(NSText::class()))
    });
    if editing {
        let mtm = MainThreadMarker::new().expect("main thread");
        let app = NSApplication::sharedApplication(mtm);
        unsafe { app.sendAction_to_from(native, None, None) };
        return;
    }
    surface_action(action);
}

pub(crate) fn focus_split(target: split::Target) {
    if let Some(view) = focused_surface() {
        goto_split(&view, target);
    }
}

pub fn goto_split(view: &SurfaceView, target: split::Target) -> bool {
    restore_zoom(view);
    let found = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let tab = state.tabs.items().iter().find(|tab| {
            split::surfaces(&tab.root)
                .iter()
                .any(|leaf| std::ptr::eq(&**leaf, view))
        })?;
        let next = split::target(&tab.root, view, target);
        Some(next)
    });
    let Some(next) = found else { return false };
    if let Some(next) = next
        && let Some(window) = next.window()
    {
        window.makeFirstResponder(Some(&*next));
    }
    true
}

fn responder_surface(state: &State) -> Option<Retained<SurfaceView>> {
    let responder = state.window.firstResponder()?;
    let object: &AnyObject = responder.as_ref();
    let matches: bool = unsafe { msg_send![object, isKindOfClass: SurfaceView::class()] };
    if !matches {
        return None;
    }
    let raw = Retained::into_raw(responder) as *mut SurfaceView;
    unsafe { Retained::from_raw(raw) }
}

fn focused_surface() -> Option<Retained<SurfaceView>> {
    STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        if let Some(view) = responder_surface(state) {
            return Some(view);
        }
        state.tabs.active()?.focused_surface()
    })
}

pub(crate) fn notification_source(id: &str) -> Option<(Retained<SurfaceView>, String)> {
    STATE.with(|state| {
        let state = state.try_borrow().ok()?;
        state.as_ref()?.tabs.items().iter().find_map(|tab| {
            split::surfaces(&tab.root)
                .into_iter()
                .find(|view| view.notification_id() == id)
                .map(|view| (view, tab.name.clone()))
        })
    })
}

pub(crate) fn is_observed(view: &SurfaceView) -> bool {
    let mtm = MainThreadMarker::new().expect("main thread");
    if !NSApplication::sharedApplication(mtm).isActive() || view.isHiddenOrHasHiddenAncestor() {
        return false;
    }
    STATE.with(|state| {
        let Ok(state) = state.try_borrow() else {
            return false;
        };
        state.as_ref().is_some_and(|state| {
            state.window.isKeyWindow()
                && state.window.isVisible()
                && !state.window.isMiniaturized()
                && state.overview.is_none()
                && state
                    .tabs
                    .active()
                    .and_then(|tab| tab.focused_surface())
                    .is_some_and(|focused| focused.notification_id() == view.notification_id())
        })
    })
}

pub(crate) fn focus_notification(id: &str) {
    let target = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut()?;
        let (tab_id, view) = state.tabs.items().iter().find_map(|tab| {
            split::surfaces(&tab.root)
                .into_iter()
                .find(|view| view.notification_id() == id)
                .map(|view| (tab.id, view))
        })?;
        let tab = state.tabs.get_mut(tab_id)?;
        if tab
            .zoom
            .as_ref()
            .is_none_or(|zoom| zoom.surface.notification_id() == id)
        {
            tab.focused = Some(view);
        }
        Some(tab_id)
    });
    if let Some(tab) = target {
        activate_tab(tab);
        reveal_window();
        NSApplication::sharedApplication(MainThreadMarker::new().expect("main thread")).activate();
        refresh_attention();
    }
}

pub(crate) fn refresh_attention() {
    if !ATTENTION_QUEUED.replace(true) {
        ghostty::on_main(attention_on_main);
    }
}

unsafe extern "C" fn attention_on_main(_: *mut c_void) {
    ATTENTION_QUEUED.set(false);
    let surfaces = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| {
                state
                    .tabs
                    .items()
                    .iter()
                    .flat_map(|tab| split::surfaces(&tab.root))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });
    for view in &surfaces {
        if view.needs_attention() && is_observed(view) {
            view.set_attention(false);
            crate::notification::acknowledge(view.notification_id());
        }
    }
    let workspaces = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let tabs: HashSet<_> = state
            .tabs
            .items()
            .iter()
            .filter(|tab| {
                split::surfaces(&tab.root)
                    .iter()
                    .any(|view| view.needs_attention())
            })
            .map(|tab| tab.id)
            .collect();
        state.tab_bar.set_attention(&tabs);
        Some(
            state
                .tabs
                .items()
                .iter()
                .filter(|tab| tabs.contains(&tab.id))
                .map(|tab| tab.workspace.clone())
                .collect(),
        )
    });
    if let Some(workspaces) = workspaces {
        sidebar_panel::set_attention(workspaces);
    }
}

pub fn refresh_labels() {
    STATE.with(|state| {
        let Ok(mut state) = state.try_borrow_mut() else {
            return;
        };
        let Some(state) = state.as_mut() else {
            return;
        };
        let focused = responder_surface(state);
        let updates: Vec<_> = state
            .tabs
            .items()
            .iter()
            .map(|tab| {
                let leaves = split::surfaces(&tab.root);
                let view = focused
                    .as_ref()
                    .filter(|view| {
                        leaves
                            .iter()
                            .any(|leaf| std::ptr::eq(&**leaf, &***view as *const SurfaceView))
                    })
                    .cloned()
                    .or_else(|| tab.focused_surface());
                let label = view
                    .as_ref()
                    .and_then(|view| view.title())
                    .unwrap_or_else(|| tab.name.clone());
                (tab.id, label, view)
            })
            .collect();
        for (id, label, view) in updates {
            let tab = state.tabs.get_mut(id).expect("existing tab");
            if tab.label != label {
                tab.label = label;
                state.tab_bar.set_label(id, &tab.label);
            }
            tab.focused = view;
        }
    });
}

pub(crate) fn toggle_overview() {
    let open = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .is_some_and(|state| state.overview.is_some())
    });
    if open {
        dismiss_overview(true);
        return;
    }
    let mtm = MainThreadMarker::new().expect("main thread");
    let mounted = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut()?;
        let active = state.tabs.active_id()?;
        let tabs: Vec<_> = state.tabs.visible().collect();
        let overview = Overview::new(mtm, state.content.bounds(), &tabs, active, activate_tab);
        state.overview_return = state.window.firstResponder();
        state.overview_tab = Some(active);
        Overview::transition(&state.content);
        state.content.addSubview(&overview);
        state.overview = Some(overview.clone());
        if let Some(button) = state.overview_button.as_ref() {
            button.setAccessibilityExpanded(true);
        }
        Some((state.window.clone(), overview))
    });
    if let Some((window, overview)) = mounted {
        window.makeFirstResponder(Some(&*overview));
    }
}

pub(crate) fn dismiss_overview(restore: bool) {
    let removed = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut()?;
        let view = state.overview.take()?;
        state.overview_tab = None;
        if let Some(button) = state.overview_button.as_ref() {
            button.setAccessibilityExpanded(false);
        }
        Some((state.window.clone(), view, state.overview_return.take()))
    });
    if let Some((window, view, responder)) = removed {
        if let Some(parent) = unsafe { view.superview() } {
            Overview::transition(&parent);
        }
        view.removeFromSuperview();
        if restore {
            if let Some(responder) = responder {
                if !window.makeFirstResponder(Some(&*responder)) {
                    focus_active();
                }
            } else {
                focus_active();
            }
        }
    }
}

fn handle_overview_event(event: &NSEvent) -> bool {
    let active = STATE.with(|state| state.borrow().as_ref().and_then(|state| state.overview_tab));
    let Some(active) = active else { return false };
    if event.r#type() != NSEventType::KeyDown {
        return false;
    }
    if event
        .modifierFlags()
        .contains(NSEventModifierFlags::Command)
    {
        let flags = event.modifierFlags();
        let toggle = flags.contains(NSEventModifierFlags::Shift)
            && !flags.intersects(NSEventModifierFlags::Option.union(NSEventModifierFlags::Control))
            && event
                .charactersIgnoringModifiers()
                .is_some_and(|key| matches!(key.to_string().as_str(), "\\" | "|"));
        if !toggle {
            dismiss_overview(true);
        }
        return false;
    }
    match event.keyCode() {
        53 => dismiss_overview(true),
        36 | 76 => {
            let card = STATE.with(|state| {
                let state = state.borrow();
                let state = state.as_ref()?;
                let responder = state.window.firstResponder()?;
                let object: &AnyObject = responder.as_ref();
                let is_card: bool = unsafe { msg_send![object, isKindOfClass: ClickView::class()] };
                if !is_card {
                    return None;
                }
                let overview = state.overview.as_ref()?;
                let inside: bool = unsafe { msg_send![object, isDescendantOf: &**overview] };
                inside.then_some(responder)
            });
            if let Some(card) = card {
                let _: bool = unsafe { msg_send![&*card, accessibilityPerformPress] };
            } else {
                activate_tab(active);
            }
        }
        48 => return false,
        _ => {}
    }
    true
}

fn focus_active() {
    if STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .is_some_and(|state| state.overview.is_some())
    }) {
        return;
    }
    let Some(view) = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        state.tabs.active()?.focused_surface()
    }) else {
        return;
    };
    if let Some(window) = view.window() {
        window.makeFirstResponder(Some(&*view));
    }
}

fn sync_tabs() {
    let sessions = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;

        let active = state.tabs.active_id();
        if let Some(button) = state.overview_button.as_ref() {
            button.setEnabled(active.is_some());
        }
        for tab in state.tabs.items() {
            let hidden = Some(tab.id) != active;
            tab.root.setHidden(hidden);
            for view in split::surfaces(&tab.root) {
                view.set_occluded(hidden || view.isHiddenOrHasHiddenAncestor());
            }
        }

        state.tab_bar.update(
            state.tabs.visible().map(|tab| (tab.id, tab.label.as_str())),
            active,
        );
        Some((
            state.tabs.current().map(str::to_owned),
            state
                .tabs
                .items()
                .iter()
                .map(|tab| tab.workspace.clone())
                .collect(),
        ))
    });
    if let Some((current, opened)) = sessions {
        sidebar_panel::set_sessions(current, opened);
    }
    refresh_attention();
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeChromeView"]
    #[ivars = ()]
    struct ChromeView;

    impl ChromeView {
        #[unsafe(method(viewDidChangeEffectiveAppearance))]
        fn view_did_change_effective_appearance(&self) {
            let _: () = unsafe { msg_send![super(self), viewDidChangeEffectiveAppearance] };
            ghostty::on_main(appearance_on_main);
        }

        #[unsafe(method(layout))]
        fn layout(&self) {
            let _: () = unsafe { msg_send![super(self), layout] };
            layout_chrome();
        }
    }
);

define_class!(
    #[unsafe(super(NSSplitView))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeSplitView"]
    #[ivars = ()]
    struct SplitView;

    impl SplitView {
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let _: () = unsafe { msg_send![super(self), mouseDown: event] };
        }

        #[unsafe(method(dividerThickness))]
        fn divider_thickness(&self) -> CGFloat { 0.0 }

        #[unsafe(method(drawDividerInRect:))]
        fn draw_divider(&self, _rect: NSRect) {}

    }
);

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeSplitDelegate"]
    #[ivars = ()]
    struct SplitDelegate;

    unsafe impl NSObjectProtocol for SplitDelegate {}

    unsafe impl NSSplitViewDelegate for SplitDelegate {
        #[unsafe(method(splitView:canCollapseSubview:))]
        fn can_collapse(&self, split: &NSSplitView, subview: &NSView) -> bool {
            is_sidebar(split, subview)
        }

        #[unsafe(method(splitView:shouldAdjustSizeOfSubview:))]
        fn should_adjust(&self, split: &NSSplitView, subview: &NSView) -> bool {
            !is_sidebar(split, subview)
        }

        #[unsafe(method(splitView:constrainMinCoordinate:ofSubviewAt:))]
        fn constrain_min(
            &self,
            _split: &NSSplitView,
            _proposed: CGFloat,
            _index: NSInteger,
        ) -> CGFloat {
            sidebar_panel::MIN_WIDTH + 2.0 * INSET
        }

        #[unsafe(method(splitView:constrainMaxCoordinate:ofSubviewAt:))]
        fn constrain_max(
            &self,
            _split: &NSSplitView,
            _proposed: CGFloat,
            _index: NSInteger,
        ) -> CGFloat {
            sidebar_panel::MAX_WIDTH + 2.0 * INSET
        }

        #[unsafe(method(splitViewDidResizeSubviews:))]
        fn did_resize(&self, _notification: &NSNotification) {
            let Some(width) = STATE.with(|state| {
                let state = state.borrow();
                let state = state.as_ref()?;
                Some(
                    sidebar_open(state)
                        .then(|| state.sidebar_pane.frame().size.width - 2.0 * INSET),
                )
            }) else {
                return;
            };
            sidebar_panel::resized(width);
            layout_chrome();
        }

        #[unsafe(method(splitView:effectiveRect:forDrawnRect:ofDividerAtIndex:))]
        fn divider_grab(
            &self,
            _split: &NSSplitView,
            _proposed: NSRect,
            drawn: NSRect,
            index: NSInteger,
        ) -> NSRect {
            if index != 0 {
                return NSRect::default();
            }
            NSRect::new(
                NSPoint::new(drawn.origin.x - INSET, drawn.origin.y),
                NSSize::new(drawn.size.width + DIVIDER_GRAB, drawn.size.height),
            )
        }
    }
);

fn sidebar_open(state: &State) -> bool {
    !state.split.isSubviewCollapsed(&state.sidebar_pane)
}

fn is_sidebar(split: &NSSplitView, subview: &NSView) -> bool {
    split
        .subviews()
        .firstObject()
        .is_some_and(|first| std::ptr::eq(&*first as *const NSView, subview as *const NSView))
}

fn leading_inset(window: &NSWindow) -> f64 {
    if window.styleMask().contains(NSWindowStyleMask::FullScreen) {
        FULLSCREEN_INSET
    } else {
        TRAFFIC_INSET
    }
}

unsafe extern "C" fn appearance_on_main(_: *mut c_void) {
    sync_appearance();
}

pub(crate) fn sync_appearance() {
    let Some((window, surfaces)) = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let surfaces: Vec<_> = state
            .tabs
            .items()
            .iter()
            .flat_map(|tab| split::surfaces(&tab.root))
            .collect();
        Some((state.window.clone(), surfaces))
    }) else {
        return;
    };
    let names = unsafe { NSArray::from_slice(&[NSAppearanceNameDarkAqua, NSAppearanceNameAqua]) };
    let dark = window
        .effectiveAppearance()
        .bestMatchFromAppearancesWithNames(&names)
        .is_some_and(|name| &*name == unsafe { NSAppearanceNameDarkAqua });
    if let Some(layer) = window.contentView().and_then(|view| view.layer()) {
        let color = hex(habits::background(dark)).CGColor();
        let _: () = unsafe { msg_send![&*layer, setBackgroundColor: &*color] };
    }
    STATE.with(|state| {
        if let Some(overview) = state
            .borrow()
            .as_ref()
            .and_then(|state| state.overview.as_ref())
        {
            overview.setNeedsDisplay(true);
        }
    });
    window.invalidateShadow();
    ghostty::set_appearance(dark);
    for surface in surfaces {
        surface.sync_appearance();
    }
}

fn deactivate_chrome() {
    sidebar_panel::deactivate();
    quota_panel::deactivate();
}

fn sidebar_changed(animated: bool) {
    let Some((split, pane)) = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| (state.split.clone(), state.sidebar_pane.clone()))
    }) else {
        return;
    };
    let pinned = sidebar_panel::pinned();
    let remembered = sidebar_panel::width();
    ANIMATE_CHROME.set(animated);
    if pane.isHidden() == pinned {
        pane.setHidden(!pinned);
        split.adjustSubviews();
        if pinned {
            split.setPosition_ofDividerAtIndex(remembered + 2.0 * INSET, 0);
        }
    }
    layout_chrome();
    ANIMATE_CHROME.set(false);
}

fn layout_chrome() {
    let animate = ANIMATE_CHROME.get();
    if animate {
        NSAnimationContext::beginGrouping();
        let reduce = NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion();
        let context = NSAnimationContext::currentContext();
        context.setDuration(if reduce { 0.0 } else { 0.38 });
        context.setAllowsImplicitAnimation(true);
        context.setTimingFunction(Some(&CAMediaTimingFunction::functionWithControlPoints(
            0.22, 0.8, 0.2, 1.0,
        )));
    }
    STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else { return };
        let Some(root) = state.window.contentView() else {
            return;
        };
        let size = root.frame().size;
        if let Some(layer) = root.layer() {
            let radius = if state
                .window
                .styleMask()
                .contains(NSWindowStyleMask::FullScreen)
            {
                0.0
            } else {
                WINDOW_RADIUS
            };
            let current: f64 = unsafe { msg_send![&*layer, cornerRadius] };
            if current != radius {
                let _: () = unsafe { msg_send![&*layer, setCornerRadius: radius] };
                state.window.invalidateShadow();
            }
        }
        let inset = leading_inset(&state.window);
        let pinned = sidebar_panel::pinned();
        let edge = sidebar_panel::layout(size, inset, animate);
        state.tab_bar.set_frame(NSRect::new(
            NSPoint::new(edge + INSET, size.height - TOP_BAR_HEIGHT),
            NSSize::new(
                (size.width - edge - 2.0 * INSET - 48.0).max(0.0),
                TOP_BAR_HEIGHT,
            ),
        ));
        if let Some(button) = state.overview_button.as_ref() {
            button.setFrame(NSRect::new(
                NSPoint::new(size.width - INSET - 36.0, size.height - INSET - 36.0),
                NSSize::new(36.0, 36.0),
            ));
        }
        if let Some(right) = unsafe { state.content.superview() } {
            let right_size = right.frame().size;
            let terminal_inset = if pinned { 0.0 } else { INSET };
            let status_h = quota_panel::layout(right_size.width, terminal_inset);
            state.content.setFrame(NSRect::new(
                NSPoint::new(terminal_inset, status_h.max(INSET)),
                NSSize::new(
                    (right_size.width - terminal_inset - INSET).max(0.0),
                    (right_size.height - TOP_BAR_HEIGHT - status_h.max(INSET)).max(0.0),
                ),
            ));
        }
        if !state
            .window
            .styleMask()
            .contains(NSWindowStyleMask::FullScreen)
        {
            for (index, kind) in [
                NSWindowButton::CloseButton,
                NSWindowButton::MiniaturizeButton,
                NSWindowButton::ZoomButton,
            ]
            .into_iter()
            .enumerate()
            {
                if let Some(button) = state.window.standardWindowButton(kind)
                    && let Some(parent) = unsafe { button.superview() }
                {
                    let origin = parent.convertPoint_fromView(
                        NSPoint::new(
                            20.0 + index as f64 * 20.0,
                            size.height - TOP_BAR_HEIGHT / 2.0 - button.frame().size.height / 2.0,
                        ),
                        Some(&root),
                    );
                    button.setFrameOrigin(origin);
                }
            }
        }
    });
    if animate {
        NSAnimationContext::endGrouping();
    }
}

thread_local! {
    static PENDING: RefCell<Vec<Retained<SurfaceView>>> = const { RefCell::new(Vec::new()) };
}

pub fn queue_close(view: Retained<SurfaceView>) {
    PENDING.with(|pending| pending.borrow_mut().push(view));
    ghostty::on_main(drain_pending);
}

unsafe extern "C" fn drain_pending(_: *mut c_void) {
    let pending: Vec<Retained<SurfaceView>> =
        PENDING.with(|pending| pending.borrow_mut().drain(..).collect());
    for view in pending {
        close_leaf(&view);
    }
}

fn close_leaf(view: &SurfaceView) {
    dismiss_overview(false);
    restore_zoom(view);
    let owner = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        state
            .tabs
            .items()
            .iter()
            .find(|tab| {
                split::surfaces(&tab.root)
                    .iter()
                    .any(|leaf| std::ptr::eq(&**leaf, view))
            })
            .map(|tab| (tab.id, split::surfaces(&tab.root).len()))
    });
    let Some((id, leaves)) = owner else { return };
    if leaves <= 1 {
        request_close_tab(id);
        return;
    }
    split::close(view);
    focus_active();
}

fn quota_window_live() -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    let active = NSApplication::sharedApplication(mtm).isActive();
    active
        && STATE.with(|state| {
            let state = state.borrow();
            state
                .as_ref()
                .is_some_and(|state| state.window.isKeyWindow() && !state.window.isMiniaturized())
        })
}
