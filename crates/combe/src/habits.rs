use std::fmt::Write as _;
use std::time::Duration;

pub const SHELL: &str = "/bin/zsh -l";

pub const FONT_FAMILY: &str = "Fira Code";
pub const FONT_FAMILY_CJK: &str = "Noto Sans Mono CJK SC";
pub const CJK_CODEPOINTS: &str = "U+2E80-U+2FFF,U+3000-U+303F,U+31C0-U+31EF,U+3400-U+4DBF,U+4E00-U+9FFF,U+F900-U+FAFF,U+FF00-U+FFEF";
pub const FONT_SIZE: f64 = 13.0;

pub const CHROME_ICON_SIZE: f64 = 12.0;
pub const CHROME_FONT_SIZE: f64 = 12.0;
pub const CHROME_LINE_HEIGHT: f64 = 18.0;

pub const CHROME_TEXT: (u32, u32) = (0x24292fff, 0xf1f1f2ff);
pub const CHROME_STRONG: (u32, u32) = (0x292d33ff, 0xe6e7e9ff);
pub const CHROME_MUTED: (u32, u32) = (0x656970ff, 0x9c9ea3ff);
pub const CHROME_SOFT: (u32, u32) = (0x555960ff, 0xc3c4c7ff);
pub const CHROME_BUTTON_HOVER: (u32, u32) = (0x00000009, 0xffffff15);
pub const CHROME_BUTTON_PRESSED: (u32, u32) = (0x00000018, 0xffffff26);
pub const CHROME_SELECTION: (u32, u32) = (0x00000008, 0xffffff12);
pub const CHROME_HINT: (u32, u32) = (0x00000006, 0xffffff0d);
pub const CHROME_SESSION: (u32, u32) = (0x248247ff, 0x65c888ff);
pub const CHROME_SESSION_IDLE: (u32, u32) = (0x85878dff, 0x85878dff);
pub const CHROME_ATTENTION: (u32, u32) = (0x0969daff, 0x58a6ffff);
pub const GLASS_CONTROL: [(u32, u32); 4] = [
    (0xdfe1e650, 0x3b3d4140),
    (0xffffffa6, 0xffffff12),
    (0xffffff26, 0xffffff00),
    (0xffffff70, 0xffffff06),
];
pub const GLASS_PANEL: [(u32, u32); 4] = [
    (0xe1e3e8b0, 0x35373b9e),
    (0xffffffbf, 0xffffff1f),
    (0xffffff40, 0xffffff03),
    (0xffffff80, 0xffffff0b),
];
pub const GLASS_QUOTA: [(u32, u32); 4] = [
    (0xd9dce31a, 0x3b3d411a),
    (0xffffff70, 0xffffff08),
    (0xffffff10, 0xffffff00),
    (0xffffff30, 0xffffff03),
];
pub const GLASS_QUOTA_PANEL: [(u32, u32); 4] = [
    (0xe1e3e89e, 0x292b2f70),
    (0xffffffa0, 0xffffff14),
    (0xffffff30, 0xffffff02),
    (0xffffff70, 0xffffff07),
];
pub const GLASS_EDGE: (u32, u32) = (0xffffffb0, 0xffffff20);
pub const GLASS_PANEL_EDGE: (u32, u32) = (0xffffffb0, 0xffffff2e);

pub const BACKGROUND: &str = "161719";
pub const FOREGROUND: &str = "e6edf3";
pub const CURSOR_COLOR: &str = "2f81f7";
pub const CURSOR_TEXT: &str = "6fc1ff";
pub const SELECTION_BACKGROUND: &str = "e6edf3";
pub const SELECTION_FOREGROUND: &str = "0d1117";

pub const PALETTE: [&str; 16] = [
    "484f58", "ff7b72", "3fb950", "d29922", "58a6ff", "bc8cff", "39c5cf", "b1bac4", "6e7681",
    "ffa198", "56d364", "e3b341", "79c0ff", "d2a8ff", "56d4dd", "ffffff",
];

pub const LIGHT_PALETTE: [&str; 16] = [
    "24292f", "cf222e", "116329", "4d2d00", "0969da", "8250df", "1b7c83", "6e7781", "57606a",
    "a40e26", "1a7f37", "633c01", "218bff", "a475f9", "3192aa", "ffffff",
];

pub fn background(dark: bool) -> &'static str {
    if dark { BACKGROUND } else { "f5f5f7" }
}

pub const PADDING_X: u16 = 8;
pub const PADDING_Y: u16 = 8;
pub const PADDING_BALANCE: bool = true;

pub const CURSOR_STYLE: &str = "block";
pub const CURSOR_BLINK: bool = true;
pub const SHELL_INTEGRATION_FEATURES: &str = "no-cursor";

pub const OPTION_AS_ALT: bool = true;
pub const COPY_ON_SELECT: bool = true;

pub const SCROLLBACK_LINES: usize = 10_000;

pub const WINDOW_WIDTH: f64 = 1200.0;
pub const WINDOW_HEIGHT: f64 = 780.0;

pub const SIDEBAR_WIDTH: f64 = 300.0;
pub const SIDEBAR_VISIBLE: bool = false;

pub const QUOTA_POLL: Duration = Duration::from_secs(15 * 60);
pub const QUOTA_FRESH: Duration = Duration::from_secs(5 * 60);
pub const COMMAND_NOTIFY_AFTER: Duration = Duration::from_secs(5);
pub const NOTIFICATION_BATCH: Duration = Duration::from_millis(250);
pub const NOTIFICATION_INTERVAL: Duration = Duration::from_secs(5);
pub const PANE_GUIDE_DURATION: f64 = 1.2;

pub const ALLOW_OSC52_READ: bool = false;

pub fn ghostty_config(dark: bool) -> String {
    let background = background(dark);
    let (foreground, cursor, cursor_text, selection, selection_text, palette) = if dark {
        (
            FOREGROUND,
            CURSOR_COLOR,
            CURSOR_TEXT,
            SELECTION_BACKGROUND,
            SELECTION_FOREGROUND,
            PALETTE,
        )
    } else {
        (
            "24292f",
            "0969da",
            "ffffff",
            "b6e3ff",
            "24292f",
            LIGHT_PALETTE,
        )
    };
    let mut config = String::new();
    let _ = writeln!(config, "font-family = {FONT_FAMILY}");
    let _ = writeln!(config, "font-family = {FONT_FAMILY_CJK}");
    let _ = writeln!(
        config,
        "font-codepoint-map = {CJK_CODEPOINTS}={FONT_FAMILY_CJK}"
    );
    let _ = writeln!(config, "font-size = {FONT_SIZE}");
    let _ = writeln!(config, "background = {background}");
    let _ = writeln!(config, "foreground = {foreground}");
    let _ = writeln!(config, "cursor-color = {cursor}");
    let _ = writeln!(config, "cursor-text = {cursor_text}");
    let _ = writeln!(config, "selection-background = {selection}");
    let _ = writeln!(config, "selection-foreground = {selection_text}");
    for (index, color) in palette.iter().enumerate() {
        let _ = writeln!(config, "palette = {index}={color}");
    }
    let _ = writeln!(config, "window-padding-x = {PADDING_X}");
    let _ = writeln!(config, "window-padding-y = {PADDING_Y}");
    let _ = writeln!(config, "window-padding-balance = {PADDING_BALANCE}");
    let _ = writeln!(config, "cursor-style = {CURSOR_STYLE}");
    let _ = writeln!(config, "cursor-style-blink = {CURSOR_BLINK}");
    let _ = writeln!(
        config,
        "shell-integration-features = {SHELL_INTEGRATION_FEATURES}"
    );
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
