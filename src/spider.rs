use crate::cards::{Card, Rank, Suit, shuffle};
use crate::replay::{CURRENT_REPLAY_VERSION, Replay};
use serde::{Deserialize, Serialize};

const COLUMN_COUNT: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuitMode {
    One,
    Two,
    Four,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpiderCard {
    pub card: Card,
    pub face_up: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub seed: u64,
    pub mode: SuitMode,
    pub columns: [Vec<SpiderCard>; COLUMN_COUNT],
    pub stock: Vec<Card>,
    pub completed_runs: u8,
    pub score: i32,
    pub moves: u32,
}

impl State {
    #[must_use]
    pub fn new(seed: u64, mode: SuitMode) -> Self {
        let mut deck = spider_deck(mode);
        shuffle(&mut deck, seed);
        let mut columns: [Vec<SpiderCard>; COLUMN_COUNT] = Default::default();
        for row in 0..6 {
            let width = if row < 5 { COLUMN_COUNT } else { 4 };
            for column in columns.iter_mut().take(width) {
                let card = deck.pop().unwrap_or(Card::new(Suit::Spades, Rank::Ace));
                column.push(SpiderCard {
                    card,
                    face_up: row == if width == 4 { 5 } else { 4 },
                });
            }
        }
        // The row-based deal above leaves the fifth row face-up in columns 4-9
        // and the sixth row face-up in columns 0-3.
        for column in &mut columns {
            if let Some(card) = column.last_mut() {
                card.face_up = true;
            }
            for card in column.iter_mut().rev().skip(1) {
                card.face_up = false;
            }
        }
        Self {
            seed,
            mode,
            columns,
            stock: deck,
            completed_runs: 0,
            score: 500,
            moves: 0,
        }
    }

    #[must_use]
    pub fn is_won(&self) -> bool {
        self.completed_runs == 8
    }

    #[must_use]
    pub fn card_count(&self) -> usize {
        self.stock.len()
            + self.columns.iter().map(Vec::len).sum::<usize>()
            + usize::from(self.completed_runs) * 13
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    DealRow,
    Move { from: u8, to: u8, count: u8 },
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
    pub fn new(seed: u64, mode: SuitMode) -> Self {
        Self {
            state: State::new(seed, mode),
            undo: Vec::new(),
            redo: Vec::new(),
            actions: Vec::new(),
            redo_actions: Vec::new(),
        }
    }

    /// Applies an action atomically.
    ///
    /// # Errors
    ///
    /// Returns [`MoveError`] if the action violates Spider rules.
    pub fn apply(&mut self, action: Action) -> Result<(), MoveError> {
        if self.actions.len() >= crate::replay::MAX_REPLAY_ACTIONS {
            return Err(MoveError::ResourceLimit);
        }
        let before = self.state.clone();
        let result = match action {
            Action::DealRow => self.deal_row(),
            Action::Move { from, to, count } => self.move_run(from, to, count),
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
    pub fn replay(&self) -> Replay<Action, SuitMode> {
        Replay {
            version: CURRENT_REPLAY_VERSION,
            game: "spider".into(),
            seed: self.state.seed,
            setup: self.state.mode,
            actions: self.actions.clone(),
        }
    }

    /// Reconstructs and validates a Spider replay.
    ///
    /// # Errors
    ///
    /// Returns an error for the wrong game identifier or an illegal action.
    pub fn from_replay(replay: &Replay<Action, SuitMode>) -> Result<Self, MoveError> {
        crate::replay::validate_version(replay.version)
            .map_err(|_| MoveError::UnsupportedReplayVersion(replay.version))?;
        crate::replay::validate_action_count(replay.actions.len())
            .map_err(|_| MoveError::ResourceLimit)?;
        if replay.game != "spider" {
            return Err(MoveError::WrongGame);
        }
        let mut game = Self::new(replay.seed, replay.setup);
        let deadline = Replay::<Action, SuitMode>::reconstruction_deadline();
        for (step, action) in replay.actions.iter().enumerate() {
            Replay::<Action, SuitMode>::check_reconstruction(deadline, step + 1)
                .map_err(|_| MoveError::ResourceLimit)?;
            game.apply(action.clone())?;
        }
        Ok(game)
    }

    #[must_use]
    pub fn hint(&self) -> Option<Action> {
        for from in 0..COLUMN_COUNT {
            let movable = movable_count(&self.state.columns[from]);
            for count in 1..=movable {
                for to in 0..COLUMN_COUNT {
                    let action = Action::Move {
                        from: to_u8(from),
                        to: to_u8(to),
                        count: to_u8(count),
                    };
                    let mut probe = Self {
                        state: self.state.clone(),
                        undo: Vec::new(),
                        redo: Vec::new(),
                        actions: Vec::new(),
                        redo_actions: Vec::new(),
                    };
                    if probe.apply(action.clone()).is_ok() {
                        return Some(action);
                    }
                }
            }
        }
        (self.state.stock.len() >= COLUMN_COUNT
            && self.state.columns.iter().all(|column| !column.is_empty()))
        .then_some(Action::DealRow)
    }

    fn deal_row(&mut self) -> Result<(), MoveError> {
        if self.state.stock.len() < COLUMN_COUNT {
            return Err(MoveError::EmptyStock);
        }
        if self.state.columns.iter().any(Vec::is_empty) {
            return Err(MoveError::EmptyColumnDuringDeal);
        }
        for column in &mut self.state.columns {
            let card = self.state.stock.pop().ok_or(MoveError::EmptyStock)?;
            column.push(SpiderCard {
                card,
                face_up: true,
            });
        }
        self.finish_action()?;
        Ok(())
    }

    fn move_run(&mut self, from: u8, to: u8, count: u8) -> Result<(), MoveError> {
        if from == to || count == 0 {
            return Err(MoveError::InvalidMove);
        }
        let from = usize::from(from);
        let to = usize::from(to);
        if from >= COLUMN_COUNT || to >= COLUMN_COUNT {
            return Err(MoveError::InvalidColumn);
        }
        let count = usize::from(count);
        let source = &self.state.columns[from];
        if count > source.len() {
            return Err(MoveError::InvalidMove);
        }
        let selected = &source[source.len() - count..];
        if !same_suit_run(selected) {
            return Err(MoveError::BrokenRun);
        }
        if let Some(destination) = self.state.columns[to].last()
            && destination.card.rank.value() != selected[0].card.rank.value() + 1
        {
            return Err(MoveError::InvalidDestination);
        }
        let split = self.state.columns[from].len() - count;
        let moved = self.state.columns[from].split_off(split);
        self.state.columns[to].extend(moved);
        self.flip_exposed(from);
        self.finish_action()?;
        Ok(())
    }

    fn finish_action(&mut self) -> Result<(), MoveError> {
        self.state.moves = self
            .state
            .moves
            .checked_add(1)
            .ok_or(MoveError::CounterOverflow)?;
        self.state.score -= 1;
        for column in 0..COLUMN_COUNT {
            if complete_run(&self.state.columns[column]) {
                let new_length = self.state.columns[column].len() - 13;
                self.state.columns[column].truncate(new_length);
                self.state.completed_runs += 1;
                self.state.score += 100;
                self.flip_exposed(column);
            }
        }
        Ok(())
    }

    fn flip_exposed(&mut self, column: usize) {
        if let Some(card) = self.state.columns[column].last_mut() {
            card.face_up = true;
        }
    }
}

fn push_bounded_history(history: &mut Vec<State>, state: State) {
    if history.len() == crate::replay::MAX_HISTORY_ACTIONS {
        history.remove(0);
    }
    history.push(state);
}

#[must_use]
pub fn spider_deck(mode: SuitMode) -> Vec<Card> {
    let (suits, copies): (&[Suit], usize) = match mode {
        SuitMode::One => (&[Suit::Spades], 8),
        SuitMode::Two => (&[Suit::Spades, Suit::Hearts], 4),
        SuitMode::Four => (&Suit::ALL, 2),
    };
    (0..copies)
        .flat_map(|_| {
            suits
                .iter()
                .copied()
                .flat_map(|suit| Rank::ALL.into_iter().map(move |rank| Card::new(suit, rank)))
        })
        .collect()
}

fn movable_count(column: &[SpiderCard]) -> usize {
    let mut count = 0;
    for card in column.iter().rev() {
        if !card.face_up {
            break;
        }
        if count > 0 {
            let lower = &column[column.len() - count];
            if card.card.suit != lower.card.suit
                || card.card.rank.value() != lower.card.rank.value() + 1
            {
                break;
            }
        }
        count += 1;
    }
    count
}

fn same_suit_run(cards: &[SpiderCard]) -> bool {
    cards.iter().all(|card| card.face_up)
        && cards.windows(2).all(|pair| {
            pair[0].card.suit == pair[1].card.suit
                && pair[0].card.rank.value() == pair[1].card.rank.value() + 1
        })
}

fn complete_run(column: &[SpiderCard]) -> bool {
    if column.len() < 13 {
        return false;
    }
    let run = &column[column.len() - 13..];
    same_suit_run(run)
        && run.first().is_some_and(|card| card.card.rank == Rank::King)
        && run.last().is_some_and(|card| card.card.rank == Rank::Ace)
}

fn to_u8(value: usize) -> u8 {
    u8::try_from(value).unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    EmptyStock,
    EmptyColumnDuringDeal,
    InvalidMove,
    InvalidColumn,
    BrokenRun,
    InvalidDestination,
    WrongGame,
    UnsupportedReplayVersion(u16),
    ResourceLimit,
    CounterOverflow,
}

impl std::fmt::Display for MoveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "illegal Spider action: {self:?}")
    }
}

impl std::error::Error for MoveError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn up(suit: Suit, rank: Rank) -> SpiderCard {
        SpiderCard {
            card: Card::new(suit, rank),
            face_up: true,
        }
    }

    fn assert_state_invariants(game: &Game) {
        assert_eq!(game.state.card_count(), 104);
        assert!(game.state.completed_runs <= 8);
        for column in &game.state.columns {
            if let Some(top) = column.last() {
                assert!(top.face_up);
            }
            let mut reached_face_up_suffix = false;
            for card in column {
                if card.face_up {
                    reached_face_up_suffix = true;
                } else {
                    assert!(!reached_face_up_suffix);
                }
            }
        }
    }

    fn candidate_actions(game: &Game) -> Vec<Action> {
        let mut actions = vec![Action::DealRow];
        for (from, column) in game.state.columns.iter().enumerate() {
            for to in 0..COLUMN_COUNT {
                for count in 0..=column.len() + 1 {
                    actions.push(Action::Move {
                        from: to_u8(from),
                        to: to_u8(to),
                        count: to_u8(count),
                    });
                }
            }
        }
        actions
    }

    type HostileCase = (&'static str, Game, Action, MoveError);

    const fn move_action(from: u8, to: u8, count: u8) -> Action {
        Action::Move { from, to, count }
    }

    fn hostile_action_cases() -> Vec<HostileCase> {
        let base = Game::new(17, SuitMode::Four);
        let oversized_count = to_u8(base.state.columns[0].len() + 1);
        let mut face_down = base.clone();
        face_down.state.columns[0].last_mut().unwrap().face_up = false;
        let mut broken_run = base.clone();
        let broken_length = broken_run.state.columns[0].len();
        broken_run.state.columns[0][broken_length - 2] = up(Suit::Spades, Rank::Eight);
        broken_run.state.columns[0][broken_length - 1] = up(Suit::Hearts, Rank::Seven);
        let mut bad_destination = base.clone();
        *bad_destination.state.columns[0].last_mut().unwrap() = up(Suit::Spades, Rank::Seven);
        *bad_destination.state.columns[1].last_mut().unwrap() = up(Suit::Hearts, Rank::Nine);
        let mut empty_stock = base.clone();
        empty_stock.state.stock.clear();
        let mut empty_column = base.clone();
        empty_column.state.columns[0].clear();

        vec![
            (
                "zero count",
                base.clone(),
                move_action(0, 1, 0),
                MoveError::InvalidMove,
            ),
            (
                "self move",
                base.clone(),
                move_action(0, 0, 1),
                MoveError::InvalidMove,
            ),
            (
                "source out of bounds",
                base.clone(),
                move_action(10, 0, 1),
                MoveError::InvalidColumn,
            ),
            (
                "destination out of bounds",
                base.clone(),
                move_action(0, 10, 1),
                MoveError::InvalidColumn,
            ),
            (
                "oversized run",
                base.clone(),
                move_action(0, 1, oversized_count),
                MoveError::InvalidMove,
            ),
            (
                "face-down card",
                face_down,
                move_action(0, 1, 1),
                MoveError::BrokenRun,
            ),
            (
                "broken suited run",
                broken_run,
                move_action(0, 1, 2),
                MoveError::BrokenRun,
            ),
            (
                "bad destination",
                bad_destination,
                move_action(0, 1, 1),
                MoveError::InvalidDestination,
            ),
            (
                "empty stock",
                empty_stock,
                Action::DealRow,
                MoveError::EmptyStock,
            ),
            (
                "empty column during deal",
                empty_column,
                Action::DealRow,
                MoveError::EmptyColumnDuringDeal,
            ),
        ]
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
    fn fixed_seed_mode_action_space_preserves_spider_invariants() {
        for mode in [SuitMode::One, SuitMode::Two, SuitMode::Four] {
            let mut legal_deals = 0;
            let mut legal_moves = 0;
            let mut rejected_actions = 0;

            for seed in [1, 41] {
                let mut game = Game::new(seed, mode);
                assert_state_invariants(&game);

                for step in 0..5 {
                    let mut next_deal = None;
                    let mut next_move = None;
                    for action in candidate_actions(&game) {
                        let before = game.clone();
                        let mut probe = before.clone();
                        if probe.apply(action.clone()).is_ok() {
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
                                Action::DealRow => {
                                    legal_deals += 1;
                                    next_deal.get_or_insert(after);
                                }
                                Action::Move { .. } => {
                                    legal_moves += 1;
                                    next_move.get_or_insert(after);
                                }
                            }
                        } else {
                            rejected_actions += 1;
                            assert_eq!(probe, before);
                        }
                    }

                    game = if step % 2 == 0 {
                        next_deal.or(next_move)
                    } else {
                        next_move.or(next_deal)
                    }
                    .expect("every swept state should have a legal continuation");
                }
            }

            assert!(legal_deals > 0, "{mode:?}");
            assert!(legal_moves > 0, "{mode:?}");
            assert!(rejected_actions > 0, "{mode:?}");
        }
    }

    #[test]
    fn synthetic_final_run_wins_but_is_not_a_full_deal_replay() {
        let mut game = Game::new(73, SuitMode::One);
        game.state.columns = Default::default();
        game.state.stock.clear();
        game.state.columns[0] = Rank::ALL
            .into_iter()
            .rev()
            .filter(|rank| *rank != Rank::Ace)
            .map(|rank| up(Suit::Spades, rank))
            .collect();
        game.state.columns[1] = vec![up(Suit::Spades, Rank::Ace)];
        game.state.completed_runs = 7;
        game.state.score = 1_200;
        game.state.moves = 41;
        assert_state_invariants(&game);

        let before = game.clone();
        let final_action = Action::Move {
            from: 1,
            to: 0,
            count: 1,
        };
        game.apply(final_action.clone()).unwrap();

        assert!(game.state.is_won());
        assert_eq!(game.state.completed_runs, 8);
        assert_eq!(game.state.score, 1_299);
        assert_eq!(game.state.moves, 42);
        assert_eq!(game.state.card_count(), 104);
        assert!(game.state.stock.is_empty());
        assert!(game.state.columns.iter().all(Vec::is_empty));

        let won = game.clone();
        assert!(game.undo());
        assert_eq!(game.state, before.state);
        assert!(!game.state.is_won());
        assert!(game.redo());
        assert_eq!(game, won);

        // This synthetic seven-run prefix was injected directly. Its one-action replay
        // intentionally cannot reconstruct a complete deal from the seeded boundary.
        let replay = game.replay();
        assert_eq!(replay.actions, vec![final_action]);
        assert_ne!(
            Game::from_replay(&replay).map(|candidate| candidate.state),
            Ok(won.state)
        );
    }

    #[test]
    fn variants_have_104_cards_with_expected_suits() {
        for (mode, expected) in [
            (SuitMode::One, vec![(Suit::Spades, 104)]),
            (SuitMode::Two, vec![(Suit::Spades, 52), (Suit::Hearts, 52)]),
            (
                SuitMode::Four,
                Suit::ALL.into_iter().map(|suit| (suit, 26)).collect(),
            ),
        ] {
            let counts = spider_deck(mode).into_iter().fold(
                HashMap::<Suit, usize>::new(),
                |mut counts, card| {
                    *counts.entry(card.suit).or_default() += 1;
                    counts
                },
            );
            assert_eq!(counts.len(), expected.len());
            for (suit, count) in expected {
                assert_eq!(counts.get(&suit), Some(&count));
            }
        }
    }

    #[test]
    fn move_counter_overflow_is_rejected_atomically() {
        let mut game = Game::new(3, SuitMode::One);
        game.state.moves = u32::MAX;
        let before = game.state.clone();
        assert_eq!(game.apply(Action::DealRow), Err(MoveError::CounterOverflow));
        assert_eq!(game.state, before);
    }

    #[test]
    fn deal_shape_is_deterministic_and_conserves_cards() {
        let first = State::new(88, SuitMode::Four);
        let second = State::new(88, SuitMode::Four);
        assert_eq!(first, second);
        assert_eq!(first.stock.len(), 50);
        assert_eq!(first.card_count(), 104);
        assert_eq!(
            first.columns[..4].iter().map(Vec::len).collect::<Vec<_>>(),
            [6; 4]
        );
        assert_eq!(
            first.columns[4..].iter().map(Vec::len).collect::<Vec<_>>(),
            [5; 6]
        );
        assert!(
            first
                .columns
                .iter()
                .all(|column| column.last().unwrap().face_up)
        );
    }

    #[test]
    fn move_requires_same_suit_run_but_build_accepts_any_suit() {
        let mut game = Game::new(1, SuitMode::Four);
        game.state.columns = Default::default();
        game.state.columns[0] = vec![up(Suit::Spades, Rank::Eight), up(Suit::Spades, Rank::Seven)];
        game.state.columns[1] = vec![up(Suit::Hearts, Rank::Nine)];
        game.apply(Action::Move {
            from: 0,
            to: 1,
            count: 2,
        })
        .unwrap();
        assert_eq!(game.state.columns[1].len(), 3);

        game.state.columns[2] = vec![up(Suit::Spades, Rank::Five), up(Suit::Hearts, Rank::Four)];
        assert_eq!(
            game.apply(Action::Move {
                from: 2,
                to: 3,
                count: 2
            }),
            Err(MoveError::BrokenRun)
        );
    }

    #[test]
    fn stock_deal_requires_no_empty_columns() {
        let mut game = Game::new(2, SuitMode::One);
        game.state.columns[0].clear();
        let before = game.state.clone();
        assert_eq!(
            game.apply(Action::DealRow),
            Err(MoveError::EmptyColumnDuringDeal)
        );
        assert_eq!(game.state, before);
    }

    #[test]
    fn complete_suited_run_is_removed_and_scored() {
        let mut game = Game::new(3, SuitMode::One);
        game.state.columns = Default::default();
        game.state.stock.clear();
        game.state.columns[0] = Rank::ALL
            .into_iter()
            .rev()
            .map(|rank| up(Suit::Spades, rank))
            .collect();
        game.state.columns[1] = vec![up(Suit::Hearts, Rank::Two)];
        game.apply(Action::Move {
            from: 1,
            to: 2,
            count: 1,
        })
        .unwrap();
        assert!(game.state.columns[0].is_empty());
        assert_eq!(game.state.completed_runs, 1);
        assert_eq!(game.state.score, 599);
        assert_eq!(game.state.card_count(), 14);
    }

    #[test]
    fn undo_redo_and_replay_restore_a_deal_row() {
        let mut game = Game::new(7, SuitMode::Two);
        game.apply(Action::DealRow).unwrap();
        let dealt = game.state.clone();
        let replay = game.replay();
        assert!(game.can_undo());
        assert!(game.undo());
        assert!(game.can_redo());
        assert_eq!(game.state.stock.len(), 50);
        assert!(game.redo());
        assert_eq!(game.state, dealt);
        assert_eq!(Game::from_replay(&replay).unwrap().state, dealt);
        assert_eq!(
            Game::from_replay(&replay).unwrap().state.mode,
            SuitMode::Two
        );
    }

    #[test]
    fn replay_limit_is_independent_from_the_undo_window() {
        let mut game = Game::new(43, SuitMode::One);
        game.actions = vec![Action::DealRow; crate::replay::MAX_REPLAY_ACTIONS];
        let before = game.state.clone();
        assert_eq!(game.apply(Action::DealRow), Err(MoveError::ResourceLimit));
        assert_eq!(game.state, before);
    }

    #[test]
    fn legal_move_cycle_exceeds_undo_window_and_replay_rebuilds() {
        let mut game = Game::new(1, SuitMode::One);
        let forward = Action::Move {
            from: 2,
            to: 5,
            count: 1,
        };
        let reverse = Action::Move {
            from: 5,
            to: 2,
            count: 1,
        };
        let cycles = crate::replay::MAX_HISTORY_ACTIONS / 2 + 20;
        for _ in 0..cycles {
            game.apply(forward.clone()).unwrap();
            game.apply(reverse.clone()).unwrap();
        }

        let replay = game.replay();
        assert!(replay.actions.len() > crate::replay::MAX_HISTORY_ACTIONS);
        assert_eq!(Game::from_replay(&replay).unwrap().state, game.state);

        let mut undo_probe = game;
        let mut undo_count = 0;
        while undo_probe.undo() {
            undo_count += 1;
        }
        assert_eq!(undo_count, crate::replay::MAX_HISTORY_ACTIONS);
    }

    #[test]
    fn typed_replay_with_wrong_version_is_rejected() {
        let mut replay = Game::new(1, SuitMode::Four).replay();
        replay.version = 99;
        assert_eq!(
            Game::from_replay(&replay),
            Err(MoveError::UnsupportedReplayVersion(99))
        );
    }
}
