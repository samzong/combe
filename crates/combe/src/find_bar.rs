use std::cell::{Cell, RefCell};

use objc2::rc::{Retained, Weak};
use objc2::runtime::{ProtocolObject, Sel};
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSAutoresizingMaskOptions, NSBezierPath, NSColor, NSControl,
    NSControlTextEditingDelegate, NSEvent, NSEventModifierFlags, NSFont, NSSearchField,
    NSSearchFieldDelegate, NSTextAlignment, NSTextField, NSTextFieldDelegate, NSTextView, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSNotification, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};

use crate::chrome_view::ClickView;
use crate::habits;
use crate::surface::SurfaceView;

pub const WIDTH: f64 = 340.0;
pub const HEIGHT: f64 = 34.0;
const INSET: f64 = 6.0;
const COUNT_WIDTH: f64 = 64.0;
const BUTTON_WIDTH: f64 = 22.0;
const RADIUS: f64 = 8.0;

type Action = fn(&FindBar);

pub struct FindBarIvars {
    field: RefCell<Option<Retained<NSSearchField>>>,
    count: RefCell<Option<Retained<NSTextField>>>,
    total: Cell<Option<usize>>,
    selected: Cell<Option<usize>>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeFindBar"]
    #[ivars = FindBarIvars]
    pub struct FindBar;

    unsafe impl NSObjectProtocol for FindBar {}

    unsafe impl NSControlTextEditingDelegate for FindBar {
        #[unsafe(method(controlTextDidChange:))]
        fn control_text_did_change(&self, _note: &NSNotification) {
            self.search();
        }

        #[unsafe(method(control:textView:doCommandBySelector:))]
        unsafe fn do_command(
            &self,
            _control: &NSControl,
            _text_view: &NSTextView,
            selector: Sel,
        ) -> bool {
            if selector == sel!(insertNewline:) {
                let mtm = MainThreadMarker::from(self);
                let shift = NSApplication::sharedApplication(mtm)
                    .currentEvent()
                    .is_some_and(|event| {
                        event
                            .modifierFlags()
                            .contains(NSEventModifierFlags::Shift)
                    });
                self.navigate(!shift);
                true
            } else if selector == sel!(insertLineBreak:) {
                self.navigate(false);
                true
            } else if selector == sel!(cancelOperation:) {
                self.act("end_search");
                true
            } else {
                false
            }
        }
    }

    unsafe impl NSTextFieldDelegate for FindBar {}

    unsafe impl NSSearchFieldDelegate for FindBar {}

    impl FindBar {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            NSColor::windowBackgroundColor().setFill();
            NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(self.bounds(), RADIUS, RADIUS)
                .fill();
            let bounds = self.bounds();
            let inner = NSRect::new(
                NSPoint::new(bounds.origin.x + 0.5, bounds.origin.y + 0.5),
                NSSize::new(bounds.size.width - 1.0, bounds.size.height - 1.0),
            );
            NSColor::separatorColor().setStroke();
            NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(inner, RADIUS, RADIUS).stroke();
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {}

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, _event: &NSEvent) {}

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, _event: &NSEvent) {}
    }
);

