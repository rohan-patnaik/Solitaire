use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::time::{Duration, Instant};

pub const CURRENT_REPLAY_VERSION: u16 = 2;
pub const MAX_REPLAY_ACTIONS: usize = 4_096;
pub const MAX_HISTORY_ACTIONS: usize = 512;
pub const MAX_RECONSTRUCTION_TIME: Duration = Duration::from_secs(2);

/// A portable, versioned record of a deterministic game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replay<A, S = ()> {
    pub version: u16,
    pub game: String,
    pub seed: u64,
    pub setup: S,
    pub actions: Vec<A>,
}

impl<A, S> Replay<A, S> {
    #[must_use]
    pub fn new(game: impl Into<String>, seed: u64, setup: S) -> Self {
        Self {
            version: CURRENT_REPLAY_VERSION,
            game: game.into(),
            seed,
            setup,
            actions: Vec::new(),
        }
    }

    /// Adds an action without exceeding the portable replay limit.
    ///
    /// # Errors
    /// Returns [`ReplayError::TooManyActions`] at the limit.
    pub fn push(&mut self, action: A) -> Result<(), ReplayError> {
        validate_action_count(self.actions.len().saturating_add(1))?;
        self.actions.push(action);
        Ok(())
    }

    /// Rejects typed replay values that bypassed JSON construction.
    ///
    /// # Errors
    /// Returns [`ReplayError::UnsupportedVersion`] for any non-current version.
    pub fn validate_version(&self) -> Result<(), ReplayError> {
        validate_version(self.version)?;
        validate_action_count(self.actions.len())
    }

    #[must_use]
    pub fn reconstruction_deadline() -> Instant {
        Instant::now() + MAX_RECONSTRUCTION_TIME
    }

    /// Checks both reconstruction step and wall-clock budgets.
    ///
    /// # Errors
    /// Returns a resource error when either budget is exceeded.
    pub fn check_reconstruction(deadline: Instant, step: usize) -> Result<(), ReplayError> {
        if step > MAX_REPLAY_ACTIONS {
            return Err(ReplayError::TooManyActions(step));
        }
        if Instant::now() > deadline {
            return Err(ReplayError::ReconstructionTimedOut);
        }
        Ok(())
    }
}

impl<A: Serialize, S: Serialize> Replay<A, S> {
    /// Serializes this replay to compact JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if an action cannot be serialized.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

impl<A: DeserializeOwned, S: DeserializeOwned> Replay<A, S> {
    /// Reads a replay and rejects unsupported format versions.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed JSON or an unsupported version.
    pub fn from_json(json: &str) -> Result<Self, ReplayError> {
        #[derive(Deserialize)]
        struct Header {
            version: u16,
        }
        let header: Header = serde_json::from_str(json)?;
        validate_version(header.version)?;
        let replay: Self = serde_json::from_str(json)?;
        replay.validate_version()?;
        Ok(replay)
    }
}

/// Validates a replay format version without parsing its version-specific body.
///
/// # Errors
/// Returns [`ReplayError::UnsupportedVersion`] for any non-current version.
pub fn validate_version(version: u16) -> Result<(), ReplayError> {
    if version == CURRENT_REPLAY_VERSION {
        Ok(())
    } else {
        Err(ReplayError::UnsupportedVersion(version))
    }
}

/// Validates the portable replay action-count limit.
///
/// # Errors
/// Returns [`ReplayError::TooManyActions`] when `count` exceeds the limit.
pub fn validate_action_count(count: usize) -> Result<(), ReplayError> {
    if count <= MAX_REPLAY_ACTIONS {
        Ok(())
    } else {
        Err(ReplayError::TooManyActions(count))
    }
}

#[derive(Debug)]
pub enum ReplayError {
    InvalidJson(serde_json::Error),
    UnsupportedVersion(u16),
    TooManyActions(usize),
    ReconstructionTimedOut,
}

impl From<serde_json::Error> for ReplayError {
    fn from(value: serde_json::Error) -> Self {
        Self::InvalidJson(value)
    }
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(error) => write!(formatter, "invalid replay: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported replay version {version}")
            }
            Self::TooManyActions(count) => write!(
                formatter,
                "replay has {count} actions; limit is {MAX_REPLAY_ACTIONS}"
            ),
            Self::ReconstructionTimedOut => write!(formatter, "replay reconstruction timed out"),
        }
    }
}

impl std::error::Error for ReplayError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    enum Action {
        Draw,
    }

    #[test]
    fn replay_round_trips() {
        let mut replay = Replay::new("klondike", 7, "draw-one".to_owned());
        replay.push(Action::Draw).unwrap();
        let json = replay.to_json().unwrap();
        assert_eq!(Replay::from_json(&json).unwrap(), replay);
    }

    #[test]
    fn future_replay_is_rejected_cleanly() {
        let json = r#"{"version":99,"game":"klondike","seed":7,"setup":null,"actions":[]}"#;
        let error = Replay::<Action>::from_json(json).unwrap_err();
        assert!(matches!(error, ReplayError::UnsupportedVersion(99)));
    }

    #[test]
    fn legacy_shape_is_classified_by_version_before_v2_fields() {
        let json = r#"{"version":1,"game":"klondike","seed":7,"actions":[]}"#;
        let error = Replay::<Action, String>::from_json(json).unwrap_err();
        assert!(matches!(error, ReplayError::UnsupportedVersion(1)));
    }

    #[test]
    fn reconstruction_checks_step_and_time_budgets() {
        let past = Instant::now()
            .checked_sub(Duration::from_millis(1))
            .unwrap();
        assert!(matches!(
            Replay::<Action>::check_reconstruction(past, 1),
            Err(ReplayError::ReconstructionTimedOut)
        ));
        assert!(matches!(
            Replay::<Action>::check_reconstruction(
                Instant::now() + Duration::from_secs(1),
                MAX_REPLAY_ACTIONS + 1
            ),
            Err(ReplayError::TooManyActions(_))
        ));
    }
}
