use crate::habits;
use objc2::rc::Retained;
use objc2::{AnyThread, DefinedClass, MainThreadOnly, Message, define_class, msg_send};
use objc2_app_kit::NSAppearanceCustomization;
use objc2_app_kit::{
    NSAccessibility, NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSApplication,
    NSAutoresizingMaskOptions, NSBezierPath, NSButton, NSColor, NSColorSpace, NSEvent, NSEventType,
    NSFont, NSGradient, NSImage, NSImageView, NSLineBreakMode, NSTextField, NSTrackingArea,
    NSTrackingAreaOptions, NSView, NSViewLayerContentsRedrawPolicy, NSVisualEffectBlendingMode,
    NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSMutableAttributedString, NSPoint, NSRange, NSRect, NSSize,
    NSString,
};
use std::cell::{Cell, RefCell};

const PILL_RADIUS: f64 = 16.0;
const SESSION_DOT: f64 = 6.0;

pub(crate) struct ClickIvars {
    click: Box<dyn Fn()>,
    hover_button: RefCell<Option<Retained<ActionButton>>>,
    label: RefCell<Option<Retained<NSTextField>>>,
    selected: Cell<bool>,
    opened: Cell<Option<bool>>,
    dim_when_idle: Cell<bool>,
    text_color: Cell<(u32, u32)>,
    warn: RefCell<Vec<(NSRange, Retained<NSColor>)>>,
    hovered: Cell<bool>,
    hover_highlight: Cell<bool>,
    shortcut: Cell<Option<usize>>,
    tracking: RefCell<Option<Retained<NSTrackingArea>>>,
    focus_visible: Cell<bool>,
    corner_radius: Cell<f64>,
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
                color(habits::CHROME_SELECTION).setFill();
                NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
                    self.bounds(),
                    self.ivars().corner_radius.get(),
                    self.ivars().corner_radius.get(),
                )
                .fill();
            }
            if self.ivars().focus_visible.get() {
                let bounds = self.bounds();
                let frame = NSRect::new(NSPoint::new(2.0, 2.0), NSSize::new((bounds.size.width - 4.0).max(0.0), (bounds.size.height - 4.0).max(0.0)));
                NSColor::keyboardFocusIndicatorColor().setStroke();
                let radius = self.ivars().corner_radius.get() - 2.0;
                let ring = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(frame, radius, radius);
                ring.setLineWidth(2.0);
                ring.stroke();
            }
            let Some(opened) = self.ivars().opened.get() else {
                return;
            };
            let bounds = self.bounds();
            let y = ((bounds.size.height - SESSION_DOT) / 2.0).max(0.0);
            if opened {
                color(habits::CHROME_SESSION).setFill();
            } else {
                color(habits::CHROME_SESSION_IDLE).setFill();
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
                field.setTextColor(Some(&color(habits::CHROME_SOFT)));
                let frame = NSRect::new(NSPoint::new(bounds.size.width - 27.0, (bounds.size.height - 18.0) / 2.0), NSSize::new(18.0, 18.0));
                if self.ivars().shortcut.get().is_some() {
                    color(habits::CHROME_HINT).setFill();
                    NSBezierPath::bezierPathWithOvalInRect(frame).fill();
                }
                field.setFrame(frame);
                field.setAlignment(objc2_app_kit::NSTextAlignment::Center);
                let mut text_frame = frame;
                if self.ivars().shortcut.get().is_some() {
                    field.sizeToFit();
                    text_frame.size.height = field.frame().size.height;
                    text_frame.origin.y += (frame.size.height - text_frame.size.height) / 2.0;
                }
                if let Some(cell) = field.cell() { cell.drawWithFrame_inView(text_frame, self); }
            }
        }

        #[unsafe(method(updateTrackingAreas))]
        fn update_tracking_areas(&self) {
            if self.ivars().tracking.borrow().is_none() {
                let area = unsafe { NSTrackingArea::initWithRect_options_owner_userInfo(NSTrackingArea::alloc(), self.bounds(), NSTrackingAreaOptions::MouseEnteredAndExited | NSTrackingAreaOptions::ActiveInKeyWindow | NSTrackingAreaOptions::InVisibleRect, Some(self), None) };
                self.addTrackingArea(&area);
                *self.ivars().tracking.borrow_mut() = Some(area);
            }
            if self.ivars().hover_button.borrow().is_some() && let Some(window) = self.window() {
                    let point = self.convertPoint_fromView(window.mouseLocationOutsideOfEventStream(), None);
                    let bounds = self.bounds();
                    self.set_hovered(window.isKeyWindow() && point.x >= 0.0 && point.y >= 0.0 && point.x < bounds.size.width && point.y < bounds.size.height);
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
            self.update_hover_button();
            self.setNeedsDisplay(true);
            accepted
        }

        #[unsafe(method(resignFirstResponder))]
        fn resign_first_responder(&self) -> bool {
            self.ivars().focus_visible.set(false);
            self.update_hover_button();
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
            hover_button: RefCell::new(None),
            label: RefCell::new(None),
            selected: Cell::new(false),
            opened: Cell::new(None),
            dim_when_idle: Cell::new(false),
            text_color: Cell::new(habits::CHROME_TEXT),
            warn: RefCell::new(Vec::new()),
            hovered: Cell::new(false),
            hover_highlight: Cell::new(true),
            shortcut: Cell::new(None),
            tracking: RefCell::new(None),
            focus_visible: Cell::new(false),
            corner_radius: Cell::new(PILL_RADIUS),
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
        label.setTextColor(Some(&color(habits::CHROME_TEXT)));
        label.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
        label.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
        this.addSubview(&label);
        *this.ivars().label.borrow_mut() = Some(label);
        this.setAccessibilityElement(true);
        this.setAccessibilityRole(Some(&NSString::from_str("AXButton")));
        this.setAccessibilityLabel(Some(&NSString::from_str(text)));
        this
    }

    pub(crate) fn set_corner_radius(&self, radius: f64) {
        self.ivars().corner_radius.set(radius);
        self.setNeedsDisplay(true);
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
        self.update_hover_button();
        self.apply_label_color();
        self.setNeedsDisplay(true);
    }

    pub(crate) fn reveal_on_hover(&self, button: &ActionButton) {
        *self.ivars().hover_button.borrow_mut() = Some(button.retain());
        self.update_hover_button();
    }

    fn update_hover_button(&self) {
        if let Some(button) = self.ivars().hover_button.borrow().as_ref() {
            button.set_revealed(self.ivars().hovered.get() || self.ivars().focus_visible.get());
        }
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

    pub(crate) fn set_warn(&self, ranges: Vec<(NSRange, Retained<NSColor>)>) {
        *self.ivars().warn.borrow_mut() = ranges;
        self.apply_label_color();
    }

    pub(crate) fn set_text_color(&self, value: (u32, u32)) {
        self.ivars().text_color.set(value);
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
        let color = if selected {
            color(habits::CHROME_TEXT)
        } else if dim {
            color(habits::CHROME_MUTED)
        } else {
            color(self.ivars().text_color.get())
        };
        label.setTextColor(Some(&color));
        let ranges = self.ivars().warn.borrow();
        if !ranges.is_empty() {
            let text = NSMutableAttributedString::initWithAttributedString(
                NSMutableAttributedString::alloc(),
                &label.attributedStringValue(),
            );
            for (range, warning) in ranges.iter() {
                unsafe {
                    text.addAttribute_value_range(
                        objc2_app_kit::NSForegroundColorAttributeName,
                        warning,
                        *range,
                    )
                };
            }
            label.setAttributedStringValue(&text);
        }
    }
}

fn is_dark(appearance: &NSAppearance) -> bool {
    let names = unsafe { NSArray::from_slice(&[NSAppearanceNameDarkAqua, NSAppearanceNameAqua]) };
    appearance
        .bestMatchFromAppearancesWithNames(&names)
        .is_some_and(|name| &*name == unsafe { NSAppearanceNameDarkAqua })
}

fn rgba(value: u32) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(
        ((value >> 24) & 255) as f64 / 255.0,
        ((value >> 16) & 255) as f64 / 255.0,
        ((value >> 8) & 255) as f64 / 255.0,
        (value & 255) as f64 / 255.0,
    )
}

