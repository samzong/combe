use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Mutex;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{ClassType, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAccessibility, NSAlert, NSAlertFirstButtonReturn, NSAlertStyle,
    NSAnimatablePropertyContainer, NSAnimationContext, NSAppearance, NSAppearanceCustomization,
    NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSApplication, NSApplicationDelegate,
    NSApplicationTerminateReply, NSAutoresizingMaskOptions, NSBackingStoreType, NSBezierPath,
    NSButton, NSColor, NSEvent, NSEventModifierFlags, NSEventType, NSFont, NSImage, NSImageView,
    NSMenu, NSMenuItem, NSMenuWillSendActionNotification, NSOpenPanel, NSResponder, NSScrollView,
    NSShadow, NSSplitView, NSSplitViewDelegate, NSSplitViewDividerStyle, NSText,
    NSUserInterfaceItemIdentification, NSView, NSViewLayerContentsRedrawPolicy, NSWindow,
    NSWindowButton, NSWindowDelegate, NSWindowStyleMask, NSWindowTitleVisibility, NSWorkspace,
};
use objc2_core_foundation::CGFloat;
use objc2_foundation::{
    MainThreadMarker, NSArray, NSInteger, NSNotification, NSNotificationCenter, NSPoint, NSRect,
    NSSize, NSString, NSTimer,
};
use objc2_quartz_core::CAMediaTimingFunction;

use crate::chrome_view::{self, ClickView};
use crate::ghostty;
use crate::habits;
use crate::overview::Overview;
use crate::quota_panel;
use crate::sidebar;
use crate::split;
use crate::surface::SurfaceView;
use crate::tabs::Tabs;

const ROW_HEIGHT: f64 = 36.0;
const HEADER_HEIGHT: f64 = 30.0;
const TOP_BAR_HEIGHT: f64 = 60.0;
const TAB_HEIGHT: f64 = 36.0;
const TAB_WIDTH: f64 = 180.0;
const TAB_RADIUS: f64 = 18.0;
const TRAFFIC_INSET: f64 = 84.0;
const FULLSCREEN_INSET: f64 = 12.0;
const TOGGLE_WIDTH: f64 = 28.0;
const TOGGLE_HEIGHT: f64 = 28.0;
const INSET: f64 = 12.0;
const WINDOW_RADIUS: f64 = 34.0;
const SIDEBAR_RADIUS: f64 = 18.0;
const SIDEBAR_MIN_WIDTH: f64 = 160.0;
const SIDEBAR_MAX_WIDTH: f64 = 420.0;
const DIVIDER_GRAB: f64 = 4.0;

const FILL: NSAutoresizingMaskOptions = NSAutoresizingMaskOptions(
    NSAutoresizingMaskOptions::ViewWidthSizable.0 | NSAutoresizingMaskOptions::ViewHeightSizable.0,
);

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static SIDEBAR_SCANNING: Cell<bool> = const { Cell::new(false) };
    static SIDEBAR_DIRTY: Cell<bool> = const { Cell::new(false) };
    static SIDEBAR_TIMER: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
    static ANIMATE_CHROME: Cell<bool> = const { Cell::new(false) };
}
static SIDEBAR_INCOMING: Mutex<Option<Vec<sidebar::Repo>>> = Mutex::new(None);

