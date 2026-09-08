use std::path::{Path, PathBuf};

use combe_catalog::{State, add_repo, catalog, load_state, save_state, state_path};

pub struct Row {
    pub label: String,
    pub path: PathBuf,
    pub pinned: bool,
}

pub struct Repo {
    pub path: PathBuf,
    pub name: String,
    pub rows: Vec<Row>,
}

pub fn repos() -> Vec<Repo> {
    let Some(state) = read_state() else {
        return Vec::new();
    };
    let found = match catalog(&state) {
        Ok(found) => found,
        Err(err) => {
            eprintln!("combe: {err}");
            return Vec::new();
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
                pinned: row.pinned,
            })
            .collect();
        if rows.is_empty() {
            continue;
        }
        repos.push(Repo {
            path: repo.path.clone(),
            name: repo_name(&repo.path),
            rows,
        });
    }
    repos
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
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_else(|| path.to_str().unwrap_or("repo"))
        .to_string()
}
