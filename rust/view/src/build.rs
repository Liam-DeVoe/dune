//! Builds a [`GameView`] for one viewer from the view replica (see `replica.rs`).
//!
//! Every field says which C# client code it stands for. The rule for hidden information is in
//! `rust/docs/protocol.md` (Game view, Visibility): a field is filled only for the viewers the
//! C# client shows it to, and never for a viewer the game rules hide it from.

use std::rc::Rc;

use treachery_engine::compat::cs;
use treachery_engine::game::GameStatus;
use treachery_engine::prelude::*;

use crate::actions;
use crate::board;
use crate::shield;
use crate::describe::Describer;
use crate::expr::{self, message};
pub use crate::log::report_lines;
use crate::schema::*;

/// Who a view is for, as the server knows them: a user id, resolved against the game's
/// participation the way `Client.cs` does (`Player`, `IsObserver`, `IsHost`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewer {
    pub user_id: i32,
}

/// The viewer resolved against one game.
#[derive(Clone, Copy, Debug)]
pub struct ResolvedViewer {
    pub player: Option<PlayerIndex>,
    pub faction: Option<Faction>,
    pub is_host: bool,
    pub is_observer: bool,
}

impl Viewer {
    pub fn resolve(self, game: &Game) -> ResolvedViewer {
        let player = game.get_player_by_user_id(Some(self.user_id));
        ResolvedViewer {
            player,
            faction: player.map(|p| game.player(p).faction),
            is_host: game.is_host(Some(self.user_id)),
            is_observer: game.is_observer(Some(self.user_id)),
        }
    }
}

fn name<T: ToString>(v: T) -> String {
    v.to_string()
}

/// Builds the view. Takes `&mut Game` because `GameStatus.DetermineStatus` and
/// `GetApplicableEvents` write battle-sequence state (hazards 4.3), as they do in each C#
/// client's own copy of the game. Call it on the view replica, never on the server's game.
/// `timer` is the bidding time limit the host set (`Client.Timer`, -1 for none).
pub fn build_view(game: &mut Game, viewer: Viewer, d: &dyn Describer, timer: i32) -> GameView {
    let v = viewer.resolve(game);
    build_view_resolved(game, &v, d, timer)
}

/// [`build_view`] for a viewer already resolved against the game's participation. Tests use it
/// to look at games without seated users (the bot-game corpus).
pub fn build_view_resolved(game: &mut Game, v: &ResolvedViewer, d: &dyn Describer, timer: i32) -> GameView {
    // Client.PerformPostEventTasks: Status, then Actions.
    let status = GameStatus::determine_status(game, v.player, !v.is_observer);
    let action_types = game.get_applicable_events(v.player, v.is_host);

    let game: &Game = game;
    // ExpressionComponent's `Game` parameter: live collections and leader skills.
    let in_game = crate::describe::InGame::new(d, game);
    let d: &dyn Describer = &in_game;
    let me = v.player.filter(|_| !v.is_observer);
    GameView {
        event_number: game.event_count(),
        version: game.version,
        turn: game.current_turn,
        maximum_turns: game.maximum_turns(),
        maximum_players: game.maximum_players(),
        main_phase: name(game.current_main_phase),
        phase: name(game.current_phase),
        viewer: ViewerInfo { faction: v.faction.map(name), is_host: v.is_host, is_observer: v.is_observer },
        players: game.player_indices().map(|p| public_player(game, p)).collect(),
        me: me.map(|p| shield::private_player(game, p, &action_types)),
        board: board::board(game, &status, v),
        status: status_view(game, &status, v, d),
        actions: action_types.iter().map(|t| actions::action_view(game, v, *t, d)).collect(),
        log: crate::log::log_view(game, v, d),
        ruleset: name(game.ruleset),
        rules: game.rules.iter().map(|r| name(*r)).collect(),
        rules_for_bots: game.rules_for_bots.iter().map(|r| name(*r)).collect(),
        economics_status: name(game.economics_status),
        piles: board::piles(game, v),
        tanks: board::tanks(game, v),
        bidding: board::bidding(game, v),
        battle: board::battle(game, v),
        notes: me.and_then(|p| shield::notes(game, p)),
        timing: board::timing(game, timer),
        deals: crate::side::deals(game, v, d),
        bots_speed: crate::side::bots_speed(game),
        tracker: crate::side::tracker(game, v),
        mentat: crate::side::mentat(game),
        ending: crate::ending::ending(game, d),
        faction_sheets: crate::skin_texts::faction_sheets(game, d),
        rule_labels: crate::skin_texts::rule_labels(game, d),
        recent_milestones: crate::skin_texts::recent_milestones(game),
    }
}

