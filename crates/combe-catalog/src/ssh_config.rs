use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn is_alias(alias: &str) -> bool {
    !alias.is_empty()
        && alias.len() <= 255
        && !alias.starts_with('-')
        && alias
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".-_".contains(character))
}

pub fn hosts_in(file: &Path, ssh_dir: &Path) -> Vec<String> {
    let mut hosts = Vec::new();
    let mut seen = HashSet::new();
    read_config(file, ssh_dir, 0, &mut hosts, &mut seen);
    hosts
}

pub fn hosts() -> Vec<String> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let ssh_dir = home.join(".ssh");
    hosts_in(&ssh_dir.join("config"), &ssh_dir)
}

fn read_config(
    file: &Path,
    ssh_dir: &Path,
    depth: usize,
    hosts: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    if depth >= 16 {
        return;
    }
    let Ok(contents) = fs::read_to_string(file) else {
        return;
    };
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let tokens = tokenize(line);
        let Some((keyword, arguments)) = directive(&tokens) else {
            continue;
        };
        if keyword.eq_ignore_ascii_case("Host") {
            for alias in arguments {
                if alias.contains(['*', '?', '!']) || !is_alias(alias) {
                    continue;
                }
                if seen.insert(alias.to_string()) {
                    hosts.push(alias.to_string());
                }
            }
        } else if keyword.eq_ignore_ascii_case("Include") {
            for pattern in arguments {
                for included in include_paths(pattern, ssh_dir) {
                    read_config(&included, ssh_dir, depth + 1, hosts, seen);
                }
            }
        }
    }
}

fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut quoted = false;
    let mut started = false;
    for character in line.chars() {
        match character {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            character if character.is_whitespace() && !quoted => {
                if started {
                    tokens.push(std::mem::take(&mut token));
                    started = false;
                }
            }
            character => {
                token.push(character);
                started = true;
            }
        }
    }
    if started {
        tokens.push(token);
    }
    tokens
}

fn directive(tokens: &[String]) -> Option<(&str, Vec<&str>)> {
    let first = tokens.first()?;
    let mut arguments = Vec::new();
    let keyword = if let Some((keyword, inline_argument)) = first.split_once('=') {
        if !inline_argument.is_empty() {
            arguments.push(inline_argument);
        }
        arguments.extend(tokens.iter().skip(1).map(String::as_str));
        keyword
    } else {
        arguments.extend(tokens.iter().skip(1).map(String::as_str));
        if arguments.first().is_some_and(|argument| *argument == "=") {
            arguments.remove(0);
        } else if let Some(argument) = arguments.first_mut()
            && let Some(value) = argument.strip_prefix('=')
        {
            *argument = value;
        }
        first
    };
    Some((keyword, arguments))
}

