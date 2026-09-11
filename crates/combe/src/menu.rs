use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSApplication,
    NSEventModifierFlags, NSMenu, NSMenuItem, NSMenuWillSendActionNotification,
};
use objc2_foundation::{
    MainThreadMarker, NSInteger, NSNotification, NSNotificationCenter, NSString,
};

use crate::{sidebar_panel, split, window};

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeCommands"]
    #[ivars = ()]
    pub(crate) struct Commands;

    impl Commands {
        #[unsafe(method(menuWillSendAction:))]
        fn menu_will_send_action(&self, notification: &NSNotification) {
            let Some(info) = notification.userInfo() else { return };
            let Some(item) = info.objectForKey(&NSString::from_str("MenuItem")) else { return };
            let Some(item) = item.downcast_ref::<NSMenuItem>() else { return };
            if item.action() != Some(sel!(toggleTabOverview:)) {
                window::dismiss_overview(true);
            }
        }

        #[unsafe(method(newTab:))]
        fn new_tab(&self, _sender: Option<&AnyObject>) {
            window::new_current_tab();
        }

        #[unsafe(method(toggleTabOverview:))]
        fn toggle_tab_overview(&self, _sender: Option<&AnyObject>) { window::toggle_overview(); }

        #[unsafe(method(closeFocused:))]
        fn close_focused(&self, _sender: Option<&AnyObject>) {
            window::close_focused();
        }

        #[unsafe(method(splitRight:))]
        fn split_right(&self, _sender: Option<&AnyObject>) {
            window::divide(true);
        }

        #[unsafe(method(splitDown:))]
        fn split_down(&self, _sender: Option<&AnyObject>) {
            window::divide(false);
        }

        #[unsafe(method(toggleSplitZoom:))]
        fn toggle_split_zoom(&self, _sender: Option<&AnyObject>) {
            window::toggle_split_zoom();
        }

        #[unsafe(method(previousTab:))]
        fn previous_tab(&self, _sender: Option<&AnyObject>) {
            window::goto_tab(window::TabTarget::Previous);
        }

        #[unsafe(method(nextTab:))]
        fn next_tab(&self, _sender: Option<&AnyObject>) {
            window::goto_tab(window::TabTarget::Next);
        }

        #[unsafe(method(find:))]
        fn find(&self, _sender: Option<&AnyObject>) {
            window::surface_action("start_search");
        }

        #[unsafe(method(findNext:))]
        fn find_next(&self, _sender: Option<&AnyObject>) {
            window::surface_action("navigate_search:next");
        }

        #[unsafe(method(findPrevious:))]
        fn find_previous(&self, _sender: Option<&AnyObject>) {
            window::surface_action("navigate_search:previous");
        }

        #[unsafe(method(focusSplitLeft:))]
        fn focus_split_left(&self, _sender: Option<&AnyObject>) {
            window::focus_split(split::Target::Left);
        }

        #[unsafe(method(focusSplitRight:))]
        fn focus_split_right(&self, _sender: Option<&AnyObject>) {
            window::focus_split(split::Target::Right);
        }

        #[unsafe(method(focusSplitUp:))]
        fn focus_split_up(&self, _sender: Option<&AnyObject>) {
            window::focus_split(split::Target::Up);
        }

        #[unsafe(method(focusSplitDown:))]
        fn focus_split_down(&self, _sender: Option<&AnyObject>) {
            window::focus_split(split::Target::Down);
        }

        #[unsafe(method(toggleSidebar:))]
        fn toggle_sidebar(&self, _sender: Option<&AnyObject>) {
            sidebar_panel::toggle();
        }

        #[unsafe(method(copyText:))]
        fn copy_text(&self, _sender: Option<&AnyObject>) {
            window::clipboard_action("copy_to_clipboard", sel!(copy:));
        }

        #[unsafe(method(pasteText:))]
        fn paste_text(&self, _sender: Option<&AnyObject>) {
            window::clipboard_action("paste_from_clipboard", sel!(paste:));
        }

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
            window::sync_appearance();
        }

        #[unsafe(method(closeAllWindows:))]
        fn close_all_windows(&self, _sender: Option<&AnyObject>) {
            window::close_all_windows();
        }
    }
);

thread_local! {
    static COMMANDS: RefCell<Option<Retained<Commands>>> = const { RefCell::new(None) };
}

pub(crate) fn commands(mtm: MainThreadMarker) -> Retained<Commands> {
    COMMANDS.with(|slot| {
        let mut slot = slot.borrow_mut();
        slot.get_or_insert_with(|| {
            let this = Commands::alloc(mtm).set_ivars(());
            unsafe { msg_send![super(this), init] }
        })
        .clone()
    })
}

pub(crate) fn install(mtm: MainThreadMarker, app: &NSApplication) {
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
