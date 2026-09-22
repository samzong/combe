use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use objc2_foundation::{NSDate, NSDateFormatter, NSString, NSUUID};
use serde::{Deserialize, Serialize};

use crate::habits;
use crate::log::note;

const SCHEMA: u32 = 2;
const DIR_ENV: &str = "COMBE_TELEMETRY_DIR";

static ENABLED: AtomicBool = AtomicBool::new(false);
static DROPPED: AtomicU64 = AtomicU64::new(0);
static SENDER: OnceLock<SyncSender<Op>> = OnceLock::new();
static SESSION: OnceLock<String> = OnceLock::new();
static ORIGIN: OnceLock<Instant> = OnceLock::new();

thread_local! {
    static SEQ: Cell<u64> = const { Cell::new(0) };
    static IDS: RefCell<Ids> = RefCell::new(Ids::default());
    static LAST: Cell<Option<Instant>> = const { Cell::new(None) };
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Source {
    Key,
    Menu,
    Button,
    TabBar,
    Sidebar,
    Overview,
    Notification,
    Terminal,
    Process,
    Url,
    App,
    Startup,
    Auto,
    System,
}

impl Source {
    fn interactive(self) -> bool {
        !matches!(
            self,
            Self::Process | Self::Startup | Self::Auto | Self::System
        )
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Outcome {
    Ok,
    Cancel,
    Noop,
}

#[derive(Default)]
struct Ids {
    keys: HashMap<String, u32>,
    next: u32,
}

impl Ids {
    fn id(&mut self, key: &str) -> u32 {
        if let Some(found) = self.keys.get(key) {
            return *found;
        }
        self.next += 1;
        self.keys.insert(key.to_owned(), self.next);
        self.next
    }
}

#[derive(Clone, Copy, Serialize)]
struct Record {
    event: &'static str,
    source: Source,
    seq: u64,
    unix_ms: u64,
    mono_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tab: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pane: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tabs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    panes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    outcome: Option<Outcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    idle_ms: Option<u64>,
}

#[derive(Serialize)]
struct Line<'a> {
    v: u32,
    app: &'a str,
    session: &'a str,
    #[serde(flatten)]
    record: Record,
    #[serde(skip_serializing_if = "Option::is_none")]
    dropped: Option<u64>,
}

fn line(record: Record, session: &str, dropped: u64) -> Line<'_> {
    Line {
        v: SCHEMA,
        app: env!("CARGO_PKG_VERSION"),
        session,
        record,
        dropped: (dropped > 0).then_some(dropped),
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Switch {
    pub(crate) enabled: bool,
    pub(crate) changed_at_unix_ms: u64,
}

enum Op {
    Line(Record),
    Flush(SyncSender<()>),
}

pub(crate) struct Entry {
    record: Option<Record>,
}

impl Entry {
    fn with(mut self, fill: impl FnOnce(&mut Record)) -> Self {
        if let Some(record) = self.record.as_mut() {
            fill(record);
        }
        self
    }

    pub(crate) fn workspace(self, path: &str) -> Self {
        self.with(|record| record.workspace = Some(stable(path)))
    }

    pub(crate) fn tab(self, id: u64) -> Self {
        self.with(|record| record.tab = Some(id))
    }

    pub(crate) fn pane(self, key: &str) -> Self {
        self.with(|record| record.pane = Some(intern(key)))
    }

    pub(crate) fn counts(self, tabs: usize, panes: usize) -> Self {
        self.with(|record| {
            record.tabs = Some(tabs as u32);
            record.panes = Some(panes as u32);
        })
    }

    pub(crate) fn detail(self, detail: &'static str) -> Self {
        self.with(|record| record.detail = Some(detail))
    }

    pub(crate) fn outcome(self, outcome: Outcome) -> Self {
        self.with(|record| record.outcome = Some(outcome))
    }

    fn idle(self, span: Duration) -> Self {
        self.with(|record| record.idle_ms = Some(span.as_millis() as u64))
    }

    pub(crate) fn emit(self) {
        if let Some(record) = self.record {
            send(Op::Line(record));
        }
    }
}

fn intern(key: &str) -> u32 {
    IDS.with(|ids| ids.borrow_mut().id(key))
}

fn stable(path: &str) -> u64 {
    let mut value: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in path.as_bytes() {
        value ^= u64::from(*byte);
        value = value.wrapping_mul(0x1000_0000_01b3);
    }
    value
}

pub(crate) fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub(crate) fn record(event: &'static str, source: Source) -> Entry {
    if !ENABLED.load(Ordering::Relaxed) {
        return Entry { record: None };
    }
    if source.interactive() {
        touch();
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let origin = ORIGIN.get_or_init(Instant::now);
    let seq = SEQ.with(|seq| {
        let next = seq.get() + 1;
        seq.set(next);
        next
    });
    Entry {
        record: Some(Record {
            event,
            source,
            seq,
            unix_ms: now.as_millis() as u64,
            mono_ms: origin.elapsed().as_millis() as u64,
            workspace: None,
            tab: None,
            pane: None,
            tabs: None,
            panes: None,
            detail: None,
            outcome: None,
            idle_ms: None,
        }),
    }
}

pub(crate) fn touch() {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let now = Instant::now();
    let Some(previous) = LAST.with(|last| last.replace(Some(now))) else {
        return;
    };
    let span = now.duration_since(previous);
    if span >= habits::TELEMETRY_IDLE_AFTER {
        record("idle.resume", Source::System).idle(span).emit();
    }
}

fn send(op: Op) {
    let Some(sender) = writer() else {
        DROPPED.fetch_add(1, Ordering::Relaxed);
        return;
    };
    if sender.try_send(op).is_err() {
        DROPPED.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) fn flush() {
    if SENDER.get().is_none() {
        return;
    }
    let (ack, done) = sync_channel(1);
    send(Op::Flush(ack));
    let _ = done.recv_timeout(habits::TELEMETRY_FLUSH);
}

fn writer() -> Option<&'static SyncSender<Op>> {
    if let Some(sender) = SENDER.get() {
        return Some(sender);
    }
    let path = events_path()?;
    let session = SESSION.get()?.clone();
    let (sender, receiver) = sync_channel(habits::TELEMETRY_QUEUE);
    thread::Builder::new()
        .name("combe-telemetry".into())
        .spawn(move || run(receiver, path, session))
        .ok()?;
    Some(SENDER.get_or_init(|| sender))
}

fn run(receiver: Receiver<Op>, path: PathBuf, session: String) {
    let mut file = open(&path);
    while let Ok(op) = receiver.recv() {
        match op {
            Op::Line(record) => {
                let dropped = DROPPED.swap(0, Ordering::Relaxed);
                if dropped > 0 {
                    note!("telemetry dropped {dropped} records");
                }
                let Some(handle) = file.as_mut() else {
                    continue;
                };
                let Ok(body) = serde_json::to_string(&line(record, &session, dropped)) else {
                    continue;
                };
                if let Err(error) = writeln!(handle, "{body}") {
                    note!("telemetry write failed: {error}");
                    file = None;
                }
            }
            Op::Flush(ack) => {
                let _ = ack.send(());
            }
        }
    }
}

fn open(path: &Path) -> Option<fs::File> {
    if let Some(parent) = path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        note!("telemetry directory unusable: {error}");
        return None;
    }
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(file) => Some(file),
        Err(error) => {
            note!("telemetry log unopenable: {error}");
            None
        }
    }
}

fn dir() -> Option<PathBuf> {
    if let Some(custom) = std::env::var_os(DIR_ENV) {
        return Some(PathBuf::from(custom));
    }
    Some(combe_catalog::state_path()?.parent()?.join("telemetry"))
}

fn switch_path() -> Option<PathBuf> {
    Some(dir()?.join("switch.json"))
}

pub(crate) fn events_path() -> Option<PathBuf> {
    Some(dir()?.join("events.jsonl"))
}

pub(crate) fn read_switch() -> Result<Option<Switch>, String> {
    let Some(path) = switch_path() else {
        return Err("no application support directory".to_owned());
    };
    let body = match fs::read_to_string(&path) {
        Ok(body) => body,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    serde_json::from_str(&body)
        .map(Some)
        .map_err(|error| format!("{}: {error}", path.display()))
}

pub(crate) fn write_switch(enabled: bool) -> Result<(), String> {
    let Some(path) = switch_path() else {
        return Err("no application support directory".to_owned());
    };
    let switch = Switch {
        enabled,
        changed_at_unix_ms: unix_ms(),
    };
    let body = serde_json::to_string_pretty(&switch).map_err(|error| error.to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temp = path.with_extension("tmp");
    fs::write(&temp, &body).map_err(|error| error.to_string())?;
    fs::rename(&temp, &path).map_err(|error| error.to_string())
}

pub(crate) fn stamp(unix_ms: u64) -> String {
    let formatter = NSDateFormatter::new();
    formatter.setDateFormat(Some(&NSString::from_str("yyyy-MM-dd HH:mm:ss")));
    let date = NSDate::dateWithTimeIntervalSince1970(unix_ms as f64 / 1000.0);
    formatter.stringFromDate(&date).to_string()
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub(crate) fn start() {
    SESSION.get_or_init(|| NSUUID::UUID().UUIDString().to_string());
    ORIGIN.get_or_init(Instant::now);
    LAST.with(|last| last.set(Some(Instant::now())));
    apply(enabled_on_disk(), true);
}

pub(crate) fn reload() {
    apply(enabled_on_disk(), false);
}

fn enabled_on_disk() -> bool {
    match read_switch() {
        Ok(switch) => switch.is_some_and(|switch| switch.enabled),
        Err(error) => {
            note!("telemetry switch unreadable, staying off: {error}");
            false
        }
    }
}

fn apply(enabled: bool, boot: bool) {
    if enabled == ENABLED.load(Ordering::Relaxed) {
        return;
    }
    if enabled {
        ENABLED.store(true, Ordering::Relaxed);
        LAST.with(|last| last.set(Some(Instant::now())));
        record(
            if boot { "app.start" } else { "telemetry.on" },
            Source::System,
        )
        .emit();
        note!("telemetry on");
        return;
    }
    record("telemetry.off", Source::System).emit();
    ENABLED.store(false, Ordering::Relaxed);
    flush();
    note!("telemetry off");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full() -> Record {
        Record {
            event: "tab.close",
            source: Source::Key,
            seq: 7,
            unix_ms: 1_758_470_645_123,
            mono_ms: 186_423,
            workspace: Some(0x2b8f_1a03_77c4_e519),
            tab: Some(9),
            pane: Some(4),
            tabs: Some(3),
            panes: Some(2),
            detail: Some("right"),
            outcome: Some(Outcome::Cancel),
            idle_ms: Some(180_000),
        }
    }

    fn keys(body: &str) -> Vec<String> {
        let value: serde_json::Value = serde_json::from_str(body).expect("json object");
        let mut keys: Vec<String> = value
            .as_object()
            .expect("object")
            .keys()
            .map(String::from)
            .collect();
        keys.sort();
        keys
    }

    #[test]
    fn every_field_is_on_the_whitelist() {
        let body = serde_json::to_string(&line(full(), "session", 5)).expect("line");
        assert_eq!(
            keys(&body),
            [
                "app",
                "detail",
                "dropped",
                "event",
                "idle_ms",
                "mono_ms",
                "outcome",
                "pane",
                "panes",
                "seq",
                "session",
                "source",
                "tab",
                "tabs",
                "unix_ms",
                "v",
                "workspace",
            ]
        );
    }

    #[test]
    fn an_empty_record_writes_only_the_base_fields() {
        let record = Record {
            workspace: None,
            tab: None,
            pane: None,
            tabs: None,
            panes: None,
            detail: None,
            outcome: None,
            idle_ms: None,
            ..full()
        };
        let body = serde_json::to_string(&line(record, "session", 0)).expect("line");
        assert_eq!(
            keys(&body),
            [
                "app", "event", "mono_ms", "seq", "session", "source", "unix_ms", "v",
            ]
        );
    }

    #[test]
    fn identifiers_replace_the_text_they_stand_for() {
        let mut ids = Ids::default();
        let workspace = "/Users/sentinel-person/git/sentinel-project";
        let pane = "SENTINEL-4F0A-PANE-UUID";
        let record = Record {
            workspace: Some(stable(workspace)),
            pane: Some(ids.id(pane)),
            detail: None,
            ..full()
        };
        let body = serde_json::to_string(&line(record, "session", 0)).expect("line");
        assert!(!body.contains("sentinel"), "{body}");
        assert!(!body.contains("SENTINEL"), "{body}");
        assert!(!body.contains('/'), "{body}");
        assert_eq!(ids.id(pane), 1);
    }

    #[test]
    fn a_workspace_keeps_its_identifier_across_sessions() {
        let worktree = "/Users/sentinel-person/git/sentinel-project";
        let sibling = "/Users/sentinel-person/git/sentinel-project-fix";
        assert_eq!(stable(worktree), 17_981_198_846_072_539_745);
        assert_ne!(stable(worktree), stable(sibling));
    }
}
