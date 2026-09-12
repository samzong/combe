use std::path::{Path, PathBuf};

use combe_catalog::{
    HOME_LABEL, State, Workspace, add_repo, catalog, home_dir, is_home_path, load_state,
    save_state, should_inject_home, state_path,
};

pub struct Row {
    pub label: String,
    pub path: PathBuf,
}

pub struct Repo {
    pub path: PathBuf,
    pub name: String,
    pub rows: Vec<Row>,
    pub has_heading: bool,
}

pub fn repos() -> Vec<Repo> {
    let Some(state) = read_state() else {
        return home_catalog();
    };
    let found = match catalog(&state) {
        Ok(found) => found,
        Err(err) => {
            eprintln!("combe: {err}");
            return home_catalog();
        }
    };
    for err in &found.errors {
        eprintln!("combe: {err}");
    }

    let mut repos: Vec<Repo> = Vec::new();
    for repo in &state.repos {
        let rows: Vec<Row> = found
            .rows
            .iter()
            .filter(|row| row.repo_path == repo.path)
            .map(|row| Row {
                label: row.label(),
                path: row.path.clone(),
            })
            .collect();
        if rows.is_empty() {
            continue;
        }
        repos.push(Repo {
            path: repo.path.clone(),
            name: repo_name(&repo.path),
            rows,
            has_heading: true,
        });
    }
    with_home(repos, &found.rows)
}

pub fn rows() -> Vec<Workspace> {
    let Some(state) = read_state() else {
        return Vec::new();
    };
    match catalog(&state) {
        Ok(found) => found.rows,
        Err(err) => {
            eprintln!("combe: {err}");
            Vec::new()
        }
    }
}

fn home_catalog() -> Vec<Repo> {
    match home_dir() {
        Some(home) => vec![synthetic_home(home)],
        None => Vec::new(),
    }
}

fn with_home(mut repos: Vec<Repo>, rows: &[Workspace]) -> Vec<Repo> {
    let Some(home) = home_dir() else {
        return repos;
    };
    if should_inject_home(rows, &home) {
        repos.insert(0, synthetic_home(home));
    }
    repos
}

fn synthetic_home(home: PathBuf) -> Repo {
    Repo {
        path: home.clone(),
        name: HOME_LABEL.to_string(),
        rows: vec![Row {
            label: HOME_LABEL.to_string(),
            path: home,
        }],
        has_heading: false,
    }
}

pub fn add(paths: &[PathBuf]) {
    let Some(file) = state_path() else {
        eprintln!("combe: cannot resolve the application support directory");
        return;
    };
    let Some(mut state) = read_state() else {
        return;
    };
    for path in paths {
        if let Err(err) = add_repo(&mut state, path) {
            eprintln!("combe: {err}");
            return;
        }
    }
    if let Err(err) = save_state(&file, &state) {
        eprintln!("combe: {err}");
    }
}

fn read_state() -> Option<State> {
    let Some(file) = state_path() else {
        eprintln!("combe: cannot resolve the application support directory");
        return None;
    };
    match load_state(&file) {
        Ok(state) => Some(state),
        Err(err) => {
            eprintln!("combe: {err}");
            None
        }
    }
}

fn repo_name(path: &Path) -> String {
    if is_home_path(path) {
        return HOME_LABEL.to_string();
    }
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_else(|| path.to_str().unwrap_or("repo"))
        .to_string()
}
