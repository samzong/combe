use std::cell::Cell;
use std::ffi::{CStr, CString, c_char, c_void};
use std::fs;
use std::path::PathBuf;
use std::ptr;

use ghostty_sys as sys;
use objc2_app_kit::{NSBeep, NSPasteboard, NSPasteboardTypeString, NSWorkspace};
use objc2_foundation::{MainThreadMarker, NSString, NSURL};

use crate::habits;

thread_local! {
    static APP: Cell<sys::ghostty_app_t> = const { Cell::new(ptr::null_mut()) };
    static DARK: Cell<Option<bool>> = const { Cell::new(None) };
}

pub fn app() -> sys::ghostty_app_t {
    APP.with(Cell::get)
}

pub fn init() {
    unsafe {
        assert_eq!(
            sys::ghostty_init(0, ptr::null_mut()),
            0,
            "ghostty_init failed"
        );

        let config = load_config(true);

        let runtime = sys::ghostty_runtime_config_s {
            userdata: ptr::null_mut(),
            supports_selection_clipboard: false,
            wakeup_cb: Some(wakeup),
            action_cb: Some(action),
            read_clipboard_cb: Some(read_clipboard),
            confirm_read_clipboard_cb: Some(confirm_read_clipboard),
            write_clipboard_cb: Some(write_clipboard),
            close_surface_cb: Some(close_surface),
        };

        let handle = sys::ghostty_app_new(&runtime, config);
        sys::ghostty_config_free(config);
        assert!(!handle.is_null(), "ghostty_app_new failed");
        APP.with(|cell| cell.set(handle));
    }
}

pub fn tick() {
    let handle = app();
    if !handle.is_null() {
        unsafe { sys::ghostty_app_tick(handle) };
    }
}

pub fn set_focus(focused: bool) {
    let handle = app();
    if !handle.is_null() {
        unsafe { sys::ghostty_app_set_focus(handle, focused) };
    }
}

pub fn color_scheme() -> sys::ghostty_color_scheme_e {
    if DARK.with(Cell::get).unwrap_or(true) {
        sys::GHOSTTY_COLOR_SCHEME_DARK
    } else {
        sys::GHOSTTY_COLOR_SCHEME_LIGHT
    }
}

pub fn set_appearance(dark: bool) {
    if DARK.with(|state| state.replace(Some(dark))) == Some(dark) {
        return;
    }
    unsafe {
        let config = load_config(dark);
        sys::ghostty_app_set_color_scheme(app(), color_scheme());
        sys::ghostty_app_update_config(app(), config);
        sys::ghostty_config_free(config);
    }
}

fn load_config(dark: bool) -> sys::ghostty_config_t {
    let config_path = write_config(dark);
    let path = CString::new(config_path.as_os_str().as_encoded_bytes())
        .expect("config path has no interior nul");
    unsafe {
        let config = sys::ghostty_config_new();
        sys::ghostty_config_load_file(config, path.as_ptr());
        fs::remove_file(config_path).expect("remove generated ghostty config");
        sys::ghostty_config_finalize(config);
        report_diagnostics(config);
        config
    }
}

fn write_config(dark: bool) -> PathBuf {
    let path = std::env::temp_dir().join(format!("combe-ghostty-{}.conf", std::process::id()));
    fs::write(&path, habits::ghostty_config(dark)).expect("write generated ghostty config");
    path
}

unsafe fn report_diagnostics(config: sys::ghostty_config_t) {
    let count = unsafe { sys::ghostty_config_diagnostics_count(config) };
    for index in 0..count {
        let diagnostic = unsafe { sys::ghostty_config_get_diagnostic(config, index) };
        if diagnostic.message.is_null() {
            continue;
        }
        let message = unsafe { CStr::from_ptr(diagnostic.message) };
        eprintln!("combe: config: {}", message.to_string_lossy());
    }
}

unsafe extern "C" {
    static _dispatch_main_q: c_void;
    fn dispatch_async_f(
        queue: *mut c_void,
        context: *mut c_void,
        work: unsafe extern "C" fn(*mut c_void),
    );
}

pub fn on_main(work: unsafe extern "C" fn(*mut c_void)) {
    unsafe {
        dispatch_async_f(
            &raw const _dispatch_main_q as *mut c_void,
            ptr::null_mut(),
            work,
        );
    }
}

unsafe extern "C" fn tick_on_main(_: *mut c_void) {
    tick();
}

unsafe extern "C" fn wakeup(_: *mut c_void) {
    on_main(tick_on_main);
}

unsafe extern "C" fn action(
    _: sys::ghostty_app_t,
    target: sys::ghostty_target_s,
    action: sys::ghostty_action_s,
) -> bool {
    if action.tag == sys::GHOSTTY_ACTION_OPEN_URL {
        return open_url_action(action);
    }
    if action.tag != sys::GHOSTTY_ACTION_SET_TITLE
        && action.tag != sys::GHOSTTY_ACTION_SET_TAB_TITLE
    {
        return false;
    }
    if target.tag != sys::GHOSTTY_TARGET_SURFACE || MainThreadMarker::new().is_none() {
        return false;
    }
    let surface = unsafe { target.target.surface };
    if surface.is_null() {
        return false;
    }
    let userdata = unsafe { sys::ghostty_surface_userdata(surface) };
    let Some(view) = crate::surface::view_from_userdata(userdata) else {
        return false;
    };
    let title = unsafe { action.action.set_title.title };
    if title.is_null() {
        return false;
    }
    view.set_title(&unsafe { CStr::from_ptr(title) }.to_string_lossy());
    crate::window::refresh_labels();
    true
}

