use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use combe_catalog::{
    State, add_repo, catalog, cleanup, home_dir, load_state, remove_repo, save_state, state_path,
};
use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSString, NSURL, NSURLComponents};

use crate::entry::HOP_SCHEME;
use crate::habits;

const USAGE: &str = "\
combe — a worktree-aware terminal

Usage:
  combe <path>              Open a tab on that directory
  combe list                List registered repos and their worktrees
  combe add <path>...       Register repos
  combe remove <path>...    Unregister repos
  combe cleanup             Drop registered paths missing from disk
  combe help                Show this help
";

pub fn run() -> Option<ExitCode> {
    let mut args: Vec<String> = Vec::new();
    for arg in std::env::args_os().skip(1) {
        let Ok(arg) = arg.into_string() else {
            eprintln!("combe: argument is not valid UTF-8");
            return Some(ExitCode::FAILURE);
        };
        args.push(arg);
    }
    let Some(command) = args.first().map(String::as_str) else {
        if launched_from_bundle() {
            return None;
        }
        print!("{USAGE}");
        return Some(ExitCode::SUCCESS);
    };
    let rest = &args[1..];
    match command {
        "list" => Some(no_args("list", rest).unwrap_or_else(list)),
        "add" => Some(add(rest)),
        "remove" => Some(remove(rest)),
        "cleanup" => Some(no_args("cleanup", rest).unwrap_or_else(clean)),
        "help" | "-h" | "--help" => {
            print!("{USAGE}");
            Some(ExitCode::SUCCESS)
        }
        other => Some(hop(other, rest)),
    }
}

fn hop(target: &str, rest: &[String]) -> ExitCode {
    if !rest.is_empty() {
        eprintln!("combe: expected one path");
        return ExitCode::from(2);
    }
    let Ok(resolved) = std::fs::canonicalize(expand(target)) else {
        if looks_like_path(target) {
            eprintln!("combe: no such path: {target}");
            return ExitCode::FAILURE;
        }
        eprintln!("combe: '{target}' is not a combe command. See `combe help`.");
        return ExitCode::from(2);
    };
    if !resolved.is_dir() {
        eprintln!("combe: not a directory: {target}");
        return ExitCode::FAILURE;
    }
    if read_state().is_none() {
        return ExitCode::FAILURE;
    }
    if inside() {
        eprintln!("combe: already inside Combe");
        return ExitCode::SUCCESS;
    }
    let Some(url) = hop_url(&resolved) else {
        eprintln!("combe: cannot build a URL for {}", resolved.display());
        return ExitCode::FAILURE;
    };
    if NSWorkspace::sharedWorkspace().openURL(&url) {
        return ExitCode::SUCCESS;
    }
    eprintln!("combe: cannot reach Combe.app — install it with `make install`");
    ExitCode::FAILURE
}

fn no_args(command: &str, rest: &[String]) -> Option<ExitCode> {
    if rest.is_empty() {
        return None;
    }
    eprintln!("combe: {command} takes no arguments");
    Some(ExitCode::from(2))
}

fn hop_url(dir: &Path) -> Option<Retained<NSURL>> {
    let components = NSURLComponents::init(NSURLComponents::alloc());
    components.setScheme(Some(&NSString::from_str(HOP_SCHEME)));
    components.setPath(Some(&NSString::from_str(dir.to_str()?)));
    components.URL()
}

fn expand(target: &str) -> PathBuf {
    let Some(rest) = target.strip_prefix('~') else {
        return PathBuf::from(target);
    };
    let Some(home) = home_dir() else {
        return PathBuf::from(target);
    };
    match rest.strip_prefix('/') {
        Some(rest) => home.join(rest),
        None if rest.is_empty() => home,
        None => PathBuf::from(target),
    }
}

fn inside() -> bool {
    std::env::var(habits::NEST_ENV).as_deref() == Ok("1")
}

fn looks_like_path(target: &str) -> bool {
    target.contains('/') || target.starts_with('.') || target.starts_with('~')
}

fn launched_from_bundle() -> bool {
    let Some(exe) = std::env::args_os().next().map(PathBuf::from) else {
        return false;
    };
    exe.parent().and_then(Path::parent).is_some_and(|contents| {
        contents.file_name() == Some(OsStr::new("Contents"))
            && contents.join("Info.plist").is_file()
    })
}

fn list() -> ExitCode {
    let Some(state) = read_state() else {
        return ExitCode::FAILURE;
    };
    if state.repos.is_empty() {
        println!("no repos registered — combe add <path>");
        return ExitCode::SUCCESS;
    }
    let found = match catalog(&state) {
        Ok(found) => found,
        Err(err) => {
            eprintln!("combe: {err}");
            return ExitCode::FAILURE;
        }
    };
    for repo in &state.repos {
        let rows: Vec<_> = found
            .rows
            .iter()
            .filter(|row| row.repo_path == repo.path)
            .collect();
        if rows.is_empty() {
            println!("{}  (missing)", repo.path.display());
            continue;
        }
        println!("{}", repo.path.display());
        for row in rows {
            println!("  {:<28} {}", row.label(), row.path.display());
        }
    }
    ExitCode::SUCCESS
}

fn add(paths: &[String]) -> ExitCode {
    if paths.is_empty() {
        eprintln!("combe: add needs at least one path");
        return ExitCode::from(2);
    }
    edit(|state| {
        let mut failed = false;
        for path in paths {
            match add_repo(state, Path::new(path)) {
                Ok(resolved) => println!("added {}", resolved.display()),
                Err(err) => {
                    eprintln!("combe: {err}");
                    failed = true;
                }
            }
        }
        !failed
    })
}

fn remove(paths: &[String]) -> ExitCode {
    if paths.is_empty() {
        eprintln!("combe: remove needs at least one path");
        return ExitCode::from(2);
    }
    edit(|state| {
        let mut failed = false;
        for path in paths {
            if remove_repo(state, Path::new(path)) {
                println!("removed {path}");
            } else {
                eprintln!("combe: not registered: {path}");
                failed = true;
            }
        }
        !failed
    })
}

fn clean() -> ExitCode {
    edit(|state| {
        let cleaned = cleanup(state);
        if cleaned.is_empty() {
            println!("nothing to clean");
            return true;
        }
        for path in &cleaned {
            println!("dropped repo {}", path.display());
        }
        true
    })
}

fn edit(apply: impl FnOnce(&mut State) -> bool) -> ExitCode {
    let (Some(file), Some(mut state)) = (state_path(), read_state()) else {
        return ExitCode::FAILURE;
    };
    let before = state.clone();
    let ok = apply(&mut state);
    if state != before
        && let Err(err) = save_state(&file, &state)
    {
        eprintln!("combe: {err}");
        return ExitCode::FAILURE;
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn read_state() -> Option<State> {
    let file = state_path()?;
    match load_state(&file) {
        Ok(state) => Some(state),
        Err(err) => {
            eprintln!("combe: {err}");
            None
        }
    }
}
