//! Events only the host submits: player setup, deck customization, ending phases, replacing players.

mod automation_configured;
pub use automation_configured::*;
mod cards_determined;
pub use cards_determined::*;
mod end_phase;
pub use end_phase::*;
mod establish_players;
pub use establish_players::*;
mod hide_secrets;
pub use hide_secrets::*;
mod perform_setup;
pub use perform_setup::*;
mod player_replaced;
pub use player_replaced::*;