fn include_paths(pattern: &str, ssh_dir: &Path) -> Vec<PathBuf> {
    let path = if let Some(remainder) = pattern.strip_prefix("~/") {
        ssh_dir.parent().unwrap_or(ssh_dir).join(remainder)
    } else {
        let path = PathBuf::from(pattern);
        if path.is_absolute() {
            path
        } else {
            ssh_dir.join(path)
        }
    };
    let Some(final_component) = path.file_name() else {
        return vec![path];
    };
    let pattern_name = final_component.to_string_lossy();
    if !pattern_name.contains(['*', '?']) {
        return vec![path];
    }
    let Some(parent) = path.parent() else {
        return Vec::new();
    };
    if parent.components().any(|component| {
        let component = component.as_os_str().to_string_lossy();
        component.contains(['*', '?'])
    }) {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut matches = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter(|entry| {
            let name = entry.file_name();
            wildcard_matches(&pattern_name, &name.to_string_lossy())
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
    matches
}

fn wildcard_matches(pattern: &str, value: &str) -> bool {
    let pattern = pattern.chars().collect::<Vec<_>>();
    let value = value.chars().collect::<Vec<_>>();
    let mut previous = vec![false; value.len() + 1];
    previous[0] = true;
    for character in pattern {
        let mut current = vec![false; value.len() + 1];
        if character == '*' {
            current[0] = previous[0];
            for index in 1..=value.len() {
                current[index] = previous[index] || current[index - 1];
            }
        } else {
            for index in 1..=value.len() {
                current[index] =
                    previous[index - 1] && (character == '?' || character == value[index - 1]);
            }
        }
        previous = current;
    }
    previous[value.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(root: &Path, contents: &str) -> (PathBuf, PathBuf) {
        let ssh_dir = root.join(".ssh");
        fs::create_dir_all(&ssh_dir).unwrap();
        let file = ssh_dir.join("config");
        fs::write(&file, contents).unwrap();
        (file, ssh_dir)
    }

    #[test]
    fn reads_concrete_hosts_in_order_and_deduplicates() {
        let root = tempfile::tempdir().unwrap();
        let (file, ssh_dir) = config(root.path(), "Host alpha beta\nHost beta gamma\n");
        assert_eq!(hosts_in(&file, &ssh_dir), ["alpha", "beta", "gamma"]);
    }

    #[test]
    fn skips_wildcard_and_negated_hosts() {
        let root = tempfile::tempdir().unwrap();
        let (file, ssh_dir) = config(root.path(), "Host *\nHost *.corp\nHost !bad good\n");
        assert_eq!(hosts_in(&file, &ssh_dir), ["good"]);
    }

    #[test]
    fn ignores_match_directives() {
        let root = tempfile::tempdir().unwrap();
        let (file, ssh_dir) = config(root.path(), "Match host ignored\nHost included\n");
        assert_eq!(hosts_in(&file, &ssh_dir), ["included"]);
    }

    #[test]
    fn parses_case_insensitive_keyword_and_equals_separator() {
        let root = tempfile::tempdir().unwrap();
        let (file, ssh_dir) = config(root.path(), "hOsT=alpha\nhost = beta\n");
        assert_eq!(hosts_in(&file, &ssh_dir), ["alpha", "beta"]);
    }

    #[test]
    fn parses_quoted_argument_as_one_token() {
        let root = tempfile::tempdir().unwrap();
        let (file, ssh_dir) = config(root.path(), "Host \"host local\" visible\n");
        assert_eq!(hosts_in(&file, &ssh_dir), ["visible"]);
    }

    #[test]
    fn includes_paths_relative_to_ssh_directory() {
        let root = tempfile::tempdir().unwrap();
        let ssh_dir = root.path().join(".ssh");
        fs::create_dir_all(&ssh_dir).unwrap();
        fs::write(ssh_dir.join("extra"), "Host included\n").unwrap();
        let file = ssh_dir.join("config");
        fs::write(&file, "Host first\nInclude extra\nHost last\n").unwrap();
        assert_eq!(hosts_in(&file, &ssh_dir), ["first", "included", "last"]);
    }

    #[test]
    fn includes_final_component_glob_in_sorted_order() {
        let root = tempfile::tempdir().unwrap();
        let ssh_dir = root.path().join(".ssh");
        let include_dir = ssh_dir.join("conf.d");
        fs::create_dir_all(&include_dir).unwrap();
        fs::write(include_dir.join("z.conf"), "Host zulu\n").unwrap();
        fs::write(include_dir.join("a.conf"), "Host alpha\n").unwrap();
        let file = ssh_dir.join("config");
        fs::write(&file, "Include conf.d/*\n").unwrap();
        assert_eq!(hosts_in(&file, &ssh_dir), ["alpha", "zulu"]);
    }

    #[test]
    fn include_loop_terminates() {
        let root = tempfile::tempdir().unwrap();
        let ssh_dir = root.path().join(".ssh");
        fs::create_dir_all(&ssh_dir).unwrap();
        let config_file = ssh_dir.join("config");
        fs::write(&config_file, "Host first\nInclude config\nHost last\n").unwrap();
        assert_eq!(hosts_in(&config_file, &ssh_dir), ["first", "last"]);
    }

    #[test]
    fn missing_file_returns_empty() {
        let root = tempfile::tempdir().unwrap();
        assert!(hosts_in(&root.path().join("missing"), root.path()).is_empty());
    }

    #[test]
    fn validates_aliases() {
        for alias in ["-oProxyCommand=x", "a b", "a;b", ""] {
            assert!(!is_alias(alias), "{alias}");
        }
        for alias in ["xbp", "m5pro", "dev-tokener-ai", "host.local"] {
            assert!(is_alias(alias), "{alias}");
        }
    }
}
