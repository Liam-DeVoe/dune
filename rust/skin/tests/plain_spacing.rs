//! Plain text puts a space between two parts the client draws as separate tokens, where their
//! texts would run together ("3 forces", not "3forces"). Text parts keep their own spacing.

use treachery_engine::enumerations::{Concept, Faction, FactionForce, FactionSpecialForce};
use treachery_engine::express;
use treachery_engine::express_part_if;
use treachery_engine::messages::payment::Payment;
use treachery_engine::messages::message::Message;
use treachery_skin::default_skin;

fn text(m: &Message) -> String {
    default_skin().render(m)
}

#[test]
fn a_number_and_a_force_are_spaced() {
    let m = express!("The storm kills ", 3, Faction::Fremen, " forces");
    assert_eq!(text(&m), "The storm kills 3 Fremen forces");
    let m = express!(Faction::Emperor, " lose ", 2, FactionForce::Emperor, 1, FactionSpecialForce::Emperor);
    assert_eq!(text(&m), "Emperor lose 2 forces 1 Sardaukar");
}

#[test]
fn a_faction_and_an_amount_are_spaced() {
    let m = express!("Bids: ", Faction::Atreides, Payment::of(3), ", ", Faction::Guild, Payment::of(0));
    assert_eq!(text(&m), "Bids: Atreides 3 Spice, Guild 0 Spice");
    let m = express!("Storm dial: ", 4, Faction::Guild, " + ", 1, Faction::Atreides, " = ", 5);
    assert_eq!(text(&m), "Storm dial: 4 Guild + 1 Atreides = 5");
}

#[test]
fn parts_inside_phrases_are_spaced_at_their_edges() {
    let m = express!("move ", express_part_if!(true; 5, FactionForce::Fremen), express_part_if!(true; 1, FactionSpecialForce::Fremen), " to");
    assert_eq!(text(&m), "move 5 forces 1 Fedaykin to");
}

#[test]
fn text_keeps_its_own_spacing() {
    // No space is added next to text, nor between tokens that don't run together.
    let m = express!(3, "rd time; ", Concept::Resource, "-", 2, " (", Faction::Guild, ")");
    assert_eq!(text(&m), "3rd time; Spice-2 (Guild)");
}
