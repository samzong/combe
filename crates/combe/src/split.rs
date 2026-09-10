use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{ClassType, MainThreadOnly, Message, msg_send};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSSplitView, NSView, NSWindowOrderingMode};
use objc2_foundation::{MainThreadMarker, NSRect};

use crate::surface::SurfaceView;

const FILL: NSAutoresizingMaskOptions = NSAutoresizingMaskOptions(
    NSAutoresizingMaskOptions::ViewWidthSizable.0 | NSAutoresizingMaskOptions::ViewHeightSizable.0,
);

pub struct Zoom {
    pub surface: Retained<SurfaceView>,
    tree: Retained<NSView>,
    placeholder: Retained<NSView>,
}

impl Zoom {
    pub fn new(root: &NSView, surface: &SurfaceView) -> Option<Self> {
        let parent = unsafe { surface.superview() }?;
        if !is_pane(&parent) {
            return None;
        }
        let tree = root.subviews().firstObject()?;
        let placeholder = NSView::initWithFrame(NSView::alloc(surface.mtm()), surface.frame());
        placeholder.setAutoresizingMask(surface.autoresizingMask());
        let surface = surface.retain();
        parent.replaceSubview_with(&surface, &placeholder);
        tree.setHidden(true);
        root.addSubview(&surface);
        surface.setFrame(root.bounds());
        Some(Self {
            surface,
            tree,
            placeholder,
        })
    }

    pub fn restore(self) {
        if let Some(parent) = unsafe { self.placeholder.superview() } {
            self.surface.removeFromSuperview();
            self.surface.setFrame(self.placeholder.frame());
            parent.replaceSubview_with(&self.placeholder, &self.surface);
        }
        self.tree.setHidden(false);
    }
}

pub fn leaf(
    mtm: MainThreadMarker,
    frame: NSRect,
    cwd: &str,
    input: Option<&str>,
) -> Retained<SurfaceView> {
    let view = SurfaceView::new(mtm, frame, cwd, input);
    view.setAutoresizingMask(FILL);
    view
}

pub fn root(
    mtm: MainThreadMarker,
    frame: NSRect,
    cwd: &str,
    input: Option<&str>,
) -> Retained<NSView> {
    let container = NSView::initWithFrame(NSView::alloc(mtm), frame);
    container.setAutoresizingMask(FILL);
    let view = leaf(mtm, container.bounds(), cwd, input);
    container.addSubview(&view);
    container
}

pub fn divide(
    mtm: MainThreadMarker,
    focused: &SurfaceView,
    vertical: bool,
    cwd: &str,
) -> Option<Retained<SurfaceView>> {
    let parent = unsafe { focused.superview() }?;
    let frame = focused.frame();
    let mask = focused.autoresizingMask();

    let pane = NSSplitView::initWithFrame(NSSplitView::alloc(mtm), frame);
    pane.setVertical(vertical);
    pane.setAutoresizingMask(mask);
    pane.setDividerStyle(objc2_app_kit::NSSplitViewDividerStyle::Thin);
    parent.addSubview_positioned_relativeTo(&pane, NSWindowOrderingMode::Above, Some(focused));

    let kept = focused.retain();
    kept.removeFromSuperview();
    kept.setAutoresizingMask(FILL);
    pane.addSubview(&kept);

    let fresh = leaf(mtm, pane.bounds(), cwd, None);
    pane.addSubview(&fresh);
    pane.adjustSubviews();
    Some(fresh)
}

pub fn close(view: &SurfaceView) {
    let Some(parent) = (unsafe { view.superview() }) else {
        return;
    };
    view.close();
    view.removeFromSuperview();

    if !is_pane(&parent) {
        return;
    }
    let remaining = parent.subviews();
    if remaining.len() != 1 {
        return;
    }
    let survivor = remaining.firstObject().expect("one subview");
    let Some(grandparent) = (unsafe { parent.superview() }) else {
        return;
    };
    grandparent.addSubview_positioned_relativeTo(
        &survivor,
        NSWindowOrderingMode::Above,
        Some(&parent),
    );
    survivor.setFrame(parent.frame());
    survivor.setAutoresizingMask(parent.autoresizingMask());
    parent.removeFromSuperview();
    if is_pane(&grandparent) {
        let pane: &NSSplitView =
            unsafe { &*(&*grandparent as *const NSView as *const NSSplitView) };
        pane.adjustSubviews();
    }
}

pub fn surfaces(root: &NSView) -> Vec<Retained<SurfaceView>> {
    let mut found = Vec::new();
    collect(root, &mut found);
    found
}

fn collect(view: &NSView, found: &mut Vec<Retained<SurfaceView>>) {
    for child in view.subviews() {
        if is_surface(&child) {
            let surface: &SurfaceView =
                unsafe { &*(&*child as *const NSView as *const SurfaceView) };
            found.push(surface.retain());
        } else {
            collect(&child, found);
        }
    }
}

fn is_pane(view: &NSView) -> bool {
    let object: &AnyObject = view.as_ref();
    unsafe { msg_send![object, isKindOfClass: NSSplitView::class()] }
}

fn is_surface(view: &NSView) -> bool {
    let object: &AnyObject = view.as_ref();
    unsafe { msg_send![object, isKindOfClass: SurfaceView::class()] }
}
