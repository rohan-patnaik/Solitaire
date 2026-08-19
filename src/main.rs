use slint::{ModelRc, SharedString, VecModel};
use solitaire::cards::{Card, Rank, Suit};
use solitaire::freecell::{self, Game as FreeCellGame};
use solitaire::klondike::{Action, DrawMode, Game, Options, Pile, Scoring};
use solitaire::persistence::{
    SaveError, default_freecell_save_path, default_save_path, default_spider_save_path,
    load_freecell, load_klondike, load_spider, quarantine_save, save_freecell, save_klondike,
    save_spider,
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
    next_seed: u64,
    status: String,
}

impl Controller {
    fn new() -> Self {
        let mut save_path = default_save_path();
        let mut status = "Choose a card to begin".to_owned();
        let saved = load_or_recover(&mut save_path, load_klondike, &mut status);
        let seed = saved.as_ref().map_or_else(seed_now, |game| game.state.seed);
        let mut spider_save_path = default_spider_save_path();
        let spider = load_or_recover(&mut spider_save_path, load_spider, &mut status)
            .unwrap_or_else(|| SpiderGame::new(seed.wrapping_add(1), SuitMode::One));
        let mut freecell_save_path = default_freecell_save_path();
        let freecell = load_or_recover(&mut freecell_save_path, load_freecell, &mut status)
            .unwrap_or_else(|| FreeCellGame::new(seed.wrapping_add(2)));
        Self {
            active: GameKind::Klondike,
            game: saved.unwrap_or_else(|| Game::new(seed, Options::default())),
            selection: None,
            save_path,
            spider,
            spider_selection: None,
            spider_save_path,
            freecell,
            freecell_selection: None,
            freecell_save_path,
            next_seed: seed.wrapping_add(3),
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
                self.save();
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
        self.status = format!("{} ready", self.game_name());
    }

    fn new_game(&mut self, variant: &str) {
        match self.active {
            GameKind::Klondike => {
                let draw_mode = if variant == "Draw 3" {
                    DrawMode::Three
                } else {
                    DrawMode::One
                };
                self.game = Game::new(
                    self.next_seed,
                    Options {
                        draw_mode,
                        scoring: Scoring::Standard,
                        max_redeals: None,
                        timed: false,
                    },
                );
            }
            GameKind::Spider => {
                let mode = match variant {
                    "2 suits" => SuitMode::Two,
                    "4 suits" => SuitMode::Four,
                    _ => SuitMode::One,
                };
                self.spider = SpiderGame::new(self.next_seed, mode);
            }
            GameKind::FreeCell => self.freecell = FreeCellGame::new(self.next_seed),
        }
        self.next_seed = self.next_seed.wrapping_add(1);
        self.selection = None;
        self.spider_selection = None;
        self.freecell_selection = None;
        self.status = format!("New {} deal", self.game_name());
        self.save();
    }

    fn save(&mut self) {
        let result = match self.active {
            GameKind::Klondike => self
                .save_path
                .as_deref()
                .map(|path| save_klondike(path, &self.game)),
            GameKind::Spider => self
                .spider_save_path
                .as_deref()
                .map(|path| save_spider(path, &self.spider)),
            GameKind::FreeCell => self
                .freecell_save_path
                .as_deref()
                .map(|path| save_freecell(path, &self.freecell)),
        };
        if let Some(Err(error)) = result {
            self.status = format!("Move kept in memory; save failed: {error}");
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
                self.save();
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
                self.save();
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
            self.save();
            "Move undone".into()
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
            self.save();
            "Move restored".into()
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
            self.save();
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
}

fn load_or_recover<T>(
    save_path: &mut Option<PathBuf>,
    load: impl FnOnce(&std::path::Path) -> Result<T, SaveError>,
    status: &mut String,
) -> Option<T> {
    let path = save_path.clone()?;
    match load(&path) {
        Ok(game) => Some(game),
        Err(SaveError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            match quarantine_save(&path) {
                Ok(quarantined) => {
                    *status = format!(
                        "Unreadable save preserved as {}; opened a fresh deal ({error})",
                        quarantined.display()
                    );
                }
                Err(quarantine_error) => {
                    *status = format!(
                        "Save recovery failed; original left untouched ({error}; {quarantine_error})"
                    );
                    *save_path = None;
                }
            }
            None
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    app.on_fan_spacing(bounded_fan_spacing);
    let controller = Rc::new(RefCell::new(Controller::new()));
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
            next_seed: seed.wrapping_add(1),
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

    fn to_i32(value: usize) -> i32 {
        i32::try_from(value).unwrap_or(i32::MAX)
    }
}
