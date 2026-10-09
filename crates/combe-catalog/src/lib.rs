mod home;
mod porcelain;
mod remote;
mod scan;
mod ssh_config;
mod store;

pub use home::{HOME_LABEL, home_dir, home_workspace, is_home_path, should_inject_home};
pub use porcelain::{WorktreeKind, WorktreeRecord, parse_worktree_list};
pub use remote::{REMOTE_HOME, Remote, quote, remote_path, scan_remote, split_key, workspace_key};
pub use scan::{CatalogError, scan_repo};
pub use ssh_config::{hosts, hosts_in, is_alias};
pub use store::{Repo, State, StoreError, load_state, save_state, state_path};

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub host: Option<String>,
    pub repo_path: PathBuf,
    pub path: PathBuf,
    pub kind: WorktreeKind,
    pub branch: Option<String>,
    pub head: Option<String>,
}

impl Workspace {
    pub fn key(&self) -> String {
        workspace_key(self.host.as_deref(), &self.path.to_string_lossy())
    }

    pub fn label(&self) -> String {
        if self.host.is_none() && self.kind == WorktreeKind::Folder && is_home_path(&self.path) {
            return HOME_LABEL.to_string();
        }
        if let Some(branch) = &self.branch {
            return branch.clone();
        }
        match self.path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_string(),
            None => match self.kind {
                WorktreeKind::Folder => "folder".to_string(),
                WorktreeKind::Git => "git".to_string(),
            },
        }
    }

    pub fn repo_label(&self) -> String {
        if self.host.is_none() && is_home_path(&self.repo_path) {
            return HOME_LABEL.to_string();
        }
        self.repo_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(self.repo_path.to_str().unwrap_or("repo"))
            .to_string()
    }
}

#[derive(Debug)]
pub struct Catalog {
    pub rows: Vec<Workspace>,
    pub errors: Vec<CatalogError>,
    pub unreachable: BTreeSet<String>,
}

