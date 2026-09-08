use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::ffi::c_void;
use std::path::PathBuf;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{ClassType, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAccessibility, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSApplication, NSApplicationDelegate, NSAutoresizingMaskOptions, NSBackingStoreType, NSButton,
    NSColor, NSEventModifierFlags, NSFont, NSImage, NSMenu, NSMenuItem, NSOpenPanel, NSScrollView,
    NSSplitView, NSSplitViewDelegate, NSSplitViewDividerStyle, NSTextField, NSView, NSWindow,
    NSWindowStyleMask, NSWindowTitleVisibility,
};
use objc2_core_foundation::CGFloat;
use objc2_foundation::{
    MainThreadMarker, NSArray, NSInteger, NSNotification, NSPoint, NSRect, NSSize, NSString,
};

use crate::chrome_view::{ClickView, FlippedView};
use crate::ghostty;
use crate::habits;
use crate::quota_panel;
use crate::sidebar;
use crate::split;
use crate::surface::SurfaceView;
use crate::tabs::Tabs;

const ROW_HEIGHT: f64 = 28.0;
const HEADER_HEIGHT: f64 = 30.0;
const TOP_BAR_HEIGHT: f64 = 40.0;
const TAB_HEIGHT: f64 = 28.0;
const TAB_WIDTH: f64 = 168.0;
const TRAFFIC_INSET: f64 = 78.0;
const FULLSCREEN_INSET: f64 = 8.0;
const TOGGLE_WIDTH: f64 = 26.0;
const TOGGLE_HEIGHT: f64 = 22.0;
const TOGGLE_TRAIL: f64 = 8.0;
const SIDEBAR_MIN_WIDTH: f64 = 160.0;
const SIDEBAR_MAX_WIDTH: f64 = 420.0;
const DIVIDER_GRAB: f64 = 4.0;

const FILL: NSAutoresizingMaskOptions = NSAutoresizingMaskOptions(
    NSAutoresizingMaskOptions::ViewWidthSizable.0 | NSAutoresizingMaskOptions::ViewHeightSizable.0,
);

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

struct State {
    window: Retained<NSWindow>,
    split: Retained<SplitView>,
    _split_delegate: Retained<SplitDelegate>,
    sidebar_pane: Retained<NSView>,
    sidebar: Retained<NSScrollView>,
    toggle: Option<Retained<NSButton>>,
    add_repo: Option<Retained<NSButton>>,
    tab_bar: Retained<NSView>,
    content: Retained<NSView>,
    tabs: Tabs,
    repos: Vec<sidebar::Repo>,
    collapsed_repos: HashSet<PathBuf>,
    sidebar_width: f64,
    sidebar_height: Cell<f64>,
}

enum Click {
    Open(String, String),
    SelectTab(u64),
    CloseTab(u64),
    NewTab,
    AddRepo,
    ToggleRepo(PathBuf),
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeCommands"]
    #[ivars = ()]
    struct Commands;

    impl Commands {
        #[unsafe(method(newTab:))]
        fn new_tab(&self, _sender: Option<&AnyObject>) {
            dispatch(Click::NewTab);
        }

        #[unsafe(method(closeFocused:))]
        fn close_focused(&self, _sender: Option<&AnyObject>) {
            close_focused();
        }

        #[unsafe(method(splitRight:))]
        fn split_right(&self, _sender: Option<&AnyObject>) {
            divide(true);
        }

        #[unsafe(method(splitDown:))]
        fn split_down(&self, _sender: Option<&AnyObject>) {
            divide(false);
        }

        #[unsafe(method(toggleSplitZoom:))]
        fn toggle_split_zoom(&self, _sender: Option<&AnyObject>) {
            toggle_split_zoom();
        }

        #[unsafe(method(previousTab:))]
        fn previous_tab(&self, _sender: Option<&AnyObject>) {
            step_tab(-1);
        }

        #[unsafe(method(nextTab:))]
        fn next_tab(&self, _sender: Option<&AnyObject>) {
            step_tab(1);
        }

        #[unsafe(method(toggleSidebar:))]
        fn toggle_sidebar(&self, _sender: Option<&AnyObject>) {
            toggle_sidebar();
        }

        #[unsafe(method(copyText:))]
        fn copy_text(&self, _sender: Option<&AnyObject>) {
            if let Some(view) = focused_surface() {
                view.binding_action("copy_to_clipboard");
            }
        }

