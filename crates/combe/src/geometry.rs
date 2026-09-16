use objc2_foundation::{NSPoint, NSRect, NSSize};

pub(crate) fn rect(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
}
