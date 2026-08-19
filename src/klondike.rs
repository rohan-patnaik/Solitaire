use crate::cards::{Card, Rank, Suit, shuffle, standard_deck};
use crate::replay::Replay;
use serde::{Deserialize, Serialize};

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
    pub fn replay(&self) -> Replay<Action> {
        Replay {
            version: crate::replay::CURRENT_REPLAY_VERSION,
            game: "klondike".into(),
            seed: self.state.seed,
            actions: self.actions.clone(),
        }
    }

    /// Reconstructs a game by validating every recorded action.
    ///
    /// # Errors
    ///
    /// Returns an error if the replay identifies another game or contains an
    /// action that is illegal for the supplied options.
    pub fn from_replay(replay: &Replay<Action>, options: Options) -> Result<Self, MoveError> {
        if replay.game != "klondike" {
            return Err(MoveError::WrongGame);
        }
        let mut game = Self::new(replay.seed, options);
        for action in &replay.actions {
            game.apply(action.clone())?;
        }
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
                if self.is_legal(&action) {
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
        self.state.moves += 1;
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
        self.state.redeals += 1;
        self.state.moves += 1;
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
        self.state.moves += 1;
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
        let mut copy = self.clone();
        copy.apply_to_state(action).is_ok()
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
                if self.is_legal(&action) {
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
            self.is_legal(&action).then_some(action)
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
}

impl std::fmt::Display for MoveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "illegal Klondike action: {self:?}")
    }
}

impl std::error::Error for MoveError {}

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
        let rebuilt = Game::from_replay(&game.replay(), options).unwrap();
        assert_eq!(rebuilt.state, game.state);
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
}
