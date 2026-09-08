use std::collections::BTreeMap;
use std::fs::{self, File, Metadata};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;

const SESSION_MINUTES: u32 = 300;
const WEEKLY_MINUTES: u32 = 10_080;
const WINDOW_TOLERANCE: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Claude,
    Codex,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Warn {
    None,
    Orange,
    Red,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    pub used: f64,
    pub minutes: u32,
    pub resets_at: Option<SystemTime>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Quota {
    pub provider: Provider,
    pub updated_at: Option<SystemTime>,
    pub session: Option<Window>,
    pub weekly: Option<Window>,
    pub fable: Option<Window>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Detail {
    pub label: String,
    pub percent: String,
    pub used: f64,
    pub reset: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Refresh {
    pub claude: Option<Quota>,
    pub codex: Option<Quota>,
}

#[derive(Deserialize)]
struct ClaudeUsage {
    five_hour: Option<ClaudeUsageWindow>,
    seven_day: Option<ClaudeUsageWindow>,
    fable_weekly: Option<ClaudeUsageWindow>,
    fable_seven_day: Option<ClaudeUsageWindow>,
    seven_day_fable: Option<ClaudeUsageWindow>,
    limits: Option<Vec<ClaudeLimit>>,
}

#[derive(Deserialize)]
struct ClaudeUsageWindow {
    utilization: Option<f64>,
    used_percentage: Option<f64>,
    resets_at: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct ClaudeLimit {
    kind: Option<String>,
    percent: Option<f64>,
    resets_at: Option<serde_json::Value>,
    scope: Option<ClaudeLimitScope>,
}

#[derive(Deserialize)]
struct ClaudeLimitScope {
    model: Option<ClaudeLimitModel>,
}

#[derive(Deserialize)]
struct ClaudeLimitModel {
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct CodexEvent {
    timestamp: String,
    #[serde(rename = "type")]
    kind: String,
    payload: CodexPayload,
}

#[derive(Deserialize)]
struct CodexPayload {
    #[serde(rename = "type")]
    kind: String,
    rate_limits: Option<CodexRateLimit>,
}

#[derive(Deserialize)]
struct CodexRateLimit {
    limit_id: Option<String>,
    primary: Option<CodexUsageWindow>,
    secondary: Option<CodexUsageWindow>,
}

#[derive(Deserialize)]
struct CodexUsageWindow {
    used_percent: f64,
    window_minutes: u32,
    resets_at: Option<f64>,
}

impl Provider {
    pub const ALL: [Self; 2] = [Self::Claude, Self::Codex];

    pub fn name(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Codex => "Codex",
        }
    }
}

pub fn warn(used: f64) -> Warn {
    if used >= 80.0 {
        Warn::Red
    } else if used >= 60.0 {
        Warn::Orange
    } else {
        Warn::None
    }
}

pub fn chip(quota: &Quota) -> Option<(String, f64)> {
    let window = tightest(quota)?;
    Some((
        format!("{}  {}", quota.provider.name(), percent_label(window.used)),
        window.used,
    ))
}

pub fn details(quota: &Quota, now: SystemTime) -> Vec<Detail> {
    let mut rows = Vec::new();
    if let Some(window) = &quota.session {
        rows.push(detail_row(window_label(window.minutes), window, now));
    }
    if let Some(window) = &quota.weekly {
        rows.push(detail_row(window_label(window.minutes), window, now));
    }
    if let Some(window) = &quota.fable {
        rows.push(detail_row("Fable", window, now));
    }
    rows
}

pub fn parse_claude_usage(raw: &str) -> Option<Quota> {
    let data: ClaudeUsage = serde_json::from_str(raw).ok()?;
    let session = map_claude_window(data.five_hour.as_ref(), SESSION_MINUTES);
    let weekly = map_claude_window(data.seven_day.as_ref(), WEEKLY_MINUTES);
    let fable = fable_window(&data);
    if session.is_none() && weekly.is_none() && fable.is_none() {
        return None;
    }
    Some(Quota {
        provider: Provider::Claude,
        updated_at: None,
        session,
        weekly,
        fable,
    })
}

fn parse_codex_event(raw: &[u8]) -> Option<Quota> {
    let event: CodexEvent = serde_json::from_slice(raw).ok()?;
    if event.kind != "event_msg" || event.payload.kind != "token_count" {
        return None;
    }
    let limits = event.payload.rate_limits?;
    if limits.limit_id.as_deref().is_some_and(|id| id != "codex") {
        return None;
    }
    let (session, weekly) = classify_codex(
        limits.primary.as_ref().and_then(map_codex_window),
        limits.secondary.as_ref().and_then(map_codex_window),
    );
    if session.is_none() && weekly.is_none() {
        return None;
    }
    Some(Quota {
        provider: Provider::Codex,
        updated_at: Some(parse_rfc3339(&event.timestamp)?),
        session,
        weekly,
        fable: None,
    })
}

const CODEX_CANDIDATE_FILES: usize = 8;
const CODEX_TAIL_BYTES: u64 = 1024 * 1024;
const CLAUDE_CACHE_BYTES: u64 = 64 * 1024;

type FileStamp = (u64, u64, SystemTime);

#[derive(Default)]
pub struct Reader {
    files: BTreeMap<PathBuf, (FileStamp, Option<Quota>)>,
}

impl Reader {
    pub fn refresh(&mut self) -> Refresh {
        self.refresh_in(&claude_home(), &codex_home())
    }

    fn refresh_in(&mut self, claude_home: &Path, codex_home: &Path) -> Refresh {
        let claude_path = claude_home.join("rate-limits.json");
        let claude = self.read(&claude_path, Provider::Claude);
        let mut candidates = Vec::new();
        rollout_files(&codex_home.join("sessions"), 3, &mut candidates);
        candidates.sort_unstable_by(|a, b| b.cmp(a));
        candidates.truncate(CODEX_CANDIDATE_FILES);
        let codex = candidates
            .iter()
            .filter_map(|(_, path)| self.read(path, Provider::Codex))
            .max_by_key(|quota| quota.updated_at);
        self.files.retain(|path, _| {
            path == &claude_path || candidates.iter().any(|(_, candidate)| candidate == path)
        });
        Refresh { claude, codex }
    }

    fn read(&mut self, path: &Path, provider: Provider) -> Option<Quota> {
        let metadata = fs::metadata(path).ok()?;
        if !metadata.is_file() {
            return None;
        }
        let stamp = file_stamp(&metadata)?;
        if let Some((previous, quota)) = self.files.get(path)
            && *previous == stamp
        {
            return quota.clone();
        }
        let mut file = File::open(path).ok()?;
        let metadata = file.metadata().ok()?;
        let stamp = file_stamp(&metadata)?;
        let budget = match provider {
            Provider::Claude => CLAUDE_CACHE_BYTES,
            Provider::Codex => CODEX_TAIL_BYTES,
        };
        let start = metadata.len().saturating_sub(budget);
        file.seek(SeekFrom::Start(start)).ok()?;
        let mut bytes = Vec::new();
        file.take(budget).read_to_end(&mut bytes).ok()?;
        let quota = match provider {
            Provider::Claude if start == 0 => std::str::from_utf8(&bytes)
                .ok()
                .and_then(parse_claude_usage),
            Provider::Claude => None,
            Provider::Codex => {
                let offset = if start == 0 {
                    0
                } else {
                    bytes
                        .iter()
                        .position(|byte| *byte == b'\n')
                        .map_or(bytes.len(), |n| n + 1)
                };
                bytes[offset..]
                    .split_inclusive(|byte| *byte == b'\n')
                    .filter(|line| line.ends_with(b"\n"))
                    .filter_map(parse_codex_event)
                    .max_by_key(|quota| quota.updated_at)
            }
        };
        self.files
            .insert(path.to_path_buf(), (stamp, quota.clone()));
        quota
    }
}

fn file_stamp(metadata: &Metadata) -> Option<FileStamp> {
    Some((metadata.ino(), metadata.len(), metadata.modified().ok()?))
}

fn rollout_files(root: &Path, depth: usize, files: &mut Vec<(SystemTime, PathBuf)>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if depth > 0 && kind.is_dir() {
            rollout_files(&path, depth - 1, files);
        } else if kind.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "jsonl")
            && entry.file_name().to_string_lossy().starts_with("rollout-")
            && let Ok(modified) = entry.metadata().and_then(|metadata| metadata.modified())
        {
            files.push((modified, path));
        }
    }
}

fn claude_home() -> PathBuf {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".claude"))
}

