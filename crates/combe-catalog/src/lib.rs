mod porcelain;
mod scan;
mod store;

pub use porcelain::{WorktreeKind, WorktreeRecord, parse_worktree_list};
pub use scan::{CatalogError, scan_repo};
pub use store::{Repo, State, StoreError, load_state, save_state, state_path};

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub repo_path: PathBuf,
    pub path: PathBuf,
    pub kind: WorktreeKind,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub pinned: bool,
}

impl Workspace {
    pub fn label(&self) -> String {
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
}

pub fn catalog(state: &State) -> Result<Catalog, CatalogError> {
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    for repo in &state.repos {
        let records = match scan_repo(&repo.path) {
            Ok(records) => records,
            Err(err @ CatalogError::NotADirectory(_)) => {
                errors.push(err);
                continue;
            }
            Err(err) => return Err(err),
        };
        for record in records {
            let pinned = state.pinned.iter().any(|p| paths_equal(p, &record.path));
            rows.push(Workspace {
                repo_path: repo.path.clone(),
                path: record.path,
                kind: record.kind,
                branch: record.branch,
                head: record.head,
                pinned,
            });
        }
    }
    rows.sort_by(|left, right| {
        right
            .pinned
            .cmp(&left.pinned)
            .then_with(|| left.repo_path.cmp(&right.repo_path))
            .then_with(|| left.path.cmp(&right.path))
    });
    Ok(Catalog { rows, errors })
}

pub fn add_repo(state: &mut State, path: &Path) -> Result<PathBuf, CatalogError> {
    let resolved = resolve_dir(path)?;
    if state
        .repos
        .iter()
        .any(|repo| paths_equal(&repo.path, &resolved))
    {
        return Ok(resolved);
    }
    state.repos.push(Repo {
        path: resolved.clone(),
    });
    Ok(resolved)
}

pub fn remove_repo(state: &mut State, path: &Path) -> Result<bool, CatalogError> {
    let resolved = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let before = state.repos.len();
    state
        .repos
        .retain(|repo| !paths_equal(&repo.path, &resolved));
    if before == state.repos.len() {
        return Ok(false);
    }
    if let Ok(rows) = scan_repo(&resolved) {
        state
            .pinned
            .retain(|pinned| !rows.iter().any(|row| paths_equal(&row.path, pinned)));
    }
    Ok(true)
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Cleaned {
    pub repos: Vec<PathBuf>,
    pub pinned: Vec<PathBuf>,
}

impl Cleaned {
    pub fn is_empty(&self) -> bool {
        self.repos.is_empty() && self.pinned.is_empty()
    }
}

pub fn cleanup(state: &mut State) -> Cleaned {
    let mut cleaned = Cleaned::default();
    state.repos.retain(|repo| {
        if repo.path.is_dir() {
            return true;
        }
        cleaned.repos.push(repo.path.clone());
        false
    });
    state.pinned.retain(|path| {
        if path.is_dir() {
            return true;
        }
        cleaned.pinned.push(path.clone());
        false
    });
    cleaned
}

pub fn toggle_pin(state: &mut State, path: &Path) -> bool {
    let resolved = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if let Some(index) = state.pinned.iter().position(|p| paths_equal(p, &resolved)) {
        state.pinned.remove(index);
        false
    } else {
        state.pinned.push(resolved);
        true
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

fn paths_equal(left: &Path, right: &Path) -> bool {
    left == right
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
    fn toggle_pin_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = std::fs::canonicalize(dir.path()).unwrap();
        let mut state = State::default();
        assert!(toggle_pin(&mut state, &path));
        assert_eq!(state.pinned, vec![path.clone()]);
        assert!(!toggle_pin(&mut state, &path));
        assert!(state.pinned.is_empty());
    }

    #[test]
    fn catalog_puts_pins_first() {
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("alpha");
        let second = root.path().join("beta");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        let mut state = State::default();
        add_repo(&mut state, &first).unwrap();
        add_repo(&mut state, &second).unwrap();
        toggle_pin(&mut state, &second);
        let found = catalog(&state).unwrap();
        assert_eq!(found.rows[0].path, std::fs::canonicalize(&second).unwrap());
        assert!(found.rows[0].pinned);
        assert!(!found.rows[1].pinned);
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
        toggle_pin(&mut state, &live);
        toggle_pin(&mut state, &gone);
        std::fs::remove_dir_all(&gone).unwrap();

        let cleaned = cleanup(&mut state);

        let live = std::fs::canonicalize(&live).unwrap();
        assert_eq!(state.repos, vec![Repo { path: live.clone() }]);
        assert_eq!(state.pinned, vec![live]);
        assert_eq!(cleaned.repos.len(), 1);
        assert_eq!(cleaned.pinned.len(), 1);
    }

    #[test]
    fn catalog_skips_missing_repo() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("gone");
        let mut state = State::default();
        add_repo(&mut state, dir.path()).unwrap();
        state.repos.push(Repo {
            path: missing.clone(),
        });
        let found = catalog(&state).unwrap();
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
}
