//! The view replica: a second copy of a game that only the view code touches.
//!
//! It executes each event the server accepted (`execute(false, false)`, no validation), and
//! after each event every view is built from it, which runs `determine_status` and
//! `get_applicable_events`. The state those calls write (the battle sequence, the neighbour
//! cache) stays in the replica and never reaches the server's authoritative game.
//!
//! One replica serves every viewer of a game, so those calls run once per viewer per event.
//! The views don't depend on the order, because the only write that changes a later result is
//! `next_player_to_battle` advancing the battle sequence to the next player with a battle, after
//! which further reads return the same player without advancing (client.md, The view replica).

use std::collections::BTreeMap;

use treachery_engine::events::GameEventType;
use treachery_engine::game::GameState;
use treachery_engine::json;
use treachery_engine::prelude::*;

use crate::actions::QueryError;
use crate::build::{self, ResolvedViewer, Seating, Viewer};
use crate::describe::{default_describer, Describer};
use crate::schema::{DraftResult, GameView, ReportLines, RichText};

pub struct ViewReplica {
    pub game: Game,
    /// The applicable event types of each viewer (by user id) in the last view built for them.
    /// A draft query must be for one of these.
    actions: BTreeMap<i32, Vec<GameEventType>>,
    /// The game's skin, for the text of log lines and prompts: `DefaultSkin.Default` until the
    /// server replaces it with the host's skin (`RequestSetSkin`).
    pub describer: Box<dyn Describer>,
    /// The bidding time limit in seconds, or -1, which the host sets with `SetTimer`. Server
    /// state, carried into `timing.timer`.
    pub timer: i32,
    /// The names of the seated users and the bot speed. Server state, like `timer`. The server
    /// also keeps `game.human_seats` up to date.
    pub seating: Seating,
}

impl ViewReplica {
    /// Replays a savegame in a game that bots play.
    pub fn load(savegame: &str) -> Result<ViewReplica, String> {
        let state: GameState = GameState::load(savegame).map_err(|e| e.message)?;
        Game::check_version(state.version)?;
        let mut game = Game::of_version(state.version);
        for (nr, e) in state.events.as_ref().ok_or("savegame without events")?.iter().enumerate() {
            e.initialize_game(&game);
            if let Some(m) = e.execute(&mut game, false, false) {
                return Err(format!("event {nr} ({}) failed: {:?}", e.event_type().name(), treachery_skin::default_skin().render(&m)));
            }
        }
        Ok(ViewReplica { game, actions: BTreeMap::new(), describer: default_describer(), timer: -1, seating: Seating::default() })
    }

    /// A replica of a game with no events yet, that bots play. Events follow with
    /// [`ViewReplica::apply`].
    pub fn new(version: i32) -> ViewReplica {
        ViewReplica { game: Game::of_version(version), actions: BTreeMap::new(), describer: default_describer(), timer: -1, seating: Seating::default() }
    }

    /// The game of the web client's `/scene` route: a savegame replayed with one user, id 1 and
    /// named "Scene", seated as `faction` and made host. The other seats have no user, so they
    /// count as bots.
    pub fn scene(savegame: &str, faction: Faction) -> Result<(ViewReplica, Viewer), String> {
        const SCENE_USER_ID: i32 = 1;
        let mut replica = ViewReplica::load(savegame)?;
        let player = replica.game.get_player(Some(faction)).ok_or_else(|| format!("{faction} is not in this game"))?;
        let seat = replica.game.player(player).seat;
        replica.game.human_seats.insert(seat);
        replica.seating.names.insert(seat, "Scene".to_string());
        replica.seating.seated_users = 1;
        Ok((replica, Viewer { user_id: SCENE_USER_ID, seat, is_host: true, is_observer: false }))
    }

    /// Executes an event that the server has accepted, without validating it again.
    pub fn apply(&mut self, event_json: &str) -> Result<(), String> {
        let e: GameEvent = json::deserialize(event_json).map_err(|e| format!("{e:?}"))?;
        e.initialize_game(&self.game);
        match e.execute(&mut self.game, false, false) {
            None => Ok(()),
            Some(m) => Err(format!("{:?}", treachery_skin::default_skin().render(&m))),
        }
    }

    /// Builds the view of `viewer` and remembers their applicable actions for queries.
    pub fn view(&mut self, viewer: Viewer) -> GameView {
        let view = build::build_view(&mut self.game, viewer, &self.seating, self.describer.as_ref(), self.timer);
        let types = view.actions.iter().filter_map(|a| GameEventType::ALL.iter().copied().find(|t| t.name() == a.event_type)).collect();
        self.actions.insert(viewer.user_id, types);
        view
    }

