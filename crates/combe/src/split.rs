use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{ClassType, MainThreadOnly, Message, msg_send};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSSplitView, NSView, NSWindowOrderingMode};
use objc2_foundation::{MainThreadMarker, NSRect};

use crate::pane_overlay::ZoomCorners;
use crate::surface::SurfaceView;

const FILL: NSAutoresizingMaskOptions = NSAutoresizingMaskOptions(
    NSAutoresizingMaskOptions::ViewWidthSizable.0 | NSAutoresizingMaskOptions::ViewHeightSizable.0,
);

#[derive(Clone, Copy)]
pub(crate) enum Target {
    Previous,
    Next,
    Up,
    Left,
    Down,
    Right,
}

pub(crate) struct Zoom {
    pub surface: Retained<SurfaceView>,
    tree: Retained<NSView>,
    placeholder: Retained<NSView>,
    corners: Retained<ZoomCorners>,
}

impl Zoom {
    pub(crate) fn new(root: &NSView, surface: &SurfaceView) -> Option<Self> {
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
        let corners = ZoomCorners::new(surface.mtm(), root.bounds());
        root.addSubview(&corners);
        Some(Self {
            surface,
            tree,
            placeholder,
            corners,
        })
    }

    pub(crate) fn restore(self) {
        self.corners.removeFromSuperview();
        if let Some(parent) = unsafe { self.placeholder.superview() } {
            self.surface.removeFromSuperview();
            self.surface.setFrame(self.placeholder.frame());
            parent.replaceSubview_with(&self.placeholder, &self.surface);
        }
        self.tree.setHidden(false);
    }

    pub(crate) fn pane_rect(&self, root: &NSView) -> NSRect {
        self.placeholder
            .convertRect_toView(self.placeholder.bounds(), Some(root))
    }
}

pub(crate) fn leaf(
    mtm: MainThreadMarker,
    frame: NSRect,
    cwd: &str,
    input: Option<&str>,
) -> Retained<SurfaceView> {
    let view = SurfaceView::new(mtm, frame, cwd, input);
    view.setAutoresizingMask(FILL);
    view
}

fn container(mtm: MainThreadMarker, frame: NSRect) -> Retained<NSView> {
    let view = NSView::initWithFrame(NSView::alloc(mtm), frame);
    view.setAutoresizingMask(FILL);
    view
}

pub(crate) fn root(
    mtm: MainThreadMarker,
    frame: NSRect,
    cwd: &str,
    input: Option<&str>,
) -> Retained<NSView> {
    let container = container(mtm, frame);
    let view = leaf(mtm, container.bounds(), cwd, input);
    container.addSubview(&view);
    container
}

pub(crate) fn adopt(mtm: MainThreadMarker, frame: NSRect, view: &SurfaceView) -> Retained<NSView> {
    let container = container(mtm, frame);
    detach(view);
    view.setFrame(container.bounds());
    view.setAutoresizingMask(FILL);
    container.addSubview(view);
    container
}

