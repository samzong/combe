use crate::habits;
use objc2::rc::Retained;
use objc2::{AnyThread, DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAccessibility, NSApplication, NSAutoresizingMaskOptions, NSBezierPath, NSColor, NSEvent,
    NSEventType, NSFont, NSImage, NSImageView, NSLineBreakMode, NSTextField, NSTrackingArea,
    NSTrackingAreaOptions, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSVisualEffectView,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use std::cell::{Cell, RefCell};

const PILL_RADIUS: f64 = 16.0;
const SESSION_DOT: f64 = 6.0;

pub(crate) struct ClickIvars {
    click: Box<dyn Fn()>,
    label: RefCell<Option<Retained<NSTextField>>>,
    selected: Cell<bool>,
    opened: Cell<Option<bool>>,
    dim_when_idle: Cell<bool>,
    warn: RefCell<Option<Retained<NSColor>>>,
    hovered: Cell<bool>,
    hover_highlight: Cell<bool>,
    shortcut: Cell<Option<usize>>,
    tracking: RefCell<Option<Retained<NSTrackingArea>>>,
    focus_visible: Cell<bool>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeClickView"]
    #[ivars = ClickIvars]
    pub(crate) struct ClickView;

    impl ClickView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            if self.ivars().selected.get() || (self.ivars().hovered.get() && self.ivars().hover_highlight.get()) {
                if self.ivars().hovered.get() && self.ivars().opened.get().is_some() {
                    NSColor::controlAccentColor().setFill();
                } else {
                    NSColor::labelColor().colorWithAlphaComponent(0.08).setFill();
                }
                NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
                    self.bounds(),
                    PILL_RADIUS,
                    PILL_RADIUS,
                )
                .fill();
            }
            if self.ivars().focus_visible.get() {
                let bounds = self.bounds();
                let frame = NSRect::new(NSPoint::new(2.0, 2.0), NSSize::new((bounds.size.width - 4.0).max(0.0), (bounds.size.height - 4.0).max(0.0)));
                NSColor::keyboardFocusIndicatorColor().setStroke();
                let ring = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(frame, 14.0, 14.0);
                ring.setLineWidth(2.0);
                ring.stroke();
            }
            let Some(opened) = self.ivars().opened.get() else {
                return;
            };
            let bounds = self.bounds();
            let y = ((bounds.size.height - SESSION_DOT) / 2.0).max(0.0);
            if opened {
                NSColor::systemGreenColor().setFill();
            } else {
                NSColor::tertiaryLabelColor().setFill();
            }
            NSBezierPath::bezierPathWithOvalInRect(NSRect::new(
                NSPoint::new(32.0, y),
                NSSize::new(SESSION_DOT, SESSION_DOT),
            ))
            .fill();
            let marker = if let Some(number) = self.ivars().shortcut.get() {
                Some(number.to_string())
            } else if self.ivars().selected.get() {
                Some("✓".into())
            } else {
                None
            };
            if let Some(marker) = marker {
                let field = NSTextField::labelWithString(&NSString::from_str(&marker), self.mtm());
                field.setFont(Some(&NSFont::monospacedDigitSystemFontOfSize_weight(11.0, unsafe { objc2_app_kit::NSFontWeightRegular })));
                field.setTextColor(Some(&NSColor::secondaryLabelColor()));
                let frame = NSRect::new(NSPoint::new(bounds.size.width - 27.0, (bounds.size.height - 18.0) / 2.0), NSSize::new(18.0, 18.0));
                if self.ivars().shortcut.get().is_some() {
                    NSColor::labelColor().colorWithAlphaComponent(0.05).setFill();
                    NSBezierPath::bezierPathWithOvalInRect(frame).fill();
                }
                field.setFrame(frame);
                field.setAlignment(objc2_app_kit::NSTextAlignment::Center);
                if let Some(cell) = field.cell() { cell.drawWithFrame_inView(frame, self); }
            }
        }

        #[unsafe(method(updateTrackingAreas))]
        fn update_tracking_areas(&self) {
            if self.ivars().tracking.borrow().is_none() {
                let area = unsafe { NSTrackingArea::initWithRect_options_owner_userInfo(NSTrackingArea::alloc(), self.bounds(), NSTrackingAreaOptions::MouseEnteredAndExited | NSTrackingAreaOptions::ActiveInKeyWindow | NSTrackingAreaOptions::InVisibleRect, Some(self), None) };
                self.addTrackingArea(&area);
                *self.ivars().tracking.borrow_mut() = Some(area);
            }
            let _: () = unsafe { msg_send![super(self), updateTrackingAreas] };
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) { self.set_hovered(true); }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) { self.set_hovered(false); }

        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool { true }

        #[unsafe(method(becomeFirstResponder))]
        fn become_first_responder(&self) -> bool {
            let accepted: bool = unsafe { msg_send![super(self), becomeFirstResponder] };
            self.ivars().focus_visible.set(accepted && NSApplication::sharedApplication(self.mtm()).currentEvent().is_none_or(|event| event.r#type() == NSEventType::KeyDown));
            self.setNeedsDisplay(true);
            accepted
        }

        #[unsafe(method(resignFirstResponder))]
        fn resign_first_responder(&self) -> bool {
            self.ivars().focus_visible.set(false);
            self.setNeedsDisplay(true);
            unsafe { msg_send![super(self), resignFirstResponder] }
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            if matches!(event.keyCode(), 36 | 49) { (self.ivars().click)(); }
            else { let _: () = unsafe { msg_send![super(self), keyDown: event] }; }
        }

        #[unsafe(method(hitTest:))]
        fn hit_test(&self, point: NSPoint) -> *mut NSView {
            let hit: *mut NSView = unsafe { msg_send![super(self), hitTest: point] };
            if hit.is_null() { hit } else { self as *const Self as *mut NSView }
        }

        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {
            if let Some(window) = self.window() { window.makeFirstResponder(Some(self)); }
            (self.ivars().click)();
        }

        #[unsafe(method(accessibilityPerformPress))]
        fn accessibility_perform_press(&self) -> bool {
            (self.ivars().click)();
            true
        }
    }
);

