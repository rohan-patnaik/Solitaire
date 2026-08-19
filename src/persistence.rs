use crate::freecell;
use crate::klondike::{Game, ValidationError};
use crate::replay::Replay;
use crate::spider;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const CURRENT_SAVE_VERSION: u16 = 1;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Serialize, Deserialize)]
struct SaveEnvelope<T> {
    version: u16,
    game: String,
    payload: T,
}

#[must_use]
pub fn default_save_path() -> Option<PathBuf> {
    default_named_save_path("klondike-save.json")
}

#[must_use]
pub fn default_spider_save_path() -> Option<PathBuf> {
    default_named_save_path("spider-save.json")
}

#[must_use]
pub fn default_freecell_save_path() -> Option<PathBuf> {
    default_named_save_path("freecell-save.json")
}

fn default_named_save_path(file_name: &str) -> Option<PathBuf> {
    save_root(
        std::env::var_os("XDG_DATA_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
    .map(|root| root.join("solitaire").join(file_name))
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

/// Saves a Spider game as a versioned deterministic replay.
///
/// # Errors
/// Returns an I/O or serialization error if the atomic save fails.
pub fn save_spider(path: &Path, game: &spider::Game) -> Result<(), SaveError> {
    save_replay(path, "spider", &game.replay())
}

/// Loads and legally reconstructs a Spider game from its saved replay.
///
/// # Errors
/// Returns a typed error for malformed, unsupported, mismatched, or illegal data.
pub fn load_spider(path: &Path) -> Result<spider::Game, SaveError> {
    let replay = load_replay::<Replay<spider::Action, spider::SuitMode>>(path, "spider")?;
    spider::Game::from_replay(&replay).map_err(|error| SaveError::InvalidReplay(error.to_string()))
}

/// Saves a `FreeCell` game as a versioned deterministic replay.
///
/// # Errors
/// Returns an I/O or serialization error if the atomic save fails.
pub fn save_freecell(path: &Path, game: &freecell::Game) -> Result<(), SaveError> {
    save_replay(path, "freecell", &game.replay())
}

/// Loads and legally reconstructs a `FreeCell` game from its saved replay.
///
/// # Errors
/// Returns a typed error for malformed, unsupported, mismatched, or illegal data.
pub fn load_freecell(path: &Path) -> Result<freecell::Game, SaveError> {
    let replay = load_replay::<Replay<freecell::Action>>(path, "freecell")?;
    freecell::Game::from_replay(&replay)
        .map_err(|error| SaveError::InvalidReplay(error.to_string()))
}

fn save_replay<T: Serialize>(path: &Path, game: &str, replay: &T) -> Result<(), SaveError> {
    let envelope = SaveEnvelope {
        version: CURRENT_SAVE_VERSION,
        game: game.to_owned(),
        payload: replay,
    };
    atomic_write(path, &serde_json::to_vec(&envelope)?)?;
    Ok(())
}

fn load_replay<T: DeserializeOwned>(path: &Path, expected_game: &str) -> Result<T, SaveError> {
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .and_then(|version| u16::try_from(version).ok())
        .ok_or_else(|| serde_json::Error::io(io::Error::other("invalid save version")))?;
    if version != CURRENT_SAVE_VERSION {
        return Err(SaveError::UnsupportedVersion(version));
    }
    let envelope: SaveEnvelope<T> = serde_json::from_value(value)?;
    if envelope.game != expected_game {
        return Err(SaveError::WrongGame(envelope.game));
    }
    Ok(envelope.payload)
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
    let _lock = SaveLock::acquire(path)?;
    let file_name = path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid save filename"))?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".{file_name}.tmp-{}-{sequence}",
        std::process::id()
    ));
    let result = (|| {
        let mut file = private_create_new(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[derive(Debug)]
struct SaveLock {
    file: File,
}

impl SaveLock {
    fn acquire(path: &Path) -> io::Result<Self> {
        let file = open_lock_file(path)?;
        file.lock()?;
        Ok(Self { file })
    }

    #[cfg(test)]
    fn try_acquire(path: &Path) -> io::Result<Self> {
        let file = open_lock_file(path)?;
        file.try_lock()?;
        Ok(Self { file })
    }
}

impl Drop for SaveLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

fn open_lock_file(path: &Path) -> io::Result<File> {
    let lock_path = path.with_extension("json.lock");
    private_open(&lock_path, false)
}

fn private_create_new(path: &Path) -> io::Result<File> {
    private_open(path, true)
}

#[cfg(unix)]
fn private_open(path: &Path, create_new: bool) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut options = OpenOptions::new();
    options.read(true).write(true).mode(0o600);
    if create_new {
        options.create_new(true);
    } else {
        options.create(true);
    }
    options.open(path)
}

#[cfg(not(unix))]
fn private_open(path: &Path, create_new: bool) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    if create_new {
        options.create_new(true);
    } else {
        options.create(true);
    }
    options.open(path)
}

#[derive(Debug)]
pub enum SaveError {
    Io(io::Error),
    Json(serde_json::Error),
    UnsupportedVersion(u16),
    WrongGame(String),
    InvalidState(ValidationError),
    InvalidReplay(String),
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
            Self::WrongGame(game) => write!(f, "save contains unexpected game {game}"),
            Self::InvalidState(error) => error.fmt(f),
            Self::InvalidReplay(error) => write!(f, "saved replay is invalid: {error}"),
        }
    }
}
impl std::error::Error for SaveError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::freecell::{Action as FreeCellAction, Pile as FreeCellPile};
    use crate::klondike::Options;
    use crate::spider::SuitMode;

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
    fn spider_and_freecell_replay_saves_round_trip() {
        let spider_path = test_path("spider.json");
        let mut spider = spider::Game::new(19, SuitMode::Two);
        spider.apply(spider::Action::DealRow).unwrap();
        save_spider(&spider_path, &spider).unwrap();
        assert_eq!(load_spider(&spider_path).unwrap().state, spider.state);
        fs::remove_file(spider_path).unwrap();

        let freecell_path = test_path("freecell.json");
        let mut freecell = freecell::Game::new(27);
        freecell
            .apply(FreeCellAction {
                from: FreeCellPile::Cascade(0),
                to: FreeCellPile::FreeCell(0),
                count: 1,
            })
            .unwrap();
        save_freecell(&freecell_path, &freecell).unwrap();
        assert_eq!(load_freecell(&freecell_path).unwrap().state, freecell.state);
        fs::remove_file(freecell_path).unwrap();
    }

    #[test]
    fn illegal_spider_and_freecell_saved_replays_are_rejected() {
        let spider_path = test_path("spider-illegal.json");
        let replay = Replay {
            version: crate::replay::CURRENT_REPLAY_VERSION,
            game: "spider".into(),
            seed: 1,
            setup: SuitMode::One,
            actions: vec![spider::Action::Move {
                from: 0,
                to: 0,
                count: 1,
            }],
        };
        save_replay(&spider_path, "spider", &replay).unwrap();
        assert!(matches!(
            load_spider(&spider_path),
            Err(SaveError::InvalidReplay(_))
        ));
        fs::remove_file(spider_path).unwrap();

        let freecell_path = test_path("freecell-illegal.json");
        let replay = Replay {
            version: crate::replay::CURRENT_REPLAY_VERSION,
            game: "freecell".into(),
            seed: 1,
            setup: (),
            actions: vec![FreeCellAction {
                from: FreeCellPile::Cascade(0),
                to: FreeCellPile::Cascade(0),
                count: 1,
            }],
        };
        save_replay(&freecell_path, "freecell", &replay).unwrap();
        assert!(matches!(
            load_freecell(&freecell_path),
            Err(SaveError::InvalidReplay(_))
        ));
        fs::remove_file(freecell_path).unwrap();
    }

    #[test]
    fn future_replay_save_is_classified_before_its_payload() {
        let path = test_path("spider-future.json");
        atomic_write(&path, br#"{"version":99,"game":"spider","payload":{}}"#).unwrap();
        assert!(matches!(
            load_spider(&path),
            Err(SaveError::UnsupportedVersion(99))
        ));
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
    fn untimed_save_with_elapsed_time_is_rejected() {
        let path = test_path("untimed-elapsed.json");
        let game = Game::new(778, Options::default());
        save_klondike(&path, &game).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["payload"]["state"]["elapsed_seconds"] = serde_json::json!(1);
        atomic_write(&path, &serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(matches!(
            load_klondike(&path),
            Err(SaveError::InvalidState(
                ValidationError::ElapsedTimeInUntimedGame
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

    #[test]
    fn save_lock_excludes_another_writer() {
        let path = test_path("locked.json");
        let first = SaveLock::try_acquire(&path).unwrap();
        let error = SaveLock::try_acquire(&path).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        drop(first);
        assert!(SaveLock::try_acquire(&path).is_ok());
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn concurrent_atomic_writes_use_collision_free_private_files() {
        let path = test_path("concurrent.json");
        let threads = (0..8)
            .map(|value| {
                let path = path.clone();
                std::thread::spawn(move || atomic_write(&path, &[value; 64]).unwrap())
            })
            .collect::<Vec<_>>();
        for thread in threads {
            thread.join().unwrap();
        }
        let bytes = fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 64);
        assert!(bytes.iter().all(|byte| *byte == bytes[0]));
        let prefix = format!(".{}.tmp-", path.file_name().unwrap().to_string_lossy());
        assert!(
            fs::read_dir(path.parent().unwrap())
                .unwrap()
                .filter_map(Result::ok)
                .all(|entry| !entry.file_name().to_string_lossy().starts_with(&prefix))
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }
}