        #[unsafe(method(pasteText:))]
        fn paste_text(&self, _sender: Option<&AnyObject>) {
            if let Some(view) = focused_surface() {
                view.binding_action("paste_from_clipboard");
            }
        }

        #[unsafe(method(addRepo:))]
        fn add_repo(&self, _sender: Option<&AnyObject>) {
            dispatch(Click::AddRepo);
        }

        #[unsafe(method(closeAllWindows:))]
        fn close_all_windows(&self, _sender: Option<&AnyObject>) {
            close_all_windows();
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
            true
        }

        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn should_handle_reopen(&self, _app: &NSApplication, _has_visible_windows: bool) -> bool {
            reveal_window();
            true
        }

        #[unsafe(method(applicationDidBecomeActive:))]
        fn did_become_active(&self, _notification: &AnyObject) {
            ghostty::set_focus(true);
            refresh_sidebar();
            quota_panel::refresh();
        }

        #[unsafe(method(applicationDidResignActive:))]
        fn did_resign_active(&self, _notification: &AnyObject) {
            ghostty::set_focus(false);
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
    delegate
}

thread_local! {
    static COMMANDS: RefCell<Option<Retained<Commands>>> = const { RefCell::new(None) };
}

fn commands(mtm: MainThreadMarker) -> Retained<Commands> {
    COMMANDS.with(|slot| {
        let mut slot = slot.borrow_mut();
        slot.get_or_insert_with(|| {
            let this = Commands::alloc(mtm).set_ivars(());
            unsafe { msg_send![super(this), init] }
        })
        .clone()
    })
}

pub fn install_menu(mtm: MainThreadMarker, app: &NSApplication) {
    let commands = commands(mtm);

    let menubar = NSMenu::new(mtm);

    let app_item = NSMenuItem::new(mtm);
    menubar.addItem(&app_item);
    let app_menu = NSMenu::new(mtm);
    app_menu.addItem(&item(
        mtm,
        "Hide Combe",
        sel!(hide:),
        None,
        "h",
        NSEventModifierFlags::Command,
    ));
    app_menu.addItem(&item(
        mtm,
        "Hide Others",
        sel!(hideOtherApplications:),
        None,
        "h",
        NSEventModifierFlags::Command.union(NSEventModifierFlags::Option),
    ));
    app_menu.addItem(&item(
        mtm,
        "Show All",
        sel!(unhideAllApplications:),
        None,
        "",
        NSEventModifierFlags::empty(),
    ));
    app_menu.addItem(&NSMenuItem::separatorItem(mtm));
    app_menu.addItem(&item(
        mtm,
        "Quit Combe",
        sel!(terminate:),
        None,
        "q",
        NSEventModifierFlags::Command,
    ));
    app_item.setSubmenu(Some(&app_menu));

    let terminal_item = NSMenuItem::new(mtm);
    menubar.addItem(&terminal_item);
    let terminal_menu = NSMenu::new(mtm);
    terminal_menu.setTitle(&NSString::from_str("Terminal"));
    let entries: [(&str, objc2::runtime::Sel, &str, NSEventModifierFlags); 10] = [
        (
            "Zoom Split",
            sel!(toggleSplitZoom:),
            "\r",
            NSEventModifierFlags::Command.union(NSEventModifierFlags::Shift),
        ),
        ("New Tab", sel!(newTab:), "t", NSEventModifierFlags::Command),
        (
            "Close",
            sel!(closeFocused:),
            "w",
            NSEventModifierFlags::Command,
        ),
        (
            "Split Right",
            sel!(splitRight:),
            "d",
            NSEventModifierFlags::Command,
        ),
        (
            "Split Down",
            sel!(splitDown:),
            "D",
            NSEventModifierFlags::Command.union(NSEventModifierFlags::Shift),
        ),
        (
            "Previous Tab",
            sel!(previousTab:),
            "\u{F702}",
            NSEventModifierFlags::Command.union(NSEventModifierFlags::Option),
        ),
        (
            "Next Tab",
            sel!(nextTab:),
            "\u{F703}",
            NSEventModifierFlags::Command.union(NSEventModifierFlags::Option),
        ),
        (
            "Toggle Sidebar",
            sel!(toggleSidebar:),
            "b",
            NSEventModifierFlags::Command,
        ),
        ("Copy", sel!(copyText:), "c", NSEventModifierFlags::Command),
        (
            "Paste",
            sel!(pasteText:),
            "v",
            NSEventModifierFlags::Command,
        ),
    ];
    for (title, action, key, mask) in entries {
        terminal_menu.addItem(&item(mtm, title, action, Some(&commands), key, mask));
    }
    terminal_item.setSubmenu(Some(&terminal_menu));

    let view_item = NSMenuItem::new(mtm);
    menubar.addItem(&view_item);
    let view_menu = NSMenu::new(mtm);
    view_menu.setTitle(&NSString::from_str("View"));
    view_menu.addItem(&item(
        mtm,
        "Enter Full Screen",
        sel!(toggleFullScreen:),
        None,
        "f",
        NSEventModifierFlags::Command.union(NSEventModifierFlags::Control),
    ));
    view_item.setSubmenu(Some(&view_menu));

    let window_item = NSMenuItem::new(mtm);
    menubar.addItem(&window_item);
    let window_menu = NSMenu::new(mtm);
    window_menu.setTitle(&NSString::from_str("Window"));
    window_menu.addItem(&item(
        mtm,
        "Minimize",
        sel!(performMiniaturize:),
        None,
        "m",
        NSEventModifierFlags::Command,
    ));
    window_menu.addItem(&item(
        mtm,
        "Close All Windows",
        sel!(closeAllWindows:),
        Some(&commands),
        "w",
        NSEventModifierFlags::Command.union(NSEventModifierFlags::Option),
    ));
    window_item.setSubmenu(Some(&window_menu));
    app.setWindowsMenu(Some(&window_menu));

    app.setMainMenu(Some(&menubar));
}

fn item(
    mtm: MainThreadMarker,
    title: &str,
    action: objc2::runtime::Sel,
    target: Option<&Commands>,
    key: &str,
    mask: NSEventModifierFlags,
) -> Retained<NSMenuItem> {
    let entry = NSMenuItem::new(mtm);
    unsafe {
        entry.setTitle(&NSString::from_str(title));
        entry.setAction(Some(action));
        entry.setKeyEquivalent(&NSString::from_str(key));
        entry.setKeyEquivalentModifierMask(mask);
        if let Some(target) = target {
            entry.setTarget(Some(target));
        }
    }
    entry
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
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            frame,
            style,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    window.setTitle(&NSString::from_str("Combe"));
    window.setTitlebarAppearsTransparent(true);
    window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
    unsafe { window.setReleasedWhenClosed(false) };

    let split = SplitView::alloc(mtm).set_ivars(());
    let split: Retained<SplitView> = unsafe { msg_send![super(split), initWithFrame: frame] };
    split.setVertical(true);
    split.setDividerStyle(NSSplitViewDividerStyle::Thin);
    split.setAutoresizingMask(FILL);

    let sidebar_frame = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(habits::SIDEBAR_WIDTH, frame.size.height),
    );
    let sidebar_pane = NSView::initWithFrame(NSView::alloc(mtm), sidebar_frame);
    sidebar_pane.setAutoresizingMask(NSAutoresizingMaskOptions::ViewHeightSizable);

