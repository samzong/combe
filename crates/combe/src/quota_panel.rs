use crate::chrome_view::{ClickView, glass};
use crate::{ghostty, habits, quota};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAccessibility, NSAnimatablePropertyContainer, NSAnimationContext, NSApplication,
    NSAutoresizingMaskOptions, NSColor, NSEvent, NSEventModifierFlags, NSEventType, NSFont,
    NSFontWeightRegular, NSFontWeightSemibold, NSTextAlignment, NSTextField, NSView,
    NSVisualEffectView, NSWindowOrderingMode, NSWorkspace,
};
use objc2_foundation::{
    MainThreadMarker, NSObjectNSDelayedPerforming, NSPoint, NSRect, NSSize, NSString,
};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

const STATUS_HEIGHT: f64 = 40.0;
const STATUS_FONT: f64 = 12.0;
const CHIP_HEIGHT: f64 = 28.0;
const DETAILS_WIDTH: f64 = 304.0;

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static QUOTA_FETCHING: Cell<bool> = const { Cell::new(false) };
    static QUOTA_LAST: Cell<Option<Instant>> = const { Cell::new(None) };
}
static QUOTA_EPOCH: AtomicU64 = AtomicU64::new(0);
static QUOTA_READER: Mutex<Option<quota::Reader>> = Mutex::new(None);
static QUOTA_INCOMING: Mutex<Option<quota::Refresh>> = Mutex::new(None);

struct State {
    live: fn() -> bool,
    layout: fn(),
    restore_focus: fn(),
    bar: Retained<StatusView>,
    panel: Retained<NSVisualEffectView>,
    chip: Option<Retained<ClickView>>,
    details: Option<Retained<DetailsView>>,
    open: bool,
    keyboard: bool,
    hovered: Option<bool>,
    pending: Option<bool>,
    claude: Option<quota::Quota>,
    codex: Option<quota::Quota>,
}

pub(crate) fn mount(
    mtm: MainThreadMarker,
    parent: &NSView,
    width: f64,
    live: fn() -> bool,
    layout: fn(),
    restore_focus: fn(),
) -> f64 {
    let status = StatusView::alloc(mtm).set_ivars(());
    let status: Retained<StatusView> = unsafe {
        msg_send![super(status), initWithFrame: NSRect::new(
            NSPoint::new(0.0, 0.0), NSSize::new(width, 0.0))]
    };
    status.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewMaxYMargin,
    );
    status.setHidden(true);
    parent.addSubview(&status);
    let panel = glass(mtm, NSRect::default(), 14.0);
    status.addSubview(&panel);
    STATE.with(|state| {
        *state.borrow_mut() = Some(State {
            live,
            layout,
            restore_focus,
            bar: status,
            panel,
            chip: None,
            details: None,
            open: false,
            keyboard: false,
            hovered: None,
            pending: None,
            claude: None,
            codex: None,
        });
    });
    0.0
}

pub(crate) fn start() {
    rebuild_status();
    request_quota(true);
}

pub(crate) fn refresh() {
    request_quota(false);
}

pub(crate) fn layout(width: f64, content_inset: f64) -> f64 {
    STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else {
            return 0.0;
        };
        let shown = state.chip.is_some();
        state
            .panel
            .setFrameOrigin(NSPoint::new(content_inset + 6.0, 12.0));
        let height = state
            .details
            .as_ref()
            .map_or(0.0, |view| view.frame().size.height);
        state.bar.setHidden(!shown);
        state.bar.setFrame(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(width, if shown { STATUS_HEIGHT + height } else { 0.0 }),
        ));
        if let Some(parent) = unsafe { state.bar.superview() } {
            parent.addSubview_positioned_relativeTo(&state.bar, NSWindowOrderingMode::Above, None);
        }
        if shown { STATUS_HEIGHT } else { 0.0 }
    })
}

pub(crate) fn deactivate() {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            cancel_hover(state);
            state.hovered = None;
        }
    });
}