struct State {
    window: Retained<NSWindow>,
    _window_delegate: Retained<WindowDelegate>,
    split: Retained<SplitView>,
    _split_delegate: Retained<SplitDelegate>,
    sidebar_pane: Retained<NSView>,
    sidebar: Retained<NSScrollView>,
    sidebar_glass: Retained<NSView>,
    sidebar_material: Retained<chrome_view::GlassView>,
    workspace_chip: Retained<ClickView>,
    sidebar_mode: Cell<SidebarMode>,
    sidebar_pointer: Cell<u8>,
    sidebar_entered: Cell<bool>,
    sidebar_keyboard: Cell<bool>,
    command_held: Cell<bool>,
    toggle: Option<Retained<NSButton>>,
    add_repo: Option<Retained<NSButton>>,
    overview: Option<Retained<Overview>>,
    overview_return: Option<Retained<NSResponder>>,
    overview_tab: Option<u64>,
    overview_button: Option<Retained<NSButton>>,
    tab_bar: Retained<NSView>,
    tab_scroller: Retained<NSScrollView>,
    content: Retained<NSView>,
    tabs: Tabs,
    repos: Vec<sidebar::Repo>,
    collapsed_repos: HashSet<PathBuf>,
    sidebar_width: Cell<f64>,
    sidebar_height: Cell<f64>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SidebarMode {
    Closed,
    Transient,
    Pinned,
}

enum Click {
    Open(String, String),
    SelectTab(u64),
    CloseTab(u64),
    NewTab,
    AddRepo,
    ToggleRepo(PathBuf),
}

pub enum TabTarget {
    Previous,
    Next,
    Last,
    Index(usize),
}

#[derive(Clone, Copy)]
pub enum SplitTarget {
    Previous,
    Next,
    Up,
    Left,
    Down,
    Right,
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
            let sidebar = handle_sidebar_event(event);
            if !quota && !sidebar { let _: () = unsafe { msg_send![super(self), sendEvent: event] }; }
            if event.r#type() == NSEventType::KeyDown {
                let close = STATE.with(|state| state.borrow().as_ref().is_some_and(|state| state.sidebar_mode.get() == SidebarMode::Transient && state.sidebar_keyboard.get() && !sidebar_has_focus(state)));
                if close { set_sidebar_mode(SidebarMode::Closed); }
            }
        }
    }
);

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeCommands"]
    #[ivars = ()]
    struct Commands;

    impl Commands {
        #[unsafe(method(menuWillSendAction:))]
        fn menu_will_send_action(&self, notification: &NSNotification) {
            let Some(info) = notification.userInfo() else { return };
            let Some(item) = info.objectForKey(&NSString::from_str("MenuItem")) else { return };
            let Some(item) = item.downcast_ref::<NSMenuItem>() else { return };
            if item.action() != Some(sel!(toggleTabOverview:)) {
                dismiss_overview(true);
            }
        }

        #[unsafe(method(newTab:))]
        fn new_tab(&self, _sender: Option<&AnyObject>) {
            dispatch(Click::NewTab);
        }

        #[unsafe(method(toggleTabOverview:))]
        fn toggle_tab_overview(&self, _sender: Option<&AnyObject>) { toggle_overview(); }

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
            goto_tab(TabTarget::Previous);
        }

        #[unsafe(method(nextTab:))]
        fn next_tab(&self, _sender: Option<&AnyObject>) {
            goto_tab(TabTarget::Next);
        }

        #[unsafe(method(find:))]
        fn find(&self, _sender: Option<&AnyObject>) {
            surface_action("start_search");
        }

        #[unsafe(method(findNext:))]
        fn find_next(&self, _sender: Option<&AnyObject>) {
            surface_action("navigate_search:next");
        }

        #[unsafe(method(findPrevious:))]
        fn find_previous(&self, _sender: Option<&AnyObject>) {
            surface_action("navigate_search:previous");
        }

        #[unsafe(method(focusSplitLeft:))]
        fn focus_split_left(&self, _sender: Option<&AnyObject>) {
            focus_split(SplitTarget::Left);
        }

        #[unsafe(method(focusSplitRight:))]
        fn focus_split_right(&self, _sender: Option<&AnyObject>) {
            focus_split(SplitTarget::Right);
        }

        #[unsafe(method(focusSplitUp:))]
        fn focus_split_up(&self, _sender: Option<&AnyObject>) {
            focus_split(SplitTarget::Up);
        }

        #[unsafe(method(focusSplitDown:))]
        fn focus_split_down(&self, _sender: Option<&AnyObject>) {
            focus_split(SplitTarget::Down);
        }

        #[unsafe(method(toggleSidebar:))]
        fn toggle_sidebar(&self, _sender: Option<&AnyObject>) {
            toggle_sidebar();
        }

        #[unsafe(method(copyText:))]
        fn copy_text(&self, _sender: Option<&AnyObject>) {
            clipboard_action("copy_to_clipboard", sel!(copy:));
        }

        #[unsafe(method(pasteText:))]
        fn paste_text(&self, _sender: Option<&AnyObject>) {
            clipboard_action("paste_from_clipboard", sel!(paste:));
        }

        #[unsafe(method(addRepo:))]
        fn add_repo(&self, _sender: Option<&AnyObject>) {
            dispatch(Click::AddRepo);
        }

        #[unsafe(method(openSidebar:))]
        fn open_sidebar(&self, _sender: Option<&AnyObject>) { set_sidebar_mode(SidebarMode::Transient); }

        #[unsafe(method(closeSidebar:))]
        fn close_sidebar(&self, _sender: Option<&AnyObject>) {
            if !sidebar_keeps_keyboard_focus() { set_sidebar_mode(SidebarMode::Closed); }
        }

        #[unsafe(method(foldSidebar:))]
        fn fold_sidebar(&self, _sender: Option<&AnyObject>) { set_sidebar_mode(SidebarMode::Closed); }

        #[unsafe(method(changeAppearance:))]
        fn change_appearance(&self, sender: &NSMenuItem) {
            let mtm = MainThreadMarker::new().expect("main thread");
            let appearance = match sender.tag() {
                1 => NSAppearance::appearanceNamed(unsafe { NSAppearanceNameAqua }),
                2 => NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua }),
                _ => None,
            };
            NSApplication::sharedApplication(mtm).setAppearance(appearance.as_deref());
            if let Some(menu) = unsafe { sender.menu() } {
                for entry in menu.itemArray() {
                    if entry.action() == Some(sel!(changeAppearance:)) { entry.setState(if entry.tag() == sender.tag() { 1 } else { 0 }); }
                }
            }
            sync_appearance();
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

        #[unsafe(method(applicationDidBecomeActive:))]
        fn did_become_active(&self, _notification: &AnyObject) {
            ghostty::set_focus(true);
            refresh_sidebar();
            quota_panel::refresh();
        }

        #[unsafe(method(applicationDidResignActive:))]
        fn did_resign_active(&self, _notification: &AnyObject) {
            ghostty::set_focus(false);
            deactivate_chrome();
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
    unsafe {
        NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
            &commands,
            sel!(menuWillSendAction:),
            Some(NSMenuWillSendActionNotification),
            None,
        );
    }

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
    let entries: [(&str, objc2::runtime::Sel, &str, NSEventModifierFlags); 17] = [
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
            "Focus Split Left",
            sel!(focusSplitLeft:),
            "\u{F702}",
            NSEventModifierFlags::Command.union(NSEventModifierFlags::Option),
        ),
        (
            "Focus Split Right",
            sel!(focusSplitRight:),
            "\u{F703}",
            NSEventModifierFlags::Command.union(NSEventModifierFlags::Option),
        ),
        (
            "Focus Split Up",
            sel!(focusSplitUp:),
            "\u{F700}",
            NSEventModifierFlags::Command.union(NSEventModifierFlags::Option),
        ),
        (
            "Focus Split Down",
            sel!(focusSplitDown:),
            "\u{F701}",
            NSEventModifierFlags::Command.union(NSEventModifierFlags::Option),
        ),
        (
            "Previous Tab",
            sel!(previousTab:),
            "[",
            NSEventModifierFlags::Command.union(NSEventModifierFlags::Shift),
        ),
        (
            "Next Tab",
            sel!(nextTab:),
            "]",
            NSEventModifierFlags::Command.union(NSEventModifierFlags::Shift),
        ),
        (
            "Toggle Sidebar",
            sel!(toggleSidebar:),
            "b",
            NSEventModifierFlags::Command,
        ),
        ("Find", sel!(find:), "f", NSEventModifierFlags::Command),
        (
            "Find Next",
            sel!(findNext:),
            "g",
            NSEventModifierFlags::Command,
        ),
        (
            "Find Previous",
            sel!(findPrevious:),
            "g",
            NSEventModifierFlags::Command.union(NSEventModifierFlags::Shift),
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
        "Tab Overview",
        sel!(toggleTabOverview:),
        Some(&commands),
        "\\",
        NSEventModifierFlags::Command.union(NSEventModifierFlags::Shift),
    ));
    view_menu.addItem(&item(
        mtm,
        "Enter Full Screen",
        sel!(toggleFullScreen:),
        None,
        "f",
        NSEventModifierFlags::Command.union(NSEventModifierFlags::Control),
    ));
    view_menu.addItem(&NSMenuItem::separatorItem(mtm));
    for (index, title) in ["Follow System", "Light", "Dark"].into_iter().enumerate() {
        let entry = item(
            mtm,
            title,
            sel!(changeAppearance:),
            Some(&commands),
            "",
            NSEventModifierFlags::empty(),
        );
        entry.setTag(index as NSInteger);
        entry.setState(if index == 0 { 1 } else { 0 });
        view_menu.addItem(&entry);
    }
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
    sidebar_view.setAutoresizingMask(NSAutoresizingMaskOptions::empty());

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
    let tab_scroller = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), tab_bar.frame());
    tab_scroller.setAutomaticallyAdjustsContentInsets(false);
    tab_scroller.setDrawsBackground(false);
    tab_scroller.setHasHorizontalScroller(false);
    tab_scroller.setDocumentView(Some(&tab_bar));

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
    root.addSubview(&tab_scroller);
    let overview_button = icon_button(
        mtm,
        "square.grid.2x2",
        sel!(toggleTabOverview:),
        NSRect::default(),
    );
    if let Some(button) = overview_button.as_ref() {
        button.setAccessibilityLabel(Some(&NSString::from_str("Tab Overview")));
        button.setToolTip(Some(&NSString::from_str("Tab Overview (⌘⇧\\)")));
        root.addSubview(button);
    }
    let sidebar_glass = NSView::initWithFrame(NSView::alloc(mtm), NSRect::default());
    sidebar_glass.setWantsLayer(true);
    sidebar_glass.setLayerContentsRedrawPolicy(NSViewLayerContentsRedrawPolicy::DuringViewResize);
    if let Some(layer) = sidebar_glass.layer() {
        let _: () = unsafe { msg_send![&*layer, setCornerRadius: SIDEBAR_RADIUS] };
    }
    root.addSubview(&sidebar_glass);
    let material = chrome_view::glass(mtm, NSRect::default(), SIDEBAR_RADIUS);
    material.setAutoresizingMask(FILL);
    sidebar_glass.addSubview(&material);
    material.addSubview(&sidebar_view);
    let workspace_chip = ClickView::new(
        mtm,
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(162.0, TAB_HEIGHT)),
        "Workspaces",
        12.0,
        28.0,
        || {
            let mode = STATE.with(|state| {
                state
                    .borrow()
                    .as_ref()
                    .map(|state| state.sidebar_mode.get())
            });
            if mode != Some(SidebarMode::Pinned) {
                set_sidebar_mode(SidebarMode::Transient);
            }
        },
    );
    workspace_chip.disable_hover_highlight();
    chrome_view::symbol(
        mtm,
        &workspace_chip,
        "chevron.down",
        NSRect::new(NSPoint::new(140.0, 13.0), NSSize::new(10.0, 10.0)),
    );
    root.addSubview(&workspace_chip);
    let toggle = icon_button(mtm, "sidebar.left", sel!(toggleSidebar:), NSRect::default());
    if let Some(toggle) = &toggle {
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
            sidebar: sidebar_view.clone(),
            sidebar_glass,
            sidebar_material: material,
            workspace_chip,
            sidebar_mode: Cell::new(SidebarMode::Pinned),
            sidebar_pointer: Cell::new(0),
            sidebar_entered: Cell::new(false),
            sidebar_keyboard: Cell::new(false),
            command_held: Cell::new(false),
            toggle,
            add_repo,
            overview: None,
            overview_return: None,
            overview_tab: None,
            overview_button,
            tab_bar: tab_bar.clone(),
            tab_scroller,
            content: content.clone(),
            tabs: Tabs::default(),
            repos: Vec::new(),
            collapsed_repos: HashSet::new(),
            sidebar_width: Cell::new(habits::SIDEBAR_WIDTH),
            sidebar_height: Cell::new(0.0),
        })
    });

    sync_appearance();
    if !habits::SIDEBAR_VISIBLE {
        toggle_sidebar();
    }

    window.center();
    window.makeKeyAndOrderFront(None);

    set_repos(sidebar::repos());
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
    button.setContentTintColor(Some(&chrome_view::color(habits::CHROME_SOFT)));
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
    dismiss_overview(false);
    match click {
        Click::Open(path, label) => {
            open_worktree(&path, &label);
            STATE.with(|state| {
                let state = state.borrow();
                let Some(state) = state.as_ref() else { return };
                if state.sidebar_keyboard.get()
                    && let Some(document) = state.sidebar.documentView()
                    && let Some(row) = document.subviews().into_iter().find(|row| {
                        row.accessibilityIdentifier()
                            .is_some_and(|id| id.to_string() == path)
                    })
                {
                    state.window.makeFirstResponder(Some(&row));
                    row.scrollRectToVisible(row.bounds());
                }
            });
        }
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
    dismiss_overview(false);
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
        rebuild_sidebar();
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

fn surface_action(action: &str) {
    if let Some(view) = focused_surface() {
        view.binding_action(action);
    }
}

fn clipboard_action(action: &str, native: objc2::runtime::Sel) {
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

fn focus_split(target: SplitTarget) {
    if let Some(view) = focused_surface() {
        goto_split(&view, target);
    }
}

pub fn goto_split(view: &SurfaceView, target: SplitTarget) -> bool {
    restore_zoom(view);
    let found = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let tab = state.tabs.items().iter().find(|tab| {
            split::surfaces(&tab.root)
                .iter()
                .any(|leaf| std::ptr::eq(&**leaf, view))
        })?;
        let leaves = split::surfaces(&tab.root);
        let index = leaves.iter().position(|leaf| std::ptr::eq(&**leaf, view))?;
        let next = match target {
            SplitTarget::Previous => {
                Some(leaves[(index + leaves.len() - 1) % leaves.len()].clone())
            }
            SplitTarget::Next => Some(leaves[(index + 1) % leaves.len()].clone()),
            _ => neighbor(&tab.root, view, &leaves, target),
        };
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

fn neighbor(
    root: &NSView,
    view: &SurfaceView,
    leaves: &[Retained<SurfaceView>],
    target: SplitTarget,
) -> Option<Retained<SurfaceView>> {
    let edges = |leaf: &SurfaceView| {
        let rect = root.convertRect_fromView(leaf.bounds(), Some(leaf));
        (
            rect.origin.x,
            rect.origin.x + rect.size.width,
            rect.origin.y,
            rect.origin.y + rect.size.height,
        )
    };
    let overlap = |a0: f64, a1: f64, b0: f64, b1: f64| a1.min(b1) - a0.max(b0);
    let (fx0, fx1, fy0, fy1) = edges(view);
    leaves
        .iter()
        .filter(|leaf| !std::ptr::eq(&***leaf, view))
        .filter_map(|leaf| {
            let (x0, x1, y0, y1) = edges(leaf);
            let (distance, shared) = match target {
                SplitTarget::Left => (fx0 - x1, overlap(y0, y1, fy0, fy1)),
                SplitTarget::Right => (x0 - fx1, overlap(y0, y1, fy0, fy1)),
                SplitTarget::Up => (y0 - fy1, overlap(x0, x1, fx0, fx1)),
                SplitTarget::Down => (fy0 - y1, overlap(x0, x1, fx0, fx1)),
                SplitTarget::Previous | SplitTarget::Next => return None,
            };
            (distance >= 0.0 && shared > 0.0).then_some((distance, shared, leaf.clone()))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0).then(b.1.total_cmp(&a.1)))
        .map(|(_, _, leaf)| leaf)
}

fn toggle_sidebar() {
    let pinned = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .is_some_and(|state| state.sidebar_mode.get() == SidebarMode::Pinned)
    });
    set_sidebar_mode(if pinned {
        SidebarMode::Closed
    } else {
        SidebarMode::Pinned
    });
}

fn set_sidebar_mode(mode: SidebarMode) {
    cancel_sidebar_intent();
    let Some((split, pane, remembered, restore_focus)) = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let restore_focus = mode == SidebarMode::Closed && sidebar_has_focus(state);
        state.sidebar_mode.set(mode);
        state.sidebar_entered.set(false);
        if mode == SidebarMode::Closed {
            state.sidebar_keyboard.set(false);
        }
        Some((
            state.split.clone(),
            state.sidebar_pane.clone(),
            state.sidebar_width.get(),
            restore_focus,
        ))
    }) else {
        return;
    };
    let pinned = mode == SidebarMode::Pinned;
    ANIMATE_CHROME.set(true);
    if pane.isHidden() == pinned {
        pane.setHidden(!pinned);
        split.adjustSubviews();
        if pinned {
            split.setPosition_ofDividerAtIndex(remembered + 2.0 * INSET, 0);
        }
    }
    layout_chrome();
    ANIMATE_CHROME.set(false);
    if restore_focus {
        focus_active();
    }
}