fn codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".codex"))
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn map_claude_window(raw: Option<&ClaudeUsageWindow>, minutes: u32) -> Option<Window> {
    let raw = raw?;
    let used = raw.utilization.or(raw.used_percentage)?;
    Some(Window {
        used: clamp_used(used),
        minutes,
        resets_at: raw.resets_at.as_ref().and_then(parse_reset_value),
    })
}

fn fable_window(data: &ClaudeUsage) -> Option<Window> {
    if let Some(limits) = &data.limits {
        for limit in limits {
            let kind = limit.kind.as_deref().unwrap_or_default();
            let name = limit
                .scope
                .as_ref()
                .and_then(|scope| scope.model.as_ref())
                .and_then(|model| model.display_name.as_deref())
                .unwrap_or_default();
            if kind == "weekly_scoped"
                && name.eq_ignore_ascii_case("fable")
                && let Some(used) = limit.percent
            {
                return Some(Window {
                    used: clamp_used(used),
                    minutes: WEEKLY_MINUTES,
                    resets_at: limit.resets_at.as_ref().and_then(parse_reset_value),
                });
            }
        }
    }
    map_claude_window(data.fable_weekly.as_ref(), WEEKLY_MINUTES)
        .or_else(|| map_claude_window(data.fable_seven_day.as_ref(), WEEKLY_MINUTES))
        .or_else(|| map_claude_window(data.seven_day_fable.as_ref(), WEEKLY_MINUTES))
}

