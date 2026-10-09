use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, PoisonError};

use crate::log::note;
use combe_catalog::{
    Catalog, CatalogError, HOME_LABEL, REMOTE_HOME, Reach, Remote, State, Workspace, add_repo,
    catalog, home_dir, is_alias, is_home_path, load_state, save_state, should_inject_home,
    state_path, workspace_key,
};

static REMOTE: LazyLock<Mutex<Remote>> = LazyLock::new(|| Mutex::new(Remote::default()));

pub(crate) struct Row {
    pub label: String,
    pub key: String,
}

pub(crate) struct Repo {
    pub host: Option<String>,
    pub key: String,
    pub name: String,
    pub rows: Vec<Row>,
    pub has_heading: bool,
}

#[derive(Default)]
pub(crate) struct Listing {
    pub repos: Vec<Repo>,
    pub hosts: Vec<String>,
    pub unreachable: BTreeSet<String>,
}

impl Listing {
    pub(crate) fn label(&self, key: &str) -> Option<String> {
        self.repos
            .iter()
            .flat_map(|repo| &repo.rows)
            .find(|row| row.key == key)
            .map(|row| row.label.clone())
    }
}

pub(crate) fn listing(scan: bool) -> Listing {
    let Some(state) = read_state() else {
        return with_homes(Vec::new(), &[], Vec::new(), BTreeSet::new());
    };
    let found = match read_catalog(&REMOTE, &state, scan) {
        Ok(found) => found,
        Err(err) => {
            note!("{err}");
            return with_homes(Vec::new(), &[], hosts(&state), BTreeSet::new());
        }
    };
    for err in &found.errors {
        if scan || !matches!(err, CatalogError::Unreachable { .. }) {
            note!("{err}");
        }
    }

    let mut repos: Vec<Repo> = Vec::new();
    for repo in &state.repos {
        let rows: Vec<Row> = found
            .rows
            .iter()
            .filter(|row| row.host == repo.host && row.repo_path == repo.path)
            .map(|row| Row {
                label: row.label(),
                key: row.key(),
            })
            .collect();
        if rows.is_empty() {
            continue;
        }
        repos.push(Repo {
            host: repo.host.clone(),
            key: workspace_key(repo.host.as_deref(), &repo.path.to_string_lossy()),
            name: repo_name(repo.host.is_none(), &repo.path),
            rows,
            has_heading: true,
        });
    }
    with_homes(repos, &found.rows, hosts(&state), found.unreachable)
}

fn read_catalog(
    remote: &Mutex<Remote>,
    state: &State,
    scan: bool,
) -> Result<Catalog, CatalogError> {
    let lock = || remote.lock().unwrap_or_else(PoisonError::into_inner);
    if !scan {
        return catalog(state, Reach::Cached(&lock()));
    }
    let mut snapshot = lock().clone();
    let found = catalog(state, Reach::Scan(&mut snapshot));
    *lock() = snapshot;
    found
}

pub(crate) fn rows() -> Vec<Workspace> {
    let Some(state) = read_state() else {
        return Vec::new();
    };
    match catalog(&state, Reach::Local) {
        Ok(found) => found.rows,
        Err(err) => {
            note!("{err}");
            Vec::new()
        }
    }
}

fn hosts(state: &State) -> Vec<String> {
    let mut hosts = combe_catalog::hosts();
    for host in state.repos.iter().filter_map(|repo| repo.host.as_ref()) {
        if is_alias(host) && !hosts.contains(host) {
            hosts.push(host.clone());
        }
    }
    hosts
}

fn with_homes(
    mut repos: Vec<Repo>,
    rows: &[Workspace],
    hosts: Vec<String>,
    unreachable: BTreeSet<String>,
) -> Listing {
    let mut homes = Vec::new();
    if let Some(home) = home_dir() {
        let local: Vec<Workspace> = rows
            .iter()
            .filter(|row| row.host.is_none())
            .cloned()
            .collect();
        if should_inject_home(&local, &home) {
            homes.push(synthetic_home(None, &home.to_string_lossy()));
        }
    }
    for host in &hosts {
        homes.push(synthetic_home(Some(host), REMOTE_HOME));
    }
    repos.splice(0..0, homes);
    Listing {
        repos,
        hosts,
        unreachable,
    }
}

