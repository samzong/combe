#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeKind {
    Git,
    Folder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorktreeRecord {
    pub(crate) path: std::path::PathBuf,
    pub(crate) kind: WorktreeKind,
    pub(crate) branch: Option<String>,
    pub(crate) prunable: bool,
}

pub(crate) fn parse_worktree_list(output: &str) -> Vec<WorktreeRecord> {
    let mut records = Vec::new();
    let mut current: Option<WorktreeRecord> = None;

    for raw in output.split('\n') {
        if raw.is_empty() {
            push(&mut records, current.take());
            continue;
        }

        if let Some(path) = raw.strip_prefix("worktree ") {
            push(&mut records, current.take());
            current = Some(WorktreeRecord {
                path: std::path::PathBuf::from(path),
                kind: WorktreeKind::Git,
                branch: None,
                prunable: false,
            });
            continue;
        }

        let Some(record) = current.as_mut() else {
            continue;
        };

        if let Some(branch) = raw.strip_prefix("branch ") {
            record.branch = Some(branch.trim_start_matches("refs/heads/").to_string());
        } else if raw == "detached" || raw.starts_with("detached ") {
            record.branch = Some("(detached)".to_string());
        } else if raw == "prunable" || raw.starts_with("prunable ") {
            record.prunable = true;
        }
    }

    push(&mut records, current);
    records
}

fn push(records: &mut Vec<WorktreeRecord>, record: Option<WorktreeRecord>) {
    if let Some(record) = record.filter(|record| !record.prunable) {
        records.push(record);
    }
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
    fn retains_bare_and_detached_worktrees_and_skips_prunable() {
        let output = "\
worktree /tmp/bare
HEAD 0123456789abcdef0123456789abcdef01234567
bare

worktree /tmp/detached
HEAD fedcba9876543210fedcba9876543210fedcba98
detached

worktree /tmp/deleted
HEAD fedcba9876543210fedcba9876543210fedcba98
branch refs/heads/gone
prunable gitdir file points to non-existent location
";
        let records = parse_worktree_list(output);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].path.as_os_str(), "/tmp/bare");
        assert_eq!(records[1].branch.as_deref(), Some("(detached)"));
    }
}
