//! Builds a [`GameView`] for one viewer from the server's game.
//!
//! The rule for hidden information is in `rust/docs/protocol.md` (Game view, Visibility): a
//! field is filled only for the viewers who may see it under the game rules.

use std::collections::BTreeMap;


use treachery_engine::game::GameStatus;
use treachery_engine::prelude::*;

use crate::actions;
use crate::board;
use crate::shield;
use crate::skin_texts::FactionSheetCache;
use crate::describe::Describer;
use crate::expr::{self, message};
pub use crate::log::report_lines;
use crate::schema::*;

/// Who a view is for, as the server knows them: a user (the server's user id), their seat if
/// they have one, and whether they are a host or an observer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewer {
    pub user_id: i64,
    pub seat: Option<Seat>,
    pub is_host: bool,
    pub is_observer: bool,
}

/// The viewer resolved against one game.
#[derive(Clone, Copy, Debug)]
pub struct ResolvedViewer {
    pub player: Option<PlayerId>,
    pub faction: Option<Faction>,
    pub is_host: bool,
    pub is_observer: bool,
}

impl Viewer {
    pub fn resolve(self, game: &Game) -> ResolvedViewer {
        let player = self.seat.and_then(|s| game.get_player_by_seat(s));
        ResolvedViewer { player, faction: player.and_then(|p| game.player(p).faction), is_host: self.is_host, is_observer: self.is_observer }
    }
}

/// What the views show of the game's seating, which the server keeps: who sits where, and how
/// fast the bots play. Which seats are bots' is in the game (`Game::human_seats`).
#[derive(Clone, Debug)]
pub struct Seating {
    /// The name of the user in each seat, of the first one when two share a seat.
    pub names: BTreeMap<Seat, String>,
    /// How many users hold a seat or wait for one.
    pub seated_users: i32,
    pub bots_speed: i32,
}

impl Default for Seating {
    /// No users, and bots at speed 1.
    fn default() -> Self {
        Seating { names: BTreeMap::new(), seated_users: 0, bots_speed: 1 }
    }
}

impl Seating {
    /// "Bot" for a bot, else the name of the user in the player's seat ("?" if it has none).
    pub fn player_name(&self, game: &Game, p: PlayerId) -> String {
        if game.is_bot(p) {
            "Bot".to_string()
        } else {
            game.player(p).seat.and_then(|s| self.names.get(&s)).cloned().unwrap_or_else(|| "?".to_string())
        }
    }
}

fn name<T: ToString>(v: T) -> String {
    v.to_string()
}

/// Builds the view. `timer` is the bidding time limit the host set (-1 for none).
pub fn build_view(game: &Game, viewer: Viewer, s: &Seating, d: &dyn Describer, timer: i32) -> GameView {
    let v = viewer.resolve(game);
    build_view_resolved(game, &v, s, d, timer)
}

/// [`build_view`] for a viewer already resolved against the game. Tests use it to look at games
/// without seated users (the bot-game corpus).
pub fn build_view_resolved(game: &Game, v: &ResolvedViewer, s: &Seating, d: &dyn Describer, timer: i32) -> GameView {
    build_view_cached(game, v, s, d, timer, &mut FactionSheetCache::default())
}

/// [`build_view_resolved`] with the faction sheets taken from `sheets` when they haven't changed.
/// A [`crate::cache::ViewCache`] keeps the sheets between views.
pub(crate) fn build_view_cached(game: &Game, v: &ResolvedViewer, s: &Seating, d: &dyn Describer, timer: i32, sheets: &mut FactionSheetCache) -> GameView {
    let status = GameStatus::determine_status(game, v.player, !v.is_observer);
    let action_types = game.get_applicable_events(v.player, v.is_host);

    // Rich text reads live collections and leader skills from the game.
    let in_game = crate::describe::InGame::new(d, game);
    let d: &dyn Describer = &in_game;
    let me = v.player.filter(|_| !v.is_observer);
    GameView {
        event_number: game.event_count(),
        version: crate::wire::shown_version(game),
        turn: game.turn.current_turn,
        maximum_turns: game.maximum_turns(),
        maximum_players: game.maximum_players(),
        main_phase: name(game.turn.current_main_phase),
        phase: name(game.turn.current_phase),
        viewer: ViewerInfo { faction: v.player.map(|_| name_or_absent(v.faction)), is_host: v.is_host, is_observer: v.is_observer },
        players: game.player_indices().map(|p| public_player(game, s, p)).collect(),
        me: me.map(|p| shield::private_player(game, p, &action_types)),
        board: board::board(game, &status, v),
        status: status_view(game, &status, v, d),
        actions: action_types.iter().map(|t| actions::action_view(game, v, s, *t, d)).collect(),
        waiting_actions: me.map_or_else(Vec::new, |p| waiting_actions(game, p, d)),
        log: crate::log::log_view(game, v, d),
        ruleset: name_or_absent(game.rules.ruleset),
        rules: game.rules.active.iter().map(|r| name(*r)).collect(),
        rules_for_bots: game.rules.rules_for_bots.iter().map(|r| name(*r)).collect(),
        economics_status: name_or_absent(game.charity.economics_status),
        piles: board::piles(game, v),
        tanks: board::tanks(game, v),
        bidding: board::bidding(game, v),
        battle: board::battle(game, v),
        notes: me.and_then(|p| shield::notes(game, p)),
        timing: board::timing(game, timer),
        deals: crate::side::deals(game, v, d),
        bots_speed: s.bots_speed,
        tracker: crate::side::tracker(game, v),
        mentat: crate::side::mentat(game),
        ending: crate::ending::ending(game, s, d),
        faction_sheets: sheets.get(game, d),
        rule_labels: crate::skin_texts::rule_labels(game, d),
        recent_milestones: crate::skin_texts::recent_milestones(game),
    }
}

