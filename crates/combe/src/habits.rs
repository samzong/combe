use std::fmt::Write as _;

pub const SHELL: &str = "/bin/zsh -l";

pub const FONT_FAMILY: &str = "Fira Code";
pub const FONT_FAMILY_CJK: &str = "Noto Sans Mono CJK SC";
pub const CJK_CODEPOINTS: &str = "U+2E80-U+2FFF,U+3000-U+303F,U+31C0-U+31EF,U+3400-U+4DBF,U+4E00-U+9FFF,U+F900-U+FAFF,U+FF00-U+FFEF";
pub const FONT_SIZE: f64 = 12.0;
pub const LINE_HEIGHT: f64 = FONT_SIZE + 6.0;

pub const BACKGROUND: &str = "0d1117";
pub const FOREGROUND: &str = "e6edf3";
pub const CURSOR_COLOR: &str = "2f81f7";
pub const CURSOR_TEXT: &str = "6fc1ff";
pub const SELECTION_BACKGROUND: &str = "e6edf3";
pub const SELECTION_FOREGROUND: &str = "0d1117";

pub const PALETTE: [&str; 16] = [
    "484f58", "ff7b72", "3fb950", "d29922", "58a6ff", "bc8cff", "39c5cf", "b1bac4", "6e7681",
    "ffa198", "56d364", "e3b341", "79c0ff", "d2a8ff", "56d4dd", "ffffff",
];

pub const PADDING_X: u16 = 8;
pub const PADDING_Y: u16 = 6;
pub const PADDING_BALANCE: bool = true;

pub const CURSOR_STYLE: &str = "block";
pub const CURSOR_BLINK: bool = true;

pub const OPTION_AS_ALT: bool = true;
pub const COPY_ON_SELECT: bool = false;

pub const SCROLLBACK_LINES: usize = 10_000;

pub const WINDOW_WIDTH: f64 = 1200.0;
pub const WINDOW_HEIGHT: f64 = 780.0;

pub const SIDEBAR_WIDTH: f64 = 220.0;
pub const SIDEBAR_VISIBLE: bool = true;

pub const ALLOW_OSC52_READ: bool = false;

pub fn ghostty_config() -> String {
    let mut config = String::new();
    let _ = writeln!(config, "font-family = {FONT_FAMILY}");
    let _ = writeln!(config, "font-family = {FONT_FAMILY_CJK}");
    let _ = writeln!(
        config,
        "font-codepoint-map = {CJK_CODEPOINTS}={FONT_FAMILY_CJK}"
    );
    let _ = writeln!(config, "font-size = {FONT_SIZE}");
    let _ = writeln!(config, "background = {BACKGROUND}");
    let _ = writeln!(config, "foreground = {FOREGROUND}");
    let _ = writeln!(config, "cursor-color = {CURSOR_COLOR}");
    let _ = writeln!(config, "cursor-text = {CURSOR_TEXT}");
    let _ = writeln!(config, "selection-background = {SELECTION_BACKGROUND}");
    let _ = writeln!(config, "selection-foreground = {SELECTION_FOREGROUND}");
    for (index, color) in PALETTE.iter().enumerate() {
        let _ = writeln!(config, "palette = {index}={color}");
    }
    let _ = writeln!(config, "window-padding-x = {PADDING_X}");
    let _ = writeln!(config, "window-padding-y = {PADDING_Y}");
    let _ = writeln!(config, "window-padding-balance = {PADDING_BALANCE}");
    let _ = writeln!(config, "cursor-style = {CURSOR_STYLE}");
    let _ = writeln!(config, "cursor-style-blink = {CURSOR_BLINK}");
    let _ = writeln!(config, "macos-option-as-alt = {OPTION_AS_ALT}");
    let _ = writeln!(config, "copy-on-select = {COPY_ON_SELECT}");
    let _ = writeln!(config, "scrollback-limit-lines = {SCROLLBACK_LINES}");
    let _ = writeln!(config, "command = {SHELL}");
    let _ = writeln!(config, "bell-features = no-attention,no-title");
    let _ = writeln!(config, "link-url = true");
    let _ = writeln!(config, "mouse-hide-while-typing = true");
    let _ = writeln!(config, "resize-overlay = never");
    let _ = writeln!(config, "clipboard-write = allow");
    let _ = writeln!(
        config,
        "clipboard-read = {}",
        if ALLOW_OSC52_READ { "allow" } else { "ask" }
    );
    let _ = writeln!(config, "clipboard-paste-protection = true");
    config
}