    let sidebar_header = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(
            NSPoint::new(0.0, sidebar_frame.size.height - TOP_BAR_HEIGHT),
            NSSize::new(sidebar_frame.size.width, TOP_BAR_HEIGHT),
        ),
    );
    sidebar_header.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewMinYMargin,
    );
    sidebar_pane.addSubview(&sidebar_header);

    let sidebar_view = NSScrollView::initWithFrame(
        NSScrollView::alloc(mtm),
        NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(
                sidebar_frame.size.width,
                sidebar_frame.size.height - TOP_BAR_HEIGHT,
            ),
        ),
    );
    sidebar_view.setHasVerticalScroller(true);
    sidebar_view.setDrawsBackground(false);
    sidebar_view.setAutoresizingMask(FILL);
    sidebar_pane.addSubview(&sidebar_view);
    split.addSubview(&sidebar_pane);

    let right_frame = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(frame.size.width - habits::SIDEBAR_WIDTH, frame.size.height),
    );
    let right = NSView::initWithFrame(NSView::alloc(mtm), right_frame);
    right.setAutoresizingMask(FILL);

    let tab_bar = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(
            NSPoint::new(0.0, right_frame.size.height - TOP_BAR_HEIGHT),
            NSSize::new(right_frame.size.width, TOP_BAR_HEIGHT),
        ),
    );
    tab_bar.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewMinYMargin,
    );
    right.addSubview(&tab_bar);

    let status_h = quota_panel::mount(
        mtm,
        &right,
        right_frame.size.width,
        quota_window_live,
        layout_chrome,
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
    root.addSubview(&split);
    let toggle = icon_button(
        mtm,
        "sidebar.left",
        sel!(toggleSidebar:),
        NSRect::new(
            NSPoint::new(
                TRAFFIC_INSET,
                frame.size.height - TOP_BAR_HEIGHT + (TOP_BAR_HEIGHT - TOGGLE_HEIGHT) / 2.0,
            ),
            NSSize::new(TOGGLE_WIDTH, TOGGLE_HEIGHT),
        ),
    );
    if let Some(toggle) = &toggle {
        toggle.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMinYMargin);
        root.addSubview(toggle);
    }
    let add_repo = icon_button(mtm, "plus", sel!(addRepo:), NSRect::default());
    if let Some(add_repo) = &add_repo {
        add_repo.setToolTip(Some(&NSString::from_str("Add repo")));
        add_repo.setAccessibilityLabel(Some(&NSString::from_str("Add repo")));
        root.addSubview(add_repo);
    }
    window.setContentView(Some(&root));

    let split_delegate = SplitDelegate::alloc(mtm).set_ivars(());
    let split_delegate: Retained<SplitDelegate> = unsafe { msg_send![super(split_delegate), init] };
    split.setDelegate(Some(ProtocolObject::from_ref(&*split_delegate)));
    split.setPosition_ofDividerAtIndex(habits::SIDEBAR_WIDTH, 0);

    STATE.with(|state| {
        *state.borrow_mut() = Some(State {
            window: window.clone(),
            split: split.clone(),
            _split_delegate: split_delegate,
            sidebar_pane: sidebar_pane.clone(),
            sidebar: sidebar_view.clone(),
            toggle,
            add_repo,
            tab_bar: tab_bar.clone(),
            content: content.clone(),
            tabs: Tabs::default(),
            repos: Vec::new(),
            collapsed_repos: HashSet::new(),
            sidebar_width: habits::SIDEBAR_WIDTH,
            sidebar_height: Cell::new(0.0),
        })
    });

    sync_appearance();
    if !habits::SIDEBAR_VISIBLE {
        toggle_sidebar();
    }

    window.center();
    window.makeKeyAndOrderFront(None);

    refresh_sidebar();
    quota_panel::start();
    if let Some(first) = first_row() {
        dispatch(Click::Open(first.0, first.1));
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

fn icon_button(
    mtm: MainThreadMarker,
    symbol: &str,
    action: objc2::runtime::Sel,
    frame: NSRect,
) -> Option<Retained<NSButton>> {
    let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
        &NSString::from_str(symbol),
        None,
    )?;
    let target = commands(mtm);
    let button = unsafe {
        NSButton::buttonWithImage_target_action(&image, Some(&target), Some(action), mtm)
    };
    button.setFrame(frame);
    button.setBordered(false);
    button.setContentTintColor(Some(&NSColor::secondaryLabelColor()));
    Some(button)
}