impl ClickView {
    pub(crate) fn new(
        mtm: MainThreadMarker,
        frame: NSRect,
        text: &str,
        indent: f64,
        trail: f64,
        click: impl Fn() + 'static,
    ) -> Retained<Self> {
        let ivars = ClickIvars {
            click: Box::new(click),
            label: RefCell::new(None),
            selected: Cell::new(false),
            opened: Cell::new(None),
            dim_when_idle: Cell::new(false),
            warn: RefCell::new(None),
            hovered: Cell::new(false),
            hover_highlight: Cell::new(true),
            shortcut: Cell::new(None),
            tracking: RefCell::new(None),
            focus_visible: Cell::new(false),
        };
        let this = Self::alloc(mtm).set_ivars(ivars);
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };

        let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
        label.setFrame(NSRect::new(
            NSPoint::new(
                indent,
                (frame.size.height - habits::CHROME_LINE_HEIGHT) / 2.0,
            ),
            NSSize::new(
                (frame.size.width - indent - trail).max(0.0),
                habits::CHROME_LINE_HEIGHT,
            ),
        ));
        label.setFont(Some(&NSFont::systemFontOfSize(habits::CHROME_FONT_SIZE)));
        label.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
        label.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
        this.addSubview(&label);
        *this.ivars().label.borrow_mut() = Some(label);
        this.setAccessibilityElement(true);
        this.setAccessibilityRole(Some(&NSString::from_str("AXButton")));
        this.setAccessibilityLabel(Some(&NSString::from_str(text)));
        this
    }

    pub(crate) fn set_text(&self, text: &str) {
        if let Some(label) = self.ivars().label.borrow().as_ref() {
            label.setStringValue(&NSString::from_str(text));
        }
        self.setAccessibilityLabel(Some(&NSString::from_str(text)));
    }

    pub(crate) fn set_shortcut(&self, shortcut: Option<usize>) {
        self.ivars().shortcut.set(shortcut);
        self.setNeedsDisplay(true);
    }

    fn set_hovered(&self, hovered: bool) {
        self.ivars().hovered.set(hovered);
        self.apply_label_color();
        self.setNeedsDisplay(true);
    }

    pub(crate) fn disable_hover_highlight(&self) {
        self.ivars().hover_highlight.set(false);
    }

    pub(crate) fn dim_when_idle(&self) {
        self.ivars().dim_when_idle.set(true);
        self.apply_label_color();
    }

    pub(crate) fn set_selected(&self, selected: bool) {
        self.setAccessibilitySelected(selected);
        self.ivars().selected.set(selected);
        self.apply_label_color();
        self.setNeedsDisplay(true);
    }

    pub(crate) fn set_opened(&self, opened: bool) {
        self.ivars().opened.set(Some(opened));
        self.setNeedsDisplay(true);
    }

    pub(crate) fn set_warn(&self, color: Option<Retained<NSColor>>) {
        *self.ivars().warn.borrow_mut() = color;
        self.apply_label_color();
    }

    pub(crate) fn set_font(&self, font: &NSFont) {
        if let Some(label) = self.ivars().label.borrow().as_ref() {
            label.setFont(Some(font));
        }
    }

    fn apply_label_color(&self) {
        let Some(label) = self.ivars().label.borrow().clone() else {
            return;
        };
        let selected = self.ivars().selected.get();
        let dim = self.ivars().dim_when_idle.get() && !selected;
        let color = if self.ivars().hovered.get() && self.ivars().opened.get().is_some() {
            NSColor::whiteColor()
        } else if selected {
            NSColor::labelColor()
        } else if let Some(warn) = self.ivars().warn.borrow().clone() {
            warn
        } else if dim {
            NSColor::secondaryLabelColor()
        } else {
            NSColor::labelColor()
        };
        label.setTextColor(Some(&color));
    }
}

pub(crate) fn glass(
    mtm: MainThreadMarker,
    frame: NSRect,
    radius: f64,
) -> Retained<NSVisualEffectView> {
    let view = NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm), frame);
    view.setBlendingMode(NSVisualEffectBlendingMode::WithinWindow);
    view.setMaterial(NSVisualEffectMaterial::Popover);
    view.setState(NSVisualEffectState::FollowsWindowActiveState);
    view.setWantsLayer(true);
    if let Some(layer) = view.layer() {
        let _: () = unsafe { msg_send![&*layer, setCornerRadius: radius] };
        layer.setMasksToBounds(true);
        let _: () = unsafe { msg_send![&*layer, setBorderWidth: 0.5_f64] };
        let color = NSColor::whiteColor()
            .colorWithAlphaComponent(0.15)
            .CGColor();
        let _: () = unsafe { msg_send![&*layer, setBorderColor: &*color] };
    }
    view
}

pub(crate) fn symbol(mtm: MainThreadMarker, parent: &NSView, name: &str, frame: NSRect) {
    if let Some(image) =
        NSImage::imageWithSystemSymbolName_accessibilityDescription(&NSString::from_str(name), None)
    {
        let view = NSImageView::initWithFrame(NSImageView::alloc(mtm), frame);
        view.setImage(Some(&image));
        view.setContentTintColor(Some(&NSColor::secondaryLabelColor()));
        parent.addSubview(&view);
    }
}
