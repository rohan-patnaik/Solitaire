use crate::cards::{Card, Color, Rank, Suit, shuffle, standard_deck};
use crate::replay::Replay;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const TABLEAU_COUNT: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrawMode {
    One,
    Three,
}

impl DrawMode {
    const fn count(self) -> usize {
        match self {
            Self::One => 1,
            Self::Three => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scoring {
    Standard,
    Vegas,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Options {
    pub draw_mode: DrawMode,
    pub scoring: Scoring,
    /// `None` allows unlimited passes through the stock.
    pub max_redeals: Option<u8>,
    pub timed: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            draw_mode: DrawMode::One,
            scoring: Scoring::Standard,
            max_redeals: None,
            timed: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableauCard {
    pub card: Card,
    pub face_up: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub seed: u64,
    pub options: Options,
    pub tableau: [Vec<TableauCard>; TABLEAU_COUNT],
    /// The end of the vector is the top of the stock.
    pub stock: Vec<Card>,
    /// The end of the vector is the visible waste card.
    pub waste: Vec<Card>,
    /// Foundations are ordered clubs, diamonds, hearts, spades.
    pub foundations: [Vec<Card>; 4],
    pub redeals: u8,
    pub score: i32,
    pub moves: u32,
    pub elapsed_seconds: u64,
}

impl State {
    #[must_use]
    pub fn new(seed: u64, options: Options) -> Self {
        let mut deck = standard_deck();
        shuffle(&mut deck, seed);
        let mut tableau: [Vec<TableauCard>; TABLEAU_COUNT] = Default::default();
        for (column, pile) in tableau.iter_mut().enumerate() {
            for row in 0..=column {
                let card = deck.pop().unwrap_or(Card::new(Suit::Clubs, Rank::Ace));
                pile.push(TableauCard {
                    card,
                    face_up: row == column,
                });
            }
        }
        Self {
            seed,
            options,
            tableau,
            stock: deck,
            waste: Vec::new(),
            foundations: Default::default(),
            redeals: 0,
            score: match options.scoring {
                Scoring::Standard => 0,
                Scoring::Vegas => -52,
            },
            moves: 0,
            elapsed_seconds: 0,
        }
    }

    #[must_use]
    pub fn is_won(&self) -> bool {
        self.foundations.iter().map(Vec::len).sum::<usize>() == 52
    }

    #[must_use]
    pub fn card_count(&self) -> usize {
        self.stock.len()
            + self.waste.len()
            + self.tableau.iter().map(Vec::len).sum::<usize>()
            + self.foundations.iter().map(Vec::len).sum::<usize>()
    }

    pub fn advance_time(&mut self, seconds: u64) {
        if self.options.timed && !self.is_won() {
            self.elapsed_seconds = self.elapsed_seconds.saturating_add(seconds);
        }
    }

    fn score(&mut self, standard_points: i32, vegas_points: i32) {
        self.score += match self.options.scoring {
            Scoring::Standard => standard_points,
            Scoring::Vegas => vegas_points,
        };
    }

    fn flip_exposed(&mut self, column: usize) {
        if let Some(card) = self.tableau[column].last_mut()
            && !card.face_up
        {
            card.face_up = true;
            self.score(5, 0);
        }
    }

    /// Checks structural, card-conservation, and pile-order invariants.
    ///
    /// # Errors
    /// Returns [`ValidationError`] if this state could not result from legal play.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if !self.options.timed && self.elapsed_seconds != 0 {
            return Err(ValidationError::ElapsedTimeInUntimedGame);
        }
        if self.card_count() != 52 {
            return Err(ValidationError::CardCount(self.card_count()));
        }
        let cards = self
            .stock
            .iter()
            .chain(&self.waste)
            .copied()
            .chain(self.tableau.iter().flatten().map(|entry| entry.card))
            .chain(self.foundations.iter().flatten().copied())
            .collect::<HashSet<_>>();
        if cards.len() != 52 {
            return Err(ValidationError::DuplicateCard);
        }
        for column in &self.tableau {
            let mut saw_face_up = false;
            let mut previous_face_up: Option<Card> = None;
            for entry in column {
                if entry.face_up {
                    saw_face_up = true;
                    if let Some(previous) = previous_face_up
                        && (previous.color() == entry.card.color()
                            || previous.rank.value() != entry.card.rank.value() + 1)
                    {
                        return Err(ValidationError::InvalidTableauRun);
                    }
                    previous_face_up = Some(entry.card);
                } else if saw_face_up {
                    return Err(ValidationError::FaceDownAboveFaceUp);
                }
            }
            if column.last().is_some_and(|entry| !entry.face_up) {
                return Err(ValidationError::NoExposedTableauCard);
            }
        }
        if self
            .options
            .max_redeals
            .is_some_and(|maximum| self.redeals > maximum)
        {
            return Err(ValidationError::RedealCounterExceedsLimit);
        }
        for (index, foundation) in self.foundations.iter().enumerate() {
            for (rank_index, card) in foundation.iter().enumerate() {
                if suit_index(card.suit) != index
                    || usize::from(card.rank.value()) != rank_index + 1
                {
                    return Err(ValidationError::FoundationOrder);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pile {
    Waste,
    Tableau(u8),
    Foundation(Suit),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Draw,
    Recycle,
    Move { from: Pile, to: Pile, count: u8 },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplaySetup {
    pub options: Options,
    pub elapsed_seconds: u64,
}

impl ReplaySetup {
    fn validate(self) -> Result<(), MoveError> {
        if !self.options.timed && self.elapsed_seconds != 0 {
            return Err(MoveError::InvalidReplaySetup);
        }
        Ok(())
    }
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

    /// Applies one validated action atomically.
    ///
    /// # Errors
    ///
    /// Returns [`MoveError`] when the requested action is illegal. The game is
    /// unchanged on error.
    pub fn apply(&mut self, action: Action) -> Result<(), MoveError> {
        if self.actions.len() >= crate::replay::MAX_HISTORY_ACTIONS {
            return Err(MoveError::ResourceLimit);
        }
        let before = self.state.clone();
        if let Err(error) = self.apply_to_state(&action) {
            self.state = before;
            return Err(error);
        }
        self.undo.push(before);
        self.redo.clear();
        self.redo_actions.clear();
        self.actions.push(action);
        Ok(())
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
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
        self.undo.push(std::mem::replace(&mut self.state, next));
        if let Some(action) = self.redo_actions.pop() {
            self.actions.push(action);
        }
        true
    }

    #[must_use]
    pub fn replay(&self) -> Replay<Action, ReplaySetup> {
        Replay {
            version: crate::replay::CURRENT_REPLAY_VERSION,
            game: "klondike".into(),
            seed: self.state.seed,
            setup: ReplaySetup {
                options: self.state.options,
                elapsed_seconds: self.state.elapsed_seconds,
            },
            actions: self.actions.clone(),
        }
    }

    /// Reconstructs a game by validating every recorded action.
    ///
    /// # Errors
    ///
    /// Returns an error if the replay identifies another game or contains an
    /// action that is illegal for the recorded options.
    pub fn from_replay(replay: &Replay<Action, ReplaySetup>) -> Result<Self, MoveError> {
        crate::replay::validate_version(replay.version)
            .map_err(|_| MoveError::UnsupportedReplayVersion(replay.version))?;
        crate::replay::validate_action_count(replay.actions.len())
            .map_err(|_| MoveError::ResourceLimit)?;
        if replay.game != "klondike" {
            return Err(MoveError::WrongGame);
        }
        replay.setup.validate()?;
        let mut game = Self::new(replay.seed, replay.setup.options);
        let deadline = crate::replay::Replay::<Action, ReplaySetup>::reconstruction_deadline();
        for (step, action) in replay.actions.iter().enumerate() {
            crate::replay::Replay::<Action, ReplaySetup>::check_reconstruction(deadline, step + 1)
                .map_err(|_| MoveError::ResourceLimit)?;
            game.apply(action.clone())?;
        }
        game.state.elapsed_seconds = replay.setup.elapsed_seconds;
        Ok(game)
    }

    /// Serializes the complete resumable game, including undo history.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// Reads a complete resumable game.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed JSON.
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    /// Validates the current state and every state reachable through undo/redo.
    ///
    /// # Errors
    /// Returns [`ValidationError`] when any serialized state violates invariants.
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.state.validate()?;
        if self.undo.len() != self.actions.len() {
            return Err(ValidationError::UndoActionCardinality);
        }
        if self.redo.len() != self.redo_actions.len() {
            return Err(ValidationError::RedoActionCardinality);
        }
        if self.actions.len().saturating_add(self.redo_actions.len())
            > crate::replay::MAX_HISTORY_ACTIONS
        {
            return Err(ValidationError::HistoryLimit);
        }
        for state in self.undo.iter().chain(&self.redo) {
            state.validate()?;
        }
        let mut cursor = Self::new(self.state.seed, self.state.options);
        for (action, expected_before) in self.actions.iter().zip(&self.undo) {
            if !states_equivalent(&cursor.state, expected_before) {
                return Err(ValidationError::InvalidUndoTransition);
            }
            cursor
                .apply_to_state(action)
                .map_err(|_| ValidationError::IllegalHistoryAction)?;
        }
        if !states_equivalent(&cursor.state, &self.state) {
            return Err(ValidationError::CurrentStateDoesNotMatchActions);
        }
        cursor.state = self.state.clone();
        for (action, expected_after) in self.redo_actions.iter().rev().zip(self.redo.iter().rev()) {
            cursor
                .apply_to_state(action)
                .map_err(|_| ValidationError::IllegalHistoryAction)?;
            if !states_equivalent(&cursor.state, expected_after) {
                return Err(ValidationError::InvalidRedoTransition);
            }
        }
        Ok(())
    }

    /// Returns a deterministic, immediately legal suggestion.
    #[must_use]
    pub fn hint(&self) -> Option<Action> {
        for column in 0..TABLEAU_COUNT {
            if let Some(card) = self.state.tableau[column].last()
                && card.face_up
            {
                let action = Action::Move {
                    from: Pile::Tableau(to_u8(column)),
                    to: Pile::Foundation(card.card.suit),
                    count: 1,
                };
                if self.is_legal(&action) && self.safe_for_foundation(card.card) {
                    return Some(action);
                }
            }
        }
        if let Some(card) = self.state.waste.last() {
            let action = Action::Move {
                from: Pile::Waste,
                to: Pile::Foundation(card.suit),
                count: 1,
            };
            if self.is_legal(&action) {
                return Some(action);
            }
        }
        if !self.state.stock.is_empty() {
            return Some(Action::Draw);
        }
        if !self.state.waste.is_empty() && self.can_recycle() {
            return Some(Action::Recycle);
        }
        self.first_tableau_move()
    }

    /// Moves every currently available card to a foundation until no progress
    /// is possible, returning the number of moves made.
    pub fn autocomplete(&mut self) -> usize {
        let mut completed = 0;
        while let Some(action) = self.foundation_hint() {
            if self.apply(action).is_err() {
                break;
            }
            completed += 1;
        }
        completed
    }

    fn apply_to_state(&mut self, action: &Action) -> Result<(), MoveError> {
        match action {
            Action::Draw => self.draw(),
            Action::Recycle => self.recycle(),
            Action::Move { from, to, count } => self.move_cards(*from, *to, usize::from(*count)),
        }
    }

    fn draw(&mut self) -> Result<(), MoveError> {
        if self.state.stock.is_empty() {
            return Err(MoveError::EmptyStock);
        }
        for _ in 0..self.state.options.draw_mode.count() {
            let Some(card) = self.state.stock.pop() else {
                break;
            };
            self.state.waste.push(card);
        }
        self.state.moves = self
            .state
            .moves
            .checked_add(1)
            .ok_or(MoveError::CounterOverflow)?;
        Ok(())
    }

    fn can_recycle(&self) -> bool {
        self.state
            .options
            .max_redeals
            .is_none_or(|maximum| self.state.redeals < maximum)
    }

    fn recycle(&mut self) -> Result<(), MoveError> {
        if !self.state.stock.is_empty() || self.state.waste.is_empty() {
            return Err(MoveError::CannotRecycle);
        }
        if !self.can_recycle() {
            return Err(MoveError::RedealLimitReached);
        }
        self.state.stock.extend(self.state.waste.drain(..).rev());
        self.state.redeals = self
            .state
            .redeals
            .checked_add(1)
            .ok_or(MoveError::CounterOverflow)?;
        self.state.moves = self
            .state
            .moves
            .checked_add(1)
            .ok_or(MoveError::CounterOverflow)?;
        self.state.score(-100, 0);
        Ok(())
    }

    fn move_cards(&mut self, from: Pile, to: Pile, count: usize) -> Result<(), MoveError> {
        if from == to || count == 0 {
            return Err(MoveError::InvalidCount);
        }
        let cards = self.source_cards(from, count)?;
        self.validate_destination(to, &cards)?;
        self.remove_source(from, count);
        self.add_destination(to, &cards);
        self.apply_move_score(from, to);
        if let Pile::Tableau(column) = from {
            self.state.flip_exposed(usize::from(column));
        }
        self.state.moves = self
            .state
            .moves
            .checked_add(1)
            .ok_or(MoveError::CounterOverflow)?;
        Ok(())
    }

    fn source_cards(&self, from: Pile, count: usize) -> Result<Vec<Card>, MoveError> {
        match from {
            Pile::Waste => {
                if count != 1 {
                    return Err(MoveError::InvalidCount);
                }
                Ok(vec![*self.state.waste.last().ok_or(MoveError::EmptyPile)?])
            }
            Pile::Foundation(suit) => {
                if count != 1 {
                    return Err(MoveError::InvalidCount);
                }
                Ok(vec![
                    *self.foundation(suit).last().ok_or(MoveError::EmptyPile)?,
                ])
            }
            Pile::Tableau(column) => {
                let pile = self.tableau(column)?;
                if count > pile.len() {
                    return Err(MoveError::InvalidCount);
                }
                let selected = &pile[pile.len() - count..];
                if selected.iter().any(|card| !card.face_up) {
                    return Err(MoveError::FaceDownCard);
                }
                let cards = selected.iter().map(|card| card.card).collect::<Vec<_>>();
                if !valid_tableau_run(&cards) {
                    return Err(MoveError::BrokenRun);
                }
                Ok(cards)
            }
        }
    }

    fn validate_destination(&self, to: Pile, cards: &[Card]) -> Result<(), MoveError> {
        match to {
            Pile::Waste => Err(MoveError::InvalidDestination),
            Pile::Foundation(suit) => {
                if cards.len() != 1 || cards[0].suit != suit {
                    return Err(MoveError::InvalidFoundation);
                }
                let expected = self
                    .foundation(suit)
                    .last()
                    .map_or(1, |card| card.rank.value() + 1);
                if cards[0].rank.value() != expected {
                    return Err(MoveError::InvalidFoundation);
                }
                Ok(())
            }
            Pile::Tableau(column) => {
                let destination = self.tableau(column)?;
                match destination.last() {
                    None if cards[0].rank == Rank::King => Ok(()),
                    Some(top)
                        if top.face_up
                            && top.card.color() != cards[0].color()
                            && top.card.rank.descending_from(cards[0].rank) =>
                    {
                        Ok(())
                    }
                    _ => Err(MoveError::InvalidTableau),
                }
            }
        }
    }

    fn remove_source(&mut self, from: Pile, count: usize) {
        match from {
            Pile::Waste => {
                self.state.waste.pop();
            }
            Pile::Foundation(suit) => {
                self.foundation_mut(suit).pop();
            }
            Pile::Tableau(column) => {
                let length = self.state.tableau[usize::from(column)].len();
                self.state.tableau[usize::from(column)].truncate(length - count);
            }
        }
    }

    fn add_destination(&mut self, to: Pile, cards: &[Card]) {
        match to {
            Pile::Foundation(suit) => self.foundation_mut(suit).push(cards[0]),
            Pile::Tableau(column) => {
                self.state.tableau[usize::from(column)].extend(cards.iter().copied().map(|card| {
                    TableauCard {
                        card,
                        face_up: true,
                    }
                }));
            }
            Pile::Waste => unreachable!("waste destination was rejected"),
        }
    }

    fn apply_move_score(&mut self, from: Pile, to: Pile) {
        match (from, to) {
            (Pile::Waste, Pile::Tableau(_)) => self.state.score(5, 0),
            (Pile::Foundation(_), Pile::Tableau(_)) => self.state.score(-15, -5),
            (_, Pile::Foundation(_)) => self.state.score(10, 5),
            _ => {}
        }
    }

    fn tableau(&self, column: u8) -> Result<&Vec<TableauCard>, MoveError> {
        self.state
            .tableau
            .get(usize::from(column))
            .ok_or(MoveError::InvalidColumn)
    }

    fn foundation(&self, suit: Suit) -> &Vec<Card> {
        &self.state.foundations[suit_index(suit)]
    }

    fn foundation_mut(&mut self, suit: Suit) -> &mut Vec<Card> {
        &mut self.state.foundations[suit_index(suit)]
    }

    fn is_legal(&self, action: &Action) -> bool {
        let mut probe = Self {
            state: self.state.clone(),
            undo: Vec::new(),
            redo: Vec::new(),
            actions: Vec::new(),
            redo_actions: Vec::new(),
        };
        probe.apply_to_state(action).is_ok()
    }

    fn foundation_hint(&self) -> Option<Action> {
        for column in 0..TABLEAU_COUNT {
            if let Some(card) = self.state.tableau[column].last()
                && card.face_up
            {
                let action = Action::Move {
                    from: Pile::Tableau(to_u8(column)),
                    to: Pile::Foundation(card.card.suit),
                    count: 1,
                };
                if self.is_legal(&action) && self.safe_for_foundation(card.card) {
                    return Some(action);
                }
            }
        }
        self.state.waste.last().and_then(|card| {
            let action = Action::Move {
                from: Pile::Waste,
                to: Pile::Foundation(card.suit),
                count: 1,
            };
            (self.is_legal(&action) && self.safe_for_foundation(*card)).then_some(action)
        })
    }

    fn safe_for_foundation(&self, card: Card) -> bool {
        let rank = card.rank.value();
        if rank <= Rank::Two.value() {
            return true;
        }
        let opposite = match card.color() {
            Color::Red => [Suit::Clubs, Suit::Spades],
            Color::Black => [Suit::Diamonds, Suit::Hearts],
        };
        opposite.into_iter().all(|suit| {
            self.state.foundations[suit_index(suit)]
                .last()
                .is_some_and(|foundation| foundation.rank.value() >= rank - 1)
        })
    }

    fn first_tableau_move(&self) -> Option<Action> {
        for source in 0..TABLEAU_COUNT {
            let Some(face_up) = self.state.tableau[source]
                .iter()
                .position(|card| card.face_up)
            else {
                continue;
            };
            let count = self.state.tableau[source].len() - face_up;
            for destination in 0..TABLEAU_COUNT {
                let action = Action::Move {
                    from: Pile::Tableau(to_u8(source)),
                    to: Pile::Tableau(to_u8(destination)),
                    count: to_u8(count),
                };
                if self.is_legal(&action) {
                    return Some(action);
                }
            }
        }
        None
    }
}

fn valid_tableau_run(cards: &[Card]) -> bool {
    cards.windows(2).all(|pair| {
        pair[0].color() != pair[1].color() && pair[0].rank.descending_from(pair[1].rank)
    })
}

fn states_equivalent(first: &State, second: &State) -> bool {
    let mut first = first.clone();
    let mut second = second.clone();
    if first.options.timed && second.options.timed {
        first.elapsed_seconds = 0;
        second.elapsed_seconds = 0;
    }
    first == second
}

const fn suit_index(suit: Suit) -> usize {
    match suit {
        Suit::Clubs => 0,
        Suit::Diamonds => 1,
        Suit::Hearts => 2,
        Suit::Spades => 3,
    }
}

fn to_u8(value: usize) -> u8 {
    u8::try_from(value).unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    EmptyStock,
    CannotRecycle,
    RedealLimitReached,
    InvalidCount,
    EmptyPile,
    FaceDownCard,
    BrokenRun,
    InvalidDestination,
    InvalidFoundation,
    InvalidTableau,
    InvalidColumn,
    WrongGame,
    UnsupportedReplayVersion(u16),
    InvalidReplaySetup,
    ResourceLimit,
    CounterOverflow,
}

impl std::fmt::Display for MoveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "illegal Klondike action: {self:?}")
    }
}

impl std::error::Error for MoveError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    CardCount(usize),
    DuplicateCard,
    FaceDownAboveFaceUp,
    NoExposedTableauCard,
    InvalidTableauRun,
    FoundationOrder,
    RedealCounterExceedsLimit,
    ElapsedTimeInUntimedGame,
    UndoActionCardinality,
    RedoActionCardinality,
    InvalidUndoTransition,
    InvalidRedoTransition,
    IllegalHistoryAction,
    CurrentStateDoesNotMatchActions,
    HistoryLimit,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid Klondike save state: {self:?}")
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn card(suit: Suit, rank: Rank) -> Card {
        Card::new(suit, rank)
    }

    fn empty_game(options: Options) -> Game {
        let mut game = Game::new(1, options);
        game.state.tableau = Default::default();
        game.state.stock.clear();
        game.state.waste.clear();
        game.state.foundations = Default::default();
        game.state.score = 0;
        game
    }

    #[test]
    fn deal_has_standard_shape_and_all_cards() {
        let state = State::new(2026, Options::default());
        assert_eq!(state.stock.len(), 24);
        assert_eq!(state.card_count(), 52);
        for (index, column) in state.tableau.iter().enumerate() {
            assert_eq!(column.len(), index + 1);
            assert!(column.last().unwrap().face_up);
            assert!(column[..index].iter().all(|card| !card.face_up));
        }
        let cards = state
            .stock
            .iter()
            .copied()
            .chain(state.tableau.iter().flatten().map(|card| card.card))
            .collect::<HashSet<_>>();
        assert_eq!(cards.len(), 52);
    }

    #[test]
    fn autocomplete_does_not_strand_opposite_color_tableau_builders() {
        let mut game = empty_game(Options::default());
        game.state.foundations[suit_index(Suit::Hearts)] =
            vec![card(Suit::Hearts, Rank::Ace), card(Suit::Hearts, Rank::Two)];
        game.state.tableau[0].push(TableauCard {
            card: card(Suit::Hearts, Rank::Three),
            face_up: true,
        });
        assert_eq!(game.autocomplete(), 0);

        for suit in [Suit::Clubs, Suit::Spades] {
            game.state.foundations[suit_index(suit)] =
                vec![card(suit, Rank::Ace), card(suit, Rank::Two)];
        }
        assert_eq!(game.autocomplete(), 1);
    }

    #[test]
    fn move_counter_overflow_is_rejected_atomically() {
        let mut game = Game::new(10, Options::default());
        game.state.moves = u32::MAX;
        let before = game.state.clone();
        assert_eq!(game.apply(Action::Draw), Err(MoveError::CounterOverflow));
        assert_eq!(game.state, before);
    }

    #[test]
    fn bounded_history_rejects_an_additional_action_before_mutation() {
        let mut game = Game::new(11, Options::default());
        game.actions = vec![Action::Draw; crate::replay::MAX_HISTORY_ACTIONS];
        let before = game.state.clone();
        assert_eq!(game.apply(Action::Draw), Err(MoveError::ResourceLimit));
        assert_eq!(game.state, before);
    }

    #[test]
    fn draw_modes_and_recycle_preserve_order() {
        let mut one = Game::new(9, Options::default());
        let original_top = *one.state.stock.last().unwrap();
        one.apply(Action::Draw).unwrap();
        assert_eq!(one.state.waste, [original_top]);

        let mut three = Game::new(
            9,
            Options {
                draw_mode: DrawMode::Three,
                max_redeals: Some(1),
                ..Options::default()
            },
        );
        while !three.state.stock.is_empty() {
            three.apply(Action::Draw).unwrap();
        }
        let first_pass = three.state.waste.clone();
        three.apply(Action::Recycle).unwrap();
        assert_eq!(
            three.state.stock.iter().rev().copied().collect::<Vec<_>>(),
            first_pass
        );
        assert_eq!(three.apply(Action::Recycle), Err(MoveError::CannotRecycle));
        while !three.state.stock.is_empty() {
            three.apply(Action::Draw).unwrap();
        }
        assert_eq!(
            three.apply(Action::Recycle),
            Err(MoveError::RedealLimitReached)
        );
    }

    #[test]
    fn tableau_requires_alternating_descending_runs_and_kings_on_spaces() {
        let mut game = empty_game(Options::default());
        game.state.tableau[0] = vec![TableauCard {
            card: card(Suit::Spades, Rank::Eight),
            face_up: true,
        }];
        game.state.tableau[1] = vec![
            TableauCard {
                card: card(Suit::Hearts, Rank::Seven),
                face_up: true,
            },
            TableauCard {
                card: card(Suit::Clubs, Rank::Six),
                face_up: true,
            },
        ];
        game.apply(Action::Move {
            from: Pile::Tableau(1),
            to: Pile::Tableau(0),
            count: 2,
        })
        .unwrap();
        assert_eq!(game.state.tableau[0].len(), 3);

        game.state.waste.push(card(Suit::Hearts, Rank::Queen));
        assert_eq!(
            game.apply(Action::Move {
                from: Pile::Waste,
                to: Pile::Tableau(2),
                count: 1,
            }),
            Err(MoveError::InvalidTableau)
        );
        game.state.waste.pop();
        game.state.waste.push(card(Suit::Clubs, Rank::King));
        game.apply(Action::Move {
            from: Pile::Waste,
            to: Pile::Tableau(2),
            count: 1,
        })
        .unwrap();
    }

    #[test]
    fn foundations_are_suit_ascending_and_flip_exposed_cards() {
        let mut game = empty_game(Options::default());
        game.state.tableau[0] = vec![
            TableauCard {
                card: card(Suit::Clubs, Rank::King),
                face_up: false,
            },
            TableauCard {
                card: card(Suit::Hearts, Rank::Ace),
                face_up: true,
            },
        ];
        game.apply(Action::Move {
            from: Pile::Tableau(0),
            to: Pile::Foundation(Suit::Hearts),
            count: 1,
        })
        .unwrap();
        assert!(game.state.tableau[0][0].face_up);
        assert_eq!(game.state.score, 15);
        game.state.waste.push(card(Suit::Hearts, Rank::Three));
        let before = game.state.clone();
        assert_eq!(
            game.apply(Action::Move {
                from: Pile::Waste,
                to: Pile::Foundation(Suit::Hearts),
                count: 1,
            }),
            Err(MoveError::InvalidFoundation)
        );
        assert_eq!(game.state, before);
    }

    #[test]
    fn undo_redo_save_and_replay_restore_state() {
        let options = Options::default();
        let mut game = Game::new(77, options);
        game.apply(Action::Draw).unwrap();
        let after_draw = game.state.clone();
        assert!(game.undo());
        assert!(game.can_redo());
        assert!(game.redo());
        assert_eq!(game.state, after_draw);

        let saved = game.to_json().unwrap();
        assert_eq!(Game::from_json(&saved).unwrap(), game);
        let rebuilt = Game::from_replay(&game.replay()).unwrap();
        assert_eq!(rebuilt.state, game.state);
    }

    #[test]
    fn replay_preserves_variant_and_timed_elapsed_state() {
        let options = Options {
            draw_mode: DrawMode::Three,
            scoring: Scoring::Vegas,
            max_redeals: Some(1),
            timed: true,
        };
        let mut game = Game::new(314, options);
        game.apply(Action::Draw).unwrap();
        game.state.advance_time(37);
        let rebuilt = Game::from_replay(&game.replay()).unwrap();
        assert_eq!(rebuilt.state, game.state);
        assert_eq!(rebuilt.state.options, options);
    }

    #[test]
    fn typed_replay_with_wrong_version_is_rejected() {
        let game = Game::new(1, Options::default());
        let mut replay = game.replay();
        replay.version = 1;
        assert_eq!(
            Game::from_replay(&replay),
            Err(MoveError::UnsupportedReplayVersion(1))
        );
    }

    #[test]
    fn untimed_replay_with_elapsed_time_is_rejected_from_typed_and_json_values() {
        let game = Game::new(1, Options::default());
        let mut replay = game.replay();
        replay.setup.elapsed_seconds = 1;
        assert_eq!(
            Game::from_replay(&replay),
            Err(MoveError::InvalidReplaySetup)
        );

        let json = replay.to_json().unwrap();
        let decoded = Replay::<Action, ReplaySetup>::from_json(&json).unwrap();
        assert_eq!(
            Game::from_replay(&decoded),
            Err(MoveError::InvalidReplaySetup)
        );
    }

    #[test]
    fn every_legal_action_preserves_cards() {
        for seed in 0..128 {
            let mut game = Game::new(seed, Options::default());
            for _ in 0..24 {
                let action = game.hint().expect("a new deal has a draw hint");
                game.apply(action).unwrap();
                assert_eq!(game.state.card_count(), 52);
            }
        }
    }

    #[test]
    fn state_only_legality_matches_full_game_probe() {
        let mut game = Game::new(5, Options::default());
        for _ in 0..12 {
            game.apply(Action::Draw).unwrap();
        }
        let candidates = [
            Action::Draw,
            Action::Recycle,
            Action::Move {
                from: Pile::Waste,
                to: Pile::Tableau(0),
                count: 1,
            },
        ];
        for action in candidates {
            let mut full = game.clone();
            assert_eq!(game.is_legal(&action), full.apply_to_state(&action).is_ok());
        }
    }

    #[test]
    fn vegas_scoring_starts_in_debt_and_only_pays_foundations() {
        let mut game = empty_game(Options {
            scoring: Scoring::Vegas,
            ..Options::default()
        });
        game.state.score = -52;
        game.state.waste.push(card(Suit::Clubs, Rank::Ace));
        game.apply(Action::Move {
            from: Pile::Waste,
            to: Pile::Foundation(Suit::Clubs),
            count: 1,
        })
        .unwrap();
        assert_eq!(game.state.score, -47);
    }

    #[test]
    fn timer_only_advances_when_enabled() {
        let mut untimed = State::new(1, Options::default());
        untimed.advance_time(10);
        assert_eq!(untimed.elapsed_seconds, 0);
        let mut timed = State::new(
            1,
            Options {
                timed: true,
                ..Options::default()
            },
        );
        timed.advance_time(10);
        assert_eq!(timed.elapsed_seconds, 10);
    }

    #[test]
    fn validation_rejects_duplicate_and_malformed_piles() {
        let mut duplicate = State::new(1, Options::default());
        duplicate.stock[0] = duplicate.stock[1];
        assert_eq!(duplicate.validate(), Err(ValidationError::DuplicateCard));

        let mut malformed = State::new(2, Options::default());
        malformed.tableau[1][0].face_up = true;
        malformed.tableau[1][1].face_up = false;
        assert_eq!(
            malformed.validate(),
            Err(ValidationError::FaceDownAboveFaceUp)
        );
    }

    #[test]
    fn validation_rejects_rule_and_history_corruption() {
        let mut untimed = Game::new(8, Options::default());
        untimed.state.elapsed_seconds = 1;
        assert_eq!(
            untimed.validate(),
            Err(ValidationError::ElapsedTimeInUntimedGame)
        );

        let mut redeals = Game::new(
            9,
            Options {
                max_redeals: Some(1),
                ..Options::default()
            },
        );
        redeals.state.redeals = 2;
        assert_eq!(
            redeals.validate(),
            Err(ValidationError::RedealCounterExceedsLimit)
        );

        let mut history = Game::new(10, Options::default());
        history.apply(Action::Draw).unwrap();
        history.actions.clear();
        assert_eq!(
            history.validate(),
            Err(ValidationError::UndoActionCardinality)
        );

        let mut tableau = Game::new(11, Options::default());
        tableau.state.tableau[1][0].face_up = true;
        assert_eq!(
            tableau.state.validate(),
            Err(ValidationError::InvalidTableauRun)
        );
    }
}