fn first_row() -> Option<(String, String)> {
    STATE.with(|state| {
        let state = state.borrow();
        let row = state.as_ref()?.repos.first()?.rows.first()?;
        Some((row.path.to_string_lossy().into_owned(), row.label.clone()))
    })
}

fn dispatch(click: Click) {
    match click {
        Click::Open(path, label) => open_worktree(&path, &label),
        Click::SelectTab(id) => activate_tab(id),
        Click::CloseTab(id) => request_close_tab(id),
        Click::NewTab => {
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
                .or_else(first_row);
            if let Some((path, name)) = target {
                new_tab(&path, &name);
            }
        }
        Click::AddRepo => add_repo(),
        Click::ToggleRepo(path) => {
            STATE.with(|state| {
                let mut state = state.borrow_mut();
                let Some(state) = state.as_mut() else { return };
                if !state.collapsed_repos.remove(&path) {
                    state.collapsed_repos.insert(path);
                }
            });
            rebuild_sidebar();
        }
    }
}

fn leaf_name(path: &str) -> String {
    PathBuf::from(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

fn add_repo() {
    let mtm = MainThreadMarker::new().expect("main thread");
    let panel = NSOpenPanel::openPanel(mtm);
    panel.setCanChooseFiles(false);
    panel.setCanChooseDirectories(true);
    panel.setAllowsMultipleSelection(true);
    panel.setPrompt(Some(&NSString::from_str("Add repo")));
    if panel.runModal() != objc2_app_kit::NSModalResponseOK {
        return;
    }
    let paths: Vec<PathBuf> = panel
        .URLs()
        .iter()
        .filter_map(|url| url.path())
        .map(|path| PathBuf::from(path.to_string()))
        .collect();
    sidebar::add(&paths);
    refresh_sidebar();
}

fn open_worktree(path: &str, name: &str) {
    let existing = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut()?;
        state.tabs.enter(path)
    });
    match existing {
        Some(id) => activate_tab(id),
        None => new_tab(path, name),
    }
}

