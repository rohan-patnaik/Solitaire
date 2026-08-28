use crate::freecell;
use crate::klondike::{Game, ValidationError};
use crate::profile::{LocalProfile, ProfileError};
use crate::pyramid;
use crate::replay::Replay;
use crate::spider;
use crate::tripeaks;
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
pub fn default_tripeaks_save_path() -> Option<PathBuf> {
    default_named_save_path("tripeaks-save.json")
}

#[must_use]
pub fn default_pyramid_save_path() -> Option<PathBuf> {
    default_named_save_path("pyramid-save.json")
}

#[must_use]
pub fn default_deal_counters_path() -> Option<PathBuf> {
    default_named_save_path("deal-counters.json")
}

#[must_use]
pub fn default_local_profile_path() -> Option<PathBuf> {
    default_named_save_path("local-profile.json")
}

/// Atomically saves the bounded device-local profile.
///
/// # Errors
/// Returns a typed validation, serialization, or I/O error.
pub fn save_local_profile(path: &Path, profile: &LocalProfile) -> Result<(), SaveError> {
    profile.validate()?;
    let envelope = SaveEnvelope {
        version: CURRENT_SAVE_VERSION,
        game: "local-profile".into(),
        payload: profile,
    };
    finish_namespace_write(
        atomic_write(path, &bounded_json(&envelope)?)?,
        "saving local profile",
    )
}

/// Saves the device-local profile only when the on-disk revision still matches `expected`.
///
/// # Errors
/// Returns a typed validation, stale ownership, serialization, or I/O error.
pub fn save_local_profile_checked(
    path: &Path,
    profile: &LocalProfile,
    expected: &mut Option<SaveRevision>,
) -> Result<(), SaveError> {
    profile.validate()?;
    let envelope = SaveEnvelope {
        version: CURRENT_SAVE_VERSION,
        game: "local-profile".into(),
        payload: profile,
    };
    compare_and_write(path, &bounded_json(&envelope)?, expected)
}

/// Loads and validates the device-local profile.
///
/// # Errors
/// Returns a typed bounded-I/O, envelope, JSON, or profile-validation error.
pub fn load_local_profile(path: &Path) -> Result<LocalProfile, SaveError> {
    let bytes = read_bounded(path)?;
    parse_local_profile(&bytes)
}

fn parse_local_profile(bytes: &[u8]) -> Result<LocalProfile, SaveError> {
    validate_json_depth(bytes)?;
    let envelope: SaveEnvelope<LocalProfile> = serde_json::from_slice(bytes)?;
    if envelope.version != CURRENT_SAVE_VERSION {
        return Err(SaveError::UnsupportedVersion(envelope.version));
    }
    if envelope.game != "local-profile" {
        return Err(SaveError::WrongGame(envelope.game));
    }
    envelope.payload.validate()?;
    Ok(envelope.payload)
}

/// Loads the local profile and its compare-and-replace revision under one bounded lock.
///
/// # Errors
/// Returns a typed load or bounded-lock error.
pub fn load_local_profile_revisioned(
    path: &Path,
) -> Result<(LocalProfile, SaveRevision), SaveError> {
    load_with_revision(path, parse_local_profile)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DealCounters {
    pub klondike: u64,
    pub spider: u64,
    pub freecell: u64,
    #[serde(default)]
    pub tripeaks: u64,
    #[serde(default)]
    pub pyramid: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DealKind {
    Klondike,
    Spider,
    FreeCell,
    TriPeaks,
    Pyramid,
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
    finish_namespace_write(
        atomic_write(path, &bounded_json(&envelope)?)?,
        "saving deal counters",
    )
}

/// Loads independent per-game next-deal counters.
///
/// # Errors
/// Returns a typed error for bounded I/O, malformed data, or an unsupported envelope.
pub fn load_deal_counters(path: &Path) -> Result<DealCounters, SaveError> {
    let bytes = read_bounded(path)?;
    parse_deal_counters(&bytes)
}

/// Ensures the durable per-game counter file exists without reserving a deal.
///
/// Existing values are never lowered, so concurrent or previously persisted
/// reservations remain authoritative.
///
/// # Errors
/// Returns a typed error for bounded locking, malformed storage, or atomic write failure.
pub fn ensure_deal_counters(path: &Path, minimum: DealCounters) -> Result<DealCounters, SaveError> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "save path has no parent"))?;
    fs::create_dir_all(parent)?;
    let _lock = SaveLock::acquire(path)?;
    let mut counters = match read_bounded(path) {
        Ok(bytes) => parse_deal_counters(&bytes)?,
        Err(SaveError::Io(error)) if error.kind() == io::ErrorKind::NotFound => minimum,
        Err(error) => return Err(error),
    };
    counters.klondike = counters.klondike.max(minimum.klondike);
    counters.spider = counters.spider.max(minimum.spider);
    counters.freecell = counters.freecell.max(minimum.freecell);
    counters.tripeaks = counters.tripeaks.max(minimum.tripeaks);
    counters.pyramid = counters.pyramid.max(minimum.pyramid);
    let envelope = SaveEnvelope {
        version: CURRENT_SAVE_VERSION,
        game: "deal-counters".into(),
        payload: counters,
    };
    finish_namespace_write(
        atomic_write_locked(path, &bounded_json(&envelope)?)?,
        "initializing deal counters",
    )?;
    Ok(counters)
}