pub enum Reach<'a> {
    Local,
    Cached(&'a Remote),
    Scan(&'a mut Remote),
}

pub fn catalog(state: &State, mut reach: Reach<'_>) -> Result<Catalog, CatalogError> {
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    let mut unreachable = BTreeSet::new();
    for repo in &state.repos {
        let records = match (&repo.host, &mut reach) {
            (None, _) => match scan_repo(&repo.path) {
                Ok(records) => records,
                Err(err @ CatalogError::NotADirectory(_)) => {
                    errors.push(err);
                    continue;
                }
                Err(err) => return Err(err),
            },
            (Some(_), Reach::Local) => continue,
            (Some(host), Reach::Cached(remote)) => match remote.recall(host, &repo.path) {
                Some(records) => records.to_vec(),
                None => continue,
            },
            (Some(host), Reach::Scan(remote)) => {
                let scanned = if unreachable.contains(host) {
                    None
                } else {
                    match remote.scan(host, &repo.path) {
                        Ok((records, skipped)) => {
                            errors.extend(skipped.into_iter().map(|path| {
                                CatalogError::ControlCharacter {
                                    host: host.clone(),
                                    path,
                                }
                            }));
                            Some(records)
                        }
                        Err(err) => {
                            if matches!(err, CatalogError::Unreachable { .. }) {
                                unreachable.insert(host.clone());
                            } else {
                                remote.forget(host, &repo.path);
                            }
                            errors.push(err);
                            None
                        }
                    }
                };
                let kept = || {
                    unreachable
                        .contains(host)
                        .then(|| remote.recall(host, &repo.path).map(<[_]>::to_vec))
                        .flatten()
                };
                match scanned.or_else(kept) {
                    Some(records) => records,
                    None => continue,
                }
            }
        };
        for record in records {
            rows.push(Workspace {
                host: repo.host.clone(),
                repo_path: repo.path.clone(),
                path: record.path,
                kind: record.kind,
                branch: record.branch,
                head: record.head,
            });
        }
    }
    rows.sort_by(|left, right| {
        left.host
            .cmp(&right.host)
            .then_with(|| left.repo_path.cmp(&right.repo_path))
            .then_with(|| left.path.cmp(&right.path))
    });
    Ok(Catalog {
        rows,
        errors,
        unreachable,
    })
}

pub fn add_repo(state: &mut State, path: &Path) -> Result<PathBuf, CatalogError> {
    let resolved = resolve_dir(path)?;
    if state
        .repos
        .iter()
        .any(|repo| repo.host.is_none() && repo.path == resolved)
    {
        return Ok(resolved);
    }
    state.repos.push(Repo {
        path: resolved.clone(),
        host: None,
    });
    Ok(resolved)
}

pub fn add_remote_repo(state: &mut State, host: &str, path: &str) -> Result<PathBuf, CatalogError> {
    if !is_alias(host) {
        return Err(CatalogError::InvalidHost(host.to_owned()));
    }
    let path = PathBuf::from(
        remote_path(path).ok_or_else(|| CatalogError::InvalidRemotePath(path.to_owned()))?,
    );
    if !state
        .repos
        .iter()
        .any(|repo| repo.host.as_deref() == Some(host) && repo.path == path)
    {
        state.repos.push(Repo {
            path: path.clone(),
            host: Some(host.to_owned()),
        });
    }
    Ok(path)
}

pub fn remove_repo(state: &mut State, path: &Path) -> bool {
    let resolved = resolve_vanished(path);
    let before = state.repos.len();
    state
        .repos
        .retain(|repo| repo.host.is_some() || repo.path != resolved);
    before != state.repos.len()
}

pub fn remove_remote_repo(state: &mut State, host: &str, path: &str) -> bool {
    let Some(path) = remote_path(path).map(PathBuf::from) else {
        return false;
    };
    let before = state.repos.len();
    state
        .repos
        .retain(|repo| repo.host.as_deref() != Some(host) || repo.path != path);
    before != state.repos.len()
}

pub fn cleanup(state: &mut State) -> Vec<PathBuf> {
    let mut cleaned = Vec::new();
    state.repos.retain(|repo| {
        if repo.host.is_some() || repo.path.is_dir() {
            return true;
        }
        cleaned.push(repo.path.clone());
        false
    });
    cleaned
}

fn resolve_vanished(path: &Path) -> PathBuf {
    if let Ok(resolved) = std::fs::canonicalize(path) {
        return resolved;
    }
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let (Some(parent), Some(name)) = (absolute.parent(), absolute.file_name()) else {
        return absolute;
    };
    match std::fs::canonicalize(parent) {
        Ok(parent) => parent.join(name),
        Err(_) => absolute,
    }
}

fn resolve_dir(path: &Path) -> Result<PathBuf, CatalogError> {
    let resolved = std::fs::canonicalize(path).map_err(|err| CatalogError::Io {
        path: path.to_path_buf(),
        source: err,
    })?;
    if !resolved.is_dir() {
        return Err(CatalogError::NotADirectory(resolved));
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_repo_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = State::default();
        add_repo(&mut state, dir.path()).unwrap();
        add_repo(&mut state, dir.path()).unwrap();
        assert_eq!(state.repos.len(), 1);
    }

    #[test]
    fn catalog_orders_rows_by_repo_then_path() {
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("beta");
        let second = root.path().join("alpha");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        let mut state = State::default();
        add_repo(&mut state, &first).unwrap();
        add_repo(&mut state, &second).unwrap();
        let found = catalog(&state, Reach::Local).unwrap();
        assert_eq!(found.rows[0].path, std::fs::canonicalize(&second).unwrap());
        assert_eq!(found.rows[1].path, std::fs::canonicalize(&first).unwrap());
        assert!(found.errors.is_empty());
    }

    #[test]
    fn cleanup_drops_only_vanished_paths() {
        let root = tempfile::tempdir().unwrap();
        let live = root.path().join("live");
        let gone = root.path().join("gone");
        std::fs::create_dir_all(&live).unwrap();
        std::fs::create_dir_all(&gone).unwrap();
        let mut state = State::default();
        add_repo(&mut state, &live).unwrap();
        add_repo(&mut state, &gone).unwrap();
        std::fs::remove_dir_all(&gone).unwrap();

        let cleaned = cleanup(&mut state);

        let live = std::fs::canonicalize(&live).unwrap();
        assert_eq!(
            state.repos,
            vec![Repo {
                path: live,
                host: None
            }]
        );
        assert_eq!(cleaned.len(), 1);
    }

    #[test]
    fn remove_repo_matches_a_vanished_path() {
        let root = tempfile::tempdir().unwrap();
        let gone = root.path().join("gone");
        std::fs::create_dir_all(&gone).unwrap();
        let mut state = State::default();
        add_repo(&mut state, &gone).unwrap();
        std::fs::remove_dir_all(&gone).unwrap();

        assert!(remove_repo(&mut state, &root.path().join(".").join("gone")));
        assert!(state.repos.is_empty());
    }

    #[test]
    fn catalog_does_not_inject_home() {
        let found = catalog(&State::default(), Reach::Local).unwrap();
        assert!(found.rows.is_empty());
    }

    #[test]
    fn catalog_skips_missing_repo() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("gone");
        let mut state = State::default();
        add_repo(&mut state, dir.path()).unwrap();
        state.repos.push(Repo {
            path: missing.clone(),
            host: None,
        });
        let found = catalog(&state, Reach::Local).unwrap();
        assert_eq!(found.rows.len(), 1);
        assert_eq!(
            found.rows[0].path,
            std::fs::canonicalize(dir.path()).unwrap()
        );
        assert!(matches!(
            &found.errors[..],
            [CatalogError::NotADirectory(path)] if path == &missing
        ));
    }

    fn fake_ssh(root: &Path) -> (PathBuf, PathBuf) {
        let flag = root.join("down");
        let script = root.join("ssh");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\nif [ -e '{}' ]; then echo 'ssh: connect to host fake: Host is down' >&2; exit 255; fi\nfor last; do :; done\nexec /bin/sh -c \"$last\"\n",
                flag.display()
            ),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        (script, flag)
    }

    fn git(cwd: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(["-c", "user.name=Combe", "-c", "user.email=combe@test"])
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
    fn remote_repo_lists_worktrees_over_ssh_and_keeps_them_while_unreachable() {
        let root = tempfile::tempdir().unwrap();
        let (ssh, flag) = fake_ssh(root.path());
        let repo = root.path().join("with space");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-b", "main"]);
        std::fs::write(repo.join("README"), "hi").unwrap();
        git(&repo, &["add", "README"]);
        git(&repo, &["commit", "-m", "init"]);
        let linked = root.path().join("feat");
        git(
            &repo,
            &["worktree", "add", linked.to_str().unwrap(), "-b", "feat"],
        );
        let folder = root.path().join("notes");
        std::fs::create_dir_all(&folder).unwrap();

        let mut state = State::default();
        add_remote_repo(&mut state, "fake", repo.to_str().unwrap()).unwrap();
        add_remote_repo(&mut state, "fake", folder.to_str().unwrap()).unwrap();
        add_remote_repo(&mut state, "fake", "/nonexistent/combe").unwrap();
        let mut remote = Remote::with_ssh(&ssh);

        let found = catalog(&state, Reach::Scan(&mut remote)).unwrap();
        assert!(found.unreachable.is_empty());
        assert!(
            found
                .rows
                .iter()
                .all(|row| row.host.as_deref() == Some("fake"))
        );
        let mut labels: Vec<String> = found.rows.iter().map(Workspace::label).collect();
        labels.sort();
        assert_eq!(labels, ["feat", "main", "notes"]);
        assert!(
            found
                .rows
                .iter()
                .all(|row| row.key().starts_with("ssh://fake/"))
        );
        assert!(matches!(
            &found.errors[..],
            [CatalogError::RemoteMissing { host, .. }] if host == "fake"
        ));
        assert!(catalog(&state, Reach::Local).unwrap().rows.is_empty());

        std::fs::write(&flag, "").unwrap();
        let offline = catalog(&state, Reach::Scan(&mut remote)).unwrap();
        assert_eq!(offline.unreachable.iter().collect::<Vec<_>>(), ["fake"]);
        assert_eq!(offline.rows, found.rows);
        assert_eq!(
            offline
                .errors
                .iter()
                .filter(|err| matches!(err, CatalogError::Unreachable { .. }))
                .count(),
            1
        );
        assert_eq!(
            catalog(&state, Reach::Cached(&remote)).unwrap().rows,
            found.rows
        );

        std::fs::remove_file(&flag).unwrap();
        std::fs::remove_dir_all(&folder).unwrap();
        let removed = catalog(&state, Reach::Scan(&mut remote)).unwrap();
        assert!(removed.unreachable.is_empty());
        assert!(!removed.rows.iter().any(|row| row.label() == "notes"));
        assert_eq!(removed.rows.len(), 2);
        assert_eq!(
            removed
                .errors
                .iter()
                .filter(|err| matches!(err, CatalogError::RemoteMissing { .. }))
                .count(),
            2
        );
        assert_eq!(
            catalog(&state, Reach::Cached(&remote)).unwrap().rows,
            removed.rows
        );
        std::fs::write(&flag, "").unwrap();
        let still_offline = catalog(&state, Reach::Scan(&mut remote)).unwrap();
        assert_eq!(still_offline.rows, removed.rows);

        let mut cold = Remote::with_ssh(&ssh);
        let first = catalog(&state, Reach::Scan(&mut cold)).unwrap();
        assert!(first.rows.is_empty());
        assert!(first.unreachable.contains("fake"));
    }

    #[test]
    fn remote_worktrees_with_control_characters_are_skipped() {
        let root = tempfile::tempdir().unwrap();
        let (ssh, _) = fake_ssh(root.path());
        let repo = root.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-b", "main"]);
        git(&repo, &["commit", "--allow-empty", "-m", "init"]);
        for (name, branch) in [("tab\tpath", "tab"), ("cr\rpath", "cr")] {
            let linked = root.path().join(name);
            git(
                &repo,
                &["worktree", "add", linked.to_str().unwrap(), "-b", branch],
            );
        }
        let mut state = State::default();
        add_remote_repo(&mut state, "fake", repo.to_str().unwrap()).unwrap();
        let mut remote = Remote::with_ssh(&ssh);

        let found = catalog(&state, Reach::Scan(&mut remote)).unwrap();

        let labels: Vec<String> = found.rows.iter().map(Workspace::label).collect();
        assert_eq!(labels, ["main"]);
        assert_eq!(
            found
                .errors
                .iter()
                .filter(|err| matches!(err, CatalogError::ControlCharacter { .. }))
                .count(),
            2
        );
        assert_eq!(
            catalog(&state, Reach::Cached(&remote)).unwrap().rows,
            found.rows
        );
    }

    #[test]
    fn remote_and_local_registrations_are_independent() {
        let dir = tempfile::tempdir().unwrap();
        let local = std::fs::canonicalize(dir.path()).unwrap();
        let mut state = State::default();
        add_repo(&mut state, &local).unwrap();
        let path = local.to_str().unwrap();
        add_remote_repo(&mut state, "xbp", path).unwrap();
        add_remote_repo(&mut state, "xbp", &format!("{path}/")).unwrap();
        assert_eq!(state.repos.len(), 2);
        assert!(add_remote_repo(&mut state, "-oProxyCommand=id", path).is_err());
        assert!(add_remote_repo(&mut state, "xbp", "relative").is_err());
        assert!(cleanup(&mut state).is_empty());
        assert!(remove_repo(&mut state, &local));
        assert_eq!(state.repos[0].host.as_deref(), Some("xbp"));
        assert!(!remove_remote_repo(&mut state, "other", path));
        assert!(remove_remote_repo(&mut state, "xbp", path));
        assert!(state.repos.is_empty());
    }
}
