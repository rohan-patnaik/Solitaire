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

    /// Applies one draw or tableau removal atomically.
    ///
    /// # Errors
    /// Returns [`MoveError`] for a covered or non-adjacent card, or empty stock.
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
            Action::Remove(index) => self.remove(usize::from(index)),
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
            game: "tripeaks".into(),
            seed: self.state.seed,
            setup: self.state.options,
            actions: self.actions.clone(),
        }
    }

    /// Reconstructs a `TriPeaks` replay.
    ///
    /// # Errors
    /// Returns an error for a wrong game identifier or illegal action.
    pub fn from_replay(replay: &Replay<Action, Options>) -> Result<Self, MoveError> {
        crate::replay::validate_version(replay.version)
            .map_err(|_| MoveError::UnsupportedReplayVersion(replay.version))?;
        crate::replay::validate_action_count(replay.actions.len())
            .map_err(|_| MoveError::ResourceLimit)?;
        if replay.game != "tripeaks" {
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
        let moves = self
            .state
            .moves
            .checked_add(1)
            .ok_or(MoveError::CounterOverflow)?;
        let card = self.state.stock.pop().ok_or(MoveError::EmptyStock)?;
        self.state.waste.push(card);
        self.state.streak = 0;
        self.state.moves = moves;
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
        let streak = self
            .state
            .streak
            .checked_add(1)
            .ok_or(MoveError::CounterOverflow)?;
        let score = streak
            .checked_mul(100)
            .and_then(|points| self.state.score.checked_add(points))
            .ok_or(MoveError::CounterOverflow)?;
        let moves = self
            .state
            .moves
            .checked_add(1)
            .ok_or(MoveError::CounterOverflow)?;
        self.state.tableau[index] = None;
        self.state.waste.push(card);
        self.state.streak = streak;
        self.state.score = score;
        self.state.moves = moves;
        Ok(())
    }
}