fn parse_deal_counters(bytes: &[u8]) -> Result<DealCounters, SaveError> {
    validate_json_depth(bytes)?;
    let envelope: SaveEnvelope<DealCounters> = serde_json::from_slice(bytes)?;
    if envelope.version != CURRENT_SAVE_VERSION {
        return Err(SaveError::UnsupportedVersion(envelope.version));
    }
    if envelope.game != "deal-counters" {
        return Err(SaveError::WrongGame(envelope.game));
    }
    Ok(envelope.payload)
}

/// Reserves one unique per-game deal number in a single locked read-modify-write transaction.
///
/// Existing counters are raised to `minimum` so older counter files can never reuse a deal that
/// is already open. The returned counters are the durably committed next values.
///
/// # Errors
/// Returns a typed error for bounded locking, malformed storage, exhaustion, or atomic write.
pub fn reserve_deal(
    path: &Path,
    minimum: DealCounters,
    kind: DealKind,
) -> Result<(u64, DealCounters), SaveError> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "save path has no parent"))?;
    fs::create_dir_all(parent)?;
    let _lock = SaveLock::acquire(path)?;
    let mut counters = match read_bounded(path) {
        Ok(bytes) => parse_deal_counters(&bytes)?,
        Err(SaveError::Io(error)) if error.kind() == io::ErrorKind::NotFound => minimum,
        Err(error) => return Err(error),
    };
    counters.klondike = counters.klondike.max(minimum.klondike);
    counters.spider = counters.spider.max(minimum.spider);
    counters.freecell = counters.freecell.max(minimum.freecell);
    counters.tripeaks = counters.tripeaks.max(minimum.tripeaks);
    counters.pyramid = counters.pyramid.max(minimum.pyramid);
    let seed = match kind {
        DealKind::Klondike => counters.klondike,
        DealKind::Spider => counters.spider,
        DealKind::FreeCell => counters.freecell,
        DealKind::TriPeaks => counters.tripeaks,
        DealKind::Pyramid => counters.pyramid,
    };
    let next = seed.checked_add(1).ok_or(SaveError::CounterOverflow)?;
    match kind {
        DealKind::Klondike => counters.klondike = next,
        DealKind::Spider => counters.spider = next,
        DealKind::FreeCell => counters.freecell = next,
        DealKind::TriPeaks => counters.tripeaks = next,
        DealKind::Pyramid => counters.pyramid = next,
    }
    let envelope = SaveEnvelope {
        version: CURRENT_SAVE_VERSION,
        game: "deal-counters".into(),
        payload: counters,
    };
    finish_namespace_write(
        atomic_write_locked(path, &bounded_json(&envelope)?)?,
        "reserving a deal number",
    )?;
    Ok((seed, counters))
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
    finish_namespace_write(atomic_write(path, &json)?, "saving Klondike")
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

/// Rechecks save ownership under the same bounded inter-process lock used by writers.
///
/// A missing path returns `None`. Existing malformed content still returns its revision, while
/// permission and other I/O failures remain errors so callers cannot assume ownership.
///
/// # Errors
/// Returns a bounded lock or I/O error when absence cannot be confirmed safely.
pub fn confirm_current_save_revision(path: &Path) -> Result<Option<SaveRevision>, SaveError> {
    let _lock = SaveLock::acquire(path)?;
    current_save_revision(path)
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
    parse_klondike(&bytes)
}

fn parse_klondike(bytes: &[u8]) -> Result<Game, SaveError> {
    validate_json_depth(bytes)?;
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
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
    load_with_revision(path, parse_klondike)
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
    let bytes = read_bounded(path)?;
    parse_spider(&bytes)
}

fn parse_spider(bytes: &[u8]) -> Result<spider::Game, SaveError> {
    let replay = parse_replay::<Replay<spider::Action, spider::SuitMode>>(bytes, "spider")?;
    spider::Game::from_replay(&replay).map_err(|error| SaveError::InvalidReplay(error.to_string()))
}

/// Loads Spider and its compare-and-replace revision under one bounded lock.
///
/// # Errors
/// Returns a typed load or bounded-lock error.
pub fn load_spider_revisioned(path: &Path) -> Result<(spider::Game, SaveRevision), SaveError> {
    load_with_revision(path, parse_spider)
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
    let bytes = read_bounded(path)?;
    parse_freecell(&bytes)
}

fn parse_freecell(bytes: &[u8]) -> Result<freecell::Game, SaveError> {
    let replay = parse_replay::<Replay<freecell::Action>>(bytes, "freecell")?;
    freecell::Game::from_replay(&replay)
        .map_err(|error| SaveError::InvalidReplay(error.to_string()))
}

/// Loads `FreeCell` and its compare-and-replace revision under one bounded lock.
///
/// # Errors
/// Returns a typed load or bounded-lock error.
pub fn load_freecell_revisioned(path: &Path) -> Result<(freecell::Game, SaveRevision), SaveError> {
    load_with_revision(path, parse_freecell)
}

/// Saves a `TriPeaks` game as a versioned deterministic replay.
///
/// # Errors
/// Returns an error if serialization or the atomic save fails.
pub fn save_tripeaks(path: &Path, game: &tripeaks::Game) -> Result<(), SaveError> {
    save_replay(path, "tripeaks", &game.replay())
}

/// Saves `TriPeaks` only if the on-disk revision still matches `expected`.
///
/// # Errors
/// Returns a typed error for stale ownership, bounded I/O, or serialization.
pub fn save_tripeaks_checked(
    path: &Path,
    game: &tripeaks::Game,
    expected: &mut Option<SaveRevision>,
) -> Result<(), SaveError> {
    save_replay_checked(path, "tripeaks", &game.replay(), expected)
}

