//! Payloads of the setup forms, built in `actions/setup.rs`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The `EstablishPlayers` form: the host's game settings. Its choices are static and the same as
/// the lobby's schedule form (factions, rulesets, rule groups, the maximum players and turns), so
/// they are in `web/public/static/lobby.json` (`lobby.rs`). The draft's validation is what the
/// form needs from the game.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EstablishPlayersOptions {
    /// `Game.Participation.SeatedPlayers.Count`: the server refuses to start with fewer seated
    /// users than `NumberOfPlayers`, because empty seats would become bots (protocol.md, Bots).
    pub seated_players: i32,
}

/// The `PerformSetup` form: the host sets one faction's starting spice and forces at a time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PerformSetupOptions {
    /// `Game.NextFactionToPerformCustomSetup`, the faction being set up (`Placer`).
    #[ts(type = "Faction")]
    pub faction: String,
    /// The placer's reserves, which cap what may be staged.
    pub forces_in_reserve: i32,
    pub special_forces_in_reserve: i32,
    /// `MayStageSpecial`: `Placer.HasSpecialForces && !Placer.Is(Faction.Richese)`.
    pub may_stage_special: bool,
    /// `ValidLocations`: `Game.Map.Locations(false)`, ids.
    pub locations: Vec<i32>,
    /// The range of the spice picker (0 to 100).
    pub max_resources: i32,
}