fn cancel_sidebar_intent() {
    SIDEBAR_TIMER.with(|timer| {
        if let Some(timer) = timer.borrow_mut().take() {
            timer.invalidate();
        }
    });
}

fn schedule_sidebar_intent(action: objc2::runtime::Sel, delay: f64) {
    cancel_sidebar_intent();
    let mtm = MainThreadMarker::new().expect("main thread");
    let target = commands(mtm);
    let timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            delay, &target, action, None, false,
        )
    };
    SIDEBAR_TIMER.with(|pending| *pending.borrow_mut() = Some(timer));
}

fn sidebar_keeps_keyboard_focus() -> bool {
    STATE.with(|state| {
        let state = state.borrow();
        state
            .as_ref()
            .is_some_and(|state| state.sidebar_keyboard.get() && sidebar_has_focus(state))
    })
}

fn sidebar_has_focus(state: &State) -> bool {
    let Some(responder) = state.window.firstResponder() else {
        return false;
    };
    let Ok(view) = responder.downcast::<NSView>() else {
        return false;
    };
    view.isDescendantOf(&state.sidebar_glass)
        || view.isDescendantOf(&state.workspace_chip)
        || state
            .toggle
            .as_ref()
            .is_some_and(|control| view.isDescendantOf(control))
        || state
            .add_repo
            .as_ref()
            .is_some_and(|control| view.isDescendantOf(control))
}