/// Loads and legally reconstructs a `TriPeaks` game from its saved replay.
///
/// # Errors
/// Returns a typed error for malformed, unsupported, mismatched, or illegal data.
pub fn load_tripeaks(path: &Path) -> Result<tripeaks::Game, SaveError> {
    let bytes = read_bounded(path)?;
    parse_tripeaks(&bytes)
}

fn parse_tripeaks(bytes: &[u8]) -> Result<tripeaks::Game, SaveError> {
    let replay = parse_replay::<Replay<tripeaks::Action, tripeaks::Options>>(bytes, "tripeaks")?;
    tripeaks::Game::from_replay(&replay)
        .map_err(|error| SaveError::InvalidReplay(error.to_string()))
}

/// Loads `TriPeaks` and its compare-and-replace revision under one bounded lock.
///
/// # Errors
/// Returns a typed load or bounded-lock error.
pub fn load_tripeaks_revisioned(path: &Path) -> Result<(tripeaks::Game, SaveRevision), SaveError> {
    load_with_revision(path, parse_tripeaks)
}

/// Saves a standard Pyramid game as a versioned deterministic replay.
///
/// # Errors
/// Returns an I/O or serialization error if the atomic save fails.
pub fn save_pyramid(path: &Path, game: &pyramid::Game) -> Result<(), SaveError> {
    save_replay(path, "pyramid", &game.replay())
}

/// Saves Pyramid only if the on-disk revision still matches `expected`.
///
/// # Errors
/// Returns a typed error for stale ownership, bounded I/O, or serialization.
pub fn save_pyramid_checked(
    path: &Path,
    game: &pyramid::Game,
    expected: &mut Option<SaveRevision>,
) -> Result<(), SaveError> {
    save_replay_checked(path, "pyramid", &game.replay(), expected)
}

/// Loads and legally reconstructs a Pyramid game from its saved replay.
///
/// # Errors
/// Returns a typed error for malformed, unsupported, mismatched, or illegal data.
pub fn load_pyramid(path: &Path) -> Result<pyramid::Game, SaveError> {
    let bytes = read_bounded(path)?;
    parse_pyramid(&bytes)
}

fn parse_pyramid(bytes: &[u8]) -> Result<pyramid::Game, SaveError> {
    let replay = parse_replay::<Replay<pyramid::Action, pyramid::Options>>(bytes, "pyramid")?;
    if replay.setup != pyramid::Options::default() {
        return Err(SaveError::InvalidReplay(
            "standard Pyramid saves require exactly two redeals".into(),
        ));
    }
    pyramid::Game::from_replay(&replay).map_err(|error| SaveError::InvalidReplay(error.to_string()))
}

/// Loads Pyramid and its compare-and-replace revision under one bounded lock.
///
/// # Errors
/// Returns a typed load or bounded-lock error.
pub fn load_pyramid_revisioned(path: &Path) -> Result<(pyramid::Game, SaveRevision), SaveError> {
    load_with_revision(path, parse_pyramid)
}

fn load_with_revision<T>(
    path: &Path,
    parse: impl FnOnce(&[u8]) -> Result<T, SaveError>,
) -> Result<(T, SaveRevision), SaveError> {
    let _lock = SaveLock::acquire(path)?;
    let bytes = read_bounded(path)?;
    let value = parse(&bytes)?;
    Ok((value, revision_for(&bytes)))
}

#[derive(Debug)]
pub enum RecoveredSave<T> {
    Loaded(T, SaveRevision),
    Quarantined {
        path: PathBuf,
        reason: String,
        durability_warning: Option<String>,
    },
}

#[derive(Debug)]
pub enum NamespaceMutation<T> {
    Durable(T),
    CommittedButNotDurable { value: T, error: io::Error },
}

/// Loads or atomically quarantines a malformed Klondike save under one bounded lock.
///
/// # Errors
/// Returns without moving the source if bounded reading, identity checking, or quarantine fails.
pub fn recover_klondike_revisioned(path: &Path) -> Result<RecoveredSave<Game>, SaveError> {
    recover_with_revision(path, parse_klondike, || {}, sync_directory)
}

/// Loads or atomically quarantines a malformed Spider save under one bounded lock.
///
/// # Errors
/// Returns without moving the source if bounded reading, identity checking, or quarantine fails.
pub fn recover_spider_revisioned(path: &Path) -> Result<RecoveredSave<spider::Game>, SaveError> {
    recover_with_revision(path, parse_spider, || {}, sync_directory)
}

/// Loads or atomically quarantines a malformed `FreeCell` save under one bounded lock.
///
/// # Errors
/// Returns without moving the source if bounded reading, identity checking, or quarantine fails.
pub fn recover_freecell_revisioned(
    path: &Path,
) -> Result<RecoveredSave<freecell::Game>, SaveError> {
    recover_with_revision(path, parse_freecell, || {}, sync_directory)
}

/// Loads or atomically quarantines a malformed `TriPeaks` save under one bounded lock.
///
/// # Errors
/// Returns without moving the source if bounded reading, identity checking, or quarantine fails.
pub fn recover_tripeaks_revisioned(
    path: &Path,
) -> Result<RecoveredSave<tripeaks::Game>, SaveError> {
    recover_with_revision(path, parse_tripeaks, || {}, sync_directory)
}

