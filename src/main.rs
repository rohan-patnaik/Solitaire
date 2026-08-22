use slint::{ModelRc, SharedString, VecModel};
use solitaire::cards::{Card, Rank, Suit};
use solitaire::freecell::{self, Game as FreeCellGame};
use solitaire::klondike::{Action, DrawMode, Game, Options, Pile, Scoring};
use solitaire::persistence::{
    DealCounters, DealKind, RecoveredSave, SaveError, SaveRevision, confirm_current_save_revision,
    default_deal_counters_path, default_freecell_save_path, default_pyramid_save_path,
    default_save_path, default_spider_save_path, default_tripeaks_save_path, load_deal_counters,
    load_freecell_revisioned, load_klondike_revisioned, load_pyramid_revisioned,
    load_spider_revisioned, load_tripeaks_revisioned, recover_freecell_revisioned,
    recover_klondike_revisioned, recover_pyramid_revisioned, recover_spider_revisioned,
    recover_tripeaks_revisioned, reserve_deal, save_freecell_checked, save_klondike_checked,
    save_pyramid_checked, save_spider_checked, save_tripeaks_checked,
};
use solitaire::pyramid::{self, Game as PyramidGame};
use solitaire::spider::{self, Game as SpiderGame, SuitMode};
use solitaire::tripeaks::{self, Game as TriPeaksGame};
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
    TriPeaks,
    Pyramid,
}

struct PendingNewDeal {
    game: GameKind,
    variant: String,
}

