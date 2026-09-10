use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::{AnyThread, DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAccessibility, NSAutoresizingMaskOptions, NSBezierPath, NSBitmapImageRep, NSColor,
    NSCompositingOperation, NSDeviceRGBColorSpace, NSGraphicsContext, NSImage, NSImageScaling,
    NSImageView, NSLineBreakMode, NSScrollView, NSTextAlignment, NSTextField, NSView, NSWorkspace,
};
use objc2_core_foundation::CFType;
use objc2_core_image::{CIContext, CIImage};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use objc2_io_surface::IOSurfaceRef;
use objc2_quartz_core::{CAMediaTiming, CAMediaTimingFunction, CATransition};

use crate::chrome_view::ClickView;
use crate::split;
use crate::tabs::Tab;

const INSET: f64 = 24.0;
const GAP: f64 = 24.0;

struct Card {
    button: Retained<ClickView>,
    image: Retained<NSImageView>,
    title: Retained<NSTextField>,
}

pub struct OverviewIvars {
    scroll: RefCell<Option<Retained<NSScrollView>>>,
    document: Retained<NSView>,
    cards: Vec<Card>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeTabOverview"]
    #[ivars = OverviewIvars]
    pub struct Overview;

    impl Overview {
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool { true }

        #[unsafe(method(setFrameSize:))]
        fn set_frame_size(&self, size: NSSize) {
            let _: () = unsafe { msg_send![super(self), setFrameSize: size] };
            self.layout_cards();
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let Some(color) = self.window()
                .and_then(|window| window.contentView())
                .and_then(|view| view.layer())
                .and_then(|layer| {
                    let color: Option<&_> = unsafe { msg_send![&*layer, backgroundColor] };
                    color.and_then(NSColor::colorWithCGColor)
                })
            else { return };
            color.setFill();
            NSBezierPath::fillRect(self.bounds());
        }
    }
);

impl Overview {
    pub fn transition(parent: &NSView) {
        let Some(layer) = parent.layer() else { return };
        let key = NSString::from_str("tabOverview");
        layer.removeAnimationForKey(&key);
        if NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion() {
            return;
        }
        let transition = CATransition::animation();
        transition.setType(&NSString::from_str("fade"));
        transition.setDuration(0.18);
        transition.setTimingFunction(Some(&CAMediaTimingFunction::functionWithControlPoints(
            0.42, 0.0, 0.58, 1.0,
        )));
        layer.addAnimation_forKey(&transition, Some(&key));
    }

    pub fn new(
        mtm: MainThreadMarker,
        frame: NSRect,
        tabs: &[&Tab],
        active: u64,
        activate: fn(u64),
    ) -> Retained<Self> {
        let document = NSView::initWithFrame(NSView::alloc(mtm), frame);
        let context = unsafe { CIContext::new() };
        let cards = tabs
            .iter()
            .map(|tab| {
                let id = tab.id;
                let button =
                    ClickView::new(mtm, NSRect::default(), "", 0.0, 0.0, move || activate(id));
                button.set_selected(id == active);
                button.setAccessibilityLabel(Some(&NSString::from_str(&tab.label)));
                button.setAccessibilityHelp(Some(&NSString::from_str("Open tab")));
                let image = NSImageView::initWithFrame(NSImageView::alloc(mtm), NSRect::default());
                image.setImage(snapshot(tab, &context).as_deref());
                image.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
                image.setAccessibilityElement(false);
                button.addSubview(&image);
                let title = NSTextField::labelWithString(&NSString::from_str(&tab.label), mtm);
                title.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
                title.setAlignment(NSTextAlignment::Center);
                title.setAccessibilityElement(false);
                button.addSubview(&title);
                document.addSubview(&button);
                Card {
                    button,
                    image,
                    title,
                }
            })
            .collect();
        let allocated = Self::alloc(mtm).set_ivars(OverviewIvars {
            scroll: RefCell::new(None),
            document,
            cards,
        });
        let view: Retained<Self> = unsafe { msg_send![super(allocated), initWithFrame: frame] };
        view.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        view.setAccessibilityElement(true);
        view.setAccessibilityRole(Some(&NSString::from_str("AXGroup")));
        view.setAccessibilityLabel(Some(&NSString::from_str("Tab Overview")));
        let scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), view.bounds());
        scroll.setDrawsBackground(false);
        scroll.setHasVerticalScroller(true);
        scroll.setAutomaticallyAdjustsContentInsets(false);
        scroll.setDocumentView(Some(&view.ivars().document));
        view.addSubview(&scroll);
        *view.ivars().scroll.borrow_mut() = Some(scroll);
        view.layout_cards();
        if let Some(scroll) = view.ivars().scroll.borrow().as_ref() {
            let top =
                (view.ivars().document.frame().size.height - scroll.contentSize().height).max(0.0);
            scroll.contentView().scrollToPoint(NSPoint::new(0.0, top));
            scroll.reflectScrolledClipView(&scroll.contentView());
        }
        view
    }

    fn layout_cards(&self) {
        let scroll = self.ivars().scroll.borrow();
        let Some(scroll) = scroll.as_ref() else {
            return;
        };
        scroll.setFrame(self.bounds());
        let size = scroll.contentSize();
        let (columns, width, height, content_height) = grid(size, self.ivars().cards.len());
        self.ivars()
            .document
            .setFrameSize(NSSize::new(size.width, content_height));
        for (index, card) in self.ivars().cards.iter().enumerate() {
            let x = INSET + (index % columns) as f64 * (width + GAP);
            let y = content_height - INSET - height - (index / columns) as f64 * (height + GAP);
            card.button
                .setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(width, height)));
            card.image.setFrame(NSRect::new(
                NSPoint::new(8.0, 8.0),
                NSSize::new((width - 16.0).max(0.0), (height - 44.0).max(0.0)),
            ));
            card.title.setFrame(NSRect::new(
                NSPoint::new(8.0, height - 28.0),
                NSSize::new((width - 16.0).max(0.0), 20.0),
            ));
        }
    }
}

