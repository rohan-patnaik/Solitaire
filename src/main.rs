use slint::{ModelRc, SharedString, Timer, TimerMode, VecModel};
use solitaire::cards::{Card, Rank, Suit};
use solitaire::freecell::{self, Game as FreeCellGame};
use solitaire::klondike::{Action, DrawMode, Game, Options, Pile, Scoring};
use solitaire::persistence::{
    DealCounters, DealKind, RecoveredSave, SaveError, SaveRevision, confirm_current_save_revision,
    default_deal_counters_path, default_freecell_save_path, default_local_profile_path,
    default_pyramid_save_path, default_save_path, default_spider_save_path,
    default_tripeaks_save_path, ensure_deal_counters, load_deal_counters, load_freecell_revisioned,
    load_klondike_revisioned, load_local_profile_revisioned, load_pyramid_revisioned,
    load_spider_revisioned, load_tripeaks_revisioned, recover_freecell_revisioned,
    recover_klondike_revisioned, recover_local_profile_revisioned, recover_pyramid_revisioned,
    recover_spider_revisioned, recover_tripeaks_revisioned, reserve_deal, save_freecell_checked,
    save_klondike_checked, save_local_profile_checked, save_pyramid_checked, save_spider_checked,
    save_tripeaks_checked,
};
use solitaire::profile::{GameKind as ProfileGameKind, LocalProfile};
use solitaire::pyramid::{self, Game as PyramidGame};
use solitaire::spider::{self, Game as SpiderGame, SuitMode};
use solitaire::tripeaks::{self, Game as TriPeaksGame};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

slint::include_modules!();

const KLONDIKE_TIMER_CHECKPOINT_SECONDS: u64 = 15;
const POINTER_DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone, PartialEq, Eq)]
enum KlondikePointerSource {
    Waste,
    Tableau { column: i32, card_index: i32 },
}

#[derive(Clone, PartialEq, Eq)]
struct KlondikePointerIdentity {
    source: KlondikePointerSource,
    card: String,
    deal_instance: String,
    interaction_generation: String,
}

#[derive(Default)]
struct PointerClickState {
    pending: Option<KlondikePointerIdentity>,
    double_armed: bool,
}

#[derive(Default)]
struct PointerClickTimer {
    timer: Timer,
    state: RefCell<PointerClickState>,
}

impl PointerClickTimer {
    fn pointer_pressed(&self, identity: &KlondikePointerIdentity) {
        self.timer.stop();
        let mut state = self.state.borrow_mut();
        state.double_armed = state.pending.as_ref() == Some(identity);
        if !state.double_armed {
            state.pending = None;
        }
    }

    fn pointer_clicked(
        self: &Rc<Self>,
        identity: KlondikePointerIdentity,
        callback: impl FnMut() + 'static,
    ) {
        self.pointer_clicked_after(identity, POINTER_DOUBLE_CLICK_INTERVAL, callback);
    }

    fn pointer_clicked_after(
        self: &Rc<Self>,
        identity: KlondikePointerIdentity,
        interval: Duration,
        mut callback: impl FnMut() + 'static,
    ) {
        {
            let mut state = self.state.borrow_mut();
            if !state.double_armed {
                state.pending = Some(identity.clone());
            }
        }
        let weak = Rc::downgrade(self);
        self.timer.start(TimerMode::SingleShot, interval, move || {
            let Some(timer) = weak.upgrade() else {
                return;
            };
            let mut state = timer.state.borrow_mut();
            if state.pending.as_ref() != Some(&identity) {
                return;
            }
            state.pending = None;
            state.double_armed = false;
            drop(state);
            callback();
        });
    }

    fn take_double(&self, identity: &KlondikePointerIdentity) -> bool {
        self.timer.stop();
        let mut state = self.state.borrow_mut();
        let matched = state.double_armed && state.pending.as_ref() == Some(identity);
        state.pending = None;
        state.double_armed = false;
        matched
    }
}

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum NewDealVariant {
    Klondike {
        draw_mode: DrawMode,
        scoring: Scoring,
        max_redeals: Option<u8>,
        timed: bool,
    },
    Spider(SuitMode),
    FreeCell(FreeCellDeal),
    TriPeaks {
        wraparound: bool,
    },
    Pyramid {
        max_redeals: u8,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FreeCellDeal {
    Next,
    Exact(u64),
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct PendingNewDeal {
    game: GameKind,
    variant: NewDealVariant,
    restart_seed: Option<u64>,
}

impl PendingNewDeal {
    const fn is_restart(self) -> bool {
        self.restart_seed.is_some()
    }
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
    klondike_deal_instance: u64,
    interaction_generation: u64,
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
    local_profile: LocalProfile,
    local_profile_path: Option<PathBuf>,
    local_profile_revision: Option<SaveRevision>,
    local_profile_dirty: bool,
    klondike_elapsed_dirty: bool,
    klondike_uncheckpointed_seconds: u64,
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
        let next_seeds =
            load_initial_deal_counters(deal_counters_path.as_deref(), defaults, &mut status);
        let (local_profile, local_profile_path, local_profile_revision) =
            load_initial_local_profile(&mut status);
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
            klondike_deal_instance: 0,
            interaction_generation: 0,
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
            local_profile,
            local_profile_path,
            local_profile_revision,
            local_profile_dirty: false,
            klondike_elapsed_dirty: false,
            klondike_uncheckpointed_seconds: 0,
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

    fn activate_tableau_pointer(
        &mut self,
        column: i32,
        card_index: i32,
        card: &str,
        deal_instance: &str,
        interaction_generation: &str,
    ) {
        let Some((_, current)) = self.klondike_tableau_top(
            column,
            card_index,
            card,
            deal_instance,
            interaction_generation,
        ) else {
            return;
        };
        debug_assert_eq!(card_label(current), card);
        self.activate_tableau(column, card_index);
    }

    fn double_activate_tableau(
        &mut self,
        column: i32,
        card_index: i32,
        card: &str,
        deal_instance: &str,
        interaction_generation: &str,
    ) {
        let Some((column, current)) = self.klondike_tableau_top(
            column,
            card_index,
            card,
            deal_instance,
            interaction_generation,
        ) else {
            return;
        };
        self.apply(Action::Move {
            from: Pile::Tableau(column),
            to: Pile::Foundation(current.suit),
            count: 1,
        });
    }

    fn klondike_tableau_top(
        &mut self,
        column: i32,
        card_index: i32,
        expected_card: &str,
        expected_deal_instance: &str,
        expected_interaction_generation: &str,
    ) -> Option<(u8, Card)> {
        if !self.klondike_pointer_context_matches(
            expected_deal_instance,
            expected_interaction_generation,
        ) {
            return None;
        }
        let Ok(column) = u8::try_from(column) else {
            self.status = "That Klondike card is no longer available; click again".into();
            return None;
        };
        let Ok(card_index) = usize::try_from(card_index) else {
            self.status = "That Klondike card is no longer available; click again".into();
            return None;
        };
        let Some(pile) = self.game.state.tableau.get(usize::from(column)) else {
            self.status = "That Klondike card is no longer available; click again".into();
            return None;
        };
        let Some(card) = pile.get(card_index) else {
            self.status = "That Klondike card is no longer available; click again".into();
            return None;
        };
        if !card.face_up
            || card_index.checked_add(1) != Some(pile.len())
            || card_label(card.card) != expected_card
        {
            self.status = "That Klondike card is no longer available; click again".into();
            return None;
        }
        Some((column, card.card))
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

    fn activate_waste_pointer(
        &mut self,
        card: &str,
        deal_instance: &str,
        interaction_generation: &str,
    ) {
        if self
            .klondike_waste_top(card, deal_instance, interaction_generation)
            .is_some()
        {
            self.activate_waste();
        }
    }

    fn double_activate_waste(
        &mut self,
        card: &str,
        deal_instance: &str,
        interaction_generation: &str,
    ) {
        let Some(current) = self.klondike_waste_top(card, deal_instance, interaction_generation)
        else {
            return;
        };
        self.apply(Action::Move {
            from: Pile::Waste,
            to: Pile::Foundation(current.suit),
            count: 1,
        });
    }

    fn klondike_waste_top(
        &mut self,
        expected_card: &str,
        expected_deal_instance: &str,
        expected_interaction_generation: &str,
    ) -> Option<Card> {
        if !self.klondike_pointer_context_matches(
            expected_deal_instance,
            expected_interaction_generation,
        ) {
            return None;
        }
        let Some(card) = self.game.state.waste.last().copied() else {
            self.status = "That Klondike card is no longer available; click again".into();
            return None;
        };
        if card_label(card) != expected_card {
            self.status = "That Klondike card is no longer available; click again".into();
            return None;
        }
        Some(card)
    }

    fn klondike_pointer_context_matches(
        &mut self,
        expected_deal_instance: &str,
        expected_interaction_generation: &str,
    ) -> bool {
        if self.active == GameKind::Klondike
            && self.klondike_deal_instance.to_string() == expected_deal_instance
            && self.interaction_generation.to_string() == expected_interaction_generation
        {
            return true;
        }
        self.reject_klondike_pointer();
        false
    }

    fn reject_klondike_pointer(&mut self) {
        self.status = "That Klondike card is no longer available; click again".into();
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
        if self.active == GameKind::Klondike && !self.checkpoint_klondike_elapsed() {
            return;
        }
        let next = match game {
            "Spider" => GameKind::Spider,
            "FreeCell" => GameKind::FreeCell,
            "TriPeaks" => GameKind::TriPeaks,
            "Pyramid" => GameKind::Pyramid,
            _ => GameKind::Klondike,
        };
        if self.active != next && (self.active == GameKind::Klondike || next == GameKind::Klondike)
        {
            self.klondike_deal_instance = self.klondike_deal_instance.wrapping_add(1);
        }
        self.active = next;
        self.selection = None;
        self.spider_selection = None;
        self.freecell_selection = None;
        self.pyramid_selection = None;
        self.pending_new_deal = None;
        self.pending_new_deal_conflict = false;
        self.status = format!("{} ready", self.game_name());
    }

    fn new_game(&mut self, variant: &str) {
        let Some(variant) = parse_new_deal_variant(self.active, variant) else {
            self.status = "Invalid new-deal options; current game preserved".into();
            return;
        };
        self.stage_new_deal(PendingNewDeal {
            game: self.active,
            variant,
            restart_seed: None,
        });
    }

    fn new_freecell_game(&mut self, deal_number: &str) {
        if self.active != GameKind::FreeCell {
            self.status = "Numbered deal entry is available only in FreeCell".into();
            return;
        }
        let Some(deal_number) = parse_freecell_deal_number(deal_number) else {
            self.status = "Enter a decimal FreeCell deal number from 0 through 18446744073709551615; current game preserved".into();
            return;
        };
        self.stage_new_deal(PendingNewDeal {
            game: GameKind::FreeCell,
            variant: NewDealVariant::FreeCell(FreeCellDeal::Exact(deal_number)),
            restart_seed: None,
        });
    }

    fn restart_current_deal(&mut self) {
        let request = match self.active {
            GameKind::Klondike => PendingNewDeal {
                game: GameKind::Klondike,
                variant: NewDealVariant::Klondike {
                    draw_mode: self.game.state.options.draw_mode,
                    scoring: self.game.state.options.scoring,
                    max_redeals: self.game.state.options.max_redeals,
                    timed: self.game.state.options.timed,
                },
                restart_seed: Some(self.game.state.seed),
            },
            GameKind::Spider => PendingNewDeal {
                game: GameKind::Spider,
                variant: NewDealVariant::Spider(self.spider.state.mode),
                restart_seed: Some(self.spider.state.seed),
            },
            GameKind::FreeCell => PendingNewDeal {
                game: GameKind::FreeCell,
                variant: NewDealVariant::FreeCell(FreeCellDeal::Exact(
                    self.freecell.state.deal_number,
                )),
                restart_seed: Some(self.freecell.state.deal_number),
            },
            GameKind::TriPeaks => PendingNewDeal {
                game: GameKind::TriPeaks,
                variant: NewDealVariant::TriPeaks {
                    wraparound: self.tripeaks.state.options.wraparound,
                },
                restart_seed: Some(self.tripeaks.state.seed),
            },
            GameKind::Pyramid => PendingNewDeal {
                game: GameKind::Pyramid,
                variant: NewDealVariant::Pyramid {
                    max_redeals: self.pyramid.state.options.max_redeals,
                },
                restart_seed: Some(self.pyramid.state.seed),
            },
        };
        self.stage_new_deal(request);
    }

    fn stage_new_deal(&mut self, request: PendingNewDeal) {
        self.pending_new_deal = Some(request);
        self.pending_new_deal_conflict = false;
        if self.dirty[self.active_index()] || self.local_profile_dirty {
            self.status = if request.is_restart() {
                "This deal or its local statistics have unsaved progress. Retry save before restarting this deal, or cancel.".into()
            } else {
                "This deal or its local statistics have unsaved progress. Retry save before starting a new deal, or cancel.".into()
            };
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
        if !new_deal_variant_matches(request.game, request.variant) {
            self.pending_new_deal = Some(request);
            self.status = "Invalid pending new-deal options; current game preserved".into();
            return;
        }
        let Some(seed) = self.seed_for_new_deal(request) else {
            if !self.status.starts_with("Could not ") {
                self.status =
                    "No further deal number is representable; existing game preserved".into();
            }
            self.pending_new_deal = Some(request);
            return;
        };
        let restarting = request.is_restart();
        let candidate = prospective_game(request, seed);
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
                self.status = if restarting {
                    format!("Restarted {} deal", self.game_name())
                } else {
                    format!("New {} deal", self.game_name())
                };
            }
            Some(Err(error)) if error.committed_but_not_durable() => {
                self.replace_game(candidate);
                self.dirty[index] = true;
                self.pending_new_deal_conflict = false;
                self.clear_selections();
                self.status = if restarting {
                    format!(
                        "The restarted deal replaced the on-disk entry and is now current in memory, but durability is indeterminate: {error}"
                    )
                } else {
                    format!(
                        "The new deal replaced the on-disk entry and is now current in memory, but durability is indeterminate: {error}"
                    )
                };
            }
            Some(Err(error)) => {
                self.pending_new_deal_conflict = error.is_conflict();
                self.pending_new_deal = Some(request);
                self.status = if restarting {
                    format!(
                        "Deal was not restarted; the current game remains in memory because saving the prospective restart failed: {error}. Retry, discard, or cancel."
                    )
                } else {
                    format!(
                        "New deal was not started; the current game remains in memory because saving the prospective deal failed: {error}. Retry, discard, or cancel."
                    )
                };
            }
            None => {
                self.pending_new_deal_conflict = false;
                self.pending_new_deal = Some(request);
                self.status = if restarting {
                    "Deal was not restarted; the current game remains in memory because no writable save location is available. Retry, discard, or cancel.".into()
                } else {
                    "New deal was not started; the current game remains in memory because no writable save location is available. Retry, discard, or cancel.".into()
                };
            }
        }
    }

    fn replace_game(&mut self, candidate: ProspectiveGame) {
        match candidate {
            ProspectiveGame::Klondike(game) => {
                self.game = game;
                self.klondike_deal_instance = self.klondike_deal_instance.wrapping_add(1);
                self.klondike_elapsed_dirty = false;
                self.klondike_uncheckpointed_seconds = 0;
            }
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

    fn seed_for_new_deal(&mut self, request: PendingNewDeal) -> Option<u64> {
        if let Some(seed) = request.restart_seed {
            return Some(seed);
        }
        match request.variant {
            NewDealVariant::FreeCell(FreeCellDeal::Exact(deal_number)) => {
                if let Some(path) = self.deal_counters_path.as_deref() {
                    match ensure_deal_counters(path, self.next_seeds) {
                        Ok(counters) => self.next_seeds = counters,
                        Err(error) => {
                            self.status = format!(
                                "Could not preserve the independent next-deal sequence: {error}"
                            );
                            return None;
                        }
                    }
                }
                Some(deal_number)
            }
            _ => self.take_next_seed(request.game),
        }
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
                if self.active == GameKind::Klondike {
                    self.klondike_elapsed_dirty = false;
                    self.klondike_uncheckpointed_seconds = 0;
                }
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

    fn klondike_timer_running(&self) -> bool {
        self.active == GameKind::Klondike
            && self.pending_new_deal.is_none()
            && self.game.state.options.timed
            && !self.game.state.is_won()
    }

    fn advance_klondike_timer(&mut self, seconds: u64) -> bool {
        if seconds == 0 || !self.klondike_timer_running() {
            return false;
        }
        let before = self.game.state.elapsed_seconds;
        self.game.state.advance_time(seconds);
        let advanced = self.game.state.elapsed_seconds.saturating_sub(before);
        if advanced == 0 {
            return false;
        }
        self.klondike_elapsed_dirty = true;
        self.klondike_uncheckpointed_seconds = self
            .klondike_uncheckpointed_seconds
            .saturating_add(advanced);
        if self.klondike_uncheckpointed_seconds >= KLONDIKE_TIMER_CHECKPOINT_SECONDS {
            let _ = self.checkpoint_klondike_elapsed();
        }
        true
    }

    fn checkpoint_klondike_elapsed(&mut self) -> bool {
        if !self.klondike_elapsed_dirty {
            return true;
        }
        self.klondike_uncheckpointed_seconds = 0;
        let result = self
            .save_path
            .as_deref()
            .map(|path| save_klondike_checked(path, &self.game, &mut self.save_revisions[0]));
        match result {
            Some(Ok(())) => {
                self.dirty[0] = false;
                self.klondike_elapsed_dirty = false;
                true
            }
            Some(Err(error)) if error.committed_but_not_durable() => {
                self.dirty[0] = true;
                self.status = format!(
                    "The timed Klondike checkpoint reached disk, but durability is indeterminate: {error}"
                );
                false
            }
            Some(Err(error)) => {
                self.dirty[0] = true;
                self.status = format!(
                    "Timed Klondike progress remains in memory; checkpoint failed: {error}. Retry before closing."
                );
                false
            }
            None => {
                self.dirty[0] = true;
                self.status = "Timed Klondike progress remains in memory; no writable save location. Retry before closing.".into();
                false
            }
        }
    }

    fn persist_mutation(&mut self) {
        let index = self.active_index();
        self.dirty[index] = true;
        let _ = self.save();
        self.observe_active_profile();
    }

    fn observe_active_profile(&mut self) {
        if self.local_profile_path.is_none() {
            return;
        }
        let mut candidate = self.local_profile.clone();
        let observed = candidate.observe(
            self.profile_game_kind(),
            self.active_deal_number(),
            self.active_game_is_won(),
        );
        match observed {
            Ok(false) => {}
            Ok(true) => {
                self.local_profile = candidate;
                self.local_profile_dirty = true;
                let _ = self.save_local_profile();
            }
            Err(error) => {
                self.status = format!(
                    "Local statistics were not changed because the observation was rejected: {error}"
                );
            }
        }
    }

    fn save_local_profile(&mut self) -> bool {
        let result = self.local_profile_path.as_deref().map(|path| {
            save_local_profile_checked(path, &self.local_profile, &mut self.local_profile_revision)
        });
        match result {
            Some(Ok(())) => {
                self.local_profile_dirty = false;
                true
            }
            Some(Err(error)) if error.committed_but_not_durable() => {
                self.local_profile_dirty = true;
                self.status = format!(
                    "Local statistics reached the on-disk profile, but durability is indeterminate: {error}"
                );
                false
            }
            Some(Err(error)) => {
                self.local_profile_dirty = true;
                self.status = format!(
                    "Local statistics remain in memory; profile save failed: {error}. Retry before closing."
                );
                false
            }
            None => {
                self.local_profile_dirty = true;
                self.status = "Local statistics remain in memory; no writable profile location is available. Retry before closing.".into();
                false
            }
        }
    }

    const fn profile_game_kind(&self) -> ProfileGameKind {
        match self.active {
            GameKind::Klondike => ProfileGameKind::Klondike,
            GameKind::Spider => ProfileGameKind::Spider,
            GameKind::FreeCell => ProfileGameKind::FreeCell,
            GameKind::TriPeaks => ProfileGameKind::TriPeaks,
            GameKind::Pyramid => ProfileGameKind::Pyramid,
        }
    }

    fn active_deal_number(&self) -> u64 {
        match self.active {
            GameKind::Klondike => self.game.state.seed,
            GameKind::Spider => self.spider.state.seed,
            GameKind::FreeCell => self.freecell.state.deal_number,
            GameKind::TriPeaks => self.tripeaks.state.seed,
            GameKind::Pyramid => self.pyramid.state.seed,
        }
    }

    fn active_game_is_won(&self) -> bool {
        match self.active {
            GameKind::Klondike => self.game.state.is_won(),
            GameKind::Spider => self.spider.state.is_won(),
            GameKind::FreeCell => self.freecell.state.is_won(),
            GameKind::TriPeaks => self.tripeaks.state.is_won(),
            GameKind::Pyramid => self.pyramid.state.is_won(),
        }
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
            self.clear_selections();
            let count = self.game.autocomplete();
            self.status = if count == 1 {
                "Moved 1 safe card to a foundation".into()
            } else {
                format!("Moved {count} safe cards to foundations")
            };
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
        let game_was_dirty = self.dirty[self.active_index()];
        let profile_was_dirty = self.local_profile_dirty;
        if game_was_dirty && !self.save() {
            return;
        }
        if profile_was_dirty && !self.save_local_profile() {
            return;
        }
        if self.pending_new_deal.is_some() {
            self.commit_pending_new_deal();
        } else if !game_was_dirty && !profile_was_dirty {
            self.status = "No unsaved changes".into();
        } else {
            self.status = "Changes saved".into();
        }
    }

    fn discard_progress_and_start_pending(&mut self) {
        if self.local_profile_dirty {
            self.status = if self
                .pending_new_deal
                .is_some_and(PendingNewDeal::is_restart)
            {
                "Local statistics remain unsaved; retry their save before restarting this deal"
                    .into()
            } else {
                "Local statistics remain unsaved; retry their save before starting a new deal"
                    .into()
            };
            return;
        }
        self.commit_pending_new_deal();
    }

    fn cancel_pending_new_deal(&mut self) {
        if let Some(request) = self.pending_new_deal.take() {
            self.pending_new_deal_conflict = false;
            self.status = if request.is_restart() {
                "Restart cancelled; current game preserved".into()
            } else {
                "New deal cancelled; current game preserved".into()
            };
        } else {
            self.status = "No new deal is waiting for confirmation".into();
        }
    }

    fn confirm_missing_save_ownership(&mut self) {
        let index = self.active_index();
        if self.dirty[index] || self.pending_new_deal.is_none() || !self.pending_new_deal_conflict {
            self.status = "Missing-save ownership can only be refreshed for a clean pending deal-change conflict".into();
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
                self.status = "Confirmed under lock that the save is missing; ownership was refreshed. Retry or cancel the pending deal change.".into();
            }
            Ok(Some(_)) => {
                self.status = "The save path still exists; ownership was not changed. Reload the disk copy or cancel the pending deal change.".into();
            }
            Err(error) => {
                self.status = format!(
                    "Could not confirm that the save is missing; ownership was not changed: {error}"
                );
            }
        }
    }

    fn discard_unsaved_and_close(&mut self) {
        self.advance_interaction_generation();
        self.pending_new_deal = None;
        self.pending_new_deal_conflict = false;
        self.dirty = [false; 5];
        self.local_profile_dirty = false;
        self.klondike_elapsed_dirty = false;
        self.klondike_uncheckpointed_seconds = 0;
        self.status = "Unsaved progress and local statistics discarded; closing".into();
    }

    fn close_requested(&mut self) -> bool {
        self.advance_interaction_generation();
        let _ = self.checkpoint_klondike_elapsed();
        if self.dirty.iter().any(|dirty| *dirty) || self.local_profile_dirty {
            self.status =
                "Unsaved changes remain. Retry save before closing the application.".into();
            return false;
        }
        true
    }

    fn advance_interaction_generation(&mut self) {
        self.interaction_generation = self.interaction_generation.wrapping_add(1);
    }

    fn reload_disk_copy(&mut self) {
        let index = self.active_index();
        let reload_game = self.dirty[index] || self.pending_new_deal_conflict;
        let reload_profile = self.local_profile_dirty;
        let game_result = reload_game.then(|| match self.active {
            GameKind::Klondike => self
                .save_path
                .as_deref()
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "no Klondike save path")
                        .into()
                })
                .and_then(load_klondike_revisioned)
                .map(|(game, revision)| (ProspectiveGame::Klondike(game), revision)),
            GameKind::Spider => self
                .spider_save_path
                .as_deref()
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "no Spider save path").into()
                })
                .and_then(load_spider_revisioned)
                .map(|(game, revision)| (ProspectiveGame::Spider(game), revision)),
            GameKind::FreeCell => self
                .freecell_save_path
                .as_deref()
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "no FreeCell save path")
                        .into()
                })
                .and_then(load_freecell_revisioned)
                .map(|(game, revision)| (ProspectiveGame::FreeCell(game), revision)),
            GameKind::TriPeaks => self
                .tripeaks_save_path
                .as_deref()
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "no TriPeaks save path")
                        .into()
                })
                .and_then(load_tripeaks_revisioned)
                .map(|(game, revision)| (ProspectiveGame::TriPeaks(game), revision)),
            GameKind::Pyramid => self
                .pyramid_save_path
                .as_deref()
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "no Pyramid save path").into()
                })
                .and_then(load_pyramid_revisioned)
                .map(|(game, revision)| (ProspectiveGame::Pyramid(game), revision)),
        });
        let profile_result = reload_profile.then(|| {
            self.local_profile_path
                .as_deref()
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "no local profile path")
                        .into()
                })
                .and_then(load_local_profile_revisioned)
        });
        if let Some(Err(error)) = game_result.as_ref() {
            self.status =
                format!("Could not reload the disk copy; in-memory changes remain: {error}");
            return;
        }
        if let Some(Err(error)) = profile_result.as_ref() {
            self.status =
                format!("Could not reload the local profile; in-memory statistics remain: {error}");
            return;
        }
        if let Some(Ok((game, revision))) = game_result {
            self.replace_game(game);
            self.save_revisions[index] = Some(revision);
            self.dirty[index] = false;
            self.pending_new_deal_conflict = false;
            self.clear_selections();
        }
        if let Some(Ok((profile, revision))) = profile_result {
            self.local_profile = profile;
            self.local_profile_revision = Some(revision);
            self.local_profile_dirty = false;
        }
        if reload_game || reload_profile {
            self.status = if self.pending_new_deal.is_some() {
                "Reloaded newer disk state and refreshed save ownership; retry or cancel the pending deal change".into()
            } else {
                "Reloaded newer disk state; in-memory changes were discarded".into()
            };
        } else {
            self.status = "No unsaved disk state needs reloading".into();
        }
    }
}

