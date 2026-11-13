//! Resource cards are built once, in `Map.GetResourceCardsInAndOutsidePlay`,
//! and compared by identity: the six Shai-Hulud cards are equal in every field
//! yet distinct cards. A `ResourceCard` is a `Copy` handle holding the card's
//! position in that list, so the six Shai-Hulud cards are distinct handles.
//! Every call of `get_resource_cards_in_and_outside_play` returns the same
//! handles. The engine calls it once per game (`EstablishPlayers`); the other
//! caller (`GameStatus`) only displays cards.

use std::fmt;
use std::sync::OnceLock;

use crate::map::location::Location;
use crate::map::Map;
use crate::map::territory::Territory;

/// The fields of a resource card. They are never changed after creation.
#[derive(Debug, Clone)]
pub struct ResourceCardData {
    pub skin_id: i32,
    pub is_sand_trout: bool,
    pub is_great_maker: bool,
    pub location: Option<Location>,
    pub discovery_location: Option<Location>,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ResourceCard(u8);

static CARDS: OnceLock<(Vec<ResourceCardData>, Vec<ResourceCard>)> = OnceLock::new();

fn cards() -> &'static (Vec<ResourceCardData>, Vec<ResourceCard>) {
    CARDS.get_or_init(|| {
        let data = get_resource_cards_in_and_outside_play(&Map::new());
        let handles = (0..data.len()).map(|i| ResourceCard(i as u8)).collect();
        (data, handles)
    })
}

/// `Map.GetResourceCardsInAndOutsidePlay(m)`.
fn get_resource_cards_in_and_outside_play(m: &Map) -> Vec<ResourceCardData> {
    let card = |skin_id, location, discovery_location| ResourceCardData {
        skin_id,
        is_sand_trout: false,
        is_great_maker: false,
        location,
        discovery_location,
    };
    let mut result = Vec::new();

    for location in m.locations(true).filter(|l| l.spice_blow_amount() > 0) {
        result.push(card(location.territory().id(), Some(location), None));
    }

    for _ in 1..=6 {
        result.push(card(98, None, None));
    }

    result.push(ResourceCardData { is_sand_trout: true, ..card(99, None, None) });

    result.push(ResourceCardData { is_great_maker: true, ..card(100, None, None) });

    result.push(card(41, m.sihaya_ridge().resource_blow_location(), m.cielago_east().discovery_token_location()));
    result.push(card(42, m.rock_outcroppings().resource_blow_location(), m.meridan().discovery_token_location()));
    result.push(card(43, m.hagga_basin().resource_blow_location(), m.gara_kulon().discovery_token_location()));
    result.push(card(44, Some(m.funeral_plain()), m.pasty_mesa().discovery_token_location()));
    result.push(card(45, m.wind_pass_north().resource_blow_location(), m.plastic_basin().discovery_token_location()));
    result.push(card(46, m.old_gap().resource_blow_location(), m.false_wall_west().discovery_token_location()));

    result
}

impl ResourceCard {

    /// Every card of `GetResourceCardsInAndOutsidePlay`, in order.
    pub fn get_resource_cards_in_and_outside_play() -> &'static [ResourceCard] {
        &cards().1
    }

    #[track_caller]
    pub fn data(self) -> &'static ResourceCardData {
        &cards().0[self.0 as usize]
    }

    /// Position in `GetResourceCardsInAndOutsidePlay`; this is the card's identity.
    #[track_caller]
    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub fn is_sand_trout(self) -> bool {
        self.data().is_sand_trout
    }

    pub fn is_great_maker(self) -> bool {
        self.data().is_great_maker
    }

    pub fn skin_id(self) -> i32 {
        self.data().skin_id
    }

    pub fn location(self) -> Option<Location> {
        self.data().location
    }

    pub fn discovery_location(self) -> Option<Location> {
        self.data().discovery_location
    }

    pub fn is_shai_hulud(self) -> bool {
        self.location().is_none() && !self.is_sand_trout() && !self.is_great_maker()
    }

    pub fn is_spice_blow(self) -> bool {
        self.location().is_some()
    }

    pub fn is_discovery(self) -> bool {
        self.discovery_location().is_some()
    }

    pub fn territory(self) -> Option<Territory> {
        self.location().map(|l| l.territory())
    }
}

impl fmt::Debug for ResourceCard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ResourceCard(#{} skin {})", self.0, self.skin_id())
    }
}