fn new_tab(path: &str, name: &str) {
    let mtm = MainThreadMarker::new().expect("main thread");
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        let root = split::root(mtm, state.content.bounds(), path);
        state.content.addSubview(&root);
        state.tabs.push(path.to_owned(), name.to_owned(), root);
    });
    sync_tabs();
    refresh_sidebar();
    focus_active();
}

fn activate_tab(id: u64) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        state.tabs.set_active(id);
    });
    sync_tabs();
    rebuild_sidebar();
    focus_active();
}

fn request_close_tab(id: u64) {
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
    if last && current {
        if let Some(other) = other {
            close_tab(id);
            activate_tab(other);
        } else {
            close_window();
        }
        return;
    }
    close_tab(id);
    if last {
        rebuild_sidebar();
    }
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
    STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else { return };
        state.window.performClose(None);
    });
}

fn close_all_windows() {
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

fn toggle_split_zoom() {
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

fn close_focused() {
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
    restore_zoom(&view);
    split::close(&view);
    focus_active();
}

fn divide(vertical: bool) {
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

fn step_tab(delta: isize) {
    let next = STATE.with(|state| state.borrow().as_ref().and_then(|s| s.tabs.step(delta)));
    if let Some(id) = next {
        activate_tab(id);
    }
}

fn toggle_sidebar() {
    let Some((split, pane, open, remembered)) = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        Some((
            state.split.clone(),
            state.sidebar_pane.clone(),
            sidebar_open(state),
            state.sidebar_width,
        ))
    }) else {
        return;
    };

    if open {
        let width = pane
            .frame()
            .size
            .width
            .clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
        STATE.with(|state| {
            if let Some(state) = state.borrow_mut().as_mut() {
                state.sidebar_width = width;
            }
        });
        pane.setHidden(true);
        split.adjustSubviews();
    } else {
        pane.setHidden(false);
        split.adjustSubviews();
        split.setPosition_ofDividerAtIndex(remembered, 0);
    }

    sync_tabs();
    layout_chrome();
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

pub fn refresh_labels() {
    let changed = STATE.with(|state| {
        let Ok(mut state) = state.try_borrow_mut() else {
            return false;
        };
        let Some(state) = state.as_mut() else {
            return false;
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
        updates
            .into_iter()
            .fold(false, |changed, (id, label, view)| {
                let tab = state.tabs.get_mut(id).expect("existing tab");
                let changed = changed || tab.label != label;
                tab.label = label;
                tab.focused = view;
                changed
            })
    });
    if changed {
        sync_tabs();
    }
}

fn focus_active() {
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
    let mtm = MainThreadMarker::new().expect("main thread");
    STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else { return };

        let active = state.tabs.active_id();
        for tab in state.tabs.items() {
            let hidden = Some(tab.id) != active;
            tab.root.setHidden(hidden);
            for view in split::surfaces(&tab.root) {
                view.set_occluded(hidden || view.isHiddenOrHasHiddenAncestor());
            }
        }

        for child in state.tab_bar.subviews() {
            child.removeFromSuperview();
        }

        let y = (TOP_BAR_HEIGHT - TAB_HEIGHT) / 2.0;
        let mut x = 8.0;
        for tab in state.tabs.visible() {
            let frame = NSRect::new(NSPoint::new(x, y), NSSize::new(TAB_WIDTH, TAB_HEIGHT));
            let id = tab.id;
            let view = ClickView::new(mtm, frame, &tab.label, 12.0, 28.0, move || {
                dispatch(Click::SelectTab(id))
            });
            view.dim_when_idle();
            view.set_selected(Some(tab.id) == active);
            state.tab_bar.addSubview(&view);

            let close = ClickView::new(
                mtm,
                NSRect::new(
                    NSPoint::new(x + TAB_WIDTH - 24.0, y),
                    NSSize::new(20.0, TAB_HEIGHT),
                ),
                "\u{00D7}",
                6.0,
                0.0,
                move || dispatch(Click::CloseTab(id)),
            );
            close.dim_when_idle();
            state.tab_bar.addSubview(&close);
            x += TAB_WIDTH + 2.0;
        }

        let plus = ClickView::new(
            mtm,
            NSRect::new(NSPoint::new(x + 4.0, y), NSSize::new(28.0, TAB_HEIGHT)),
            "+",
            10.0,
            0.0,
            || dispatch(Click::NewTab),
        );
        plus.dim_when_idle();
        state.tab_bar.addSubview(&plus);
    });
}

fn refresh_sidebar() {
    let repos = sidebar::repos();
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.repos = repos;
        }
    });
    rebuild_sidebar();
}

