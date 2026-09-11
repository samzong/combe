use std::cell::{Cell, RefCell};
use std::ffi::{CString, c_void};
use std::ptr;

use ghostty_sys as sys;
use objc2::ClassType;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{AnyThread, DefinedClass, MainThreadOnly, Message, define_class, msg_send};
use objc2_app_kit::{
    NSEvent, NSEventModifierFlags, NSEventPhase, NSEventType, NSTextInputClient, NSTrackingArea,
    NSTrackingAreaOptions, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSAttributedString, NSPoint, NSRange, NSRect, NSSize, NSString,
};

use crate::find_bar::FindBar;
use crate::ghostty;

const NX_DEVICE_RSHIFT: usize = 0x00000004;
const NX_DEVICE_RCTRL: usize = 0x00002000;
const NX_DEVICE_RALT: usize = 0x00000040;
const NX_DEVICE_RCMD: usize = 0x00000010;

pub struct SurfaceIvars {
    surface: Cell<sys::ghostty_surface_t>,
    cwd: RefCell<String>,
    input: RefCell<Option<String>>,
    title: RefCell<Option<String>>,
    marked_text: RefCell<String>,
    key_text: RefCell<Option<Vec<String>>>,
    content_size: Cell<NSSize>,
    find: RefCell<Option<Retained<FindBar>>>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "CombeSurfaceView"]
    #[ivars = SurfaceIvars]
    pub struct SurfaceView;

    impl SurfaceView {
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            false
        }

        #[unsafe(method(viewDidMoveToWindow))]
        fn view_did_move_to_window(&self) {
            if self.window().is_some() {
                self.create_surface();
            }
        }

        #[unsafe(method(viewDidChangeBackingProperties))]
        fn view_did_change_backing_properties(&self) {
            let Some(surface) = self.handle() else { return };
            let frame = self.frame();
            if frame.size.width <= 0.0 || frame.size.height <= 0.0 {
                return;
            }
            let backing = self.convertRectToBacking(frame);
            unsafe {
                sys::ghostty_surface_set_content_scale(
                    surface,
                    backing.size.width / frame.size.width,
                    backing.size.height / frame.size.height,
                );
            }
            self.push_size();
        }

        #[unsafe(method(setFrameSize:))]
        fn set_frame_size(&self, size: NSSize) {
            unsafe { msg_send![super(self), setFrameSize: size] }
            self.ivars().content_size.set(size);
            self.push_size();
        }

        #[unsafe(method(updateTrackingAreas))]
        fn update_tracking_areas(&self) {
            for area in self.trackingAreas() {
                self.removeTrackingArea(&area);
            }
            let options = NSTrackingAreaOptions::MouseEnteredAndExited
                | NSTrackingAreaOptions::MouseMoved
                | NSTrackingAreaOptions::InVisibleRect
                | NSTrackingAreaOptions::ActiveAlways;
            let area = unsafe {
                NSTrackingArea::initWithRect_options_owner_userInfo(
                    NSTrackingArea::alloc(),
                    self.frame(),
                    options,
                    Some(self),
                    None,
                )
            };
            self.addTrackingArea(&area);
        }

        #[unsafe(method(becomeFirstResponder))]
        fn become_first_responder(&self) -> bool {
            if let Some(surface) = self.handle() {
                unsafe { sys::ghostty_surface_set_focus(surface, true) };
            }
            crate::window::refresh_labels();
            true
        }

        #[unsafe(method(resignFirstResponder))]
        fn resign_first_responder(&self) -> bool {
            if let Some(surface) = self.handle() {
                unsafe { sys::ghostty_surface_set_focus(surface, false) };
            }
            true
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            self.handle_key_down(event);
        }

        #[unsafe(method(keyUp:))]
        fn key_up(&self, event: &NSEvent) {
            self.send_key(sys::GHOSTTY_ACTION_RELEASE, event, None, false);
        }

        #[unsafe(method(flagsChanged:))]
        fn flags_changed(&self, event: &NSEvent) {
            self.handle_flags_changed(event);
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            self.send_button(sys::GHOSTTY_MOUSE_PRESS, sys::GHOSTTY_MOUSE_LEFT, event);
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            self.send_button(sys::GHOSTTY_MOUSE_RELEASE, sys::GHOSTTY_MOUSE_LEFT, event);
        }

        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &NSEvent) {
            self.send_button(sys::GHOSTTY_MOUSE_PRESS, sys::GHOSTTY_MOUSE_RIGHT, event);
        }

        #[unsafe(method(rightMouseUp:))]
        fn right_mouse_up(&self, event: &NSEvent) {
            self.send_button(sys::GHOSTTY_MOUSE_RELEASE, sys::GHOSTTY_MOUSE_RIGHT, event);
        }

        #[unsafe(method(otherMouseDown:))]
        fn other_mouse_down(&self, event: &NSEvent) {
            self.send_button(sys::GHOSTTY_MOUSE_PRESS, sys::GHOSTTY_MOUSE_MIDDLE, event);
        }

        #[unsafe(method(otherMouseUp:))]
        fn other_mouse_up(&self, event: &NSEvent) {
            self.send_button(sys::GHOSTTY_MOUSE_RELEASE, sys::GHOSTTY_MOUSE_MIDDLE, event);
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            self.send_pos(event);
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            self.send_pos(event);
        }

        #[unsafe(method(rightMouseDragged:))]
        fn right_mouse_dragged(&self, event: &NSEvent) {
            self.send_pos(event);
        }

        #[unsafe(method(otherMouseDragged:))]
        fn other_mouse_dragged(&self, event: &NSEvent) {
            self.send_pos(event);
        }

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            self.send_scroll(event);
        }
    }

    unsafe impl NSTextInputClient for SurfaceView {
        #[unsafe(method(hasMarkedText))]
        fn has_marked_text(&self) -> bool {
            !self.ivars().marked_text.borrow().is_empty()
        }

        #[unsafe(method(markedRange))]
        fn marked_range(&self) -> NSRange {
            let len = self.ivars().marked_text.borrow().encode_utf16().count();
            if len == 0 {
                NSRange::new(usize::MAX, 0)
            } else {
                NSRange::new(0, len)
            }
        }

        #[unsafe(method(selectedRange))]
        fn selected_range(&self) -> NSRange {
            NSRange::new(usize::MAX, 0)
        }

        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        fn set_marked_text(&self, string: &AnyObject, _selected: NSRange, _replacement: NSRange) {
            *self.ivars().marked_text.borrow_mut() = any_object_string(string);
            if self.ivars().key_text.borrow().is_none() {
                self.sync_preedit(true);
            }
        }

        #[unsafe(method(unmarkText))]
        fn unmark_text(&self) {
            if self.ivars().marked_text.borrow().is_empty() {
                return;
            }
            self.ivars().marked_text.borrow_mut().clear();
            self.sync_preedit(true);
        }

        #[unsafe(method_id(validAttributesForMarkedText))]
        fn valid_attributes_for_marked_text(&self) -> Retained<NSArray<NSString>> {
            NSArray::new()
        }

        #[unsafe(method_id(attributedSubstringForProposedRange:actualRange:))]
        fn attributed_substring(
            &self,
            _range: NSRange,
            _actual: *mut NSRange,
        ) -> Option<Retained<NSAttributedString>> {
            None
        }

        #[unsafe(method(characterIndexForPoint:))]
        fn character_index_for_point(&self, _point: NSPoint) -> usize {
            0
        }

        #[unsafe(method(firstRectForCharacterRange:actualRange:))]
        fn first_rect_for_character_range(
            &self,
            _range: NSRange,
            _actual: *mut NSRange,
        ) -> NSRect {
            let frame = self.frame();
            let Some(surface) = self.handle() else {
                return NSRect::new(frame.origin, NSSize::new(0.0, 0.0));
            };

            let mut x = 0.0f64;
            let mut y = 0.0f64;
            let mut width = 0.0f64;
            let mut height = 0.0f64;
            unsafe { sys::ghostty_surface_ime_point(surface, &mut x, &mut y, &mut width, &mut height) };

            let view_rect = NSRect::new(
                NSPoint::new(x, frame.size.height - y),
                NSSize::new(width, height),
            );
            let window_rect = self.convertRect_toView(view_rect, None);
            match self.window() {
                Some(window) => window.convertRectToScreen(window_rect),
                None => window_rect,
            }
        }

        #[unsafe(method(insertText:replacementRange:))]
        fn insert_text(&self, string: &AnyObject, _replacement: NSRange) {
            let chars = any_object_string(string);
            self.unmark_text_internal();

            if let Some(accumulator) = self.ivars().key_text.borrow_mut().as_mut() {
                accumulator.push(chars);
                return;
            }
            if !chars.is_empty() {
                self.send_committed_text(sys::GHOSTTY_ACTION_PRESS, &chars);
            }
        }

        #[unsafe(method(doCommandBySelector:))]
        fn do_command_by_selector(&self, _selector: Sel) {}
    }
);

