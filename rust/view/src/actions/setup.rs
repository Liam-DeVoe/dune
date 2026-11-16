//! The setup forms, `EstablishPlayers` and `PerformSetup`. Both are host actions, so a host
//! without a seat acts as `None`, as every host does before factions exist.

use treachery_engine::events::EventType;
use treachery_engine::prelude::*;

use crate::build::{event, validate, ResolvedViewer, Seating};
use crate::describe::Describer;
use crate::schema::*;

/// The event types built here.
pub const TYPES: [EventType; 2] = [EventType::EstablishPlayers, EventType::PerformSetup];

/// Whether a host without a seat may act on `t` (as `Faction.None`).
pub fn seatless_host_may_act(v: &ResolvedViewer, t: EventType) -> bool {
    v.is_host && t == EventType::EstablishPlayers
}

/// The game settings the form starts from: six players, ten turns, every faction, the basic
/// rules.
fn initial_settings() -> GameSettings {
    let mut s = GameSettings::default();
    s.initial_rules = Some(Game::ruleset_definition().iter().find(|(r, _)| **r == Ruleset::BasicGame).map(|(_, rules)| rules.clone()).unwrap_or_default());
    s.allowed_factions_in_play = Some(Faction::ALL.to_vec());
    s
}

pub fn default_draft(_game: &Game, _faction: Option<Faction>, t: EventType) -> Option<Event> {
    Some(match t {
        EventType::EstablishPlayers => {
            let mut e = EstablishPlayers::default();
            e.settings = Some(initial_settings());
            event(e)
        }
        // No forces staged, no spice.
        EventType::PerformSetup => event(PerformSetup::default()),
        _ => return None,
    })
}

pub fn options_and_validation(game: &Game, by: Option<Faction>, s: &Seating, draft: &Event, d: &dyn Describer) -> Option<(Option<ActionOptions>, Option<RichText>)> {
    let options = match draft {
        Event::EstablishPlayers(_) => ActionOptions::EstablishPlayers(EstablishPlayersOptions { seated_players: s.seated_users }),
        Event::PerformSetup(_) => {
            let faction = game.next_faction_to_perform_custom_setup();
            let p = game.get_player(faction)?;
            let placer = game.player(p);
            PerformSetupOptions {
                faction: name_or_absent(faction),
                forces_in_reserve: placer.forces_in_reserve(),
                special_forces_in_reserve: placer.special_forces_in_reserve(),
                // Richese's special forces are No-Field tokens, which are not placed this way.
                may_stage_special: placer.has_special_forces(game) && !placer.is(Faction::Richese),
                locations: game.board.map.locations(false).map(|l| l.id()).collect(),
                max_resources: 100,
            }
            .into()
        }
        _ => return None,
    };
    Some((Some(options), validate(game, by, draft.clone(), d)))
}

impl From<PerformSetupOptions> for ActionOptions {
    fn from(o: PerformSetupOptions) -> Self {
        ActionOptions::PerformSetup(o)
    }
}