fn grid(size: NSSize, count: usize) -> (usize, f64, f64, f64) {
    let available = (size.width - 2.0 * INSET).max(1.0);
    let columns = (((available + GAP) / (260.0 + GAP)).floor() as usize)
        .max(1)
        .min(count.max(1));
    let width = ((available - GAP * (columns - 1) as f64) / columns as f64).min(400.0);
    let height = (width - 16.0).max(0.0) / 1.6 + 44.0;
    let rows = count.div_ceil(columns);
    let content_height =
        (2.0 * INSET + rows as f64 * height + rows.saturating_sub(1) as f64 * GAP).max(size.height);
    (columns, width, height, content_height)
}

fn snapshot(tab: &Tab, context: &CIContext) -> Option<Retained<NSImage>> {
    let bounds = tab.root.bounds();
    let scale = (720.0 / bounds.size.width.max(1.0)).min(450.0 / bounds.size.height.max(1.0));
    let size = NSSize::new(
        (bounds.size.width * scale).max(1.0),
        (bounds.size.height * scale).max(1.0),
    );
    let image = NSImage::initWithSize(NSImage::alloc(), size);
    let bitmap = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(), std::ptr::null_mut(), size.width.ceil() as isize, size.height.ceil() as isize,
            8, 4, true, false, NSDeviceRGBColorSpace, 0, 0,
        )
    }?;
    let graphics = NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap)?;
    NSGraphicsContext::saveGraphicsState_class();
    NSGraphicsContext::setCurrentContext(Some(&graphics));
    NSColor::windowBackgroundColor().setFill();
    NSBezierPath::fillRect(NSRect::new(NSPoint::new(0.0, 0.0), size));
    let surfaces = tab
        .zoom
        .as_ref()
        .map(|zoom| vec![zoom.surface.clone()])
        .unwrap_or_else(|| split::surfaces(&tab.root));
    for surface in surfaces {
        let Some(contents) = surface
            .layer()
            .and_then(|layer| unsafe { layer.contents() })
        else {
            continue;
        };
        let contents = unsafe { &*(&*contents as *const _ as *const CFType) };
        let Some(io) = contents.downcast_ref::<IOSurfaceRef>() else {
            continue;
        };
        let source = unsafe { CIImage::imageWithIOSurface(io) };
        let Some(bitmap) = (unsafe { context.createCGImage_fromRect(&source, source.extent()) })
        else {
            continue;
        };
        let pane = NSImage::initWithCGImage_size(NSImage::alloc(), &bitmap, surface.bounds().size);
        let frame = surface.convertRect_toView(surface.bounds(), Some(&tab.root));
        let frame = NSRect::new(
            NSPoint::new(frame.origin.x * scale, frame.origin.y * scale),
            NSSize::new(frame.size.width * scale, frame.size.height * scale),
        );
        pane.drawInRect_fromRect_operation_fraction(
            frame,
            NSRect::default(),
            NSCompositingOperation::Copy,
            1.0,
        );
    }
    NSGraphicsContext::restoreGraphicsState_class();
    image.addRepresentation(&bitmap);
    Some(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards_stay_in_bounds_and_overflow_scrolls() {
        for width in [160.0, 320.0, 864.0, 1600.0] {
            for count in [1, 2, 3, 9, 30] {
                let size = NSSize::new(width, 680.0);
                let (columns, card_width, card_height, content_height) = grid(size, count);
                assert!(card_width > 0.0 && card_height > 0.0);
                assert!(
                    INSET * 2.0 + columns as f64 * card_width + (columns - 1) as f64 * GAP
                        <= width + 0.01
                );
                let rows = count.div_ceil(columns);
                assert!(
                    INSET * 2.0 + rows as f64 * card_height + rows.saturating_sub(1) as f64 * GAP
                        <= content_height + 0.01
                );
            }
        }
    }
}