impl SurfaceView {
    pub fn new(
        mtm: MainThreadMarker,
        frame: NSRect,
        cwd: &str,
        input: Option<&str>,
    ) -> Retained<Self> {
        let ivars = SurfaceIvars {
            surface: Cell::new(ptr::null_mut()),
            cwd: RefCell::new(cwd.to_owned()),
            input: RefCell::new(input.map(str::to_owned)),
            title: RefCell::new(None),
            marked_text: RefCell::new(String::new()),
            key_text: RefCell::new(None),
            content_size: Cell::new(frame.size),
            find: RefCell::new(None),
        };
        let this = Self::alloc(mtm).set_ivars(ivars);
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }

    pub fn close(&self) {
        let surface = self.ivars().surface.replace(ptr::null_mut());
        if !surface.is_null() {
            unsafe { sys::ghostty_surface_free(surface) };
        }
    }

    pub fn sync_appearance(&self) {
        if let Some(surface) = self.handle() {
            unsafe { sys::ghostty_surface_set_color_scheme(surface, ghostty::color_scheme()) };
        }
    }

    pub fn set_occluded(&self, occluded: bool) {
        if let Some(surface) = self.handle() {
            unsafe { sys::ghostty_surface_set_occlusion(surface, !occluded) };
        }
    }

