#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeKind {
    Git,
    Folder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeRecord {
    pub path: std::path::PathBuf,
    pub kind: WorktreeKind,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub bare: bool,
}

pub fn parse_worktree_list(output: &str) -> Vec<WorktreeRecord> {
    let mut records = Vec::new();
    let mut current: Option<WorktreeRecord> = None;

    for raw in output.split('\n') {
        if raw.is_empty() {
            if let Some(record) = current.take() {
                records.push(record);
            }
            continue;
        }

        if let Some(path) = raw.strip_prefix("worktree ") {
            if let Some(record) = current.take() {
                records.push(record);
            }
            current = Some(WorktreeRecord {
                path: std::path::PathBuf::from(path),
                kind: WorktreeKind::Git,
                branch: None,
                head: None,
                bare: false,
            });
            continue;
        }

        let Some(record) = current.as_mut() else {
            continue;
        };

        if let Some(head) = raw.strip_prefix("HEAD ") {
            record.head = Some(head.to_string());
        } else if let Some(branch) = raw.strip_prefix("branch ") {
            record.branch = Some(branch.trim_start_matches("refs/heads/").to_string());
        } else if raw == "detached" || raw.starts_with("detached ") {
            record.branch = Some("(detached)".to_string());
        } else if raw == "bare" {
            record.bare = true;
        }
    }

    if let Some(record) = current {
        records.push(record);
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_main_and_linked_worktree() {
        let output = "\
worktree /tmp/repo
HEAD 0123456789abcdef0123456789abcdef01234567
branch refs/heads/main

worktree /tmp/repo-feat
HEAD fedcba9876543210fedcba9876543210fedcba98
branch refs/heads/feat
";
        let records = parse_worktree_list(output);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].path.as_os_str(), "/tmp/repo");
        assert_eq!(records[0].branch.as_deref(), Some("main"));
        assert_eq!(records[1].branch.as_deref(), Some("feat"));
        assert_eq!(records[1].kind, WorktreeKind::Git);
    }

    #[test]
    fn parses_detached_and_bare() {
        let output = "\
worktree /tmp/bare
HEAD 0123456789abcdef0123456789abcdef01234567
bare

worktree /tmp/detached
HEAD fedcba9876543210fedcba9876543210fedcba98
detached
";
        let records = parse_worktree_list(output);
        assert!(records[0].bare);
        assert_eq!(records[1].branch.as_deref(), Some("(detached)"));
    }
}
