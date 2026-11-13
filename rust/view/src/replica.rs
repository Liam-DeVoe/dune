//! The view replica: a second copy of a game that only the view code touches.
//!
//! It plays the part of the C# client's own `Game`. It executes each event the way
//! `Client.HandleGameEvent` does (`Execute(false, false)`, no validation), and after each event
//! every view is built from it, which runs `DetermineStatus` and `GetApplicableEvents` as
//! `Client.PerformPostEventTasks` did. The state those calls write (the battle sequence, the
//! neighbour cache) therefore evolves as in a C# client, and never reaches the server's
//! authoritative game, whose state must stay equal to the C# server's.
//!
//! One replica serves every viewer of a game. In C# each client had its own copy and called
//! the two functions once per event; here they run once per viewer per event. That is
//! equivalent because the only write they make that changes a later result is
//! `NextPlayerToBattle` advancing the battle sequence to the next player with a battle, after
//! which further reads return the same player without advancing (client-plan.md, The view
//! replica).

use std::collections::BTreeMap;

use treachery_engine::compat::cs::Shared;
use treachery_engine::events::GameEventType;
use treachery_engine::game::GameState;
use treachery_engine::json;
use treachery_engine::messages::message::Message;
use treachery_engine::game::Participation;
use treachery_engine::prelude::*;

use crate::actions::QueryError;
use crate::build::{self, ResolvedViewer, Viewer};
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
    /// The bidding time limit in seconds, or -1: `Client.Timer`, which the C# hub's `SetTimer`
    /// sets for every client of the game. Server state, carried into `timing.timer`.
    pub timer: i32,
}

impl ViewReplica {
    /// Replays a savegame. `participation` is shared with the server's game, as in C#, where
    /// participation changes reach every copy through the hub callbacks.
    pub fn load(savegame: &str, participation: &Shared<Participation>) -> Result<ViewReplica, String> {
        let state: GameState = GameState::load(savegame).map_err(|e| e.message)?;
        Game::check_version(state.version)?;
        let mut game = Game::new_int32_participation(state.version, participation);
        for (nr, e) in state.events.as_ref().ok_or("savegame without events")?.iter().enumerate() {
            e.initialize_game(&game);
            if let Some(m) = e.execute(&mut game, false, false) {
                return Err(format!("event {nr} ({}) failed: {:?}", e.event_type().name(), treachery_skin::default_skin().message_to_string(&m)));
            }
        }
        Ok(ViewReplica { game, actions: BTreeMap::new(), describer: default_describer(), timer: -1 })
    }

    /// A replica of a game with no events yet, as `new Game(version, participation)`. Events
    /// follow with [`ViewReplica::apply`].
    pub fn new(version: i32, participation: &Shared<Participation>) -> ViewReplica {
        ViewReplica { game: Game::new_int32_participation(version, participation), actions: BTreeMap::new(), describer: default_describer(), timer: -1 }
    }

    /// The game of the C# `/scene` page (`SceneGameService.Load`): a savegame replayed with
    /// one user, id 1 and named "Scene", seated as `faction` and made host. The other seats
    /// have no user, so they count as bots. The Playwright harness uses this on both sides
    /// (client-plan.md, Comparing with the C# client).
    pub fn scene(savegame: &str, faction: Faction) -> Result<(ViewReplica, Viewer), String> {
        const SCENE_USER_ID: i32 = 1;
        let participation = Shared::new(Participation::default());
        let replica = ViewReplica::load(savegame, &participation)?;
        let player = replica.game.get_player(Some(faction)).ok_or_else(|| format!("{faction} is not in this game"))?;
        {
            let mut p = participation.borrow_mut();
            p.seated_players.add(SCENE_USER_ID, replica.game.player(player).seat);
            p.player_names.add(SCENE_USER_ID, "Scene".to_string());
            p.hosts.add(SCENE_USER_ID);
        }
        Ok((replica, Viewer { user_id: SCENE_USER_ID }))
    }

    /// Executes an event that the server has accepted, as `Client.HandleGameEvent` does.
    pub fn apply(&mut self, event_json: &str) -> Result<(), String> {
        let e: GameEvent = json::deserialize(event_json).map_err(|e| format!("{e:?}"))?;
        e.initialize_game(&self.game);
        match e.execute(&mut self.game, false, false) {
            None => Ok(()),
            Some(m) => Err(format!("{:?}", treachery_skin::default_skin().message_to_string(&m))),
        }
    }

    /// Builds the view of `viewer` and remembers their applicable actions for queries.
    pub fn view(&mut self, viewer: Viewer) -> GameView {
        let view = build::build_view(&mut self.game, viewer, self.describer.as_ref(), self.timer);
        let types = view.actions.iter().filter_map(|a| GameEventType::ALL.iter().copied().find(|t| t.name() == a.event_type)).collect();
        self.actions.insert(viewer.user_id, types);
        view
    }