    pub fn cwd(&self) -> String {
        self.ivars().cwd.borrow().clone()
    }

    pub fn set_cwd(&self, cwd: &str) {
        let cwd = cwd.trim();
        if cwd.is_empty() || cwd.contains('\0') {
            return;
        }
        *self.ivars().cwd.borrow_mut() = cwd.to_owned();
    }

    pub fn title(&self) -> Option<String> {
        self.ivars().title.borrow().clone()
    }

    pub fn set_title(&self, title: &str) {
        let title = title.trim();
        if title.is_empty() {
            return;
        }
        *self.ivars().title.borrow_mut() = Some(title.to_owned());
    }

    pub fn needs_confirm_quit(&self) -> bool {
        self.handle()
            .is_some_and(|surface| unsafe { sys::ghostty_surface_needs_confirm_quit(surface) })
    }

    pub fn start_search(&self, needle: &str) {
        let bar = self.ivars().find.borrow().clone();
        let bar = match bar {
            Some(bar) => bar,
            None => {
                let mtm = MainThreadMarker::from(self);
                let bar = FindBar::new(mtm, self.bounds().size);
                self.addSubview(&bar);
                *self.ivars().find.borrow_mut() = Some(bar.clone());
                bar
            }
        };
        bar.focus(needle);
    }

    pub fn end_search(&self) {
        let Some(bar) = self.ivars().find.borrow_mut().take() else {
            return;
        };
        bar.removeFromSuperview();
        if let Some(window) = self.window() {
            window.makeFirstResponder(Some(self));
        }
    }

    pub fn set_search_total(&self, total: Option<usize>) {
        if let Some(bar) = self.ivars().find.borrow().as_ref() {
            bar.set_total(total);
        }
    }

    pub fn set_search_selected(&self, selected: Option<usize>) {
        if let Some(bar) = self.ivars().find.borrow().as_ref() {
            bar.set_selected(selected);
        }
    }

    pub fn binding_action(&self, action: &str) {
        let Some(surface) = self.handle() else { return };
        let ok = unsafe {
            sys::ghostty_surface_binding_action(surface, action.as_ptr().cast(), action.len())
        };
        if !ok {
            eprintln!("combe: binding action failed: {action}");
        }
    }

