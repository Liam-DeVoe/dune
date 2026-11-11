//! Text rendering with `DefaultSkin`, through the `treachery-skin` crate.
//!
//! The C# server sets `Message.DefaultDescriber = DefaultSkin.Default` at startup
//! (Program.cs:20). That changes the text the hub sends and mails:
//!
//! - `validationResult.ToString()` in `ValidateAndExecute` (GameHub_GameEvents.cs:207) and
//!   `errorMessage?.ToString()` after `Game.TryLoad` (GameHub_GameManagement.cs:25, :124) are
//!   `Message.ToString()`, which with a describer set is `ToString(DefaultDescriber) + "*"`
//!   (Model/Message.cs:85-90). This text is the `ErrorDetails` the player sees.
//! - The mails use `DefaultSkin.Default.Describe(faction)`, `DefaultSkin.Default.Format(...)` and
//!   `Message.ToString(DefaultSkin.Default)` (GameHub_GameEvents.cs:257-280,
//!   GameHub_GameManagement.cs:665-674).
//!
//! Every message rendered here is built just before it is rendered, so the collections it holds
//! are current and no game is needed for live lists (see `Skin::describe_in`).
//!
//! The describer also changes `ToString()` of `Location`, `Territory`, `TreacheryCard`, `Leader`,
//! `ResourceCard` and `Message` inside the engine (hazards.md 6.3). That is package E's audit,
//! not this module's.

use treachery_engine::enumerations::{Faction, Phase};
use treachery_engine::message::Message;
use treachery_skin::default_skin;

/// `message.ToString()` with `Message.DefaultDescriber = DefaultSkin.Default`.
pub fn message_to_string_with_default_describer(message: &Message) -> String {
    default_skin().message_to_string_with_default_describer(message)
}

/// `message.ToString(DefaultSkin.Default)`.
pub fn message_to_string(message: &Message) -> String {
    default_skin().message_to_string(message)
}

/// `DefaultSkin.Default.Describe(faction)`.
pub fn describe_faction(faction: Faction) -> String {
    default_skin().describe_faction(faction)
}

/// `DefaultSkin.Default.Format("Turn: {0}, phase: {1}", turn, phase)` (GameHub_GameEvents.cs:264).
/// `Describe(Phase)` names only a few phases and says "unknown" for the rest, as in C#.
pub fn format_turn_and_phase(turn: i32, phase: Phase) -> String {
    default_skin().format("Turn: {0}, phase: {1}", &treachery_skin::args![turn, phase])
}

#[cfg(test)]
mod tests {
    use super::*;
    use treachery_engine::enumerations::Concept;
    use treachery_engine::express;

    #[test]
    fn renders_like_the_csharp_server() {
        let m = express!(Faction::Atreides, " pays ", 3, " ", Concept::Resource);
        assert_eq!(message_to_string(&m), "Atreides pays 3 Spice");
        assert_eq!(message_to_string_with_default_describer(&m), "Atreides pays 3 Spice*");
        assert_eq!(describe_faction(Faction::Harkonnen), "Harkonnen");
        assert_eq!(format_turn_and_phase(3, Phase::Bidding), "Turn: 3, phase: Next bidding round");
        assert_eq!(format_turn_and_phase(3, Phase::BlowA), "Turn: 3, phase: unknown");
    }
}