fn rebuild_sidebar() {
    let mtm = MainThreadMarker::new().expect("main thread");
    STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else { return };

        let clip = state.sidebar.contentView().frame().size;
        let width = clip.width;
        let mut height = 12.0;
        for repo in &state.repos {
            height += HEADER_HEIGHT;
            if !state.collapsed_repos.contains(&repo.path) {
                height += repo.rows.len() as f64 * ROW_HEIGHT;
            }
        }
        state.sidebar_height.set(height);

        let document = FlippedView::alloc(mtm);
        let document: Retained<FlippedView> = unsafe {
            msg_send![document, initWithFrame: NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(width, height.max(clip.height)),
            )]
        };

        let mut y = 6.0;
        for repo in &state.repos {
            let collapsed = state.collapsed_repos.contains(&repo.path);
            let arrow = if collapsed { "▸" } else { "▾" };
            let path = repo.path.clone();
            let header = ClickView::new(
                mtm,
                NSRect::new(
                    NSPoint::new(8.0, y),
                    NSSize::new(width - 16.0, HEADER_HEIGHT),
                ),
                &format!("{arrow} {}", repo.name),
                2.0,
                8.0,
                move || dispatch(Click::ToggleRepo(path.clone())),
            );
            header.dim_when_idle();
            header.set_font(&NSFont::boldSystemFontOfSize(habits::FONT_SIZE));
            header.setAccessibilityElement(true);
            header.setAccessibilityRole(Some(&NSString::from_str("AXButton")));
            header.setAccessibilityLabel(Some(&NSString::from_str(&repo.name)));
            header.setAccessibilityExpanded(!collapsed);
            header.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
            document.addSubview(&header);
            y += HEADER_HEIGHT;
            if collapsed {
                continue;
            }

            for row in &repo.rows {
                let path = row.path.to_string_lossy().into_owned();
                let frame = NSRect::new(
                    NSPoint::new(8.0, y),
                    NSSize::new(width - 16.0, ROW_HEIGHT - 2.0),
                );
                let opened = state.tabs.opened(&path);
                let target = path.clone();
                let label = row.label.clone();
                let view = ClickView::new(mtm, frame, &row.label, 18.0, 24.0, move || {
                    dispatch(Click::Open(target.clone(), label.clone()))
                });
                view.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
                view.set_selected(state.tabs.current() == Some(path.as_str()));
                view.set_opened(opened);
                view.setAccessibilityElement(true);
                view.setAccessibilityRole(Some(&NSString::from_str("AXButton")));
                let access = if opened {
                    format!("{}, open", row.label)
                } else {
                    format!("{}, not open", row.label)
                };
                view.setAccessibilityLabel(Some(&NSString::from_str(&access)));
                if row.pinned {
                    let star = NSTextField::labelWithString(&NSString::from_str("\u{2605}"), mtm);
                    star.setFrame(NSRect::new(
                        NSPoint::new(
                            frame.size.width - 22.0,
                            (frame.size.height - habits::LINE_HEIGHT) / 2.0,
                        ),
                        NSSize::new(habits::LINE_HEIGHT, habits::LINE_HEIGHT),
                    ));
                    star.setFont(Some(&NSFont::systemFontOfSize(habits::FONT_SIZE)));
                    star.setTextColor(Some(&NSColor::systemYellowColor()));
                    star.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMinXMargin);
                    view.addSubview(&star);
                }
                document.addSubview(&view);
                y += ROW_HEIGHT;
            }
        }

        state.sidebar.setDocumentView(Some(&document));
    });
    layout_chrome();
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
        #[unsafe(method(drawDividerInRect:))]
        fn draw_divider(&self, rect: NSRect) {
            let collapsed = self
                .subviews()
                .firstObject()
                .is_some_and(|first| self.isSubviewCollapsed(&first));
            if collapsed {
                return;
            }
            let _: () = unsafe { msg_send![super(self), drawDividerInRect: rect] };
        }
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
            SIDEBAR_MIN_WIDTH
        }

        #[unsafe(method(splitView:constrainMaxCoordinate:ofSubviewAt:))]
        fn constrain_max(
            &self,
            _split: &NSSplitView,
            _proposed: CGFloat,
            _index: NSInteger,
        ) -> CGFloat {
            SIDEBAR_MAX_WIDTH
        }

        #[unsafe(method(splitViewDidResizeSubviews:))]
        fn did_resize(&self, _notification: &NSNotification) {
            layout_chrome();
        }

        #[unsafe(method(splitView:additionalEffectiveRectOfDividerAtIndex:))]
        fn divider_grab(&self, split: &NSSplitView, index: NSInteger) -> NSRect {
            let empty = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0));
            if index != 0 {
                return empty;
            }
            let Some(sidebar) = split.subviews().firstObject() else {
                return empty;
            };
            NSRect::new(
                NSPoint::new(sidebar.frame().size.width - DIVIDER_GRAB, 0.0),
                NSSize::new(
                    split.dividerThickness() + DIVIDER_GRAB * 2.0,
                    split.frame().size.height,
                ),
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

fn sync_appearance() {
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
    window.setBackgroundColor(Some(&hex(habits::background(dark))));
    ghostty::set_appearance(dark);
    for surface in surfaces {
        surface.sync_appearance();
    }
}

fn layout_chrome() {
    STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else { return };
        let inset = leading_inset(&state.window);
        let open = sidebar_open(state);
        state.split.setNeedsDisplay(true);

        let clip = state.sidebar.contentView().frame().size;
        if let Some(document) = state.sidebar.documentView() {
            document.setFrameSize(NSSize::new(
                clip.width,
                state.sidebar_height.get().max(clip.height),
            ));
        }

        if let Some(toggle) = &state.toggle
            && let Some(root) = unsafe { toggle.superview() }
        {
            let x = if open {
                (state.sidebar_pane.frame().size.width - 2.0 * (TOGGLE_WIDTH + TOGGLE_TRAIL))
                    .max(inset)
            } else {
                inset
            };
            toggle.setFrame(NSRect::new(
                NSPoint::new(
                    x + TOGGLE_WIDTH + TOGGLE_TRAIL,
                    root.frame().size.height - TOP_BAR_HEIGHT
                        + (TOP_BAR_HEIGHT - TOGGLE_HEIGHT) / 2.0,
                ),
                NSSize::new(TOGGLE_WIDTH, TOGGLE_HEIGHT),
            ));
            if let Some(add_repo) = &state.add_repo {
                add_repo.setFrame(NSRect::new(
                    NSPoint::new(x, toggle.frame().origin.y),
                    NSSize::new(TOGGLE_WIDTH, TOGGLE_HEIGHT),
                ));
            }
        }

        if let Some(right) = unsafe { state.tab_bar.superview() } {
            let size = right.frame().size;
            let offset = if open {
                0.0
            } else {
                inset + 2.0 * (TOGGLE_WIDTH + TOGGLE_TRAIL)
            };
            state.tab_bar.setFrame(NSRect::new(
                NSPoint::new(offset, size.height - TOP_BAR_HEIGHT),
                NSSize::new((size.width - offset).max(0.0), TOP_BAR_HEIGHT),
            ));
            let status_h = quota_panel::layout(size.width);
            state.content.setFrame(NSRect::new(
                NSPoint::new(0.0, status_h),
                NSSize::new(
                    size.width,
                    (size.height - TOP_BAR_HEIGHT - status_h).max(0.0),
                ),
            ));
        }
    });
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