    fn handle(&self) -> Option<sys::ghostty_surface_t> {
        let surface = self.ivars().surface.get();
        (!surface.is_null()).then_some(surface)
    }

    fn create_surface(&self) {
        if self.handle().is_some() {
            return;
        }
        let app = ghostty::app();
        assert!(!app.is_null(), "ghostty app not initialized");

        let scale = self
            .window()
            .map(|window| window.backingScaleFactor())
            .unwrap_or(2.0);
        let cwd =
            CString::new(self.ivars().cwd.borrow().as_str()).expect("cwd has no interior nul");
        let input = self
            .ivars()
            .input
            .borrow_mut()
            .take()
            .and_then(|input| CString::new(format!("{input}\n")).ok());

        let mut config = unsafe { sys::ghostty_surface_config_new() };
        config.platform_tag = sys::GHOSTTY_PLATFORM_MACOS;
        config.platform.macos.nsview = self as *const Self as *mut c_void;
        config.userdata = self as *const Self as *mut c_void;
        config.scale_factor = scale;
        config.working_directory = cwd.as_ptr();
        if let Some(input) = &input {
            config.initial_input = input.as_ptr();
        }

        let surface = unsafe { sys::ghostty_surface_new(app, &config) };
        assert!(!surface.is_null(), "ghostty_surface_new failed");
        self.ivars().surface.set(surface);
        self.sync_appearance();

        self.updateTrackingAreas();
        self.push_size();
    }

    fn push_size(&self) {
        let Some(surface) = self.handle() else { return };
        let size = self.ivars().content_size.get();
        if size.width <= 0.0 || size.height <= 0.0 {
            return;
        }
        let backing = self.convertSizeToBacking(size);
        unsafe {
            sys::ghostty_surface_set_size(surface, backing.width as u32, backing.height as u32);
        }
    }

    fn unmark_text_internal(&self) {
        if self.ivars().marked_text.borrow().is_empty() {
            return;
        }
        self.ivars().marked_text.borrow_mut().clear();
        self.sync_preedit(true);
    }

    fn sync_preedit(&self, clear_if_needed: bool) {
        let Some(surface) = self.handle() else { return };
        let marked = self.ivars().marked_text.borrow();
        if marked.is_empty() {
            if clear_if_needed {
                unsafe { sys::ghostty_surface_preedit(surface, ptr::null(), 0) };
            }
            return;
        }
        let bytes = marked.as_bytes();
        unsafe {
            sys::ghostty_surface_preedit(surface, bytes.as_ptr() as *const _, bytes.len());
        }
    }

    fn handle_key_down(&self, event: &NSEvent) {
        if self.handle().is_none() {
            return;
        }

        let action = if event.isARepeat() {
            sys::GHOSTTY_ACTION_REPEAT
        } else {
            sys::GHOSTTY_ACTION_PRESS
        };

        let marked_before = !self.ivars().marked_text.borrow().is_empty();
        *self.ivars().key_text.borrow_mut() = Some(Vec::new());
        let events = NSArray::from_slice(&[event]);
        self.interpretKeyEvents(&events);
        let accumulated = self
            .ivars()
            .key_text
            .borrow_mut()
            .take()
            .unwrap_or_default();

        self.sync_preedit(marked_before);

        let composing = !self.ivars().marked_text.borrow().is_empty() || marked_before;

        if !accumulated.is_empty() {
            for text in &accumulated {
                if suppress_composing_control(text, composing) {
                    continue;
                }
                if marked_before {
                    self.send_committed_text(action, text);
                } else {
                    self.send_key(action, event, Some(text.as_str()), false);
                }
            }
            return;
        }

        let characters = event_characters(event);
        if suppress_composing_control(characters.as_deref().unwrap_or(""), composing) {
            return;
        }
        self.send_key(action, event, characters.as_deref(), composing);
    }