pub(crate) fn handle_event(event: &NSEvent) -> bool {
    let kind = event.r#type();
    let snapshot = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        state.chip.as_ref()?;
        let point = state
            .panel
            .convertPoint_fromView(event.locationInWindow(), None);
        let size = state.panel.bounds().size;
        let inside =
            point.x >= 0.0 && point.y >= 0.0 && point.x < size.width && point.y < size.height;
        Some((inside, state.open, focus_inside(state), state.bar.clone()))
    });
    let Some((inside, open, focused, bar)) = snapshot else {
        return false;
    };
    if matches!(
        kind,
        NSEventType::LeftMouseDragged
            | NSEventType::RightMouseDragged
            | NSEventType::OtherMouseDragged
    ) {
        deactivate();
    } else if matches!(
        kind,
        NSEventType::MouseMoved | NSEventType::MouseEntered | NSEventType::MouseExited
    ) {
        if NSEvent::pressedMouseButtons() != 0 {
            deactivate();
            return false;
        }
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let Some(state) = state.as_mut() else { return };
            if state.hovered == Some(inside) {
                return;
            }
            state.hovered = Some(inside);
            cancel_hover(state);
            if inside != state.open {
                state.pending = Some(inside);
                unsafe {
                    state.bar.performSelector_withObject_afterDelay(
                        sel!(applyHover:),
                        None,
                        if inside { 0.15 } else { 0.25 },
                    );
                }
            }
        });
    } else if matches!(
        kind,
        NSEventType::LeftMouseDown | NSEventType::RightMouseDown | NSEventType::OtherMouseDown
    ) {
        STATE.with(|state| {
            if let Some(state) = state.borrow_mut().as_mut() {
                cancel_hover(state);
                state.keyboard = false;
                state.hovered = Some(inside);
            }
        });
        if !inside {
            set_open(false);
        }
    } else if kind == NSEventType::KeyDown {
        let key = event.keyCode();
        if key == 53 {
            STATE.with(|state| {
                if let Some(state) = state.borrow_mut().as_mut() {
                    cancel_hover(state);
                }
            });
            if open {
                focus_chip();
                set_open(false);
                return true;
            }
        }
        if focused {
            STATE.with(|state| {
                if let Some(state) = state.borrow_mut().as_mut() {
                    cancel_hover(state);
                    state.keyboard = true;
                }
            });
            let plain = !event.modifierFlags().intersects(
                NSEventModifierFlags::Command
                    | NSEventModifierFlags::Control
                    | NSEventModifierFlags::Option,
            );
            if key == 126 && plain {
                set_open(true);
                let details = STATE.with(|state| state.borrow().as_ref()?.details.clone());
                if let Some(details) = details
                    && let Some(window) = details.window()
                {
                    window.makeFirstResponder(Some(&*details));
                }
                return true;
            }
            if matches!(key, 36 | 49) && plain {
                focus_chip();
                set_open(!open);
                return true;
            }
            if key == 48
                && plain
                && let Some(window) = bar.window()
            {
                if event.modifierFlags().contains(NSEventModifierFlags::Shift) {
                    window.selectPreviousKeyView(None);
                } else {
                    window.selectNextKeyView(None);
                }
            }
            unsafe {
                bar.performSelector_withObject_afterDelay(sel!(checkFocus:), None, 0.0);
            }
            if key == 48 && plain {
                return true;
            }
        }
    }
    false
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeStatusView"]
    #[ivars = ()]
    struct StatusView;

    impl StatusView {
        #[unsafe(method_id(hitTest:))]
        fn hit_test(&self, point: NSPoint) -> Option<Retained<NSView>> {
            let hit: Option<Retained<NSView>> = unsafe { msg_send![super(self), hitTest: point] };
            let own: &NSView = self;
            hit.filter(|view| !std::ptr::eq(&**view, own))
        }

        #[unsafe(method(applyHover:))]
        fn apply_hover(&self, _sender: Option<&AnyObject>) {
            let next = STATE.with(|state| {
                let mut state = state.borrow_mut();
                let state = state.as_mut()?;
                let open = state.pending.take()?;
                (open || !(state.keyboard && focus_inside(state))).then_some(open)
            });
            if let Some(open) = next { set_open(open); }
        }

        #[unsafe(method(checkFocus:))]
        fn check_focus(&self, _sender: Option<&AnyObject>) {
            let close = STATE.with(|state| state.borrow().as_ref().is_some_and(|state| {
                state.open && state.keyboard && !focus_inside(state)
            }));
            if close { set_open(false); }
        }
    }
);

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeQuotaDetails"]
    struct DetailsView;

    impl DetailsView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }

        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool { true }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {}

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, _event: &NSEvent) {}
    }
);

