use crate::chrome_view::{ClickView, FlippedView};
use crate::{ghostty, habits, quota};
use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAccessibility, NSAutoresizingMaskOptions, NSBezierPath, NSColor, NSFont, NSFontWeightRegular,
    NSPopover, NSPopoverBehavior, NSPopoverDelegate, NSTextField, NSView, NSViewController,
};
use objc2_foundation::{
    MainThreadMarker, NSNotification, NSPoint, NSRect, NSRectEdge, NSSize, NSString,
};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

const STATUS_HEIGHT: f64 = 24.0;
const STATUS_FONT: f64 = 10.0;

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static QUOTA_FETCHING: Cell<bool> = const { Cell::new(false) };
    static QUOTA_LAST: Cell<Option<Instant>> = const { Cell::new(None) };
    static QUOTA_REBUILDING: Cell<bool> = const { Cell::new(false) };
}
static QUOTA_EPOCH: AtomicU64 = AtomicU64::new(0);
static QUOTA_READER: Mutex<Option<quota::Reader>> = Mutex::new(None);
static QUOTA_INCOMING: Mutex<Option<quota::Refresh>> = Mutex::new(None);

struct State {
    live: fn() -> bool,
    layout: fn(),
    bar: Retained<StatusView>,
    chip: Option<Retained<ClickView>>,
    popover: Option<Retained<NSPopover>>,
    _delegate: Retained<PopoverDelegate>,
    open: bool,
    claude: Option<quota::Quota>,
    codex: Option<quota::Quota>,
}

pub(crate) fn mount(
    mtm: MainThreadMarker,
    parent: &NSView,
    width: f64,
    live: fn() -> bool,
    layout: fn(),
) -> f64 {
    let status_h = 0.0;
    let status = StatusView::alloc(mtm).set_ivars(());
    let status: Retained<StatusView> = unsafe {
        msg_send![
            super(status),
            initWithFrame: NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(width, status_h),
            )
        ]
    };
    status.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewMaxYMargin,
    );
    status.setHidden(status_h == 0.0);
    parent.addSubview(&status);

    let popover_delegate = PopoverDelegate::alloc(mtm).set_ivars(());
    let popover_delegate: Retained<PopoverDelegate> =
        unsafe { msg_send![super(popover_delegate), init] };

    STATE.with(|state| {
        *state.borrow_mut() = Some(State {
            live,
            layout,
            bar: status,
            chip: None,
            popover: None,
            _delegate: popover_delegate,
            open: false,
            claude: None,
            codex: None,
        });
    });
    status_h
}

pub(crate) fn start() {
    rebuild_status();
    request_quota(true);
}

pub(crate) fn refresh() {
    request_quota(false);
}

pub(crate) fn layout(width: f64) -> f64 {
    STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else {
            return 0.0;
        };
        let height = if quota_bar_parts(state).is_empty() {
            0.0
        } else {
            STATUS_HEIGHT
        };
        state.bar.setHidden(height == 0.0);
        state.bar.setFrame(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(width, height),
        ));
        height
    })
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeStatusView"]
    #[ivars = ()]
    struct StatusView;

    impl StatusView {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let bounds = self.bounds();
            if bounds.size.height < 1.0 {
                return;
            }
            NSColor::separatorColor().setFill();
            NSBezierPath::bezierPathWithRect(NSRect::new(
                NSPoint::new(0.0, bounds.size.height - 1.0),
                NSSize::new(bounds.size.width, 1.0),
            ))
            .fill();
        }
    }
);

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombePopoverDelegate"]
    #[ivars = ()]
    struct PopoverDelegate;

    unsafe impl NSObjectProtocol for PopoverDelegate {}

    unsafe impl NSPopoverDelegate for PopoverDelegate {
        #[unsafe(method(popoverDidClose:))]
        fn popover_did_close(&self, _notification: &NSNotification) {
            quota_popover_closed();
        }
    }
);

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
    if QUOTA_REBUILDING.replace(true) {
        return;
    }
    let mtm = MainThreadMarker::new().expect("main thread");
    let open = STATE.with(|state| {
        let state = state.borrow();
        state.as_ref().is_some_and(|state| state.open)
    });
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        for child in state.bar.subviews() {
            child.removeFromSuperview();
        }
        state.chip = None;
        let parts = quota_bar_parts(state);
        if parts.is_empty() {
            return;
        }
        let text = parts
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>()
            .join("   ");
        let used = parts.iter().map(|(_, used)| *used).fold(0.0_f64, f64::max);
        let chip = ClickView::new(
            mtm,
            NSRect::new(
                NSPoint::new(12.0, 1.0),
                NSSize::new(280.0, STATUS_HEIGHT - 2.0),
            ),
            &text,
            8.0,
            8.0,
            toggle_quota,
        );
        chip.dim_when_idle();
        chip.set_font(&NSFont::monospacedDigitSystemFontOfSize_weight(
            STATUS_FONT,
            unsafe { NSFontWeightRegular },
        ));
        chip.set_warn(quota_warn_color(used));
        chip.set_selected(open);
        chip.setAccessibilityLabel(Some(&NSString::from_str(&text)));
        chip.size_to_text();
        state.bar.addSubview(&chip);
        state.chip = Some(chip);
    });
    QUOTA_REBUILDING.set(false);
    if open {
        STATE.with(|state| {
            if let Some(state) = state.borrow_mut().as_mut() {
                state.open = true;
            }
        });
        present_quota_popover();
    }
}

