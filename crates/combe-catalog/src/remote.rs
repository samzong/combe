use crate::porcelain::{WorktreeKind, WorktreeRecord, parse_worktree_list};
use crate::scan::CatalogError;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::{Command, Stdio};

const KEY_SCHEME: &str = "ssh://";
pub const REMOTE_HOME: &str = "~";
const MISSING: i32 = 64;
const FOLDER: i32 = 65;
const SSH_FAILED: i32 = 255;

pub fn quote(value: &str) -> String {
    let bare = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "@%+=:,./-_".contains(c));
    if bare {
        return value.to_owned();
    }
    format!("'{}'", value.replace('\'', r#"'"'"'"#))
}

pub fn workspace_key(host: Option<&str>, path: &str) -> String {
    match host {
        None => path.to_owned(),
        Some(host) => format!("{KEY_SCHEME}{host}/{}", path.trim_start_matches('/')),
    }
}

pub fn split_key(key: &str) -> (Option<&str>, String) {
    let Some((host, rest)) = key
        .strip_prefix(KEY_SCHEME)
        .and_then(|rest| rest.split_once('/'))
    else {
        return (None, key.to_owned());
    };
    if rest == REMOTE_HOME {
        return (Some(host), REMOTE_HOME.to_owned());
    }
    (Some(host), format!("/{rest}"))
}

pub fn remote_path(path: &str) -> Option<String> {
    if !path.starts_with('/') || path.chars().any(|c| c.is_control()) {
        return None;
    }
    let trimmed = path.trim_end_matches('/');
    Some(if trimmed.is_empty() { "/" } else { trimmed }.to_owned())
}

#[derive(Clone)]
pub struct Remote {
    ssh: OsString,
    last: HashMap<(String, PathBuf), Vec<WorktreeRecord>>,
}

impl Default for Remote {
    fn default() -> Self {
        Self::with_ssh("ssh")
    }
}

impl Remote {
    pub fn with_ssh(program: impl AsRef<OsStr>) -> Self {
        Self {
            ssh: program.as_ref().to_owned(),
            last: HashMap::new(),
        }
    }

    pub(crate) fn recall(&self, host: &str, path: &std::path::Path) -> Option<&[WorktreeRecord]> {
        self.last
            .get(&(host.to_owned(), path.to_path_buf()))
            .map(Vec::as_slice)
    }

    pub(crate) fn forget(&mut self, host: &str, path: &std::path::Path) {
        self.last.remove(&(host.to_owned(), path.to_path_buf()));
    }

    pub(crate) fn scan(
        &mut self,
        host: &str,
        path: &std::path::Path,
    ) -> Result<(Vec<WorktreeRecord>, Vec<PathBuf>), CatalogError> {
        let (records, skipped): (Vec<_>, Vec<_>) =
            scan_remote(&self.ssh, host, &path.to_string_lossy())?
                .into_iter()
                .partition(|record| !record.path.to_string_lossy().chars().any(char::is_control));
        self.last
            .insert((host.to_owned(), path.to_path_buf()), records.clone());
        Ok((
            records,
            skipped.into_iter().map(|record| record.path).collect(),
        ))
    }
}

pub fn scan_remote(
    ssh: &OsStr,
    host: &str,
    path: &str,
) -> Result<Vec<WorktreeRecord>, CatalogError> {
    if !crate::is_alias(host) {
        return Err(CatalogError::InvalidHost(host.to_owned()));
    }
    let quoted = quote(path);
    let script = format!(
        "cd {quoted} || exit {MISSING}; git rev-parse --git-dir >/dev/null 2>&1 || exit {FOLDER}; exec git worktree list --porcelain"
    );
    let output = Command::new(ssh)
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            "-T",
            "--",
            host,
            &script,
        ])
        .stdin(Stdio::null())
        .output()
        .map_err(|source| CatalogError::Io {
            path: PathBuf::from(path),
            source,
        })?;
    let stderr = || String::from_utf8_lossy(&output.stderr).trim().to_string();
    match output.status.code() {
        Some(0) => Ok(parse_worktree_list(&String::from_utf8_lossy(
            &output.stdout,
        ))),
        Some(FOLDER) => Ok(vec![WorktreeRecord {
            path: PathBuf::from(path),
            kind: WorktreeKind::Folder,
            branch: None,
            head: None,
            bare: false,
            prunable: false,
        }]),
        Some(MISSING) => Err(CatalogError::RemoteMissing {
            host: host.to_owned(),
            path: PathBuf::from(path),
        }),
        Some(SSH_FAILED) | None => Err(CatalogError::Unreachable {
            host: host.to_owned(),
            stderr: stderr(),
        }),
        Some(_) => Err(CatalogError::Git {
            path: PathBuf::from(format!("{host}:{path}")),
            stderr: stderr(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_neutralizes_shell_metacharacters() {
        assert_eq!(quote("/tmp/plain.command"), "/tmp/plain.command");
        assert_eq!(quote("/tmp/a b;id"), "'/tmp/a b;id'");
        assert_eq!(quote("/tmp/it's"), r#"'/tmp/it'"'"'s'"#);
        assert_eq!(quote("$(id)"), "'$(id)'");
        assert_eq!(quote(""), "''");
    }

    #[test]
    fn keys_round_trip() {
        assert_eq!(
            workspace_key(None, "/Users/x/git/combe"),
            "/Users/x/git/combe"
        );
        let key = workspace_key(Some("xbp"), "/Users/x/git/combe");
        assert_eq!(key, "ssh://xbp/Users/x/git/combe");
        assert_eq!(
            split_key(&key),
            (Some("xbp"), "/Users/x/git/combe".to_owned())
        );
        let home = workspace_key(Some("xbp"), REMOTE_HOME);
        assert_eq!(split_key(&home), (Some("xbp"), REMOTE_HOME.to_owned()));
        assert_eq!(split_key("/Users/x"), (None, "/Users/x".to_owned()));
    }

    #[test]
    fn remote_paths_must_be_absolute_and_printable() {
        assert_eq!(remote_path("/srv/repo/"), Some("/srv/repo".to_owned()));
        assert_eq!(remote_path("/"), Some("/".to_owned()));
        assert_eq!(remote_path("relative"), None);
        assert_eq!(remote_path("~/git"), None);
        assert_eq!(remote_path("/a\nb"), None);
    }

    #[test]
    fn rejects_option_shaped_hosts_before_running_ssh() {
        let err = scan_remote(OsStr::new("/nonexistent"), "-oProxyCommand=id", "/tmp").unwrap_err();
        assert!(matches!(err, CatalogError::InvalidHost(_)));
    }
}