fn cancel_hover(state: &mut State) {
    state.pending = None;
    unsafe {
        NSObject::cancelPreviousPerformRequestsWithTarget_selector_object(
            state.bar.as_ref(),
            sel!(applyHover:),
            None,
        );
    }
}

fn focus_inside(state: &State) -> bool {
    state
        .panel
        .window()
        .and_then(|window| window.firstResponder())
        .and_then(|responder| responder.downcast::<NSView>().ok())
        .is_some_and(|view| view.isDescendantOf(&state.panel))
}

fn focus_chip() {
    let chip = STATE.with(|state| state.borrow().as_ref()?.chip.clone());
    if let Some(chip) = chip
        && let Some(window) = chip.window()
    {
        window.makeFirstResponder(Some(&*chip));
    }
}

fn quota_slot(state: &State, provider: quota::Provider) -> Option<&quota::Quota> {
    match provider {
        quota::Provider::Claude => state.claude.as_ref(),
        quota::Provider::Codex => state.codex.as_ref(),
    }
}

fn quota_bar_parts(state: &State) -> Vec<(String, f64)> {
    quota::Provider::ALL
        .into_iter()
        .filter_map(|provider| quota_slot(state, provider).and_then(quota::chip))
        .collect()
}

fn rebuild_status() {
    let mtm = MainThreadMarker::new().expect("main thread");
    let focused = STATE.with(|state| state.borrow().as_ref().is_some_and(focus_inside));
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        cancel_hover(state);
        for child in state.panel.subviews() {
            child.removeFromSuperview();
        }
        state.chip = None;
        state.details = None;
        let parts = quota_bar_parts(state);
        if parts.is_empty() {
            state.open = false;
            state.keyboard = false;
            state.bar.setHidden(true);
            return;
        }
        let text = parts
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>()
            .join("   ");
        let used = parts.iter().map(|(_, used)| *used).fold(0.0_f64, f64::max);
        let font = NSFont::monospacedDigitSystemFontOfSize_weight(STATUS_FONT, unsafe {
            NSFontWeightRegular
        });
        let measure = NSTextField::labelWithString(&NSString::from_str(&text), mtm);
        measure.setFont(Some(&font));
        measure.sizeToFit();
        let chip = ClickView::new(
            mtm,
            NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(measure.frame().size.width + 28.0, CHIP_HEIGHT),
            ),
            &text,
            14.0,
            14.0,
            activate_quota,
        );
        chip.dim_when_idle();
        chip.disable_hover_highlight();
        chip.set_font(&font);
        chip.set_warn(quota_warn_color(used));
        chip.setAccessibilityElement(true);
        chip.setAccessibilityRole(Some(&NSString::from_str("AXButton")));
        chip.setAccessibilityLabel(Some(&NSString::from_str(&format!(
            "Quota remaining: {text}"
        ))));
        chip.setAccessibilityExpanded(state.open);
        state.panel.addSubview(&chip);
        state.chip = Some(chip);
        rebuild_details(state);
    });
    update_panel(false);
    if focused {
        let restore = STATE.with(|state| {
            state
                .borrow()
                .as_ref()
                .and_then(|state| state.chip.is_none().then_some(state.restore_focus))
        });
        if let Some(restore) = restore {
            restore();
        } else {
            focus_chip();
        }
    }
}

fn rebuild_details(state: &mut State) {
    if let Some(details) = state.details.take() {
        details.removeFromSuperview();
    }
    let quotas: Vec<_> = quota::Provider::ALL
        .into_iter()
        .filter_map(|provider| quota_slot(state, provider).cloned())
        .collect();
    let details = quota_details(MainThreadMarker::new().expect("main thread"), &quotas);
    details.setHidden(!state.open);
    state.panel.addSubview(&details);
    state.details = Some(details);
}