fn push_bounded_history(history: &mut Vec<State>, state: State) {
    if history.len() == crate::replay::MAX_HISTORY_ACTIONS {
        history.remove(0);
    }
    history.push(state);
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
    UnsupportedReplayVersion(u16),
    ResourceLimit,
    CounterOverflow,
    GameComplete,
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
    use std::collections::HashSet;

    fn card(rank: Rank) -> Card {
        Card::new(Suit::Clubs, rank)
    }

    fn assert_state_invariants(game: &Game) {
        let cards = game
            .state
            .tableau
            .iter()
            .flatten()
            .chain(&game.state.stock)
            .chain(&game.state.waste)
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(cards.len(), 52);
        assert_eq!(cards.iter().copied().collect::<HashSet<_>>().len(), 52);
        assert_eq!(
            usize::try_from(game.state.moves).unwrap(),
            game.actions.len()
        );

        for index in 0..TABLEAU_SIZE {
            let expected = game.state.tableau[index].is_some()
                && CHILDREN.get(index).is_none_or(|&(left, right)| {
                    game.state.tableau[left].is_none() && game.state.tableau[right].is_none()
                });
            assert_eq!(game.state.is_exposed(index), expected, "tableau {index}");
        }
        assert!(!game.state.is_exposed(TABLEAU_SIZE));
        assert!(!game.state.is_exposed(usize::MAX));
    }

    fn candidate_actions() -> impl Iterator<Item = Action> {
        std::iter::once(Action::Draw).chain((u8::MIN..=u8::MAX).map(Action::Remove))
    }

    type HostileCase = (&'static str, Game, Action, MoveError);

    fn base_hostile_action_cases(base: &Game, non_adjacent: usize) -> Vec<HostileCase> {
        vec![
            (
                "covered card",
                base.clone(),
                Action::Remove(0),
                MoveError::CoveredCard,
            ),
            (
                "first out-of-range index",
                base.clone(),
                Action::Remove(to_u8(TABLEAU_SIZE)),
                MoveError::CoveredCard,
            ),
            (
                "maximum hostile index",
                base.clone(),
                Action::Remove(u8::MAX),
                MoveError::CoveredCard,
            ),
            (
                "non-adjacent card",
                base.clone(),
                Action::Remove(to_u8(non_adjacent)),
                MoveError::NotAdjacent,
            ),
        ]
    }

    fn hostile_action_cases() -> Vec<HostileCase> {
        let base = Game::new(7, Options::default());
        let non_adjacent = (18..TABLEAU_SIZE)
            .find(|&index| {
                !adjacent(
                    *base.state.waste.last().unwrap(),
                    base.state.tableau[index].unwrap(),
                    false,
                )
            })
            .unwrap();

        let mut empty_stock = base.clone();
        empty_stock.state.stock.clear();

        let mut empty_waste = base.clone();
        empty_waste.state.waste.clear();

        let mut draw_overflow = base.clone();
        draw_overflow.state.moves = u32::MAX;

        let mut streak_overflow = base.clone();
        streak_overflow.state.tableau = [None; TABLEAU_SIZE];
        streak_overflow.state.tableau[18] = Some(card(Rank::Queen));
        streak_overflow.state.waste = vec![card(Rank::Jack)];
        streak_overflow.state.streak = u32::MAX;

        let mut score_overflow = streak_overflow.clone();
        score_overflow.state.streak = 0;
        score_overflow.state.score = u32::MAX;

        let mut move_overflow = streak_overflow.clone();
        move_overflow.state.streak = 0;
        move_overflow.state.moves = u32::MAX;

        let mut replay_full = base.clone();
        replay_full.actions = vec![Action::Draw; crate::replay::MAX_REPLAY_ACTIONS];

        let mut complete = base.clone();
        complete.state.tableau = [None; TABLEAU_SIZE];

        let mut cases = base_hostile_action_cases(&base, non_adjacent);
        cases.extend([
            (
                "empty stock",
                empty_stock,
                Action::Draw,
                MoveError::EmptyStock,
            ),
            (
                "empty waste",
                empty_waste,
                Action::Remove(18),
                MoveError::EmptyWaste,
            ),
            (
                "draw move overflow",
                draw_overflow,
                Action::Draw,
                MoveError::CounterOverflow,
            ),
            (
                "streak overflow",
                streak_overflow,
                Action::Remove(18),
                MoveError::CounterOverflow,
            ),
            (
                "score overflow",
                score_overflow,
                Action::Remove(18),
                MoveError::CounterOverflow,
            ),
            (
                "remove move overflow",
                move_overflow,
                Action::Remove(18),
                MoveError::CounterOverflow,
            ),
            (
                "replay capacity",
                replay_full,
                Action::Draw,
                MoveError::ResourceLimit,
            ),
            (
                "completed game",
                complete,
                Action::Draw,
                MoveError::GameComplete,
            ),
        ]);
        cases
    }

    #[test]
    fn hostile_actions_are_exact_and_fully_atomic() {
        for (name, mut game, action, expected) in hostile_action_cases() {
            let before = game.clone();
            let before_bytes = serde_json::to_vec(&before).unwrap();
            assert_eq!(game.apply(action), Err(expected), "{name}");
            assert_eq!(game, before, "{name}");
            assert_eq!(serde_json::to_vec(&game).unwrap(), before_bytes, "{name}");
        }
    }

    #[test]
    fn fixed_seed_rule_action_space_preserves_tripeaks_invariants() {
        let mut legal_draws = 0;
        let mut legal_removals = 0;
        let mut rejected_actions = 0;

        for wraparound in [false, true] {
            for seed in [0, 7, 41, u64::MAX] {
                let mut game = Game::new(seed, Options { wraparound });
                assert_state_invariants(&game);

                for _ in 0..52 {
                    for action in candidate_actions() {
                        let before = game.clone();
                        let mut probe = before.clone();
                        if probe.apply(action).is_ok() {
                            assert_state_invariants(&probe);
                            assert_eq!(Game::from_replay(&probe.replay()).unwrap(), probe);

                            let after = probe.clone();
                            assert!(probe.undo());
                            assert_eq!(probe.state, before.state);
                            assert_eq!(probe.replay(), before.replay());
                            assert!(probe.can_redo());
                            assert!(probe.redo());
                            assert_eq!(probe, after);

                            match action {
                                Action::Draw => legal_draws += 1,
                                Action::Remove(_) => legal_removals += 1,
                            }
                        } else {
                            rejected_actions += 1;
                            assert_eq!(probe, before);
                        }
                    }

                    let Some(action) = game.hint() else {
                        break;
                    };
                    game.apply(action).unwrap();
                    assert_state_invariants(&game);
                }
            }
        }

        assert!(legal_draws > 0);
        assert!(legal_removals > 0);
        assert!(rejected_actions > 0);
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
        assert!(game.can_redo());
        assert!(game.redo());
        assert_eq!(game.state, drawn);
        assert_eq!(Game::from_replay(&replay).unwrap().state, drawn);
    }

    #[test]
    fn illegal_covered_and_non_adjacent_removals_are_atomic() {
        let mut game = Game::new(7, Options::default());
        let before = game.clone();
        assert_eq!(game.apply(Action::Remove(0)), Err(MoveError::CoveredCard));
        assert_eq!(game, before);

        let index = (18..28)
            .find(|&index| {
                let waste = *game.state.waste.last().unwrap();
                !adjacent(waste, game.state.tableau[index].unwrap(), false)
            })
            .unwrap();
        let before = game.clone();
        assert_eq!(
            game.apply(Action::Remove(to_u8(index))),
            Err(MoveError::NotAdjacent)
        );
        assert_eq!(game, before);
    }

    #[test]
    fn fixed_seed_hint_is_legal_and_streak_scoring_accumulates() {
        let mut game = Game::new(7, Options::default());
        let hint = game.hint().unwrap();
        game.apply(hint).unwrap();
        assert_eq!(game.state.moves, 1);

        game.state.tableau = [None; TABLEAU_SIZE];
        game.state.tableau[18] = Some(card(Rank::Queen));
        game.state.tableau[19] = Some(card(Rank::King));
        game.state.waste = vec![card(Rank::Jack)];
        game.state.streak = 0;
        game.state.score = 0;
        game.apply(Action::Remove(18)).unwrap();
        game.apply(Action::Remove(19)).unwrap();
        assert_eq!(game.state.streak, 2);
        assert_eq!(game.state.score, 300);
    }

    #[test]
    fn removing_last_tableau_card_wins() {
        let mut game = Game::new(2, Options::default());
        game.state.tableau = [None; TABLEAU_SIZE];
        game.state.tableau[27] = Some(card(Rank::Queen));
        game.state.waste = vec![card(Rank::Jack)];
        game.apply(Action::Remove(27)).unwrap();
        assert!(game.state.is_won());
        assert_eq!(game.hint(), None);

        let complete = game.clone();
        assert_eq!(game.apply(Action::Draw), Err(MoveError::GameComplete));
        assert_eq!(game.apply(Action::Remove(27)), Err(MoveError::GameComplete));
        assert_eq!(game, complete);
        assert!(game.undo());
        assert!(!game.state.is_won());
        assert!(game.redo());
        assert!(game.state.is_won());
    }

    #[test]
    fn legal_seed_zero_replay_reaches_a_one_move_near_win() {
        let envelope: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/tripeaks-seed-zero-near-win.json"
        ))
        .unwrap();
        assert_eq!(envelope["version"], 1);
        assert_eq!(envelope["game"], "tripeaks");
        let replay: Replay<Action, Options> =
            serde_json::from_value(envelope["payload"].clone()).unwrap();
        let mut game = Game::from_replay(&replay).unwrap();

        assert_eq!(game.state.seed, 0);
        assert_eq!(game.state.options, Options::default());
        assert_eq!(game.state.tableau.iter().flatten().count(), 1);
        assert!(game.state.tableau[0].is_some());
        assert!(game.state.is_exposed(0));
        assert_eq!(game.state.stock.len(), 2);
        assert_eq!(game.state.waste.len(), 49);
        assert_eq!(game.state.score, 5_700);
        assert_eq!(game.state.moves, 48);
        assert_eq!(game.state.card_count(), 52);
        assert!(!game.state.is_won());

        game.apply(Action::Remove(0)).unwrap();
        assert!(game.state.is_won());
        assert_eq!(game.state.score, 5_800);
        assert_eq!(game.state.moves, 49);
        assert_eq!(game.state.card_count(), 52);
        assert_eq!(game.replay().actions.len(), 49);
    }

    #[test]
    fn malformed_and_oversized_replays_are_rejected() {
        let mut wrong_game = Game::new(1, Options::default()).replay();
        wrong_game.game = "spider".into();
        assert_eq!(Game::from_replay(&wrong_game), Err(MoveError::WrongGame));

        let mut oversized = Game::new(1, Options::default()).replay();
        oversized.actions = vec![Action::Draw; crate::replay::MAX_REPLAY_ACTIONS + 1];
        assert_eq!(Game::from_replay(&oversized), Err(MoveError::ResourceLimit));
    }

    #[test]
    fn replay_limit_rejects_before_mutation_and_history_is_bounded() {
        let mut game = Game::new(1, Options::default());
        game.actions = vec![Action::Draw; crate::replay::MAX_REPLAY_ACTIONS];
        let before = game.state.clone();
        assert_eq!(game.apply(Action::Draw), Err(MoveError::ResourceLimit));
        assert_eq!(game.state, before);

        game.undo = vec![game.state.clone(); crate::replay::MAX_HISTORY_ACTIONS];
        push_bounded_history(&mut game.undo, game.state.clone());
        assert_eq!(game.undo.len(), crate::replay::MAX_HISTORY_ACTIONS);
    }

    #[test]
    fn counter_overflow_is_rejected_atomically() {
        let mut draw = Game::new(1, Options::default());
        draw.state.moves = u32::MAX;
        let before = draw.clone();
        assert_eq!(draw.apply(Action::Draw), Err(MoveError::CounterOverflow));
        assert_eq!(draw, before);

        let mut remove = Game::new(1, Options::default());
        remove.state.tableau = [None; TABLEAU_SIZE];
        remove.state.tableau[18] = Some(card(Rank::Queen));
        remove.state.waste = vec![card(Rank::Jack)];
        remove.state.score = u32::MAX;
        let before = remove.clone();
        assert_eq!(
            remove.apply(Action::Remove(18)),
            Err(MoveError::CounterOverflow)
        );
        assert_eq!(remove, before);
    }

    #[test]
    fn typed_replay_with_wrong_version_is_rejected() {
        let mut replay = Game::new(1, Options::default()).replay();
        replay.version = 3;
        assert_eq!(
            Game::from_replay(&replay),
            Err(MoveError::UnsupportedReplayVersion(3))
        );
    }

    fn to_u8(value: usize) -> u8 {
        u8::try_from(value).unwrap()
    }
}
