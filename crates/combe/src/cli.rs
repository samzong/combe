use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use combe_catalog::{
    Catalog, State, add_repo, catalog, cleanup, home_dir, load_state, remove_repo, save_state,
    state_path,
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
  combe <path>              Raise Combe and open a tab on that directory
  combe list                Show registered repos and their worktrees
  combe add <path>...       Register repos
  combe remove <path>...    Unregister repos
  combe cleanup             Drop registered paths that no longer exist on disk
  combe help                Show this help

`combe .` opens the registered worktree or folder workspace that owns the
directory. An unregistered directory opens as a tab of the Home workspace
starting there; nothing is registered. A hop from a shell already inside
Combe does not raise the app.

The window also opens from the Dock, Finder, or `open -a Combe`.
";

pub fn run() -> Option<ExitCode> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first().map(String::as_str) else {
        if launched_from_bundle() {
            if inside() {
                eprintln!("combe: already inside Combe");
                return Some(ExitCode::FAILURE);
            }
            return None;
        }
        print_usage();
        return Some(ExitCode::SUCCESS);
    };
    let rest = &args[1..];
    match command {
        "list" => Some(list()),
        "add" => Some(add(rest)),
        "remove" => Some(remove(rest)),
        "cleanup" => Some(clean()),
        "help" | "-h" | "--help" => {
            print_usage();
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
    let expanded = expand(target);
    let Ok(resolved) = std::fs::canonicalize(&expanded) else {
        if looks_like_path(target) {
            eprintln!("combe: no such path: {target}");
            return ExitCode::FAILURE;
        }
        eprintln!("combe: unknown command '{target}'");
        print_usage();
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
    match hop_url(&resolved) {
        Some(url) if NSWorkspace::sharedWorkspace().openURL(&url) => ExitCode::SUCCESS,
        _ => {
            eprintln!("combe: cannot reach Combe.app — install it with `make install`");
            ExitCode::FAILURE
        }
    }
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

fn print_usage() {
    print!("{USAGE}");
    if let Some(path) = state_path() {
        println!("\nState: {}", path.display());
    }
}

fn list() -> ExitCode {
    let Some(state) = read_state() else {
        return ExitCode::FAILURE;
    };
    if state.repos.is_empty() {
        println!("no repos registered — combe add <path>");
        return ExitCode::SUCCESS;
    }
    let found: Catalog = match catalog(&state) {
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
            match remove_repo(state, Path::new(path)) {
                Ok(true) => println!("removed {path}"),
                Ok(false) => {
                    eprintln!("combe: not registered: {path}");
                    failed = true;
                }
                Err(err) => {
                    eprintln!("combe: {err}");
                    failed = true;
                }
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
        for path in &cleaned.repos {
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
    let file: PathBuf = state_path()?;
    match load_state(&file) {
        Ok(state) => Some(state),
        Err(err) => {
            eprintln!("combe: {err}");
            None
        }
    }
}
