use crate::{glass::color, habits};
use objc2::rc::Retained;
use objc2::{MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAccessibility, NSAutoresizingMaskOptions, NSBezierPath, NSView, NSWorkspace,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSNumber, NSPoint, NSRect, NSString};
use objc2_quartz_core::{
    CAKeyframeAnimation, CAMediaTiming, CAMediaTimingFunction, kCAAnimationDiscrete,
};

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombePaneGuide"]
    pub(crate) struct PaneGuide;

    impl PaneGuide {
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> *mut NSView {
            std::ptr::null_mut()
        }
    }
);

impl PaneGuide {
    pub(crate) fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let view: Retained<Self> = unsafe { msg_send![Self::alloc(mtm), initWithFrame: frame] };
        view.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        view.setAccessibilityElement(false);
        view.setWantsLayer(true);
        if let Some(layer) = view.layer() {
            layer.setBorderWidth(1.0);
            layer.setOpacity(0.0);
        }
        view
    }

    pub(crate) fn show(&self) {
        let Some(layer) = self.layer() else { return };
        let border = color(habits::CHROME_ATTENTION).CGColor();
        unsafe {
            let _: () = msg_send![&*layer, setBorderColor: &*border];
        }
        let reduce = NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion();
        let animation =
            CAKeyframeAnimation::animationWithKeyPath(Some(&NSString::from_str("opacity")));
        let values = NSArray::from_retained_slice(&[
            NSNumber::new_f64(if reduce { 0.65 } else { 0.0 }),
            NSNumber::new_f64(0.65),
            NSNumber::new_f64(0.0),
        ]);
        unsafe {
            let _: () = msg_send![&animation, setValues: &*values];
        }
        animation.setKeyTimes(Some(&NSArray::from_retained_slice(&[
            NSNumber::new_f64(0.0),
            NSNumber::new_f64(0.15),
            NSNumber::new_f64(1.0),
        ])));
        animation.setDuration(habits::PANE_GUIDE_DURATION);
        if reduce {
            animation.setCalculationMode(unsafe { kCAAnimationDiscrete });
        } else {
            let timing = CAMediaTimingFunction::functionWithControlPoints(0.42, 0.0, 0.58, 1.0);
            animation.setTimingFunctions(Some(&NSArray::from_slice(&[&*timing, &*timing])));
        }
        layer.addAnimation_forKey(&animation, Some(&NSString::from_str("pane-guide")));
    }
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeZoomCorners"]
    pub(crate) struct ZoomCorners;

    impl ZoomCorners {
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> *mut NSView {
            std::ptr::null_mut()
        }

        #[unsafe(method(viewDidChangeEffectiveAppearance))]
        fn appearance_changed(&self) {
            let _: () = unsafe { msg_send![super(self), viewDidChangeEffectiveAppearance] };
            self.setNeedsDisplay(true);
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let size = self.bounds().size;
            let inset = habits::ZOOM_CORNER_INSET;
            let length = habits::ZOOM_CORNER_LENGTH
                .min(size.width / 2.0 - inset)
                .min(size.height / 2.0 - inset)
                .max(0.0);
            color(habits::CHROME_MUTED).colorWithAlphaComponent(0.5).setStroke();
            let path = NSBezierPath::bezierPath();
            path.setLineWidth(1.0);
            for (x, dx) in [(inset + 0.5, 1.0), (size.width - inset - 0.5, -1.0)] {
                for (y, dy) in [(inset + 0.5, 1.0), (size.height - inset - 0.5, -1.0)] {
                    path.moveToPoint(NSPoint::new(x + dx * (length - 0.5), y));
                    path.lineToPoint(NSPoint::new(x, y));
                    path.lineToPoint(NSPoint::new(x, y + dy * (length - 0.5)));
                }
            }
            path.stroke();
        }
    }
);

impl ZoomCorners {
    pub(crate) fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let view: Retained<Self> = unsafe { msg_send![Self::alloc(mtm), initWithFrame: frame] };
        view.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        view.setAccessibilityElement(false);
        view.setWantsLayer(true);
        view
    }
}
