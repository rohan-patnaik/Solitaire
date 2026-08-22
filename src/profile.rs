use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameKind {
    Klondike,
    Spider,
    FreeCell,
    TriPeaks,
    Pyramid,
}

impl GameKind {
    const fn index(self) -> usize {
        match self {
            Self::Klondike => 0,
            Self::Spider => 1,
            Self::FreeCell => 2,
            Self::TriPeaks => 3,
            Self::Pyramid => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameStatistics {
    pub deals_played: u64,
    pub deals_won: u64,
    pub latest_played_deal: Option<u64>,
    pub latest_won_deal: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalProfile {
    statistics: [GameStatistics; 5],
}

impl LocalProfile {
    #[must_use]
    pub const fn statistics(&self, game: GameKind) -> GameStatistics {
        self.statistics[game.index()]
    }

    /// Records the first successful move and optional win for one monotonically numbered deal.
    ///
    /// Repeating the same observation is idempotent. Older observations fail closed so delayed
    /// or malformed input cannot inflate counters.
    ///
    /// # Errors
    /// Returns a typed error for stale deal numbers, counter overflow, or invalid stored state.
    pub fn observe(&mut self, game: GameKind, deal: u64, won: bool) -> Result<bool, ProfileError> {
        self.validate()?;
        let mut updated = self.statistics[game.index()];
        let mut changed = false;
        match updated.latest_played_deal {
            Some(latest) if deal < latest => return Err(ProfileError::StaleDeal),
            Some(latest) if deal == latest => {}
            _ => {
                updated.deals_played = updated
                    .deals_played
                    .checked_add(1)
                    .ok_or(ProfileError::CounterOverflow)?;
                updated.latest_played_deal = Some(deal);
                changed = true;
            }
        }
        if won {
            match updated.latest_won_deal {
                Some(latest) if deal < latest => return Err(ProfileError::StaleDeal),
                Some(latest) if deal == latest => {}
                _ => {
                    updated.deals_won = updated
                        .deals_won
                        .checked_add(1)
                        .ok_or(ProfileError::CounterOverflow)?;
                    updated.latest_won_deal = Some(deal);
                    changed = true;
                }
            }
        }
        validate_statistics(updated)?;
        self.statistics[game.index()] = updated;
        Ok(changed)
    }

    /// Validates all persisted profile invariants.
    ///
    /// # Errors
    /// Returns a typed error when counters and deal markers disagree.
    pub fn validate(&self) -> Result<(), ProfileError> {
        self.statistics
            .iter()
            .copied()
            .try_for_each(validate_statistics)
    }
}

fn validate_statistics(statistics: GameStatistics) -> Result<(), ProfileError> {
    if statistics.deals_won > statistics.deals_played {
        return Err(ProfileError::WinsExceedPlayed);
    }
    if (statistics.deals_played == 0) != statistics.latest_played_deal.is_none()
        || (statistics.deals_won == 0) != statistics.latest_won_deal.is_none()
    {
        return Err(ProfileError::MissingDealMarker);
    }
    if statistics
        .latest_won_deal
        .zip(statistics.latest_played_deal)
        .is_some_and(|(won, played)| won > played)
    {
        return Err(ProfileError::WinAfterLatestPlayedDeal);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileError {
    StaleDeal,
    CounterOverflow,
    WinsExceedPlayed,
    MissingDealMarker,
    WinAfterLatestPlayedDeal,
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid local profile: {self:?}")
    }
}

impl std::error::Error for ProfileError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observations_are_per_game_idempotent_and_wins_imply_play() {
        let mut profile = LocalProfile::default();
        assert!(profile.observe(GameKind::Klondike, 41, false).unwrap());
        assert!(!profile.observe(GameKind::Klondike, 41, false).unwrap());
        assert!(profile.observe(GameKind::Klondike, 41, true).unwrap());
        assert!(!profile.observe(GameKind::Klondike, 41, true).unwrap());
        assert!(profile.observe(GameKind::Spider, 900, true).unwrap());

        assert_eq!(
            profile.statistics(GameKind::Klondike),
            GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(41),
                latest_won_deal: Some(41),
            }
        );
        assert_eq!(profile.statistics(GameKind::Spider).deals_played, 1);
        assert_eq!(profile.statistics(GameKind::Spider).deals_won, 1);
        assert_eq!(
            profile.statistics(GameKind::FreeCell),
            GameStatistics::default()
        );
    }

    #[test]
    fn stale_and_overflowing_observations_are_atomic() {
        let mut profile = LocalProfile::default();
        profile.observe(GameKind::Pyramid, 20, true).unwrap();
        let before = profile.clone();
        assert_eq!(
            profile.observe(GameKind::Pyramid, 19, false),
            Err(ProfileError::StaleDeal)
        );
        assert_eq!(profile, before);

        profile.statistics[GameKind::TriPeaks.index()] = GameStatistics {
            deals_played: u64::MAX,
            deals_won: 0,
            latest_played_deal: Some(4),
            latest_won_deal: None,
        };
        let before = profile.clone();
        assert_eq!(
            profile.observe(GameKind::TriPeaks, 5, false),
            Err(ProfileError::CounterOverflow)
        );
        assert_eq!(profile, before);
    }
}
