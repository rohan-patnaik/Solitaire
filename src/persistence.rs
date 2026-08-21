use crate::freecell;
use crate::klondike::{Game, ValidationError};
use crate::replay::Replay;
use crate::spider;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

pub const CURRENT_SAVE_VERSION: u16 = 1;
pub const MAX_SAVE_BYTES: u64 = 1_048_576;
pub const MAX_JSON_DEPTH: usize = 64;
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

#[must_use]
pub fn default_deal_counters_path() -> Option<PathBuf> {
    default_named_save_path("deal-counters.json")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DealCounters {
    pub klondike: u64,
    pub spider: u64,
    pub freecell: u64,
}

/// Persists independent per-game next-deal counters.
///
/// # Errors
/// Returns a typed error when serialization or the atomic write fails.
pub fn save_deal_counters(path: &Path, counters: DealCounters) -> Result<(), SaveError> {
    let envelope = SaveEnvelope {
        version: CURRENT_SAVE_VERSION,
        game: "deal-counters".into(),
        payload: counters,
    };
    atomic_write(path, &bounded_json(&envelope)?)?;
    Ok(())
}

/// Loads independent per-game next-deal counters.
///
/// # Errors
/// Returns a typed error for bounded I/O, malformed data, or an unsupported envelope.
pub fn load_deal_counters(path: &Path) -> Result<DealCounters, SaveError> {
    let bytes = read_bounded(path)?;
    validate_json_depth(&bytes)?;
    let envelope: SaveEnvelope<DealCounters> = serde_json::from_slice(&bytes)?;
    if envelope.version != CURRENT_SAVE_VERSION {
        return Err(SaveError::UnsupportedVersion(envelope.version));
    }
    if envelope.game != "deal-counters" {
        return Err(SaveError::WrongGame(envelope.game));
    }
    Ok(envelope.payload)
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
    let json = bounded_json(&envelope)?;
    atomic_write(path, &json)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveRevision {
    length: u64,
    content_hash: u64,
}

/// Fingerprints the current bounded save for compare-and-replace ownership.
///
/// # Errors
/// Returns a typed error when an existing save cannot be read within limits.
pub fn current_save_revision(path: &Path) -> Result<Option<SaveRevision>, SaveError> {
    match read_bounded(path) {
        Ok(bytes) => Ok(Some(revision_for(&bytes))),
        Err(SaveError::Io(error)) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Saves Klondike only if the on-disk revision still matches `expected`.
///
/// # Errors
/// Returns a typed error for invalid state, stale ownership, bounded I/O, or serialization.
pub fn save_klondike_checked(
    path: &Path,
    game: &Game,
    expected: &mut Option<SaveRevision>,
) -> Result<(), SaveError> {
    game.validate()?;
    let envelope = SaveEnvelope {
        version: CURRENT_SAVE_VERSION,
        game: "klondike".into(),
        payload: game,
    };
    compare_and_write(path, &bounded_json(&envelope)?, expected)
}

/// Loads a versioned save or migrates the legacy unversioned `Game` shape.
///
/// # Errors
/// Returns a typed error for I/O, JSON, version, game-kind, or invariant failure.
pub fn load_klondike(path: &Path) -> Result<Game, SaveError> {
    let bytes = read_bounded(path)?;
    validate_json_depth(&bytes)?;
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

/// Loads Klondike and its compare-and-replace revision under one bounded lock.
///
/// # Errors
/// Returns a typed load or bounded-lock error.
pub fn load_klondike_revisioned(path: &Path) -> Result<(Game, SaveRevision), SaveError> {
    load_with_revision(path, load_klondike)
}

/// Saves a Spider game as a versioned deterministic replay.
///
/// # Errors
/// Returns an I/O or serialization error if the atomic save fails.
pub fn save_spider(path: &Path, game: &spider::Game) -> Result<(), SaveError> {
    save_replay(path, "spider", &game.replay())
}

/// Saves Spider only if the on-disk revision still matches `expected`.
///
/// # Errors
/// Returns a typed error for stale ownership, bounded I/O, or serialization.
pub fn save_spider_checked(
    path: &Path,
    game: &spider::Game,
    expected: &mut Option<SaveRevision>,
) -> Result<(), SaveError> {
    save_replay_checked(path, "spider", &game.replay(), expected)
}

/// Loads and legally reconstructs a Spider game from its saved replay.
///
/// # Errors
/// Returns a typed error for malformed, unsupported, mismatched, or illegal data.
pub fn load_spider(path: &Path) -> Result<spider::Game, SaveError> {
    let replay = load_replay::<Replay<spider::Action, spider::SuitMode>>(path, "spider")?;
    spider::Game::from_replay(&replay).map_err(|error| SaveError::InvalidReplay(error.to_string()))
}

/// Loads Spider and its compare-and-replace revision under one bounded lock.
///
/// # Errors
/// Returns a typed load or bounded-lock error.
pub fn load_spider_revisioned(path: &Path) -> Result<(spider::Game, SaveRevision), SaveError> {
    load_with_revision(path, load_spider)
}

/// Saves a `FreeCell` game as a versioned deterministic replay.
///
/// # Errors
/// Returns an I/O or serialization error if the atomic save fails.
pub fn save_freecell(path: &Path, game: &freecell::Game) -> Result<(), SaveError> {
    save_replay(path, "freecell", &game.replay())
}

/// Saves `FreeCell` only if the on-disk revision still matches `expected`.
///
/// # Errors
/// Returns a typed error for stale ownership, bounded I/O, or serialization.
pub fn save_freecell_checked(
    path: &Path,
    game: &freecell::Game,
    expected: &mut Option<SaveRevision>,
) -> Result<(), SaveError> {
    save_replay_checked(path, "freecell", &game.replay(), expected)
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

/// Loads `FreeCell` and its compare-and-replace revision under one bounded lock.
///
/// # Errors
/// Returns a typed load or bounded-lock error.
pub fn load_freecell_revisioned(path: &Path) -> Result<(freecell::Game, SaveRevision), SaveError> {
    load_with_revision(path, load_freecell)
}

fn load_with_revision<T>(
    path: &Path,
    load: impl FnOnce(&Path) -> Result<T, SaveError>,
) -> Result<(T, SaveRevision), SaveError> {
    let _lock = SaveLock::acquire(path)?;
    let value = load(path)?;
    let bytes = read_bounded(path)?;
    Ok((value, revision_for(&bytes)))
}

fn save_replay<T: Serialize>(path: &Path, game: &str, replay: &T) -> Result<(), SaveError> {
    let envelope = SaveEnvelope {
        version: CURRENT_SAVE_VERSION,
        game: game.to_owned(),
        payload: replay,
    };
    atomic_write(path, &bounded_json(&envelope)?)?;
    Ok(())
}

fn save_replay_checked<T: Serialize>(
    path: &Path,
    game: &str,
    replay: &T,
    expected: &mut Option<SaveRevision>,
) -> Result<(), SaveError> {
    let envelope = SaveEnvelope {
        version: CURRENT_SAVE_VERSION,
        game: game.to_owned(),
        payload: replay,
    };
    compare_and_write(path, &bounded_json(&envelope)?, expected)
}

fn compare_and_write(
    path: &Path,
    bytes: &[u8],
    expected: &mut Option<SaveRevision>,
) -> Result<(), SaveError> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "save path has no parent"))?;
    fs::create_dir_all(parent)?;
    let _lock = SaveLock::acquire(path)?;
    let actual = current_save_revision(path)?;
    if actual != *expected {
        return Err(SaveError::Conflict {
            expected: *expected,
            actual,
        });
    }
    atomic_write_locked(path, bytes)?;
    *expected = Some(revision_for(bytes));
    Ok(())
}

fn revision_for(bytes: &[u8]) -> SaveRevision {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    SaveRevision {
        length: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        content_hash: hasher.finish(),
    }
}

fn load_replay<T: DeserializeOwned>(path: &Path, expected_game: &str) -> Result<T, SaveError> {
    let bytes = read_bounded(path)?;
    validate_json_depth(&bytes)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
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

fn bounded_json<T: Serialize>(value: &T) -> Result<Vec<u8>, SaveError> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > usize::try_from(MAX_SAVE_BYTES).unwrap_or(usize::MAX) {
        return Err(SaveError::TooLarge(
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        ));
    }
    Ok(bytes)
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, SaveError> {
    let file = File::open(path)?;
    let size = file.metadata()?.len();
    if size > MAX_SAVE_BYTES {
        return Err(SaveError::TooLarge(size));
    }
    let capacity = usize::try_from(size).map_err(|_| SaveError::TooLarge(size))?;
    let mut bytes = Vec::with_capacity(capacity);
    file.take(MAX_SAVE_BYTES + 1).read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_SAVE_BYTES {
        return Err(SaveError::TooLarge(
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        ));
    }
    Ok(bytes)
}

fn validate_json_depth(bytes: &[u8]) -> Result<(), SaveError> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match *byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth = depth.checked_add(1).ok_or(SaveError::JsonTooDeep)?;
                if depth > MAX_JSON_DEPTH {
                    return Err(SaveError::JsonTooDeep);
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

/// Renames an unreadable save beside the original path for later inspection.
///
/// # Errors
/// Returns an I/O error if the quarantine rename fails.
pub fn quarantine_save(path: &Path) -> io::Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "save path has no parent"))?;
    for sequence in 0..1_024_u16 {
        let suffix = if sequence == 0 {
            format!("json.corrupt-{timestamp}")
        } else {
            format!("json.corrupt-{timestamp}-{sequence}")
        };
        let quarantine = path.with_extension(suffix);
        match fs::hard_link(path, &quarantine) {
            Ok(()) => {
                if let Err(error) = fs::remove_file(path) {
                    let _ = fs::remove_file(&quarantine);
                    return Err(error);
                }
                sync_directory(parent)?;
                return Ok(quarantine);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "no collision-free quarantine name available",
    ))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "save path has no parent"))?;
    fs::create_dir_all(parent)?;
    let _lock = SaveLock::acquire(path)?;
    atomic_write_locked(path, bytes)
}

fn atomic_write_locked(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "save path has no parent"))?;
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
        fs::rename(&temporary, path)?;
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

