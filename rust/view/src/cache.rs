//! What a game's views keep between builds, and a game held together with it.
//!
//! Views are built from the game itself: [`GameStatus::determine_status`] and
//! `Game::get_applicable_events` take `&Game`. The server keeps one [`ViewCache`] per game it
//! sends views of, next to the game (`server/src/views.rs`). Tools and tests that hold a game of
//! their own use [`ViewedGame`], which keeps the two together.
//!
//! Views share the parts that are the same for every viewer, as `Arc`s, which serialize like
//! the values they hold. A [`ViewBatch`] builds the views of one game state: it borrows the
//! game, so the game can't change while it lives, and its views share the parts that don't
//! read the viewer (`build::StateParts`). The cache keeps the parts made only from the rules,
//! the factions and the skin between game states (`build::RulesParts`), each with a key that
//! holds what it reads of the game.
//!
//! [`GameStatus::determine_status`]: treachery_engine::game::GameStatus::determine_status

use treachery_engine::game::GameState;
use treachery_engine::prelude::*;

use crate::actions::QueryError;
use crate::build::{self, ResolvedViewer, RulesParts, Seating, StateParts, Viewer};
use crate::describe::{default_describer, Describer, InGame};
use crate::schema::{DraftResult, GameView, ReportLines, RichText};

/// The skin a game's views are written in, and the parts of the last view made only from the
/// rules, the factions and the skin (the faction sheets, the rule labels), which the next views
/// reuse while they stay the same.
pub struct ViewCache {
    /// `DefaultSkin.Default` until the server replaces it with the host's skin. Private, so that
    /// only [`ViewCache::set_skin`] changes it, which clears `rules`.
    describer: Box<dyn Describer>,
    rules: RulesParts,
}

impl Default for ViewCache {
    fn default() -> Self {
        ViewCache { describer: default_describer(), rules: RulesParts::default() }
    }
}

/// Builds the views of one game state, with one seating and bidding timer. Its views share the
/// parts that don't read the viewer, built once for all of them. It borrows the game, the
/// seating and the cache, so none of them can change while it lives.
pub struct ViewBatch<'a> {
    describer: &'a dyn Describer,
    rules: &'a mut RulesParts,
    game: &'a Game,
    seating: &'a Seating,
    timer: i32,
    state: StateParts,
}

impl ViewBatch<'_> {
    pub fn view(&mut self, viewer: Viewer) -> GameView {
        self.view_as(&viewer.resolve(self.game))
    }

    pub fn view_as(&mut self, v: &ResolvedViewer) -> GameView {
        build::build_view_cached(self.game, v, self.seating, self.describer, self.timer, self.rules, &mut self.state)
    }
}

impl ViewCache {
    /// The skin loaded over the default skin. A skin that doesn't load leaves the current one.
    pub fn set_skin(&mut self, skin_json: &str) -> Result<(), String> {
        let skin = treachery_skin::Skin::load(skin_json, treachery_skin::default_skin()).map_err(|e| e.to_string())?;
        self.describer = Box::new(skin);
        self.rules.clear();
        Ok(())
    }

    /// The view of `viewer`. `timer` is the bidding time limit the host set, or -1. For several
    /// views of the same game state, a [`ViewBatch`] builds what they share once.
    pub fn view(&mut self, game: &Game, viewer: Viewer, s: &Seating, timer: i32) -> GameView {
        self.batch(game, s, timer).view(viewer)
    }

    /// The view of a viewer resolved by the caller, for games whose participation seats no
    /// users (the bot-game traces).
    pub fn view_as(&mut self, game: &Game, v: &ResolvedViewer, s: &Seating, timer: i32) -> GameView {
        self.batch(game, s, timer).view_as(v)
    }

    /// A builder of views of `game` as it is now, with seating `s` and bidding timer `timer`.
    pub fn batch<'a>(&'a mut self, game: &'a Game, s: &'a Seating, timer: i32) -> ViewBatch<'a> {
        ViewBatch { describer: self.describer.as_ref(), rules: &mut self.rules, game, seating: s, timer, state: StateParts::default() }
    }

    /// A draft query for `viewer`. Refused unless the draft's type is one of the viewer's
    /// actions now, which are the actions of the view they were last sent.
    pub fn query(&self, game: &Game, viewer: Viewer, s: &Seating, draft_json: &str) -> Result<DraftResult, QueryError> {
        let v = viewer.resolve(game);
        let applicable = game.get_applicable_events(v.player, v.is_host);
        crate::actions::query(game, viewer, s, &applicable, draft_json, &InGame::new(self.describer.as_ref(), game))
    }

    /// The `report` call: the lines of report `index` that `viewer` may read, as in the view's log.
    pub fn report(&self, game: &Game, viewer: Viewer, index: usize) -> Option<ReportLines> {
        let r = game.reports().get(index)?;
        let v = viewer.resolve(game);
        Some(build::report_lines(index, r, &v, &InGame::new(self.describer.as_ref(), game)))
    }

    /// The `undoList` call: the short message of every event, for the undo panel. The number of an
    /// entry is its index in the history, the `until` of the `undo` call.
    pub fn undo_list(&self, game: &Game) -> Vec<UndoEntry> {
        let d = InGame::new(self.describer.as_ref(), game);
        game.history
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| {
                // Moment n - 1 holds the turn and phase event n happened in.
                let moment = i.checked_sub(1).and_then(|m| game.history.moments.get(m));
                UndoEntry {
                    event_number: i as i32,
                    text: crate::expr::message(&e.event.describe_short(game, e.by_in(game)), &d),
                    listed: !matches!(e.event, Event::AllyPermission(_) | Event::BattleRevision(_) | Event::AutomationConfigured(_)),
                    turn: moment.map_or(treachery_engine::game::Turn::BEFORE_START, |m| m.turn),
                    phase: moment.map_or_else(|| "None".to_string(), |m| m.phase.to_string()),
                }
            })
            .collect()
    }

    /// The `undoPreview` call: the view `viewer` had at gap `gap`, the moment just before event
    /// `gap`. The gap is clamped to `1..=` the number of events. The history is replayed into a
    /// new game with the same human seats, so `game` is not touched. A preview can't be acted
    /// on, so the view has no actions and no card plays.
    pub fn preview(&self, game: &Game, viewer: Viewer, s: &Seating, gap: i32) -> Result<GameView, String> {
        let count = game.history.entries.len() as i32;
        let gap = gap.clamp(1.min(count), count) as usize;
        let mut earlier = game.new_like();
        for (nr, e) in game.history.entries.iter().take(gap).enumerate() {
            let name = e.event_type().name();
            if let Err(error) = earlier.play(e.clone(), false, false) {
                return Err(format!("event {nr} ({}) failed: {:?}", name, treachery_skin::default_skin().render(&error.message())));
            }
        }
        let mut view = build::build_view(&earlier, viewer, s, self.describer.as_ref(), -1);
        view.actions.clear();
        if let Some(me) = &mut view.me {
            for c in &mut me.hand {
                c.playable_with.clear();
            }
        }
        Ok(view)
    }
}