fn map_codex_window(raw: &CodexUsageWindow) -> Option<Window> {
    let minutes = classify_minutes(raw.window_minutes)?;
    Some(Window {
        used: clamp_used(raw.used_percent),
        minutes,
        resets_at: raw.resets_at.and_then(parse_reset_number),
    })
}

fn classify_codex(
    primary: Option<Window>,
    secondary: Option<Window>,
) -> (Option<Window>, Option<Window>) {
    let mut session = None;
    let mut weekly = None;
    for window in [primary.as_ref(), secondary.as_ref()].into_iter().flatten() {
        match classify_minutes(window.minutes) {
            Some(kind) if kind == SESSION_MINUTES && session.is_none() => {
                session = Some(Window {
                    minutes: SESSION_MINUTES,
                    ..window.clone()
                });
            }
            Some(kind) if kind == WEEKLY_MINUTES && weekly.is_none() => {
                weekly = Some(Window {
                    minutes: WEEKLY_MINUTES,
                    ..window.clone()
                });
            }
            _ => {}
        }
    }
    (session, weekly)
}

fn classify_minutes(minutes: u32) -> Option<u32> {
    if minutes.abs_diff(SESSION_MINUTES) <= WINDOW_TOLERANCE {
        Some(SESSION_MINUTES)
    } else if minutes.abs_diff(WEEKLY_MINUTES) <= WINDOW_TOLERANCE {
        Some(WEEKLY_MINUTES)
    } else {
        None
    }
}

fn tightest(quota: &Quota) -> Option<&Window> {
    [quota.session.as_ref(), quota.weekly.as_ref()]
        .into_iter()
        .flatten()
        .max_by(|left, right| {
            left.used
                .partial_cmp(&right.used)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| right.minutes.cmp(&left.minutes))
        })
}

fn detail_row(label: &str, window: &Window, now: SystemTime) -> Detail {
    Detail {
        label: label.to_string(),
        percent: percent_label(window.used),
        used: window.used,
        reset: remaining_label(window.resets_at, label, now),
    }
}