    /// Builds the view of a viewer resolved by the caller, for games whose participation seats
    /// no users (the bot-game traces). The server uses [`ViewReplica::view`].
    pub fn view_as(&mut self, v: &ResolvedViewer) -> GameView {
        build::build_view_resolved(&mut self.game, v, self.describer.as_ref(), self.timer)
    }

    /// A draft query for `viewer`. Refused unless the draft's type was among the actions of the
    /// last view built for them.
    pub fn query(&self, viewer: Viewer, draft_json: &str) -> Result<DraftResult, QueryError> {
        let applicable = self.actions.get(&viewer.user_id).map(Vec::as_slice).unwrap_or(&[]);
        crate::actions::query(&self.game, viewer, applicable, draft_json, &crate::describe::InGame::new(self.describer.as_ref(), &self.game))
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

    /// `RequestUndoList`: `GetShortMessage` of every event, as `UndoComponent.razor` lists them.
    /// The number of an entry is its index in `History`, the `untilEventNr` of `RequestUndo`.
    pub fn undo_list(&self) -> Vec<UndoEntry> {
        let d = crate::describe::InGame::new(self.describer.as_ref(), &self.game);
        self.game
            .history
            .iter()
            .enumerate()
            .map(|(i, e)| {
                // UndoSession.MomentOf(n) = Moments[n - 1]: the turn and phase event n happened in.
                let moment = i.checked_sub(1).and_then(|m| self.game.moments.get(m));
                UndoEntry {
                    event_number: i as i32,
                    text: crate::expr::message(&e.get_short_message(&self.game), &d),
                    // UndoSession.IsListed.
                    listed: !matches!(e, GameEvent::AllyPermission(_) | GameEvent::BattleRevision(_) | GameEvent::AutomationConfigured(_)),
                    turn: moment.map_or(0, |m| m.turn),
                    phase: moment.map_or_else(|| "None".to_string(), |m| m.phase.to_string()),
                }
            })
            .collect()
    }

    /// `RequestUndoPreview`: the view `viewer` had at gap `gap`, the moment just before event
    /// `gap` (`UndoSession.MoveTo`). The gap is clamped to `1..=History.Count`, as in C#. The
    /// game is replayed from this replica's history into a new game with the same
    /// participation, so this replica is not touched. As in the C# client, which showed the
    /// preview with no actions (`IGameService.Actions` is empty while previewing), the view has
    /// no actions and no card plays.
    pub fn preview(&self, viewer: Viewer, gap: i32) -> Result<GameView, String> {
        let count = self.game.history.len() as i32;
        let gap = gap.clamp(1.min(count), count) as usize;
        let mut game = Game::new_int32_participation(self.game.version, &self.game.participation);
        for (nr, e) in self.game.history.iter().take(gap).enumerate() {
            // A copy of the event, so the replica's history stays bound to the replica's game.
            let copy: GameEvent = json::deserialize(&json::serialize(e)).map_err(|e| format!("event {nr}: {e:?}"))?;
            copy.initialize_game(&game);
            if let Some(m) = copy.execute(&mut game, false, false) {
                return Err(format!("event {nr} ({}) failed: {:?}", copy.event_type().name(), treachery_skin::default_skin().message_to_string(&m)));
            }
        }
        let mut view = build::build_view(&mut game, viewer, self.describer.as_ref(), -1);
        view.actions.clear();
        if let Some(me) = &mut view.me {
            for c in &mut me.hand {
                c.playable_with.clear();
            }
        }
        Ok(view)
    }

    /// `RequestSetSkin` and a skin given with a created or loaded game: the skin the C# client
    /// switched to (`Skin.Load(skin, DefaultSkin.Default)`). A skin that doesn't load leaves the
    /// current one, as the C# client kept its skin when loading threw.
    pub fn set_skin(&mut self, skin_json: &str) -> Result<(), String> {
        let skin = treachery_skin::Skin::load(skin_json, treachery_skin::default_skin()).map_err(|e| e.to_string())?;
        self.describer = Box::new(skin);
        Ok(())
    }

    pub fn last_message(&self) -> Option<&Message> {
        self.game.reports().last().and_then(|r| r.messages.last())
    }
}

/// One line of the undo list (`RequestUndoList`, protocol.md).
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoEntry {
    pub event_number: i32,
    pub text: RichText,
    /// `UndoSession.IsListed`: the undo panel leaves out ally permissions, battle revisions and
    /// automation settings.
    pub listed: bool,
    /// `UndoSession.MomentOf(eventNumber)`: the turn and main phase the event happened in (0 and
    /// "None" for event 0).
    pub turn: i32,
    pub phase: String,
}
