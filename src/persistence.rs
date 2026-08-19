use serde::{Serialize, de::DeserializeOwned};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[must_use]
pub fn default_save_path() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .map(|root| root.join("solitaire/klondike-save.json"))
}

/// Atomically writes a serializable value to disk.
///
/// # Errors
///
/// Returns an I/O error if the parent cannot be created or the file cannot be
/// serialized, written, or renamed into place.
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "save path has no parent"))?;
    fs::create_dir_all(parent)?;
    let temporary = path.with_extension("json.tmp");
    let json = serde_json::to_vec(value).map_err(io::Error::other)?;
    fs::write(&temporary, json)?;
    fs::rename(temporary, path)
}

/// Reads a JSON value from disk.
///
/// # Errors
///
/// Returns an I/O error if the file is absent, unreadable, or malformed.
pub fn load_json<T: DeserializeOwned>(path: &Path) -> io::Result<T> {
    let json = fs::read(path)?;
    serde_json::from_slice(&json).map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klondike::{Game, Options};

    #[test]
    fn atomic_save_round_trips_a_game() {
        let directory =
            std::env::temp_dir().join(format!("solitaire-persistence-test-{}", std::process::id()));
        let path = directory.join("nested/save.json");
        let game = Game::new(123, Options::default());
        save_json(&path, &game).unwrap();
        let loaded: Game = load_json(&path).unwrap();
        assert_eq!(loaded, game);
        fs::remove_dir_all(directory).unwrap();
    }
}