pub(crate) fn color((light, dark): (u32, u32)) -> Retained<NSColor> {
    let light = rgba(light);
    let dark = rgba(dark);
    let provider = block2::RcBlock::new(move |appearance: std::ptr::NonNull<NSAppearance>| {
        std::ptr::NonNull::from(if is_dark(unsafe { appearance.as_ref() }) {
            &*dark
        } else {
            &*light
        })
    });
    unsafe { NSColor::colorWithName_dynamicProvider(None, &provider) }
}

pub(crate) struct GlassTintIvars {
    radius: f64,
    expanded: Cell<bool>,
    quota: Cell<bool>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeGlassTint"]
    #[ivars = GlassTintIvars]
    pub(crate) struct GlassTint;

    impl GlassTint {
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> *mut NSView { std::ptr::null_mut() }

        #[unsafe(method(viewDidChangeEffectiveAppearance))]
        fn appearance_changed(&self) {
            let _: () = unsafe { msg_send![super(self), viewDidChangeEffectiveAppearance] };
            self.setNeedsDisplay(true);
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, dirty: NSRect) {
            let _: () = unsafe { msg_send![super(self), drawRect: dirty] };
            let expanded = self.ivars().expanded.get();
            let palette = match (self.ivars().quota.get(), expanded) {
                (false, false) => habits::GLASS_CONTROL,
                (false, true) => habits::GLASS_PANEL,
                (true, false) => habits::GLASS_QUOTA,
                (true, true) => habits::GLASS_QUOTA_PANEL,
            };
            let dark = is_dark(&self.effectiveAppearance());
            let colors = palette.map(|pair| rgba(if dark { pair.1 } else { pair.0 }));
            let path = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
                self.bounds(), self.ivars().radius, self.ivars().radius,
            );
            colors[0].setFill();
            path.fill();
            let stops = NSArray::from_slice(&[&*colors[1], &*colors[2], &*colors[3]]);
            let locations = [0.0, if expanded && !self.ivars().quota.get() { 0.44 } else { 0.48 }, 1.0];
            if let Some(gradient) = unsafe { NSGradient::initWithColors_atLocations_colorSpace(
                NSGradient::alloc(), &stops, locations.as_ptr(), &NSColorSpace::sRGBColorSpace(),
            ) } {
                gradient.drawInBezierPath_angle(&path, -55.0);
            }
            let edge = if expanded { habits::GLASS_PANEL_EDGE } else { habits::GLASS_EDGE };
            rgba(if dark { edge.1 } else { edge.0 }).setStroke();
            path.setLineWidth(1.5);
            path.stroke();
        }
    }
);

