use crate::cards::{Card, Rank, shuffle, standard_deck};
use crate::replay::{CURRENT_REPLAY_VERSION, Replay};
use serde::{Deserialize, Serialize};

const PYRAMID_SIZE: usize = 28;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Options {
    pub max_redeals: u8,
}

impl Default for Options {
    fn default() -> Self {
        Self { max_redeals: 2 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub seed: u64,
    pub options: Options,
    pub pyramid: [Option<Card>; PYRAMID_SIZE],
    pub stock: Vec<Card>,
    pub waste: Vec<Card>,
    pub redeals: u8,
    pub score: u32,
    pub moves: u32,
}

impl State {
    #[must_use]
    pub fn new(seed: u64, options: Options) -> Self {
        let mut deck = standard_deck();
        shuffle(&mut deck, seed);
        let mut pyramid = [None; PYRAMID_SIZE];
        for slot in &mut pyramid {
            *slot = deck.pop();
        }
        Self {
            seed,
            options,
            pyramid,
            stock: deck,
            waste: Vec::new(),
            redeals: 0,
            score: 0,
            moves: 0,
        }
    }

    #[must_use]
    pub fn is_exposed(&self, index: usize) -> bool {
        if index >= PYRAMID_SIZE || self.pyramid[index].is_none() {
            return false;
        }
        let (row, column) = position(index);
        if row == 6 {
            return true;
        }
        let next = (row + 1) * (row + 2) / 2;
        self.pyramid[next + column].is_none() && self.pyramid[next + column + 1].is_none()
    }

    #[must_use]
    pub fn is_won(&self) -> bool {
        self.pyramid.iter().all(Option::is_none)
    }

    #[must_use]
    pub fn card_count(&self) -> usize {
        self.pyramid.iter().flatten().count() + self.stock.len() + self.waste.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    Pyramid(u8),
    Waste,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Draw,
    Recycle,
    RemoveKing(Source),
    RemovePair(Source, Source),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Game {
    pub state: State,
    #[serde(default)]
    undo: Vec<State>,
    #[serde(default)]
    actions: Vec<Action>,
}

impl Game {
    #[must_use]
    pub fn new(seed: u64, options: Options) -> Self {
        Self {
            state: State::new(seed, options),
            undo: Vec::new(),
            actions: Vec::new(),
        }
    }

    /// Applies a pair, king, draw, or recycle action atomically.
    ///
    /// # Errors
    /// Returns [`MoveError`] when the action is illegal.
    pub fn apply(&mut self, action: Action) -> Result<(), MoveError> {
        let before = self.state.clone();
        let result = match action {
            Action::Draw => self.draw(),
            Action::Recycle => self.recycle(),
            Action::RemoveKing(source) => self.remove_king(source),
            Action::RemovePair(first, second) => self.remove_pair(first, second),
        };
        if let Err(error) = result {
            self.state = before;
            return Err(error);
        }
        self.undo.push(before);
        self.actions.push(action);
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.state = previous;
        self.actions.pop();
        true
    }

    #[must_use]
    pub fn replay(&self) -> Replay<Action, Options> {
        Replay {
            version: CURRENT_REPLAY_VERSION,
            game: "pyramid".into(),
            seed: self.state.seed,
            setup: self.state.options,
            actions: self.actions.clone(),
        }
    }

    /// Reconstructs a Pyramid replay.
    ///
    /// # Errors
    /// Returns an error for a wrong game identifier or illegal action.
    pub fn from_replay(replay: &Replay<Action, Options>) -> Result<Self, MoveError> {
        replay
            .validate_version()
            .map_err(|_| MoveError::UnsupportedReplayVersion(replay.version))?;
        if replay.game != "pyramid" {
            return Err(MoveError::WrongGame);
        }
        let mut game = Self::new(replay.seed, replay.setup);
        for action in &replay.actions {
            game.apply(*action)?;
        }
        Ok(game)
    }

    fn draw(&mut self) -> Result<(), MoveError> {
        let card = self.state.stock.pop().ok_or(MoveError::EmptyStock)?;
        self.state.waste.push(card);
        self.state.moves += 1;
        Ok(())
    }

    fn recycle(&mut self) -> Result<(), MoveError> {
        if !self.state.stock.is_empty() || self.state.waste.is_empty() {
            return Err(MoveError::CannotRecycle);
        }
        if self.state.redeals >= self.state.options.max_redeals {
            return Err(MoveError::RedealLimit);
        }
        self.state.stock.extend(self.state.waste.drain(..).rev());
        self.state.redeals += 1;
        self.state.moves += 1;
        Ok(())
    }

    fn remove_king(&mut self, source: Source) -> Result<(), MoveError> {
        if self.card(source)?.rank != Rank::King {
            return Err(MoveError::NotKing);
        }
        self.remove(source);
        self.state.score += 10;
        self.state.moves += 1;
        Ok(())
    }

    fn remove_pair(&mut self, first: Source, second: Source) -> Result<(), MoveError> {
        if first == second {
            return Err(MoveError::SameCard);
        }
        if self.card(first)?.rank.value() + self.card(second)?.rank.value() != 13 {
            return Err(MoveError::NotThirteen);
        }
        self.remove(first);
        self.remove(second);
        self.state.score += 20;
        self.state.moves += 1;
        Ok(())
    }

    fn card(&self, source: Source) -> Result<Card, MoveError> {
        match source {
            Source::Waste => self.state.waste.last().copied().ok_or(MoveError::EmptyPile),
            Source::Pyramid(index) => {
                let index = usize::from(index);
                if !self.state.is_exposed(index) {
                    return Err(MoveError::CoveredCard);
                }
                self.state
                    .pyramid
                    .get(index)
                    .copied()
                    .flatten()
                    .ok_or(MoveError::EmptyPile)
            }
        }
    }

    fn remove(&mut self, source: Source) {
        match source {
            Source::Waste => {
                self.state.waste.pop();
            }
            Source::Pyramid(index) => self.state.pyramid[usize::from(index)] = None,
        }
    }
}

fn position(index: usize) -> (usize, usize) {
    let mut row = 0;
    while (row + 1) * (row + 2) / 2 <= index {
        row += 1;
    }
    (row, index - row * (row + 1) / 2)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    EmptyStock,
    CannotRecycle,
    RedealLimit,
    EmptyPile,
    CoveredCard,
    NotKing,
    SameCard,
    NotThirteen,
    WrongGame,
    UnsupportedReplayVersion(u16),
}

impl std::fmt::Display for MoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal Pyramid action: {self:?}")
    }
}
impl std::error::Error for MoveError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::Suit;
    fn card(rank: Rank) -> Card {
        Card::new(Suit::Spades, rank)
    }

    #[test]
    fn deal_and_exposure_are_correct() {
        let state = State::new(12, Options::default());
        assert_eq!(state.card_count(), 52);
        assert_eq!(state.stock.len(), 24);
        assert!((0..21).all(|index| !state.is_exposed(index)));
        assert!((21..28).all(|index| state.is_exposed(index)));
    }

    #[test]
    fn pair_and_king_rules_are_atomic() {
        let mut game = Game::new(1, Options::default());
        game.state.pyramid = [None; 28];
        game.state.pyramid[21] = Some(card(Rank::Five));
        game.state.pyramid[22] = Some(card(Rank::Eight));
        game.apply(Action::RemovePair(Source::Pyramid(21), Source::Pyramid(22)))
            .unwrap();
        game.state.waste.push(card(Rank::Queen));
        let before = game.state.clone();
        assert_eq!(
            game.apply(Action::RemoveKing(Source::Waste)),
            Err(MoveError::NotKing)
        );
        assert_eq!(game.state, before);
        game.state.waste.push(card(Rank::King));
        game.apply(Action::RemoveKing(Source::Waste)).unwrap();
    }

    #[test]
    fn removing_children_exposes_parent() {
        let mut state = State::new(1, Options::default());
        assert!(!state.is_exposed(15));
        state.pyramid[21] = None;
        state.pyramid[22] = None;
        assert!(state.is_exposed(15));
    }

    #[test]
    fn recycle_limit_undo_and_replay_work() {
        let options = Options { max_redeals: 1 };
        let mut game = Game::new(44, options);
        game.apply(Action::Draw).unwrap();
        let replay = game.replay();
        let drawn = game.state.clone();
        assert!(game.undo());
        assert_eq!(Game::from_replay(&replay).unwrap().state, drawn);
        assert_eq!(Game::from_replay(&replay).unwrap().state.options, options);
        while !game.state.stock.is_empty() {
            game.apply(Action::Draw).unwrap();
        }
        game.apply(Action::Recycle).unwrap();
        while !game.state.stock.is_empty() {
            game.apply(Action::Draw).unwrap();
        }
        assert_eq!(game.apply(Action::Recycle), Err(MoveError::RedealLimit));
    }

    #[test]
    fn typed_replay_with_wrong_version_is_rejected() {
        let mut replay = Game::new(1, Options::default()).replay();
        replay.version = 1;
        assert_eq!(
            Game::from_replay(&replay),
            Err(MoveError::UnsupportedReplayVersion(1))
        );
    }
}