fn deactivate_chrome() {
    cancel_sidebar_intent();
    STATE.with(|state| {
        if let Some(state) = state.borrow().as_ref() {
            state.command_held.set(false);
            state.sidebar_keyboard.set(false);
            state.sidebar_pointer.set(0);
        }
    });
    update_workspace_hints();
    let transient = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .is_some_and(|state| state.sidebar_mode.get() == SidebarMode::Transient)
    });
    if transient {
        set_sidebar_mode(SidebarMode::Closed);
    }
    quota_panel::deactivate();
}

fn contains(rect: NSRect, point: NSPoint) -> bool {
    point.x >= rect.origin.x
        && point.x < rect.origin.x + rect.size.width
        && point.y >= rect.origin.y
        && point.y < rect.origin.y + rect.size.height
}

fn handle_sidebar_event(event: &NSEvent) -> bool {
    let kind = event.r#type();
    if matches!(
        kind,
        NSEventType::FlagsChanged | NSEventType::KeyDown | NSEventType::KeyUp
    ) {
        STATE.with(|state| {
            if let Some(state) = state.borrow().as_ref() {
                state.command_held.set(
                    event
                        .modifierFlags()
                        .contains(NSEventModifierFlags::Command),
                );
            }
        });
        update_workspace_hints();
    }
    if matches!(
        kind,
        NSEventType::MouseMoved
            | NSEventType::LeftMouseDragged
            | NSEventType::RightMouseDragged
            | NSEventType::LeftMouseDown
            | NSEventType::RightMouseDown
    ) {
        let Some((region, previous, mode, entered)) = STATE.with(|state| {
            let state = state.borrow();
            let state = state.as_ref()?;
            let root = state.window.contentView()?;
            let point = root.convertPoint_fromView(event.locationInWindow(), None);
            let mode = state.sidebar_mode.get();
            let region = if contains(state.workspace_chip.frame(), point) {
                1
            } else if contains(state.sidebar_glass.frame(), point) {
                if point.y < state.workspace_chip.frame().origin.y {
                    2
                } else {
                    3
                }
            } else {
                0
            };
            let previous = state.sidebar_pointer.replace(region);
            if region == 2 {
                state.sidebar_entered.set(true);
            }
            if matches!(
                kind,
                NSEventType::LeftMouseDown | NSEventType::RightMouseDown
            ) {
                state.sidebar_keyboard.set(false);
            }
            Some((region, previous, mode, state.sidebar_entered.get()))
        }) else {
            return false;
        };
        if matches!(
            kind,
            NSEventType::LeftMouseDown | NSEventType::RightMouseDown
        ) {
            cancel_sidebar_intent();
            if region == 0 && mode == SidebarMode::Transient {
                set_sidebar_mode(SidebarMode::Closed);
            }
        } else if NSEvent::pressedMouseButtons() != 0 {
            cancel_sidebar_intent();
        } else if region != previous && mode != SidebarMode::Pinned {
            cancel_sidebar_intent();
            if region == 1 && mode == SidebarMode::Closed {
                schedule_sidebar_intent(sel!(openSidebar:), 0.15);
            } else if region == 1 && entered {
                schedule_sidebar_intent(sel!(foldSidebar:), 0.15);
            } else if region == 0 && mode == SidebarMode::Transient {
                schedule_sidebar_intent(sel!(closeSidebar:), 0.25);
            }
        }
    }
    if kind != NSEventType::KeyDown {
        return false;
    }
    let Some((mode, focus)) = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        Some((state.sidebar_mode.get(), sidebar_has_focus(state)))
    }) else {
        return false;
    };
    if event.keyCode() == 53 {
        cancel_sidebar_intent();
        if mode == SidebarMode::Transient {
            set_sidebar_mode(SidebarMode::Closed);
            focus_active();
            return true;
        }
    }
    let modifiers = event.modifierFlags();
    let command_only = modifiers.contains(NSEventModifierFlags::Command)
        && !modifiers.intersects(
            NSEventModifierFlags::Control
                | NSEventModifierFlags::Option
                | NSEventModifierFlags::Shift,
        );
    if mode != SidebarMode::Closed
        && command_only
        && let Some(index) = event
            .charactersIgnoringModifiers()
            .and_then(|text| text.to_string().parse::<usize>().ok())
            .filter(|number| (1..=9).contains(number))
    {
        cancel_sidebar_intent();
        let target = STATE.with(|state| {
            let state = state.borrow();
            let state = state.as_ref()?;
            state
                .repos
                .iter()
                .filter(|repo| !state.collapsed_repos.contains(&repo.path))
                .flat_map(|repo| &repo.rows)
                .nth(index - 1)
                .map(|row| (row.path.to_string_lossy().into_owned(), row.label.clone()))
        });
        if let Some((path, label)) = target {
            open_worktree(&path, &label);
            focus_sidebar_row(index - 1);
        }
        return true;
    }
    if focus {
        cancel_sidebar_intent();
        STATE.with(|state| {
            if let Some(state) = state.borrow().as_ref() {
                state.sidebar_keyboard.set(true);
            }
        });
        if event.keyCode() == 125 || event.keyCode() == 126 {
            if mode == SidebarMode::Closed {
                set_sidebar_mode(SidebarMode::Transient);
            }
            move_sidebar_focus(event.keyCode() == 125);
            return true;
        }
    }
    false
}

