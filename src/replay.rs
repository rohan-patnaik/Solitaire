use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub const CURRENT_REPLAY_VERSION: u16 = 2;

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

    pub fn push(&mut self, action: A) {
        self.actions.push(action);
    }

    /// Rejects typed replay values that bypassed JSON construction.
    ///
    /// # Errors
    /// Returns [`ReplayError::UnsupportedVersion`] for any non-current version.
    pub fn validate_version(&self) -> Result<(), ReplayError> {
        validate_version(self.version)
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

#[derive(Debug)]
pub enum ReplayError {
    InvalidJson(serde_json::Error),
    UnsupportedVersion(u16),
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
        replay.push(Action::Draw);
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
}