/// Loads or atomically quarantines a malformed Pyramid save under one bounded lock.
///
/// # Errors
/// Returns without moving the source if bounded reading, identity checking, or quarantine fails.
pub fn recover_pyramid_revisioned(path: &Path) -> Result<RecoveredSave<pyramid::Game>, SaveError> {
    recover_with_revision(path, parse_pyramid, || {}, sync_directory)
}

/// Loads or atomically quarantines a malformed local profile under one bounded lock.
///
/// # Errors
/// Returns without moving the source if bounded reading, identity checking, or quarantine fails.
pub fn recover_local_profile_revisioned(
    path: &Path,
) -> Result<RecoveredSave<LocalProfile>, SaveError> {
    recover_with_revision(path, parse_local_profile, || {}, sync_directory)
}

fn recover_with_revision<T>(
    path: &Path,
    parse: impl FnOnce(&[u8]) -> Result<T, SaveError>,
    before_identity_recheck: impl FnOnce(),
    sync: impl FnOnce(&Path) -> io::Result<()>,
) -> Result<RecoveredSave<T>, SaveError> {
    let _lock = SaveLock::acquire(path)?;
    let bytes = read_bounded(path)?;
    let expected = revision_for(&bytes);
    match parse(&bytes) {
        Ok(value) => Ok(RecoveredSave::Loaded(value, expected)),
        Err(error) => {
            let reason = error.to_string();
            before_identity_recheck();
            let actual = current_save_revision(path)?;
            if actual != Some(expected) {
                return Err(SaveError::Conflict {
                    expected: Some(expected),
                    actual,
                });
            }
            match quarantine_save_locked_with_sync(path, expected, sync)? {
                NamespaceMutation::Durable(path) => Ok(RecoveredSave::Quarantined {
                    path,
                    reason,
                    durability_warning: None,
                }),
                NamespaceMutation::CommittedButNotDurable { value: path, error } => {
                    Ok(RecoveredSave::Quarantined {
                        path,
                        reason,
                        durability_warning: Some(error.to_string()),
                    })
                }
            }
        }
    }
}

fn save_replay<T: Serialize>(path: &Path, game: &str, replay: &T) -> Result<(), SaveError> {
    let envelope = SaveEnvelope {
        version: CURRENT_SAVE_VERSION,
        game: game.to_owned(),
        payload: replay,
    };
    finish_namespace_write(
        atomic_write(path, &bounded_json(&envelope)?)?,
        "saving replay",
    )
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
    compare_and_write_with_sync(path, bytes, expected, sync_directory)
}

fn compare_and_write_with_sync(
    path: &Path,
    bytes: &[u8],
    expected: &mut Option<SaveRevision>,
    sync: impl FnOnce(&Path) -> io::Result<()>,
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
    let outcome = atomic_write_locked_with_sync(path, bytes, sync)?;
    *expected = Some(revision_for(bytes));
    finish_namespace_write(outcome, "replacing the save")
}

fn revision_for(bytes: &[u8]) -> SaveRevision {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    SaveRevision {
        length: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        content_hash: hasher.finish(),
    }
}

fn parse_replay<T: DeserializeOwned>(bytes: &[u8], expected_game: &str) -> Result<T, SaveError> {
    validate_json_depth(bytes)?;
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
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
/// Returns a typed error if bounded locking, identity checking, or quarantine fails.
pub fn quarantine_save(path: &Path) -> Result<NamespaceMutation<PathBuf>, SaveError> {
    let _lock = SaveLock::acquire(path)?;
    let bytes = read_bounded(path)?;
    quarantine_save_locked(path, revision_for(&bytes))
}

fn quarantine_save_locked(
    path: &Path,
    expected: SaveRevision,
) -> Result<NamespaceMutation<PathBuf>, SaveError> {
    quarantine_save_locked_with_sync(path, expected, sync_directory)
}

fn quarantine_save_locked_with_sync(
    path: &Path,
    expected: SaveRevision,
    sync: impl FnOnce(&Path) -> io::Result<()>,
) -> Result<NamespaceMutation<PathBuf>, SaveError> {
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
                let linked_revision = match read_bounded(&quarantine) {
                    Ok(bytes) => revision_for(&bytes),
                    Err(error) => {
                        let _ = fs::remove_file(&quarantine);
                        return Err(error);
                    }
                };
                let same_identity = match same_file_identity(path, &quarantine) {
                    Ok(same) => same,
                    Err(error) => {
                        let _ = fs::remove_file(&quarantine);
                        return Err(error.into());
                    }
                };
                if linked_revision != expected || !same_identity {
                    let _ = fs::remove_file(&quarantine);
                    return Err(SaveError::Conflict {
                        expected: Some(expected),
                        actual: current_save_revision(path)?,
                    });
                }
                if let Err(error) = fs::remove_file(path) {
                    let _ = fs::remove_file(&quarantine);
                    return Err(error.into());
                }
                return Ok(match sync(parent) {
                    Ok(()) => NamespaceMutation::Durable(quarantine),
                    Err(error) => NamespaceMutation::CommittedButNotDurable {
                        value: quarantine,
                        error,
                    },
                });
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "no collision-free quarantine name available",
    )
    .into())
}

#[cfg(unix)]
fn same_file_identity(left: &Path, right: &Path) -> io::Result<bool> {
    use std::os::unix::fs::MetadataExt;
    let left = fs::metadata(left)?;
    let right = fs::metadata(right)?;
    Ok(left.dev() == right.dev() && left.ino() == right.ino())
}

