use crate::Workspace;
use std::path::{Path, PathBuf};

pub const HOME_LABEL: &str = "home";

pub fn home_dir() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    if !home.is_dir() {
        return None;
    }
    Some(std::fs::canonicalize(&home).unwrap_or(home))
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
    use crate::WorktreeKind;

    fn folder(path: PathBuf) -> Workspace {
        Workspace {
            repo_path: path.clone(),
            path,
            kind: WorktreeKind::Folder,
            branch: None,
        }
    }

    #[test]
    fn injects_home_when_absent() {
        let home = PathBuf::from("/Users/tester");
        assert!(should_inject_home(&[], &home));
        assert!(should_inject_home(
            &[folder(PathBuf::from("/tmp/other"))],
            &home
        ));
    }

    #[test]
    fn skips_home_when_folder_row_owns_path() {
        let home = PathBuf::from("/Users/tester");
        assert!(!should_inject_home(&[folder(home.clone())], &home));
    }

    #[test]
    fn skips_home_when_git_row_owns_path() {
        let home = PathBuf::from("/Users/tester");
        let rows = vec![Workspace {
            repo_path: home.clone(),
            path: home.clone(),
            kind: WorktreeKind::Git,
            branch: Some("main".into()),
        }];
        assert!(!should_inject_home(&rows, &home));
    }

    #[test]
    fn folder_at_home_uses_home_label() {
        let home = home_dir().expect("Home directory must exist to verify its workspace label");
        assert_eq!(folder(home).label(), HOME_LABEL);
    }
}
