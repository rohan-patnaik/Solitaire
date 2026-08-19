use crate::cards::{Card, Suit, shuffle, standard_deck};
use crate::replay::{CURRENT_REPLAY_VERSION, Replay};
use serde::{Deserialize, Serialize};

const CASCADE_COUNT: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub deal_number: u64,
    pub cascades: [Vec<Card>; CASCADE_COUNT],
    pub free_cells: [Option<Card>; 4],
    pub foundations: [Vec<Card>; 4],
    pub moves: u32,
}

impl State {
    #[must_use]
    pub fn new(deal_number: u64) -> Self {
        let mut deck = standard_deck();
        shuffle(&mut deck, deal_number);
        let mut cascades: [Vec<Card>; CASCADE_COUNT] = Default::default();
        for (index, card) in deck.into_iter().enumerate() {
            cascades[index % CASCADE_COUNT].push(card);
        }
        Self {
            deal_number,
            cascades,
            free_cells: Default::default(),
            foundations: Default::default(),
            moves: 0,
        }
    }

    #[must_use]
    pub fn is_won(&self) -> bool {
        self.foundations.iter().map(Vec::len).sum::<usize>() == 52
    }

    #[must_use]
    pub fn card_count(&self) -> usize {
        self.cascades.iter().map(Vec::len).sum::<usize>()
            + self.free_cells.iter().flatten().count()
            + self.foundations.iter().map(Vec::len).sum::<usize>()
    }

