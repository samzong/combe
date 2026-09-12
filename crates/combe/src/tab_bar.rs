use std::cell::RefCell;
use std::collections::HashSet;

use objc2::MainThreadOnly;
use objc2::rc::Retained;
use objc2_app_kit::{NSAccessibility, NSAutoresizingMaskOptions, NSScrollView, NSView};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};

use crate::chrome_view::{self, ActionButton, ClickView};
use crate::habits;

const HEIGHT: f64 = 36.0;
const WIDTH: f64 = 180.0;
const RADIUS: f64 = 18.0;
const TOP_BAR_HEIGHT: f64 = 60.0;
const GAP: f64 = 12.0;

pub(crate) struct TabBar {
    document: Retained<NSView>,
    scroll: Retained<NSScrollView>,
    select: fn(u64),
    close: fn(u64),
    create: fn(),
    labels: RefCell<Vec<(u64, Retained<ClickView>)>>,
}

impl TabBar {
    pub(crate) fn new(
        mtm: MainThreadMarker,
        frame: NSRect,
        select: fn(u64),
        close: fn(u64),
        create: fn(),
    ) -> Self {
        let document = NSView::initWithFrame(NSView::alloc(mtm), frame);
        document.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewMinYMargin,
        );
        let scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), frame);
        scroll.setAutomaticallyAdjustsContentInsets(false);
        scroll.setDrawsBackground(false);
        scroll.setHasHorizontalScroller(false);
        scroll.setDocumentView(Some(&document));
        Self {
            document,
            scroll,
            select,
            close,
            create,
            labels: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn view(&self) -> &NSView {
        &self.scroll
    }

    pub(crate) fn set_frame(&self, frame: NSRect) {
        self.scroll.setFrame(frame);
    }

    pub(crate) fn set_attention(&self, tabs: &HashSet<u64>) {
        for (id, view) in self.labels.borrow().iter() {
            view.set_attention(tabs.contains(id));
        }
    }

    pub(crate) fn set_label(&self, id: u64, label: &str) {
        if let Some((_, view)) = self.labels.borrow().iter().find(|(tab, _)| *tab == id) {
            view.set_text(label);
        }
    }

    pub(crate) fn update<'a>(
        &self,
        tabs: impl Iterator<Item = (u64, &'a str)>,
        active: Option<u64>,
    ) {
        let mtm = MainThreadMarker::new().expect("main thread");
        self.labels.borrow_mut().clear();
        for child in self.document.subviews() {
            child.removeFromSuperview();
        }

        let select = self.select;
        let close_tab = self.close;
        let y = (TOP_BAR_HEIGHT - HEIGHT) / 2.0;
        let mut x = 0.0;
        for (id, label) in tabs {
            let frame = NSRect::new(NSPoint::new(x, y), NSSize::new(WIDTH, HEIGHT));
            let view = ClickView::new(mtm, frame, label, 12.0, 48.0, move || (select)(id));
            view.set_corner_radius(RADIUS);
            if Some(id) != active {
                view.dim_when_idle();
            }
            if Some(id) == active {
                view.set_text_color(habits::CHROME_STRONG);
                let glass = chrome_view::glass(mtm, frame, RADIUS);
                self.document.addSubview(&glass);
            }
            view.setAccessibilitySelected(Some(id) == active);
            self.document.addSubview(&view);
            self.labels.borrow_mut().push((id, view.clone()));

            let close = ActionButton::new(
                mtm,
                NSRect::new(
                    NSPoint::new(x + WIDTH - 30.0, y + 8.0),
                    NSSize::new(20.0, 20.0),
                ),
                "xmark",
                "Close tab",
                move || (close_tab)(id),
            );
            self.document.addSubview(&close);
            view.reveal_on_hover(&close);
            x += WIDTH + GAP;
        }

        let plus = ActionButton::new(
            mtm,
            NSRect::new(NSPoint::new(x, y), NSSize::new(HEIGHT, HEIGHT)),
            "plus",
            "New tab (⌘T)",
            self.create,
        );
        self.document.addSubview(&plus);
        self.document
            .setFrameSize(NSSize::new(x + HEIGHT, TOP_BAR_HEIGHT));
    }
}