fn synthetic_home(host: Option<&str>, path: &str) -> Repo {
    let key = workspace_key(host, path);
    Repo {
        host: host.map(str::to_owned),
        key: key.clone(),
        name: HOME_LABEL.to_string(),
        rows: vec![Row {
            label: HOME_LABEL.to_string(),
            key,
        }],
        has_heading: false,
    }
}

pub(crate) fn add(paths: &[PathBuf]) {
    let Some(file) = state_path() else {
        note!("cannot resolve the application support directory");
        return;
    };
    let Some(mut state) = read_state() else {
        return;
    };
    for path in paths {
        if let Err(err) = add_repo(&mut state, path) {
            note!("{err}");
            return;
        }
    }
    if let Err(err) = save_state(&file, &state) {
        note!("{err}");
    }
}

pub(crate) fn probe_remote(host: &str, path: &str) -> Result<String, String> {
    let path = combe_catalog::remote_path(path)
        .ok_or_else(|| format!("{path} is not an absolute path on {host}."))?;
    combe_catalog::scan_remote(std::ffi::OsStr::new("ssh"), host, &path)
        .map(|_| path)
        .map_err(|err| err.to_string())
}

pub(crate) fn add_remote(host: &str, path: &str) -> Result<(), String> {
    let file = state_path().ok_or("cannot resolve the application support directory")?;
    let mut state = read_state().ok_or("cannot read the state file")?;
    combe_catalog::add_remote_repo(&mut state, host, path).map_err(|err| err.to_string())?;
    save_state(&file, &state).map_err(|err| err.to_string())
}

fn read_state() -> Option<State> {
    let Some(file) = state_path() else {
        note!("cannot resolve the application support directory");
        return None;
    };
    match load_state(&file) {
        Ok(state) => Some(state),
        Err(err) => {
            note!("{err}");
            None
        }
    }
}

fn repo_name(local: bool, path: &Path) -> String {
    if local && is_home_path(path) {
        return HOME_LABEL.to_string();
    }
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_else(|| path.to_str().unwrap_or("repo"))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;
    use std::time::{Duration, Instant};

    fn run(cwd: &Path, program: &str, args: &[&str]) {
        let output = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn cached_reads_do_not_wait_for_a_running_scan() {
        let root = std::env::temp_dir().join(format!("combe-sidebar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo = root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        run(&repo, "git", &["init", "-q", "-b", "main"]);
        let slow = root.join("slow");
        let ssh = root.join("ssh");
        std::fs::write(
            &ssh,
            format!(
                "#!/bin/sh\n[ -e '{}' ] && sleep 3\nfor last; do :; done\nexec /bin/sh -c \"$last\"\n",
                slow.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&ssh, std::fs::Permissions::from_mode(0o755)).unwrap();
        let state = State {
            repos: vec![combe_catalog::Repo {
                path: repo.clone(),
                host: Some("fake".into()),
            }],
        };
        let remote = std::sync::Arc::new(Mutex::new(Remote::with_ssh(&ssh)));
        let primed = read_catalog(&remote, &state, true).unwrap().rows;
        assert_eq!(primed.len(), 1);

        std::fs::write(&slow, "").unwrap();
        let scanner = {
            let remote = remote.clone();
            let state = state.clone();
            std::thread::spawn(move || read_catalog(&remote, &state, true).unwrap().rows)
        };
        std::thread::sleep(Duration::from_millis(500));
        let started = Instant::now();
        let cached = read_catalog(&remote, &state, false).unwrap().rows;
        let waited = started.elapsed();
        assert_eq!(cached, primed);
        assert!(waited < Duration::from_millis(500), "{waited:?}");
        assert!(!scanner.is_finished());
        assert_eq!(scanner.join().unwrap(), primed);
        let _ = std::fs::remove_dir_all(&root);
    }
}