fn percent_label(used: f64) -> String {
    format!("{:.0}%", (100.0 - clamp_used(used)).clamp(0.0, 100.0))
}

fn window_label(minutes: u32) -> &'static str {
    match classify_minutes(minutes).unwrap_or(minutes) {
        SESSION_MINUTES => "5h",
        WEEKLY_MINUTES => "7d",
        _ => "wk",
    }
}

fn remaining_label(resets_at: Option<SystemTime>, fallback: &str, now: SystemTime) -> String {
    let Some(resets_at) = resets_at else {
        return fallback.to_string();
    };
    let Some(left) = resets_at.duration_since(now).ok() else {
        return "0m".to_string();
    };
    let minutes = left.as_secs() / 60;
    if minutes < 60 {
        return format!("{minutes}m");
    }
    let hours = minutes / 60;
    let rest = minutes % 60;
    if hours < 24 {
        if rest == 0 {
            format!("{hours}h")
        } else {
            format!("{hours}h {rest}m")
        }
    } else {
        let days = hours / 24;
        let hours = hours % 24;
        if hours == 0 {
            format!("{days}d")
        } else {
            format!("{days}d {hours}h")
        }
    }
}

fn clamp_used(used: f64) -> f64 {
    used.clamp(0.0, 100.0)
}

fn parse_reset_value(value: &serde_json::Value) -> Option<SystemTime> {
    match value {
        serde_json::Value::Number(number) => number.as_f64().and_then(parse_reset_number),
        serde_json::Value::String(text) => parse_reset_string(text),
        _ => None,
    }
}

fn parse_reset_string(text: &str) -> Option<SystemTime> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(number) = text.parse::<f64>() {
        return parse_reset_number(number);
    }
    parse_rfc3339(text)
}

fn parse_reset_number(value: f64) -> Option<SystemTime> {
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    let millis = if value > 10_000_000_000.0 {
        value as u64
    } else {
        (value * 1000.0) as u64
    };
    UNIX_EPOCH.checked_add(Duration::from_millis(millis))
}

fn parse_rfc3339(text: &str) -> Option<SystemTime> {
    let text = text.trim();
    let (date, rest) = text.split_once('T')?;
    let mut date = date.split('-');
    let year: i32 = date.next()?.parse().ok()?;
    let month: u32 = date.next()?.parse().ok()?;
    let day: u32 = date.next()?.parse().ok()?;
    if !(1970..=9999).contains(&year) {
        return None;
    }
    let rest = rest.trim_end_matches('Z');
    let (time, offset) = match rest.rfind(['+', '-']) {
        Some(index) if index > 0 => (&rest[..index], Some(&rest[index..])),
        _ => (rest, None),
    };
    let mut time = time.split(':');
    let hour: u32 = time.next()?.parse().ok()?;
    let minute: u32 = time.next()?.parse().ok()?;
    let second = time.next().unwrap_or("0");
    let second: f64 = second.parse().ok()?;
    if hour > 23 || minute > 59 || !(0.0..60.0).contains(&second) {
        return None;
    }
    let mut total = days_from_civil(year, month, day)? * 86400
        + i64::from(hour) * 3600
        + i64::from(minute) * 60
        + second.floor() as i64;
    if let Some(offset) = offset {
        total -= parse_offset_seconds(offset)?;
    }
    if total < 0 {
        return None;
    }
    UNIX_EPOCH.checked_add(Duration::from_secs(total as u64))
}

