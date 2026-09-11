use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Mutex;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
use objc2::{ClassType, MainThreadOnly, Message, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAccessibility, NSAnimatablePropertyContainer, NSAnimationContext, NSAutoresizingMaskOptions,
    NSBezierPath, NSButton, NSColor, NSEvent, NSEventModifierFlags, NSEventType, NSFont,
    NSImageView, NSOpenPanel, NSScrollView, NSShadow, NSUserInterfaceItemIdentification, NSView,
    NSViewLayerContentsRedrawPolicy, NSWindow,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString, NSTimer};

use crate::chrome_view::{self, ClickView};
use crate::{ghostty, habits, sidebar};

const ROW_HEIGHT: f64 = 36.0;
const HEADER_HEIGHT: f64 = 30.0;
const TAB_HEIGHT: f64 = 36.0;
const TOGGLE_WIDTH: f64 = 28.0;
const TOGGLE_HEIGHT: f64 = 28.0;
const INSET: f64 = 12.0;
const SIDEBAR_RADIUS: f64 = 18.0;
pub(crate) const MIN_WIDTH: f64 = 160.0;
pub(crate) const MAX_WIDTH: f64 = 420.0;
const FILL: NSAutoresizingMaskOptions = NSAutoresizingMaskOptions(
    NSAutoresizingMaskOptions::ViewWidthSizable.0 | NSAutoresizingMaskOptions::ViewHeightSizable.0,
);

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static SIDEBAR_SCANNING: Cell<bool> = const { Cell::new(false) };
    static SIDEBAR_DIRTY: Cell<bool> = const { Cell::new(false) };
    static SIDEBAR_TIMER: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
}
static SIDEBAR_INCOMING: Mutex<Option<Vec<sidebar::Repo>>> = Mutex::new(None);

struct State {
    window: Retained<NSWindow>,
    actions: Retained<Actions>,
    activate: fn(&str, &str),
    prepare: fn(),
    layout: fn(bool),
    restore_focus: fn(),
    sidebar: Retained<NSScrollView>,
    sidebar_glass: Retained<NSView>,
    sidebar_material: Retained<chrome_view::GlassView>,
    workspace_chip: Retained<ClickView>,
    sidebar_mode: Cell<Mode>,
    sidebar_pointer: Cell<u8>,
    sidebar_entered: Cell<bool>,
    sidebar_keyboard: Cell<bool>,
    command_held: Cell<bool>,
    toggle: Option<Retained<NSButton>>,
    add_repo: Option<Retained<NSButton>>,
    repos: Vec<sidebar::Repo>,
    collapsed_repos: HashSet<PathBuf>,
    current: Option<String>,
    opened: HashSet<String>,
    sidebar_width: Cell<f64>,
    sidebar_height: Cell<f64>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Closed,
    Transient,
    Pinned,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeSidebarActions"]
    #[ivars = ()]
    struct Actions;

    impl Actions {
        #[unsafe(method(openSidebar:))]
        fn open_sidebar(&self, _sender: Option<&AnyObject>) { set_sidebar_mode(Mode::Transient); }

        #[unsafe(method(closeSidebar:))]
        fn close_sidebar(&self, _sender: Option<&AnyObject>) {
            if !sidebar_keeps_keyboard_focus() { set_sidebar_mode(Mode::Closed); }
        }

        #[unsafe(method(foldSidebar:))]
        fn fold_sidebar(&self, _sender: Option<&AnyObject>) { set_sidebar_mode(Mode::Closed); }

        #[unsafe(method(toggleSidebar:))]
        fn toggle_sidebar(&self, _sender: Option<&AnyObject>) { toggle(); }

        #[unsafe(method(addRepo:))]
        fn add_repo(&self, _sender: Option<&AnyObject>) { add_repo(); }
    }
);

