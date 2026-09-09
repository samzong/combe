use crate::porcelain::{WorktreeKind, WorktreeRecord, parse_worktree_list};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("not a directory: {0}")]
    NotADirectory(PathBuf),
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("git failed in {path}: {stderr}")]
    Git { path: PathBuf, stderr: String },
}

pub fn scan_repo(path: &Path) -> Result<Vec<WorktreeRecord>, CatalogError> {
    if !path.is_dir() {
        return Err(CatalogError::NotADirectory(path.to_path_buf()));
    }
    if !is_git_dir(path)? {
        return Ok(vec![WorktreeRecord {
            path: canonicalize(path),
            kind: WorktreeKind::Folder,
            branch: None,
            head: None,
            bare: false,
            prunable: false,
        }]);
    }

    let output = Command::new("git")
        .args(["-C", &path_arg(path), "worktree", "list", "--porcelain"])
        .output()
        .map_err(|source| CatalogError::Io {
            path: path.to_path_buf(),
            source,
        })?;

    if !output.status.success() {
        return Err(CatalogError::Git {
            path: path.to_path_buf(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    let mut records = parse_worktree_list(&String::from_utf8_lossy(&output.stdout));
    for record in &mut records {
        record.path = canonicalize(&record.path);
    }
    Ok(records)
}

fn canonicalize(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn is_git_dir(path: &Path) -> Result<bool, CatalogError> {
    let output = Command::new("git")
        .args(["-C", &path_arg(path), "rev-parse", "--git-dir"])
        .output()
        .map_err(|source| CatalogError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(output.status.success())
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn folder_path_is_a_single_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let rows = scan_repo(dir.path()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, WorktreeKind::Folder);
        assert_eq!(rows[0].path, std::fs::canonicalize(dir.path()).unwrap());
    }

    #[test]
    fn git_repo_lists_main_and_linked_worktree() {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("repo");
        let linked = root.path().join("feat");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-b", "main"]);
        std::fs::write(repo.join("README"), "hi").unwrap();
        git(
            &repo,
            &[
                "-c",
                "user.name=Combe",
                "-c",
                "user.email=combe@test",
                "add",
                "README",
            ],
        );
        git(
            &repo,
            &[
                "-c",
                "user.name=Combe",
                "-c",
                "user.email=combe@test",
                "commit",
                "-m",
                "init",
            ],
        );
        git(
            &repo,
            &["worktree", "add", linked.to_str().unwrap(), "-b", "feat"],
        );

        let rows = scan_repo(&repo).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.kind == WorktreeKind::Git));
        assert!(rows.iter().any(|row| row.branch.as_deref() == Some("main")));
        assert!(rows.iter().any(|row| row.branch.as_deref() == Some("feat")));
        let linked = std::fs::canonicalize(&linked).unwrap();
        assert!(
            rows.iter()
                .any(|row| { std::fs::canonicalize(&row.path).ok().as_ref() == Some(&linked) })
        );
    }

    fn git(cwd: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
