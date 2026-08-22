use slint::{ModelRc, SharedString, VecModel};
use solitaire::cards::{Card, Rank, Suit};
use solitaire::freecell::{self, Game as FreeCellGame};
use solitaire::klondike::{Action, DrawMode, Game, Options, Pile, Scoring};
use solitaire::persistence::{
    DealCounters, DealKind, RecoveredSave, SaveError, SaveRevision, default_deal_counters_path,
    default_freecell_save_path, default_save_path, default_spider_save_path, load_deal_counters,
    load_freecell_revisioned, load_klondike_revisioned, load_spider_revisioned,
    recover_freecell_revisioned, recover_klondike_revisioned, recover_spider_revisioned,
    reserve_deal, save_freecell_checked, save_klondike_checked, save_spider_checked,
};
use solitaire::spider::{self, Game as SpiderGame, SuitMode};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

slint::include_modules!();

#[derive(Clone, Copy, PartialEq, Eq)]
enum Selection {
    Waste,
    Tableau { column: u8, count: u8 },
    Foundation(Suit),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GameKind {
    Klondike,
    Spider,
    FreeCell,
}

struct PendingNewDeal {
    game: GameKind,
    variant: String,
}

enum ProspectiveGame {
    Klondike(Game),
    Spider(SpiderGame),
    FreeCell(FreeCellGame),
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SpiderSelection {
    column: u8,
    count: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FreeCellSelection {
    pile: freecell::Pile,
    count: u8,
}

impl Selection {
    const fn pile(self) -> Pile {
        match self {
            Self::Waste => Pile::Waste,
            Self::Tableau { column, .. } => Pile::Tableau(column),
            Self::Foundation(suit) => Pile::Foundation(suit),
        }
    }

    const fn count(self) -> u8 {
        match self {
            Self::Tableau { count, .. } => count,
            Self::Waste | Self::Foundation(_) => 1,
        }
    }
}

struct Controller {
    active: GameKind,
    game: Game,
    selection: Option<Selection>,
    save_path: Option<PathBuf>,
    spider: SpiderGame,
    spider_selection: Option<SpiderSelection>,
    spider_save_path: Option<PathBuf>,
    freecell: FreeCellGame,
    freecell_selection: Option<FreeCellSelection>,
    freecell_save_path: Option<PathBuf>,
    save_revisions: [Option<SaveRevision>; 3],
    dirty: [bool; 3],
    pending_new_deal: Option<PendingNewDeal>,
    pending_new_deal_conflict: bool,
    deal_counters_path: Option<PathBuf>,
    next_seeds: DealCounters,
    status: String,
}

impl Controller {
    fn new() -> Self {
        let mut save_path = default_save_path();
        let mut status = "Choose a card to begin".to_owned();
        let saved = load_or_recover(&mut save_path, recover_klondike_revisioned, &mut status);
        let seed = saved
            .as_ref()
            .map_or_else(seed_now, |(game, _)| game.state.seed);
        let klondike_revision = saved.as_ref().map(|(_, revision)| *revision);
        let mut spider_save_path = default_spider_save_path();
        let (spider, spider_revision) = load_or_recover(
            &mut spider_save_path,
            recover_spider_revisioned,
            &mut status,
        )
        .map_or_else(
            || (SpiderGame::new(seed.wrapping_add(1), SuitMode::One), None),
            |(game, revision)| (game, Some(revision)),
        );
        let mut freecell_save_path = default_freecell_save_path();
        let (freecell, freecell_revision) = load_or_recover(
            &mut freecell_save_path,
            recover_freecell_revisioned,
            &mut status,
        )
        .map_or_else(
            || (FreeCellGame::new(seed.wrapping_add(2)), None),
            |(game, revision)| (game, Some(revision)),
        );
        let deal_counters_path = default_deal_counters_path();
        let defaults = DealCounters {
            klondike: saved
                .as_ref()
                .map_or(seed, |(game, _)| game.state.seed)
                .saturating_add(1),
            spider: spider.state.seed.saturating_add(1),
            freecell: freecell.state.deal_number.saturating_add(1),
        };
        let mut next_seeds = deal_counters_path
            .as_deref()
            .and_then(|path| match load_deal_counters(path) {
                Ok(counters) => Some(counters),
                Err(SaveError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => {
                    status = format!(
                        "Deal counters could not be restored; current game seeds were used: {error}"
                    );
                    None
                }
            })
            .unwrap_or(defaults);
        next_seeds.klondike = next_seeds.klondike.max(defaults.klondike);
        next_seeds.spider = next_seeds.spider.max(defaults.spider);
        next_seeds.freecell = next_seeds.freecell.max(defaults.freecell);
        let save_revisions = [klondike_revision, spider_revision, freecell_revision];
        Self {
            active: GameKind::Klondike,
            game: saved.map_or_else(|| Game::new(seed, Options::default()), |(game, _)| game),
            selection: None,
            save_path,
            spider,
            spider_selection: None,
            spider_save_path,
            freecell,
            freecell_selection: None,
            freecell_save_path,
            save_revisions,
            dirty: [false; 3],
            pending_new_deal: None,
            pending_new_deal_conflict: false,
            deal_counters_path,
            next_seeds,
            status,
        }
    }

    fn apply(&mut self, action: Action) {
        match self.game.apply(action) {
            Ok(()) => {
                self.selection = None;
                self.status = if self.game.state.is_won() {
                    "Deal complete — beautifully played".into()
                } else {
                    "Move accepted".into()
                };
                self.persist_mutation();
            }
            Err(error) => self.status = friendly_error(error),
        }
    }

    fn activate_tableau(&mut self, column: i32, card_index: i32) {
        let Ok(column) = u8::try_from(column) else {
            return;
        };
        if let Some(selection) = self.selection {
            if selection.pile() == Pile::Tableau(column) {
                self.selection = None;
                self.status = "Selection cleared".into();
                return;
            }
            self.apply(Action::Move {
                from: selection.pile(),
                to: Pile::Tableau(column),
                count: selection.count(),
            });
            return;
        }
        let Some(pile) = self.game.state.tableau.get(usize::from(column)) else {
            return;
        };
        let Ok(index) = usize::try_from(card_index) else {
            return;
        };
        let Some(card) = pile.get(index) else {
            self.status = "Choose a card before choosing an empty column".into();
            return;
        };
        if !card.face_up {
            self.status = "That card is still face down".into();
            return;
        }
        let count = pile.len() - index;
        let Ok(count) = u8::try_from(count) else {
            return;
        };
        self.selection = Some(Selection::Tableau { column, count });
        self.status = format!("Selected {}", card_name(card.card));
    }

    fn activate_waste(&mut self) {
        if let Some(selection) = self.selection {
            if selection == Selection::Waste {
                self.selection = None;
                self.status = "Selection cleared".into();
            } else {
                self.status = "Cards cannot be moved onto the waste".into();
            }
        } else if let Some(card) = self.game.state.waste.last() {
            self.selection = Some(Selection::Waste);
            self.status = format!("Selected {}", card_name(*card));
        } else {
            self.status = "The waste is empty".into();
        }
    }

    fn activate_foundation(&mut self, index: i32) {
        let Some(suit) = suit_at(index) else {
            return;
        };
        if let Some(selection) = self.selection {
            if selection.pile() == Pile::Foundation(suit) {
                self.selection = None;
                self.status = "Selection cleared".into();
            } else {
                self.apply(Action::Move {
                    from: selection.pile(),
                    to: Pile::Foundation(suit),
                    count: selection.count(),
                });
            }
        } else if let Some(card) = self.game.state.foundations[suit_index(suit)].last() {
            self.selection = Some(Selection::Foundation(suit));
            self.status = format!("Selected {}", card_name(*card));
        } else {
            self.status = "That foundation is empty".into();
        }
    }

    fn draw_or_recycle(&mut self) {
        let action = if self.game.state.stock.is_empty() {
            Action::Recycle
        } else {
            Action::Draw
        };
        self.apply(action);
    }

    fn select_game(&mut self, game: &str) {
        self.active = match game {
            "Spider" => GameKind::Spider,
            "FreeCell" => GameKind::FreeCell,
            _ => GameKind::Klondike,
        };
        self.selection = None;
        self.spider_selection = None;
        self.freecell_selection = None;
        self.pending_new_deal = None;
        self.pending_new_deal_conflict = false;
        self.status = format!("{} ready", self.game_name());
    }

    fn new_game(&mut self, variant: &str) {
        let request = PendingNewDeal {
            game: self.active,
            variant: variant.to_owned(),
        };
        self.pending_new_deal = Some(request);
        self.pending_new_deal_conflict = false;
        if self.dirty[self.active_index()] {
            self.status = "This deal has unsaved progress. Retry save, discard it and start the new deal, or cancel.".into();
            return;
        }
        self.commit_pending_new_deal();
    }

    fn commit_pending_new_deal(&mut self) {
        let Some(request) = self.pending_new_deal.take() else {
            self.status = "No new deal is waiting for confirmation".into();
            return;
        };
        if request.game != self.active {
            self.status = "The pending new deal was cancelled after switching games".into();
            return;
        }
        let Some(seed) = self.take_next_seed(request.game) else {
            self.status = "No further deal number is representable; existing game preserved".into();
            self.pending_new_deal = Some(request);
            return;
        };
        let candidate = match request.game {
            GameKind::Klondike => {
                let draw_mode = if request.variant == "Draw 3" {
                    DrawMode::Three
                } else {
                    DrawMode::One
                };
                ProspectiveGame::Klondike(Game::new(
                    seed,
                    Options {
                        draw_mode,
                        scoring: Scoring::Standard,
                        max_redeals: None,
                        timed: false,
                    },
                ))
            }
            GameKind::Spider => {
                let mode = match request.variant.as_str() {
                    "2 suits" => SuitMode::Two,
                    "4 suits" => SuitMode::Four,
                    _ => SuitMode::One,
                };
                ProspectiveGame::Spider(SpiderGame::new(seed, mode))
            }
            GameKind::FreeCell => ProspectiveGame::FreeCell(FreeCellGame::new(seed)),
        };
        let index = self.active_index();
        let saved = match &candidate {
            ProspectiveGame::Klondike(game) => self.save_path.as_deref().map(|path| {
                save_klondike_checked(path, game, &mut self.save_revisions[index])
            }),
            ProspectiveGame::Spider(game) => self.spider_save_path.as_deref().map(|path| {
                save_spider_checked(path, game, &mut self.save_revisions[index])
            }),
            ProspectiveGame::FreeCell(game) => self.freecell_save_path.as_deref().map(|path| {
                save_freecell_checked(path, game, &mut self.save_revisions[index])
            }),
        };
        match saved {
            Some(Ok(())) => {
                match candidate {
                    ProspectiveGame::Klondike(game) => self.game = game,
                    ProspectiveGame::Spider(game) => self.spider = game,
                    ProspectiveGame::FreeCell(game) => self.freecell = game,
                }
                self.dirty[index] = false;
                self.pending_new_deal_conflict = false;
                self.clear_selections();
                self.status = format!("New {} deal", self.game_name());
            }
            Some(Err(error)) if error.committed_but_not_durable() => {
                match candidate {
                    ProspectiveGame::Klondike(game) => self.game = game,
                    ProspectiveGame::Spider(game) => self.spider = game,
                    ProspectiveGame::FreeCell(game) => self.freecell = game,
                }
                self.dirty[index] = true;
                self.pending_new_deal_conflict = false;
                self.clear_selections();
                self.status = format!(
                    "The new deal replaced the on-disk entry and is now current in memory, but durability is indeterminate: {error}"
                );
            }
            Some(Err(error)) => {
                self.pending_new_deal_conflict = error.is_conflict();
                self.pending_new_deal = Some(request);
                self.status = format!(
                    "New deal was not started; the current game remains in memory because saving the prospective deal failed: {error}. Retry, discard, or cancel."
                );
            }
            None => {
                self.pending_new_deal_conflict = false;
                self.pending_new_deal = Some(request);
                self.status = "New deal was not started; the current game remains in memory because no writable save location is available. Retry, discard, or cancel.".into();
            }
        }
    }

    fn take_next_seed(&mut self, game: GameKind) -> Option<u64> {
        let kind = match game {
            GameKind::Klondike => DealKind::Klondike,
            GameKind::Spider => DealKind::Spider,
            GameKind::FreeCell => DealKind::FreeCell,
        };
        if let Some(path) = self.deal_counters_path.as_deref() {
            match reserve_deal(path, self.next_seeds, kind) {
                Ok((seed, counters)) => {
                    self.next_seeds = counters;
                    return Some(seed);
                }
                Err(error) => {
                    self.status = format!("Could not reserve the next deal persistently: {error}");
                    return None;
                }
            }
        }
        let seed = match game {
            GameKind::Klondike => self.next_seeds.klondike,
            GameKind::Spider => self.next_seeds.spider,
            GameKind::FreeCell => self.next_seeds.freecell,
        };
        let next = seed.checked_add(1)?;
        match game {
            GameKind::Klondike => self.next_seeds.klondike = next,
            GameKind::Spider => self.next_seeds.spider = next,
            GameKind::FreeCell => self.next_seeds.freecell = next,
        }
        Some(seed)
    }

    fn save(&mut self) -> bool {
        let index = self.active_index();
        let result = match self.active {
            GameKind::Klondike => self.save_path.as_deref().map(|path| {
                save_klondike_checked(path, &self.game, &mut self.save_revisions[index])
            }),
            GameKind::Spider => self.spider_save_path.as_deref().map(|path| {
                save_spider_checked(path, &self.spider, &mut self.save_revisions[index])
            }),
            GameKind::FreeCell => self.freecell_save_path.as_deref().map(|path| {
                save_freecell_checked(path, &self.freecell, &mut self.save_revisions[index])
            }),
        };
        match result {
            Some(Ok(())) => {
                self.dirty[index] = false;
                true
            }
            Some(Err(error)) if error.committed_but_not_durable() => {
                self.dirty[index] = true;
                self.status = format!(
                    "The on-disk entry now contains the current game, but durability is indeterminate: {error}"
                );
                false
            }
            Some(Err(error)) => {
                self.dirty[index] = true;
                self.status = format!(
                    "Unsaved changes remain in memory; save failed: {error}. Retry before closing."
                );
                false
            }
            None => {
                self.dirty[index] = true;
                self.status = "Unsaved changes remain in memory; no writable save location. Retry before closing.".into();
                false
            }
        }
    }

    fn persist_mutation(&mut self) {
        let index = self.active_index();
        self.dirty[index] = true;
        let _ = self.save();
    }

    const fn active_index(&self) -> usize {
        match self.active {
            GameKind::Klondike => 0,
            GameKind::Spider => 1,
            GameKind::FreeCell => 2,
        }
    }

    const fn game_name(&self) -> &'static str {
        match self.active {
            GameKind::Klondike => "Klondike",
            GameKind::Spider => "Spider",
            GameKind::FreeCell => "FreeCell",
        }
    }

    fn apply_spider(&mut self, action: spider::Action) {
        match self.spider.apply(action) {
            Ok(()) => {
                self.spider_selection = None;
                self.status = if self.spider.state.is_won() {
                    "Spider complete — all eight runs are home".into()
                } else {
                    "Move accepted".into()
                };
                self.persist_mutation();
            }
            Err(error) => self.status = friendly_spider_error(error),
        }
    }

    fn activate_spider_tableau(&mut self, column: i32, card_index: i32) {
        let (Ok(column), Ok(index)) = (u8::try_from(column), usize::try_from(card_index)) else {
            return;
        };
        if let Some(selection) = self.spider_selection {
            if selection.column == column {
                self.spider_selection = None;
                self.status = "Selection cleared".into();
            } else {
                self.apply_spider(spider::Action::Move {
                    from: selection.column,
                    to: column,
                    count: selection.count,
                });
            }
            return;
        }
        let Some(pile) = self.spider.state.columns.get(usize::from(column)) else {
            return;
        };
        let Some(card) = pile.get(index) else {
            self.status = "Choose a run before choosing an empty column".into();
            return;
        };
        if !card.face_up {
            self.status = "That card is still face down".into();
            return;
        }
        let Ok(count) = u8::try_from(pile.len() - index) else {
            return;
        };
        self.spider_selection = Some(SpiderSelection { column, count });
        self.status = format!("Selected {} and the cards below it", card_name(card.card));
    }

    fn deal_spider_row(&mut self) {
        self.apply_spider(spider::Action::DealRow);
    }

    fn apply_freecell(&mut self, action: freecell::Action) {
        match self.freecell.apply(action) {
            Ok(()) => {
                self.freecell_selection = None;
                self.status = if self.freecell.state.is_won() {
                    "FreeCell complete — every suit is home".into()
                } else {
                    "Move accepted".into()
                };
                self.persist_mutation();
            }
            Err(error) => self.status = friendly_freecell_error(error),
        }
    }

    fn activate_freecell_pile(&mut self, pile: freecell::Pile, count: u8) {
        if let Some(selection) = self.freecell_selection {
            if selection.pile == pile {
                self.freecell_selection = None;
                self.status = "Selection cleared".into();
            } else {
                self.apply_freecell(freecell::Action {
                    from: selection.pile,
                    to: pile,
                    count: selection.count,
                });
            }
            return;
        }
        self.freecell_selection = Some(FreeCellSelection { pile, count });
        self.status = "Selected cards; choose a cascade, free cell, or foundation".into();
    }

    fn activate_freecell_cascade(&mut self, column: i32, card_index: i32) {
        let (Ok(column), Ok(index)) = (u8::try_from(column), usize::try_from(card_index)) else {
            return;
        };
        let Some(cascade) = self.freecell.state.cascades.get(usize::from(column)) else {
            return;
        };
        if cascade.get(index).is_none() && self.freecell_selection.is_none() {
            self.status = "Choose cards before choosing an empty cascade".into();
            return;
        }
        let count = if cascade.is_empty() {
            1
        } else {
            let Ok(count) = u8::try_from(cascade.len() - index) else {
                return;
            };
            count
        };
        self.activate_freecell_pile(freecell::Pile::Cascade(column), count);
    }

    fn activate_freecell_cell(&mut self, index: i32) {
        let Ok(index) = u8::try_from(index) else {
            return;
        };
        if self.freecell_selection.is_none()
            && self
                .freecell
                .state
                .free_cells
                .get(usize::from(index))
                .is_none_or(Option::is_none)
        {
            self.status = "That free cell is empty".into();
            return;
        }
        self.activate_freecell_pile(freecell::Pile::FreeCell(index), 1);
    }

    fn activate_freecell_foundation(&mut self, index: i32) {
        let Some(suit) = suit_at(index) else {
            return;
        };
        if self.freecell_selection.is_none()
            && self.freecell.state.foundations[suit_index(suit)].is_empty()
        {
            self.status = "That foundation is empty".into();
            return;
        }
        self.activate_freecell_pile(freecell::Pile::Foundation(suit), 1);
    }

    fn undo(&mut self) {
        self.clear_selections();
        let changed = match self.active {
            GameKind::Klondike => self.game.undo(),
            GameKind::Spider => self.spider.undo(),
            GameKind::FreeCell => self.freecell.undo(),
        };
        self.status = if changed {
            self.status = "Move undone".into();
            self.persist_mutation();
            self.status.clone()
        } else {
            "Nothing to undo".into()
        };
    }

    fn redo(&mut self) {
        self.clear_selections();
        let changed = match self.active {
            GameKind::Klondike => self.game.redo(),
            GameKind::Spider => self.spider.redo(),
            GameKind::FreeCell => self.freecell.redo(),
        };
        self.status = if changed {
            self.status = "Move restored".into();
            self.persist_mutation();
            self.status.clone()
        } else {
            "Nothing to redo".into()
        };
    }

    fn hint(&mut self) {
        self.status = match self.active {
            GameKind::Klondike => self.game.hint().map_or_else(
                || "No immediate move found".into(),
                |action| format!("Try {}", describe_action(&action)),
            ),
            GameKind::Spider => self.spider.hint().map_or_else(
                || "No move remains; undo or start a new deal".into(),
                |action| format!("Try {}", describe_spider_action(&action)),
            ),
            GameKind::FreeCell => self.freecell.hint().map_or_else(
                || "No immediate move found; undo or start a new deal".into(),
                |action| format!("Try {}", describe_freecell_action(action)),
            ),
        };
    }

    fn autocomplete(&mut self) {
        if self.active == GameKind::Klondike {
            let count = self.game.autocomplete();
            self.status = format!("Moved {count} safe cards to foundations");
            if count > 0 {
                self.persist_mutation();
            }
        }
    }

    fn clear_selections(&mut self) {
        self.selection = None;
        self.spider_selection = None;
        self.freecell_selection = None;
    }

    fn can_undo(&self) -> bool {
        match self.active {
            GameKind::Klondike => self.game.can_undo(),
            GameKind::Spider => self.spider.can_undo(),
            GameKind::FreeCell => self.freecell.can_undo(),
        }
    }

    fn can_redo(&self) -> bool {
        match self.active {
            GameKind::Klondike => self.game.can_redo(),
            GameKind::Spider => self.spider.can_redo(),
            GameKind::FreeCell => self.freecell.can_redo(),
        }
    }

    fn retry_save(&mut self) {
        if self.pending_new_deal.is_some() {
            if self.dirty[self.active_index()] && !self.save() {
                return;
            }
            self.commit_pending_new_deal();
        } else if !self.dirty[self.active_index()] {
            self.status = "No unsaved changes".into();
        } else if self.save() {
            self.status = "Changes saved".into();
        }
    }

    fn discard_progress_and_start_pending(&mut self) {
        self.commit_pending_new_deal();
    }

    fn cancel_pending_new_deal(&mut self) {
        if self.pending_new_deal.take().is_some() {
            self.pending_new_deal_conflict = false;
            self.status = "New deal cancelled; current game preserved".into();
        } else {
            self.status = "No new deal is waiting for confirmation".into();
        }
    }

    fn discard_unsaved_and_close(&mut self) {
        self.pending_new_deal = None;
        self.pending_new_deal_conflict = false;
        self.dirty = [false; 3];
        self.status = "Unsaved progress discarded; closing".into();
    }

    fn reload_disk_copy(&mut self) {
        let result = match self.active {
            GameKind::Klondike => self.save_path.clone().map(|path| {
                load_klondike_revisioned(&path).map(|(game, revision)| {
                    self.game = game;
                    revision
                })
            }),
            GameKind::Spider => self.spider_save_path.clone().map(|path| {
                load_spider_revisioned(&path).map(|(game, revision)| {
                    self.spider = game;
                    revision
                })
            }),
            GameKind::FreeCell => self.freecell_save_path.clone().map(|path| {
                load_freecell_revisioned(&path).map(|(game, revision)| {
                    self.freecell = game;
                    revision
                })
            }),
        };
        match result {
            Some(Ok(revision)) => {
                let index = self.active_index();
                self.save_revisions[index] = Some(revision);
                self.dirty[index] = false;
                self.pending_new_deal_conflict = false;
                self.clear_selections();
                self.status = if self.pending_new_deal.is_some() {
                    "Reloaded the newer disk copy and refreshed save ownership; retry or cancel the pending new deal".into()
                } else {
                    "Reloaded the newer disk copy; in-memory changes were discarded".into()
                };
            }
            Some(Err(error)) => {
                self.status =
                    format!("Could not reload the disk copy; in-memory changes remain: {error}");
            }
            None => self.status = "No save path is available; in-memory changes remain".into(),
        }
    }
}

fn load_or_recover<T>(
    save_path: &mut Option<PathBuf>,
    load: impl FnOnce(&std::path::Path) -> Result<RecoveredSave<T>, SaveError>,
    status: &mut String,
) -> Option<(T, SaveRevision)> {
    let path = save_path.clone()?;
    match load(&path) {
        Ok(RecoveredSave::Loaded(game, revision)) => Some((game, revision)),
        Ok(RecoveredSave::Quarantined {
            path: quarantined,
            reason,
            durability_warning,
        }) => {
            *status = durability_warning.map_or_else(
                || {
                    format!(
                        "Unreadable save preserved as {}; opened a fresh deal ({reason})",
                        quarantined.display()
                    )
                },
                |warning| {
                    format!(
                        "Unreadable save moved to {}; the original path is gone, but directory durability is indeterminate ({warning}); opened a fresh deal ({reason})",
                        quarantined.display()
                    )
                },
            );
            None
        }
        Err(SaveError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            *status = format!("Save recovery failed; original left untouched ({error})");
            *save_path = None;
            None
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    app.on_fan_spacing(bounded_fan_spacing);
    let controller = Rc::new(RefCell::new(Controller::new()));
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.window().on_close_requested(move || {
            let mut controller = controller.borrow_mut();
            if controller.dirty.iter().any(|dirty| *dirty) {
                controller.status =
                    "Unsaved changes remain. Retry save before closing the application.".into();
                if let Some(app) = weak.upgrade() {
                    render(&app, &controller);
                }
                slint::CloseRequestResponse::KeepWindowShown
            } else {
                slint::CloseRequestResponse::HideWindow
            }
        });
    }
    render(&app, &controller.borrow());
    register_klondike_handlers(&app, &controller);
    register_spider_freecell_handlers(&app, &controller);
    register_toolbar_handlers(&app, &controller);
    app.run()
}

fn register_klondike_handlers(app: &AppWindow, controller: &Rc<RefCell<Controller>>) {
    let controller = Rc::clone(controller);
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_select_game(move |game| {
            update(&weak, &controller, |state| state.select_game(game.as_str()));
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_draw_stock(move || update(&weak, &controller, Controller::draw_or_recycle));
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_waste_activated(move || update(&weak, &controller, Controller::activate_waste));
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_tableau_activated(move |column, index| {
            update(&weak, &controller, |state| {
                state.activate_tableau(column, index);
            });
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_foundation_activated(move |index| {
            update(&weak, &controller, |state| {
                state.activate_foundation(index);
            });
        });
    }
}

fn register_spider_freecell_handlers(app: &AppWindow, controller: &Rc<RefCell<Controller>>) {
    let controller = Rc::clone(controller);
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_spider_deal_stock(move || {
            update(&weak, &controller, Controller::deal_spider_row);
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_spider_tableau_activated(move |column, index| {
            update(&weak, &controller, |state| {
                state.activate_spider_tableau(column, index);
            });
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_freecell_cascade_activated(move |column, index| {
            update(&weak, &controller, |state| {
                state.activate_freecell_cascade(column, index);
            });
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_freecell_cell_activated(move |index| {
            update(&weak, &controller, |state| {
                state.activate_freecell_cell(index);
            });
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_freecell_foundation_activated(move |index| {
            update(&weak, &controller, |state| {
                state.activate_freecell_foundation(index);
            });
        });
    }
}

fn register_toolbar_handlers(app: &AppWindow, controller: &Rc<RefCell<Controller>>) {
    let controller = Rc::clone(controller);
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_new_game(move |mode| {
            update(&weak, &controller, |state| state.new_game(mode.as_str()));
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_undo_requested(move || {
            update(&weak, &controller, Controller::undo);
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_redo_requested(move || {
            update(&weak, &controller, Controller::redo);
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_hint_requested(move || {
            update(&weak, &controller, Controller::hint);
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_autocomplete_requested(move || {
            update(&weak, &controller, Controller::autocomplete);
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_retry_save_requested(move || {
            update(&weak, &controller, Controller::retry_save);
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_discard_progress_and_start_requested(move || {
            update(
                &weak,
                &controller,
                Controller::discard_progress_and_start_pending,
            );
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_cancel_new_deal_requested(move || {
            update(&weak, &controller, Controller::cancel_pending_new_deal);
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_discard_and_close_requested(move || {
            {
                let mut controller = controller.borrow_mut();
                controller.discard_unsaved_and_close();
                if let Some(app) = weak.upgrade() {
                    render(&app, &controller);
                    let _ = app.hide();
                }
            }
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_reload_disk_requested(move || {
            update(&weak, &controller, Controller::reload_disk_copy);
        });
    }
}

fn update(
    weak: &slint::Weak<AppWindow>,
    controller: &Rc<RefCell<Controller>>,
    operation: impl FnOnce(&mut Controller),
) {
    let mut controller = controller.borrow_mut();
    operation(&mut controller);
    if let Some(app) = weak.upgrade() {
        render(&app, &controller);
    }
}

fn render(app: &AppWindow, controller: &Controller) {
    app.set_game_kind(controller.game_name().into());
    app.set_can_undo(controller.can_undo());
    app.set_can_redo(controller.can_redo());
    app.set_has_unsaved_changes(controller.dirty[controller.active_index()]);
    app.set_has_any_unsaved_changes(controller.dirty.iter().any(|dirty| *dirty));
    app.set_has_pending_new_deal(controller.pending_new_deal.is_some());
    app.set_has_pending_save_conflict(controller.pending_new_deal_conflict);
    app.set_status_text(controller.status.as_str().into());
    match controller.active {
        GameKind::Klondike => render_klondike(app, controller),
        GameKind::Spider => render_spider(app, controller),
        GameKind::FreeCell => render_freecell(app, controller),
    }
}

fn render_klondike(app: &AppWindow, controller: &Controller) {
    let state = &controller.game.state;
    let columns = state
        .tableau
        .iter()
        .enumerate()
        .map(|(column, cards)| UiColumn {
            cards: ModelRc::new(VecModel::from(
                cards
                    .iter()
                    .enumerate()
                    .map(|(index, card)| {
                        let selected = matches!(
                            controller.selection,
                            Some(Selection::Tableau {
                                column: selected_column,
                                count
                            }) if usize::from(selected_column) == column
                                && index >= cards.len() - usize::from(count)
                        );
                        ui_card(card.card, card.face_up, selected)
                    })
                    .collect::<Vec<_>>(),
            )),
        })
        .collect::<Vec<_>>();
    app.set_columns(ModelRc::new(VecModel::from(columns)));

    let foundations = Suit::ALL
        .into_iter()
        .map(|suit| {
            state.foundations[suit_index(suit)].last().map_or_else(
                || empty_foundation(suit),
                |card| {
                    ui_card(
                        *card,
                        true,
                        controller.selection == Some(Selection::Foundation(suit)),
                    )
                },
            )
        })
        .collect::<Vec<_>>();
    app.set_foundations(ModelRc::new(VecModel::from(foundations)));

    if let Some(card) = state.waste.last() {
        app.set_has_waste(true);
        app.set_waste_card(ui_card(
            *card,
            true,
            controller.selection == Some(Selection::Waste),
        ));
    } else {
        app.set_has_waste(false);
        app.set_waste_card(ui_card(Card::new(Suit::Clubs, Rank::Ace), false, false));
    }
    app.set_stock_count(i32::try_from(state.stock.len()).unwrap_or_default());
    app.set_score(state.score);
    app.set_moves(i32::try_from(state.moves).unwrap_or(i32::MAX));
    app.set_completed_runs(0);
    app.set_free_cells(ModelRc::default());
    app.set_free_cell_occupied(ModelRc::default());
    app.set_longest_column(longest_column(&state.tableau));
    app.set_deal_id(i32::try_from(state.seed).unwrap_or(i32::MAX));
}

fn render_spider(app: &AppWindow, controller: &Controller) {
    let state = &controller.spider.state;
    let columns = state
        .columns
        .iter()
        .enumerate()
        .map(|(column, cards)| UiColumn {
            cards: ModelRc::new(VecModel::from(
                cards
                    .iter()
                    .enumerate()
                    .map(|(index, card)| {
                        let selected = controller.spider_selection.is_some_and(|selection| {
                            usize::from(selection.column) == column
                                && index >= cards.len() - usize::from(selection.count)
                        });
                        ui_card(card.card, card.face_up, selected)
                    })
                    .collect::<Vec<_>>(),
            )),
        })
        .collect::<Vec<_>>();
    app.set_columns(ModelRc::new(VecModel::from(columns)));
    app.set_foundations(ModelRc::default());
    app.set_free_cells(ModelRc::default());
    app.set_free_cell_occupied(ModelRc::default());
    app.set_has_waste(false);
    app.set_stock_count(i32::try_from(state.stock.len()).unwrap_or_default());
    app.set_score(state.score);
    app.set_moves(i32::try_from(state.moves).unwrap_or(i32::MAX));
    app.set_completed_runs(i32::from(state.completed_runs));
    app.set_longest_column(longest_column(&state.columns));
    app.set_deal_id(i32::try_from(state.seed).unwrap_or(i32::MAX));
}

fn render_freecell(app: &AppWindow, controller: &Controller) {
    let state = &controller.freecell.state;
    let columns = state
        .cascades
        .iter()
        .enumerate()
        .map(|(column, cards)| UiColumn {
            cards: ModelRc::new(VecModel::from(
                cards
                    .iter()
                    .enumerate()
                    .map(|(index, card)| {
                        let selected = controller.freecell_selection.is_some_and(|selection| {
                            selection.pile == freecell::Pile::Cascade(to_u8(column))
                                && index >= cards.len() - usize::from(selection.count)
                        });
                        ui_card(*card, true, selected)
                    })
                    .collect::<Vec<_>>(),
            )),
        })
        .collect::<Vec<_>>();
    app.set_columns(ModelRc::new(VecModel::from(columns)));
    let free_cells = state
        .free_cells
        .iter()
        .enumerate()
        .map(|(index, card)| {
            card.map_or_else(
                || ui_card(Card::new(Suit::Clubs, Rank::Ace), false, false),
                |card| {
                    ui_card(
                        card,
                        true,
                        controller.freecell_selection.is_some_and(|selection| {
                            selection.pile == freecell::Pile::FreeCell(to_u8(index))
                        }),
                    )
                },
            )
        })
        .collect::<Vec<_>>();
    app.set_free_cells(ModelRc::new(VecModel::from(free_cells)));
    app.set_free_cell_occupied(ModelRc::new(VecModel::from(
        state
            .free_cells
            .iter()
            .map(Option::is_some)
            .collect::<Vec<_>>(),
    )));
    let foundations = Suit::ALL
        .into_iter()
        .map(|suit| {
            state.foundations[suit_index(suit)].last().map_or_else(
                || empty_foundation(suit),
                |card| {
                    ui_card(
                        *card,
                        true,
                        controller.freecell_selection.is_some_and(|selection| {
                            selection.pile == freecell::Pile::Foundation(suit)
                        }),
                    )
                },
            )
        })
        .collect::<Vec<_>>();
    app.set_foundations(ModelRc::new(VecModel::from(foundations)));
    app.set_has_waste(false);
    app.set_stock_count(0);
    app.set_score(
        i32::try_from(state.foundations.iter().map(Vec::len).sum::<usize>()).unwrap_or(i32::MAX),
    );
    app.set_moves(i32::try_from(state.moves).unwrap_or(i32::MAX));
    app.set_completed_runs(0);
    app.set_longest_column(longest_column(&state.cascades));
    app.set_deal_id(i32::try_from(state.deal_number).unwrap_or(i32::MAX));
}

fn longest_column<T, const N: usize>(columns: &[Vec<T>; N]) -> i32 {
    columns
        .iter()
        .map(Vec::len)
        .max()
        .and_then(|length| i32::try_from(length).ok())
        .unwrap_or_default()
}

fn ui_card(card: Card, face_up: bool, selected: bool) -> UiCard {
    UiCard {
        label: if face_up {
            card_label(card).into()
        } else {
            SharedString::default()
        },
        red: matches!(card.suit, Suit::Diamonds | Suit::Hearts),
        face_up,
        selected,
        accessible_label: if face_up {
            card_name(card).into()
        } else {
            "Face-down card".into()
        },
    }
}

fn empty_foundation(suit: Suit) -> UiCard {
    UiCard {
        label: suit_symbol(suit).into(),
        red: matches!(suit, Suit::Diamonds | Suit::Hearts),
        face_up: true,
        selected: false,
        accessible_label: format!("Empty {} foundation", suit_name(suit)).into(),
    }
}

fn card_label(card: Card) -> String {
    format!("{}{}", rank_label(card.rank), suit_symbol(card.suit))
}

fn card_name(card: Card) -> String {
    format!("{} of {}", rank_name(card.rank), suit_name(card.suit))
}

const fn rank_label(rank: Rank) -> &'static str {
    match rank {
        Rank::Ace => "A",
        Rank::Two => "2",
        Rank::Three => "3",
        Rank::Four => "4",
        Rank::Five => "5",
        Rank::Six => "6",
        Rank::Seven => "7",
        Rank::Eight => "8",
        Rank::Nine => "9",
        Rank::Ten => "10",
        Rank::Jack => "J",
        Rank::Queen => "Q",
        Rank::King => "K",
    }
}

const fn rank_name(rank: Rank) -> &'static str {
    match rank {
        Rank::Ace => "Ace",
        Rank::Two => "Two",
        Rank::Three => "Three",
        Rank::Four => "Four",
        Rank::Five => "Five",
        Rank::Six => "Six",
        Rank::Seven => "Seven",
        Rank::Eight => "Eight",
        Rank::Nine => "Nine",
        Rank::Ten => "Ten",
        Rank::Jack => "Jack",
        Rank::Queen => "Queen",
        Rank::King => "King",
    }
}

const fn suit_symbol(suit: Suit) -> &'static str {
    match suit {
        Suit::Clubs => "♣",
        Suit::Diamonds => "♦",
        Suit::Hearts => "♥",
        Suit::Spades => "♠",
    }
}

const fn suit_name(suit: Suit) -> &'static str {
    match suit {
        Suit::Clubs => "clubs",
        Suit::Diamonds => "diamonds",
        Suit::Hearts => "hearts",
        Suit::Spades => "spades",
    }
}

const fn suit_index(suit: Suit) -> usize {
    match suit {
        Suit::Clubs => 0,
        Suit::Diamonds => 1,
        Suit::Hearts => 2,
        Suit::Spades => 3,
    }
}

const fn suit_at(index: i32) -> Option<Suit> {
    match index {
        0 => Some(Suit::Clubs),
        1 => Some(Suit::Diamonds),
        2 => Some(Suit::Hearts),
        3 => Some(Suit::Spades),
        _ => None,
    }
}

fn seed_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(1, |duration| duration.as_secs())
}

fn describe_action(action: &Action) -> String {
    match action {
        Action::Draw => "drawing from the stock".into(),
        Action::Recycle => "recycling the waste".into(),
        Action::Move { from, to, count } => {
            format!("moving {count} card(s) from {from:?} to {to:?}")
        }
    }
}

fn describe_spider_action(action: &spider::Action) -> String {
    match action {
        spider::Action::DealRow => "dealing one card onto every column".into(),
        spider::Action::Move { from, to, count } => {
            format!(
                "moving {count} card(s) from column {} to {}",
                from + 1,
                to + 1
            )
        }
    }
}

fn describe_freecell_action(action: freecell::Action) -> String {
    format!(
        "moving {} card(s) from {:?} to {:?}",
        action.count, action.from, action.to
    )
}

fn friendly_error(error: solitaire::klondike::MoveError) -> String {
    use solitaire::klondike::MoveError;
    match error {
        MoveError::EmptyStock => "The stock is empty; recycle the waste".into(),
        MoveError::CannotRecycle => "Draw the remaining stock before recycling".into(),
        MoveError::RedealLimitReached => "No redeals remain".into(),
        MoveError::InvalidTableau => {
            "Build downward in alternating colors; spaces need kings".into()
        }
        MoveError::InvalidFoundation => {
            "Foundations build upward by suit, starting with aces".into()
        }
        MoveError::FaceDownCard => "Face-down cards cannot move".into(),
        MoveError::BrokenRun => "That group is not a complete alternating run".into(),
        MoveError::ResourceLimit => {
            "This deal reached the 4096-action replay limit; start a new deal to continue".into()
        }
        _ => "That move is not available".into(),
    }
}

fn friendly_spider_error(error: spider::MoveError) -> String {
    match error {
        spider::MoveError::EmptyStock => "No stock rows remain".into(),
        spider::MoveError::EmptyColumnDuringDeal => {
            "Fill every empty column before dealing another row".into()
        }
        spider::MoveError::BrokenRun => "Only a descending same-suit run can move together".into(),
        spider::MoveError::InvalidDestination => {
            "Build downward by rank, or move onto an empty column".into()
        }
        spider::MoveError::ResourceLimit => {
            "This deal reached the 4096-action replay limit; start a new deal to continue".into()
        }
        _ => "That Spider move is not available".into(),
    }
}

fn friendly_freecell_error(error: freecell::MoveError) -> String {
    match error {
        freecell::MoveError::BrokenRun | freecell::MoveError::InvalidCascade => {
            "Cascades build downward in alternating colors".into()
        }
        freecell::MoveError::SupermoveTooLarge => {
            "That run needs more open free cells or empty cascades".into()
        }
        freecell::MoveError::OccupiedFreeCell => "That free cell is occupied".into(),
        freecell::MoveError::InvalidFoundation => {
            "Foundations build upward by suit, starting with aces".into()
        }
        freecell::MoveError::ResourceLimit => {
            "This deal reached the 4096-action replay limit; start a new deal to continue".into()
        }
        _ => "That FreeCell move is not available".into(),
    }
}

fn bounded_fan_spacing(card_count: i32, available_height: f32) -> f32 {
    const CARD_HEIGHT: f32 = 142.0;
    const MIN_SPACING: f32 = 1.0;
    const MAX_SPACING: f32 = 30.0;
    if card_count <= 1 {
        return MAX_SPACING;
    }
    let divisor = i16::try_from(card_count - 1).unwrap_or(i16::MAX);
    ((available_height - CARD_HEIGHT) / f32::from(divisor)).clamp(MIN_SPACING, MAX_SPACING)
}

fn to_u8(value: usize) -> u8 {
    u8::try_from(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_save(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "solitaire-controller-{}-{name}.json",
            std::process::id()
        ))
    }

    fn remove_save(path: &std::path::Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(path.with_extension("json.lock"));
    }

    fn controller(seed: u64) -> Controller {
        Controller {
            active: GameKind::Klondike,
            game: Game::new(seed, Options::default()),
            selection: None,
            save_path: None,
            spider: SpiderGame::new(seed, SuitMode::One),
            spider_selection: None,
            spider_save_path: None,
            freecell: FreeCellGame::new(seed),
            freecell_selection: None,
            freecell_save_path: None,
            save_revisions: [None; 3],
            dirty: [false; 3],
            pending_new_deal: None,
            pending_new_deal_conflict: false,
            deal_counters_path: None,
            next_seeds: DealCounters {
                klondike: seed.saturating_add(1),
                spider: seed.saturating_add(1),
                freecell: seed.saturating_add(1),
            },
            status: "Ready".into(),
        }
    }

    #[test]
    fn fan_spacing_keeps_long_legal_columns_visible() {
        let spacing = bounded_fan_spacing(19, 410.0);
        assert!((1.0..=30.0).contains(&spacing));
        assert!(spacing * 18.0 + 142.0 <= 410.0 + f32::EPSILON);
        assert!((bounded_fan_spacing(1, 410.0) - 30.0).abs() < f32::EPSILON);
        let extreme = bounded_fan_spacing(104, 410.0);
        assert!(extreme * 103.0 + 142.0 <= 410.0 + f32::EPSILON);
    }

    #[test]
    fn spider_surface_routes_variants_moves_and_history() {
        let mut controller = controller(41);
        controller.select_game("Spider");
        let path = test_save("spider-surface");
        remove_save(&path);
        controller.spider_save_path = Some(path.clone());
        controller.new_game("2 suits");
        assert_eq!(controller.spider.state.mode, SuitMode::Two);
        assert_eq!(controller.spider.state.stock.len(), 50);

        let action = controller.spider.hint().unwrap();
        match action {
            spider::Action::DealRow => controller.deal_spider_row(),
            spider::Action::Move { from, to, count } => {
                let source = &controller.spider.state.columns[usize::from(from)];
                let index = source.len() - usize::from(count);
                controller.activate_spider_tableau(from.into(), to_i32(index));
                controller.activate_spider_tableau(to.into(), 0);
            }
        }
        assert!(controller.spider.can_undo());
        let moved = controller.spider.state.clone();
        controller.undo();
        assert!(controller.spider.can_redo());
        controller.redo();
        assert_eq!(controller.spider.state, moved);
        remove_save(&path);
    }

    #[test]
    fn freecell_surface_routes_piles_and_history() {
        let mut controller = controller(73);
        controller.select_game("FreeCell");
        let action = controller.freecell.hint().unwrap();
        controller.activate_freecell_pile(action.from, action.count);
        controller.activate_freecell_pile(action.to, 1);
        assert!(controller.freecell.can_undo());
        let moved = controller.freecell.state.clone();
        controller.undo();
        assert!(controller.freecell.can_redo());
        controller.redo();
        assert_eq!(controller.freecell.state, moved);
    }

    #[test]
    fn undo_redo_never_hide_a_save_failure_and_retry_remains_available() {
        let mut controller = controller(91);
        controller.apply(Action::Draw);
        assert!(controller.dirty[0]);
        assert!(controller.status.contains("Unsaved changes remain"));
        controller.undo();
        assert!(controller.dirty[0]);
        assert!(controller.status.contains("Unsaved changes remain"));
        controller.redo();
        assert!(controller.dirty[0]);
        assert!(controller.status.contains("Unsaved changes remain"));
        controller.retry_save();
        assert!(controller.status.contains("Unsaved changes remain"));
    }

    #[test]
    fn dirty_new_deal_requires_an_explicit_choice_and_no_path_preserves_state() {
        let mut controller = controller(101);
        controller.apply(Action::Draw);
        let current = controller.game.clone();

        controller.new_game("Draw 3");
        assert_eq!(controller.game, current);
        assert!(controller.pending_new_deal.is_some());
        assert!(controller.status.contains("Retry save"));

        controller.discard_progress_and_start_pending();
        assert_eq!(controller.game, current);
        assert!(controller.pending_new_deal.is_some());
        assert!(controller.status.contains("no writable save location"));

        controller.cancel_pending_new_deal();
        assert_eq!(controller.game, current);
        assert!(controller.pending_new_deal.is_none());
        assert!(controller.dirty[0]);
    }

    #[test]
    fn discard_stages_a_new_deal_until_the_prospective_save_succeeds() {
        let path = test_save("discard-new-deal");
        remove_save(&path);
        let mut controller = controller(202);
        controller.save_path = Some(path.clone());
        assert!(controller.save());
        controller.dirty[0] = true;
        let old_seed = controller.game.state.seed;

        controller.new_game("Draw 3");
        assert_eq!(controller.game.state.seed, old_seed);
        controller.discard_progress_and_start_pending();

        assert_ne!(controller.game.state.seed, old_seed);
        assert_eq!(controller.game.state.options.draw_mode, DrawMode::Three);
        assert!(!controller.dirty[0]);
        assert!(controller.pending_new_deal.is_none());
        let (saved, _) = load_klondike_revisioned(&path).unwrap();
        assert_eq!(saved, controller.game);
        remove_save(&path);
    }

    #[test]
    fn retry_saves_dirty_progress_before_committing_the_pending_deal() {
        let path = test_save("retry-new-deal");
        remove_save(&path);
        let mut controller = controller(303);
        controller.save_path = Some(path.clone());
        assert!(controller.save());
        controller.dirty[0] = true;
        controller.new_game("Draw 1");

        controller.retry_save();

        assert!(controller.pending_new_deal.is_none());
        assert!(!controller.dirty[0]);
        assert_eq!(controller.game.state.seed, 304);
        let (saved, _) = load_klondike_revisioned(&path).unwrap();
        assert_eq!(saved, controller.game);
        remove_save(&path);
    }

    #[test]
    fn clean_pending_deal_conflict_can_reload_ownership_and_retry() {
        let path = test_save("clean-pending-conflict");
        remove_save(&path);
        let mut controller = controller(350);
        controller.save_path = Some(path.clone());
        assert!(controller.save());
        let original = controller.game.clone();
        let newer_disk_game = Game::new(999, Options::default());
        solitaire::persistence::save_klondike(&path, &newer_disk_game).unwrap();

        controller.new_game("Draw 3");

        assert_eq!(controller.game, original);
        assert!(!controller.dirty[0]);
        assert!(controller.pending_new_deal.is_some());
        assert!(controller.pending_new_deal_conflict);
        assert!(controller.status.contains("save changed in another"));

        controller.reload_disk_copy();
        assert_eq!(controller.game, newer_disk_game);
        assert!(controller.pending_new_deal.is_some());
        assert!(!controller.pending_new_deal_conflict);
        assert!(controller.status.contains("refreshed save ownership"));

        controller.retry_save();
        assert!(controller.pending_new_deal.is_none());
        assert!(!controller.dirty[0]);
        assert_ne!(controller.game, newer_disk_game);
        assert_eq!(controller.game.state.options.draw_mode, DrawMode::Three);
        let (saved, _) = load_klondike_revisioned(&path).unwrap();
        assert_eq!(saved, controller.game);
        remove_save(&path);
    }

    #[test]
    fn discard_and_close_explicitly_releases_the_close_guard() {
        let mut controller = controller(404);
        controller.dirty = [true; 3];
        controller.new_game("Draw 1");
        assert!(controller.pending_new_deal.is_some());

        controller.discard_unsaved_and_close();

        assert_eq!(controller.dirty, [false; 3]);
        assert!(controller.pending_new_deal.is_none());
        assert!(controller.status.contains("closing"));
    }

    fn to_i32(value: usize) -> i32 {
        i32::try_from(value).unwrap_or(i32::MAX)
    }
}