pub(crate) fn mount(
    mtm: MainThreadMarker,
    window: &NSWindow,
    parent: &NSView,
    activate: fn(&str, &str),
    prepare: fn(),
    layout: fn(bool),
    restore_focus: fn(),
) {
    let actions = Actions::alloc(mtm).set_ivars(());
    let actions: Retained<Actions> = unsafe { msg_send![super(actions), init] };
    let sidebar_view = NSScrollView::initWithFrame(
        NSScrollView::alloc(mtm),
        NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(
                habits::SIDEBAR_WIDTH + 2.0 * INSET,
                parent.frame().size.height - 60.0,
            ),
        ),
    );
    sidebar_view.setHasVerticalScroller(true);
    sidebar_view.setDrawsBackground(false);
    sidebar_view.setAutoresizingMask(NSAutoresizingMaskOptions::empty());
    let sidebar_glass = NSView::initWithFrame(NSView::alloc(mtm), NSRect::default());
    sidebar_glass.setWantsLayer(true);
    sidebar_glass.setLayerContentsRedrawPolicy(NSViewLayerContentsRedrawPolicy::DuringViewResize);
    if let Some(layer) = sidebar_glass.layer() {
        let _: () = unsafe { msg_send![&*layer, setCornerRadius: SIDEBAR_RADIUS] };
    }
    parent.addSubview(&sidebar_glass);
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
            if mode != Some(Mode::Pinned) {
                set_sidebar_mode(Mode::Transient);
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
    parent.addSubview(&workspace_chip);
    let toggle = chrome_view::icon_button(
        mtm,
        "sidebar.left",
        &actions,
        sel!(toggleSidebar:),
        NSRect::default(),
    );
    if let Some(toggle) = &toggle {
        parent.addSubview(toggle);
    }
    let add_repo =
        chrome_view::icon_button(mtm, "plus", &actions, sel!(addRepo:), NSRect::default());
    if let Some(add_repo) = &add_repo {
        add_repo.setToolTip(Some(&NSString::from_str("Add repo")));
        add_repo.setAccessibilityLabel(Some(&NSString::from_str("Add repo")));
        parent.addSubview(add_repo);
    }
    STATE.with(|state| {
        *state.borrow_mut() = Some(State {
            window: window.retain(),
            actions,
            activate,
            prepare,
            layout,
            restore_focus,
            sidebar: sidebar_view,
            sidebar_glass,
            sidebar_material: material,
            workspace_chip,
            sidebar_mode: Cell::new(Mode::Pinned),
            sidebar_pointer: Cell::new(0),
            sidebar_entered: Cell::new(false),
            sidebar_keyboard: Cell::new(false),
            command_held: Cell::new(false),
            toggle,
            add_repo,
            repos: Vec::new(),
            collapsed_repos: HashSet::new(),
            current: None,
            opened: HashSet::new(),
            sidebar_width: Cell::new(habits::SIDEBAR_WIDTH),
            sidebar_height: Cell::new(0.0),
        });
    });
}

pub(crate) fn set_sessions(current: Option<String>, opened: HashSet<String>) {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.current = current;
            state.opened = opened;
        }
    });
}

pub(crate) fn select(path: &str, label: &str) {
    prepare_action();
    activate(path, label);
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

fn activate(path: &str, label: &str) {
    let activate = STATE.with(|state| state.borrow().as_ref().map(|state| state.activate));
    if let Some(activate) = activate {
        activate(path, label);
    }
}

fn prepare_action() {
    let prepare = STATE.with(|state| state.borrow().as_ref().map(|state| state.prepare));
    if let Some(prepare) = prepare {
        prepare();
    }
}

fn request_layout(animated: bool) {
    let layout = STATE.with(|state| state.borrow().as_ref().map(|state| state.layout));
    if let Some(layout) = layout {
        layout(animated);
    }
}

fn restore_focus() {
    let restore = STATE.with(|state| state.borrow().as_ref().map(|state| state.restore_focus));
    if let Some(restore) = restore {
        restore();
    }
}

fn toggle_repo(path: PathBuf) {
    prepare_action();
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        if !state.collapsed_repos.remove(&path) {
            state.collapsed_repos.insert(path);
        }
    });
    rebuild();
}

