use crate::klondike::{Game, ValidationError};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const CURRENT_SAVE_VERSION: u16 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct SaveEnvelope<T> {
    version: u16,
    game: String,
    payload: T,
}

#[must_use]
pub fn default_save_path() -> Option<PathBuf> {
    save_root(
        std::env::var_os("XDG_DATA_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
    .map(|root| root.join("solitaire/klondike-save.json"))
}

fn save_root(xdg_data_home: Option<&OsStr>, home: Option<&OsStr>) -> Option<PathBuf> {
    xdg_data_home
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty() && path.is_absolute())
        .or_else(|| {
            home.map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .map(|path| path.join(".local/share"))
        })
}

/// Atomically writes a versioned Klondike save.
///
/// # Errors
/// Returns an I/O error if serialization or the atomic write fails.
pub fn save_klondike(path: &Path, game: &Game) -> Result<(), SaveError> {
    game.validate()?;
    let envelope = SaveEnvelope {
        version: CURRENT_SAVE_VERSION,
        game: "klondike".into(),
        payload: game,
    };
    let json = serde_json::to_vec(&envelope)?;
    atomic_write(path, &json)?;
    Ok(())
}

/// Loads a versioned save or migrates the legacy unversioned `Game` shape.
///
/// # Errors
/// Returns a typed error for I/O, JSON, version, game-kind, or invariant failure.
pub fn load_klondike(path: &Path) -> Result<Game, SaveError> {
    let bytes = fs::read(path)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let game = if value.get("version").is_some() {
        let version = value
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .and_then(|version| u16::try_from(version).ok())
            .ok_or_else(|| serde_json::Error::io(io::Error::other("invalid save version")))?;
        if version != CURRENT_SAVE_VERSION {
            return Err(SaveError::UnsupportedVersion(version));
        }
        let envelope: SaveEnvelope<Game> = serde_json::from_value(value)?;
        if envelope.game != "klondike" {
            return Err(SaveError::WrongGame(envelope.game));
        }
        envelope.payload
    } else {
        serde_json::from_value(value)?
    };
    game.validate()?;
    Ok(game)
}

/// Renames an unreadable save beside the original path for later inspection.
///
/// # Errors
/// Returns an I/O error if the quarantine rename fails.
pub fn quarantine_save(path: &Path) -> io::Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let quarantine = path.with_extension(format!("json.corrupt-{timestamp}"));
    fs::rename(path, &quarantine)?;
    Ok(quarantine)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "save path has no parent"))?;
    fs::create_dir_all(parent)?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, path)
}

#[derive(Debug)]
pub enum SaveError {
    Io(io::Error),
    Json(serde_json::Error),
    UnsupportedVersion(u16),
    WrongGame(String),
    InvalidState(ValidationError),
}

impl From<io::Error> for SaveError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<serde_json::Error> for SaveError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
impl From<ValidationError> for SaveError {
    fn from(value: ValidationError) -> Self {
        Self::InvalidState(value)
    }
}
impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "save I/O failed: {error}"),
            Self::Json(error) => write!(f, "save data is malformed: {error}"),
            Self::UnsupportedVersion(version) => write!(f, "save version {version} is unsupported"),
            Self::WrongGame(game) => write!(f, "save contains {game}, not Klondike"),
            Self::InvalidState(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for SaveError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klondike::Options;

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("solitaire-save-test-{}-{name}", std::process::id()))
    }

    #[test]
    fn versioned_save_round_trips() {
        let path = test_path("versioned.json");
        let game = Game::new(123, Options::default());
        save_klondike(&path, &game).unwrap();
        assert_eq!(load_klondike(&path).unwrap(), game);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn legacy_raw_game_is_migrated() {
        let path = test_path("legacy.json");
        let game = Game::new(321, Options::default());
        atomic_write(&path, game.to_json().unwrap().as_bytes()).unwrap();
        assert_eq!(load_klondike(&path).unwrap(), game);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn future_and_corrupt_fixtures_are_rejected() {
        let future = test_path("future.json");
        atomic_write(&future, br#"{"version":99,"game":"klondike","payload":{}}"#).unwrap();
        assert!(matches!(
            load_klondike(&future),
            Err(SaveError::UnsupportedVersion(99))
        ));
        fs::remove_file(future).unwrap();

        let corrupt = test_path("corrupt.json");
        atomic_write(
            &corrupt,
            br#"{"version":1,"game":"klondike","payload":{"seed":1}}"#,
        )
        .unwrap();
        assert!(matches!(load_klondike(&corrupt), Err(SaveError::Json(_))));
        let quarantined = quarantine_save(&corrupt).unwrap();
        assert!(!corrupt.exists());
        assert!(quarantined.exists());
        fs::remove_file(quarantined).unwrap();
    }

    #[test]
    fn unrelated_history_snapshot_fixture_is_rejected() {
        use crate::klondike::Action;

        let path = test_path("history-corrupt.json");
        let mut game = Game::new(777, Options::default());
        game.apply(Action::Draw).unwrap();
        save_klondike(&path, &game).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["payload"]["actions"] = serde_json::json!([]);
        atomic_write(&path, &serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(matches!(
            load_klondike(&path),
            Err(SaveError::InvalidState(
                ValidationError::UndoActionCardinality
            ))
        ));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn xdg_root_requires_nonempty_absolute_path() {
        let home = OsStr::new("/home/player");
        assert_eq!(
            save_root(Some(OsStr::new("")), Some(home)),
            Some(PathBuf::from("/home/player/.local/share"))
        );
        assert_eq!(
            save_root(Some(OsStr::new("relative")), Some(home)),
            Some(PathBuf::from("/home/player/.local/share"))
        );
        assert_eq!(
            save_root(Some(OsStr::new("/data/player")), Some(home)),
            Some(PathBuf::from("/data/player"))
        );
    }
}