fn open_url_action(action: sys::ghostty_action_s) -> bool {
    let open = unsafe { action.action.open_url };
    if open.url.is_null() || open.len == 0 {
        return true;
    }
    let bytes = unsafe { std::slice::from_raw_parts(open.url.cast::<u8>(), open.len) };
    let Ok(text) = std::str::from_utf8(bytes) else {
        return true;
    };
    let text = text.to_owned();
    if MainThreadMarker::new().is_some() {
        open_in_browser(&text);
    } else {
        let ctx = Box::into_raw(Box::new(text)).cast();
        unsafe {
            dispatch_async_f(
                &raw const _dispatch_main_q as *mut c_void,
                ctx,
                open_url_on_main,
            );
        }
    }
    true
}

unsafe extern "C" fn open_url_on_main(ctx: *mut c_void) {
    let text = unsafe { Box::from_raw(ctx.cast::<String>()) };
    open_in_browser(&text);
}

fn open_in_browser(text: &str) {
    if !allowed_open_url(text) {
        return;
    }
    let Some(url) = NSURL::URLWithString(&NSString::from_str(text)) else {
        return;
    };
    let _ = NSWorkspace::sharedWorkspace().openURL(&url);
}

fn allowed_open_url(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() || !text.is_ascii() || text.bytes().any(|b| b < 0x20 || b == 0x7f) {
        return false;
    }
    let lower = text.to_ascii_lowercase();
    if let Some(rest) = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
    {
        let host = rest.split(['/', '?', '#']).next().unwrap_or("");
        let host = host.rsplit_once('@').map(|(_, host)| host).unwrap_or(host);
        return !host.is_empty();
    }
    lower.starts_with("mailto:") && lower.len() > "mailto:".len()
}

unsafe extern "C" fn close_surface(userdata: *mut c_void, _: bool) {
    if let Some(view) = crate::surface::view_from_userdata(userdata) {
        crate::window::queue_close(view);
    }
}

unsafe extern "C" fn read_clipboard(
    userdata: *mut c_void,
    location: sys::ghostty_clipboard_e,
    state: *mut c_void,
    _mimes: *const *const c_char,
    _mimes_len: usize,
    _list: bool,
) -> sys::ghostty_clipboard_read_result_e {
    if location != sys::GHOSTTY_CLIPBOARD_STANDARD {
        return sys::GHOSTTY_CLIPBOARD_READ_UNSUPPORTED;
    }
    let Some(surface) = crate::surface::handle_from_userdata(userdata) else {
        return sys::GHOSTTY_CLIPBOARD_READ_UNAVAILABLE;
    };

    let text = pasteboard_string().unwrap_or_default();
    let data = CString::new(text).unwrap_or_default();
    let mime = c"text/plain;charset=utf-8";
    let content = sys::ghostty_clipboard_content_s {
        mime: mime.as_ptr(),
        data: data.as_ptr(),
        len: data.as_bytes().len(),
    };
    let complete = sys::ghostty_clipboard_complete_s {
        contents: &content,
        contents_len: 1,
        available: ptr::null(),
        available_len: 0,
        confirmed: false,
        remember: false,
    };
    unsafe { sys::ghostty_surface_complete_clipboard_request(surface, &complete, state) };
    sys::GHOSTTY_CLIPBOARD_READ_STARTED
}

unsafe extern "C" fn confirm_read_clipboard(
    userdata: *mut c_void,
    _confirm: *const sys::ghostty_clipboard_confirm_s,
    state: *mut c_void,
    request: sys::ghostty_clipboard_request_e,
) {
    let Some(surface) = crate::surface::handle_from_userdata(userdata) else {
        return;
    };
    unsafe { sys::ghostty_surface_deny_clipboard_request(surface, state) };
    if request == sys::GHOSTTY_CLIPBOARD_REQUEST_PASTE {
        NSBeep();
    }
}

unsafe extern "C" fn write_clipboard(
    _userdata: *mut c_void,
    location: sys::ghostty_clipboard_e,
    contents: *const sys::ghostty_clipboard_content_s,
    contents_len: usize,
    _confirm: bool,
) {
    if location != sys::GHOSTTY_CLIPBOARD_STANDARD || contents.is_null() {
        return;
    }
    for index in 0..contents_len {
        let content = unsafe { *contents.add(index) };
        if content.data.is_null() {
            continue;
        }
        let bytes = unsafe { std::slice::from_raw_parts(content.data as *const u8, content.len) };
        let Ok(text) = std::str::from_utf8(bytes) else {
            continue;
        };
        set_pasteboard_string(text);
        return;
    }
}

fn pasteboard_string() -> Option<String> {
    unsafe {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard
            .stringForType(NSPasteboardTypeString)
            .map(|value| value.to_string())
    }
}

fn set_pasteboard_string(text: &str) {
    unsafe {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();
        pasteboard.setString_forType(&NSString::from_str(text), NSPasteboardTypeString);
    }
}

#[cfg(test)]
mod tests {
    use super::allowed_open_url;

    #[test]
    fn allows_http_https_and_mailto() {
        assert!(allowed_open_url(
            "https://github.com/lathe-cli/lathe/pull/179"
        ));
        assert!(allowed_open_url("http://example.com"));
        assert!(allowed_open_url("mailto:sam@example.com"));
    }

    #[test]
    fn rejects_unsafe_or_local_targets() {
        assert!(!allowed_open_url("https:relative"));
        assert!(!allowed_open_url("file:///etc/passwd"));
        assert!(!allowed_open_url("javascript:alert(1)"));
        assert!(!allowed_open_url("https://example.com/\u{202e}"));
        assert!(!allowed_open_url(""));
    }
}