fn parse_new_deal_variant(game: GameKind, variant: &str) -> Option<NewDealVariant> {
    if game == GameKind::Klondike {
        return parse_klondike_variant(variant);
    }
    match (game, variant) {
        (GameKind::Spider, "1 suit") => Some(NewDealVariant::Spider(SuitMode::One)),
        (GameKind::Spider, "2 suits") => Some(NewDealVariant::Spider(SuitMode::Two)),
        (GameKind::Spider, "4 suits") => Some(NewDealVariant::Spider(SuitMode::Four)),
        (GameKind::FreeCell, "Next numbered deal") => {
            Some(NewDealVariant::FreeCell(FreeCellDeal::Next))
        }
        (GameKind::TriPeaks, "Standard") => Some(NewDealVariant::TriPeaks { wraparound: false }),
        (GameKind::TriPeaks, "Ace-King wrap") => {
            Some(NewDealVariant::TriPeaks { wraparound: true })
        }
        (GameKind::Pyramid, "No redeals") => Some(NewDealVariant::Pyramid { max_redeals: 0 }),
        (GameKind::Pyramid, "1 redeal") => Some(NewDealVariant::Pyramid { max_redeals: 1 }),
        (GameKind::Pyramid, "2 redeals") => Some(NewDealVariant::Pyramid { max_redeals: 2 }),
        _ => None,
    }
}

fn parse_klondike_variant(variant: &str) -> Option<NewDealVariant> {
    let mut fields = variant.split(" · ");
    let draw_mode = match fields.next()? {
        "Draw 1" => DrawMode::One,
        "Draw 3" => DrawMode::Three,
        _ => return None,
    };
    let scoring = match fields.next() {
        None | Some("Standard") => Scoring::Standard,
        Some("Vegas") => Scoring::Vegas,
        _ => return None,
    };
    let max_redeals = match fields.next() {
        None | Some("Unlimited") => None,
        Some("1 redeal") => Some(1),
        Some("3 redeals") => Some(3),
        _ => return None,
    };
    let timed = match fields.next() {
        None | Some("Untimed") => false,
        Some("Timed") => true,
        _ => return None,
    };
    if fields.next().is_some() {
        return None;
    }
    Some(NewDealVariant::Klondike {
        draw_mode,
        scoring,
        max_redeals,
        timed,
    })
}

fn parse_freecell_deal_number(input: &str) -> Option<u64> {
    const MAX_U64_DECIMAL_DIGITS: usize = 20;
    if input.is_empty()
        || input.len() > MAX_U64_DECIMAL_DIGITS
        || !input.as_bytes().iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    input.parse().ok()
}

const fn new_deal_variant_matches(game: GameKind, variant: NewDealVariant) -> bool {
    matches!(
        (game, variant),
        (GameKind::Klondike, NewDealVariant::Klondike { .. })
            | (GameKind::Spider, NewDealVariant::Spider(_))
            | (GameKind::FreeCell, NewDealVariant::FreeCell(_))
            | (GameKind::TriPeaks, NewDealVariant::TriPeaks { .. })
            | (GameKind::Pyramid, NewDealVariant::Pyramid { .. })
    )
}

fn prospective_game(request: PendingNewDeal, seed: u64) -> ProspectiveGame {
    match (request.game, request.variant) {
        (
            GameKind::Klondike,
            NewDealVariant::Klondike {
                draw_mode,
                scoring,
                max_redeals,
                timed,
            },
        ) => ProspectiveGame::Klondike(Game::new(
            seed,
            Options {
                draw_mode,
                scoring,
                max_redeals,
                timed,
            },
        )),
        (GameKind::Spider, NewDealVariant::Spider(mode)) => {
            ProspectiveGame::Spider(SpiderGame::new(seed, mode))
        }
        (GameKind::FreeCell, NewDealVariant::FreeCell(_)) => {
            ProspectiveGame::FreeCell(FreeCellGame::new(seed))
        }
        (GameKind::TriPeaks, NewDealVariant::TriPeaks { wraparound }) => {
            ProspectiveGame::TriPeaks(TriPeaksGame::new(seed, tripeaks::Options { wraparound }))
        }
        (GameKind::Pyramid, NewDealVariant::Pyramid { max_redeals }) => {
            ProspectiveGame::Pyramid(PyramidGame::new(seed, pyramid::Options { max_redeals }))
        }
        _ => unreachable!("new-deal variant compatibility was checked before seed reservation"),
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

fn load_initial_local_profile(
    status: &mut String,
) -> (LocalProfile, Option<PathBuf>, Option<SaveRevision>) {
    let mut path = default_local_profile_path();
    let Some(profile_path) = path.clone() else {
        return (LocalProfile::default(), None, None);
    };
    match recover_local_profile_revisioned(&profile_path) {
        Ok(RecoveredSave::Loaded(profile, revision)) => (profile, path, Some(revision)),
        Ok(RecoveredSave::Quarantined {
            path: quarantined,
            reason,
            durability_warning,
        }) => {
            *status = durability_warning.map_or_else(
                || {
                    format!(
                        "Unreadable local profile preserved as {}; started empty local statistics ({reason})",
                        quarantined.display()
                    )
                },
                |warning| {
                    format!(
                        "Unreadable local profile moved to {}; directory durability is indeterminate ({warning}); started empty local statistics ({reason})",
                        quarantined.display()
                    )
                },
            );
            (LocalProfile::default(), path, None)
        }
        Err(SaveError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            (LocalProfile::default(), path, None)
        }
        Err(error) => {
            *status = format!(
                "Local profile recovery failed; original left untouched and statistics disabled ({error})"
            );
            path = None;
            (LocalProfile::default(), path, None)
        }
    }
}

fn load_initial_deal_counters(
    path: Option<&std::path::Path>,
    defaults: DealCounters,
    status: &mut String,
) -> DealCounters {
    let mut loaded = false;
    let mut counters = path
        .and_then(|path| match load_deal_counters(path) {
            Ok(counters) => {
                loaded = true;
                Some(counters)
            }
            Err(SaveError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                *status = format!(
                    "Deal counters could not be restored; current game seeds were used: {error}"
                );
                None
            }
        })
        .unwrap_or(defaults);
    raise_deal_counters(&mut counters, defaults, loaded);
    counters
}

fn raise_deal_counters(
    counters: &mut DealCounters,
    minimum: DealCounters,
    preserve_loaded_freecell: bool,
) {
    counters.klondike = counters.klondike.max(minimum.klondike);
    counters.spider = counters.spider.max(minimum.spider);
    if !preserve_loaded_freecell {
        counters.freecell = counters.freecell.max(minimum.freecell);
    }
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
    app.on_klondike_top_x(klondike_top_x);
    app.on_tripeaks_x(tripeaks_x);
    app.on_pyramid_x(pyramid_x);
    app.on_pyramid_y(pyramid_y);
    let controller = Rc::new(RefCell::new(Controller::new()));
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        app.window().on_close_requested(move || {
            let mut controller = controller.borrow_mut();
            if controller.close_requested() {
                slint::CloseRequestResponse::HideWindow
            } else {
                if let Some(app) = weak.upgrade() {
                    render(&app, &controller);
                }
                slint::CloseRequestResponse::KeepWindowShown
            }
        });
    }
    render(&app, &controller.borrow());
    register_klondike_handlers(&app, &controller);
    register_spider_freecell_handlers(&app, &controller);
    register_toolbar_handlers(&app, &controller);
    let timer = Timer::default();
    {
        let weak = app.as_weak();
        let controller = Rc::clone(&controller);
        let last_tick = Cell::new(Instant::now());
        timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
            let now = Instant::now();
            let mut state = controller.borrow_mut();
            if !state.klondike_timer_running() {
                last_tick.set(now);
                return;
            }
            let previous = last_tick.get();
            let seconds = now.saturating_duration_since(previous).as_secs();
            if seconds == 0 {
                return;
            }
            last_tick.set(previous + Duration::from_secs(seconds));
            if state.advance_klondike_timer(seconds)
                && let Some(app) = weak.upgrade()
            {
                render(&app, &state);
            }
        });
    }
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
    register_klondike_pointer_handlers(app, &controller);
}

fn waste_pointer_identity(
    card: &str,
    deal_instance: &str,
    interaction_generation: &str,
) -> KlondikePointerIdentity {
    KlondikePointerIdentity {
        source: KlondikePointerSource::Waste,
        card: card.into(),
        deal_instance: deal_instance.into(),
        interaction_generation: interaction_generation.into(),
    }
}

fn tableau_pointer_identity(
    column: i32,
    card_index: i32,
    card: &str,
    deal_instance: &str,
    interaction_generation: &str,
) -> KlondikePointerIdentity {
    KlondikePointerIdentity {
        source: KlondikePointerSource::Tableau { column, card_index },
        card: card.into(),
        deal_instance: deal_instance.into(),
        interaction_generation: interaction_generation.into(),
    }
}

fn register_klondike_pointer_handlers(app: &AppWindow, controller: &Rc<RefCell<Controller>>) {
    let pointer_click = Rc::new(PointerClickTimer::default());
    register_klondike_waste_pointer_handlers(app, controller, &pointer_click);
    register_klondike_tableau_pointer_handlers(app, controller, &pointer_click);
}