fn focus_sidebar_row(index: usize) {
    STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else { return };
        state.sidebar_keyboard.set(true);
        if let Some(document) = state.sidebar.documentView()
            && let Some(row) = document
                .subviews()
                .into_iter()
                .filter(|view| {
                    view.isKindOfClass(ClickView::class())
                        && view
                            .accessibilityIdentifier()
                            .is_some_and(|id| !id.to_string().starts_with("repo:"))
                })
                .nth(index)
        {
            state.window.makeFirstResponder(Some(&row));
            row.scrollRectToVisible(row.bounds());
        }
    });
}

fn move_sidebar_focus(forward: bool) {
    STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else { return };
        let Some(document) = state.sidebar.documentView() else {
            return;
        };
        let rows: Vec<_> = document
            .subviews()
            .into_iter()
            .filter(|view| view.isKindOfClass(ClickView::class()))
            .collect();
        if rows.is_empty() {
            return;
        }
        let current = state.window.firstResponder();
        let index = rows.iter().position(|row| {
            current.as_ref().is_some_and(|current| {
                std::ptr::eq(
                    &**row as *const _ as *const AnyObject,
                    &**current as *const _ as *const AnyObject,
                )
            })
        });
        let index = index
            .map(|index| {
                if forward {
                    (index + 1) % rows.len()
                } else {
                    (index + rows.len() - 1) % rows.len()
                }
            })
            .unwrap_or(0);
        state.window.makeFirstResponder(Some(&rows[index]));
        rows[index].scrollRectToVisible(rows[index].bounds());
    });
}