pub(crate) fn after_event(event: &NSEvent) {
    if event.r#type() == NSEventType::KeyDown {
        let close = STATE.with(|state| {
            state.borrow().as_ref().is_some_and(|state| {
                state.sidebar_mode.get() == Mode::Transient
                    && state.sidebar_keyboard.get()
                    && !sidebar_has_focus(state)
            })
        });
        if close {
            set_sidebar_mode(Mode::Closed);
        }
    }
}

pub(crate) fn width() -> f64 {
    STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map_or(habits::SIDEBAR_WIDTH, |state| state.sidebar_width.get())
    })
}

pub(crate) fn pinned() -> bool {
    STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .is_some_and(|state| state.sidebar_mode.get() == Mode::Pinned)
    })
}

pub(crate) fn resized(width: Option<f64>) {
    STATE.with(|state| {
        if let Some(state) = state.borrow().as_ref() {
            if let Some(width) = width {
                state.sidebar_width.set(width.clamp(MIN_WIDTH, MAX_WIDTH));
            } else if state.sidebar_mode.get() == Mode::Pinned {
                state.sidebar_mode.set(Mode::Closed);
            }
        }
    });
}

fn place(view: &NSView, frame: NSRect, animated: bool) {
    if animated {
        view.animator().setFrame(frame);
    } else if view.frame() != frame {
        view.setFrame(frame);
    }
}

pub(crate) fn layout(size: NSSize, inset: f64, animated: bool) -> f64 {
    STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else {
            return habits::SIDEBAR_WIDTH + INSET;
        };
        let mode = state.sidebar_mode.get();
        let pinned = mode == Mode::Pinned;
        let open = mode != Mode::Closed;
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
            animated,
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
            animated,
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
                chrome_view::symbol_image(if open { "chevron.up" } else { "chevron.down" }, 10.0)
                    .as_deref(),
            );
        }
        if let Some(toggle) = &state.toggle {
            toggle.setState(if pinned { 1 } else { 0 });
            toggle.setFrame(NSRect::new(
                NSPoint::new(edge - 32.0, header_y + 4.0),
                NSSize::new(TOGGLE_WIDTH, TOGGLE_HEIGHT),
            ));
            toggle.setToolTip(Some(&NSString::from_str(if pinned {
                "Unpin sidebar (⌘B)"
            } else {
                "Pin sidebar (⌘B)"
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
        edge
    })
}

fn chip_title(repo: &sidebar::Repo, row: &sidebar::Row) -> String {
    if repo.has_heading {
        format!("{} / {}", repo.name, row.label)
    } else {
        row.label.clone()
    }
}

pub(crate) fn first_row() -> Option<(String, String)> {
    STATE.with(|state| {
        let state = state.borrow();
        let row = state.as_ref()?.repos.first()?.rows.first()?;
        Some((row.path.to_string_lossy().into_owned(), row.label.clone()))
    })
}

fn add_repo() {
    prepare_action();
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
    refresh();
}

pub(crate) fn toggle() {
    let pinned = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .is_some_and(|state| state.sidebar_mode.get() == Mode::Pinned)
    });
    set_sidebar_mode(if pinned { Mode::Closed } else { Mode::Pinned });
}

fn set_sidebar_mode(mode: Mode) {
    cancel_sidebar_intent();
    let update = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let restore_focus = mode == Mode::Closed && sidebar_has_focus(state);
        state.sidebar_mode.set(mode);
        state.sidebar_entered.set(false);
        if mode == Mode::Closed {
            state.sidebar_keyboard.set(false);
        }
        Some((state.layout, state.restore_focus, restore_focus))
    });
    if let Some((layout, restore, restore_focus)) = update {
        layout(true);
        if restore_focus {
            restore();
        }
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
    let Some(target) =
        STATE.with(|state| state.borrow().as_ref().map(|state| state.actions.clone()))
    else {
        return;
    };
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

pub(crate) fn deactivate() {
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
            .is_some_and(|state| state.sidebar_mode.get() == Mode::Transient)
    });
    if transient {
        set_sidebar_mode(Mode::Closed);
    }
}