#[cfg(not(unix))]
fn same_file_identity(left: &Path, right: &Path) -> io::Result<bool> {
    Ok(fs::canonicalize(left)? == fs::canonicalize(right)?)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<NamespaceMutation<()>> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "save path has no parent"))?;
    fs::create_dir_all(parent)?;
    let _lock = SaveLock::acquire(path)?;
    atomic_write_locked(path, bytes)
}

fn atomic_write_locked(path: &Path, bytes: &[u8]) -> io::Result<NamespaceMutation<()>> {
    atomic_write_locked_with_sync(path, bytes, sync_directory)
}

fn atomic_write_locked_with_sync(
    path: &Path,
    bytes: &[u8],
    sync: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<NamespaceMutation<()>> {
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
        Ok(match sync(parent) {
            Ok(()) => NamespaceMutation::Durable(()),
            Err(error) => NamespaceMutation::CommittedButNotDurable { value: (), error },
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn finish_namespace_write(
    outcome: NamespaceMutation<()>,
    operation: &'static str,
) -> Result<(), SaveError> {
    match outcome {
        NamespaceMutation::Durable(()) => Ok(()),
        NamespaceMutation::CommittedButNotDurable { error, .. } => {
            Err(SaveError::CommittedButNotDurable { operation, error })
        }
    }
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
    InvalidProfile(ProfileError),
    TooLarge(u64),
    JsonTooDeep,
    CounterOverflow,
    CommittedButNotDurable {
        operation: &'static str,
        error: io::Error,
    },
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
impl From<ProfileError> for SaveError {
    fn from(value: ProfileError) -> Self {
        Self::InvalidProfile(value)
    }
}
impl SaveError {
    #[must_use]
    pub const fn committed_but_not_durable(&self) -> bool {
        matches!(self, Self::CommittedButNotDurable { .. })
    }

    #[must_use]
    pub const fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict { .. })
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
            Self::InvalidProfile(error) => error.fmt(f),
            Self::TooLarge(size) => write!(f, "save is {size} bytes; limit is {MAX_SAVE_BYTES}"),
            Self::JsonTooDeep => write!(f, "save JSON nesting exceeds {MAX_JSON_DEPTH} levels"),
            Self::CounterOverflow => write!(f, "no further deal number is representable"),
            Self::CommittedButNotDurable { operation, error } => write!(
                f,
                "{operation} changed the on-disk entry, but directory synchronization failed: {error}; retry to confirm durability"
            ),
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
    use crate::profile::GameKind as ProfileGameKind;
    use crate::spider::SuitMode;
    use std::io::BufRead;
    use std::process::{Command, Stdio};

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("solitaire-save-test-{}-{name}", std::process::id()))
    }

    fn durable_path(outcome: NamespaceMutation<PathBuf>) -> PathBuf {
        match outcome {
            NamespaceMutation::Durable(path) => path,
            NamespaceMutation::CommittedButNotDurable { .. } => {
                panic!("test filesystem unexpectedly failed directory synchronization")
            }
        }
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
            tripeaks: 77,
            pyramid: 303,
        };
        save_deal_counters(&path, counters).unwrap();
        let restored = load_deal_counters(&path).unwrap();
        assert_eq!(restored, counters);
        assert_eq!(restored.klondike.checked_add(1), Some(42));
        assert_eq!(restored.spider.checked_add(1), Some(901));
        assert_eq!(restored.freecell.checked_add(1), Some(6));
        assert_eq!(restored.tripeaks.checked_add(1), Some(78));
        assert_eq!(restored.pyramid.checked_add(1), Some(304));
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn ensuring_deal_counters_never_reserves_or_lowers_existing_values() {
        let path = test_path("ensure-deal-counters.json");
        let minimum = DealCounters {
            klondike: 41,
            spider: 900,
            freecell: 5,
            tripeaks: 77,
            pyramid: 303,
        };
        assert_eq!(ensure_deal_counters(&path, minimum).unwrap(), minimum);

        let advanced = DealCounters {
            klondike: 44,
            spider: 901,
            freecell: 12,
            tripeaks: 80,
            pyramid: 400,
        };
        save_deal_counters(&path, advanced).unwrap();
        assert_eq!(ensure_deal_counters(&path, minimum).unwrap(), advanced);
        assert_eq!(load_deal_counters(&path).unwrap(), advanced);

        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn older_deal_counters_default_pyramid_without_changing_other_games() {
        let path = test_path("deal-counters-before-pyramid.json");
        fs::write(
            &path,
            br#"{"version":1,"game":"deal-counters","payload":{"klondike":41,"spider":900,"freecell":5,"tripeaks":77}}"#,
        )
        .unwrap();

        assert_eq!(
            load_deal_counters(&path).unwrap(),
            DealCounters {
                klondike: 41,
                spider: 900,
                freecell: 5,
                tripeaks: 77,
                pyramid: 0,
            }
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn local_profile_reopens_equivalently_and_repeated_events_are_idempotent() {
        let path = test_path("local-profile.json");
        let mut profile = LocalProfile::default();
        profile
            .observe(ProfileGameKind::Klondike, 41, false)
            .unwrap();
        profile
            .observe(ProfileGameKind::Klondike, 41, true)
            .unwrap();
        profile
            .observe(ProfileGameKind::Klondike, 41, true)
            .unwrap();
        save_local_profile(&path, &profile).unwrap();

        assert_eq!(load_local_profile(&path).unwrap(), profile);
        assert_eq!(
            profile.statistics(ProfileGameKind::Klondike).deals_played,
            1
        );
        assert_eq!(profile.statistics(ProfileGameKind::Klondike).deals_won, 1);
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn stale_local_profile_writer_cannot_replace_newer_statistics() {
        let path = test_path("local-profile-conflict.json");
        save_local_profile(&path, &LocalProfile::default()).unwrap();
        let (mut first, first_revision) = load_local_profile_revisioned(&path).unwrap();
        let (mut stale, stale_revision) = load_local_profile_revisioned(&path).unwrap();
        first.observe(ProfileGameKind::Spider, 10, false).unwrap();
        let mut expected = Some(first_revision);
        save_local_profile_checked(&path, &first, &mut expected).unwrap();
        let committed = fs::read(&path).unwrap();

        stale.observe(ProfileGameKind::Spider, 11, true).unwrap();
        let stale_in_memory = stale.clone();
        let mut stale_expected = Some(stale_revision);
        assert!(matches!(
            save_local_profile_checked(&path, &stale, &mut stale_expected),
            Err(SaveError::Conflict { .. })
        ));
        assert_eq!(stale, stale_in_memory);
        assert_eq!(fs::read(&path).unwrap(), committed);
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn invalid_local_profile_is_quarantined_without_losing_source_bytes() {
        let path = test_path("local-profile-invalid.json");
        let invalid = serde_json::json!({
            "version": CURRENT_SAVE_VERSION,
            "game": "local-profile",
            "payload": {
                "statistics": [
                    {"deals_played": 0, "deals_won": 1,
                     "latest_played_deal": null, "latest_won_deal": 1},
                    {"deals_played": 0, "deals_won": 0,
                     "latest_played_deal": null, "latest_won_deal": null},
                    {"deals_played": 0, "deals_won": 0,
                     "latest_played_deal": null, "latest_won_deal": null},
                    {"deals_played": 0, "deals_won": 0,
                     "latest_played_deal": null, "latest_won_deal": null},
                    {"deals_played": 0, "deals_won": 0,
                     "latest_played_deal": null, "latest_won_deal": null}
                ]
            }
        });
        let source = serde_json::to_vec(&invalid).unwrap();
        atomic_write(&path, &source).unwrap();
        assert!(matches!(
            load_local_profile(&path),
            Err(SaveError::InvalidProfile(ProfileError::WinsExceedPlayed))
        ));

        let RecoveredSave::Quarantined {
            path: quarantined,
            reason,
            ..
        } = recover_local_profile_revisioned(&path).unwrap()
        else {
            panic!("invalid local profile should be quarantined");
        };
        assert!(reason.contains("WinsExceedPlayed"));
        assert!(!path.exists());
        assert_eq!(fs::read(&quarantined).unwrap(), source);
        fs::remove_file(quarantined).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn reserve_deal_child_process() {
        let Some(path) = std::env::var_os("SOLITAIRE_RESERVE_CHILD_PATH") else {
            return;
        };
        let minimum = DealCounters {
            klondike: 100,
            spider: 900,
            freecell: 5,
            tripeaks: 77,
            pyramid: 303,
        };
        for attempt in 1..=16 {
            match reserve_deal(Path::new(&path), minimum, DealKind::Klondike) {
                Ok((seed, _)) => {
                    println!("RESERVED={seed}");
                    io::stdout().flush().unwrap();
                    return;
                }
                Err(SaveError::Io(error))
                    if error.kind() == io::ErrorKind::WouldBlock && attempt < 16 =>
                {
                    std::thread::yield_now();
                }
                Err(error) => panic!("deal reservation failed on attempt {attempt}: {error}"),
            }
        }
    }

    #[test]
    fn multiple_processes_reserve_unique_per_game_deals() {
        let path = test_path("reserve-processes.json");
        let children = (0..8)
            .map(|_| {
                Command::new(std::env::current_exe().unwrap())
                    .args([
                        "persistence::tests::reserve_deal_child_process",
                        "--exact",
                        "--nocapture",
                    ])
                    .env("SOLITAIRE_RESERVE_CHILD_PATH", &path)
                    .stdout(Stdio::piped())
                    .spawn()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let mut seeds = children
            .into_iter()
            .map(|child| {
                let output = child.wait_with_output().unwrap();
                assert!(output.status.success());
                let stdout = String::from_utf8(output.stdout).unwrap();
                stdout
                    .lines()
                    .find_map(|line| line.strip_prefix("RESERVED="))
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        seeds.sort_unstable();
        assert_eq!(seeds, (100..108).collect::<Vec<_>>());
        assert_eq!(load_deal_counters(&path).unwrap().klondike, 108);
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
    fn tripeaks_replay_save_reopens_equivalent_state_and_history() {
        let path = test_path("tripeaks.json");
        let mut game = tripeaks::Game::new(31, tripeaks::Options::default());
        game.apply(tripeaks::Action::Draw).unwrap();
        let expected = game.state.clone();
        save_tripeaks(&path, &game).unwrap();

        let mut reopened = load_tripeaks(&path).unwrap();
        assert_eq!(reopened.state, expected);
        assert!(reopened.can_undo());
        assert!(reopened.undo());
        assert!(reopened.can_redo());
        assert!(reopened.redo());
        assert_eq!(reopened.state, expected);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn wraparound_tripeaks_checked_save_reopens_equivalent() {
        let path = test_path("tripeaks-wraparound-save.json");
        let standard = tripeaks::Game::new(31, tripeaks::Options::default());
        save_tripeaks(&path, &standard).unwrap();
        let (_, revision) = load_tripeaks_revisioned(&path).unwrap();
        let mut expected = Some(revision);
        let mut wraparound = tripeaks::Game::new(32, tripeaks::Options { wraparound: true });
        wraparound.apply(tripeaks::Action::Draw).unwrap();

        save_tripeaks_checked(&path, &wraparound, &mut expected).unwrap();
        let (mut reopened, reopened_revision) = load_tripeaks_revisioned(&path).unwrap();
        assert_eq!(expected, Some(reopened_revision));
        assert_eq!(reopened, wraparound);
        assert!(reopened.undo());
        assert!(reopened.redo());
        assert_eq!(reopened, wraparound);
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn tripeaks_counter_overflow_preserves_counter_file() {
        let path = test_path("tripeaks-counter-overflow.json");
        let minimum = DealCounters {
            klondike: 1,
            spider: 1,
            freecell: 1,
            tripeaks: u64::MAX,
            pyramid: 1,
        };
        save_deal_counters(&path, minimum).unwrap();
        let before = fs::read(&path).unwrap();
        assert!(matches!(
            reserve_deal(&path, minimum, DealKind::TriPeaks),
            Err(SaveError::CounterOverflow)
        ));
        assert_eq!(fs::read(&path).unwrap(), before);
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn corrupt_tripeaks_save_is_quarantined_without_losing_source_bytes() {
        let path = test_path("tripeaks-corrupt.json");
        let corrupt = br#"{"version":1,"game":"tripeaks","payload":{"version":2}}"#;
        atomic_write(&path, corrupt).unwrap();

        let RecoveredSave::Quarantined {
            path: quarantined,
            reason,
            ..
        } = recover_tripeaks_revisioned(&path).unwrap()
        else {
            panic!("corrupt TriPeaks save should be quarantined");
        };
        assert!(!reason.is_empty());
        assert!(!path.exists());
        assert_eq!(fs::read(&quarantined).unwrap(), corrupt);
        fs::remove_file(quarantined).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn wraparound_tripeaks_setup_is_accepted_and_preserved() {
        let path = test_path("tripeaks-wraparound.json");
        let replay: Replay<tripeaks::Action, tripeaks::Options> = Replay {
            version: crate::replay::CURRENT_REPLAY_VERSION,
            game: "tripeaks".into(),
            seed: 1,
            setup: tripeaks::Options { wraparound: true },
            actions: Vec::new(),
        };
        save_replay(&path, "tripeaks", &replay).unwrap();
        let loaded = load_tripeaks(&path).unwrap();
        assert!(loaded.state.options.wraparound);
        assert_eq!(loaded.replay(), replay);
        assert!(matches!(
            recover_tripeaks_revisioned(&path).unwrap(),
            RecoveredSave::Loaded(game, _) if game == loaded
        ));
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn pyramid_replay_save_reopens_equivalent_state_and_history() {
        let path = test_path("pyramid.json");
        let mut game = pyramid::Game::new(43, pyramid::Options::default());
        game.apply(pyramid::Action::Draw).unwrap();
        let expected = game.state.clone();
        save_pyramid(&path, &game).unwrap();

        let mut reopened = load_pyramid(&path).unwrap();
        assert_eq!(reopened.state, expected);
        assert!(reopened.can_undo());
        assert!(reopened.undo());
        assert!(reopened.can_redo());
        assert!(reopened.redo());
        assert_eq!(reopened.state, expected);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn pyramid_counter_overflow_preserves_counter_file() {
        let path = test_path("pyramid-counter-overflow.json");
        let minimum = DealCounters {
            klondike: 1,
            spider: 1,
            freecell: 1,
            tripeaks: 1,
            pyramid: u64::MAX,
        };
        save_deal_counters(&path, minimum).unwrap();
        let before = fs::read(&path).unwrap();
        assert!(matches!(
            reserve_deal(&path, minimum, DealKind::Pyramid),
            Err(SaveError::CounterOverflow)
        ));
        assert_eq!(fs::read(&path).unwrap(), before);
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn corrupt_pyramid_save_is_quarantined_without_losing_source_bytes() {
        let path = test_path("pyramid-corrupt.json");
        let corrupt = br#"{"version":1,"game":"pyramid","payload":{"version":2}}"#;
        atomic_write(&path, corrupt).unwrap();

        let RecoveredSave::Quarantined {
            path: quarantined,
            reason,
            ..
        } = recover_pyramid_revisioned(&path).unwrap()
        else {
            panic!("corrupt Pyramid save should be quarantined");
        };
        assert!(!reason.is_empty());
        assert!(!path.exists());
        assert_eq!(fs::read(&quarantined).unwrap(), corrupt);
        fs::remove_file(quarantined).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn illegal_pyramid_saved_replay_is_rejected() {
        let path = test_path("pyramid-illegal.json");
        let replay = Replay {
            version: crate::replay::CURRENT_REPLAY_VERSION,
            game: "pyramid".into(),
            seed: 1,
            setup: pyramid::Options::default(),
            actions: vec![pyramid::Action::RemoveKing(pyramid::Source::Pyramid(0))],
        };
        save_replay(&path, "pyramid", &replay).unwrap();
        assert!(matches!(
            load_pyramid(&path),
            Err(SaveError::InvalidReplay(_))
        ));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn nonstandard_pyramid_setups_are_rejected_and_quarantined() {
        for max_redeals in [0, u8::MAX] {
            let path = test_path(&format!("pyramid-redeals-{max_redeals}.json"));
            let replay: Replay<pyramid::Action, pyramid::Options> = Replay {
                version: crate::replay::CURRENT_REPLAY_VERSION,
                game: "pyramid".into(),
                seed: 1,
                setup: pyramid::Options { max_redeals },
                actions: Vec::new(),
            };
            save_replay(&path, "pyramid", &replay).unwrap();
            let source = fs::read(&path).unwrap();
            assert!(matches!(
                load_pyramid(&path),
                Err(SaveError::InvalidReplay(reason))
                    if reason == "standard Pyramid saves require exactly two redeals"
            ));

            let RecoveredSave::Quarantined {
                path: quarantined,
                reason,
                ..
            } = recover_pyramid_revisioned(&path).unwrap()
            else {
                panic!("nonstandard Pyramid save should be quarantined");
            };
            assert!(reason.contains("exactly two redeals"));
            assert!(!path.exists());
            assert_eq!(fs::read(&quarantined).unwrap(), source);
            fs::remove_file(quarantined).unwrap();
            fs::remove_file(path.with_extension("json.lock")).unwrap();
        }
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
        let quarantined = durable_path(quarantine_save(&corrupt).unwrap());
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
    fn post_rename_sync_failure_advances_revision_and_retry_is_safe() {
        let path = test_path("post-rename-sync.json");
        let first = Game::new(501, Options::default());
        save_klondike(&path, &first).unwrap();
        let mut expected = current_save_revision(&path).unwrap();
        let replacement = Game::new(502, Options::default());
        let bytes = bounded_json(&SaveEnvelope {
            version: CURRENT_SAVE_VERSION,
            game: "klondike".to_owned(),
            payload: &replacement,
        })
        .unwrap();

        let error = compare_and_write_with_sync(&path, &bytes, &mut expected, |_| {
            Err(io::Error::other("injected directory sync failure"))
        })
        .unwrap_err();

        assert!(error.committed_but_not_durable());
        assert_eq!(expected, Some(revision_for(&bytes)));
        assert_eq!(load_klondike(&path).unwrap(), replacement);
        compare_and_write(&path, &bytes, &mut expected).unwrap();
        assert_eq!(load_klondike(&path).unwrap(), replacement);
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
                            Ok(_) => return attempt,
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
        let quarantined = durable_path(quarantine_save(&path).unwrap());
        assert_ne!(quarantined, collision);
        assert_eq!(fs::read(&collision).unwrap(), b"existing evidence");
        assert_eq!(fs::read(&quarantined).unwrap(), b"broken");
        fs::remove_file(quarantined).unwrap();
        fs::remove_file(collision).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn corrupt_recovery_excludes_a_coordinated_replacement_until_quarantined() {
        let path = test_path("quarantine-coordinated.json");
        atomic_write(&path, b"broken").unwrap();
        let replacement = bounded_json(&SaveEnvelope {
            version: CURRENT_SAVE_VERSION,
            game: "klondike".to_owned(),
            payload: Game::new(818, Options::default()),
        })
        .unwrap();
        let recovery = recover_with_revision(
            &path,
            |_| Err::<Game, SaveError>(SaveError::JsonTooDeep),
            || {
                let path = path.clone();
                let writer = std::thread::spawn(move || atomic_write(&path, &replacement));
                let error = writer.join().unwrap().unwrap_err();
                assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
            },
            sync_directory,
        )
        .unwrap();
        let RecoveredSave::Quarantined {
            path: quarantined, ..
        } = recovery
        else {
            panic!("corrupt save should be quarantined");
        };
        assert!(!path.exists());
        assert_eq!(fs::read(&quarantined).unwrap(), b"broken");
        fs::remove_file(quarantined).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn identity_change_aborts_quarantine_without_moving_replacement() {
        let path = test_path("quarantine-identity.json");
        atomic_write(&path, b"broken").unwrap();
        let replacement = b"replacement remains";
        let result = recover_with_revision(
            &path,
            |_| Err::<Game, SaveError>(SaveError::JsonTooDeep),
            || fs::write(&path, replacement).unwrap(),
            sync_directory,
        );
        assert!(matches!(result, Err(SaveError::Conflict { .. })));
        assert_eq!(fs::read(&path).unwrap(), replacement);
        fs::remove_file(&path).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }

    #[test]
    fn post_quarantine_sync_failure_reports_committed_location() {
        let path = test_path("post-quarantine-sync.json");
        atomic_write(&path, b"broken").unwrap();

        let recovery = recover_with_revision(
            &path,
            |_| Err::<Game, SaveError>(SaveError::JsonTooDeep),
            || {},
            |_| Err(io::Error::other("injected directory sync failure")),
        )
        .unwrap();
        let RecoveredSave::Quarantined {
            path: quarantined,
            durability_warning: Some(warning),
            ..
        } = recovery
        else {
            panic!("post-unlink sync failure must retain the committed quarantine path");
        };

        assert!(warning.contains("injected directory sync failure"));
        assert!(!path.exists());
        assert_eq!(fs::read(&quarantined).unwrap(), b"broken");
        assert!(matches!(
            recover_klondike_revisioned(&path),
            Err(SaveError::Io(error)) if error.kind() == io::ErrorKind::NotFound
        ));
        fs::remove_file(quarantined).unwrap();
        fs::remove_file(path.with_extension("json.lock")).unwrap();
    }
}
