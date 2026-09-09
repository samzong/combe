use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid state file {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub repos: Vec<Repo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repo {
    pub path: PathBuf,
}

pub fn state_path() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("combe").join("state.json"))
}

pub fn load_state(path: &Path) -> Result<State, StoreError> {
    match std::fs::read_to_string(path) {
        Ok(body) => serde_json::from_str(&body).map_err(|source| StoreError::Parse {
            path: path.to_path_buf(),
            source,
        }),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(State::default()),
        Err(source) => Err(StoreError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

pub fn save_state(path: &Path, state: &State) -> Result<(), StoreError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| StoreError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let body = serde_json::to_string_pretty(state).expect("state is serializable");
    let write = || -> std::io::Result<()> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(body.as_bytes())?;
        temp.as_file().sync_all()?;
        temp.persist(path).map_err(|err| err.error)?;
        Ok(())
    };
    write().map_err(|source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn missing_file_is_empty_state() {
        let path = std::env::temp_dir().join("combe-missing-state.json");
        let _ = std::fs::remove_file(&path);
        let state = load_state(&path).unwrap();
        assert_eq!(state, State::default());
    }

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        let state = State {
            repos: vec![Repo {
                path: PathBuf::from("/tmp/repo"),
            }],
        };
        save_state(&path, &state).unwrap();
        assert_eq!(load_state(&path).unwrap(), state);

        let mut previous = std::fs::File::open(&path).unwrap();
        let updated = State { repos: Vec::new() };
        save_state(&path, &updated).unwrap();
        assert_eq!(load_state(&path).unwrap(), updated);

        let mut body = String::new();
        previous.read_to_string(&mut body).unwrap();
        assert_eq!(serde_json::from_str::<State>(&body).unwrap(), state);
    }
}