enum ProspectiveGame {
    Klondike(Game),
    Spider(SpiderGame),
    FreeCell(FreeCellGame),
    TriPeaks(TriPeaksGame),
    Pyramid(PyramidGame),
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
    tripeaks: TriPeaksGame,
    tripeaks_save_path: Option<PathBuf>,
    pyramid: PyramidGame,
    pyramid_selection: Option<pyramid::Source>,
    pyramid_save_path: Option<PathBuf>,
    save_revisions: [Option<SaveRevision>; 5],
    dirty: [bool; 5],
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
        let mut tripeaks_save_path = default_tripeaks_save_path();
        let (tripeaks, tripeaks_revision) = load_or_recover(
            &mut tripeaks_save_path,
            recover_tripeaks_revisioned,
            &mut status,
        )
        .map_or_else(
            || {
                (
                    TriPeaksGame::new(seed.wrapping_add(3), tripeaks::Options::default()),
                    None,
                )
            },
            |(game, revision)| (game, Some(revision)),
        );
        let (pyramid, pyramid_save_path, pyramid_revision) =
            load_initial_pyramid(seed, &mut status);
        let deal_counters_path = default_deal_counters_path();
        let defaults = DealCounters {
            klondike: saved
                .as_ref()
                .map_or(seed, |(game, _)| game.state.seed)
                .saturating_add(1),
            spider: spider.state.seed.saturating_add(1),
            freecell: freecell.state.deal_number.saturating_add(1),
            tripeaks: tripeaks.state.seed.saturating_add(1),
            pyramid: pyramid.state.seed.saturating_add(1),
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
        raise_deal_counters(&mut next_seeds, defaults);
        let save_revisions = [
            klondike_revision,
            spider_revision,
            freecell_revision,
            tripeaks_revision,
            pyramid_revision,
        ];
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
            tripeaks,
            tripeaks_save_path,
            pyramid,
            pyramid_selection: None,
            pyramid_save_path,
            save_revisions,
            dirty: [false; 5],
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
            "TriPeaks" => GameKind::TriPeaks,
            "Pyramid" => GameKind::Pyramid,
            _ => GameKind::Klondike,
        };
        self.selection = None;
        self.spider_selection = None;
        self.freecell_selection = None;
        self.pyramid_selection = None;
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
            GameKind::TriPeaks => {
                ProspectiveGame::TriPeaks(TriPeaksGame::new(seed, tripeaks::Options::default()))
            }
            GameKind::Pyramid => {
                ProspectiveGame::Pyramid(PyramidGame::new(seed, pyramid::Options::default()))
            }
        };
        let index = self.active_index();
        let saved = match &candidate {
            ProspectiveGame::Klondike(game) => self
                .save_path
                .as_deref()
                .map(|path| save_klondike_checked(path, game, &mut self.save_revisions[index])),
            ProspectiveGame::Spider(game) => self
                .spider_save_path
                .as_deref()
                .map(|path| save_spider_checked(path, game, &mut self.save_revisions[index])),
            ProspectiveGame::FreeCell(game) => self
                .freecell_save_path
                .as_deref()
                .map(|path| save_freecell_checked(path, game, &mut self.save_revisions[index])),
            ProspectiveGame::TriPeaks(game) => self
                .tripeaks_save_path
                .as_deref()
                .map(|path| save_tripeaks_checked(path, game, &mut self.save_revisions[index])),
            ProspectiveGame::Pyramid(game) => self
                .pyramid_save_path
                .as_deref()
                .map(|path| save_pyramid_checked(path, game, &mut self.save_revisions[index])),
        };
        match saved {
            Some(Ok(())) => {
                self.replace_game(candidate);
                self.dirty[index] = false;
                self.pending_new_deal_conflict = false;
                self.clear_selections();
                self.status = format!("New {} deal", self.game_name());
            }
            Some(Err(error)) if error.committed_but_not_durable() => {
                self.replace_game(candidate);
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

    fn replace_game(&mut self, candidate: ProspectiveGame) {
        match candidate {
            ProspectiveGame::Klondike(game) => self.game = game,
            ProspectiveGame::Spider(game) => self.spider = game,
            ProspectiveGame::FreeCell(game) => self.freecell = game,
            ProspectiveGame::TriPeaks(game) => self.tripeaks = game,
            ProspectiveGame::Pyramid(game) => self.pyramid = game,
        }
    }

    fn take_next_seed(&mut self, game: GameKind) -> Option<u64> {
        let kind = match game {
            GameKind::Klondike => DealKind::Klondike,
            GameKind::Spider => DealKind::Spider,
            GameKind::FreeCell => DealKind::FreeCell,
            GameKind::TriPeaks => DealKind::TriPeaks,
            GameKind::Pyramid => DealKind::Pyramid,
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
            GameKind::TriPeaks => self.next_seeds.tripeaks,
            GameKind::Pyramid => self.next_seeds.pyramid,
        };
        let next = seed.checked_add(1)?;
        match game {
            GameKind::Klondike => self.next_seeds.klondike = next,
            GameKind::Spider => self.next_seeds.spider = next,
            GameKind::FreeCell => self.next_seeds.freecell = next,
            GameKind::TriPeaks => self.next_seeds.tripeaks = next,
            GameKind::Pyramid => self.next_seeds.pyramid = next,
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
            GameKind::TriPeaks => self.tripeaks_save_path.as_deref().map(|path| {
                save_tripeaks_checked(path, &self.tripeaks, &mut self.save_revisions[index])
            }),
            GameKind::Pyramid => self.pyramid_save_path.as_deref().map(|path| {
                save_pyramid_checked(path, &self.pyramid, &mut self.save_revisions[index])
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
            GameKind::TriPeaks => 3,
            GameKind::Pyramid => 4,
        }
    }

    const fn game_name(&self) -> &'static str {
        match self.active {
            GameKind::Klondike => "Klondike",
            GameKind::Spider => "Spider",
            GameKind::FreeCell => "FreeCell",
            GameKind::TriPeaks => "TriPeaks",
            GameKind::Pyramid => "Pyramid",
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

    fn apply_tripeaks(&mut self, action: tripeaks::Action) {
        match self.tripeaks.apply(action) {
            Ok(()) => {
                self.status = if self.tripeaks.state.is_won() {
                    "TriPeaks complete — all three peaks are clear".into()
                } else {
                    "Move accepted".into()
                };
                self.persist_mutation();
            }
            Err(error) => self.status = friendly_tripeaks_error(error),
        }
    }

    fn activate_tripeaks_card(&mut self, index: i32) {
        let Ok(index) = u8::try_from(index) else {
            return;
        };
        self.apply_tripeaks(tripeaks::Action::Remove(index));
    }

    fn draw_tripeaks_stock(&mut self) {
        self.apply_tripeaks(tripeaks::Action::Draw);
    }

    fn apply_pyramid(&mut self, action: pyramid::Action) {
        match self.pyramid.apply(action) {
            Ok(()) => {
                self.pyramid_selection = None;
                self.status = if self.pyramid.state.is_won() {
                    "Pyramid complete — every tableau card is clear".into()
                } else {
                    "Move accepted".into()
                };
                self.persist_mutation();
            }
            Err(error) => self.status = friendly_pyramid_error(error),
        }
    }

    fn activate_pyramid_card(&mut self, index: i32) {
        let Ok(index) = u8::try_from(index) else {
            return;
        };
        self.activate_pyramid_source(pyramid::Source::Pyramid(index));
    }

    fn activate_pyramid_waste(&mut self) {
        self.activate_pyramid_source(pyramid::Source::Waste);
    }

    fn activate_pyramid_source(&mut self, source: pyramid::Source) {
        let Some(card) = pyramid_source_card(&self.pyramid.state, source) else {
            self.status = match source {
                pyramid::Source::Pyramid(_) => "That Pyramid card is covered or empty".into(),
                pyramid::Source::Waste => "The Pyramid waste is empty".into(),
            };
            return;
        };
        if self.pyramid_selection == Some(source) {
            self.pyramid_selection = None;
            self.status = "Pyramid selection cleared".into();
        } else if let Some(first) = self.pyramid_selection {
            self.apply_pyramid(pyramid::Action::RemovePair(first, source));
        } else if card.rank == Rank::King {
            self.apply_pyramid(pyramid::Action::RemoveKing(source));
        } else {
            self.pyramid_selection = Some(source);
            self.status = format!("Selected {}; choose a card that makes 13", card_name(card));
        }
    }

    fn draw_pyramid_stock(&mut self) {
        let action = if self.pyramid.state.stock.is_empty() {
            pyramid::Action::Recycle
        } else {
            pyramid::Action::Draw
        };
        self.apply_pyramid(action);
    }

    fn undo(&mut self) {
        self.clear_selections();
        let changed = match self.active {
            GameKind::Klondike => self.game.undo(),
            GameKind::Spider => self.spider.undo(),
            GameKind::FreeCell => self.freecell.undo(),
            GameKind::TriPeaks => self.tripeaks.undo(),
            GameKind::Pyramid => self.pyramid.undo(),
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
            GameKind::TriPeaks => self.tripeaks.redo(),
            GameKind::Pyramid => self.pyramid.redo(),
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
            GameKind::TriPeaks => self.tripeaks.hint().map_or_else(
                || "No move remains; undo or start a new deal".into(),
                |action| format!("Try {}", describe_tripeaks_action(action)),
            ),
            GameKind::Pyramid => self.pyramid.hint().map_or_else(
                || "No move remains; undo or start a new deal".into(),
                |action| format!("Try {}", describe_pyramid_action(action)),
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
        self.pyramid_selection = None;
    }

    fn can_undo(&self) -> bool {
        match self.active {
            GameKind::Klondike => self.game.can_undo(),
            GameKind::Spider => self.spider.can_undo(),
            GameKind::FreeCell => self.freecell.can_undo(),
            GameKind::TriPeaks => self.tripeaks.can_undo(),
            GameKind::Pyramid => self.pyramid.can_undo(),
        }
    }

    fn can_redo(&self) -> bool {
        match self.active {
            GameKind::Klondike => self.game.can_redo(),
            GameKind::Spider => self.spider.can_redo(),
            GameKind::FreeCell => self.freecell.can_redo(),
            GameKind::TriPeaks => self.tripeaks.can_redo(),
            GameKind::Pyramid => self.pyramid.can_redo(),
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

    fn confirm_missing_save_ownership(&mut self) {
        let index = self.active_index();
        if self.dirty[index] || self.pending_new_deal.is_none() || !self.pending_new_deal_conflict {
            self.status =
                "Missing-save ownership can only be refreshed for a clean pending deal conflict"
                    .into();
            return;
        }
        let path = match self.active {
            GameKind::Klondike => self.save_path.as_deref(),
            GameKind::Spider => self.spider_save_path.as_deref(),
            GameKind::FreeCell => self.freecell_save_path.as_deref(),
            GameKind::TriPeaks => self.tripeaks_save_path.as_deref(),
            GameKind::Pyramid => self.pyramid_save_path.as_deref(),
        };
        let Some(path) = path else {
            self.status = "No save path is available; ownership was not changed".into();
            return;
        };
        match confirm_current_save_revision(path) {
            Ok(None) => {
                self.save_revisions[index] = None;
                self.pending_new_deal_conflict = false;
                self.status = "Confirmed under lock that the save is missing; ownership was refreshed. Retry or cancel the pending new deal.".into();
            }
            Ok(Some(_)) => {
                self.status = "The save path still exists; ownership was not changed. Reload the disk copy or cancel the pending new deal.".into();
            }
            Err(error) => {
                self.status = format!(
                    "Could not confirm that the save is missing; ownership was not changed: {error}"
                );
            }
        }
    }

    fn discard_unsaved_and_close(&mut self) {
        self.pending_new_deal = None;
        self.pending_new_deal_conflict = false;
        self.dirty = [false; 5];
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
            GameKind::TriPeaks => self.tripeaks_save_path.clone().map(|path| {
                load_tripeaks_revisioned(&path).map(|(game, revision)| {
                    self.tripeaks = game;
                    revision
                })
            }),
            GameKind::Pyramid => self.pyramid_save_path.clone().map(|path| {
                load_pyramid_revisioned(&path).map(|(game, revision)| {
                    self.pyramid = game;
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

fn load_initial_pyramid(
    seed: u64,
    status: &mut String,
) -> (PyramidGame, Option<PathBuf>, Option<SaveRevision>) {
    let mut path = default_pyramid_save_path();
    let (game, revision) = load_or_recover(&mut path, recover_pyramid_revisioned, status)
        .map_or_else(
            || {
                (
                    PyramidGame::new(seed.wrapping_add(4), pyramid::Options::default()),
                    None,
                )
            },
            |(game, revision)| (game, Some(revision)),
        );
    (game, path, revision)
}

fn raise_deal_counters(counters: &mut DealCounters, minimum: DealCounters) {
    counters.klondike = counters.klondike.max(minimum.klondike);
    counters.spider = counters.spider.max(minimum.spider);
    counters.freecell = counters.freecell.max(minimum.freecell);
    counters.tripeaks = counters.tripeaks.max(minimum.tripeaks);
    counters.pyramid = counters.pyramid.max(minimum.pyramid);
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
    app.on_tripeaks_x(tripeaks_x);
    app.on_pyramid_x(pyramid_x);
    app.on_pyramid_y(pyramid_y);
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
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_tripeaks_draw_stock(move || {
            update(&weak, &controller, Controller::draw_tripeaks_stock);
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_tripeaks_tableau_activated(move |index| {
            update(&weak, &controller, |state| {
                state.activate_tripeaks_card(index);
            });
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_pyramid_draw_stock(move || {
            update(&weak, &controller, Controller::draw_pyramid_stock);
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_pyramid_waste_activated(move || {
            update(&weak, &controller, Controller::activate_pyramid_waste);
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_pyramid_tableau_activated(move |index| {
            update(&weak, &controller, |state| {
                state.activate_pyramid_card(index);
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
        app.on_confirm_missing_save_requested(move || {
            update(
                &weak,
                &controller,
                Controller::confirm_missing_save_ownership,
            );
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.on_discard_and_close_requested(move || {
            let mut controller = controller.borrow_mut();
            controller.discard_unsaved_and_close();
            if let Some(app) = weak.upgrade() {
                render(&app, &controller);
                let _ = app.hide();
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
        GameKind::TriPeaks => render_tripeaks(app, controller),
        GameKind::Pyramid => render_pyramid(app, controller),
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
    app.set_tripeaks_cards(ModelRc::default());
    app.set_pyramid_cards(ModelRc::default());
    app.set_longest_column(longest_column(&state.tableau));
    app.set_deal_id(i32::try_from(state.seed).unwrap_or(i32::MAX));
    app.set_deal_number(SharedString::default());
    app.set_redeals(0);
    app.set_redeals_remaining(0);
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
    app.set_tripeaks_cards(ModelRc::default());
    app.set_pyramid_cards(ModelRc::default());
    app.set_has_waste(false);
    app.set_stock_count(i32::try_from(state.stock.len()).unwrap_or_default());
    app.set_score(state.score);
    app.set_moves(i32::try_from(state.moves).unwrap_or(i32::MAX));
    app.set_completed_runs(i32::from(state.completed_runs));
    app.set_longest_column(longest_column(&state.columns));
    app.set_deal_id(i32::try_from(state.seed).unwrap_or(i32::MAX));
    app.set_deal_number(SharedString::default());
    app.set_redeals(0);
    app.set_redeals_remaining(0);
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
    app.set_tripeaks_cards(ModelRc::default());
    app.set_pyramid_cards(ModelRc::default());
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
    app.set_deal_number(SharedString::default());
    app.set_redeals(0);
    app.set_redeals_remaining(0);
}

fn render_tripeaks(app: &AppWindow, controller: &Controller) {
    let state = &controller.tripeaks.state;
    let cards = state
        .tableau
        .iter()
        .enumerate()
        .map(|(index, card)| {
            let present = card.is_some();
            let exposed = state.is_exposed(index);
            let card = card.unwrap_or(Card::new(Suit::Clubs, Rank::Ace));
            let position = index + 1;
            UiTriPeaksCard {
                card: tripeaks_ui_card(card, exposed, position),
                present,
            }
        })
        .collect::<Vec<_>>();
    app.set_tripeaks_cards(ModelRc::new(VecModel::from(cards)));
    app.set_pyramid_cards(ModelRc::default());
    app.set_columns(ModelRc::default());
    app.set_foundations(ModelRc::default());
    app.set_free_cells(ModelRc::default());
    app.set_free_cell_occupied(ModelRc::default());
    if let Some(card) = state.waste.last() {
        app.set_has_waste(true);
        app.set_waste_card(ui_card_labeled(
            *card,
            format!(
                "Waste card, {}; activate to draw the next stock card",
                card_name(*card)
            ),
        ));
    } else {
        app.set_has_waste(false);
        app.set_waste_card(ui_card(Card::new(Suit::Clubs, Rank::Ace), false, false));
    }
    app.set_stock_count(i32::try_from(state.stock.len()).unwrap_or_default());
    app.set_score(i32::try_from(state.score).unwrap_or(i32::MAX));
    app.set_moves(i32::try_from(state.moves).unwrap_or(i32::MAX));
    app.set_completed_runs(0);
    app.set_longest_column(0);
    app.set_deal_id(0);
    app.set_deal_number(tripeaks_deal_number(state.seed));
    app.set_redeals(0);
    app.set_redeals_remaining(0);
}

fn render_pyramid(app: &AppWindow, controller: &Controller) {
    let state = &controller.pyramid.state;
    let cards = state
        .pyramid
        .iter()
        .enumerate()
        .map(|(index, card)| {
            let present = card.is_some();
            let exposed = state.is_exposed(index);
            let card = card.unwrap_or(Card::new(Suit::Clubs, Rank::Ace));
            let source = pyramid::Source::Pyramid(to_u8(index));
            UiPyramidCard {
                card: pyramid_ui_card(
                    card,
                    exposed,
                    controller.pyramid_selection == Some(source),
                    index + 1,
                ),
                present,
            }
        })
        .collect::<Vec<_>>();
    app.set_pyramid_cards(ModelRc::new(VecModel::from(cards)));
    app.set_tripeaks_cards(ModelRc::default());
    app.set_columns(ModelRc::default());
    app.set_foundations(ModelRc::default());
    app.set_free_cells(ModelRc::default());
    app.set_free_cell_occupied(ModelRc::default());
    if let Some(card) = state.waste.last() {
        let selected = controller.pyramid_selection == Some(pyramid::Source::Waste);
        app.set_has_waste(true);
        app.set_waste_card(UiCard {
            label: card_label(*card).into(),
            red: matches!(card.suit, Suit::Diamonds | Suit::Hearts),
            face_up: true,
            selected,
            accessible_label: format!(
                "Pyramid waste, {}{}; activate to select or remove",
                card_name(*card),
                if selected { ", selected" } else { "" }
            )
            .into(),
        });
    } else {
        app.set_has_waste(false);
        app.set_waste_card(ui_card(Card::new(Suit::Clubs, Rank::Ace), false, false));
    }
    app.set_stock_count(i32::try_from(state.stock.len()).unwrap_or_default());
    app.set_score(i32::try_from(state.score).unwrap_or(i32::MAX));
    app.set_moves(i32::try_from(state.moves).unwrap_or(i32::MAX));
    app.set_completed_runs(0);
    app.set_longest_column(0);
    app.set_deal_id(0);
    app.set_deal_number(pyramid_deal_number(state.seed));
    app.set_redeals(i32::from(state.redeals));
    app.set_redeals_remaining(i32::from(
        state.options.max_redeals.saturating_sub(state.redeals),
    ));
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

fn ui_card_labeled(card: Card, accessible_label: String) -> UiCard {
    UiCard {
        label: card_label(card).into(),
        red: matches!(card.suit, Suit::Diamonds | Suit::Hearts),
        face_up: true,
        selected: false,
        accessible_label: accessible_label.into(),
    }
}

fn tripeaks_ui_card(card: Card, exposed: bool, position: usize) -> UiCard {
    if exposed {
        return ui_card_labeled(
            card,
            format!("{}, tableau position {position}, exposed", card_name(card)),
        );
    }
    UiCard {
        label: SharedString::default(),
        red: false,
        face_up: false,
        selected: false,
        accessible_label: format!("Tableau position {position}, covered, face-down").into(),
    }
}

fn pyramid_ui_card(card: Card, exposed: bool, selected: bool, position: usize) -> UiCard {
    if exposed {
        return UiCard {
            label: card_label(card).into(),
            red: matches!(card.suit, Suit::Diamonds | Suit::Hearts),
            face_up: true,
            selected,
            accessible_label: format!(
                "{}, Pyramid tableau position {position}, exposed{}",
                card_name(card),
                if selected { ", selected" } else { "" }
            )
            .into(),
        };
    }
    UiCard {
        label: SharedString::default(),
        red: false,
        face_up: false,
        selected: false,
        accessible_label: format!("Pyramid tableau position {position}, covered, face-down").into(),
    }
}

fn tripeaks_deal_number(seed: u64) -> SharedString {
    seed.to_string().into()
}

fn pyramid_deal_number(seed: u64) -> SharedString {
    seed.to_string().into()
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

fn describe_tripeaks_action(action: tripeaks::Action) -> String {
    match action {
        tripeaks::Action::Draw => "drawing the next stock card".into(),
        tripeaks::Action::Remove(index) => {
            format!("removing exposed tableau card {}", index + 1)
        }
    }
}

fn describe_pyramid_action(action: pyramid::Action) -> String {
    match action {
        pyramid::Action::Draw => "drawing the next Pyramid stock card".into(),
        pyramid::Action::Recycle => "recycling the Pyramid waste".into(),
        pyramid::Action::RemoveKing(source) => {
            format!("removing the king at {}", pyramid_source_name(source))
        }
        pyramid::Action::RemovePair(first, second) => format!(
            "removing the pair at {} and {}",
            pyramid_source_name(first),
            pyramid_source_name(second)
        ),
    }
}

fn pyramid_source_name(source: pyramid::Source) -> String {
    match source {
        pyramid::Source::Pyramid(index) => format!("tableau position {}", index + 1),
        pyramid::Source::Waste => "the waste".into(),
    }
}

fn pyramid_source_card(state: &pyramid::State, source: pyramid::Source) -> Option<Card> {
    match source {
        pyramid::Source::Pyramid(index) => {
            let index = usize::from(index);
            state
                .is_exposed(index)
                .then(|| state.pyramid.get(index).copied().flatten())
                .flatten()
        }
        pyramid::Source::Waste => state.waste.last().copied(),
    }
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

fn friendly_tripeaks_error(error: tripeaks::MoveError) -> String {
    match error {
        tripeaks::MoveError::EmptyStock => "The TriPeaks stock is empty".into(),
        tripeaks::MoveError::CoveredCard => "That tableau card is still covered".into(),
        tripeaks::MoveError::NotAdjacent => {
            "Choose an exposed card one rank above or below the waste".into()
        }
        tripeaks::MoveError::ResourceLimit => {
            "This deal reached the 4096-action replay limit; start a new deal to continue".into()
        }
        tripeaks::MoveError::CounterOverflow => {
            "This deal reached a numeric limit; the move was not applied".into()
        }
        tripeaks::MoveError::GameComplete => {
            "TriPeaks is complete; undo or start a new deal".into()
        }
        _ => "That TriPeaks move is not available".into(),
    }
}

fn friendly_pyramid_error(error: pyramid::MoveError) -> String {
    match error {
        pyramid::MoveError::EmptyStock => "The Pyramid stock is empty".into(),
        pyramid::MoveError::CannotRecycle => {
            "Draw the remaining Pyramid stock before recycling".into()
        }
        pyramid::MoveError::RedealLimit => "No Pyramid redeals remain".into(),
        pyramid::MoveError::CoveredCard => "That Pyramid card is still covered".into(),
        pyramid::MoveError::NotKing => "Only a king can be removed by itself".into(),
        pyramid::MoveError::SameCard => "Choose two different cards".into(),
        pyramid::MoveError::NotThirteen => "Choose two exposed cards whose ranks total 13".into(),
        pyramid::MoveError::ResourceLimit => {
            "This deal reached the 4096-action replay limit; start a new deal to continue".into()
        }
        pyramid::MoveError::CounterOverflow => {
            "This deal reached a numeric limit; the move was not applied".into()
        }
        pyramid::MoveError::GameComplete => "Pyramid is complete; undo or start a new deal".into(),
        _ => "That Pyramid move is not available".into(),
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

fn tripeaks_x(index: i32, available_width: f32) -> f32 {
    let step = ((available_width - 104.0) / 9.0).max(0.0);
    let slot = match index {
        0 => 1.5,
        1 => 4.5,
        2 => 7.5,
        3 => 1.0,
        4 => 2.0,
        5 => 4.0,
        6 => 5.0,
        7 => 7.0,
        8 => 8.0,
        9..=17 => f32::from(i16::try_from(index - 9).unwrap_or_default()) + 0.5,
        18..=27 => f32::from(i16::try_from(index - 18).unwrap_or_default()),
        _ => 0.0,
    };
    slot * step
}

fn pyramid_x(index: i32, available_width: f32) -> f32 {
    let Some((row, column)) = pyramid_position(index) else {
        return 0.0;
    };
    let step = ((available_width - 104.0) / 6.0).max(0.0);
    let slot = f32::from(column) + f32::from(6 - row) / 2.0;
    slot * step
}

fn pyramid_y(index: i32) -> f32 {
    pyramid_position(index).map_or(0.0, |(row, _)| f32::from(row) * 38.0)
}

fn pyramid_position(index: i32) -> Option<(i16, i16)> {
    let index = i16::try_from(index)
        .ok()
        .filter(|index| (0..28).contains(index))?;
    let mut row = 0_i16;
    while (row + 1) * (row + 2) / 2 <= index {
        row += 1;
    }
    Some((row, index - row * (row + 1) / 2))
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
            tripeaks: TriPeaksGame::new(seed, tripeaks::Options::default()),
            tripeaks_save_path: None,
            pyramid: PyramidGame::new(seed, pyramid::Options::default()),
            pyramid_selection: None,
            pyramid_save_path: None,
            save_revisions: [None; 5],
            dirty: [false; 5],
            pending_new_deal: None,
            pending_new_deal_conflict: false,
            deal_counters_path: None,
            next_seeds: DealCounters {
                klondike: seed.saturating_add(1),
                spider: seed.saturating_add(1),
                freecell: seed.saturating_add(1),
                tripeaks: seed.saturating_add(1),
                pyramid: seed.saturating_add(1),
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
    fn tripeaks_surface_routes_standard_play_history_and_reopen() {
        let path = test_save("tripeaks-surface");
        remove_save(&path);
        let mut controller = controller(81);
        controller.select_game("TriPeaks");
        controller.tripeaks_save_path = Some(path.clone());
        controller.new_game("Standard");
        assert!(!controller.tripeaks.state.options.wraparound);

        match controller.tripeaks.hint().unwrap() {
            tripeaks::Action::Draw => controller.draw_tripeaks_stock(),
            tripeaks::Action::Remove(index) => controller.activate_tripeaks_card(index.into()),
        }
        assert!(controller.tripeaks.can_undo());
        let moved = controller.tripeaks.state.clone();
        controller.undo();
        assert!(controller.tripeaks.can_redo());
        controller.redo();
        assert_eq!(controller.tripeaks.state, moved);
        assert_eq!(load_tripeaks_revisioned(&path).unwrap().0.state, moved);
        remove_save(&path);
    }

    #[test]
    fn tripeaks_save_conflict_preserves_memory_until_reload() {
        let path = test_save("tripeaks-conflict");
        remove_save(&path);
        let mut controller = controller(82);
        controller.select_game("TriPeaks");
        controller.tripeaks_save_path = Some(path.clone());
        assert!(controller.save());

        let disk_game = TriPeaksGame::new(999, tripeaks::Options::default());
        solitaire::persistence::save_tripeaks(&path, &disk_game).unwrap();
        controller.draw_tripeaks_stock();
        assert!(controller.dirty[3]);
        assert!(controller.status.contains("save changed in another"));

        controller.reload_disk_copy();
        assert_eq!(controller.tripeaks, disk_game);
        assert!(!controller.dirty[3]);
        remove_save(&path);
    }

    #[test]
    fn tripeaks_counter_overflow_preserves_current_deal() {
        let mut controller = controller(83);
        controller.select_game("TriPeaks");
        controller.next_seeds.tripeaks = u64::MAX;
        let current = controller.tripeaks.clone();
        controller.new_game("Standard");
        assert_eq!(controller.tripeaks, current);
        assert!(controller.pending_new_deal.is_some());
        assert!(controller.status.contains("No further deal number"));
    }

    #[test]
    fn tripeaks_layout_positions_all_four_rows_inside_the_surface() {
        let width = 1084.0;
        for index in 0..28 {
            let x = tripeaks_x(index, width);
            assert!((0.0..=width - 104.0).contains(&x));
        }
        assert!(tripeaks_x(0, width) < tripeaks_x(1, width));
        assert!(tripeaks_x(1, width) < tripeaks_x(2, width));
        assert!(tripeaks_x(18, width).abs() < f32::EPSILON);
        assert!((tripeaks_x(27, width) - (width - 104.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn tripeaks_card_model_hides_covered_identity_and_names_exposed_cards() {
        let card = Card::new(Suit::Hearts, Rank::Queen);
        let covered = tripeaks_ui_card(card, false, 4);
        assert!(!covered.face_up);
        assert!(!covered.red);
        assert!(covered.label.is_empty());
        assert_eq!(
            covered.accessible_label.as_str(),
            "Tableau position 4, covered, face-down"
        );
        assert!(!covered.accessible_label.contains("Queen"));
        assert!(!covered.accessible_label.contains("Hearts"));

        let exposed = tripeaks_ui_card(card, true, 19);
        assert!(exposed.face_up);
        assert_eq!(exposed.label.as_str(), "Q♥");
        assert!(exposed.accessible_label.contains("Queen of hearts"));
        assert!(exposed.accessible_label.contains("exposed"));
    }

    #[test]
    fn tripeaks_deal_number_preserves_full_u64_seed() {
        let seed = u64::from(u32::MAX) + 17;
        assert_eq!(tripeaks_deal_number(seed).as_str(), seed.to_string());
    }

    #[test]
    fn tripeaks_completion_error_is_actionable_and_preserves_state() {
        let mut controller = controller(84);
        controller.select_game("TriPeaks");
        controller.tripeaks.state.tableau = [None; 28];
        let complete = controller.tripeaks.clone();
        controller.draw_tripeaks_stock();
        assert_eq!(controller.tripeaks, complete);
        assert_eq!(
            controller.status,
            "TriPeaks is complete; undo or start a new deal"
        );
    }

    #[test]
    fn pyramid_surface_routes_standard_play_history_and_reopen() {
        let path = test_save("pyramid-surface");
        remove_save(&path);
        let mut controller = controller(85);
        controller.select_game("Pyramid");
        controller.pyramid_save_path = Some(path.clone());
        controller.new_game("Standard");
        assert_eq!(controller.pyramid.state.options.max_redeals, 2);

        match controller.pyramid.hint().unwrap() {
            pyramid::Action::Draw | pyramid::Action::Recycle => controller.draw_pyramid_stock(),
            pyramid::Action::RemoveKing(source) => controller.activate_pyramid_source(source),
            pyramid::Action::RemovePair(first, second) => {
                controller.activate_pyramid_source(first);
                controller.activate_pyramid_source(second);
            }
        }
        assert!(controller.pyramid.can_undo());
        let moved = controller.pyramid.state.clone();
        controller.undo();
        assert!(controller.pyramid.can_redo());
        controller.redo();
        assert_eq!(controller.pyramid.state, moved);
        assert_eq!(load_pyramid_revisioned(&path).unwrap().0.state, moved);
        remove_save(&path);
    }

    #[test]
    fn pyramid_save_conflict_preserves_memory_until_reload() {
        let path = test_save("pyramid-conflict");
        remove_save(&path);
        let mut controller = controller(86);
        controller.select_game("Pyramid");
        controller.pyramid_save_path = Some(path.clone());
        assert!(controller.save());

        let disk_game = PyramidGame::new(999, pyramid::Options::default());
        solitaire::persistence::save_pyramid(&path, &disk_game).unwrap();
        controller.draw_pyramid_stock();
        assert!(controller.dirty[4]);
        assert!(controller.status.contains("save changed in another"));

        controller.reload_disk_copy();
        assert_eq!(controller.pyramid, disk_game);
        assert!(!controller.dirty[4]);
        remove_save(&path);
    }

    #[test]
    fn pyramid_counter_overflow_preserves_current_deal() {
        let mut controller = controller(87);
        controller.select_game("Pyramid");
        controller.next_seeds.pyramid = u64::MAX;
        let current = controller.pyramid.clone();
        controller.new_game("Standard");
        assert_eq!(controller.pyramid, current);
        assert!(controller.pending_new_deal.is_some());
        assert!(controller.status.contains("No further deal number"));
    }

    #[test]
    fn pyramid_layout_positions_all_seven_rows_inside_the_surface() {
        let width = 1084.0;
        for index in 0..28 {
            let x = pyramid_x(index, width);
            let y = pyramid_y(index);
            assert!((0.0..=width - 104.0).contains(&x));
            assert!((0.0..=228.0).contains(&y));
        }
        assert!((pyramid_x(0, width) - (width - 104.0) / 2.0).abs() < f32::EPSILON);
        assert!(pyramid_x(21, width).abs() < f32::EPSILON);
        assert!((pyramid_x(27, width) - (width - 104.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn pyramid_card_model_hides_covered_identity_and_names_selection() {
        let card = Card::new(Suit::Diamonds, Rank::Five);
        let covered = pyramid_ui_card(card, false, false, 3);
        assert!(!covered.face_up);
        assert!(!covered.red);
        assert!(covered.label.is_empty());
        assert_eq!(
            covered.accessible_label.as_str(),
            "Pyramid tableau position 3, covered, face-down"
        );
        assert!(!covered.accessible_label.contains("Five"));
        assert!(!covered.accessible_label.contains("diamonds"));

        let exposed = pyramid_ui_card(card, true, true, 22);
        assert!(exposed.face_up);
        assert!(exposed.red);
        assert!(exposed.selected);
        assert_eq!(exposed.label.as_str(), "5♦");
        assert!(exposed.accessible_label.contains("Five of diamonds"));
        assert!(exposed.accessible_label.contains("selected"));
    }

    #[test]
    fn pyramid_deal_number_preserves_full_u64_seed() {
        let seed = u64::from(u32::MAX) + 29;
        assert_eq!(pyramid_deal_number(seed).as_str(), seed.to_string());
    }

    #[test]
    fn pyramid_completion_error_is_actionable_and_preserves_state() {
        let mut controller = controller(88);
        controller.select_game("Pyramid");
        controller.pyramid.state.pyramid = [None; 28];
        let complete = controller.pyramid.clone();
        controller.draw_pyramid_stock();
        assert_eq!(controller.pyramid, complete);
        assert_eq!(
            controller.status,
            "Pyramid is complete; undo or start a new deal"
        );
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
    fn deleted_save_conflict_requires_confirmation_before_retry() {
        let path = test_save("deleted-pending-conflict");
        remove_save(&path);
        let mut controller = controller(360);
        controller.save_path = Some(path.clone());
        assert!(controller.save());
        fs::remove_file(&path).unwrap();

        controller.new_game("Draw 1");
        assert!(controller.pending_new_deal_conflict);
        assert!(controller.save_revisions[0].is_some());

        controller.confirm_missing_save_ownership();
        assert!(!controller.pending_new_deal_conflict);
        assert_eq!(controller.save_revisions[0], None);
        assert!(controller.status.contains("Confirmed under lock"));

        controller.retry_save();
        assert!(controller.pending_new_deal.is_none());
        assert!(!controller.dirty[0]);
        let (saved, _) = load_klondike_revisioned(&path).unwrap();
        assert_eq!(saved, controller.game);
        remove_save(&path);
    }

    #[test]
    fn quarantined_save_conflict_fails_closed_until_absence_is_confirmed() {
        let path = test_save("quarantined-pending-conflict");
        remove_save(&path);
        let mut controller = controller(370);
        controller.save_path = Some(path.clone());
        assert!(controller.save());
        fs::write(&path, b"malformed competing save").unwrap();

        controller.new_game("Draw 3");
        assert!(controller.pending_new_deal_conflict);
        let stale_revision = controller.save_revisions[0];
        controller.confirm_missing_save_ownership();
        assert!(controller.pending_new_deal_conflict);
        assert_eq!(controller.save_revisions[0], stale_revision);
        assert!(controller.status.contains("still exists"));

        let quarantine = solitaire::persistence::quarantine_save(&path).unwrap();
        let quarantined = match quarantine {
            solitaire::persistence::NamespaceMutation::Durable(path)
            | solitaire::persistence::NamespaceMutation::CommittedButNotDurable {
                value: path,
                ..
            } => path,
        };
        controller.confirm_missing_save_ownership();
        assert!(!controller.pending_new_deal_conflict);
        assert_eq!(controller.save_revisions[0], None);

        controller.retry_save();
        assert!(controller.pending_new_deal.is_none());
        assert!(!controller.dirty[0]);
        let (saved, _) = load_klondike_revisioned(&path).unwrap();
        assert_eq!(saved, controller.game);
        fs::remove_file(quarantined).unwrap();
        remove_save(&path);
    }

    #[test]
    fn discard_and_close_explicitly_releases_the_close_guard() {
        let mut controller = controller(404);
        controller.dirty = [true; 5];
        controller.new_game("Draw 1");
        assert!(controller.pending_new_deal.is_some());

        controller.discard_unsaved_and_close();

        assert_eq!(controller.dirty, [false; 5]);
        assert!(controller.pending_new_deal.is_none());
        assert!(controller.status.contains("closing"));
    }

    fn to_i32(value: usize) -> i32 {
        i32::try_from(value).unwrap_or(i32::MAX)
    }
}
