//! Text the server writes (refusal messages, mail) with the default skin.

use treachery_engine::enumerations::{Faction, Phase};
use treachery_engine::messages::message::Message;
use treachery_skin::default_skin;

pub fn message_to_string(message: &Message) -> String {
    default_skin().message_to_string(message)
}

pub fn describe_faction(faction: Faction) -> String {
    default_skin().describe_faction(faction)
}

/// "Turn: 3, phase: ...", for the async-play mail.
pub fn format_turn_and_phase(turn: i32, phase: Phase) -> String {
    default_skin().format("Turn: {0}, phase: {1}", &treachery_skin::args![turn, phase])
}

#[cfg(test)]
mod tests {
    use super::*;
    use treachery_engine::enumerations::Concept;
    use treachery_engine::express;

    #[test]
    fn renders_with_the_default_skin() {
        let m = express!(Faction::Atreides, " pays ", 3, " ", Concept::Resource);
        assert_eq!(message_to_string(&m), "Atreides pays 3 Spice");
        assert_eq!(describe_faction(Faction::Harkonnen), "Harkonnen");
        assert_eq!(format_turn_and_phase(3, Phase::Bidding), "Turn: 3, phase: Next bidding round");
    }
}
