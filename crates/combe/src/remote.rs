use combe_catalog::{REMOTE_HOME, is_alias, quote};

const RESOURCES: &str = "/Applications/Combe.app/Contents/Resources";
const FEATURES: &str = "sudo,title";

pub(crate) fn launchable(host: Option<&str>, path: &str) -> bool {
    host.is_none_or(|host| command(host, path).is_some())
}

pub(crate) fn command(host: &str, path: &str) -> Option<String> {
    if !is_alias(host) || path.chars().any(|c| c.is_control()) {
        return None;
    }
    if path != REMOTE_HOME && !path.starts_with('/') {
        return None;
    }
    let cd = if path == REMOTE_HOME {
        "cd".to_owned()
    } else {
        format!("cd {}", quote(path))
    };
    let remote = format!(
        "{cd} && R={RESOURCES} && if [ -d \"$R/ghostty/shell-integration/zsh\" ]; then exec env TERM=xterm-ghostty TERMINFO=\"$R/terminfo\" GHOSTTY_SHELL_FEATURES={FEATURES} ZDOTDIR=\"$R/ghostty/shell-integration/zsh\" GHOSTTY_ZSH_ZDOTDIR=\"${{ZDOTDIR:-$HOME}}\" zsh -l; else exec env TERM=xterm-256color zsh -l; fi"
    );
    Some(format!(
        "ssh -t -o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=15 -o ServerAliveCountMax=3 -- {host} {}",
        quote(&remote)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn words(line: &str) -> Vec<String> {
        let rest = line.strip_prefix("ssh ").unwrap();
        let output = Command::new("/bin/sh")
            .arg("-c")
            .arg(format!(
                "set -- {rest}; for word; do printf '%s\\0' \"$word\"; done"
            ))
            .output()
            .unwrap();
        String::from_utf8(output.stdout)
            .unwrap()
            .split('\0')
            .filter(|word| !word.is_empty())
            .map(str::to_owned)
            .collect()
    }

    fn remote_script(line: &str) -> String {
        let words = words(line);
        assert_eq!(
            &words[..10],
            [
                "-t",
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=10",
                "-o",
                "ServerAliveInterval=15",
                "-o",
                "ServerAliveCountMax=3",
                "--",
            ]
        );
        assert_eq!(words.len(), 12, "{words:?}");
        words[11].clone()
    }

    fn cd_target(script: &str) -> String {
        let probe = script.replacen(" && R=", " && pwd -P >&3; exit 0; R=", 1);
        let output = Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("exec 3>&1; {probe}"))
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    #[test]
    fn host_and_remote_script_stay_single_words() {
        let line = command("xbp", "/Users/x/git/combe").unwrap();
        assert!(line.starts_with("ssh -t "));
        let words = words(&line);
        assert_eq!(words[10], "xbp");
        assert!(remote_script(&line).starts_with("cd /Users/x/git/combe && "));
    }

    #[test]
    fn hostile_paths_reach_cd_as_one_literal_argument() {
        let root = std::env::temp_dir().join(format!("combe-remote-{}", std::process::id()));
        for name in ["a b", "it's", "$(touch pwned)", "`id`;x", "semi;colon", "*"] {
            let dir = root.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            let dir = std::fs::canonicalize(&dir).unwrap();
            let line = command("xbp", dir.to_str().unwrap()).unwrap();
            assert_eq!(
                cd_target(&remote_script(&line)),
                dir.to_str().unwrap(),
                "{name}"
            );
        }
        assert!(!root.join("pwned").exists());
        let _ = std::fs::remove_dir_all(&root);
        assert!(!std::path::Path::new("pwned").exists());
    }

    #[test]
    fn remote_home_uses_a_bare_cd() {
        let line = command("xbp", REMOTE_HOME).unwrap();
        assert!(remote_script(&line).starts_with("cd && "));
    }

    #[test]
    fn rejects_hosts_and_paths_that_could_inject() {
        for host in ["-oProxyCommand=id", "a b", "a;b", "", "$(id)", "x\ny"] {
            assert!(command(host, "/tmp").is_none(), "{host:?}");
        }
        for path in ["/tmp/a\nb", "relative", "~/git", "/tmp/\u{1b}[2J"] {
            assert!(command("xbp", path).is_none(), "{path:?}");
        }
    }

    #[test]
    fn control_characters_make_a_remote_path_unlaunchable() {
        for path in ["/srv/a\tb", "/srv/a\rb", "/srv/a\u{7f}b"] {
            assert!(command("xbp", path).is_none(), "{path:?}");
            assert!(!launchable(Some("xbp"), path), "{path:?}");
        }
        assert!(launchable(None, "/srv/a\tb"));
        assert!(launchable(Some("xbp"), "/srv/a b"));
    }

    #[test]
    fn integration_falls_back_to_plain_zsh() {
        let script = remote_script(&command("xbp", "/tmp").unwrap());
        assert!(script.contains("ZDOTDIR=\"$R/ghostty/shell-integration/zsh\""));
        assert!(script.contains("TERMINFO=\"$R/terminfo\""));
        assert!(script.ends_with("else exec env TERM=xterm-256color zsh -l; fi"));
    }
}