/// A game together with its [`ViewCache`] and the server state its views show (the seating and
/// the bidding timer), for tools and tests that hold a game of their own. The server keeps
/// these in its own structures.
pub struct ViewedGame {
    pub game: Game,
    pub cache: ViewCache,
    /// The bidding time limit in seconds, or -1, which the host sets with `SetTimer`.
    pub timer: i32,
    /// The names of the seated users and the bot speed. The seats themselves are in
    /// `game.human_seats`.
    pub seating: Seating,
}

impl ViewedGame {
    /// A game with no events yet, that bots play. Events follow with [`ViewedGame::apply`].
    pub fn new(version: i32) -> ViewedGame {
        ViewedGame::of(Game::of_version(version))
    }

    pub fn of(game: Game) -> ViewedGame {
        ViewedGame { game, cache: ViewCache::default(), timer: -1, seating: Seating::default() }
    }

    /// Loads a savegame (of version 191; any other version is refused) in a game that bots play.
    pub fn load(savegame: &str) -> Result<ViewedGame, String> {
        let state: GameState = GameState::load(savegame)?;
        Game::check_version(state.version)?;
        let mut game = Game::of_version(state.version);
        for (nr, e) in state.events.iter().enumerate() {
            if let Err(error) = game.play(e.clone(), false, false) {
                return Err(format!("event {nr} ({}) failed: {:?}", e.event_type().name(), treachery_skin::default_skin().render(&error.message())));
            }
        }
        Ok(ViewedGame::of(game))
    }

    /// The game of the web client's `/scene` route: a savegame loaded with one user, id 1 and
    /// named "Scene", seated as `faction` and made host. The other seats have no user, so they
    /// count as bots.
    pub fn scene(savegame: &str, faction: Faction) -> Result<(ViewedGame, Viewer), String> {
        const SCENE_USER_ID: i64 = 1;
        let mut g = ViewedGame::load(savegame)?;
        let player = g.game.get_player(Some(faction)).ok_or_else(|| format!("{faction} is not in this game"))?;
        let seat = g.game.player(player).seated();
        g.game.human_seats.insert(seat);
        g.seating.names.insert(seat, "Scene".to_string());
        g.seating.seated_users = 1;
        Ok((g, Viewer { user_id: SCENE_USER_ID, seat: Some(seat), is_host: true, is_observer: false }))
    }

    /// Executes an event without validating it.
    pub fn apply(&mut self, event_json: &str) -> Result<(), String> {
        let e: HistoryEntry = serde_json::from_str(event_json).map_err(|e| e.to_string())?;
        match self.game.play(e, false, false) {
            Ok(()) => Ok(()),
            Err(error) => Err(format!("{:?}", treachery_skin::default_skin().render(&error.message()))),
        }
    }

    pub fn view(&mut self, viewer: Viewer) -> GameView {
        self.cache.view(&self.game, viewer, &self.seating, self.timer)
    }

    pub fn view_as(&mut self, v: &ResolvedViewer) -> GameView {
        self.cache.view_as(&self.game, v, &self.seating, self.timer)
    }

    pub fn query(&self, viewer: Viewer, draft_json: &str) -> Result<DraftResult, QueryError> {
        self.cache.query(&self.game, viewer, &self.seating, draft_json)
    }

    pub fn report(&self, viewer: Viewer, index: usize) -> Option<ReportLines> {
        self.cache.report(&self.game, viewer, index)
    }

    pub fn undo_list(&self) -> Vec<UndoEntry> {
        self.cache.undo_list(&self.game)
    }

    pub fn preview(&self, viewer: Viewer, gap: i32) -> Result<GameView, String> {
        self.cache.preview(&self.game, viewer, &self.seating, gap)
    }

    pub fn set_skin(&mut self, skin_json: &str) -> Result<(), String> {
        self.cache.set_skin(skin_json)
    }
}

/// One line of the undo list (`undoList`, protocol.md). Its TypeScript type is generated with
/// the protocol's, in `web/src/protocol/`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct UndoEntry {
    pub event_number: i32,
    pub text: RichText,
    /// Whether the undo panel lists the event: it leaves out ally permissions, battle revisions
    /// and automation settings.
    pub listed: bool,
    /// The turn and main phase the event happened in (0 and "None" for event 0).
    #[ts(type = "number")]
    pub turn: treachery_engine::game::Turn,
    #[ts(type = "MainPhase")]
    pub phase: String,
}
