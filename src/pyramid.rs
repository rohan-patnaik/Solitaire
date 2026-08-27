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
    redo: Vec<State>,
    #[serde(default)]
    actions: Vec<Action>,
    #[serde(default)]
    redo_actions: Vec<Action>,
}

impl Game {
    #[must_use]
    pub fn new(seed: u64, options: Options) -> Self {
        Self {
            state: State::new(seed, options),
            undo: Vec::new(),
            redo: Vec::new(),
            actions: Vec::new(),
            redo_actions: Vec::new(),
        }
    }

    /// Applies a pair, king, draw, or recycle action atomically.
    ///
    /// # Errors
    /// Returns [`MoveError`] when the action is illegal.
    pub fn apply(&mut self, action: Action) -> Result<(), MoveError> {
        if self.state.is_won() {
            return Err(MoveError::GameComplete);
        }
        if self.actions.len() >= crate::replay::MAX_REPLAY_ACTIONS {
            return Err(MoveError::ResourceLimit);
        }
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
        push_bounded_history(&mut self.undo, before);
        self.redo.clear();
        self.actions.push(action);
        self.redo_actions.clear();
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(&mut self.state, previous));
        if let Some(action) = self.actions.pop() {
            self.redo_actions.push(action);
        }
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        push_bounded_history(&mut self.undo, std::mem::replace(&mut self.state, next));
        if let Some(action) = self.redo_actions.pop() {
            self.actions.push(action);
        }
        true
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
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
        crate::replay::validate_version(replay.version)
            .map_err(|_| MoveError::UnsupportedReplayVersion(replay.version))?;
        crate::replay::validate_action_count(replay.actions.len())
            .map_err(|_| MoveError::ResourceLimit)?;
        if replay.game != "pyramid" {
            return Err(MoveError::WrongGame);
        }
        let mut game = Self::new(replay.seed, replay.setup);
        let deadline = Replay::<Action, Options>::reconstruction_deadline();
        for (step, action) in replay.actions.iter().enumerate() {
            Replay::<Action, Options>::check_reconstruction(deadline, step + 1)
                .map_err(|_| MoveError::ResourceLimit)?;
            game.apply(*action)?;
        }
        Ok(game)
    }

    #[must_use]
    pub fn hint(&self) -> Option<Action> {
        if self.state.is_won() {
            return None;
        }
        let sources = self.available_sources();
        if let Some(source) = sources
            .iter()
            .copied()
            .find(|&source| self.card(source).is_ok_and(|card| card.rank == Rank::King))
        {
            return Some(Action::RemoveKing(source));
        }
        for (index, first) in sources.iter().copied().enumerate() {
            for second in sources.iter().copied().skip(index + 1) {
                if self.card(first).is_ok_and(|first_card| {
                    self.card(second).is_ok_and(|second_card| {
                        first_card.rank.value() + second_card.rank.value() == 13
                    })
                }) {
                    return Some(Action::RemovePair(first, second));
                }
            }
        }
        if !self.state.stock.is_empty() {
            Some(Action::Draw)
        } else if !self.state.waste.is_empty()
            && self.state.redeals < self.state.options.max_redeals
        {
            Some(Action::Recycle)
        } else {
            None
        }
    }

    fn available_sources(&self) -> Vec<Source> {
        let mut sources = (0..PYRAMID_SIZE)
            .filter(|&index| self.state.is_exposed(index))
            .map(|index| Source::Pyramid(u8::try_from(index).unwrap_or_default()))
            .collect::<Vec<_>>();
        if !self.state.waste.is_empty() {
            sources.push(Source::Waste);
        }
        sources
    }

    fn draw(&mut self) -> Result<(), MoveError> {
        let moves = self
            .state
            .moves
            .checked_add(1)
            .ok_or(MoveError::CounterOverflow)?;
        let card = self.state.stock.pop().ok_or(MoveError::EmptyStock)?;
        self.state.waste.push(card);
        self.state.moves = moves;
        Ok(())
    }

    fn recycle(&mut self) -> Result<(), MoveError> {
        if !self.state.stock.is_empty() || self.state.waste.is_empty() {
            return Err(MoveError::CannotRecycle);
        }
        if self.state.redeals >= self.state.options.max_redeals {
            return Err(MoveError::RedealLimit);
        }
        let redeals = self
            .state
            .redeals
            .checked_add(1)
            .ok_or(MoveError::CounterOverflow)?;
        let moves = self
            .state
            .moves
            .checked_add(1)
            .ok_or(MoveError::CounterOverflow)?;
        self.state.stock.extend(self.state.waste.drain(..).rev());
        self.state.redeals = redeals;
        self.state.moves = moves;
        Ok(())
    }

    fn remove_king(&mut self, source: Source) -> Result<(), MoveError> {
        if self.card(source)?.rank != Rank::King {
            return Err(MoveError::NotKing);
        }
        let score = self
            .state
            .score
            .checked_add(10)
            .ok_or(MoveError::CounterOverflow)?;
        let moves = self
            .state
            .moves
            .checked_add(1)
            .ok_or(MoveError::CounterOverflow)?;
        self.remove(source);
        self.state.score = score;
        self.state.moves = moves;
        Ok(())
    }

    fn remove_pair(&mut self, first: Source, second: Source) -> Result<(), MoveError> {
        if first == second {
            return Err(MoveError::SameCard);
        }
        if self.card(first)?.rank.value() + self.card(second)?.rank.value() != 13 {
            return Err(MoveError::NotThirteen);
        }
        let score = self
            .state
            .score
            .checked_add(20)
            .ok_or(MoveError::CounterOverflow)?;
        let moves = self
            .state
            .moves
            .checked_add(1)
            .ok_or(MoveError::CounterOverflow)?;
        self.remove(first);
        self.remove(second);
        self.state.score = score;
        self.state.moves = moves;
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

fn push_bounded_history(history: &mut Vec<State>, state: State) {
    if history.len() == crate::replay::MAX_HISTORY_ACTIONS {
        history.remove(0);
    }
    history.push(state);
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
    ResourceLimit,
    CounterOverflow,
    GameComplete,
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
        assert_eq!(state, State::new(12, Options::default()));
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
        game.state.pyramid[23] = Some(card(Rank::Ace));
        game.state.pyramid[24] = Some(card(Rank::Two));
        game.apply(Action::RemovePair(Source::Pyramid(21), Source::Pyramid(22)))
            .unwrap();
        game.state.waste.push(card(Rank::Queen));
        game.apply(Action::RemovePair(Source::Waste, Source::Pyramid(23)))
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
        assert_eq!(game.state.score, 50);

        let before = game.clone();
        assert_eq!(
            game.apply(Action::RemoveKing(Source::Pyramid(u8::MAX))),
            Err(MoveError::CoveredCard)
        );
        assert_eq!(game, before);
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
        assert!(game.can_redo());
        assert!(game.redo());
        assert_eq!(game.state, drawn);
        assert!(game.undo());
        assert_eq!(Game::from_replay(&replay).unwrap().state, drawn);
        assert_eq!(Game::from_replay(&replay).unwrap().state.options, options);
        while !game.state.stock.is_empty() {
            game.apply(Action::Draw).unwrap();
        }
        game.state.pyramid = [None; PYRAMID_SIZE];
        game.state.pyramid[27] = Some(card(Rank::Five));
        game.state.waste = vec![card(Rank::Five)];
        assert_eq!(game.hint(), Some(Action::Recycle));
        game.apply(Action::Recycle).unwrap();
        while !game.state.stock.is_empty() {
            game.apply(Action::Draw).unwrap();
        }
        assert_eq!(game.apply(Action::Recycle), Err(MoveError::RedealLimit));
    }

    #[test]
    fn fixed_seed_hint_is_legal_and_prefers_removals() {
        let mut game = Game::new(12, Options::default());
        let hint = game.hint().unwrap();
        game.apply(hint).unwrap();

        game.state.pyramid = [None; PYRAMID_SIZE];
        game.state.pyramid[21] = Some(card(Rank::King));
        game.state.pyramid[22] = Some(card(Rank::Five));
        game.state.pyramid[23] = Some(card(Rank::Eight));
        game.state.stock = vec![card(Rank::Ace)];
        game.state.score = 0;
        assert_eq!(game.hint(), Some(Action::RemoveKing(Source::Pyramid(21))));
        game.apply(game.hint().unwrap()).unwrap();
        let pair = game.hint().unwrap();
        assert!(matches!(pair, Action::RemovePair(_, _)));
        game.apply(pair).unwrap();
        assert_eq!(game.state.score, 30);
    }

    #[test]
    fn removing_last_pyramid_card_wins_and_blocks_new_actions() {
        let mut game = Game::new(2, Options::default());
        game.state.pyramid = [None; PYRAMID_SIZE];
        game.state.pyramid[27] = Some(card(Rank::King));
        game.apply(Action::RemoveKing(Source::Pyramid(27))).unwrap();
        assert!(game.state.is_won());
        assert_eq!(game.hint(), None);

        let complete = game.clone();
        assert_eq!(game.apply(Action::Draw), Err(MoveError::GameComplete));
        assert_eq!(
            game.apply(Action::RemoveKing(Source::Pyramid(27))),
            Err(MoveError::GameComplete)
        );
        assert_eq!(game, complete);
        assert!(game.undo());
        assert!(!game.state.is_won());
        assert!(game.redo());
        assert!(game.state.is_won());
    }

    #[test]
    fn legal_seed_zero_replay_reaches_a_one_pair_near_win() {
        let envelope: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/pyramid-seed-zero-near-win.json"
        ))
        .unwrap();
        assert_eq!(envelope["version"], 1);
        assert_eq!(envelope["game"], "pyramid");
        let replay: Replay<Action, Options> =
            serde_json::from_value(envelope["payload"].clone()).unwrap();
        let mut game = Game::from_replay(&replay).unwrap();

        assert_eq!(game.state.seed, 0);
        assert_eq!(game.state.options, Options::default());
        assert_eq!(game.state.pyramid.iter().flatten().count(), 1);
        assert!(game.state.pyramid[0].is_some());
        assert!(game.state.is_exposed(0));
        assert_eq!(game.state.stock.len(), 10);
        assert_eq!(game.state.waste.len(), 1);
        assert_eq!(game.state.redeals, 2);
        assert_eq!(game.state.score, 400);
        assert_eq!(game.state.moves, 62);
        assert_eq!(game.state.card_count(), 12);
        assert_eq!(
            game.state.pyramid[0].unwrap().rank.value()
                + game.state.waste.last().unwrap().rank.value(),
            13
        );
        assert!(!game.state.is_won());

        game.apply(Action::RemovePair(Source::Pyramid(0), Source::Waste))
            .unwrap();
        assert!(game.state.is_won());
        assert_eq!(game.state.score, 420);
        assert_eq!(game.state.moves, 63);
        assert_eq!(game.state.card_count(), 10);
        assert_eq!(game.replay().actions.len(), 63);
    }

    #[test]
    fn malformed_and_oversized_replays_are_rejected() {
        let mut wrong_game = Game::new(1, Options::default()).replay();
        wrong_game.game = "freecell".into();
        assert_eq!(Game::from_replay(&wrong_game), Err(MoveError::WrongGame));

        let mut oversized = Game::new(1, Options::default()).replay();
        oversized.actions = vec![Action::Draw; crate::replay::MAX_REPLAY_ACTIONS + 1];
        assert_eq!(Game::from_replay(&oversized), Err(MoveError::ResourceLimit));
    }

    #[test]
    fn legal_play_exceeds_undo_window_and_replay_stays_bounded() {
        let mut game = Game::new(55, Options { max_redeals: 255 });
        for _ in 0..600 {
            let action = if game.state.stock.is_empty() {
                Action::Recycle
            } else {
                Action::Draw
            };
            game.apply(action).unwrap();
        }
        assert_eq!(game.undo.len(), crate::replay::MAX_HISTORY_ACTIONS);
        assert_eq!(game.replay().actions.len(), 600);
        assert_eq!(Game::from_replay(&game.replay()).unwrap().state, game.state);

        game.actions = vec![Action::Draw; crate::replay::MAX_REPLAY_ACTIONS];
        let before = game.state.clone();
        assert_eq!(game.apply(Action::Draw), Err(MoveError::ResourceLimit));
        assert_eq!(game.state, before);
    }

    #[test]
    fn counter_overflow_is_rejected_atomically() {
        let mut draw = Game::new(1, Options::default());
        draw.state.moves = u32::MAX;
        let before = draw.clone();
        assert_eq!(draw.apply(Action::Draw), Err(MoveError::CounterOverflow));
        assert_eq!(draw, before);

        let mut king = Game::new(1, Options::default());
        king.state.pyramid = [None; PYRAMID_SIZE];
        king.state.pyramid[27] = Some(card(Rank::King));
        king.state.score = u32::MAX;
        let before = king.clone();
        assert_eq!(
            king.apply(Action::RemoveKing(Source::Pyramid(27))),
            Err(MoveError::CounterOverflow)
        );
        assert_eq!(king, before);
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