fn update_workspace_hints() {
    STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else { return };
        let Some(document) = state.sidebar.documentView() else {
            return;
        };
        let mut index = 0;
        for view in document.subviews() {
            if view
                .accessibilityIdentifier()
                .is_none_or(|id| id.to_string().starts_with("repo:"))
            {
                continue;
            }
            index += 1;
            if let Ok(row) = view.downcast::<ClickView>() {
                row.set_shortcut(
                    (state.command_held.get()
                        && state.sidebar_mode.get() != SidebarMode::Closed
                        && index <= 9)
                        .then_some(index),
                );
            }
        }
    });
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

fn toggle_overview() {
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

fn dismiss_overview(restore: bool) {
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
    let mtm = MainThreadMarker::new().expect("main thread");
    STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else { return };

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

        for child in state.tab_bar.subviews() {
            child.removeFromSuperview();
        }

        let y = (TOP_BAR_HEIGHT - TAB_HEIGHT) / 2.0;
        let mut x = 0.0;
        for tab in state.tabs.visible() {
            let frame = NSRect::new(NSPoint::new(x, y), NSSize::new(TAB_WIDTH, TAB_HEIGHT));
            let id = tab.id;
            let view = ClickView::new(mtm, frame, &tab.label, 12.0, 28.0, move || {
                dispatch(Click::SelectTab(id))
            });
            view.set_corner_radius(TAB_RADIUS);
            if Some(tab.id) != active {
                view.dim_when_idle();
            }
            if Some(tab.id) == active {
                view.set_text_color(habits::CHROME_STRONG);
                let glass = chrome_view::glass(mtm, frame, TAB_RADIUS);
                state.tab_bar.addSubview(&glass);
            }
            view.setAccessibilitySelected(Some(tab.id) == active);
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
            close.disable_hover_highlight();
            state.tab_bar.addSubview(&close);
            x += TAB_WIDTH + INSET;
        }

        let plus = ClickView::new(
            mtm,
            NSRect::new(NSPoint::new(x, y), NSSize::new(TAB_HEIGHT, TAB_HEIGHT)),
            "+",
            10.0,
            0.0,
            || dispatch(Click::NewTab),
        );
        plus.dim_when_idle();
        plus.disable_hover_highlight();
        state.tab_bar.addSubview(&plus);
        state
            .tab_bar
            .setFrameSize(NSSize::new(x + TAB_HEIGHT, TOP_BAR_HEIGHT));
    });
}

fn set_repos(repos: Vec<sidebar::Repo>) {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.repos = repos;
        }
    });
    rebuild_sidebar();
}

fn refresh_sidebar() {
    if SIDEBAR_SCANNING.get() {
        SIDEBAR_DIRTY.set(true);
        return;
    }
    SIDEBAR_SCANNING.set(true);
    let spawned = std::thread::Builder::new()
        .name("combe-catalog".into())
        .spawn(|| {
            let repos = sidebar::repos();
            *SIDEBAR_INCOMING
                .lock()
                .unwrap_or_else(|poison| poison.into_inner()) = Some(repos);
            ghostty::on_main(apply_sidebar_on_main);
        });
    if spawned.is_err() {
        SIDEBAR_SCANNING.set(false);
    }
}