fn toggle_quota() {
    let shown = STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else {
            return false;
        };
        state.open
            && state
                .popover
                .as_ref()
                .is_some_and(|popover| popover.isShown())
    });
    if shown {
        close_quota_popover();
        return;
    }
    present_quota_popover();
}

fn present_quota_popover() {
    let mtm = MainThreadMarker::new().expect("main thread");
    let Some((quotas, chip, delegate, existing)) = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let quotas: Vec<_> = quota::Provider::ALL
            .into_iter()
            .filter_map(|provider| quota_slot(state, provider).cloned())
            .collect();
        if quotas.is_empty() {
            return None;
        }
        Some((
            quotas,
            state.chip.clone()?,
            state._delegate.clone(),
            state.popover.clone(),
        ))
    }) else {
        return;
    };
    let (view, size) = quota_popover_content(mtm, &quotas);
    let controller = NSViewController::new(mtm);
    controller.setView(&view);
    let popover = existing.unwrap_or_else(|| {
        let popover = NSPopover::new(mtm);
        popover.setBehavior(NSPopoverBehavior::Transient);
        popover.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        popover
    });
    popover.setContentSize(size);
    popover.setContentViewController(Some(&controller));
    popover.showRelativeToRect_ofView_preferredEdge(chip.bounds(), &chip, NSRectEdge::MinY);
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        state.popover = Some(popover);
        state.open = true;
    });
    set_quota_chip_selected(true);
}

fn set_quota_chip_selected(open: bool) {
    STATE.with(|state| {
        let state = state.borrow();
        let Some(state) = state.as_ref() else { return };
        if let Some(chip) = &state.chip {
            chip.set_selected(open);
        }
    });
}

fn close_quota_popover() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        if let Some(popover) = state.popover.take()
            && popover.isShown()
        {
            popover.close();
        }
        state.open = false;
    });
    set_quota_chip_selected(false);
}

fn quota_popover_closed() {
    if QUOTA_REBUILDING.get() {
        return;
    }
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        state.open = false;
        state.popover = None;
    });
    set_quota_chip_selected(false);
}

fn quota_popover_content(
    mtm: MainThreadMarker,
    quotas: &[quota::Quota],
) -> (Retained<NSView>, NSSize) {
    let now = std::time::SystemTime::now();
    let groups: Vec<(&quota::Quota, Vec<quota::Detail>)> = quotas
        .iter()
        .map(|quota| (quota, quota::details(quota, now)))
        .filter(|(_, rows)| !rows.is_empty())
        .collect();
    let pad = 12.0;
    let line = STATUS_FONT + 6.0;
    let row_h = line + 4.0;
    let title_h = line + 8.0;
    let gap = 10.0;
    let width = 228.0;
    let height = pad
        + groups
            .iter()
            .enumerate()
            .map(|(index, (_, rows))| {
                title_h
                    + rows.len() as f64 * row_h
                    + if index + 1 < groups.len() { gap } else { 0.0 }
            })
            .sum::<f64>()
        + pad;
    let view = FlippedView::alloc(mtm);
    let view: Retained<FlippedView> = unsafe {
        msg_send![
            view,
            initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height))
        ]
    };
    let font =
        NSFont::monospacedDigitSystemFontOfSize_weight(STATUS_FONT, unsafe { NSFontWeightRegular });
    let mut y = pad;
    for (index, (quota, rows)) in groups.iter().enumerate() {
        add_popover_label(
            mtm,
            &view,
            quota.provider.name(),
            NSRect::new(NSPoint::new(pad, y), NSSize::new(width - pad * 2.0, line)),
            &NSFont::boldSystemFontOfSize(STATUS_FONT),
            &NSColor::labelColor(),
        );
        y += title_h;
        for row in rows {
            add_popover_label(
                mtm,
                &view,
                &row.label,
                NSRect::new(NSPoint::new(pad, y), NSSize::new(48.0, line)),
                &font,
                &NSColor::secondaryLabelColor(),
            );
            add_popover_label(
                mtm,
                &view,
                &row.percent,
                NSRect::new(NSPoint::new(pad + 52.0, y), NSSize::new(48.0, line)),
                &font,
                &quota_warn_color(row.used).unwrap_or_else(NSColor::labelColor),
            );
            add_popover_label(
                mtm,
                &view,
                &row.reset,
                NSRect::new(
                    NSPoint::new(pad + 104.0, y),
                    NSSize::new(width - pad - 104.0, line),
                ),
                &font,
                &NSColor::secondaryLabelColor(),
            );
            y += row_h;
        }
        if index + 1 < groups.len() {
            y += gap;
        }
    }
    (Retained::into_super(view), NSSize::new(width, height))
}

fn add_popover_label(
    mtm: MainThreadMarker,
    parent: &NSView,
    text: &str,
    frame: NSRect,
    font: &NSFont,
    color: &NSColor,
) {
    let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    label.setFrame(frame);
    label.setFont(Some(font));
    label.setTextColor(Some(color));
    parent.addSubview(&label);
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
        close_quota_popover();
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
