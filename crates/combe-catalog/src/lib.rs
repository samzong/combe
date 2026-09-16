mod home;
mod porcelain;
mod scan;
mod store;

pub use home::{HOME_LABEL, home_dir, home_workspace, is_home_path, should_inject_home};
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
}

impl Workspace {
    pub fn label(&self) -> String {
        if self.kind == WorktreeKind::Folder && is_home_path(&self.path) {
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
        if is_home_path(&self.repo_path) {
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
            rows.push(Workspace {
                repo_path: repo.path.clone(),
                path: record.path,
                kind: record.kind,
                branch: record.branch,
                head: record.head,
            });
        }
    }
    rows.sort_by(|left, right| {
        left.repo_path
            .cmp(&right.repo_path)
            .then_with(|| left.path.cmp(&right.path))
    });
    Ok(Catalog { rows, errors })
}

pub fn add_repo(state: &mut State, path: &Path) -> Result<PathBuf, CatalogError> {
    let resolved = resolve_dir(path)?;
    if state.repos.iter().any(|repo| repo.path == resolved) {
        return Ok(resolved);
    }
    state.repos.push(Repo {
        path: resolved.clone(),
    });
    Ok(resolved)
}

pub fn remove_repo(state: &mut State, path: &Path) -> bool {
    let resolved = resolve_vanished(path);
    let before = state.repos.len();
    state.repos.retain(|repo| repo.path != resolved);
    before != state.repos.len()
}

pub fn cleanup(state: &mut State) -> Vec<PathBuf> {
    let mut cleaned = Vec::new();
    state.repos.retain(|repo| {
        if repo.path.is_dir() {
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
        let found = catalog(&state).unwrap();
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
        assert_eq!(state.repos, vec![Repo { path: live }]);
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
        let found = catalog(&State::default()).unwrap();
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