    fn handle_flags_changed(&self, event: &NSEvent) {
        let keycode = event.keyCode();
        let bit = match keycode {
            0x39 => sys::GHOSTTY_MODS_CAPS,
            0x38 | 0x3C => sys::GHOSTTY_MODS_SHIFT,
            0x3B | 0x3E => sys::GHOSTTY_MODS_CTRL,
            0x3A | 0x3D => sys::GHOSTTY_MODS_ALT,
            0x37 | 0x36 => sys::GHOSTTY_MODS_SUPER,
            _ => return,
        };
        if !self.ivars().marked_text.borrow().is_empty() {
            return;
        }

        let flags = event.modifierFlags();
        let mods = ghostty_mods(flags);
        let mut action = sys::GHOSTTY_ACTION_RELEASE;
        if mods & bit != 0 {
            let raw = flags.0;
            let side_pressed = match keycode {
                0x3C => raw & NX_DEVICE_RSHIFT != 0,
                0x3E => raw & NX_DEVICE_RCTRL != 0,
                0x3D => raw & NX_DEVICE_RALT != 0,
                0x36 => raw & NX_DEVICE_RCMD != 0,
                _ => true,
            };
            if side_pressed {
                action = sys::GHOSTTY_ACTION_PRESS;
            }
        }
        self.send_key(action, event, None, false);
    }

    fn send_key(
        &self,
        action: sys::ghostty_input_action_e,
        event: &NSEvent,
        text: Option<&str>,
        composing: bool,
    ) -> bool {
        let Some(surface) = self.handle() else {
            return false;
        };

        let flags = event.modifierFlags();
        let mut key = sys::ghostty_input_key_s {
            action,
            mods: ghostty_mods(flags),
            consumed_mods: ghostty_mods(
                flags & !(NSEventModifierFlags::Control | NSEventModifierFlags::Command),
            ),
            keycode: event.keyCode() as u32,
            text: ptr::null(),
            unshifted_codepoint: unshifted_codepoint(event),
            composing,
        };

        match text.and_then(key_event_text) {
            Some(text) => {
                let text = CString::new(text).expect("key text has no interior nul");
                key.text = text.as_ptr();
                unsafe { sys::ghostty_surface_key(surface, key) }
            }
            None => unsafe { sys::ghostty_surface_key(surface, key) },
        }
    }

    fn send_committed_text(&self, action: sys::ghostty_input_action_e, text: &str) {
        let Some(surface) = self.handle() else { return };
        let Ok(text) = CString::new(text) else { return };
        let key = sys::ghostty_input_key_s {
            action,
            mods: sys::GHOSTTY_MODS_NONE,
            consumed_mods: sys::GHOSTTY_MODS_NONE,
            keycode: 0,
            text: text.as_ptr(),
            unshifted_codepoint: 0,
            composing: false,
        };
        unsafe { sys::ghostty_surface_key(surface, key) };
    }

    fn send_button(
        &self,
        state: sys::ghostty_input_mouse_state_e,
        button: sys::ghostty_input_mouse_button_e,
        event: &NSEvent,
    ) {
        if state == sys::GHOSTTY_MOUSE_PRESS
            && let Some(window) = self.window()
        {
            let focused = window.firstResponder().is_some_and(|responder| {
                std::ptr::eq(
                    &*responder as *const _ as *const (),
                    self as *const Self as *const (),
                )
            });
            if !focused {
                window.makeFirstResponder(Some(self));
            }
        }
        let Some(surface) = self.handle() else { return };
        let mods = ghostty_mods(event.modifierFlags());
        unsafe { sys::ghostty_surface_mouse_button(surface, state, button, mods) };
    }

    fn send_pos(&self, event: &NSEvent) {
        let Some(surface) = self.handle() else { return };
        let point = self.convertPoint_fromView(event.locationInWindow(), None);
        let mods = ghostty_mods(event.modifierFlags());
        unsafe {
            sys::ghostty_surface_mouse_pos(
                surface,
                point.x,
                self.frame().size.height - point.y,
                mods,
            );
        }
    }

    fn send_scroll(&self, event: &NSEvent) {
        let Some(surface) = self.handle() else { return };
        let precision = event.hasPreciseScrollingDeltas();
        let multiplier = if precision { 2.0 } else { 1.0 };
        let x = event.scrollingDeltaX() * multiplier;
        let y = event.scrollingDeltaY() * multiplier;

        let mut mods: sys::ghostty_input_scroll_mods_t = 0;
        if precision {
            mods |= 1;
        }
        mods |= (momentum(event.momentumPhase()) as i32) << 1;
        unsafe { sys::ghostty_surface_mouse_scroll(surface, x, y, mods) };
    }
}

