//! Alliances, deals, and transfers between players.

mod alliance_broken;
pub use alliance_broken::*;
mod alliance_by_ambassador;
pub use alliance_by_ambassador::*;
mod alliance_by_terror;
pub use alliance_by_terror::*;
mod alliance_offered;
pub use alliance_offered::*;
mod alliance_passed;
pub use alliance_passed::*;
mod ally_permission;
pub use ally_permission::*;
mod card_traded;
pub use card_traded::*;
mod deal_accepted;
pub use deal_accepted::*;
mod deal_offered;
pub use deal_offered::*;
mod deal_rejected;
pub use deal_rejected::*;
mod donated;
pub use donated::*;
mod resources_transferred;
pub use resources_transferred::*;
mod richese_gave_card;
pub use richese_gave_card::*;