fn activate_quota() {
    let mtm = MainThreadMarker::new().expect("main thread");
    let keyboard = NSApplication::sharedApplication(mtm)
        .currentEvent()
        .is_none_or(|event| event.r#type() != NSEventType::LeftMouseDown);
    let open = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else {
            return false;
        };
        state.keyboard = keyboard;
        !keyboard || !state.open
    });
    set_open(open);
    if keyboard {
        focus_chip();
    }
}

fn set_open(open: bool) {
    let changed = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else {
            return false;
        };
        cancel_hover(state);
        if state.open == open || state.chip.is_none() {
            return false;
        }
        state.open = open;
        if open {
            rebuild_details(state);
        } else {
            state.keyboard = false;
        }
        true
    });
    if changed {
        update_panel(true);
    }
}

fn update_panel(animated: bool) {
    STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else { return };
        let Some(chip) = &state.chip else { return };
        let details_height = state
            .details
            .as_ref()
            .map_or(0.0, |view| view.frame().size.height);
        let frame = NSRect::new(
            NSPoint::new(state.panel.frame().origin.x, 12.0),
            NSSize::new(
                if state.open {
                    DETAILS_WIDTH
                } else {
                    chip.frame().size.width
                },
                CHIP_HEIGHT + if state.open { details_height } else { 0.0 },
            ),
        );
        let mut bar_frame = state.bar.frame();
        bar_frame.size.height = STATUS_HEIGHT + details_height;
        state.bar.setFrame(bar_frame);
        state.bar.setHidden(false);
        chip.setAccessibilityExpanded(state.open);
        if let Some(details) = &state.details {
            details.setHidden(!state.open);
        }
        let duration = if animated
            && !NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
        {
            0.34
        } else {
            0.0
        };
        NSAnimationContext::beginGrouping();
        NSAnimationContext::currentContext().setDuration(duration);
        NSAnimationContext::currentContext().setAllowsImplicitAnimation(true);
        state.panel.animator().setFrame(frame);
        NSAnimationContext::endGrouping();
    });
}

fn quota_details(mtm: MainThreadMarker, quotas: &[quota::Quota]) -> Retained<DetailsView> {
    let now = std::time::SystemTime::now();
    let groups: Vec<_> = quotas
        .iter()
        .map(|quota| (quota, quota::details(quota, now)))
        .filter(|(_, rows)| !rows.is_empty())
        .collect();
    let height = 32.0
        + groups
            .iter()
            .map(|(_, rows)| 24.0 + rows.len() as f64 * 30.0)
            .sum::<f64>()
        + groups.len().saturating_sub(1) as f64 * 16.0;
    let view = DetailsView::alloc(mtm);
    let view: Retained<DetailsView> = unsafe {
        msg_send![view, initWithFrame: NSRect::new(
        NSPoint::new(0.0, CHIP_HEIGHT), NSSize::new(DETAILS_WIDTH, height))]
    };
    view.setAccessibilityElement(true);
    view.setAccessibilityRole(Some(&NSString::from_str("AXGroup")));
    view.setAccessibilityLabel(Some(&NSString::from_str("Quota details")));
    let font =
        NSFont::monospacedDigitSystemFontOfSize_weight(STATUS_FONT, unsafe { NSFontWeightRegular });
    let heading = NSFont::systemFontOfSize_weight(13.0, unsafe { NSFontWeightSemibold });
    let mut y = 18.0;
    for (quota, rows) in groups {
        add_label(
            mtm,
            &view,
            quota.provider.name(),
            NSRect::new(NSPoint::new(18.0, y), NSSize::new(268.0, 18.0)),
            &heading,
            &NSColor::labelColor(),
        );
        y += 24.0;
        for row in rows {
            let label = add_label(
                mtm,
                &view,
                &row.label,
                NSRect::new(NSPoint::new(18.0, y + 6.0), NSSize::new(50.0, 18.0)),
                &font,
                &NSColor::secondaryLabelColor(),
            );
            label.setAccessibilityLabel(Some(&NSString::from_str(&format!(
                "{}, {} remaining, resets in {}",
                row.label, row.percent, row.reset
            ))));
            let percent = add_label(
                mtm,
                &view,
                &row.percent,
                NSRect::new(NSPoint::new(78.0, y + 6.0), NSSize::new(124.0, 18.0)),
                &NSFont::monospacedDigitSystemFontOfSize_weight(13.0, unsafe {
                    objc2_app_kit::NSFontWeightMedium
                }),
                &quota_warn_color(row.used).unwrap_or_else(NSColor::labelColor),
            );
            percent.setAccessibilityElement(false);
            let reset = add_label(
                mtm,
                &view,
                &row.reset,
                NSRect::new(NSPoint::new(212.0, y + 6.0), NSSize::new(74.0, 18.0)),
                &font,
                &NSColor::secondaryLabelColor(),
            );
            reset.setAlignment(NSTextAlignment::Right);
            reset.setAccessibilityElement(false);
            y += 30.0;
        }
        y += 16.0;
    }
    view
}

