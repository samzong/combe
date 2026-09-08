use crate::habits;
use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSBezierPath, NSColor, NSEvent, NSFont, NSLineBreakMode,
    NSTextField, NSView,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use std::cell::{Cell, RefCell};

const PILL_RADIUS: f64 = 6.0;
const SESSION_DOT: f64 = 6.0;

pub(crate) struct ClickIvars {
    click: Box<dyn Fn()>,
    label: RefCell<Option<Retained<NSTextField>>>,
    selected: Cell<bool>,
    opened: Cell<Option<bool>>,
    dim_when_idle: Cell<bool>,
    warn: RefCell<Option<Retained<NSColor>>>,
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
            if self.ivars().selected.get() {
                NSColor::labelColor().colorWithAlphaComponent(0.12).setFill();
                NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
                    self.bounds(),
                    PILL_RADIUS,
                    PILL_RADIUS,
                )
                .fill();
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
                NSPoint::new(6.0, y),
                NSSize::new(SESSION_DOT, SESSION_DOT),
            ))
            .fill();
        }

        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {
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
        };
        let this = Self::alloc(mtm).set_ivars(ivars);
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };

        let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
        label.setFrame(NSRect::new(
            NSPoint::new(indent, (frame.size.height - habits::LINE_HEIGHT) / 2.0),
            NSSize::new(
                (frame.size.width - indent - trail).max(0.0),
                habits::LINE_HEIGHT,
            ),
        ));
        label.setFont(Some(&NSFont::systemFontOfSize(habits::FONT_SIZE)));
        label.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
        label.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
        this.addSubview(&label);
        *this.ivars().label.borrow_mut() = Some(label);
        this
    }

    pub(crate) fn dim_when_idle(&self) {
        self.ivars().dim_when_idle.set(true);
        self.apply_label_color();
    }

    pub(crate) fn set_selected(&self, selected: bool) {
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

    pub(crate) fn size_to_text(&self) {
        let Some(label) = self.ivars().label.borrow().clone() else {
            return;
        };
        label.setAutoresizingMask(NSAutoresizingMaskOptions(0));
        label.sizeToFit();
        let text = label.frame().size;
        let width = text.width + 16.0;
        let mut frame = self.frame();
        frame.size.width = width;
        self.setFrame(frame);
        label.setFrame(NSRect::new(
            NSPoint::new(8.0, ((frame.size.height - text.height) / 2.0).max(0.0)),
            text,
        ));
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
        let color = if selected {
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

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeFlippedView"]
    pub(crate) struct FlippedView;

    impl FlippedView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }
    }
);
