use crate::cards::{Card, shuffle, standard_deck};
use crate::replay::{CURRENT_REPLAY_VERSION, Replay};
use serde::{Deserialize, Serialize};

const TABLEAU_SIZE: usize = 28;
const CHILDREN: [(usize, usize); 18] = [
    (3, 4),
    (5, 6),
    (7, 8),
    (9, 10),
    (10, 11),
    (12, 13),
    (13, 14),
    (15, 16),
    (16, 17),
    (18, 19),
    (19, 20),
    (20, 21),
    (21, 22),
    (22, 23),
    (23, 24),
    (24, 25),
    (25, 26),
    (26, 27),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Options {
    pub wraparound: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub seed: u64,
    pub options: Options,
    pub tableau: [Option<Card>; TABLEAU_SIZE],
    pub stock: Vec<Card>,
    pub waste: Vec<Card>,
    pub streak: u32,
    pub score: u32,
    pub moves: u32,
}

impl State {
    #[must_use]
    pub fn new(seed: u64, options: Options) -> Self {
        let mut deck = standard_deck();
        shuffle(&mut deck, seed);
        let mut tableau = [None; TABLEAU_SIZE];
        for slot in &mut tableau {
            *slot = deck.pop();
        }
        let waste = deck.pop().into_iter().collect();
        Self {
            seed,
            options,
            tableau,
            stock: deck,
            waste,
            streak: 0,
            score: 0,
            moves: 0,
        }
    }

    #[must_use]
    pub fn is_exposed(&self, index: usize) -> bool {
        if index >= TABLEAU_SIZE || self.tableau[index].is_none() {
            return false;
        }
        CHILDREN.get(index).is_none_or(|&(left, right)| {
            self.tableau[left].is_none() && self.tableau[right].is_none()
        })
    }

    #[must_use]
    pub fn is_won(&self) -> bool {
        self.tableau.iter().all(Option::is_none)
    }
    #[must_use]
    pub fn card_count(&self) -> usize {
        self.tableau.iter().flatten().count() + self.stock.len() + self.waste.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Draw,
    Remove(u8),
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

    /// Applies one draw or tableau removal atomically.
    ///
    /// # Errors
    /// Returns [`MoveError`] for a covered or non-adjacent card, or empty stock.
    pub fn apply(&mut self, action: Action) -> Result<(), MoveError> {
        let before = self.state.clone();
        let result = match action {
            Action::Draw => self.draw(),
            Action::Remove(index) => self.remove(usize::from(index)),
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
    pub fn replay(&self) -> Replay<Action> {
        Replay {
            version: CURRENT_REPLAY_VERSION,
            game: "tripeaks".into(),
            seed: self.state.seed,
            actions: self.actions.clone(),
        }
    }

    /// Reconstructs a `TriPeaks` replay.
    ///
    /// # Errors
    /// Returns an error for a wrong game identifier or illegal action.
    pub fn from_replay(replay: &Replay<Action>, options: Options) -> Result<Self, MoveError> {
        if replay.game != "tripeaks" {
            return Err(MoveError::WrongGame);
        }
        let mut game = Self::new(replay.seed, options);
        for action in &replay.actions {
            game.apply(*action)?;
        }
        Ok(game)
    }

    #[must_use]
    pub fn hint(&self) -> Option<Action> {
        let waste = self.state.waste.last()?;
        (0..TABLEAU_SIZE)
            .find(|&index| {
                self.state.is_exposed(index)
                    && adjacent(
                        *waste,
                        self.state.tableau[index].unwrap_or(*waste),
                        self.state.options.wraparound,
                    )
            })
            .map(|index| Action::Remove(u8::try_from(index).unwrap_or_default()))
            .or_else(|| (!self.state.stock.is_empty()).then_some(Action::Draw))
    }

    fn draw(&mut self) -> Result<(), MoveError> {
        let card = self.state.stock.pop().ok_or(MoveError::EmptyStock)?;
        self.state.waste.push(card);
        self.state.streak = 0;
        self.state.moves += 1;
        Ok(())
    }

    fn remove(&mut self, index: usize) -> Result<(), MoveError> {
        if !self.state.is_exposed(index) {
            return Err(MoveError::CoveredCard);
        }
        let card = self.state.tableau[index].ok_or(MoveError::EmptyCard)?;
        let waste = *self.state.waste.last().ok_or(MoveError::EmptyWaste)?;
        if !adjacent(waste, card, self.state.options.wraparound) {
            return Err(MoveError::NotAdjacent);
        }
        self.state.tableau[index] = None;
        self.state.waste.push(card);
        self.state.streak += 1;
        self.state.score += self.state.streak * 100;
        self.state.moves += 1;
        Ok(())
    }
}

fn adjacent(first: Card, second: Card, wraparound: bool) -> bool {
    let difference = first.rank.value().abs_diff(second.rank.value());
    difference == 1 || (wraparound && difference == 12)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    EmptyStock,
    CoveredCard,
    EmptyCard,
    EmptyWaste,
    NotAdjacent,
    WrongGame,
}
impl std::fmt::Display for MoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal TriPeaks action: {self:?}")
    }
}
impl std::error::Error for MoveError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{Rank, Suit};
    fn card(rank: Rank) -> Card {
        Card::new(Suit::Clubs, rank)
    }

    #[test]
    fn deal_is_stable_and_bottom_row_is_exposed() {
        let state = State::new(7, Options::default());
        assert_eq!(state, State::new(7, Options::default()));
        assert_eq!(state.card_count(), 52);
        assert_eq!(state.stock.len(), 23);
        assert!((0..18).all(|index| !state.is_exposed(index)));
        assert!((18..28).all(|index| state.is_exposed(index)));
    }

    #[test]
    fn dependency_graph_exposes_cards_after_children_clear() {
        let mut state = State::new(1, Options::default());
        state.tableau[18] = None;
        state.tableau[19] = None;
        assert!(state.is_exposed(9));
        state.tableau[9] = None;
        state.tableau[20] = None;
        assert!(state.is_exposed(10));
    }

    #[test]
    fn adjacency_and_optional_wraparound_are_enforced() {
        let mut game = Game::new(1, Options::default());
        game.state.tableau = [None; 28];
        game.state.tableau[18] = Some(card(Rank::Queen));
        game.state.waste = vec![card(Rank::Jack)];
        game.apply(Action::Remove(18)).unwrap();
        assert_eq!(game.state.streak, 1);
        assert_eq!(game.state.score, 100);
        game.state.tableau[19] = Some(card(Rank::Ace));
        game.state.waste = vec![card(Rank::King)];
        let before = game.state.clone();
        assert_eq!(game.apply(Action::Remove(19)), Err(MoveError::NotAdjacent));
        assert_eq!(game.state, before);
        game.state.options.wraparound = true;
        game.apply(Action::Remove(19)).unwrap();
    }

    #[test]
    fn draw_resets_streak_and_replay_undo_restore() {
        let mut game = Game::new(91, Options::default());
        game.state.streak = 4;
        game.apply(Action::Draw).unwrap();
        assert_eq!(game.state.streak, 0);
        let replay = game.replay();
        let drawn = game.state.clone();
        assert!(game.undo());
        assert_eq!(
            Game::from_replay(&replay, Options::default())
                .unwrap()
                .state,
            drawn
        );
    }
}