fn add_label(
    mtm: MainThreadMarker,
    parent: &NSView,
    text: &str,
    frame: NSRect,
    font: &NSFont,
    color: &NSColor,
) -> Retained<NSTextField> {
    let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    label.setFrame(frame);
    label.setFont(Some(font));
    label.setTextColor(Some(color));
    parent.addSubview(&label);
    label
}

fn quota_warn_color(used: f64) -> Option<Retained<NSColor>> {
    match quota::warn(used) {
        quota::Warn::Red => Some(NSColor::systemRedColor()),
        quota::Warn::Orange => Some(NSColor::systemOrangeColor()),
        quota::Warn::None => None,
    }
}

fn request_quota(force: bool) {
    if QUOTA_FETCHING.get() {
        return;
    }
    if !force {
        if let Some(last) = QUOTA_LAST.get()
            && last.elapsed() < habits::QUOTA_FRESH
        {
            return;
        }
        if !STATE.with(|state| state.borrow().as_ref().is_some_and(|state| (state.live)())) {
            schedule_poll();
            return;
        }
    }
    QUOTA_FETCHING.set(true);
    QUOTA_EPOCH.fetch_add(1, Ordering::Relaxed);
    let spawned = std::thread::Builder::new()
        .name("combe-quota".into())
        .spawn(|| {
            let refresh = QUOTA_READER
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .get_or_insert_with(quota::Reader::default)
                .refresh();
            let mut incoming = QUOTA_INCOMING
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            *incoming = Some(refresh);
            ghostty::on_main(apply_quota_on_main);
        });
    if spawned.is_err() {
        QUOTA_FETCHING.set(false);
    }
}

unsafe extern "C" fn apply_quota_on_main(_: *mut c_void) {
    apply_quota();
}

unsafe extern "C" fn poll_quota_on_main(_: *mut c_void) {
    request_quota(false);
}

fn apply_quota() {
    QUOTA_FETCHING.set(false);
    QUOTA_LAST.set(Some(Instant::now()));
    let refresh = {
        let mut incoming = QUOTA_INCOMING
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        incoming.take()
    };
    let Some(refresh) = refresh else {
        return;
    };
    let empty = refresh.claude.is_none() && refresh.codex.is_none();
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        state.claude = refresh.claude;
        state.codex = refresh.codex;
    });
    if empty {
        set_open(false);
    }
    rebuild_status();
    if let Some(layout) = STATE.with(|state| state.borrow().as_ref().map(|state| state.layout)) {
        layout();
    }
    schedule_poll();
}

fn schedule_poll() {
    let epoch = QUOTA_EPOCH.fetch_add(1, Ordering::Relaxed) + 1;
    std::thread::Builder::new()
        .name("combe-quota-wait".into())
        .spawn(move || {
            std::thread::sleep(habits::QUOTA_POLL);
            if QUOTA_EPOCH.load(Ordering::Relaxed) != epoch {
                return;
            }
            ghostty::on_main(poll_quota_on_main);
        })
        .ok();
}