fn contains(rect: NSRect, point: NSPoint) -> bool {
    point.x >= rect.origin.x
        && point.x < rect.origin.x + rect.size.width
        && point.y >= rect.origin.y
        && point.y < rect.origin.y + rect.size.height
}

pub(crate) fn handle_event(event: &NSEvent) -> bool {
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
            if region == 0 && mode == Mode::Transient {
                set_sidebar_mode(Mode::Closed);
            }
        } else if NSEvent::pressedMouseButtons() != 0 {
            cancel_sidebar_intent();
        } else if region != previous && mode != Mode::Pinned {
            cancel_sidebar_intent();
            if region == 1 && mode == Mode::Closed {
                schedule_sidebar_intent(sel!(openSidebar:), 0.15);
            } else if region == 1 && entered {
                schedule_sidebar_intent(sel!(foldSidebar:), 0.15);
            } else if region == 0 && mode == Mode::Transient {
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
        if mode == Mode::Transient {
            set_sidebar_mode(Mode::Closed);
            restore_focus();
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
    if mode != Mode::Closed
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
                .filter(|repo| !repo.has_heading || !state.collapsed_repos.contains(&repo.path))
                .flat_map(|repo| &repo.rows)
                .nth(index - 1)
                .map(|row| (row.path.to_string_lossy().into_owned(), row.label.clone()))
        });
        if let Some((path, label)) = target {
            activate(&path, &label);
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
            if mode == Mode::Closed {
                set_sidebar_mode(Mode::Transient);
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
                        && state.sidebar_mode.get() != Mode::Closed
                        && index <= 9)
                        .then_some(index),
                );
            }
        }
    });
}

pub(crate) fn set_repos(repos: Vec<sidebar::Repo>) {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.repos = repos;
        }
    });
    rebuild();
}

pub(crate) fn refresh() {
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
        refresh();
    }
}

pub(crate) fn rebuild() {
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
            if repo.has_heading {
                height += HEADER_HEIGHT;
                if !state.collapsed_repos.contains(&repo.path) {
                    height += repo.rows.len() as f64 * ROW_HEIGHT;
                }
            } else {
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
            let collapsed = repo.has_heading && state.collapsed_repos.contains(&repo.path);

            if repo.has_heading {
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
                    move || toggle_repo(path.clone()),
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
            }

            for row in &repo.rows {
                let path = row.path.to_string_lossy().into_owned();
                let frame = NSRect::new(
                    NSPoint::new(6.0, y + 2.0),
                    NSSize::new(width - 12.0, ROW_HEIGHT - 2.0),
                );
                let opened = state.opened.contains(&path);
                let target = path.clone();
                let label = row.label.clone();
                let view = ClickView::new(mtm, frame, &row.label, 48.0, 34.0, move || {
                    select(&target, &label)
                });
                view.set_font(&NSFont::systemFontOfSize(13.0));
                view.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
                view.set_selected(state.current.as_deref() == Some(path.as_str()));
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
        let current = state.current.as_deref();
        let title = state.repos.iter().find_map(|repo| {
            repo.rows
                .iter()
                .find(|row| current == Some(row.path.to_string_lossy().as_ref()))
                .map(|row| chip_title(repo, row))
        });
        state
            .workspace_chip
            .set_text(title.as_deref().unwrap_or("Workspaces"));
    });
    update_workspace_hints();
    request_layout(false);
}

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
            }) {
                let y = header.frame().origin.y;
                if y <= 8.0 {
                    continue;
                }
                NSBezierPath::fillRect(NSRect::new(
                    NSPoint::new(6.0, y - 8.5),
                    NSSize::new((self.bounds().size.width - 12.0).max(0.0), 0.5),
                ));
            }
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {
            refresh();
        }
    }
);
