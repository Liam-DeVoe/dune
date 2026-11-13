//! Action options: for each applicable event type, the choices its form offers and the
//! validation of the event the form would send.
//!
//! The builders are split by area into the submodules. A builder gets the draft event (the
//! form's current inputs): the default draft for the view, or the client's draft for a query
//! (protocol.md, Dependent inputs). Choices that depend on other inputs are computed from the
//! draft.

use treachery_engine::events::GameEventType;
use treachery_engine::json;
use treachery_engine::prelude::*;

use crate::build::{event, validate, ResolvedViewer, Viewer};
use crate::describe::Describer;
use crate::expr::message;
use crate::schema::*;

pub mod revival;

pub mod ship_move;

pub mod bidding;
pub mod battle;
pub mod other;
mod other_nexus;
mod widgets;
pub mod setup;

fn name<T: ToString>(v: T) -> String {
    v.to_string()
}

fn text(s: &str) -> RichText {
    vec![Part::Text { v: s.to_string() }]
}

fn simple(title: RichText, confirm_label: &str, alert: bool) -> ActionOptions {
    ActionOptions::Simple { title: Some(title), confirm_label: confirm_label.to_string(), alert, allow_auto_confirm: false }
}

/// The entry of `GameView.actions` for one applicable event type, with the form's defaults.
pub fn action_view(game: &Game, v: &ResolvedViewer, t: GameEventType, d: &dyn Describer) -> ActionView {
    let Some(faction) = v.faction.or_else(|| setup::seatless_host_may_act(v, t).then_some(Faction::None)) else {
        return ActionView { event_type: t.name().to_string(), options: None, validation: None };
    };
    match default_draft(game, v, faction, t) {
        Some(draft) => {
            let (options, validation) = options_and_validation(game, v, &draft, d);
            ActionView { event_type: t.name().to_string(), options, validation }
        }
        None => ActionView { event_type: t.name().to_string(), options: None, validation: None },
    }
}

/// The event a form sends before the player touches it: the form's starting inputs,
/// after its widgets have clamped them (a number picker clamps its value into its range).
/// Public for the observable-behavior harness (`rust/observe`), which fuzzes from these drafts.
pub fn default_draft(game: &Game, _v: &ResolvedViewer, faction: Faction, t: GameEventType) -> Option<GameEvent> {
    if revival::TYPES.contains(&t) { return revival::default_draft(game, faction, t); }
    if setup::TYPES.contains(&t) { return setup::default_draft(game, faction, t); }
    if let Some(e) = _v.player.and_then(|p| ship_move::default_draft(game, p, faction, t)) { return Some(e); }
    if let Some(d) = battle::default_draft(game, _v, faction, t) { return d; }
    if let Some(e) = other::default_draft(game, faction, t) { return Some(e); }
    Some(match t {
        GameEventType::AllianceOffered => {
            // A required image choice starts at the first target.
            let mut e = AllianceOffered::new(game, faction);
            if let Some(p) = _v.player {
                e.target = AllianceOffered::valid_targets(game, p).next().unwrap_or(Faction::None);
            }
            event(e)
        }
        GameEventType::AlliancePassed => event(AlliancePassed::new(game, faction)),
        GameEventType::AllianceBroken => event(AllianceBroken::new(game, faction)),
        GameEventType::EndPhase => event(EndPhase::new(game, faction)),
        GameEventType::StormDialled => {
            let mut e = StormDialled::new(game, faction);
            e.amount = 0i32.clamp(StormDialled::valid_min_amount(game), StormDialled::valid_max_amount(game).max(StormDialled::valid_min_amount(game)));
            event(e)
        }
        GameEventType::Bid | GameEventType::BlackMarketBid => return bidding::default_draft(game, faction, t),
        _ => return None,
    })
}

/// The options of the form for `draft`, and the validation of `draft` itself.
fn options_and_validation(game: &Game, v: &ResolvedViewer, draft: &GameEvent, d: &dyn Describer) -> (Option<ActionOptions>, Option<RichText>) {
    if let Some(r) = revival::options_and_validation(game, v, draft, d) { return r; }
    if let Some(r) = setup::options_and_validation(game, v, draft, d) { return r; }
    if let Some(r) = v.player.and_then(|p| ship_move::options_and_validation(game, p, draft, d)) { return r; }
    if let Some(r) = battle::options_and_validation(game, v, draft, d) { return r; }
    if let Some(r) = other::options_and_validation(game, v, draft, d) { return r; }
    let options = match draft {
        GameEvent::AllianceOffered(_) => {
            let me = v.player.expect("an applicable event has a player");
            Some(ActionOptions::AllianceOffered { targets: AllianceOffered::valid_targets(game, me).map(name).collect() })
        }
        // AlliancePassed asks to keep the alliance when there is one.
        GameEvent::AlliancePassed(_) => {
            let me = v.player.expect("an applicable event has a player");
            let ally = game.player(me).ally;
            Some(if game.player(me).has_ally() {
                simple(message(&treachery_engine::express!("Keep your alliance with ", ally, "?"), d), "Keep alliance", true)
            } else {
                simple(text("Done making alliances?"), "Pass", true)
            })
        }
        GameEvent::AllianceBroken(_) => Some(simple(text("Break your current alliance?"), "Break", false)),
        // The prompt holds EndPhase's "Continue" button, with the event's validation, when
        // EndPhase is applicable.
        GameEvent::EndPhase(_) => Some(ActionOptions::Simple { title: None, confirm_label: "Continue".to_string(), alert: false, allow_auto_confirm: false }),
        GameEvent::StormDialled(_) => Some(ActionOptions::StormDialled {
            min: StormDialled::valid_min_amount(game),
            max: StormDialled::valid_max_amount(game),
        }),
        GameEvent::Bid(_) | GameEvent::BlackMarketBid(_) => bidding::options(game, v, draft),
        _ => None,
    };
    (options, validate(game, draft.clone(), d))
}

/// Why a draft query was refused.
#[derive(Debug, PartialEq)]
pub enum QueryError {
    /// The draft is not valid event JSON.
    BadJson(String),
    /// The viewer holds no seat.
    NotAPlayer,
    /// The draft's event type is not applicable for the viewer now.
    NotApplicable,
}

/// A draft query (protocol.md, Dependent inputs): the options and validation of a form for the
/// inputs the player has chosen so far. `draft_json` is the event in savegame JSON. Its
/// `Initiator` is replaced by the viewer's faction, as the server does on submission.
/// `applicable` is the viewer's applicable event types from the view they are looking at.
pub fn query(game: &Game, viewer: Viewer, applicable: &[GameEventType], draft_json: &str, d: &dyn Describer) -> Result<DraftResult, QueryError> {
    let v = viewer.resolve(game);
    let draft: GameEvent = json::deserialize(draft_json).map_err(|e| QueryError::BadJson(format!("{e:?}")))?;
    let seatless_host = setup::seatless_host_may_act(&v, draft.event_type()).then_some(Faction::None);
    let faction = v.faction.filter(|_| !v.is_observer).or(seatless_host).ok_or(QueryError::NotAPlayer)?;
    if !applicable.contains(&draft.event_type()) {
        return Err(QueryError::NotApplicable);
    }
    draft.initialize(game, faction);
    let (options, validation) = options_and_validation(game, &v, &draft, d);
    Ok(DraftResult { event_type: draft.event_type().name().to_string(), options, validation })
}