    /// Builds the view of a viewer resolved by the caller, for games whose participation seats
    /// no users (the bot-game traces). The server uses [`ViewReplica::view`].
    pub fn view_as(&mut self, v: &ResolvedViewer) -> GameView {
        build::build_view_resolved(&mut self.game, v, &self.seating, self.describer.as_ref(), self.timer)
    }

    /// A draft query for `viewer`. Refused unless the draft's type was among the actions of the
    /// last view built for them.
    pub fn query(&self, viewer: Viewer, draft_json: &str) -> Result<DraftResult, QueryError> {
        let applicable = self.actions.get(&viewer.user_id).map(Vec::as_slice).unwrap_or(&[]);
        crate::actions::query(&self.game, viewer, &self.seating, applicable, draft_json, &crate::describe::InGame::new(self.describer.as_ref(), &self.game))
    }

    /// Whether a view was built for this user since the replica was made. A draft query needs
    /// one: it is checked against the actions of that view.
    pub fn has_view_for(&self, viewer: Viewer) -> bool {
        self.actions.contains_key(&viewer.user_id)
    }

    /// `RequestReport`: the lines of report `index` that `viewer` may read, as in the view's log.
    pub fn report(&self, viewer: Viewer, index: usize) -> Option<ReportLines> {
        let r = self.game.reports().get(index)?;
        let v = viewer.resolve(&self.game);
        Some(build::report_lines(index, r, &v, &crate::describe::InGame::new(self.describer.as_ref(), &self.game)))
    }

    /// `RequestUndoList`: the short message of every event, for the undo panel. The number of an
    /// entry is its index in the history, the `untilEventNr` of `RequestUndo`.
    pub fn undo_list(&self) -> Vec<UndoEntry> {
        let d = crate::describe::InGame::new(self.describer.as_ref(), &self.game);
        self.game
            .history.entries
            .iter()
            .enumerate()
            .map(|(i, e)| {
                // Moment n - 1 holds the turn and phase event n happened in.
                let moment = i.checked_sub(1).and_then(|m| self.game.history.moments.get(m));
                UndoEntry {
                    event_number: i as i32,
                    text: crate::expr::message(&e.get_short_message(&self.game), &d),
                    listed: !matches!(e, GameEvent::AllyPermission(_) | GameEvent::BattleRevision(_) | GameEvent::AutomationConfigured(_)),
                    turn: moment.map_or(0, |m| m.turn),
                    phase: moment.map_or_else(|| "None".to_string(), |m| m.phase.to_string()),
                }
            })
            .collect()
    }

    /// `RequestUndoPreview`: the view `viewer` had at gap `gap`, the moment just before event
    /// `gap`. The gap is clamped to `1..=` the number of events. The game is replayed from this
    /// replica's history into a new game with the same human seats, so this replica is not
    /// touched. A preview can't be acted on, so the view has no actions and no card plays.
    pub fn preview(&self, viewer: Viewer, gap: i32) -> Result<GameView, String> {
        let count = self.game.history.entries.len() as i32;
        let gap = gap.clamp(1.min(count), count) as usize;
        let mut game = self.game.new_like();
        for (nr, e) in self.game.history.entries.iter().take(gap).enumerate() {
            // A copy of the event, so the replica's history stays bound to the replica's game.
            let copy: GameEvent = json::deserialize(&json::serialize(e)).map_err(|e| format!("event {nr}: {e:?}"))?;
            copy.initialize_game(&game);
            if let Some(m) = copy.execute(&mut game, false, false) {
                return Err(format!("event {nr} ({}) failed: {:?}", copy.event_type().name(), treachery_skin::default_skin().render(&m)));
            }
        }
        let mut view = build::build_view(&mut game, viewer, &self.seating, self.describer.as_ref(), -1);
        view.actions.clear();
        if let Some(me) = &mut view.me {
            for c in &mut me.hand {
                c.playable_with.clear();
            }
        }
        Ok(view)
    }

    /// `RequestSetSkin` and a skin given with a created or loaded game: the skin loaded over the
    /// default skin. A skin that doesn't load leaves the current one.
    pub fn set_skin(&mut self, skin_json: &str) -> Result<(), String> {
        let skin = treachery_skin::Skin::load(skin_json, treachery_skin::default_skin()).map_err(|e| e.to_string())?;
        self.describer = Box::new(skin);
        Ok(())
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
    pub turn: i32,
    #[ts(type = "MainPhase")]
    pub phase: String,
}