    /// Returns the largest ordered run that can be moved between cascades.
    #[must_use]
    pub fn supermove_capacity(&self, destination_is_empty: bool) -> usize {
        let free = self.free_cells.iter().filter(|cell| cell.is_none()).count();
        let mut empty_cascades = self.cascades.iter().filter(|pile| pile.is_empty()).count();
        if destination_is_empty {
            empty_cascades = empty_cascades.saturating_sub(1);
        }
        (free + 1) * (1_usize << empty_cascades)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pile {
    Cascade(u8),
    FreeCell(u8),
    Foundation(Suit),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    pub from: Pile,
    pub to: Pile,
    pub count: u8,
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
    pub fn new(deal_number: u64) -> Self {
        Self {
            state: State::new(deal_number),
            undo: Vec::new(),
            redo: Vec::new(),
            actions: Vec::new(),
            redo_actions: Vec::new(),
        }
    }

    /// Applies a `FreeCell` move atomically.
    ///
    /// # Errors
    ///
    /// Returns [`MoveError`] if the source, destination, run, or supermove is
    /// illegal.
    pub fn apply(&mut self, action: Action) -> Result<(), MoveError> {
        let before = self.state.clone();
        if let Err(error) = self.apply_inner(action) {
            self.state = before;
            return Err(error);
        }
        self.undo.push(before);
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
        self.undo.push(std::mem::replace(&mut self.state, next));
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
    pub fn replay(&self) -> Replay<Action> {
        Replay {
            version: CURRENT_REPLAY_VERSION,
            game: "freecell".into(),
            seed: self.state.deal_number,
            setup: (),
            actions: self.actions.clone(),
        }
    }

    /// Reconstructs a numbered deal from a replay.
    ///
    /// # Errors
    ///
    /// Returns an error for the wrong game identifier or an illegal action.
    pub fn from_replay(replay: &Replay<Action>) -> Result<Self, MoveError> {
        replay
            .validate_version()
            .map_err(|_| MoveError::UnsupportedReplayVersion(replay.version))?;
        if replay.game != "freecell" {
            return Err(MoveError::WrongGame);
        }
        let mut game = Self::new(replay.seed);
        for action in &replay.actions {
            game.apply(*action)?;
        }
        Ok(game)
    }

    #[must_use]
    pub fn hint(&self) -> Option<Action> {
        for source in 0..CASCADE_COUNT {
            if let Some(card) = self.state.cascades[source].last() {
                let action = Action {
                    from: Pile::Cascade(to_u8(source)),
                    to: Pile::Foundation(card.suit),
                    count: 1,
                };
                if self.is_legal(action) {
                    return Some(action);
                }
            }
        }
        for cell in 0..4 {
            if let Some(card) = self.state.free_cells[cell] {
                let action = Action {
                    from: Pile::FreeCell(to_u8(cell)),
                    to: Pile::Foundation(card.suit),
                    count: 1,
                };
                if self.is_legal(action) {
                    return Some(action);
                }
            }
        }
        for source in 0..CASCADE_COUNT {
            for destination in 0..CASCADE_COUNT {
                let action = Action {
                    from: Pile::Cascade(to_u8(source)),
                    to: Pile::Cascade(to_u8(destination)),
                    count: 1,
                };
                if self.is_legal(action) {
                    return Some(action);
                }
            }
        }
        None
    }

    fn is_legal(&self, action: Action) -> bool {
        let mut probe = Self {
            state: self.state.clone(),
            undo: Vec::new(),
            redo: Vec::new(),
            actions: Vec::new(),
            redo_actions: Vec::new(),
        };
        probe.apply_inner(action).is_ok()
    }

    fn apply_inner(&mut self, action: Action) -> Result<(), MoveError> {
        if action.from == action.to || action.count == 0 {
            return Err(MoveError::InvalidMove);
        }
        let cards = self.source(action.from, usize::from(action.count))?;
        self.validate_destination(action.to, &cards)?;
        self.remove(action.from, cards.len());
        self.add(action.to, &cards);
        self.state.moves += 1;
        Ok(())
    }

    fn source(&self, pile: Pile, count: usize) -> Result<Vec<Card>, MoveError> {
        match pile {
            Pile::Cascade(index) => {
                let cascade = self
                    .state
                    .cascades
                    .get(usize::from(index))
                    .ok_or(MoveError::InvalidPile)?;
                if count > cascade.len() {
                    return Err(MoveError::InvalidMove);
                }
                let cards = cascade[cascade.len() - count..].to_vec();
                if !alternating_run(&cards) {
                    return Err(MoveError::BrokenRun);
                }
                Ok(cards)
            }
            Pile::FreeCell(index) => {
                if count != 1 {
                    return Err(MoveError::InvalidMove);
                }
                Ok(vec![
                    self.state
                        .free_cells
                        .get(usize::from(index))
                        .copied()
                        .flatten()
                        .ok_or(MoveError::EmptyPile)?,
                ])
            }
            Pile::Foundation(suit) => {
                if count != 1 {
                    return Err(MoveError::InvalidMove);
                }
                Ok(vec![
                    *self.state.foundations[suit_index(suit)]
                        .last()
                        .ok_or(MoveError::EmptyPile)?,
                ])
            }
        }
    }

    fn validate_destination(&self, pile: Pile, cards: &[Card]) -> Result<(), MoveError> {
        match pile {
            Pile::Cascade(index) => {
                let cascade = self
                    .state
                    .cascades
                    .get(usize::from(index))
                    .ok_or(MoveError::InvalidPile)?;
                if cards.len() > self.state.supermove_capacity(cascade.is_empty()) {
                    return Err(MoveError::SupermoveTooLarge);
                }
                if let Some(top) = cascade.last()
                    && (top.color() == cards[0].color()
                        || top.rank.value() != cards[0].rank.value() + 1)
                {
                    return Err(MoveError::InvalidCascade);
                }
                Ok(())
            }
            Pile::FreeCell(index) => {
                if cards.len() != 1 {
                    return Err(MoveError::InvalidMove);
                }
                match self.state.free_cells.get(usize::from(index)) {
                    Some(None) => Ok(()),
                    Some(Some(_)) => Err(MoveError::OccupiedFreeCell),
                    None => Err(MoveError::InvalidPile),
                }
            }
            Pile::Foundation(suit) => {
                if cards.len() != 1 || cards[0].suit != suit {
                    return Err(MoveError::InvalidFoundation);
                }
                let expected = self.state.foundations[suit_index(suit)]
                    .last()
                    .map_or(1, |card| card.rank.value() + 1);
                (cards[0].rank.value() == expected)
                    .then_some(())
                    .ok_or(MoveError::InvalidFoundation)
            }
        }
    }

    fn remove(&mut self, pile: Pile, count: usize) {
        match pile {
            Pile::Cascade(index) => {
                let pile = &mut self.state.cascades[usize::from(index)];
                pile.truncate(pile.len() - count);
            }
            Pile::FreeCell(index) => self.state.free_cells[usize::from(index)] = None,
            Pile::Foundation(suit) => {
                self.state.foundations[suit_index(suit)].pop();
            }
        }
    }

    fn add(&mut self, pile: Pile, cards: &[Card]) {
        match pile {
            Pile::Cascade(index) => self.state.cascades[usize::from(index)].extend(cards),
            Pile::FreeCell(index) => self.state.free_cells[usize::from(index)] = Some(cards[0]),
            Pile::Foundation(suit) => self.state.foundations[suit_index(suit)].push(cards[0]),
        }
    }
}

fn alternating_run(cards: &[Card]) -> bool {
    cards.windows(2).all(|pair| {
        pair[0].color() != pair[1].color() && pair[0].rank.value() == pair[1].rank.value() + 1
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
    InvalidMove,
    InvalidPile,
    EmptyPile,
    BrokenRun,
    SupermoveTooLarge,
    InvalidCascade,
    OccupiedFreeCell,
    InvalidFoundation,
    WrongGame,
    UnsupportedReplayVersion(u16),
}

impl std::fmt::Display for MoveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "illegal FreeCell action: {self:?}")
    }
}

impl std::error::Error for MoveError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::Rank;

    fn card(suit: Suit, rank: Rank) -> Card {
        Card::new(suit, rank)
    }

    fn empty_game() -> Game {
        let mut game = Game::new(1);
        game.state.cascades = Default::default();
        game.state.free_cells = Default::default();
        game.state.foundations = Default::default();
        game
    }

    #[test]
    fn numbered_deal_is_stable_and_complete() {
        let first = State::new(617);
        assert_eq!(first, State::new(617));
        assert_ne!(first, State::new(618));
        assert_eq!(first.card_count(), 52);
        assert_eq!(
            first.cascades[..4].iter().map(Vec::len).collect::<Vec<_>>(),
            [7; 4]
        );
        assert_eq!(
            first.cascades[4..].iter().map(Vec::len).collect::<Vec<_>>(),
            [6; 4]
        );
    }

    #[test]
    fn free_cells_hold_exactly_one_card() {
        let mut game = empty_game();
        game.state.cascades[0].push(card(Suit::Spades, Rank::Ace));
        game.apply(Action {
            from: Pile::Cascade(0),
            to: Pile::FreeCell(0),
            count: 1,
        })
        .unwrap();
        game.state.cascades[1].push(card(Suit::Hearts, Rank::Two));
        assert_eq!(
            game.apply(Action {
                from: Pile::Cascade(1),
                to: Pile::FreeCell(0),
                count: 1
            }),
            Err(MoveError::OccupiedFreeCell)
        );
    }

    #[test]
    fn cascades_build_alternating_down_and_empty_accepts_any_card() {
        let mut game = empty_game();
        game.state.cascades[0] = vec![card(Suit::Hearts, Rank::Seven)];
        game.state.cascades[1] = vec![card(Suit::Spades, Rank::Six)];
        game.apply(Action {
            from: Pile::Cascade(1),
            to: Pile::Cascade(0),
            count: 1,
        })
        .unwrap();
        game.apply(Action {
            from: Pile::Cascade(0),
            to: Pile::Cascade(2),
            count: 2,
        })
        .unwrap();
        assert_eq!(game.state.cascades[2].len(), 2);
    }

    #[test]
    fn supermove_capacity_uses_empty_cells_and_columns() {
        let mut game = empty_game();
        game.state.free_cells = [
            Some(card(Suit::Clubs, Rank::Ace)),
            None,
            None,
            Some(card(Suit::Diamonds, Rank::Ace)),
        ];
        game.state.cascades[0] = vec![
            card(Suit::Hearts, Rank::Eight),
            card(Suit::Clubs, Rank::Seven),
            card(Suit::Diamonds, Rank::Six),
            card(Suit::Spades, Rank::Five),
        ];
        game.state.cascades[1] = vec![card(Suit::Clubs, Rank::Nine)];
        // Six other cascades are empty: (2 free + 1) * 2^6.
        assert_eq!(game.state.supermove_capacity(false), 192);
        game.apply(Action {
            from: Pile::Cascade(0),
            to: Pile::Cascade(1),
            count: 4,
        })
        .unwrap();
        assert_eq!(game.state.cascades[1].len(), 5);
    }

    #[test]
    fn foundation_requires_ascending_suit() {
        let mut game = empty_game();
        game.state.cascades[0] = vec![card(Suit::Hearts, Rank::Two)];
        assert_eq!(
            game.apply(Action {
                from: Pile::Cascade(0),
                to: Pile::Foundation(Suit::Hearts),
                count: 1
            }),
            Err(MoveError::InvalidFoundation)
        );
        game.state.cascades[0] = vec![card(Suit::Hearts, Rank::Ace)];
        game.apply(Action {
            from: Pile::Cascade(0),
            to: Pile::Foundation(Suit::Hearts),
            count: 1,
        })
        .unwrap();
    }

    #[test]
    fn invalid_move_is_atomic_and_replay_restores() {
        let mut game = Game::new(99);
        let before = game.state.clone();
        assert!(
            game.apply(Action {
                from: Pile::Cascade(0),
                to: Pile::FreeCell(0),
                count: 2
            })
            .is_err()
        );
        assert_eq!(game.state, before);
        game.apply(Action {
            from: Pile::Cascade(0),
            to: Pile::FreeCell(0),
            count: 1,
        })
        .unwrap();
        let replay = game.replay();
        assert_eq!(Game::from_replay(&replay).unwrap().state, game.state);
        assert!(game.can_undo());
        assert!(game.undo());
        assert_eq!(game.state, before);
        assert!(game.can_redo());
        assert!(game.redo());
        assert_eq!(Game::from_replay(&replay).unwrap().state, game.state);
    }

    #[test]
    fn typed_replay_with_wrong_version_is_rejected() {
        let mut replay = Game::new(1).replay();
        replay.version = 1;
        assert_eq!(
            Game::from_replay(&replay),
            Err(MoveError::UnsupportedReplayVersion(1))
        );
    }
}