fn public_player(game: &Game, p: PlayerIndex) -> PublicPlayer {
    let pl = game.player(p);
    let mut out = PublicPlayer {
        faction: name(pl.faction),
        seat: pl.seat,
        name: game.get_player_name_player(p),
        is_bot: game.is_bot(p),
        ally: name(pl.ally),
        forces_in_reserve: pl.forces_in_reserve(),
        special_forces_in_reserve: pl.special_forces_in_reserve(),
        forces_killed: pl.forces_killed,
        special_forces_killed: pl.special_forces_killed,
        card_count: pl.treachery_cards.len() as i32,
        // Players.razor:195-234 draws these on the map ring for everyone.
        bribes: pl.bribes,
        resources_after_bidding: pl.resources_after_bidding,
        extortion: pl.extortion,
        banked_resources: pl.banked_resources,
        tech_tokens: pl.tech_tokens.iter().map(|t| name(*t)).collect(),
        // Players.razor:160-176 draws Ecaz's unplaced ambassadors for everyone. Kept, as C#
        // (client-plan.md, coordinator answer 2).
        ambassadors: pl.ambassadors.iter().map(|a| name(*a)).collect(),
        has_special_forces: false,
        has_room_for_cards: false,
        cards_owned: 0,
        homeworlds: vec![],
        skilled_leaders: vec![],
        loyal_leader: None,
        latest_revealed_no_field: None,
    };
    board::ring_extras(game, p, &mut out);
    out
}

fn status_view(game: &Game, status: &GameStatus, v: &ResolvedViewer, d: &dyn Describer) -> StatusView {
    let waiting_for_me = v.player.is_some_and(|p| status.waiting_for_me(p, v.is_host));
    // GameStatus.GetMessage(Player, IsHost); an observer is never awaited.
    let text = match v.player {
        Some(p) => status.get_message(p, v.is_host),
        None => status.description_when_waiting.clone(),
    };
    StatusView {
        message: message(&text, d),
        flash: board::flash(&status.flash_info, d),
        to_bid: board::to_bid(game),
        waiting_for_me,
        waiting_for_host: status.waiting_for_host,
        waiting_for: status.waiting_for_players.iter().flatten().map(|p| name(game.player(*p).faction)).collect(),
        waiting_in_sequence: status
            .waiting_in_sequence
            .iter()
            .filter_map(|se| se.player.map(|p| SequenceEntry { faction: name(game.player(p).faction), has_turn: se.has_turn }))
            .collect(),
        timed_event_time: status.timed_event.as_ref().map(|e| e.time().to_json_string()),
    }
}

/// `GameEventComponent.Validation`: `ConfirmedResult.Validate()`, with "Validation failed" when
/// it throws.
pub fn validate(game: &Game, event: GameEvent, d: &dyn Describer) -> Option<RichText> {
    match cs::catch(|| event.validate(game)) {
        Ok(m) => m.map(|m| expr::message(&m, d)),
        Err(_) => Some(vec![Part::Text { v: "Validation failed".to_string() }]),
    }
}

/// Wraps a freshly built event for [`validate`].
pub fn event<E>(e: E) -> GameEvent
where
    GameEvent: From<Rc<E>>,
{
    GameEvent::from(Rc::new(e))
}