unsafe extern "C" fn apply_sidebar_on_main(_: *mut c_void) {
    SIDEBAR_SCANNING.set(false);
    let repos = SIDEBAR_INCOMING
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .take();
    if let Some(repos) = repos {
        set_repos(repos);
    }
    if SIDEBAR_DIRTY.replace(false) {
        refresh_sidebar();
    }
}

fn rebuild_sidebar() {
    let mtm = MainThreadMarker::new().expect("main thread");
    STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else { return };

        let focused = state
            .window
            .firstResponder()
            .and_then(|responder| responder.downcast::<NSView>().ok())
            .and_then(|view| view.identifier());
        let clip = state.sidebar.contentView().frame().size;
        let width = state.sidebar_width.get();
        let mut height = 20.0 + state.repos.len().saturating_sub(1) as f64 * 16.5;
        for repo in &state.repos {
            height += HEADER_HEIGHT;
            if !state.collapsed_repos.contains(&repo.path) {
                height += repo.rows.len() as f64 * ROW_HEIGHT;
            }
        }
        state.sidebar_height.set(height);

        let document = SidebarView::alloc(mtm).set_ivars(());
        let document: Retained<SidebarView> = unsafe {
            msg_send![super(document), initWithFrame: NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(width, height.max(clip.height)),
            )]
        };

        let mut y = 8.0;
        for (index, repo) in state.repos.iter().enumerate() {
            if index > 0 {
                y += 16.5;
            }
            let collapsed = state.collapsed_repos.contains(&repo.path);

            let path = repo.path.clone();
            let header = ClickView::new(
                mtm,
                NSRect::new(
                    NSPoint::new(6.0, y),
                    NSSize::new(width - 12.0, HEADER_HEIGHT),
                ),
                &repo.name,
                34.0,
                28.0,
                move || dispatch(Click::ToggleRepo(path.clone())),
            );
            header.dim_when_idle();
            header.set_font(&NSFont::systemFontOfSize_weight(
                habits::CHROME_FONT_SIZE,
                unsafe { objc2_app_kit::NSFontWeightMedium },
            ));
            chrome_view::symbol(
                mtm,
                &header,
                "folder",
                NSRect::new(NSPoint::new(12.0, 8.0), NSSize::new(14.0, 14.0)),
            );
            chrome_view::symbol(
                mtm,
                &header,
                if collapsed {
                    "chevron.right"
                } else {
                    "chevron.down"
                },
                NSRect::new(NSPoint::new(width - 38.0, 10.0), NSSize::new(10.0, 10.0)),
            );
            if let Some(arrow) = header.subviews().lastObject() {
                arrow.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMinXMargin);
            }
            header.setAccessibilityElement(true);
            header.setAccessibilityRole(Some(&NSString::from_str("AXButton")));
            header.setAccessibilityLabel(Some(&NSString::from_str(&repo.name)));
            header.setIdentifier(Some(&NSString::from_str(&format!(
                "repo:{}",
                repo.path.display()
            ))));
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
                    NSPoint::new(6.0, y + 2.0),
                    NSSize::new(width - 12.0, ROW_HEIGHT - 2.0),
                );
                let opened = state.tabs.opened(&path);
                let target = path.clone();
                let label = row.label.clone();
                let view = ClickView::new(mtm, frame, &row.label, 48.0, 34.0, move || {
                    dispatch(Click::Open(target.clone(), label.clone()))
                });
                view.set_font(&NSFont::systemFontOfSize(13.0));
                view.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
                view.set_selected(state.tabs.current() == Some(path.as_str()));
                view.set_opened(opened);
                view.setAccessibilityIdentifier(Some(&NSString::from_str(&path)));
                view.setIdentifier(Some(&NSString::from_str(&path)));
                view.setAccessibilityElement(true);
                view.setAccessibilityRole(Some(&NSString::from_str("AXButton")));
                let access = if opened {
                    format!("{}, open", row.label)
                } else {
                    format!("{}, not open", row.label)
                };
                view.setAccessibilityLabel(Some(&NSString::from_str(&access)));
                document.addSubview(&view);
                y += ROW_HEIGHT;
            }
        }

        state.sidebar.setDocumentView(Some(&document));
        if state.sidebar_keyboard.get()
            && let Some(identifier) = focused
            && let Some(view) = document
                .subviews()
                .into_iter()
                .find(|view| view.identifier().is_some_and(|value| value == identifier))
        {
            state.window.makeFirstResponder(Some(&view));
        }
        let current = state.tabs.current();
        let title = state.repos.iter().find_map(|repo| {
            repo.rows
                .iter()
                .find(|row| current == Some(row.path.to_string_lossy().as_ref()))
                .map(|row| format!("{} / {}", repo.name, row.label))
        });
        state
            .workspace_chip
            .set_text(title.as_deref().unwrap_or("Workspaces"));
    });
    update_workspace_hints();
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
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeSidebarView"]
    #[ivars = ()]
    struct SidebarView;

    impl SidebarView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            NSColor::labelColor().colorWithAlphaComponent(0.06).setFill();
            for header in self.subviews().into_iter().filter(|view| {
                view.identifier().is_some_and(|identifier| identifier.to_string().starts_with("repo:"))
            }).skip(1) {
                NSBezierPath::fillRect(NSRect::new(
                    NSPoint::new(6.0, header.frame().origin.y - 8.5),
                    NSSize::new((self.bounds().size.width - 12.0).max(0.0), 0.5),
                ));
            }
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {
            refresh_sidebar();
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
            SIDEBAR_MIN_WIDTH + 2.0 * INSET
        }

        #[unsafe(method(splitView:constrainMaxCoordinate:ofSubviewAt:))]
        fn constrain_max(
            &self,
            _split: &NSSplitView,
            _proposed: CGFloat,
            _index: NSInteger,
        ) -> CGFloat {
            SIDEBAR_MAX_WIDTH + 2.0 * INSET
        }

        #[unsafe(method(splitViewDidResizeSubviews:))]
        fn did_resize(&self, _notification: &NSNotification) {
            STATE.with(|state| {
                if let Some(state) = state.borrow().as_ref() {
                    if sidebar_open(state) {
                        state.sidebar_width.set(
                            (state.sidebar_pane.frame().size.width - 2.0 * INSET)
                                .clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH),
                        );
                    } else if state.sidebar_mode.get() == SidebarMode::Pinned {
                        state.sidebar_mode.set(SidebarMode::Closed);
                    }
                }
            });
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