pub fn view_from_userdata(userdata: *mut c_void) -> Option<Retained<SurfaceView>> {
    if userdata.is_null() {
        return None;
    }
    let view = unsafe { &*(userdata as *const SurfaceView) };
    Some(view.retain())
}

pub fn handle_from_userdata(userdata: *mut c_void) -> Option<sys::ghostty_surface_t> {
    if userdata.is_null() {
        return None;
    }
    let view = unsafe { &*(userdata as *const SurfaceView) };
    view.handle()
}

fn any_object_string(value: &AnyObject) -> String {
    unsafe {
        let is_attributed: bool = msg_send![value, isKindOfClass: NSAttributedString::class()];
        if is_attributed {
            let attributed: &NSAttributedString = &*(value as *const AnyObject as *const _);
            return attributed.string().to_string();
        }
        let is_string: bool = msg_send![value, isKindOfClass: NSString::class()];
        if is_string {
            let string: &NSString = &*(value as *const AnyObject as *const _);
            return string.to_string();
        }
        String::new()
    }
}

fn ghostty_mods(flags: NSEventModifierFlags) -> sys::ghostty_input_mods_e {
    let mut mods = sys::GHOSTTY_MODS_NONE;
    if flags.contains(NSEventModifierFlags::Shift) {
        mods |= sys::GHOSTTY_MODS_SHIFT;
    }
    if flags.contains(NSEventModifierFlags::Control) {
        mods |= sys::GHOSTTY_MODS_CTRL;
    }
    if flags.contains(NSEventModifierFlags::Option) {
        mods |= sys::GHOSTTY_MODS_ALT;
    }
    if flags.contains(NSEventModifierFlags::Command) {
        mods |= sys::GHOSTTY_MODS_SUPER;
    }
    if flags.contains(NSEventModifierFlags::CapsLock) {
        mods |= sys::GHOSTTY_MODS_CAPS;
    }

    let raw = flags.0;
    if raw & NX_DEVICE_RSHIFT != 0 {
        mods |= sys::GHOSTTY_MODS_SHIFT_RIGHT;
    }
    if raw & NX_DEVICE_RCTRL != 0 {
        mods |= sys::GHOSTTY_MODS_CTRL_RIGHT;
    }
    if raw & NX_DEVICE_RALT != 0 {
        mods |= sys::GHOSTTY_MODS_ALT_RIGHT;
    }
    if raw & NX_DEVICE_RCMD != 0 {
        mods |= sys::GHOSTTY_MODS_SUPER_RIGHT;
    }
    mods
}

fn unshifted_codepoint(event: &NSEvent) -> u32 {
    let event_type = event.r#type();
    if event_type != NSEventType::KeyDown && event_type != NSEventType::KeyUp {
        return 0;
    }
    event
        .charactersByApplyingModifiers(NSEventModifierFlags::empty())
        .and_then(|chars| chars.to_string().chars().next())
        .map_or(0, |c| c as u32)
}

fn event_characters(event: &NSEvent) -> Option<String> {
    let characters = event.characters()?.to_string();
    let mut chars = characters.chars();
    let first = chars.next()?;
    if chars.next().is_none() {
        if (first as u32) < 0x20 {
            let flags = event.modifierFlags() & !NSEventModifierFlags::Control;
            return event
                .charactersByApplyingModifiers(flags)
                .map(|value| value.to_string());
        }
        if ('\u{F700}'..='\u{F8FF}').contains(&first) {
            return None;
        }
    }
    Some(characters)
}

fn key_event_text(text: &str) -> Option<&str> {
    let first = text.chars().next()?;
    if first.is_ascii() && (first as u32) < 0x20 {
        return None;
    }
    Some(text)
}

fn suppress_composing_control(text: &str, composing: bool) -> bool {
    if !composing {
        return false;
    }
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    chars.next().is_none() && (first as u32) < 0x20
}

fn momentum(phase: NSEventPhase) -> u8 {
    match phase {
        NSEventPhase::Began => 1,
        NSEventPhase::Stationary => 2,
        NSEventPhase::Changed => 3,
        NSEventPhase::Ended => 4,
        NSEventPhase::Cancelled => 5,
        NSEventPhase::MayBegin => 6,
        _ => 0,
    }
}