pub(crate) fn divide(
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

pub(crate) fn close(view: &SurfaceView) {
    if unsafe { view.superview() }.is_none() {
        return;
    }
    view.close();
    detach(view);
}

pub(crate) fn detach(view: &SurfaceView) {
    let Some(parent) = (unsafe { view.superview() }) else {
        return;
    };
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

pub(crate) fn surfaces(root: &NSView) -> Vec<Retained<SurfaceView>> {
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

pub(crate) fn target(
    root: &NSView,
    focused: &SurfaceView,
    direction: Target,
) -> Option<Retained<SurfaceView>> {
    let leaves = surfaces(root);
    let index = leaves
        .iter()
        .position(|leaf| std::ptr::eq(&**leaf, focused))?;
    let rects: Vec<NSRect> = leaves
        .iter()
        .map(|leaf| root.convertRect_fromView(leaf.bounds(), Some(leaf)))
        .collect();
    pick(&rects, index, direction).map(|found| leaves[found].clone())
}

fn pick(rects: &[NSRect], focused: usize, target: Target) -> Option<usize> {
    let count = rects.len();
    if focused >= count {
        return None;
    }
    match target {
        Target::Previous => Some((focused + count - 1) % count),
        Target::Next => Some((focused + 1) % count),
        _ => adjacent(rects, focused, target),
    }
}

fn adjacent(rects: &[NSRect], focused: usize, target: Target) -> Option<usize> {
    let edges = |rect: &NSRect| {
        (
            rect.origin.x,
            rect.origin.x + rect.size.width,
            rect.origin.y,
            rect.origin.y + rect.size.height,
        )
    };
    let overlap = |a0: f64, a1: f64, b0: f64, b1: f64| a1.min(b1) - a0.max(b0);
    let (fx0, fx1, fy0, fy1) = edges(&rects[focused]);
    rects
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != focused)
        .filter_map(|(index, rect)| {
            let (x0, x1, y0, y1) = edges(rect);
            let (distance, shared) = match target {
                Target::Left => (fx0 - x1, overlap(y0, y1, fy0, fy1)),
                Target::Right => (x0 - fx1, overlap(y0, y1, fy0, fy1)),
                Target::Up => (y0 - fy1, overlap(x0, x1, fx0, fx1)),
                Target::Down => (fy0 - y1, overlap(x0, x1, fx0, fx1)),
                Target::Previous | Target::Next => return None,
            };
            (distance >= 0.0 && shared > 0.0).then_some((distance, shared, index))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0).then(b.1.total_cmp(&a.1)))
        .map(|(_, _, index)| index)
}

#[cfg(test)]
mod tests {
    use super::{Target, pick};
    use crate::geometry::rect;
    use objc2_foundation::NSRect;

    fn grid() -> Vec<NSRect> {
        vec![
            rect(0.0, 0.0, 200.0, 200.0),
            rect(200.0, 0.0, 200.0, 200.0),
            rect(0.0, 200.0, 200.0, 200.0),
            rect(200.0, 200.0, 200.0, 200.0),
        ]
    }

    #[test]
    fn right_and_left_pick_the_row_neighbor() {
        let grid = grid();
        assert_eq!(pick(&grid, 0, Target::Right), Some(1));
        assert_eq!(pick(&grid, 1, Target::Left), Some(0));
        assert_eq!(pick(&grid, 2, Target::Right), Some(3));
        assert_eq!(pick(&grid, 3, Target::Left), Some(2));
    }

    #[test]
    fn up_and_down_pick_the_column_neighbor() {
        let grid = grid();
        assert_eq!(pick(&grid, 0, Target::Up), Some(2));
        assert_eq!(pick(&grid, 2, Target::Down), Some(0));
        assert_eq!(pick(&grid, 1, Target::Up), Some(3));
        assert_eq!(pick(&grid, 3, Target::Down), Some(1));
    }

    #[test]
    fn the_grid_edge_has_no_neighbor() {
        let grid = grid();
        assert_eq!(pick(&grid, 0, Target::Left), None);
        assert_eq!(pick(&grid, 0, Target::Down), None);
        assert_eq!(pick(&grid, 3, Target::Right), None);
        assert_eq!(pick(&grid, 3, Target::Up), None);
    }

    #[test]
    fn a_pane_off_the_projection_is_not_a_neighbor() {
        let rects = vec![
            rect(0.0, 0.0, 100.0, 100.0),
            rect(200.0, 200.0, 100.0, 100.0),
        ];
        assert_eq!(pick(&rects, 0, Target::Right), None);
        assert_eq!(pick(&rects, 0, Target::Up), None);
    }

    #[test]
    fn equal_distance_prefers_the_wider_shared_edge() {
        let focused = rect(0.0, 0.0, 100.0, 200.0);
        let narrow = rect(100.0, 0.0, 100.0, 50.0);
        let wide = rect(100.0, 50.0, 100.0, 150.0);
        assert_eq!(pick(&[focused, narrow, wide], 0, Target::Right), Some(2));
        assert_eq!(pick(&[focused, wide, narrow], 0, Target::Right), Some(1));
    }

    #[test]
    fn previous_and_next_wrap_around() {
        let rects = vec![
            rect(0.0, 0.0, 10.0, 10.0),
            rect(10.0, 0.0, 10.0, 10.0),
            rect(20.0, 0.0, 10.0, 10.0),
        ];
        assert_eq!(pick(&rects, 0, Target::Previous), Some(2));
        assert_eq!(pick(&rects, 2, Target::Next), Some(0));
        assert_eq!(pick(&rects, 1, Target::Previous), Some(0));
        assert_eq!(pick(&rects, 1, Target::Next), Some(2));
    }

    #[test]
    fn a_focus_outside_the_set_has_no_target() {
        let grid = grid();
        assert_eq!(pick(&grid, 4, Target::Right), None);
        assert_eq!(pick(&grid, 4, Target::Next), None);
        assert_eq!(pick(&[], 0, Target::Next), None);
    }
}