fn register_klondike_waste_pointer_handlers(
    app: &AppWindow,
    controller: &Rc<RefCell<Controller>>,
    pointer_click: &Rc<PointerClickTimer>,
) {
    {
        let pointer_click = Rc::clone(pointer_click);
        app.on_waste_pointer_pressed(move |card, deal_instance, interaction_generation| {
            pointer_click.pointer_pressed(&waste_pointer_identity(
                card.as_str(),
                deal_instance.as_str(),
                interaction_generation.as_str(),
            ));
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(controller);
        let pointer_click = Rc::clone(pointer_click);
        app.on_waste_pointer_activated(move |card, deal_instance, interaction_generation| {
            let identity = waste_pointer_identity(
                card.as_str(),
                deal_instance.as_str(),
                interaction_generation.as_str(),
            );
            let weak = weak.clone();
            let controller = Rc::clone(&controller);
            pointer_click.pointer_clicked(identity, move || {
                if controller.borrow().interaction_generation.to_string()
                    != interaction_generation.as_str()
                {
                    return;
                }
                update(&weak, &controller, |state| {
                    state.activate_waste_pointer(
                        card.as_str(),
                        deal_instance.as_str(),
                        interaction_generation.as_str(),
                    );
                });
            });
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(controller);
        let pointer_click = Rc::clone(pointer_click);
        app.on_waste_double_activated(move |card, deal_instance, interaction_generation| {
            let identity = waste_pointer_identity(
                card.as_str(),
                deal_instance.as_str(),
                interaction_generation.as_str(),
            );
            if !pointer_click.take_double(&identity) {
                update(&weak, &controller, Controller::reject_klondike_pointer);
                return;
            }
            update(&weak, &controller, |state| {
                state.double_activate_waste(
                    card.as_str(),
                    deal_instance.as_str(),
                    interaction_generation.as_str(),
                );
            });
        });
    }
}

fn register_klondike_tableau_pointer_handlers(
    app: &AppWindow,
    controller: &Rc<RefCell<Controller>>,
    pointer_click: &Rc<PointerClickTimer>,
) {
    {
        let pointer_click = Rc::clone(pointer_click);
        app.on_tableau_pointer_pressed(
            move |column, index, card, deal_instance, interaction_generation| {
                pointer_click.pointer_pressed(&tableau_pointer_identity(
                    column,
                    index,
                    card.as_str(),
                    deal_instance.as_str(),
                    interaction_generation.as_str(),
                ));
            },
        );
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(controller);
        let pointer_click = Rc::clone(pointer_click);
        app.on_tableau_pointer_activated(
            move |column, index, card, deal_instance, interaction_generation| {
                let identity = tableau_pointer_identity(
                    column,
                    index,
                    card.as_str(),
                    deal_instance.as_str(),
                    interaction_generation.as_str(),
                );
                let weak = weak.clone();
                let controller = Rc::clone(&controller);
                pointer_click.pointer_clicked(identity, move || {
                    if controller.borrow().interaction_generation.to_string()
                        != interaction_generation.as_str()
                    {
                        return;
                    }
                    update(&weak, &controller, |state| {
                        state.activate_tableau_pointer(
                            column,
                            index,
                            card.as_str(),
                            deal_instance.as_str(),
                            interaction_generation.as_str(),
                        );
                    });
                });
            },
        );
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(controller);
        let pointer_click = Rc::clone(pointer_click);
        app.on_tableau_double_activated(
            move |column, index, card, deal_instance, interaction_generation| {
                let identity = tableau_pointer_identity(
                    column,
                    index,
                    card.as_str(),
                    deal_instance.as_str(),
                    interaction_generation.as_str(),
                );
                if !pointer_click.take_double(&identity) {
                    update(&weak, &controller, Controller::reject_klondike_pointer);
                    return;
                }
                update(&weak, &controller, |state| {
                    state.double_activate_tableau(
                        column,
                        index,
                        card.as_str(),
                        deal_instance.as_str(),
                        interaction_generation.as_str(),
                    );
                });
            },
        );
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

fn register_deal_handlers(app: &AppWindow, controller: &Rc<RefCell<Controller>>) {
    {
        let weak = app.as_weak();
        let controller = Rc::clone(controller);
        app.on_restart_deal_requested(move || {
            update(&weak, &controller, Controller::restart_current_deal);
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(controller);
        app.on_new_game(move |mode| {
            update(&weak, &controller, |state| state.new_game(mode.as_str()));
        });
    }
    {
        let weak = app.as_weak();
        let controller = Rc::clone(controller);
        app.on_new_freecell_deal(move |deal_number| {
            update(&weak, &controller, |state| {
                state.new_freecell_game(deal_number.as_str());
            });
        });
    }
}

fn register_toolbar_handlers(app: &AppWindow, controller: &Rc<RefCell<Controller>>) {
    register_deal_handlers(app, controller);
    let controller = Rc::clone(controller);
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
    controller.advance_interaction_generation();
    if let Some(app) = weak.upgrade() {
        render(&app, &controller);
    }
}

fn render(app: &AppWindow, controller: &Controller) {
    app.set_game_kind(controller.game_name().into());
    app.set_interaction_generation(controller.interaction_generation.to_string().into());
    app.set_can_undo(controller.can_undo());
    app.set_can_redo(controller.can_redo());
    app.set_has_unsaved_changes(
        controller.dirty[controller.active_index()] || controller.local_profile_dirty,
    );
    app.set_has_any_unsaved_changes(
        controller.dirty.iter().any(|dirty| *dirty) || controller.local_profile_dirty,
    );
    app.set_has_pending_new_deal(controller.pending_new_deal.is_some());
    app.set_pending_deal_is_restart(
        controller
            .pending_new_deal
            .is_some_and(PendingNewDeal::is_restart),
    );
    app.set_has_pending_save_conflict(controller.pending_new_deal_conflict);
    app.set_status_text(controller.status.as_str().into());
    let statistics = controller
        .local_profile
        .statistics(controller.profile_game_kind());
    app.set_local_statistics(
        format!(
            "Local: {} played · {} won",
            statistics.deals_played, statistics.deals_won
        )
        .into(),
    );
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
    app.set_klondike_deal_instance(controller.klondike_deal_instance.to_string().into());
    app.set_redeals(i32::from(state.redeals));
    app.set_redeals_remaining(state.options.max_redeals.map_or(-1, |maximum| {
        i32::from(maximum.saturating_sub(state.redeals))
    }));
    app.set_klondike_timed_active(state.options.timed);
    app.set_elapsed_time(format_elapsed_time(state.elapsed_seconds).into());
    render_klondike_options(app, controller);
}

fn format_elapsed_time(elapsed_seconds: u64) -> String {
    let hours = elapsed_seconds / 3_600;
    let minutes = elapsed_seconds % 3_600 / 60;
    let seconds = elapsed_seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

fn render_klondike_options(app: &AppWindow, controller: &Controller) {
    if let Some(options) = klondike_ui_options_for_render(controller) {
        app.set_klondike_draw_index(options.draw_index);
        app.set_klondike_draw_mode(options.draw_mode.into());
        app.set_klondike_scoring_index(options.scoring_index);
        app.set_klondike_scoring_mode(options.scoring_mode.into());
        app.set_klondike_redeal_index(options.redeal_index);
        app.set_klondike_redeal_limit(options.redeal_limit.into());
        app.set_klondike_timing_index(options.timing_index);
        app.set_klondike_timing_mode(options.timing_mode.into());
    }
}

fn klondike_ui_options_for_render(controller: &Controller) -> Option<KlondikeUiOptions> {
    controller
        .pending_new_deal
        .is_none()
        .then(|| klondike_ui_options(controller.game.state.options))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KlondikeUiOptions {
    draw_index: i32,
    draw_mode: &'static str,
    scoring_index: i32,
    scoring_mode: &'static str,
    redeal_index: i32,
    redeal_limit: &'static str,
    timing_index: i32,
    timing_mode: &'static str,
}

fn klondike_ui_options(options: Options) -> KlondikeUiOptions {
    let (draw_index, draw_mode) = match options.draw_mode {
        DrawMode::One => (0, "Draw 1"),
        DrawMode::Three => (1, "Draw 3"),
    };
    let (scoring_index, scoring_mode) = match options.scoring {
        Scoring::Standard => (0, "Standard"),
        Scoring::Vegas => (1, "Vegas"),
    };
    let (redeal_index, redeal_limit) = match options.max_redeals {
        None => (0, "Unlimited"),
        Some(1) => (1, "1 redeal"),
        Some(3) => (2, "3 redeals"),
        Some(_) => (-1, "Custom"),
    };
    let (timing_index, timing_mode) = if options.timed {
        (1, "Timed")
    } else {
        (0, "Untimed")
    };
    KlondikeUiOptions {
        draw_index,
        draw_mode,
        scoring_index,
        scoring_mode,
        redeal_index,
        redeal_limit,
        timing_index,
        timing_mode,
    }
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
    let (_, active_mode) = spider_ui_mode(state.mode);
    app.set_spider_suit_mode_active(active_mode.into());
    if let Some((index, mode)) = spider_ui_mode_for_render(controller) {
        app.set_spider_suit_index(index);
        app.set_spider_suit_mode(mode.into());
    }
}

fn spider_ui_mode_for_render(controller: &Controller) -> Option<(i32, &'static str)> {
    controller
        .pending_new_deal
        .is_none()
        .then(|| spider_ui_mode(controller.spider.state.mode))
}

const fn spider_ui_mode(mode: SuitMode) -> (i32, &'static str) {
    match mode {
        SuitMode::One => (0, "1 suit"),
        SuitMode::Two => (1, "2 suits"),
        SuitMode::Four => (2, "4 suits"),
    }
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
    app.set_deal_id(0);
    app.set_deal_number(freecell_deal_number(state.deal_number));
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
    app.set_tripeaks_wraparound_active(state.options.wraparound);
    if let Some((index, label)) = tripeaks_ui_rule_for_render(controller) {
        app.set_tripeaks_rule_index(index);
        app.set_tripeaks_rule_mode(label.into());
    }
}

fn tripeaks_ui_rule_for_render(controller: &Controller) -> Option<(i32, &'static str)> {
    controller.pending_new_deal.is_none().then_some(
        if controller.tripeaks.state.options.wraparound {
            (1, "Ace-King wrap")
        } else {
            (0, "Standard")
        },
    )
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
    app.set_pyramid_max_redeals_active(i32::from(state.options.max_redeals));
    render_pyramid_options(app, controller);
}

fn render_pyramid_options(app: &AppWindow, controller: &Controller) {
    if let Some(options) = pyramid_ui_options_for_render(controller) {
        app.set_pyramid_redeal_index(options.redeal_index);
        app.set_pyramid_redeal_limit(options.redeal_limit.into());
    }
}

fn pyramid_ui_options_for_render(controller: &Controller) -> Option<PyramidUiOptions> {
    controller
        .pending_new_deal
        .is_none()
        .then(|| pyramid_ui_options(controller.pyramid.state.options))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PyramidUiOptions {
    redeal_index: i32,
    redeal_limit: String,
}

fn pyramid_ui_options(options: pyramid::Options) -> PyramidUiOptions {
    let (redeal_index, redeal_limit) = match options.max_redeals {
        0 => (0, "No redeals".into()),
        1 => (1, "1 redeal".into()),
        2 => (2, "2 redeals".into()),
        maximum => (-1, format!("{maximum} redeals")),
    };
    PyramidUiOptions {
        redeal_index,
        redeal_limit,
    }
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

fn freecell_deal_number(seed: u64) -> SharedString {
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

fn klondike_top_x(slot: i32, available_width: f32, left_handed: bool) -> f32 {
    const CARD_WIDTH: f32 = 104.0;
    const PILE_STEP: f32 = 116.0;
    const WASTE_OFFSET: f32 = 126.0;

    if !available_width.is_finite() {
        return 0.0;
    }
    let maximum = (available_width - CARD_WIDTH).max(0.0);
    let right_handed = match slot {
        0 => 0.0,
        1 => WASTE_OFFSET,
        2..=5 => {
            available_width - f32::from(i16::try_from(6 - slot).unwrap_or_default()) * PILE_STEP
        }
        _ => return 0.0,
    }
    .clamp(0.0, maximum);
    if left_handed {
        maximum - right_handed
    } else {
        right_handed
    }
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
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    fn expire_pointer_timer() {
        slint::platform::update_timers_and_animations();
    }

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
            klondike_deal_instance: 0,
            interaction_generation: 0,
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
            local_profile: LocalProfile::default(),
            local_profile_path: None,
            local_profile_revision: None,
            local_profile_dirty: false,
            klondike_elapsed_dirty: false,
            klondike_uncheckpointed_seconds: 0,
            status: "Ready".into(),
        }
    }

    fn drain_pyramid_stock(controller: &mut Controller) {
        while !controller.pyramid.state.stock.is_empty() {
            controller.draw_pyramid_stock();
        }
    }

    #[test]
    fn controller_records_and_reopens_one_played_deal_idempotently() {
        let game_path = test_save("profile-played-game");
        let profile_path = test_save("profile-played");
        remove_save(&game_path);
        remove_save(&profile_path);
        let mut controller = controller(41);
        controller.save_path = Some(game_path.clone());
        controller.local_profile_path = Some(profile_path.clone());

        controller.draw_or_recycle();
        controller.undo();
        controller.redo();

        let statistics = controller
            .local_profile
            .statistics(ProfileGameKind::Klondike);
        assert_eq!(statistics.deals_played, 1);
        assert_eq!(statistics.deals_won, 0);
        assert!(!controller.local_profile_dirty);
        assert_eq!(
            solitaire::persistence::load_local_profile(&profile_path).unwrap(),
            controller.local_profile
        );
        remove_save(&game_path);
        remove_save(&profile_path);
    }

    #[test]
    fn controller_records_a_win_once_across_undo_and_redo() {
        let profile_path = test_save("profile-win");
        remove_save(&profile_path);
        let mut controller = controller(7);
        controller.active = GameKind::Pyramid;
        controller.local_profile_path = Some(profile_path.clone());
        controller.pyramid.state.pyramid = [None; 28];
        controller.pyramid.state.pyramid[27] = Some(Card::new(Suit::Spades, Rank::King));

        controller.activate_pyramid_card(27);
        controller.undo();
        controller.redo();

        let statistics = controller
            .local_profile
            .statistics(ProfileGameKind::Pyramid);
        assert_eq!(statistics.deals_played, 1);
        assert_eq!(statistics.deals_won, 1);
        assert!(!controller.local_profile_dirty);
        remove_save(&profile_path);
    }

    #[test]
    fn local_profile_conflict_preserves_memory_until_explicit_reload() {
        let game_path = test_save("profile-conflict-game");
        let profile_path = test_save("profile-conflict");
        remove_save(&game_path);
        remove_save(&profile_path);
        solitaire::persistence::save_local_profile(&profile_path, &LocalProfile::default())
            .unwrap();
        let (loaded, revision) = load_local_profile_revisioned(&profile_path).unwrap();
        let (mut external, external_revision) =
            load_local_profile_revisioned(&profile_path).unwrap();
        external
            .observe(ProfileGameKind::Klondike, 42, false)
            .unwrap();
        let mut external_expected = Some(external_revision);
        save_local_profile_checked(&profile_path, &external, &mut external_expected).unwrap();

        let mut controller = controller(41);
        controller.save_path = Some(game_path.clone());
        controller.local_profile = loaded;
        controller.local_profile_path = Some(profile_path.clone());
        controller.local_profile_revision = Some(revision);
        controller.draw_or_recycle();

        assert!(controller.local_profile_dirty);
        assert!(controller.status.contains("profile save failed"));
        assert_eq!(
            controller
                .local_profile
                .statistics(ProfileGameKind::Klondike)
                .latest_played_deal,
            Some(41)
        );
        assert_eq!(
            solitaire::persistence::load_local_profile(&profile_path).unwrap(),
            external
        );

        controller.reload_disk_copy();
        assert!(!controller.local_profile_dirty);
        assert_eq!(controller.local_profile, external);
        remove_save(&game_path);
        remove_save(&profile_path);
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
    fn klondike_handed_layout_is_an_exact_bounded_mirror() {
        let width = 1_084.0;
        let maximum = width - 104.0;
        let right = [0.0, 126.0, 620.0, 736.0, 852.0, 968.0];
        for (slot, expected) in right.into_iter().enumerate() {
            let slot = i32::try_from(slot).unwrap();
            assert!((klondike_top_x(slot, width, false) - expected).abs() < f32::EPSILON);
            assert!((klondike_top_x(slot, width, true) + expected - maximum).abs() < f32::EPSILON);
        }

        for width in [-100.0, 0.0, 80.0, 104.0, 500.0] {
            let maximum = (width - 104.0_f32).max(0.0);
            for slot in -2..=7 {
                for left_handed in [false, true] {
                    let x = klondike_top_x(slot, width, left_handed);
                    assert!(x.is_finite());
                    assert!((0.0..=maximum).contains(&x));
                }
            }
        }
        assert!(klondike_top_x(0, f32::INFINITY, false).abs() < f32::EPSILON);
        assert!(klondike_top_x(0, f32::NAN, true).abs() < f32::EPSILON);
    }

    #[test]
    fn pointer_click_timer_is_idle_single_shot_and_cancelable() {
        let calls = Rc::new(Cell::new(0_u8));
        let timer = Rc::new(PointerClickTimer::default());
        let identity = waste_pointer_identity("A♣", "0", "0");
        assert_eq!(POINTER_DOUBLE_CLICK_INTERVAL, Duration::from_millis(500));
        assert_eq!(calls.get(), 0);
        assert!(!timer.timer.running());

        let fired = Rc::clone(&calls);
        timer.pointer_clicked_after(identity.clone(), Duration::ZERO, move || {
            fired.set(fired.get() + 1);
        });
        assert!(timer.timer.running());
        expire_pointer_timer();
        assert_eq!(calls.get(), 1);
        assert!(!timer.timer.running());
        expire_pointer_timer();
        assert_eq!(calls.get(), 1);

        let fired = Rc::clone(&calls);
        timer.pointer_clicked_after(identity.clone(), Duration::ZERO, move || {
            fired.set(fired.get() + 1);
        });
        timer.pointer_pressed(&identity);
        expire_pointer_timer();
        assert_eq!(calls.get(), 1);
        assert!(!timer.timer.running());
    }

    #[test]
    fn deferred_pointer_click_cannot_overtake_keyboard_or_stock_input() {
        let timer = Rc::new(PointerClickTimer::default());
        let controller = Rc::new(RefCell::new(controller(0)));
        let tableau = controller.borrow().game.state.tableau[0].clone();
        let index = i32::try_from(tableau.len() - 1).unwrap();
        let card = card_label(tableau.last().unwrap().card);

        let delayed = Rc::clone(&controller);
        let delayed_card = card.clone();
        let identity = tableau_pointer_identity(0, index, &card, "0", "0");
        timer.pointer_clicked_after(identity, Duration::ZERO, move || {
            let mut state = delayed.borrow_mut();
            if state.interaction_generation == 0 {
                state.activate_tableau_pointer(0, index, &delayed_card, "0", "0");
            }
        });
        let keyboard_index =
            i32::try_from(controller.borrow().game.state.tableau[1].len() - 1).unwrap();
        controller.borrow_mut().activate_tableau(1, keyboard_index);
        controller.borrow_mut().interaction_generation = 1;
        let keyboard_selection = controller.borrow().selection;
        expire_pointer_timer();
        assert!(controller.borrow().selection == keyboard_selection);
        assert_eq!(controller.borrow().interaction_generation, 1);

        let delayed = Rc::clone(&controller);
        let delayed_card = card;
        let identity = tableau_pointer_identity(0, index, &delayed_card, "0", "1");
        timer.pointer_clicked_after(identity, Duration::ZERO, move || {
            let mut state = delayed.borrow_mut();
            if state.interaction_generation == 1 {
                state.activate_tableau_pointer(0, index, &delayed_card, "0", "1");
            }
        });
        controller.borrow_mut().draw_or_recycle();
        controller.borrow_mut().interaction_generation = 2;
        let after_stock = controller.borrow().game.clone();
        let after_stock_selection = controller.borrow().selection;
        expire_pointer_timer();
        assert_eq!(controller.borrow().game, after_stock);
        assert!(controller.borrow().selection == after_stock_selection);
        assert_eq!(controller.borrow().interaction_generation, 2);
    }

    #[test]
    fn double_click_requires_matching_first_click_identity() {
        let timer = Rc::new(PointerClickTimer::default());
        let direct = waste_pointer_identity("A♣", "0", "0");
        assert!(!timer.take_double(&direct));

        let game_path = test_save("double-click-rebase");
        remove_save(&game_path);
        fs::write(&game_path, b"owner-bytes").unwrap();
        let controller = Rc::new(RefCell::new(controller(0)));
        controller.borrow_mut().draw_or_recycle();
        let first_card = *controller.borrow().game.state.waste.last().unwrap();
        let first = waste_pointer_identity(&card_label(first_card), "0", "0");
        timer.pointer_clicked_after(first, Duration::ZERO, || {});

        controller.borrow_mut().draw_or_recycle();
        controller.borrow_mut().interaction_generation = 1;
        let second_card = *controller.borrow().game.state.waste.last().unwrap();
        let second = waste_pointer_identity(&card_label(second_card), "0", "1");
        timer.pointer_pressed(&second);
        timer.pointer_clicked_after(second.clone(), Duration::ZERO, || {});
        let before_game = controller.borrow().game.clone();
        let before_selection = controller.borrow().selection;
        let before_profile = controller.borrow().local_profile.clone();
        assert!(!timer.take_double(&second));
        assert_eq!(controller.borrow().game, before_game);
        assert!(controller.borrow().selection == before_selection);
        assert_eq!(controller.borrow().local_profile, before_profile);
        assert_eq!(fs::read(&game_path).unwrap(), b"owner-bytes");

        let legitimate = tableau_pointer_identity(2, 3, "Q♥", "7", "9");
        timer.pointer_clicked_after(legitimate.clone(), Duration::ZERO, || {});
        timer.pointer_pressed(&legitimate);
        timer.pointer_clicked_after(legitimate.clone(), Duration::ZERO, || {});
        assert!(timer.take_double(&legitimate));
        expire_pointer_timer();
        remove_save(&game_path);
    }

    #[test]
    fn blocked_close_invalidates_a_pending_pointer_click() {
        let timer = Rc::new(PointerClickTimer::default());
        let game_path = test_save("blocked-close-pointer");
        remove_save(&game_path);
        fs::write(&game_path, b"owner-close-bytes").unwrap();
        let controller = Rc::new(RefCell::new(controller(0)));
        let tableau = controller.borrow().game.state.tableau[0].clone();
        let index = i32::try_from(tableau.len() - 1).unwrap();
        let card = card_label(tableau.last().unwrap().card);
        let identity = tableau_pointer_identity(0, index, &card, "0", "0");
        let delayed = Rc::clone(&controller);
        let delayed_game_path = game_path.clone();
        timer.pointer_clicked_after(identity, Duration::ZERO, move || {
            if delayed.borrow().interaction_generation == 0 {
                let mut state = delayed.borrow_mut();
                state.draw_or_recycle();
                state.activate_tableau_pointer(0, index, &card, "0", "0");
                state
                    .local_profile
                    .observe(ProfileGameKind::Klondike, 0, false)
                    .unwrap();
                drop(state);
                fs::write(&delayed_game_path, b"stale-close-callback-ran").unwrap();
            }
        });
        controller.borrow_mut().dirty[0] = true;
        controller.borrow_mut().local_profile_dirty = true;
        let before_game = controller.borrow().game.clone();
        let before_selection = controller.borrow().selection;
        let before_profile = controller.borrow().local_profile.clone();

        assert!(!controller.borrow_mut().close_requested());
        assert_eq!(controller.borrow().interaction_generation, 1);
        expire_pointer_timer();
        assert_eq!(controller.borrow().game, before_game);
        assert!(controller.borrow().selection == before_selection);
        assert_eq!(controller.borrow().local_profile, before_profile);
        assert_eq!(fs::read(&game_path).unwrap(), b"owner-close-bytes");
        remove_save(&game_path);
    }

    #[test]
    fn klondike_double_activation_is_exact_atomic_and_undoable() {
        let envelope: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/klondike-seed-zero-near-win.json"
        ))
        .unwrap();
        let replay: solitaire::replay::Replay<Action, solitaire::klondike::ReplaySetup> =
            serde_json::from_value(envelope["payload"].clone()).unwrap();
        let near_win = Game::from_replay(&replay).unwrap();
        let mut controller = controller(0);
        controller.game = near_win.clone();
        controller.selection = Some(Selection::Waste);
        let token = card_label(controller.game.state.tableau[0][0].card);

        for (column, index, supplied) in [
            (-1, 0, token.as_str()),
            (i32::MAX, 0, token.as_str()),
            (0, -1, token.as_str()),
            (0, 1, token.as_str()),
            (0, 0, "Q♦"),
        ] {
            controller.double_activate_tableau(column, index, supplied, "0", "0");
            assert_eq!(controller.game, near_win);
            assert!(controller.selection == Some(Selection::Waste));
            assert!(controller.status.contains("click again"));
        }
        let oversized = "K".repeat(4_096);
        controller.double_activate_tableau(0, 0, &oversized, "0", "0");
        assert_eq!(controller.game, near_win);
        assert!(controller.selection == Some(Selection::Waste));

        controller.klondike_deal_instance = 1;
        controller.double_activate_tableau(0, 0, &token, "0", "0");
        assert_eq!(controller.game, near_win);
        assert!(controller.selection == Some(Selection::Waste));
        controller.klondike_deal_instance = 0;
        controller.active = GameKind::Spider;
        controller.double_activate_tableau(0, 0, &token, "0", "0");
        assert_eq!(controller.game, near_win);
        assert!(controller.selection == Some(Selection::Waste));
        controller.active = GameKind::Klondike;

        controller.interaction_generation = 1;
        controller.double_activate_tableau(0, 0, &token, "0", "0");
        assert!(controller.selection == Some(Selection::Waste));
        controller.interaction_generation = 0;

        controller.select_game("Spider");
        controller.select_game("Klondike");
        controller.selection = Some(Selection::Waste);
        controller.double_activate_tableau(0, 0, &token, "0", "0");
        assert_eq!(controller.game, near_win);
        assert!(controller.selection == Some(Selection::Waste));
        assert!(controller.status.contains("click again"));
        let current_instance = controller.klondike_deal_instance.to_string();

        controller.selection = None;
        controller.activate_tableau_pointer(0, 0, &token, &current_instance, "0");
        assert!(
            controller.selection
                == Some(Selection::Tableau {
                    column: 0,
                    count: 1,
                })
        );
        assert_eq!(controller.game, near_win);

        controller.selection = Some(Selection::Waste);
        controller.double_activate_tableau(0, 0, &token, &current_instance, "0");
        assert!(controller.game.state.is_won());
        assert!(controller.selection.is_none());
        assert_eq!(controller.game.state.moves, 156);
        assert_eq!(controller.game.state.score, 365);
        assert!(controller.game.undo());
        assert_eq!(controller.game.state, near_win.state);
        assert_eq!(controller.game.replay(), near_win.replay());
        assert!(controller.game.redo());
        assert!(controller.game.state.is_won());

        let won = controller.game.clone();
        controller.double_activate_waste("A♣", &current_instance, "0");
        assert_eq!(controller.game, won);
        assert!(controller.status.contains("click again"));

        let mut waste_route = None;
        'seeds: for seed in 0..16 {
            let mut game = Game::new(seed, Options::default());
            while !game.state.stock.is_empty() {
                game.apply(Action::Draw).unwrap();
                let card = *game.state.waste.last().unwrap();
                let action = Action::Move {
                    from: Pile::Waste,
                    to: Pile::Foundation(card.suit),
                    count: 1,
                };
                let mut expected = game.clone();
                if expected.apply(action).is_ok() {
                    waste_route = Some((game, card, expected));
                    break 'seeds;
                }
            }
        }
        let (waste_game, waste_card, expected) = waste_route.expect("bounded waste route");
        let mut waste_controller = self::controller(waste_game.state.seed);
        waste_controller.game = waste_game.clone();
        waste_controller.double_activate_waste(&card_label(waste_card), "0", "0");
        assert_eq!(waste_controller.game, expected);
        assert!(waste_controller.game.undo());
        assert_eq!(waste_controller.game.state, waste_game.state);
        assert_eq!(waste_controller.game.replay(), waste_game.replay());
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
    fn spider_suit_options_are_strict_atomic_reopenable_and_mapped() {
        for (label, mode, index) in [
            ("1 suit", SuitMode::One, 0),
            ("2 suits", SuitMode::Two, 1),
            ("4 suits", SuitMode::Four, 2),
        ] {
            let path = test_save(&format!("spider-suit-{index}"));
            remove_save(&path);
            let mut controller = controller(51 + u64::try_from(index).unwrap());
            controller.select_game("Spider");
            controller.spider_save_path = Some(path.clone());
            controller.new_game(label);
            assert_eq!(controller.spider.state.mode, mode);
            assert_eq!(load_spider_revisioned(&path).unwrap().0, controller.spider);
            assert_eq!(spider_ui_mode_for_render(&controller), Some((index, label)));
            remove_save(&path);
        }

        let game_path = test_save("spider-suit-dirty");
        let counter_path = test_save("spider-suit-counter");
        remove_save(&game_path);
        remove_save(&counter_path);
        let mut controller = controller(55);
        controller.select_game("Spider");
        controller.spider_save_path = Some(game_path.clone());
        controller.deal_counters_path = Some(counter_path.clone());
        controller.new_game("2 suits");
        let current = controller.spider.clone();
        let saved = fs::read(&game_path).unwrap();
        let counters = fs::read(&counter_path).unwrap();
        controller.dirty[controller.active_index()] = true;
        controller.new_game("4 suits");
        let pending = controller.pending_new_deal;
        let oversized = "S".repeat(4_096);
        for invalid in [
            "",
            "one suit",
            "1 suits",
            " 2 suits",
            "2 suits ",
            "3 suits",
            "4 suits · trailing",
            oversized.as_str(),
        ] {
            controller.new_game(invalid);
            assert!(controller.pending_new_deal == pending, "{invalid:?}");
            assert_eq!(controller.spider, current, "{invalid:?}");
            assert_eq!(fs::read(&game_path).unwrap(), saved, "{invalid:?}");
            assert_eq!(fs::read(&counter_path).unwrap(), counters, "{invalid:?}");
        }
        assert_eq!(spider_ui_mode_for_render(&controller), None);
        assert_eq!(
            fs::metadata(&game_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&counter_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        controller.discard_progress_and_start_pending();
        assert_eq!(controller.spider.state.mode, SuitMode::Four);
        assert!(controller.pending_new_deal.is_none());
        assert_eq!(
            load_spider_revisioned(&game_path).unwrap().0,
            controller.spider
        );
        remove_save(&game_path);
        remove_save(&counter_path);
    }

    #[test]
    fn controller_completes_legal_klondike_replay_once_and_reopens() {
        let game_path = test_save("klondike-near-win");
        let profile_path = test_save("klondike-near-win-profile");
        remove_save(&game_path);
        remove_save(&profile_path);
        let envelope: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/klondike-seed-zero-near-win.json"
        ))
        .unwrap();
        let replay: solitaire::replay::Replay<Action, solitaire::klondike::ReplaySetup> =
            serde_json::from_value(envelope["payload"].clone()).unwrap();
        let staged = Game::from_replay(&replay).unwrap();
        solitaire::persistence::save_klondike(&game_path, &staged).unwrap();
        let (near_win, revision) = load_klondike_revisioned(&game_path).unwrap();
        let completed_statistics = solitaire::profile::GameStatistics {
            deals_played: 1,
            deals_won: 1,
            latest_played_deal: Some(0),
            latest_won_deal: Some(0),
        };

        let mut controller = controller(0);
        controller.active = GameKind::Klondike;
        controller.game = near_win;
        controller.save_path = Some(game_path.clone());
        controller.save_revisions[0] = Some(revision);
        controller.local_profile_path = Some(profile_path.clone());
        assert!(!profile_path.exists());
        assert_eq!(
            controller
                .local_profile
                .statistics(ProfileGameKind::Klondike),
            solitaire::profile::GameStatistics::default()
        );

        controller.activate_tableau(0, 0);
        assert!(matches!(
            controller.selection,
            Some(Selection::Tableau {
                column: 0,
                count: 1,
            })
        ));
        controller.activate_foundation(1);

        assert_eq!(controller.status, "Deal complete — beautifully played");
        assert!(controller.game.state.is_won());
        assert_eq!(controller.game.state.card_count(), 52);
        assert_eq!(controller.game.state.moves, 156);
        assert_eq!(controller.game.state.score, 365);
        assert_eq!(
            controller
                .local_profile
                .statistics(ProfileGameKind::Klondike),
            completed_statistics
        );
        assert!(!controller.local_profile_dirty);
        assert_eq!(
            fs::metadata(&game_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&profile_path).unwrap().permissions().mode() & 0o777,
            0o600
        );

        let won = controller.game.clone();
        let won_save = fs::read(&game_path).unwrap();
        let envelope: serde_json::Value = serde_json::from_slice(&won_save).unwrap();
        assert_eq!(envelope["version"], 1);
        assert_eq!(envelope["game"], "klondike");
        assert_eq!(envelope["payload"]["state"]["seed"], 0);
        assert_eq!(envelope["payload"]["state"]["options"]["draw_mode"], "One");
        assert_eq!(
            envelope["payload"]["state"]["options"]["scoring"],
            "Standard"
        );
        assert_eq!(
            envelope["payload"]["actions"].as_array().unwrap().len(),
            156
        );
        assert_eq!(load_klondike_revisioned(&game_path).unwrap().0, won);

        let profile_bytes = fs::read(&profile_path).unwrap();
        controller.undo();
        assert_eq!(controller.status, "Move undone");
        assert!(!controller.game.state.is_won());
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);
        controller.redo();
        assert_eq!(controller.status, "Move restored");
        assert_eq!(controller.game, won);
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);
        controller.observe_active_profile();
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);

        let (reopened_game, _) = load_klondike_revisioned(&game_path).unwrap();
        let (reopened_profile, _) = load_local_profile_revisioned(&profile_path).unwrap();
        assert_eq!(reopened_game, won);
        assert_eq!(
            reopened_profile.statistics(ProfileGameKind::Klondike),
            completed_statistics
        );
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);
        assert_eq!(fs::read(&game_path).unwrap(), won_save);

        remove_save(&game_path);
        remove_save(&profile_path);
    }

    #[test]
    fn klondike_safe_finish_is_atomic_reopenable_and_history_safe() {
        let game_path = test_save("klondike-safe-finish");
        let profile_path = test_save("klondike-safe-finish-profile");
        remove_save(&game_path);
        remove_save(&profile_path);
        let envelope: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/klondike-seed-zero-near-win.json"
        ))
        .unwrap();
        let replay: solitaire::replay::Replay<Action, solitaire::klondike::ReplaySetup> =
            serde_json::from_value(envelope["payload"].clone()).unwrap();
        let staged = Game::from_replay(&replay).unwrap();
        solitaire::persistence::save_klondike(&game_path, &staged).unwrap();
        let (near_win, revision) = load_klondike_revisioned(&game_path).unwrap();

        let mut controller = controller(0);
        controller.game = near_win.clone();
        controller.save_path = Some(game_path.clone());
        controller.save_revisions[0] = Some(revision);
        controller.local_profile_path = Some(profile_path.clone());
        controller.selection = Some(Selection::Tableau {
            column: 0,
            count: 1,
        });
        controller.autocomplete();

        assert_eq!(controller.status, "Moved 1 safe card to a foundation");
        assert!(controller.selection.is_none());
        assert!(controller.game.state.is_won());
        assert_eq!(controller.game.state.card_count(), 52);
        assert_eq!(controller.game.state.moves, 156);
        assert!(!controller.dirty[0]);
        assert!(!controller.local_profile_dirty);
        let won = controller.game.clone();
        let won_bytes = fs::read(&game_path).unwrap();
        let profile_bytes = fs::read(&profile_path).unwrap();
        assert_eq!(load_klondike_revisioned(&game_path).unwrap().0, won);
        assert_eq!(
            load_local_profile_revisioned(&profile_path)
                .unwrap()
                .0
                .statistics(ProfileGameKind::Klondike),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(0),
                latest_won_deal: Some(0),
            }
        );
        assert_eq!(
            fs::metadata(&game_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&profile_path).unwrap().permissions().mode() & 0o777,
            0o600
        );

        controller.undo();
        assert_eq!(controller.game.state, near_win.state);
        assert_eq!(controller.game.replay(), near_win.replay());
        assert!(controller.game.can_redo());
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);
        controller.redo();
        assert_eq!(controller.game, won);
        assert_eq!(fs::read(&game_path).unwrap(), won_bytes);
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);

        controller.selection = Some(Selection::Waste);
        controller.autocomplete();
        assert_eq!(controller.status, "Moved 0 safe cards to foundations");
        assert!(controller.selection.is_none());
        assert_eq!(controller.game, won);
        assert_eq!(fs::read(&game_path).unwrap(), won_bytes);
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);

        let (reopened, _) = load_klondike_revisioned(&game_path).unwrap();
        assert_eq!(reopened, won);
        remove_save(&game_path);
        remove_save(&profile_path);
    }

    #[test]
    fn klondike_safe_finish_conflict_preserves_both_owners_until_reload() {
        let path = test_save("klondike-safe-finish-conflict");
        remove_save(&path);
        let envelope: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/klondike-seed-zero-near-win.json"
        ))
        .unwrap();
        let replay: solitaire::replay::Replay<Action, solitaire::klondike::ReplaySetup> =
            serde_json::from_value(envelope["payload"].clone()).unwrap();
        let staged = Game::from_replay(&replay).unwrap();
        solitaire::persistence::save_klondike(&path, &staged).unwrap();
        let (near_win, revision) = load_klondike_revisioned(&path).unwrap();

        let mut controller = controller(0);
        controller.game = near_win;
        controller.save_path = Some(path.clone());
        controller.save_revisions[0] = Some(revision);
        controller.selection = Some(Selection::Waste);

        let external = Game::new(999, Options::default());
        solitaire::persistence::save_klondike(&path, &external).unwrap();
        let external_bytes = fs::read(&path).unwrap();
        controller.autocomplete();

        assert!(controller.game.state.is_won());
        assert!(controller.selection.is_none());
        assert!(controller.dirty[0]);
        assert!(controller.status.contains("save changed in another"));
        assert_eq!(fs::read(&path).unwrap(), external_bytes);

        controller.reload_disk_copy();
        assert_eq!(controller.game, external);
        assert!(!controller.dirty[0]);
        assert!(controller.selection.is_none());
        assert_eq!(fs::read(&path).unwrap(), external_bytes);
        remove_save(&path);
    }

    fn exercise_klondike_restart_child(phase: &str) {
        let mut restarted = Controller::new();
        assert_eq!(
            restarted.game.state.seed, 0,
            "fresh Controller did not reopen the pinned fixture: {}",
            restarted.status
        );
        assert_eq!(restarted.game.state.options, Options::default());
        assert!(restarted.pending_new_deal.is_none());
        assert!(!restarted.dirty[restarted.active_index()]);

        if phase == "complete" {
            assert!(!restarted.game.state.is_won());
            assert_eq!(restarted.game.state.moves, 155);
            assert_eq!(
                restarted
                    .local_profile
                    .statistics(ProfileGameKind::Klondike),
                solitaire::profile::GameStatistics::default()
            );
            restarted.activate_tableau(0, 0);
            restarted.activate_foundation(1);
            assert_eq!(restarted.status, "Deal complete — beautifully played");
            assert!(restarted.game.state.is_won());
            restarted.undo();
            assert!(!restarted.game.state.is_won());
            restarted.redo();
            assert!(restarted.game.state.is_won());
            return;
        }

        assert_eq!(phase, "reopen");
        assert!(restarted.game.state.is_won());
        assert_eq!(restarted.game.state.moves, 156);
        assert_eq!(restarted.game.state.score, 365);
        assert_eq!(
            restarted
                .local_profile
                .statistics(ProfileGameKind::Klondike),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(0),
                latest_won_deal: Some(0),
            }
        );
        restarted.observe_active_profile();
    }

    #[test]
    fn klondike_complete_deal_survives_normal_controller_restart() {
        const PHASE: &str = "SOLITAIRE_KLONDIKE_RESTART_PHASE";
        const ROOT: &str = "SOLITAIRE_KLONDIKE_RESTART_ROOT";
        const TOKEN: &str = "SOLITAIRE_KLONDIKE_RESTART_TOKEN";
        if let Ok(phase) = std::env::var(PHASE) {
            validate_restart_child_root(ROOT, TOKEN);
            exercise_klondike_restart_child(&phase);
            return;
        }

        let restart_root = create_restart_root("klondike");
        let data = restart_root.path().join("solitaire");
        let game_path = data.join("klondike-save.json");
        let profile_path = data.join("local-profile.json");
        let envelope: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/klondike-seed-zero-near-win.json"
        ))
        .unwrap();
        let replay: solitaire::replay::Replay<Action, solitaire::klondike::ReplaySetup> =
            serde_json::from_value(envelope["payload"].clone()).unwrap();
        let staged = Game::from_replay(&replay).unwrap();
        solitaire::persistence::save_klondike(&game_path, &staged).unwrap();

        run_restart_phase(
            restart_root.path(),
            "complete",
            restart_root.token(),
            "tests::klondike_complete_deal_survives_normal_controller_restart",
            [PHASE, ROOT, TOKEN],
        );
        let completed_save = fs::read(&game_path).unwrap();
        let completed_profile = fs::read(&profile_path).unwrap();
        assert!(
            load_klondike_revisioned(&game_path)
                .unwrap()
                .0
                .state
                .is_won()
        );

        run_restart_phase(
            restart_root.path(),
            "reopen",
            restart_root.token(),
            "tests::klondike_complete_deal_survives_normal_controller_restart",
            [PHASE, ROOT, TOKEN],
        );
        assert_eq!(fs::read(&game_path).unwrap(), completed_save);
        assert_eq!(fs::read(&profile_path).unwrap(), completed_profile);
        restart_root.finish();
    }

    #[test]
    fn controller_completes_legal_spider_replay_once_and_reopens() {
        let game_path = test_save("spider-near-win");
        let profile_path = test_save("spider-near-win-profile");
        remove_save(&game_path);
        remove_save(&profile_path);
        fs::write(
            &game_path,
            include_bytes!("../tests/fixtures/spider-one-suit-near-win.json"),
        )
        .unwrap();
        fs::set_permissions(&game_path, fs::Permissions::from_mode(0o600)).unwrap();
        let (near_win, revision) = load_spider_revisioned(&game_path).unwrap();

        let mut controller = controller(3);
        controller.active = GameKind::Spider;
        controller.spider = near_win;
        controller.spider_save_path = Some(game_path.clone());
        controller.save_revisions[1] = Some(revision);
        controller.local_profile_path = Some(profile_path.clone());
        assert!(!profile_path.exists());
        assert_eq!(
            controller.local_profile.statistics(ProfileGameKind::Spider),
            solitaire::profile::GameStatistics::default()
        );

        controller.activate_spider_tableau(0, 0);
        assert!(
            controller
                .spider_selection
                .is_some_and(|selection| { selection.column == 0 && selection.count == 10 })
        );
        controller.activate_spider_tableau(2, 0);

        assert_eq!(
            controller.status,
            "Spider complete — all eight runs are home"
        );
        assert!(controller.spider.state.is_won());
        assert_eq!(controller.spider.state.completed_runs, 8);
        assert_eq!(controller.spider.state.card_count(), 104);
        assert_eq!(controller.spider.state.score, 1_181);
        assert_eq!(controller.spider.state.moves, 119);
        assert_eq!(
            controller.local_profile.statistics(ProfileGameKind::Spider),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(3),
                latest_won_deal: Some(3),
            }
        );
        assert!(!controller.local_profile_dirty);
        assert_eq!(
            fs::metadata(&game_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&profile_path).unwrap().permissions().mode() & 0o777,
            0o600
        );

        let won = controller.spider.clone();
        let won_save = fs::read(&game_path).unwrap();
        let envelope: serde_json::Value = serde_json::from_slice(&won_save).unwrap();
        assert_eq!(envelope["version"], 1);
        assert_eq!(envelope["game"], "spider");
        assert_eq!(envelope["payload"]["version"], 2);
        assert_eq!(envelope["payload"]["setup"], "One");
        assert_eq!(
            envelope["payload"]["actions"].as_array().unwrap().len(),
            119
        );
        assert_eq!(load_spider_revisioned(&game_path).unwrap().0, won);

        let profile_bytes = fs::read(&profile_path).unwrap();
        controller.undo();
        assert_eq!(controller.status, "Move undone");
        assert_eq!(controller.spider.state.completed_runs, 7);
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);
        controller.redo();
        assert_eq!(controller.status, "Move restored");
        assert_eq!(controller.spider, won);
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);
        controller.observe_active_profile();
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);

        let (reopened_game, _) = load_spider_revisioned(&game_path).unwrap();
        let (reopened_profile, _) = load_local_profile_revisioned(&profile_path).unwrap();
        assert_eq!(reopened_game, won);
        assert_eq!(
            reopened_profile.statistics(ProfileGameKind::Spider),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(3),
                latest_won_deal: Some(3),
            }
        );
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);
        assert_eq!(fs::read(&game_path).unwrap(), won_save);
        assert_eq!(load_spider_revisioned(&game_path).unwrap().0, won);

        remove_save(&game_path);
        remove_save(&profile_path);
    }

    fn exercise_spider_restart_child(phase: &str) {
        let mut restarted = Controller::new();
        restarted.select_game("Spider");
        assert_eq!(restarted.spider.state.seed, 3);
        assert_eq!(restarted.spider.state.mode, SuitMode::One);
        assert_eq!(spider_ui_mode_for_render(&restarted), Some((0, "1 suit")));
        assert!(restarted.pending_new_deal.is_none());
        assert!(!restarted.dirty[restarted.active_index()]);

        if phase == "complete" {
            assert_eq!(restarted.spider.state.completed_runs, 7);
            assert!(!restarted.spider.state.is_won());
            assert_eq!(
                restarted.local_profile.statistics(ProfileGameKind::Spider),
                solitaire::profile::GameStatistics::default()
            );
            restarted.activate_spider_tableau(0, 0);
            restarted.activate_spider_tableau(2, 0);
            assert_eq!(
                restarted.status,
                "Spider complete — all eight runs are home"
            );
            assert!(restarted.spider.state.is_won());
            restarted.undo();
            assert_eq!(restarted.spider.state.completed_runs, 7);
            restarted.redo();
            assert!(restarted.spider.state.is_won());
            return;
        }

        assert_eq!(phase, "reopen");
        assert!(restarted.spider.state.is_won());
        assert_eq!(restarted.spider.state.completed_runs, 8);
        assert_eq!(restarted.spider.state.score, 1_181);
        assert_eq!(restarted.spider.state.moves, 119);
        assert_eq!(
            restarted.local_profile.statistics(ProfileGameKind::Spider),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(3),
                latest_won_deal: Some(3),
            }
        );
        restarted.observe_active_profile();
    }

    fn bounded_child_text(bytes: &[u8]) -> String {
        const DIAGNOSTIC_LIMIT: usize = 8 * 1_024;
        String::from_utf8_lossy(&bytes[..bytes.len().min(DIAGNOSTIC_LIMIT)]).into_owned()
    }

    fn validate_restart_child_root(root_variable: &str, token_variable: &str) -> PathBuf {
        let isolated_root = PathBuf::from(std::env::var_os(root_variable).unwrap());
        let token = std::env::var(token_variable).unwrap();
        assert_eq!(
            std::env::var_os("XDG_DATA_HOME"),
            Some(isolated_root.clone().into())
        );
        assert!(isolated_root.is_absolute());
        assert!(
            fs::canonicalize(&isolated_root)
                .unwrap()
                .starts_with(fs::canonicalize(std::env::temp_dir()).unwrap())
        );
        let marker = isolated_root.join(".restart-token");
        let metadata = fs::symlink_metadata(&marker).unwrap();
        assert!(metadata.file_type().is_file());
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        assert_eq!(fs::read_to_string(marker).unwrap(), token);
        isolated_root
    }

    struct RestartRoot {
        path: PathBuf,
        token: String,
        marker_ready: bool,
        cleaned: bool,
    }

    impl RestartRoot {
        fn path(&self) -> &Path {
            &self.path
        }

        fn token(&self) -> &str {
            &self.token
        }

        fn owns_path(&self) -> bool {
            let marker = self.path.join(".restart-token");
            fs::symlink_metadata(&marker).is_ok_and(|metadata| {
                metadata.file_type().is_file()
                    && metadata.permissions().mode() & 0o777 == 0o600
                    && fs::read_to_string(marker).is_ok_and(|token| token == self.token)
            })
        }

        fn finish(mut self) {
            assert!(self.owns_path(), "restart root ownership marker changed");
            fs::remove_dir_all(&self.path).unwrap_or_else(|error| {
                panic!(
                    "failed to remove restart root {}: {error}",
                    self.path.display()
                )
            });
            self.cleaned = true;
        }
    }

    impl Drop for RestartRoot {
        fn drop(&mut self) {
            if self.cleaned || (self.marker_ready && !self.owns_path()) {
                return;
            }
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn create_restart_root(game: &str) -> RestartRoot {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let token = format!("{}-{nonce}", std::process::id());
        let root = std::env::temp_dir().join(format!(
            "solitaire-controller-{}-{nonce}-{game}-restart",
            std::process::id(),
        ));
        fs::create_dir(&root).unwrap();
        let mut restart_root = RestartRoot {
            path: root,
            token,
            marker_ready: false,
            cleaned: false,
        };
        fs::create_dir(restart_root.path.join("solitaire")).unwrap();
        let marker = restart_root.path.join(".restart-token");
        fs::write(&marker, &restart_root.token).unwrap();
        fs::set_permissions(&marker, fs::Permissions::from_mode(0o600)).unwrap();
        restart_root.marker_ready = true;
        restart_root
    }

    #[test]
    fn restart_root_guard_removes_unexpected_residue_on_unwind() {
        let restart_root = create_restart_root("cleanup");
        let root = restart_root.path().to_path_buf();
        let residue = root
            .join("solitaire")
            .join("klondike-save.json.corrupt-test");
        let result = std::panic::catch_unwind(move || {
            let _restart_root = restart_root;
            fs::write(residue, b"preserved test residue").unwrap();
            panic!("exercise restart-root unwind cleanup");
        });
        assert!(result.is_err());
        assert!(!root.exists());
    }

    fn run_restart_phase(
        root: &Path,
        phase: &str,
        token: &str,
        test_name: &str,
        environment: [&str; 3],
    ) {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([test_name, "--exact", "--nocapture"])
            .env("XDG_DATA_HOME", root)
            .env(environment[0], phase)
            .env(environment[1], root)
            .env(environment[2], token)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if child.try_wait().unwrap().is_some() {
                let output = child.wait_with_output().unwrap();
                assert!(
                    output.status.success(),
                    "{phase} child stdout: {}\n{phase} child stderr: {}",
                    bounded_child_text(&output.stdout),
                    bounded_child_text(&output.stderr)
                );
                return;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let output = child.wait_with_output().unwrap();
                panic!(
                    "{phase} child exceeded 10 seconds; stdout: {}\nstderr: {}",
                    bounded_child_text(&output.stdout),
                    bounded_child_text(&output.stderr)
                );
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn spider_complete_deal_survives_normal_controller_restart() {
        const CHILD_PHASE: &str = "SOLITAIRE_SPIDER_RESTART_PHASE";
        const CHILD_ROOT: &str = "SOLITAIRE_SPIDER_RESTART_ROOT";
        const CHILD_TOKEN: &str = "SOLITAIRE_SPIDER_RESTART_TOKEN";
        if let Ok(phase) = std::env::var(CHILD_PHASE) {
            validate_restart_child_root(CHILD_ROOT, CHILD_TOKEN);
            exercise_spider_restart_child(&phase);
            return;
        }

        let restart_root = create_restart_root("spider");
        let data = restart_root.path().join("solitaire");
        let game_path = data.join("spider-save.json");
        let profile_path = data.join("local-profile.json");
        fs::write(
            &game_path,
            include_bytes!("../tests/fixtures/spider-one-suit-near-win.json"),
        )
        .unwrap();
        fs::set_permissions(&game_path, fs::Permissions::from_mode(0o600)).unwrap();

        run_restart_phase(
            restart_root.path(),
            "complete",
            restart_root.token(),
            "tests::spider_complete_deal_survives_normal_controller_restart",
            [CHILD_PHASE, CHILD_ROOT, CHILD_TOKEN],
        );
        let completed_save = fs::read(&game_path).unwrap();
        let completed_profile = fs::read(&profile_path).unwrap();
        assert!(load_spider_revisioned(&game_path).unwrap().0.state.is_won());
        assert_eq!(
            load_local_profile_revisioned(&profile_path)
                .unwrap()
                .0
                .statistics(ProfileGameKind::Spider),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(3),
                latest_won_deal: Some(3),
            }
        );

        run_restart_phase(
            restart_root.path(),
            "reopen",
            restart_root.token(),
            "tests::spider_complete_deal_survives_normal_controller_restart",
            [CHILD_PHASE, CHILD_ROOT, CHILD_TOKEN],
        );
        assert_eq!(fs::read(&game_path).unwrap(), completed_save);
        assert_eq!(fs::read(&profile_path).unwrap(), completed_profile);
        for path in [&game_path, &profile_path] {
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }

        restart_root.finish();
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
    fn controller_completes_legal_freecell_replay_once_and_reopens() {
        let game_path = test_save("freecell-near-win");
        let profile_path = test_save("freecell-near-win-profile");
        remove_save(&game_path);
        remove_save(&profile_path);
        fs::write(
            &game_path,
            include_bytes!("../tests/fixtures/freecell-seed-zero-near-win.json"),
        )
        .unwrap();
        fs::set_permissions(&game_path, fs::Permissions::from_mode(0o600)).unwrap();
        let (near_win, revision) = load_freecell_revisioned(&game_path).unwrap();

        let mut controller = controller(0);
        controller.active = GameKind::FreeCell;
        controller.freecell = near_win;
        controller.freecell_save_path = Some(game_path.clone());
        controller.save_revisions[2] = Some(revision);
        controller.local_profile_path = Some(profile_path.clone());
        assert!(!profile_path.exists());
        assert_eq!(
            controller
                .local_profile
                .statistics(ProfileGameKind::FreeCell),
            solitaire::profile::GameStatistics::default()
        );

        controller.activate_freecell_cell(1);
        assert!(
            controller
                .freecell_selection
                .is_some_and(|selection| selection.pile == freecell::Pile::FreeCell(1))
        );
        controller.activate_freecell_foundation(3);

        assert_eq!(controller.status, "FreeCell complete — every suit is home");
        assert!(controller.freecell.state.is_won());
        assert_eq!(controller.freecell.state.card_count(), 52);
        assert_eq!(controller.freecell.state.moves, 106);
        assert_eq!(
            controller
                .local_profile
                .statistics(ProfileGameKind::FreeCell),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(0),
                latest_won_deal: Some(0),
            }
        );
        assert!(!controller.local_profile_dirty);
        assert_eq!(
            fs::metadata(&game_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&profile_path).unwrap().permissions().mode() & 0o777,
            0o600
        );

        let won = controller.freecell.clone();
        let won_save = fs::read(&game_path).unwrap();
        let envelope: serde_json::Value = serde_json::from_slice(&won_save).unwrap();
        assert_eq!(envelope["version"], 1);
        assert_eq!(envelope["game"], "freecell");
        assert_eq!(envelope["payload"]["version"], 2);
        assert_eq!(envelope["payload"]["seed"], 0);
        assert_eq!(
            envelope["payload"]["actions"].as_array().unwrap().len(),
            106
        );
        assert_eq!(load_freecell_revisioned(&game_path).unwrap().0, won);

        let profile_bytes = fs::read(&profile_path).unwrap();
        controller.undo();
        assert_eq!(controller.status, "Move undone");
        assert!(!controller.freecell.state.is_won());
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);
        controller.redo();
        assert_eq!(controller.status, "Move restored");
        assert_eq!(controller.freecell, won);
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);
        controller.observe_active_profile();
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);

        let (reopened_game, _) = load_freecell_revisioned(&game_path).unwrap();
        let (reopened_profile, _) = load_local_profile_revisioned(&profile_path).unwrap();
        assert_eq!(reopened_game, won);
        assert_eq!(
            reopened_profile.statistics(ProfileGameKind::FreeCell),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(0),
                latest_won_deal: Some(0),
            }
        );
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);
        assert_eq!(fs::read(&game_path).unwrap(), won_save);

        remove_save(&game_path);
        remove_save(&profile_path);
    }

    fn exercise_freecell_restart_child(phase: &str) {
        let mut restarted = Controller::new();
        assert_eq!(restarted.freecell.state.deal_number, 0);
        assert!(restarted.pending_new_deal.is_none());
        restarted.select_game("FreeCell");
        assert!(!restarted.dirty[restarted.active_index()]);

        if phase == "complete" {
            assert!(!restarted.freecell.state.is_won());
            assert_eq!(restarted.freecell.state.moves, 105);
            assert_eq!(
                restarted
                    .local_profile
                    .statistics(ProfileGameKind::FreeCell),
                solitaire::profile::GameStatistics::default()
            );
            restarted.activate_freecell_cell(1);
            restarted.activate_freecell_foundation(3);
            assert_eq!(restarted.status, "FreeCell complete — every suit is home");
            assert!(restarted.freecell.state.is_won());
            restarted.undo();
            assert!(!restarted.freecell.state.is_won());
            restarted.redo();
            assert!(restarted.freecell.state.is_won());
            return;
        }

        assert_eq!(phase, "reopen");
        assert!(restarted.freecell.state.is_won());
        assert_eq!(restarted.freecell.state.moves, 106);
        assert_eq!(
            restarted
                .local_profile
                .statistics(ProfileGameKind::FreeCell),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(0),
                latest_won_deal: Some(0),
            }
        );
        restarted.observe_active_profile();
    }

    #[test]
    fn freecell_complete_deal_survives_normal_controller_restart() {
        const PHASE: &str = "SOLITAIRE_FREECELL_COMPLETE_RESTART_PHASE";
        const ROOT: &str = "SOLITAIRE_FREECELL_COMPLETE_RESTART_ROOT";
        const TOKEN: &str = "SOLITAIRE_FREECELL_COMPLETE_RESTART_TOKEN";
        if let Ok(phase) = std::env::var(PHASE) {
            validate_restart_child_root(ROOT, TOKEN);
            exercise_freecell_restart_child(&phase);
            return;
        }

        let restart_root = create_restart_root("freecell-complete");
        let data = restart_root.path().join("solitaire");
        let game_path = data.join("freecell-save.json");
        let profile_path = data.join("local-profile.json");
        fs::write(
            &game_path,
            include_bytes!("../tests/fixtures/freecell-seed-zero-near-win.json"),
        )
        .unwrap();
        fs::set_permissions(&game_path, fs::Permissions::from_mode(0o600)).unwrap();

        run_restart_phase(
            restart_root.path(),
            "complete",
            restart_root.token(),
            "tests::freecell_complete_deal_survives_normal_controller_restart",
            [PHASE, ROOT, TOKEN],
        );
        let completed_save = fs::read(&game_path).unwrap();
        let completed_profile = fs::read(&profile_path).unwrap();
        assert!(
            load_freecell_revisioned(&game_path)
                .unwrap()
                .0
                .state
                .is_won()
        );
        assert_eq!(
            load_local_profile_revisioned(&profile_path)
                .unwrap()
                .0
                .statistics(ProfileGameKind::FreeCell),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(0),
                latest_won_deal: Some(0),
            }
        );

        run_restart_phase(
            restart_root.path(),
            "reopen",
            restart_root.token(),
            "tests::freecell_complete_deal_survives_normal_controller_restart",
            [PHASE, ROOT, TOKEN],
        );
        assert_eq!(fs::read(&game_path).unwrap(), completed_save);
        assert_eq!(fs::read(&profile_path).unwrap(), completed_profile);
        for path in [&game_path, &profile_path] {
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        restart_root.finish();
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
        assert_eq!(
            tripeaks_ui_rule_for_render(&controller),
            Some((0, "Standard"))
        );

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
    fn tripeaks_wraparound_is_strict_atomic_reopenable_and_history_safe() {
        let game_path = test_save("tripeaks-wraparound-game");
        let counter_path = test_save("tripeaks-wraparound-counter");
        remove_save(&game_path);
        remove_save(&counter_path);
        let mut controller = controller(5);
        controller.select_game("TriPeaks");
        controller.tripeaks_save_path = Some(game_path.clone());
        controller.deal_counters_path = Some(counter_path.clone());
        assert!(controller.save());

        controller.new_game("Ace-King wrap");
        assert!(
            controller.pending_new_deal.is_none(),
            "{}",
            controller.status
        );
        assert!(controller.tripeaks.state.options.wraparound);
        assert_eq!(controller.tripeaks.state.seed, 6);
        assert_eq!(
            tripeaks_ui_rule_for_render(&controller),
            Some((1, "Ace-King wrap"))
        );
        assert_eq!(
            load_tripeaks_revisioned(&game_path).unwrap().0,
            controller.tripeaks
        );
        assert_eq!(load_deal_counters(&counter_path).unwrap().tripeaks, 7);

        let waste_rank = controller.tripeaks.state.waste.last().unwrap().rank;
        let boundary_rank = controller.tripeaks.state.tableau[21].unwrap().rank;
        assert!(matches!(
            (waste_rank, boundary_rank),
            (Rank::Ace, Rank::King) | (Rank::King, Rank::Ace)
        ));
        controller.activate_tripeaks_card(21);
        assert!(controller.tripeaks.state.tableau[21].is_none());
        assert_eq!(controller.tripeaks.state.moves, 1);
        let moved = controller.tripeaks.clone();
        controller.undo();
        assert!(controller.tripeaks.can_redo());
        controller.redo();
        assert_eq!(controller.tripeaks, moved);
        assert!(controller.tripeaks.state.options.wraparound);
        assert_eq!(load_tripeaks_revisioned(&game_path).unwrap().0, moved);
        assert_eq!(
            fs::metadata(&game_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&counter_path).unwrap().permissions().mode() & 0o777,
            0o600
        );

        controller.dirty[3] = true;
        controller.new_game("Standard");
        let pending = controller.pending_new_deal;
        assert!(pending.is_some());
        assert_eq!(tripeaks_ui_rule_for_render(&controller), None);
        let preserved_game = controller.tripeaks.clone();
        let preserved_save = fs::read(&game_path).unwrap();
        let preserved_counters = fs::read(&counter_path).unwrap();
        let oversized = "W".repeat(4_096);
        for invalid in [
            "",
            "standard",
            " Standard",
            "Standard ",
            "Ace King wrap",
            "Ace-King wrap ",
            "Ace-King wrap · trailing",
            oversized.as_str(),
        ] {
            controller.new_game(invalid);
            assert_eq!(controller.tripeaks, preserved_game, "{invalid:?}");
            assert!(controller.pending_new_deal == pending, "{invalid:?}");
            assert_eq!(fs::read(&game_path).unwrap(), preserved_save, "{invalid:?}");
            assert_eq!(
                fs::read(&counter_path).unwrap(),
                preserved_counters,
                "{invalid:?}"
            );
            assert_eq!(
                controller.status,
                "Invalid new-deal options; current game preserved"
            );
        }

        remove_save(&game_path);
        remove_save(&counter_path);
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
    fn controller_completes_legal_tripeaks_replay_once_and_reopens() {
        let game_path = test_save("tripeaks-near-win");
        let profile_path = test_save("tripeaks-near-win-profile");
        remove_save(&game_path);
        remove_save(&profile_path);
        fs::write(
            &game_path,
            include_bytes!("../tests/fixtures/tripeaks-seed-zero-near-win.json"),
        )
        .unwrap();
        fs::set_permissions(&game_path, fs::Permissions::from_mode(0o600)).unwrap();
        let (near_win, revision) = load_tripeaks_revisioned(&game_path).unwrap();

        let mut controller = controller(0);
        controller.active = GameKind::TriPeaks;
        controller.tripeaks = near_win;
        controller.tripeaks_save_path = Some(game_path.clone());
        controller.save_revisions[3] = Some(revision);
        controller.local_profile_path = Some(profile_path.clone());
        assert!(!profile_path.exists());
        assert_eq!(
            controller
                .local_profile
                .statistics(ProfileGameKind::TriPeaks),
            solitaire::profile::GameStatistics::default()
        );

        controller.activate_tripeaks_card(0);

        assert_eq!(
            controller.status,
            "TriPeaks complete — all three peaks are clear"
        );
        assert!(controller.tripeaks.state.is_won());
        assert_eq!(controller.tripeaks.state.card_count(), 52);
        assert_eq!(controller.tripeaks.state.score, 5_800);
        assert_eq!(controller.tripeaks.state.moves, 49);
        assert_eq!(
            controller
                .local_profile
                .statistics(ProfileGameKind::TriPeaks),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(0),
                latest_won_deal: Some(0),
            }
        );
        assert!(!controller.local_profile_dirty);
        assert_eq!(
            fs::metadata(&game_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&profile_path).unwrap().permissions().mode() & 0o777,
            0o600
        );

        let won = controller.tripeaks.clone();
        let won_save = fs::read(&game_path).unwrap();
        let envelope: serde_json::Value = serde_json::from_slice(&won_save).unwrap();
        assert_eq!(envelope["version"], 1);
        assert_eq!(envelope["game"], "tripeaks");
        assert_eq!(envelope["payload"]["version"], 2);
        assert_eq!(envelope["payload"]["setup"]["wraparound"], false);
        assert_eq!(envelope["payload"]["actions"].as_array().unwrap().len(), 49);
        assert_eq!(load_tripeaks_revisioned(&game_path).unwrap().0, won);

        let profile_bytes = fs::read(&profile_path).unwrap();
        controller.undo();
        assert_eq!(controller.status, "Move undone");
        assert!(!controller.tripeaks.state.is_won());
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);
        controller.redo();
        assert_eq!(controller.status, "Move restored");
        assert_eq!(controller.tripeaks, won);
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);
        controller.observe_active_profile();
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);

        let (reopened_game, _) = load_tripeaks_revisioned(&game_path).unwrap();
        let (reopened_profile, _) = load_local_profile_revisioned(&profile_path).unwrap();
        assert_eq!(reopened_game, won);
        assert_eq!(
            reopened_profile.statistics(ProfileGameKind::TriPeaks),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(0),
                latest_won_deal: Some(0),
            }
        );

        remove_save(&game_path);
        remove_save(&profile_path);
    }

    fn exercise_tripeaks_restart_child(phase: &str) {
        let mut restarted = Controller::new();
        assert_eq!(restarted.tripeaks.state.seed, 0);
        assert!(restarted.pending_new_deal.is_none());
        restarted.select_game("TriPeaks");
        assert!(!restarted.dirty[restarted.active_index()]);

        if phase == "complete" {
            assert!(!restarted.tripeaks.state.is_won());
            assert_eq!(restarted.tripeaks.state.moves, 48);
            assert_eq!(restarted.tripeaks.state.score, 5_700);
            assert_eq!(
                restarted
                    .local_profile
                    .statistics(ProfileGameKind::TriPeaks),
                solitaire::profile::GameStatistics::default()
            );
            restarted.activate_tripeaks_card(0);
            assert_eq!(
                restarted.status,
                "TriPeaks complete — all three peaks are clear"
            );
            assert!(restarted.tripeaks.state.is_won());
            restarted.undo();
            assert!(!restarted.tripeaks.state.is_won());
            restarted.redo();
            assert!(restarted.tripeaks.state.is_won());
            return;
        }

        assert_eq!(phase, "reopen");
        assert!(restarted.tripeaks.state.is_won());
        assert_eq!(restarted.tripeaks.state.moves, 49);
        assert_eq!(restarted.tripeaks.state.score, 5_800);
        assert_eq!(
            restarted
                .local_profile
                .statistics(ProfileGameKind::TriPeaks),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(0),
                latest_won_deal: Some(0),
            }
        );
        restarted.observe_active_profile();
    }

    #[test]
    fn tripeaks_complete_deal_survives_normal_controller_restart() {
        const PHASE: &str = "SOLITAIRE_TRIPEAKS_COMPLETE_RESTART_PHASE";
        const ROOT: &str = "SOLITAIRE_TRIPEAKS_COMPLETE_RESTART_ROOT";
        const TOKEN: &str = "SOLITAIRE_TRIPEAKS_COMPLETE_RESTART_TOKEN";
        if let Ok(phase) = std::env::var(PHASE) {
            validate_restart_child_root(ROOT, TOKEN);
            exercise_tripeaks_restart_child(&phase);
            return;
        }

        let restart_root = create_restart_root("tripeaks-complete");
        let data = restart_root.path().join("solitaire");
        let game_path = data.join("tripeaks-save.json");
        let profile_path = data.join("local-profile.json");
        fs::write(
            &game_path,
            include_bytes!("../tests/fixtures/tripeaks-seed-zero-near-win.json"),
        )
        .unwrap();
        fs::set_permissions(&game_path, fs::Permissions::from_mode(0o600)).unwrap();

        run_restart_phase(
            restart_root.path(),
            "complete",
            restart_root.token(),
            "tests::tripeaks_complete_deal_survives_normal_controller_restart",
            [PHASE, ROOT, TOKEN],
        );
        let completed_save = fs::read(&game_path).unwrap();
        let completed_profile = fs::read(&profile_path).unwrap();
        let completed = load_tripeaks_revisioned(&game_path).unwrap().0;
        assert!(completed.state.is_won());
        assert_eq!(completed.state.moves, 49);
        assert_eq!(completed.state.score, 5_800);
        assert_eq!(
            load_local_profile_revisioned(&profile_path)
                .unwrap()
                .0
                .statistics(ProfileGameKind::TriPeaks),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(0),
                latest_won_deal: Some(0),
            }
        );

        run_restart_phase(
            restart_root.path(),
            "reopen",
            restart_root.token(),
            "tests::tripeaks_complete_deal_survives_normal_controller_restart",
            [PHASE, ROOT, TOKEN],
        );
        assert_eq!(fs::read(&game_path).unwrap(), completed_save);
        assert_eq!(fs::read(&profile_path).unwrap(), completed_profile);
        for path in [&game_path, &profile_path] {
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        restart_root.finish();
    }

    #[test]
    fn pyramid_surface_routes_standard_play_history_and_reopen() {
        let path = test_save("pyramid-surface");
        remove_save(&path);
        let mut controller = controller(85);
        controller.select_game("Pyramid");
        controller.pyramid_save_path = Some(path.clone());
        controller.new_game("2 redeals");
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
    fn pyramid_redeal_limits_are_strict_atomic_reopenable_and_enforced() {
        for (label, maximum) in [("No redeals", 0), ("1 redeal", 1), ("2 redeals", 2)] {
            let path = test_save(&format!("pyramid-redeal-{maximum}"));
            remove_save(&path);
            let mut controller = controller(90 + u64::from(maximum));
            controller.select_game("Pyramid");
            controller.pyramid_save_path = Some(path.clone());
            controller.new_game(label);
            assert_eq!(controller.pyramid.state.options.max_redeals, maximum);
            assert_eq!(
                load_pyramid_revisioned(&path).unwrap().0,
                controller.pyramid
            );
            remove_save(&path);
        }

        let game_path = test_save("pyramid-redeal-enforced");
        let counter_path = test_save("pyramid-redeal-counter");
        remove_save(&game_path);
        remove_save(&counter_path);
        let mut controller = controller(94);
        controller.select_game("Pyramid");
        controller.pyramid_save_path = Some(game_path.clone());
        controller.deal_counters_path = Some(counter_path.clone());
        controller.new_game("1 redeal");
        drain_pyramid_stock(&mut controller);
        controller.draw_pyramid_stock();
        assert_eq!(controller.pyramid.state.redeals, 1);
        drain_pyramid_stock(&mut controller);
        let exhausted = controller.pyramid.clone();
        let saved = fs::read(&game_path).unwrap();
        let counters = fs::read(&counter_path).unwrap();
        controller.draw_pyramid_stock();
        assert_eq!(controller.status, "No Pyramid redeals remain");
        assert_eq!(controller.pyramid, exhausted);
        assert_eq!(fs::read(&game_path).unwrap(), saved);
        assert_eq!(fs::read(&counter_path).unwrap(), counters);
        controller.undo();
        assert!(controller.pyramid.can_redo());
        controller.redo();
        assert_eq!(controller.pyramid, exhausted);
        assert_eq!(load_pyramid_revisioned(&game_path).unwrap().0, exhausted);

        controller.dirty[controller.active_index()] = true;
        controller.new_game("No redeals");
        let pending = controller.pending_new_deal;
        let oversized = "R".repeat(4_096);
        for invalid in [
            "",
            "no redeals",
            " No redeals",
            "No redeals ",
            "0 redeals",
            "3 redeals",
            "1 redeal · trailing",
            oversized.as_str(),
        ] {
            controller.new_game(invalid);
            assert!(controller.pending_new_deal == pending, "{invalid:?}");
            assert_eq!(controller.pyramid, exhausted, "{invalid:?}");
            assert_eq!(fs::read(&game_path).unwrap(), saved, "{invalid:?}");
            assert_eq!(fs::read(&counter_path).unwrap(), counters, "{invalid:?}");
        }
        assert_eq!(pyramid_ui_options_for_render(&controller), None);
        assert_eq!(
            fs::metadata(&game_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&counter_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        controller.discard_progress_and_start_pending();
        assert_eq!(controller.pyramid.state.options.max_redeals, 0);
        assert!(controller.pending_new_deal.is_none());
        assert_eq!(
            load_pyramid_revisioned(&game_path).unwrap().0,
            controller.pyramid
        );
        remove_save(&game_path);
        remove_save(&counter_path);
    }

    #[test]
    fn reopened_pyramid_options_map_values_and_indices_without_a_display() {
        for (maximum, index, label) in [
            (0, 0, "No redeals"),
            (1, 1, "1 redeal"),
            (2, 2, "2 redeals"),
            (u8::MAX, -1, "255 redeals"),
        ] {
            assert_eq!(
                pyramid_ui_options(pyramid::Options {
                    max_redeals: maximum,
                }),
                PyramidUiOptions {
                    redeal_index: index,
                    redeal_limit: label.into(),
                }
            );
        }
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
        controller.new_game("2 redeals");
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
    fn controller_completes_legal_pyramid_replay_once_and_reopens() {
        let game_path = test_save("pyramid-near-win");
        let profile_path = test_save("pyramid-near-win-profile");
        remove_save(&game_path);
        remove_save(&profile_path);
        fs::write(
            &game_path,
            include_bytes!("../tests/fixtures/pyramid-seed-zero-near-win.json"),
        )
        .unwrap();
        fs::set_permissions(&game_path, fs::Permissions::from_mode(0o600)).unwrap();
        let (near_win, revision) = load_pyramid_revisioned(&game_path).unwrap();

        let mut controller = controller(0);
        controller.active = GameKind::Pyramid;
        controller.pyramid = near_win;
        controller.pyramid_save_path = Some(game_path.clone());
        controller.save_revisions[4] = Some(revision);
        controller.local_profile_path = Some(profile_path.clone());
        assert!(!profile_path.exists());
        assert_eq!(
            controller
                .local_profile
                .statistics(ProfileGameKind::Pyramid),
            solitaire::profile::GameStatistics::default()
        );

        controller.activate_pyramid_card(0);
        assert_eq!(
            controller.pyramid_selection,
            Some(pyramid::Source::Pyramid(0))
        );
        controller.activate_pyramid_waste();

        assert_eq!(
            controller.status,
            "Pyramid complete — every tableau card is clear"
        );
        assert!(controller.pyramid.state.is_won());
        assert_eq!(controller.pyramid.state.card_count(), 10);
        assert_eq!(controller.pyramid.state.score, 420);
        assert_eq!(controller.pyramid.state.moves, 63);
        assert_eq!(
            controller
                .local_profile
                .statistics(ProfileGameKind::Pyramid),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(0),
                latest_won_deal: Some(0),
            }
        );
        assert!(!controller.local_profile_dirty);
        assert_eq!(
            fs::metadata(&game_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&profile_path).unwrap().permissions().mode() & 0o777,
            0o600
        );

        let won = controller.pyramid.clone();
        let won_save = fs::read(&game_path).unwrap();
        let envelope: serde_json::Value = serde_json::from_slice(&won_save).unwrap();
        assert_eq!(envelope["version"], 1);
        assert_eq!(envelope["game"], "pyramid");
        assert_eq!(envelope["payload"]["version"], 2);
        assert_eq!(envelope["payload"]["setup"]["max_redeals"], 2);
        assert_eq!(envelope["payload"]["actions"].as_array().unwrap().len(), 63);
        assert_eq!(load_pyramid_revisioned(&game_path).unwrap().0, won);

        let profile_bytes = fs::read(&profile_path).unwrap();
        controller.undo();
        assert_eq!(controller.status, "Move undone");
        assert!(!controller.pyramid.state.is_won());
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);
        controller.redo();
        assert_eq!(controller.status, "Move restored");
        assert_eq!(controller.pyramid, won);
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);
        controller.observe_active_profile();
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);

        let (reopened_game, _) = load_pyramid_revisioned(&game_path).unwrap();
        let (reopened_profile, _) = load_local_profile_revisioned(&profile_path).unwrap();
        assert_eq!(reopened_game, won);
        assert_eq!(
            reopened_profile.statistics(ProfileGameKind::Pyramid),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(0),
                latest_won_deal: Some(0),
            }
        );

        remove_save(&game_path);
        remove_save(&profile_path);
    }

    fn exercise_pyramid_restart_child(phase: &str) {
        let mut restarted = Controller::new();
        assert_eq!(restarted.pyramid.state.seed, 0);
        assert!(restarted.pending_new_deal.is_none());
        assert!(restarted.pyramid_selection.is_none());
        restarted.select_game("Pyramid");
        assert!(!restarted.dirty[restarted.active_index()]);

        if phase == "complete" {
            assert!(!restarted.pyramid.state.is_won());
            assert_eq!(restarted.pyramid.state.moves, 62);
            assert_eq!(restarted.pyramid.state.score, 400);
            assert_eq!(
                restarted.local_profile.statistics(ProfileGameKind::Pyramid),
                solitaire::profile::GameStatistics::default()
            );
            restarted.activate_pyramid_card(0);
            assert_eq!(
                restarted.pyramid_selection,
                Some(pyramid::Source::Pyramid(0))
            );
            restarted.activate_pyramid_waste();
            assert_eq!(
                restarted.status,
                "Pyramid complete — every tableau card is clear"
            );
            assert!(restarted.pyramid.state.is_won());
            restarted.undo();
            assert!(!restarted.pyramid.state.is_won());
            restarted.redo();
            assert!(restarted.pyramid.state.is_won());
            return;
        }

        assert_eq!(phase, "reopen");
        assert!(restarted.pyramid.state.is_won());
        assert_eq!(restarted.pyramid.state.card_count(), 10);
        assert_eq!(restarted.pyramid.state.moves, 63);
        assert_eq!(restarted.pyramid.state.score, 420);
        assert_eq!(
            restarted.local_profile.statistics(ProfileGameKind::Pyramid),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(0),
                latest_won_deal: Some(0),
            }
        );
        restarted.observe_active_profile();
    }

    #[test]
    fn pyramid_complete_deal_survives_normal_controller_restart() {
        const PHASE: &str = "SOLITAIRE_PYRAMID_COMPLETE_RESTART_PHASE";
        const ROOT: &str = "SOLITAIRE_PYRAMID_COMPLETE_RESTART_ROOT";
        const TOKEN: &str = "SOLITAIRE_PYRAMID_COMPLETE_RESTART_TOKEN";
        if let Ok(phase) = std::env::var(PHASE) {
            validate_restart_child_root(ROOT, TOKEN);
            exercise_pyramid_restart_child(&phase);
            return;
        }

        let restart_root = create_restart_root("pyramid-complete");
        let data = restart_root.path().join("solitaire");
        let game_path = data.join("pyramid-save.json");
        let profile_path = data.join("local-profile.json");
        fs::write(
            &game_path,
            include_bytes!("../tests/fixtures/pyramid-seed-zero-near-win.json"),
        )
        .unwrap();
        fs::set_permissions(&game_path, fs::Permissions::from_mode(0o600)).unwrap();

        run_restart_phase(
            restart_root.path(),
            "complete",
            restart_root.token(),
            "tests::pyramid_complete_deal_survives_normal_controller_restart",
            [PHASE, ROOT, TOKEN],
        );
        let completed_save = fs::read(&game_path).unwrap();
        let completed_profile = fs::read(&profile_path).unwrap();
        let completed = load_pyramid_revisioned(&game_path).unwrap().0;
        assert!(completed.state.is_won());
        assert_eq!(completed.state.card_count(), 10);
        assert_eq!(completed.state.moves, 63);
        assert_eq!(completed.state.score, 420);
        assert_eq!(
            load_local_profile_revisioned(&profile_path)
                .unwrap()
                .0
                .statistics(ProfileGameKind::Pyramid),
            solitaire::profile::GameStatistics {
                deals_played: 1,
                deals_won: 1,
                latest_played_deal: Some(0),
                latest_won_deal: Some(0),
            }
        );

        run_restart_phase(
            restart_root.path(),
            "reopen",
            restart_root.token(),
            "tests::pyramid_complete_deal_survives_normal_controller_restart",
            [PHASE, ROOT, TOKEN],
        );
        assert_eq!(fs::read(&game_path).unwrap(), completed_save);
        assert_eq!(fs::read(&profile_path).unwrap(), completed_profile);
        for path in [&game_path, &profile_path] {
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        restart_root.finish();
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

    struct RestartTestCase {
        controller: Controller,
        game_path: PathBuf,
        counter_path: PathBuf,
        profile_path: PathBuf,
        counter_bytes: Vec<u8>,
    }

    fn prepared_restart_case(game: GameKind, name: &str, seed: u64) -> RestartTestCase {
        let game_path = test_save(&format!("restart-{name}-game"));
        let counter_path = test_save(&format!("restart-{name}-counters"));
        let profile_path = test_save(&format!("restart-{name}-profile"));
        for path in [&game_path, &counter_path, &profile_path] {
            remove_save(path);
        }
        let mut controller = controller(seed);
        controller.active = game;
        if game == GameKind::Klondike {
            controller.game = Game::new(
                controller.game.state.seed,
                Options {
                    timed: true,
                    ..controller.game.state.options
                },
            );
        }
        match game {
            GameKind::Klondike => controller.save_path = Some(game_path.clone()),
            GameKind::Spider => controller.spider_save_path = Some(game_path.clone()),
            GameKind::FreeCell => controller.freecell_save_path = Some(game_path.clone()),
            GameKind::TriPeaks => controller.tripeaks_save_path = Some(game_path.clone()),
            GameKind::Pyramid => controller.pyramid_save_path = Some(game_path.clone()),
        }
        controller.deal_counters_path = Some(counter_path.clone());
        controller.local_profile_path = Some(profile_path.clone());
        assert_eq!(
            ensure_deal_counters(&counter_path, controller.next_seeds).unwrap(),
            controller.next_seeds
        );
        assert!(controller.save());
        let counter_bytes = fs::read(&counter_path).unwrap();
        RestartTestCase {
            controller,
            game_path,
            counter_path,
            profile_path,
            counter_bytes,
        }
    }

    fn progress_active_game(controller: &mut Controller) {
        match controller.active {
            GameKind::Klondike => {
                controller.apply(controller.game.hint().unwrap());
                controller.game.state.advance_time(37);
                assert!(controller.save());
            }
            GameKind::Spider => controller.apply_spider(controller.spider.hint().unwrap()),
            GameKind::FreeCell => controller.apply_freecell(controller.freecell.hint().unwrap()),
            GameKind::TriPeaks => controller.apply_tripeaks(controller.tripeaks.hint().unwrap()),
            GameKind::Pyramid => controller.apply_pyramid(controller.pyramid.hint().unwrap()),
        }
    }

    fn restart_and_assert_exact_game(controller: &mut Controller, game_path: &Path) {
        match controller.active {
            GameKind::Klondike => {
                let initial = Game::new(controller.game.state.seed, controller.game.state.options);
                controller.restart_current_deal();
                assert_eq!(controller.game, initial);
                assert!(controller.game.state.options.timed);
                assert_eq!(controller.game.state.elapsed_seconds, 0);
                assert_eq!(load_klondike_revisioned(game_path).unwrap().0, initial);
            }
            GameKind::Spider => {
                let initial =
                    SpiderGame::new(controller.spider.state.seed, controller.spider.state.mode);
                controller.restart_current_deal();
                assert_eq!(controller.spider, initial);
                assert_eq!(load_spider_revisioned(game_path).unwrap().0, initial);
            }
            GameKind::FreeCell => {
                let initial = FreeCellGame::new(controller.freecell.state.deal_number);
                controller.restart_current_deal();
                assert_eq!(controller.freecell, initial);
                assert_eq!(load_freecell_revisioned(game_path).unwrap().0, initial);
            }
            GameKind::TriPeaks => {
                let initial = TriPeaksGame::new(
                    controller.tripeaks.state.seed,
                    controller.tripeaks.state.options,
                );
                controller.restart_current_deal();
                assert_eq!(controller.tripeaks, initial);
                assert_eq!(load_tripeaks_revisioned(game_path).unwrap().0, initial);
            }
            GameKind::Pyramid => {
                let initial = PyramidGame::new(
                    controller.pyramid.state.seed,
                    controller.pyramid.state.options,
                );
                controller.restart_current_deal();
                assert_eq!(controller.pyramid, initial);
                assert_eq!(load_pyramid_revisioned(game_path).unwrap().0, initial);
            }
        }
    }

    fn assert_restart_case(mut case: RestartTestCase) {
        progress_active_game(&mut case.controller);
        let next_seeds = case.controller.next_seeds;
        let profile = case.controller.local_profile.clone();
        case.controller.selection = Some(Selection::Waste);
        case.controller.spider_selection = Some(SpiderSelection {
            column: 0,
            count: 1,
        });
        case.controller.freecell_selection = Some(FreeCellSelection {
            pile: freecell::Pile::FreeCell(0),
            count: 1,
        });
        case.controller.pyramid_selection = Some(pyramid::Source::Waste);
        restart_and_assert_exact_game(&mut case.controller, &case.game_path);
        assert_eq!(
            case.controller.status,
            format!("Restarted {} deal", case.controller.game_name())
        );
        assert!(case.controller.pending_new_deal.is_none());
        assert!(!case.controller.pending_new_deal_conflict);
        assert!(!case.controller.dirty[case.controller.active_index()]);
        assert!(!case.controller.can_undo());
        assert!(!case.controller.can_redo());
        assert!(case.controller.selection.is_none());
        assert!(case.controller.spider_selection.is_none());
        assert!(case.controller.freecell_selection.is_none());
        assert!(case.controller.pyramid_selection.is_none());
        assert_eq!(case.controller.next_seeds, next_seeds);
        assert_eq!(fs::read(&case.counter_path).unwrap(), case.counter_bytes);
        assert_eq!(case.controller.local_profile, profile);
        assert_eq!(
            solitaire::persistence::load_local_profile(&case.profile_path).unwrap(),
            profile
        );
        for path in [&case.game_path, &case.counter_path, &case.profile_path] {
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            remove_save(path);
        }
    }

    #[test]
    fn restart_current_deal_preserves_seed_rules_counters_and_profile_for_all_games() {
        for (game, name, seed) in [
            (GameKind::Klondike, "klondike", 701),
            (GameKind::Spider, "spider", 702),
            (GameKind::FreeCell, "freecell", 703),
            (GameKind::TriPeaks, "tripeaks", 704),
            (GameKind::Pyramid, "pyramid", 705),
        ] {
            assert_restart_case(prepared_restart_case(game, name, seed));
        }
    }

    #[test]
    fn restart_without_a_writable_save_fails_closed_and_can_be_cancelled() {
        let mut controller = controller(801);
        let current = controller.game.clone();
        let next_seeds = controller.next_seeds;

        controller.restart_current_deal();

        assert_eq!(controller.game, current);
        assert_eq!(controller.next_seeds, next_seeds);
        assert!(
            controller
                .pending_new_deal
                .is_some_and(PendingNewDeal::is_restart)
        );
        assert!(controller.status.contains("no writable save location"));
        controller.cancel_pending_new_deal();
        assert_eq!(controller.game, current);
        assert!(controller.pending_new_deal.is_none());
        assert_eq!(
            controller.status,
            "Restart cancelled; current game preserved"
        );
    }

    #[test]
    fn restart_conflict_preserves_memory_then_reloads_and_retries_atomically() {
        let path = test_save("restart-conflict");
        remove_save(&path);
        let mut controller = controller(802);
        controller.save_path = Some(path.clone());
        assert!(controller.save());
        let current = controller.game.clone();
        let mut external = current.clone();
        external.apply(Action::Draw).unwrap();
        let (_, external_revision) = load_klondike_revisioned(&path).unwrap();
        let mut expected_external_revision = Some(external_revision);
        save_klondike_checked(&path, &external, &mut expected_external_revision).unwrap();

        controller.restart_current_deal();

        assert_eq!(controller.game, current);
        assert_eq!(load_klondike_revisioned(&path).unwrap().0, external);
        assert!(controller.pending_new_deal_conflict);
        assert!(
            controller
                .pending_new_deal
                .is_some_and(PendingNewDeal::is_restart)
        );
        controller.reload_disk_copy();
        assert_eq!(controller.game, external);
        assert!(controller.pending_new_deal.is_some());
        controller.retry_save();
        assert_eq!(controller.game, current);
        assert_eq!(load_klondike_revisioned(&path).unwrap().0, current);
        assert!(controller.pending_new_deal.is_none());
        assert!(!controller.pending_new_deal_conflict);
        remove_save(&path);
    }

    fn timed_klondike_controller(seed: u64, path: &Path) -> Controller {
        let mut controller = controller(seed);
        controller.game = Game::new(
            seed,
            Options {
                timed: true,
                ..Options::default()
            },
        );
        controller.save_path = Some(path.to_path_buf());
        assert!(controller.save());
        controller
    }

    #[test]
    fn timed_klondike_checkpoints_are_bounded_atomic_and_profile_independent() {
        let game_path = test_save("timed-checkpoint-game");
        let counter_path = test_save("timed-checkpoint-counters");
        let profile_path = test_save("timed-checkpoint-profile");
        for path in [&game_path, &counter_path, &profile_path] {
            remove_save(path);
        }
        let mut controller = timed_klondike_controller(901, &game_path);
        controller.deal_counters_path = Some(counter_path.clone());
        controller.local_profile_path = Some(profile_path.clone());
        ensure_deal_counters(&counter_path, controller.next_seeds).unwrap();
        assert!(controller.save_local_profile());
        let initial_game_bytes = fs::read(&game_path).unwrap();
        let counter_bytes = fs::read(&counter_path).unwrap();
        let profile_bytes = fs::read(&profile_path).unwrap();

        controller.advance_klondike_timer(14);
        assert_eq!(controller.game.state.elapsed_seconds, 14);
        assert!(controller.klondike_elapsed_dirty);
        assert!(!controller.dirty[0]);
        assert_eq!(fs::read(&game_path).unwrap(), initial_game_bytes);

        controller.advance_klondike_timer(1);
        assert_eq!(controller.game.state.elapsed_seconds, 15);
        assert!(!controller.klondike_elapsed_dirty);
        assert_eq!(
            load_klondike_revisioned(&game_path)
                .unwrap()
                .0
                .state
                .elapsed_seconds,
            15
        );
        assert_eq!(fs::read(&counter_path).unwrap(), counter_bytes);
        assert_eq!(fs::read(&profile_path).unwrap(), profile_bytes);
        controller.game.state.elapsed_seconds = u64::MAX;
        controller.advance_klondike_timer(1);
        assert_eq!(controller.game.state.elapsed_seconds, u64::MAX);
        assert!(!controller.klondike_elapsed_dirty);
        for path in [&game_path, &counter_path, &profile_path] {
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            remove_save(path);
        }
    }

    #[test]
    fn timed_klondike_pauses_and_checkpoints_before_switching_games() {
        let path = test_save("timed-switch-checkpoint");
        remove_save(&path);
        let mut controller = timed_klondike_controller(902, &path);
        controller.pending_new_deal = Some(PendingNewDeal {
            game: GameKind::Klondike,
            variant: NewDealVariant::Klondike {
                draw_mode: DrawMode::One,
                scoring: Scoring::Standard,
                max_redeals: None,
                timed: true,
            },
            restart_seed: Some(902),
        });
        controller.advance_klondike_timer(5);
        assert_eq!(controller.game.state.elapsed_seconds, 0);
        controller.pending_new_deal = None;

        controller.advance_klondike_timer(3);
        controller.select_game("Spider");
        assert!(controller.active == GameKind::Spider);
        assert_eq!(
            load_klondike_revisioned(&path)
                .unwrap()
                .0
                .state
                .elapsed_seconds,
            3
        );
        controller.advance_klondike_timer(5);
        assert_eq!(controller.game.state.elapsed_seconds, 3);

        controller.active = GameKind::Klondike;
        controller.game.state.stock.clear();
        controller.game.state.waste.clear();
        for column in &mut controller.game.state.tableau {
            column.clear();
        }
        controller.game.state.foundations = Suit::ALL.map(|suit| {
            Rank::ALL
                .into_iter()
                .map(|rank| Card::new(suit, rank))
                .collect()
        });
        assert!(controller.game.state.is_won());
        controller.advance_klondike_timer(5);
        assert_eq!(controller.game.state.elapsed_seconds, 3);
        remove_save(&path);
    }

    #[test]
    fn timed_klondike_checkpoint_failure_is_recoverable_and_fail_closed() {
        let path = test_save("timed-checkpoint-retry");
        remove_save(&path);
        let mut controller = controller(903);
        controller.game = Game::new(
            903,
            Options {
                timed: true,
                ..Options::default()
            },
        );

        controller.advance_klondike_timer(KLONDIKE_TIMER_CHECKPOINT_SECONDS);
        assert_eq!(controller.game.state.elapsed_seconds, 15);
        assert!(controller.klondike_elapsed_dirty);
        assert!(controller.dirty[0]);
        assert!(controller.status.contains("no writable save location"));
        controller.select_game("Spider");
        assert!(controller.active == GameKind::Klondike);
        assert_eq!(controller.game.state.elapsed_seconds, 15);

        controller.save_path = Some(path.clone());
        controller.retry_save();
        assert!(!controller.dirty[0]);
        assert!(!controller.klondike_elapsed_dirty);
        assert_eq!(load_klondike_revisioned(&path).unwrap().0, controller.game);
        remove_save(&path);
    }

    #[test]
    fn timed_klondike_stale_checkpoint_preserves_both_owners_until_reload() {
        let path = test_save("timed-checkpoint-conflict");
        remove_save(&path);
        let mut controller = timed_klondike_controller(904, &path);
        let (mut external, revision) = load_klondike_revisioned(&path).unwrap();
        external.state.advance_time(7);
        let mut external_revision = Some(revision);
        save_klondike_checked(&path, &external, &mut external_revision).unwrap();

        controller.advance_klondike_timer(KLONDIKE_TIMER_CHECKPOINT_SECONDS);
        assert_eq!(controller.game.state.elapsed_seconds, 15);
        assert_eq!(load_klondike_revisioned(&path).unwrap().0, external);
        assert!(controller.dirty[0]);
        assert!(controller.status.contains("checkpoint failed"));

        controller.reload_disk_copy();
        assert_eq!(controller.game, external);
        assert!(!controller.dirty[0]);
        assert!(!controller.klondike_elapsed_dirty);
        remove_save(&path);
    }

    #[test]
    fn elapsed_time_format_is_bounded_and_exact() {
        assert_eq!(format_elapsed_time(0), "00:00:00");
        assert_eq!(format_elapsed_time(3_661), "01:01:01");
        assert_eq!(format_elapsed_time(u64::MAX), "5124095576030431:00:15");
    }

    #[test]
    fn klondike_new_deal_choices_are_saved_and_reopen_with_exact_options() {
        for (choice, draw_mode, scoring, timed, starting_score) in [
            (
                "Draw 1 · Standard",
                DrawMode::One,
                Scoring::Standard,
                false,
                0,
            ),
            ("Draw 1 · Vegas", DrawMode::One, Scoring::Vegas, false, -52),
            (
                "Draw 3 · Standard",
                DrawMode::Three,
                Scoring::Standard,
                false,
                0,
            ),
            (
                "Draw 3 · Vegas",
                DrawMode::Three,
                Scoring::Vegas,
                false,
                -52,
            ),
            (
                "Draw 1 · Standard · Unlimited · Timed",
                DrawMode::One,
                Scoring::Standard,
                true,
                0,
            ),
            (
                "Draw 3 · Vegas · Unlimited · Timed",
                DrawMode::Three,
                Scoring::Vegas,
                true,
                -52,
            ),
        ] {
            let path = test_save(&format!("klondike-new-deal-{choice}"));
            remove_save(&path);
            let mut controller = controller(212);
            controller.save_path = Some(path.clone());

            controller.new_game(choice);

            assert_eq!(controller.game.state.options.draw_mode, draw_mode);
            assert_eq!(controller.game.state.options.scoring, scoring);
            assert_eq!(controller.game.state.options.max_redeals, None);
            assert_eq!(controller.game.state.options.timed, timed);
            assert_eq!(controller.game.state.score, starting_score);
            let (reopened, _) = load_klondike_revisioned(&path).unwrap();
            assert_eq!(reopened, controller.game);
            remove_save(&path);
        }
    }

    #[test]
    fn klondike_redeal_limits_are_atomic_reopenable_and_enforced() {
        for (label, expected) in [
            ("Unlimited", None),
            ("1 redeal", Some(1)),
            ("3 redeals", Some(3)),
        ] {
            let path = test_save(&format!("klondike-redeal-{label}"));
            remove_save(&path);
            let mut controller = controller(214);
            controller.save_path = Some(path.clone());
            controller.new_game(&format!("Draw 3 · Standard · {label}"));
            assert_eq!(controller.game.state.options.max_redeals, expected);
            assert_eq!(load_klondike_revisioned(&path).unwrap().0, controller.game);
            remove_save(&path);
        }

        let path = test_save("klondike-redeal-enforced");
        remove_save(&path);
        let mut controller = controller(215);
        controller.save_path = Some(path.clone());
        controller.new_game("Draw 3 · Standard · 1 redeal");
        for _ in 0..8 {
            controller.draw_or_recycle();
        }
        controller.draw_or_recycle();
        assert_eq!(controller.game.state.redeals, 1);
        for _ in 0..8 {
            controller.draw_or_recycle();
        }
        let exhausted = controller.game.clone();
        let saved = fs::read(&path).unwrap();
        controller.draw_or_recycle();
        assert_eq!(controller.status, "No redeals remain");
        assert_eq!(controller.game, exhausted);
        assert_eq!(fs::read(&path).unwrap(), saved);
        controller.undo();
        assert!(controller.game.can_redo());
        controller.redo();
        assert_eq!(controller.game, exhausted);
        assert_eq!(fs::read(&path).unwrap(), saved);
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(load_klondike_revisioned(&path).unwrap().0, exhausted);
        remove_save(&path);
    }

    #[test]
    fn reopened_klondike_options_map_values_and_indices_without_a_display() {
        assert_eq!(
            klondike_ui_options(Options {
                draw_mode: DrawMode::One,
                scoring: Scoring::Standard,
                max_redeals: None,
                timed: false,
            }),
            KlondikeUiOptions {
                draw_index: 0,
                draw_mode: "Draw 1",
                scoring_index: 0,
                scoring_mode: "Standard",
                redeal_index: 0,
                redeal_limit: "Unlimited",
                timing_index: 0,
                timing_mode: "Untimed",
            }
        );
        assert_eq!(
            klondike_ui_options(Options {
                draw_mode: DrawMode::Three,
                scoring: Scoring::Vegas,
                max_redeals: Some(1),
                timed: false,
            }),
            KlondikeUiOptions {
                draw_index: 1,
                draw_mode: "Draw 3",
                scoring_index: 1,
                scoring_mode: "Vegas",
                redeal_index: 1,
                redeal_limit: "1 redeal",
                timing_index: 0,
                timing_mode: "Untimed",
            }
        );
        assert_eq!(
            klondike_ui_options(Options {
                draw_mode: DrawMode::Three,
                scoring: Scoring::Vegas,
                max_redeals: Some(3),
                timed: false,
            }),
            KlondikeUiOptions {
                draw_index: 1,
                draw_mode: "Draw 3",
                scoring_index: 1,
                scoring_mode: "Vegas",
                redeal_index: 2,
                redeal_limit: "3 redeals",
                timing_index: 0,
                timing_mode: "Untimed",
            }
        );
        assert_eq!(
            klondike_ui_options(Options {
                draw_mode: DrawMode::One,
                scoring: Scoring::Standard,
                max_redeals: Some(2),
                timed: true,
            }),
            KlondikeUiOptions {
                draw_index: 0,
                draw_mode: "Draw 1",
                scoring_index: 0,
                scoring_mode: "Standard",
                redeal_index: -1,
                redeal_limit: "Custom",
                timing_index: 1,
                timing_mode: "Timed",
            }
        );

        let mut controller = controller(216);
        assert!(klondike_ui_options_for_render(&controller).is_some());
        controller.pending_new_deal =
            parse_klondike_variant("Draw 1 · Standard · 1 redeal").map(|variant| PendingNewDeal {
                game: GameKind::Klondike,
                variant,
                restart_seed: None,
            });
        assert_eq!(klondike_ui_options_for_render(&controller), None);
    }

    #[test]
    fn exact_freecell_deal_is_strict_atomic_reopenable_and_does_not_consume_next_deal() {
        let game_path = test_save("freecell-exact-number");
        let counter_path = test_save("freecell-exact-counter");
        remove_save(&game_path);
        remove_save(&counter_path);
        let mut controller = controller(212);
        controller.select_game("FreeCell");
        controller.freecell_save_path = Some(game_path.clone());
        controller.deal_counters_path = Some(counter_path.clone());
        assert!(controller.save());
        let original_game = controller.freecell.clone();
        let original_save = fs::read(&game_path).unwrap();
        let original_counters = controller.next_seeds;
        let oversized = "9".repeat(4_096);

        for invalid in [
            "",
            " ",
            "+1",
            "-1",
            "1.0",
            "1 2",
            "١",
            "18446744073709551616",
            oversized.as_str(),
        ] {
            controller.new_freecell_game(invalid);
            assert_eq!(controller.freecell, original_game, "{invalid:?}");
            assert_eq!(controller.next_seeds, original_counters, "{invalid:?}");
            assert_eq!(fs::read(&game_path).unwrap(), original_save, "{invalid:?}");
            assert!(!counter_path.exists(), "{invalid:?}");
            assert!(controller.pending_new_deal.is_none(), "{invalid:?}");
            assert_eq!(
                controller.status,
                "Enter a decimal FreeCell deal number from 0 through 18446744073709551615; current game preserved"
            );
        }

        controller.new_freecell_game("0");
        assert_eq!(controller.freecell.state.deal_number, 0);
        assert_eq!(controller.next_seeds, original_counters);
        assert_eq!(
            load_deal_counters(&counter_path).unwrap(),
            original_counters
        );
        assert_eq!(
            load_freecell_revisioned(&game_path).unwrap().0,
            controller.freecell
        );

        controller.new_freecell_game("18446744073709551615");
        assert_eq!(controller.freecell.state.deal_number, u64::MAX);
        assert_eq!(controller.next_seeds, original_counters);
        assert_eq!(
            load_deal_counters(&counter_path).unwrap(),
            original_counters
        );
        assert_eq!(
            load_freecell_revisioned(&game_path).unwrap().0,
            controller.freecell
        );

        controller.dirty[controller.active_index()] = true;
        controller.new_freecell_game("42");
        let pending = controller.pending_new_deal;
        assert!(
            pending
                == Some(PendingNewDeal {
                    game: GameKind::FreeCell,
                    variant: NewDealVariant::FreeCell(FreeCellDeal::Exact(42)),
                    restart_seed: None,
                })
        );
        controller.new_freecell_game(" 42");
        assert!(controller.pending_new_deal == pending);
        assert_eq!(controller.freecell.state.deal_number, u64::MAX);
        controller.discard_progress_and_start_pending();
        assert_eq!(controller.freecell.state.deal_number, 42);
        assert!(controller.pending_new_deal.is_none());
        assert_eq!(controller.next_seeds, original_counters);
        assert_eq!(
            load_deal_counters(&counter_path).unwrap(),
            original_counters
        );
        assert_eq!(
            load_freecell_revisioned(&game_path).unwrap().0,
            controller.freecell
        );

        controller.new_game("Next numbered deal");
        assert_eq!(
            controller.freecell.state.deal_number,
            original_counters.freecell
        );
        assert_eq!(
            controller.next_seeds.freecell,
            original_counters.freecell + 1
        );
        assert!(counter_path.exists());

        remove_save(&game_path);
        remove_save(&counter_path);
    }

    #[test]
    fn exact_freecell_deal_restart_preserves_the_durable_next_sequence() {
        const CHILD_ROOT: &str = "SOLITAIRE_EXACT_FREECELL_RESTART_ROOT";
        if std::env::var_os(CHILD_ROOT).is_some() {
            let mut restarted = Controller::new();
            restarted.select_game("FreeCell");
            assert_eq!(restarted.freecell.state.deal_number, u64::MAX);
            assert_eq!(restarted.next_seeds.freecell, 213);
            restarted.new_game("Next numbered deal");
            assert_eq!(restarted.freecell.state.deal_number, 213);
            assert_eq!(restarted.next_seeds.freecell, 214);
            return;
        }

        let root = std::env::temp_dir().join(format!(
            "solitaire-controller-{}-freecell-restart",
            std::process::id()
        ));
        let data = root.join("solitaire");
        let game_path = data.join("freecell-save.json");
        let counter_path = data.join("deal-counters.json");
        remove_save(&game_path);
        remove_save(&counter_path);

        let mut initial = controller(212);
        initial.select_game("FreeCell");
        initial.freecell_save_path = Some(game_path.clone());
        initial.deal_counters_path = Some(counter_path.clone());
        initial.new_freecell_game("18446744073709551615");
        assert_eq!(initial.freecell.state.deal_number, u64::MAX);
        assert_eq!(load_deal_counters(&counter_path).unwrap().freecell, 213);

        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "tests::exact_freecell_deal_restart_preserves_the_durable_next_sequence",
                "--exact",
                "--nocapture",
            ])
            .env("XDG_DATA_HOME", &root)
            .env(CHILD_ROOT, &root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "child stdout: {}\nchild stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            load_freecell_revisioned(&game_path)
                .unwrap()
                .0
                .state
                .deal_number,
            213
        );
        assert_eq!(load_deal_counters(&counter_path).unwrap().freecell, 214);

        for name in [
            "klondike-save.json",
            "spider-save.json",
            "freecell-save.json",
            "tripeaks-save.json",
            "pyramid-save.json",
            "deal-counters.json",
            "local-profile.json",
        ] {
            remove_save(&data.join(name));
        }
        let _ = fs::remove_dir(&data);
        let _ = fs::remove_dir(&root);
    }

    #[test]
    fn freecell_deal_entry_is_scoped_to_freecell() {
        let mut controller = controller(213);
        let original = controller.game.clone();
        let counters = controller.next_seeds;

        controller.new_freecell_game("42");

        assert_eq!(controller.game, original);
        assert_eq!(controller.next_seeds, counters);
        assert!(controller.pending_new_deal.is_none());
        assert_eq!(
            controller.status,
            "Numbered deal entry is available only in FreeCell"
        );
    }

    #[test]
    fn freecell_deal_number_preserves_full_u64_range() {
        assert_eq!(freecell_deal_number(0).as_str(), "0");
        assert_eq!(
            freecell_deal_number(u64::MAX).as_str(),
            u64::MAX.to_string()
        );
    }

    #[test]
    fn malformed_new_deal_options_preserve_game_save_counter_and_pending_request() {
        let game_path = test_save("invalid-new-deal-game");
        let counter_path = test_save("invalid-new-deal-counter");
        remove_save(&game_path);
        remove_save(&counter_path);
        let mut controller = controller(213);
        controller.save_path = Some(game_path.clone());
        controller.deal_counters_path = Some(counter_path.clone());
        assert!(controller.save());
        let original_game = controller.game.clone();
        let original_save = fs::read(&game_path).unwrap();
        let original_counters = controller.next_seeds;

        for invalid in [
            "",
            "Draw 3 Vegas",
            " Draw 3 · Vegas",
            "Draw 3 · Vegas ",
            "Draw 2 · Standard",
            "Draw 3 · Unknown",
            "Draw 3 · Vegas · 2 redeals",
            "Draw 3 · Vegas · 1 redeal · trailing",
            "Draw 3 · Vegas · 1 redeal · timed",
            "Draw 3 · Vegas · 1 redeal · Timed · trailing",
        ] {
            controller.new_game(invalid);
            assert_eq!(controller.game, original_game, "{invalid:?}");
            assert_eq!(controller.next_seeds, original_counters, "{invalid:?}");
            assert_eq!(fs::read(&game_path).unwrap(), original_save, "{invalid:?}");
            assert!(!counter_path.exists(), "{invalid:?}");
            assert!(controller.pending_new_deal.is_none(), "{invalid:?}");
            assert_eq!(
                controller.status,
                "Invalid new-deal options; current game preserved"
            );
        }

        for oversized in [
            format!("Draw 3 · Vegas · {}", "3".repeat(4_096)),
            format!("Draw 3 · Vegas · Unlimited · {}", "T".repeat(4_096)),
        ] {
            controller.new_game(&oversized);
            assert_eq!(controller.game, original_game);
            assert_eq!(controller.next_seeds, original_counters);
            assert_eq!(fs::read(&game_path).unwrap(), original_save);
            assert!(!counter_path.exists());
            assert!(controller.pending_new_deal.is_none());
        }

        controller.dirty[0] = true;
        controller.new_game("Draw 3 · Vegas");
        let pending = controller.pending_new_deal;
        assert!(pending.is_some());
        controller.new_game("Draw 3 Vegas");
        assert!(controller.pending_new_deal == pending);
        assert_eq!(controller.game, original_game);
        assert_eq!(controller.next_seeds, original_counters);
        assert_eq!(fs::read(&game_path).unwrap(), original_save);
        assert!(!counter_path.exists());

        let hostile_pending = PendingNewDeal {
            game: GameKind::Klondike,
            variant: NewDealVariant::Pyramid { max_redeals: 2 },
            restart_seed: None,
        };
        controller.pending_new_deal = Some(hostile_pending);
        controller.commit_pending_new_deal();
        assert!(controller.pending_new_deal == Some(hostile_pending));
        assert_eq!(controller.game, original_game);
        assert_eq!(controller.next_seeds, original_counters);
        assert_eq!(fs::read(&game_path).unwrap(), original_save);
        assert!(!counter_path.exists());
        assert_eq!(
            controller.status,
            "Invalid pending new-deal options; current game preserved"
        );

        remove_save(&game_path);
        remove_save(&counter_path);
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