impl FindBar {
    pub fn new(mtm: MainThreadMarker, container: NSSize) -> Retained<Self> {
        let ivars = FindBarIvars {
            field: RefCell::new(None),
            count: RefCell::new(None),
            total: Cell::new(None),
            selected: Cell::new(None),
        };
        let this = Self::alloc(mtm).set_ivars(ivars);
        let width = WIDTH.min(container.width - 2.0 * INSET).max(0.0);
        let frame = NSRect::new(
            NSPoint::new(
                container.width - width - INSET,
                container.height - HEIGHT - INSET,
            ),
            NSSize::new(width, HEIGHT),
        );
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };
        this.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewMinXMargin | NSAutoresizingMaskOptions::ViewMinYMargin,
        );

        let buttons = 3.0 * BUTTON_WIDTH;
        let field_width = (width - INSET * 3.0 - COUNT_WIDTH - buttons).max(0.0);
        let field_height = habits::LINE_HEIGHT + 6.0;
        let y = (HEIGHT - field_height) / 2.0;

        let field = NSSearchField::initWithFrame(
            NSSearchField::alloc(mtm),
            NSRect::new(
                NSPoint::new(INSET, y),
                NSSize::new(field_width, field_height),
            ),
        );
        field.setPlaceholderString(Some(&NSString::from_str("Find")));
        field.setFont(Some(&NSFont::systemFontOfSize(habits::FONT_SIZE + 1.0)));
        field.setSendsSearchStringImmediately(true);
        unsafe { field.setDelegate(Some(ProtocolObject::from_ref(&*this))) };
        this.addSubview(&field);

        let count = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        count.setFrame(NSRect::new(
            NSPoint::new(INSET + field_width, (HEIGHT - habits::LINE_HEIGHT) / 2.0),
            NSSize::new(COUNT_WIDTH, habits::LINE_HEIGHT),
        ));
        count.setFont(Some(&NSFont::systemFontOfSize(habits::FONT_SIZE)));
        count.setAlignment(NSTextAlignment::Center);
        count.setTextColor(Some(&NSColor::secondaryLabelColor()));
        this.addSubview(&count);

        let weak = Weak::from_retained(&this);
        let mut x = INSET + field_width + COUNT_WIDTH;
        let actions: [(&str, Action); 3] = [
            ("\u{2039}", |bar| bar.navigate(false)),
            ("\u{203A}", |bar| bar.navigate(true)),
            ("\u{00D7}", |bar| bar.act("end_search")),
        ];
        for (glyph, action) in actions {
            let weak = weak.clone();
            let button = ClickView::new(
                mtm,
                NSRect::new(NSPoint::new(x, 0.0), NSSize::new(BUTTON_WIDTH, HEIGHT)),
                glyph,
                6.0,
                0.0,
                move || {
                    if let Some(bar) = weak.load() {
                        action(&bar);
                    }
                },
            );
            button.set_font(&NSFont::systemFontOfSize(habits::FONT_SIZE + 4.0));
            this.addSubview(&button);
            x += BUTTON_WIDTH;
        }

        *this.ivars().field.borrow_mut() = Some(field);
        *this.ivars().count.borrow_mut() = Some(count);
        this.update_count();
        this
    }

    pub fn focus(&self, needle: &str) {
        let Some(field) = self.ivars().field.borrow().clone() else {
            return;
        };
        if !needle.is_empty() {
            field.setStringValue(&NSString::from_str(needle));
        }
        if let Some(window) = self.window() {
            window.makeFirstResponder(Some(&field));
        }
        unsafe { field.selectText(None) };
        if !needle.is_empty() {
            self.search();
        }
    }

    pub fn set_total(&self, total: Option<usize>) {
        self.ivars().total.set(total);
        self.update_count();
    }

    pub fn set_selected(&self, selected: Option<usize>) {
        self.ivars().selected.set(selected);
        self.update_count();
    }

    fn needle(&self) -> String {
        self.ivars()
            .field
            .borrow()
            .as_ref()
            .map(|field| field.stringValue().to_string())
            .unwrap_or_default()
    }

    fn search(&self) {
        self.act(&format!("search:{}", self.needle()));
    }

    fn navigate(&self, forward: bool) {
        if self.needle().is_empty() {
            return;
        }
        self.act(if forward {
            "navigate_search:next"
        } else {
            "navigate_search:previous"
        });
    }

    fn act(&self, action: &str) {
        let Some(surface) = self.surface() else {
            return;
        };
        surface.binding_action(action);
    }

    fn surface(&self) -> Option<Retained<SurfaceView>> {
        unsafe { self.superview() }?.downcast::<SurfaceView>().ok()
    }

    fn update_count(&self) {
        let Some(count) = self.ivars().count.borrow().clone() else {
            return;
        };
        let text = match (self.ivars().selected.get(), self.ivars().total.get()) {
            (_, _) if self.needle().is_empty() => String::new(),
            (Some(selected), Some(total)) => format!("{}/{total}", selected + 1),
            (Some(selected), None) => format!("{}/?", selected + 1),
            (None, Some(0)) => "None".to_owned(),
            (None, Some(total)) => total.to_string(),
            (None, None) => String::new(),
        };
        count.setStringValue(&NSString::from_str(&text));
    }
}
