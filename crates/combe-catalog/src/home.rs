use crate::{Workspace, WorktreeKind};
use std::path::{Path, PathBuf};

pub const HOME_LABEL: &str = "~";

pub fn home_dir() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    if !home.is_dir() {
        return None;
    }
    Some(std::fs::canonicalize(&home).unwrap_or(home))
}

pub fn home_workspace(home: PathBuf) -> Workspace {
    Workspace {
        repo_path: home.clone(),
        path: home,
        kind: WorktreeKind::Folder,
        branch: None,
        head: None,
    }
}

pub fn should_inject_home(rows: &[Workspace], home: &Path) -> bool {
    !rows.iter().any(|row| row.path == home)
}

pub fn is_home_path(path: &Path) -> bool {
    home_dir().is_some_and(|home| home == path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_workspace_is_folder_without_branch() {
        let home = PathBuf::from("/Users/tester");
        let workspace = home_workspace(home.clone());
        assert_eq!(workspace.kind, WorktreeKind::Folder);
        assert_eq!(workspace.path, home);
        assert_eq!(workspace.repo_path, home);
        assert_eq!(workspace.branch, None);
        assert_eq!(workspace.head, None);
    }

    #[test]
    fn injects_home_when_absent() {
        let home = PathBuf::from("/Users/tester");
        assert!(should_inject_home(&[], &home));
        assert!(should_inject_home(
            &[home_workspace(PathBuf::from("/tmp/other"))],
            &home
        ));
    }

    #[test]
    fn skips_home_when_folder_row_owns_path() {
        let home = PathBuf::from("/Users/tester");
        assert!(!should_inject_home(&[home_workspace(home.clone())], &home));
    }

    #[test]
    fn skips_home_when_git_row_owns_path() {
        let home = PathBuf::from("/Users/tester");
        let rows = vec![Workspace {
            repo_path: home.clone(),
            path: home.clone(),
            kind: WorktreeKind::Git,
            branch: Some("main".into()),
            head: None,
        }];
        assert!(!should_inject_home(&rows, &home));
    }

    #[test]
    fn folder_at_home_uses_tilde_label() {
        let Some(home) = home_dir() else {
            return;
        };
        assert_eq!(home_workspace(home).label(), HOME_LABEL);
    }
}