fn place(view: &NSView, frame: NSRect) {
    if ANIMATE_CHROME.get() {
        view.animator().setFrame(frame);
    } else if view.frame() != frame {
        view.setFrame(frame);
    }
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
        let mode = state.sidebar_mode.get();
        let pinned = mode == SidebarMode::Pinned;
        let open = mode != SidebarMode::Closed;
        state.sidebar_material.set_expanded(open);
        let width = state.sidebar_width.get();
        let edge = width + INSET;
        let glass_height = if pinned {
            size.height - 2.0 * INSET
        } else if open {
            (TAB_HEIGHT + state.sidebar_height.get()).min(size.height - 2.0 * INSET)
        } else {
            TAB_HEIGHT
        };
        let left = if open { INSET } else { inset };
        place(
            &state.sidebar_glass,
            NSRect::new(
                NSPoint::new(left, size.height - INSET - glass_height),
                NSSize::new((edge - left).max(0.0), glass_height.max(TAB_HEIGHT)),
            ),
        );
        let shadow_blur = if open { 24.0 } else { 3.0 };
        if state
            .sidebar_glass
            .shadow()
            .is_none_or(|shadow| shadow.shadowBlurRadius() != shadow_blur)
        {
            let shadow = NSShadow::new();
            shadow.setShadowColor(Some(
                &NSColor::blackColor().colorWithAlphaComponent(if open { 0.30 } else { 0.07 }),
            ));
            shadow.setShadowBlurRadius(shadow_blur);
            shadow.setShadowOffset(NSSize::new(0.0, if open { -12.0 } else { -1.0 }));
            state.sidebar_glass.setShadow(Some(&shadow));
        }
        let context = NSAnimationContext::currentContext();
        let implicit = context.allowsImplicitAnimation();
        context.setAllowsImplicitAnimation(false);
        state.sidebar.setHidden(!open);
        context.setAllowsImplicitAnimation(implicit);
        let viewport_height = (glass_height - TAB_HEIGHT).max(0.0);
        state
            .sidebar
            .setHasVerticalScroller(state.sidebar_height.get() > viewport_height);
        place(
            &state.sidebar,
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, viewport_height)),
        );
        let clip = state.sidebar.contentView().frame().size;
        if let Some(document) = state.sidebar.documentView() {
            document.setFrameSize(NSSize::new(
                clip.width,
                state.sidebar_height.get().max(clip.height),
            ));
        }
        let header_y = size.height - INSET - TAB_HEIGHT;
        state.workspace_chip.setFrame(NSRect::new(
            NSPoint::new(inset, header_y),
            NSSize::new((edge - inset - 66.0).max(0.0), TAB_HEIGHT),
        ));
        state.workspace_chip.setAccessibilityExpanded(open);
        if let Some(view) = state
            .workspace_chip
            .subviews()
            .lastObject()
            .and_then(|view| view.downcast::<NSImageView>().ok())
        {
            view.setFrameOrigin(NSPoint::new(
                (state.workspace_chip.frame().size.width - 22.0).max(0.0),
                13.0,
            ));
            view.setHidden(pinned);
            view.setImage(
                NSImage::imageWithSystemSymbolName_accessibilityDescription(
                    &NSString::from_str(if open { "chevron.up" } else { "chevron.down" }),
                    None,
                )
                .as_deref(),
            );
        }
        if let Some(toggle) = &state.toggle {
            toggle.setFrame(NSRect::new(
                NSPoint::new(edge - 32.0, header_y + 4.0),
                NSSize::new(TOGGLE_WIDTH, TOGGLE_HEIGHT),
            ));
            toggle.setToolTip(Some(&NSString::from_str(if pinned {
                "Unpin sidebar"
            } else {
                "Pin sidebar"
            })));
            toggle.setAccessibilityLabel(Some(&NSString::from_str(if pinned {
                "Unpin sidebar"
            } else {
                "Pin sidebar"
            })));
        }
        if let Some(add) = &state.add_repo {
            add.setFrame(NSRect::new(
                NSPoint::new(edge - 64.0, header_y + 4.0),
                NSSize::new(TOGGLE_WIDTH, TOGGLE_HEIGHT),
            ));
        }
        state.tab_scroller.setFrame(NSRect::new(
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
                            size.height - 24.0 - button.frame().size.height / 2.0,
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
