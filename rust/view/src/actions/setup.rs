//! The setup forms (client-plan.md, package 8): `EstablishPlayersComponent` and
//! `PerformSetupComponent`. Both are host actions (`Game.AddHostActions`), so a host without a
//! seat acts as `Faction.None`, as the C# client's host does before factions exist.

use treachery_engine::events::GameEventType;
use treachery_engine::prelude::*;

use crate::build::{event, validate, ResolvedViewer};
use crate::describe::Describer;
use crate::schema::*;

/// The event types built here.
pub const TYPES: [GameEventType; 2] = [GameEventType::EstablishPlayers, GameEventType::PerformSetup];

/// Whether a host without a seat may act on `t` (as `Faction.None`).
pub fn seatless_host_may_act(v: &ResolvedViewer, t: GameEventType) -> bool {
    v.is_host && t == GameEventType::EstablishPlayers
}

/// `EstablishPlayersComponent.Settings`'s initializer: six players, ten turns, every faction, the
/// basic rules.
fn initial_settings() -> GameSettings {
    let mut s = GameSettings::default();
    s.initial_rules = Some(Game::ruleset_definition().iter().find(|(r, _)| **r == Ruleset::BasicGame).map(|(_, rules)| rules.clone()).unwrap_or_default());
    s.allowed_factions_in_play = Some(Shared::new(get_values_except_default(Faction::None).collect()));
    s
}

pub fn default_draft(game: &Game, faction: Faction, t: GameEventType) -> Option<GameEvent> {
    Some(match t {
        GameEventType::EstablishPlayers => {
            let mut e = EstablishPlayers::new(game, faction);
            e.settings = Some(Shared::new(initial_settings()));
            event(e)
        }
        // PerformSetupComponent: no forces staged, `resources = 0`.
        GameEventType::PerformSetup => event(PerformSetup::new(game, faction)),
        _ => return None,
    })
}

pub fn options_and_validation(game: &Game, _v: &ResolvedViewer, draft: &GameEvent, d: &dyn Describer) -> Option<(Option<ActionOptions>, Option<RichText>)> {
    let options = match draft {
        GameEvent::EstablishPlayers(_) => ActionOptions::EstablishPlayers(EstablishPlayersOptions { seated_players: game.number_of_seated_players() }),
        GameEvent::PerformSetup(_) => {
            let faction = game.next_faction_to_perform_custom_setup();
            let p = game.get_player(Some(faction))?;
            let placer = game.player(p);
            PerformSetupOptions {
                faction: faction.to_string(),
                forces_in_reserve: placer.forces_in_reserve(),
                special_forces_in_reserve: placer.special_forces_in_reserve(),
                // Richese's special forces are No-Field tokens, which are not placed this way.
                may_stage_special: placer.has_special_forces(game) && !placer.is(Faction::Richese),
                locations: game.map.locations(false).map(|l| l.id()).collect(),
                max_resources: 100,
            }
            .into()
        }
        _ => return None,
    };
    Some((Some(options), validate(game, draft.clone(), d)))
}

impl From<PerformSetupOptions> for ActionOptions {
    fn from(o: PerformSetupOptions) -> Self {
        ActionOptions::PerformSetup(o)
    }
}