define_class!(
    #[unsafe(super(NSVisualEffectView))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeGlassView"]
    #[ivars = Retained<GlassTint>]
    pub(crate) struct GlassView;
);

impl GlassView {
    pub(crate) fn set_expanded(&self, expanded: bool) {
        let tint = self.ivars();
        if tint.ivars().expanded.replace(expanded) != expanded {
            tint.setNeedsDisplay(true);
        }
    }

    pub(crate) fn set_quota(&self) {
        self.ivars().ivars().quota.set(true);
        self.ivars().setNeedsDisplay(true);
    }
}

pub(crate) fn glass(mtm: MainThreadMarker, frame: NSRect, radius: f64) -> Retained<GlassView> {
    let tint = GlassTint::alloc(mtm).set_ivars(GlassTintIvars {
        radius,
        expanded: Cell::new(false),
        quota: Cell::new(false),
    });
    let tint: Retained<GlassTint> = unsafe {
        msg_send![super(tint), initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), frame.size)]
    };
    tint.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    tint.setWantsLayer(true);
    tint.setLayerContentsRedrawPolicy(NSViewLayerContentsRedrawPolicy::DuringViewResize);
    let view = GlassView::alloc(mtm).set_ivars(tint.clone());
    let view: Retained<GlassView> = unsafe { msg_send![super(view), initWithFrame: frame] };
    view.setBlendingMode(NSVisualEffectBlendingMode::WithinWindow);
    view.setMaterial(NSVisualEffectMaterial::Popover);
    view.setState(NSVisualEffectState::FollowsWindowActiveState);
    view.setWantsLayer(true);
    if let Some(layer) = view.layer() {
        let _: () = unsafe { msg_send![&*layer, setCornerRadius: radius] };
        layer.setMasksToBounds(true);
    }
    view.addSubview(&tint);
    view
}

pub(crate) fn symbol_image(name: &str, size: f64) -> Option<Retained<NSImage>> {
    let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
        &NSString::from_str(name),
        None,
    )?;
    image.imageWithSymbolConfiguration(
        &objc2_app_kit::NSImageSymbolConfiguration::configurationWithPointSize_weight(
            size,
            unsafe { objc2_app_kit::NSFontWeightRegular },
        ),
    )
}

pub(crate) fn symbol(mtm: MainThreadMarker, parent: &NSView, name: &str, frame: NSRect) {
    if let Some(image) = symbol_image(name, frame.size.height.min(14.0)) {
        let view = NSImageView::initWithFrame(NSImageView::alloc(mtm), frame);
        view.setImage(Some(&image));
        view.setContentTintColor(Some(&color(habits::CHROME_MUTED)));
        parent.addSubview(&view);
    }
}