fn parse_offset_seconds(offset: &str) -> Option<i64> {
    if offset == "Z" {
        return Some(0);
    }
    let sign = match offset.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let rest = &offset[1..];
    let (hours, minutes) = if rest.len() == 5 && rest.as_bytes().get(2) == Some(&b':') {
        (
            rest[..2].parse::<i64>().ok()?,
            rest[3..].parse::<i64>().ok()?,
        )
    } else if rest.len() == 4 {
        (
            rest[..2].parse::<i64>().ok()?,
            rest[2..].parse::<i64>().ok()?,
        )
    } else {
        return None;
    };
    if !(0..=23).contains(&hours) || !(0..=59).contains(&minutes) {
        return None;
    }
    Some(sign * (hours * 3600 + minutes * 60))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 {
        year / 400
    } else {
        (year - 399) / 400
    };
    let yoe = year - era * 400;
    let month = month as i32;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(i64::from(era) * 146097 + i64::from(doe) - 719468)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    fn now() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1_000_000)
    }

    #[test]
    fn parses_claude_windows_and_fable_limit() {
        let raw = r#"{
            "five_hour": {"used_percentage": 23.4, "resets_at": "1970-01-12T13:46:40Z"},
            "seven_day": {"utilization": 41, "resets_at": 2000000},
            "limits": [{
                "kind": "weekly_scoped",
                "percent": 12,
                "resets_at": 1757325600000,
                "scope": {"model": {"display_name": "Fable"}}
            }]
        }"#;
        let quota = parse_claude_usage(raw).unwrap();
        assert_eq!(quota.provider, Provider::Claude);
        assert_eq!(quota.session.as_ref().unwrap().used, 23.4);
        assert_eq!(
            quota.session.as_ref().unwrap().resets_at,
            Some(UNIX_EPOCH + Duration::from_secs(1_000_000))
        );
        assert_eq!(quota.weekly.as_ref().unwrap().used, 41.0);
        assert_eq!(quota.fable.as_ref().unwrap().used, 12.0);
    }

    #[test]
    fn chip_uses_tightest_bar_window() {
        let quota = Quota {
            provider: Provider::Claude,
            updated_at: Some(now()),
            session: Some(Window {
                used: 23.0,
                minutes: SESSION_MINUTES,
                resets_at: Some(now() + Duration::from_secs(2 * 3600 + 33 * 60)),
            }),
            weekly: Some(Window {
                used: 41.0,
                minutes: WEEKLY_MINUTES,
                resets_at: Some(now() + Duration::from_secs(3 * 86400)),
            }),
            fable: Some(Window {
                used: 90.0,
                minutes: WEEKLY_MINUTES,
                resets_at: None,
            }),
        };
        let (text, used) = chip(&quota).unwrap();
        assert_eq!(text, "Claude  59%");
        assert_eq!(used, 41.0);
        let rows = details(&quota, now());
        assert_eq!(rows[0].label, "5h");
        assert_eq!(rows[0].percent, "77%");
        assert_eq!(rows[2].label, "Fable");
        assert_eq!(rows[2].percent, "10%");
    }

    #[test]
    fn remaining_time_breaks_hours_and_days() {
        assert_eq!(
            remaining_label(Some(now() + Duration::from_secs(33 * 60)), "5h", now()),
            "33m"
        );
        assert_eq!(
            remaining_label(Some(now() + Duration::from_secs(2 * 3600)), "5h", now()),
            "2h"
        );
        assert_eq!(
            remaining_label(Some(now() - Duration::from_secs(10)), "5h", now()),
            "0m"
        );
        assert_eq!(remaining_label(None, "7d", now()), "7d");
    }

    #[test]
    fn warn_thresholds() {
        assert_eq!(warn(59.9), Warn::None);
        assert_eq!(warn(60.0), Warn::Orange);
        assert_eq!(warn(80.0), Warn::Red);
    }
    fn codex_line(timestamp: &str, used: f64) -> String {
        format!(
            "{}\n",
            serde_json::json!({
                "timestamp": timestamp,
                "type": "event_msg",
                "payload": {
                    "type": "token_count",
                    "rate_limits": {
                        "limit_id": "codex",
                        "primary": {"used_percent": used, "window_minutes": 10080, "resets_at": 2000000},
                        "secondary": null
                    }
                }
            })
        )
    }

    #[test]
    fn local_snapshots_follow_atomic_replacement_and_completed_log_events() {
        use std::io::Write;
        let root = std::env::temp_dir().join(format!("combe-local-quota-{}", std::process::id()));
        let claude = root.join("claude");
        let codex = root.join("codex");
        let logs = codex.join("sessions/2025/01/01");
        fs::create_dir_all(&claude).unwrap();
        fs::create_dir_all(&logs).unwrap();
        let mut reader = Reader::default();
        assert_eq!(
            reader.refresh_in(&claude, &codex),
            Refresh {
                claude: None,
                codex: None
            }
        );
        let cache = claude.join("rate-limits.json");
        let raw = r#"{"five_hour":{"used_percentage":23.6,"resets_at":2000000},"seven_day":null,"updated_at":1000000}"#;
        fs::write(&cache, raw).unwrap();
        let log = logs.join("rollout-old.jsonl");
        let first = codex_line("1970-01-12T13:46:40Z", 41.0);
        fs::write(&log, &first).unwrap();
        let first_snapshot = reader.refresh_in(&claude, &codex);
        assert_eq!(
            first_snapshot
                .claude
                .as_ref()
                .unwrap()
                .session
                .as_ref()
                .unwrap()
                .used,
            23.6
        );
        let quota = first_snapshot.codex.as_ref().unwrap();
        assert!(quota.session.is_none());
        assert_eq!(quota.weekly.as_ref().unwrap().used, 41.0);
        assert_eq!(quota.updated_at, Some(now()));
        assert_eq!(reader.refresh_in(&claude, &codex), first_snapshot);
        let replacement = claude.join("replacement");
        fs::write(&replacement, raw.replace("23.6", "33.6")).unwrap();
        fs::rename(replacement, &cache).unwrap();
        let next = codex_line("1970-01-12T14:46:40Z", 52.0);
        let mut append = fs::OpenOptions::new().append(true).open(&log).unwrap();
        append
            .write_all(&next.as_bytes()[..next.len() - 1])
            .unwrap();
        let partial = reader.refresh_in(&claude, &codex);
        assert_eq!(partial.codex, first_snapshot.codex);
        assert_eq!(partial.claude.unwrap().session.unwrap().used, 33.6);
        append.write_all(b"\n").unwrap();
        append.write_all(b"{\"type\":\"event_msg\",\"timestamp\":\"1970-01-12T15:46:40Z\",\"payload\":{\"type\":\"token_count\",\"rate_limits\":null}}\n").unwrap();
        let updated = reader.refresh_in(&claude, &codex).codex.unwrap();
        assert_eq!(updated.weekly.as_ref().unwrap().used, 52.0);
        assert_eq!(updated.updated_at, Some(now() + Duration::from_secs(3600)));
        fs::write(logs.join("rollout-newer-mtime.jsonl"), first).unwrap();
        assert_eq!(reader.refresh_in(&claude, &codex).codex, Some(updated));
        fs::write(&log, next.replace("\"codex\"", "\"codex-spark\"")).unwrap();
        assert_eq!(
            reader
                .refresh_in(&claude, &codex)
                .codex
                .unwrap()
                .weekly
                .unwrap()
                .used,
            41.0
        );
        fs::write(&cache, "null").unwrap();
        assert!(reader.refresh_in(&claude, &codex).claude.is_none());
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            reader.refresh_in(&claude, &codex),
            Refresh {
                claude: None,
                codex: None
            }
        );
    }

    #[test]
    fn bounded_tail_skips_cut_lines_and_accepts_later_snapshots() {
        use std::io::Write;
        let path = std::env::temp_dir().join(format!("combe-quota-tail-{}", std::process::id()));
        let mut file = File::create(&path).unwrap();
        file.write_all(codex_line("1970-01-12T13:46:40Z", 41.0).as_bytes())
            .unwrap();
        file.write_all(&vec![b'x'; CODEX_TAIL_BYTES as usize + 10])
            .unwrap();
        let mut reader = Reader::default();
        assert!(reader.read(&path, Provider::Codex).is_none());
        file.write_all(b"\n").unwrap();
        file.write_all(codex_line("1970-01-12T14:46:40Z", 52.0).as_bytes())
            .unwrap();
        assert_eq!(
            reader
                .read(&path, Provider::Codex)
                .unwrap()
                .weekly
                .unwrap()
                .used,
            52.0
        );
        fs::remove_file(path).unwrap();
    }
}