#[derive(Debug)]
struct SaveLock {
    file: File,
}

impl SaveLock {
    fn acquire(path: &Path) -> io::Result<Self> {
        let file = open_lock_file(path)?;
        let deadline = Instant::now() + Duration::from_millis(250);
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(Self { file }),
                Err(std::fs::TryLockError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        return Err(io::Error::new(
                            io::ErrorKind::WouldBlock,
                            "save is busy in another Solitaire process; retry",
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(std::fs::TryLockError::Error(error)) => return Err(error),
            }
        }
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
    TooLarge(u64),
    JsonTooDeep,
    Conflict {
        expected: Option<SaveRevision>,
        actual: Option<SaveRevision>,
    },
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
            Self::TooLarge(size) => write!(f, "save is {size} bytes; limit is {MAX_SAVE_BYTES}"),
            Self::JsonTooDeep => write!(f, "save JSON nesting exceeds {MAX_JSON_DEPTH} levels"),
            Self::Conflict { .. } => write!(
                f,
                "save changed in another Solitaire process; current game remains in memory—reload the other save or choose a separate data directory"
            ),
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
    use std::io::BufRead;
    use std::process::{Command, Stdio};

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("solitaire-save-test-{}-{name}", std::process::id()))
    }

    #[test]
    fn versioned_save_round_trips() {
        let path = test_path("versioned.json");
        let game = Game::new(123, Options::default());
        save_klondike(&path, &game).unwrap();
        assert_eq!(load_klondike(&path).unwrap(), game);
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn independent_deal_counters_survive_restart() {
        let path = test_path("deal-counters.json");
        let counters = DealCounters {
            klondike: 41,
            spider: 900,
            freecell: 5,
        };
        save_deal_counters(&path, counters).unwrap();
        let restored = load_deal_counters(&path).unwrap();
        assert_eq!(restored, counters);
        assert_eq!(restored.klondike.checked_add(1), Some(42));
        assert_eq!(restored.spider.checked_add(1), Some(901));
        assert_eq!(restored.freecell.checked_add(1), Some(6));
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
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
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn legacy_raw_game_is_migrated() {
        let path = test_path("legacy.json");
        let game = Game::new(321, Options::default());
        atomic_write(&path, game.to_json().unwrap().as_bytes()).unwrap();
        assert_eq!(load_klondike(&path).unwrap(), game);
        fs::remove_file(&path).unwrap();
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
    fn stale_writer_cannot_replace_a_newer_session_save() {
        let path = test_path("stale-writer.json");
        let first_game = Game::new(1, Options::default());
        save_klondike(&path, &first_game).unwrap();
        let initial = current_save_revision(&path).unwrap();
        let mut session_one = initial;
        let mut session_two = initial;
        let newer = Game::new(2, Options::default());
        save_klondike_checked(&path, &newer, &mut session_one).unwrap();
        let stale = Game::new(3, Options::default());
        assert!(matches!(
            save_klondike_checked(&path, &stale, &mut session_two),
            Err(SaveError::Conflict { .. })
        ));
        assert_eq!(load_klondike(&path).unwrap(), newer);
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn save_lock_child_process() {
        let Some(path) = std::env::var_os("SOLITAIRE_LOCK_CHILD_PATH") else {
            return;
        };
        let _lock = SaveLock::acquire(Path::new(&path)).unwrap();
        println!("LOCKED");
        io::stdout().flush().unwrap();
        std::thread::sleep(Duration::from_millis(500));
    }

    #[test]
    fn another_process_gets_bounded_busy_error_and_lock_releases_on_shutdown() {
        let path = test_path("process-lock.json");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "persistence::tests::save_lock_child_process",
                "--exact",
                "--nocapture",
            ])
            .env("SOLITAIRE_LOCK_CHILD_PATH", &path)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut output = std::io::BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        while output.read_line(&mut line).unwrap() != 0 {
            if line.contains("LOCKED") {
                break;
            }
            line.clear();
        }
        let started = Instant::now();
        let error = SaveLock::acquire(&path).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(child.wait().unwrap().success());
        assert!(SaveLock::acquire(&path).is_ok());
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn concurrent_atomic_writes_use_collision_free_private_files() {
        let path = test_path("concurrent.json");
        let threads = (0..8)
            .map(|value| {
                let path = path.clone();
                std::thread::spawn(move || {
                    const MAX_ATTEMPTS: usize = 16;
                    for attempt in 1..=MAX_ATTEMPTS {
                        match atomic_write(&path, &[value; 64]) {
                            Ok(()) => return attempt,
                            Err(error)
                                if error.kind() == io::ErrorKind::WouldBlock
                                    && attempt < MAX_ATTEMPTS =>
                            {
                                std::thread::yield_now();
                            }
                            Err(error) => panic!(
                                "atomic write did not succeed after {attempt} bounded attempts: {error}"
                            ),
                        }
                    }
                    unreachable!("the final bounded attempt always returns or panics")
                })
            })
            .collect::<Vec<_>>();
        let attempts = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(attempts.len(), 8);
        assert!(attempts.iter().all(|attempt| (1..=16).contains(attempt)));
        let bytes = fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 64);
        assert!(bytes.iter().all(|byte| *byte == bytes[0]));
        assert!(
            bytes[0] < 8,
            "the committed data came from one complete writer"
        );
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

    #[test]
    fn oversized_save_is_rejected_before_allocation() {
        let path = test_path("oversized.json");
        let file = File::create(&path).unwrap();
        file.set_len(MAX_SAVE_BYTES + 1).unwrap();
        assert!(matches!(load_klondike(&path), Err(SaveError::TooLarge(_))));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn deeply_nested_json_is_rejected_by_explicit_limit() {
        let path = test_path("deep.json");
        let mut bytes = vec![b'['; MAX_JSON_DEPTH + 1];
        bytes.extend(std::iter::repeat_n(b']', MAX_JSON_DEPTH + 1));
        atomic_write(&path, &bytes).unwrap();
        assert!(matches!(load_klondike(&path), Err(SaveError::JsonTooDeep)));
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn repetitive_replay_is_rejected_before_reconstruction() {
        let path = test_path("repetitive.json");
        let actions = std::iter::repeat_n(
            serde_json::json!({
                "from": {"Cascade": 0}, "to": {"FreeCell": 0}, "count": 1
            }),
            crate::replay::MAX_REPLAY_ACTIONS + 1,
        )
        .collect::<Vec<_>>();
        let envelope = serde_json::json!({
            "version": CURRENT_SAVE_VERSION,
            "game": "freecell",
            "payload": {"version": crate::replay::CURRENT_REPLAY_VERSION,
                "game": "freecell", "seed": 1, "setup": null, "actions": actions}
        });
        atomic_write(&path, &serde_json::to_vec(&envelope).unwrap()).unwrap();
        assert!(matches!(
            load_freecell(&path),
            Err(SaveError::InvalidReplay(_))
        ));
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn quarantine_never_clobbers_an_existing_forensic_copy() {
        let path = test_path("quarantine-collision.json");
        atomic_write(&path, b"broken").unwrap();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let collision = path.with_extension(format!("json.corrupt-{timestamp}"));
        fs::write(&collision, b"existing evidence").unwrap();
        let quarantined = quarantine_save(&path).unwrap();
        assert_ne!(quarantined, collision);
        assert_eq!(fs::read(&collision).unwrap(), b"existing evidence");
        assert_eq!(fs::read(&quarantined).unwrap(), b"broken");
        fs::remove_file(quarantined).unwrap();
        fs::remove_file(collision).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }
}