pub(crate) fn icon_button(
    mtm: MainThreadMarker,
    symbol: &str,
    target: &objc2::runtime::AnyObject,
    action: objc2::runtime::Sel,
    frame: NSRect,
) -> Option<Retained<NSButton>> {
    let image = symbol_image(symbol, habits::CHROME_ICON_SIZE)?;
    let button = ActionButton::init(mtm, frame, None);
    button.setImage(Some(&image));
    button.setImagePosition(objc2_app_kit::NSCellImagePosition::ImageOnly);
    button.setTitle(&NSString::from_str(""));
    unsafe {
        button.setTarget(Some(target));
        button.setAction(Some(action));
    }
    Some(button.into_super())
}

pub(crate) struct ActionIvars {
    click: Option<Box<dyn Fn()>>,
    hovered: Cell<bool>,
    revealed: Cell<bool>,
    focused: Cell<bool>,
    tracking: RefCell<Option<Retained<NSTrackingArea>>>,
}

define_class!(
    #[unsafe(super(NSButton))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeActionButton"]
    #[ivars = ActionIvars]
    pub(crate) struct ActionButton;

    impl ActionButton {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, dirty: NSRect) {
            if self.isHighlighted() || self.ivars().hovered.get() || self.state() != 0 {
                color(if self.isHighlighted() { habits::CHROME_BUTTON_PRESSED } else if self.ivars().hovered.get() { habits::CHROME_BUTTON_HOVER } else { habits::CHROME_SELECTION }).setFill();
                let bounds = self.bounds();
                let diameter = bounds.size.width.min(bounds.size.height).min(28.0);
                NSBezierPath::bezierPathWithOvalInRect(NSRect::new(
                    NSPoint::new((bounds.size.width - diameter) / 2.0, (bounds.size.height - diameter) / 2.0),
                    NSSize::new(diameter, diameter),
                )).fill();
            }
            let _: () = unsafe { msg_send![super(self), drawRect: dirty] };
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
        fn mouse_entered(&self, _event: &NSEvent) {
            self.ivars().hovered.set(true);
            NSView::setNeedsDisplay(self, true);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.ivars().hovered.set(false);
            NSView::setNeedsDisplay(self, true);
        }

        #[unsafe(method(becomeFirstResponder))]
        fn become_first_responder(&self) -> bool {
            let accepted: bool = unsafe { msg_send![super(self), becomeFirstResponder] };
            self.ivars().focused.set(accepted);
            self.update_visibility();
            accepted
        }

        #[unsafe(method(resignFirstResponder))]
        fn resign_first_responder(&self) -> bool {
            let accepted: bool = unsafe { msg_send![super(self), resignFirstResponder] };
            if accepted { self.ivars().focused.set(false); }
            self.update_visibility();
            accepted
        }

        #[unsafe(method(hitTest:))]
        fn hit_test(&self, point: NSPoint) -> *mut NSView {
            if !self.ivars().revealed.get() && !self.ivars().focused.get() { return std::ptr::null_mut(); }
            unsafe { msg_send![super(self), hitTest: point] }
        }

        #[unsafe(method(activate:))]
        fn activate(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            let this = self.retain();
            if let Some(click) = &this.ivars().click { click(); }
        }
    }
);

impl ActionButton {
    fn init(mtm: MainThreadMarker, frame: NSRect, click: Option<Box<dyn Fn()>>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ActionIvars {
            click,
            hovered: Cell::new(false),
            revealed: Cell::new(true),
            focused: Cell::new(false),
            tracking: RefCell::new(None),
        });
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };
        this.setBordered(false);
        this.setContentTintColor(Some(&color(habits::CHROME_SOFT)));
        this
    }

    fn set_revealed(&self, revealed: bool) {
        self.ivars().revealed.set(revealed);
        self.update_visibility();
    }

    fn update_visibility(&self) {
        self.setAlphaValue(
            if self.ivars().revealed.get() || self.ivars().focused.get() {
                1.0
            } else {
                0.0
            },
        );
    }

    pub(crate) fn new(
        mtm: MainThreadMarker,
        frame: NSRect,
        symbol: &str,
        tooltip: &str,
        click: impl Fn() + 'static,
    ) -> Retained<Self> {
        let this = Self::init(mtm, frame, Some(Box::new(click)));
        this.setImage(symbol_image(symbol, habits::CHROME_ICON_SIZE).as_deref());
        this.setImagePosition(objc2_app_kit::NSCellImagePosition::ImageOnly);
        this.setTitle(&NSString::from_str(""));
        this.setToolTip(Some(&NSString::from_str(tooltip)));
        this.setAccessibilityLabel(Some(&NSString::from_str(tooltip)));
        unsafe {
            this.setTarget(Some(&this));
            this.setAction(Some(objc2::sel!(activate:)));
        }
        this
    }
}
