use crate::habits;
use objc2::rc::Retained;
use objc2::{AnyThread, DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSAutoresizingMaskOptions, NSBezierPath, NSColor, NSColorSpace, NSGradient, NSView,
    NSViewLayerContentsRedrawPolicy, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSVisualEffectView,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSPoint, NSRect};
use std::cell::Cell;

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
    tab: Cell<bool>,
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
                _ if self.ivars().tab.get() => habits::GLASS_TAB,
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
            let edge = if self.ivars().tab.get() {
                habits::GLASS_TAB_EDGE
            } else if expanded {
                habits::GLASS_PANEL_EDGE
            } else {
                habits::GLASS_EDGE
            };
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

    pub(crate) fn set_tab(&self) {
        self.ivars().ivars().tab.set(true);
        self.ivars().setNeedsDisplay(true);
    }
}

pub(crate) fn glass(mtm: MainThreadMarker, frame: NSRect, radius: f64) -> Retained<GlassView> {
    let tint = GlassTint::alloc(mtm).set_ivars(GlassTintIvars {
        radius,
        expanded: Cell::new(false),
        quota: Cell::new(false),
        tab: Cell::new(false),
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