fn public_player(game: &Game, s: &Seating, p: PlayerId) -> PublicPlayer {
    let pl = game.player(p);
    let mut out = PublicPlayer {
        faction: name_or_absent(pl.faction),
        seat: pl.seated(),
        name: s.player_name(game, p),
        is_bot: game.is_bot(p),
        ally: name_or_absent(pl.ally),
        forces_in_reserve: pl.forces_in_reserve(),
        special_forces_in_reserve: pl.special_forces_in_reserve(),
        forces_killed: i32::from(pl.forces_killed),
        special_forces_killed: i32::from(pl.special_forces_killed),
        card_count: pl.treachery_cards.len() as i32,
        // Public: the map ring shows these for everyone.
        bribes: i32::from(pl.bribes),
        resources_after_bidding: i32::from(pl.resources_after_bidding),
        extortion: i32::from(pl.extortion),
        banked_resources: i32::from(pl.banked_resources),
        tech_tokens: pl.tech_tokens.iter().map(|t| name(*t)).collect(),
        // Ecaz's unplaced ambassadors are on the map ring for everyone.
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
        waiting_for: status.waiting_for_players.iter().flatten().map(|p| name_or_absent(game.player(*p).faction)).collect(),
        waiting_in_sequence: status
            .waiting_in_sequence
            .iter()
            .filter_map(|se| se.player.map(|p| SequenceEntry { faction: name_or_absent(game.player(p).faction), has_turn: se.has_turn }))
            .collect(),
        timed_event_time: status.timed_event.as_ref().map(|e| e.at.to_json_string()),
    }
}

/// The event's validation message for the submitter `by`, rendered, with "Validation failed"
/// when validating panics.
pub fn validate(game: &Game, by: Option<Faction>, event: Event, d: &dyn Describer) -> Option<RichText> {
    match draft_check(|| event.validate(game, By::new(game, by))) {
        Some(m) => m.err().map(|r| expr::message(&r.0, d)),
        None => Some(vec![Part::Text { v: "Validation failed".to_string() }]),
    }
}

/// Runs engine code on a form's draft, which may be missing values the code expects. `None` if
/// the code panicked.
pub fn draft_check<R>(f: impl FnOnce() -> R) -> Option<R> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok()
}

/// Wraps a freshly built event for [`validate`].
pub fn event<E>(e: E) -> Event
where
    Event: From<E>,
{
    Event::from(e)
}

/// Prescience waits while the Bene Gesserit (or their ally) may still use Voice, so a player who
/// could otherwise use it sees why its form is missing.
fn waiting_actions(game: &Game, p: PlayerId, d: &dyn Describer) -> Vec<WaitingActionView> {
    let mut result = vec![];
    if Prescience::prescience_right(game, p) && Voice::voice_pending(game) {
        // The Bene Gesserit, and their ally when they share Voice: either may decide.
        let mut deciders: Vec<Faction> =
            game.player_indices().filter(|&q| Voice::voice_right(game, Some(q)) == Some(VoiceRight::Open)).filter_map(|q| game.player(q).faction).collect();
        deciders.sort_by_key(|&f| f != Faction::BeneGesserit);
        let reason = match deciders.as_slice() {
            [a, b] => treachery_engine::express!("Available once ", *a, " or ", *b, " use or decline Voice."),
            [a, ..] => treachery_engine::express!("Available once ", *a, " use or decline Voice."),
            [] => return result,
        };
        result.push(WaitingActionView { event_type: "Prescience".into(), title: "Use Prescience".into(), reason: message(&reason, d) });
    }
    result
}
