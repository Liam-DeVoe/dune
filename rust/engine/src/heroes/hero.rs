//! `IHero` has three implementations: `Leader`, `Messiah` and `TreacheryCard`
//! (mercenary cards). [`Hero`] is the closed union of them. Two heroes are
//! equal only if they are the same object: same leader, the one messiah, or
//! the same card.

use std::fmt;

use crate::enumerations::{Faction, HeroType};
use crate::identity::IIdentifiable;
use crate::heroes::leader::Leader;
use crate::heroes::messiah::Messiah;
use crate::cards::treachery_card::TreacheryCard;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum Hero {
    Leader(Leader),
    Messiah,
    TreacheryCard(TreacheryCard),
}

impl Hero {
    pub fn id(self) -> i32 {
        match self {
            Hero::Leader(l) => l.id(),
            Hero::Messiah => Messiah::ID,
            Hero::TreacheryCard(c) => c.id(),
        }
    }

    pub fn value(self) -> i32 {
        match self {
            Hero::Leader(l) => l.value(),
            Hero::Messiah => Messiah::VALUE,
            Hero::TreacheryCard(c) => c.value(),
        }
    }

    pub fn value_in_combat_against(self, opposing_hero: Option<Hero>) -> i32 {
        match self {
            Hero::Leader(l) => l.value_in_combat_against(opposing_hero),
            Hero::Messiah => Messiah::value_in_combat_against(opposing_hero),
            Hero::TreacheryCard(c) => c.value_in_combat_against(opposing_hero),
        }
    }

    pub fn skin_id(self) -> i32 {
        match self {
            Hero::Leader(l) => l.skin_id(),
            Hero::Messiah => Messiah::SKIN_ID,
            Hero::TreacheryCard(c) => c.skin_id(),
        }
    }

    pub fn faction(self) -> Faction {
        match self {
            Hero::Leader(l) => l.faction(),
            Hero::Messiah => Messiah::FACTION,
            Hero::TreacheryCard(c) => c.faction(),
        }
    }

    pub fn cost_to_revive(self) -> i32 {
        match self {
            Hero::Leader(l) => l.cost_to_revive(),
            Hero::Messiah => Messiah::COST_TO_REVIVE,
            Hero::TreacheryCard(c) => c.cost_to_revive(),
        }
    }

    pub fn is_traitor(self, hero: Hero) -> bool {
        match self {
            Hero::Leader(l) => l.is_traitor(hero),
            Hero::Messiah => false,
            Hero::TreacheryCard(c) => c.is_traitor(hero),
        }
    }

    pub fn is_face_dancer(self, hero: Hero) -> bool {
        match self {
            Hero::Leader(l) => l.is_face_dancer(hero),
            Hero::Messiah => false,
            Hero::TreacheryCard(c) => c.is_face_dancer(hero),
        }
    }

    pub fn is(self, f: Faction) -> bool {
        match self {
            Hero::Leader(l) => l.is(f),
            Hero::Messiah => Messiah::FACTION == f,
            Hero::TreacheryCard(c) => c.is(f),
        }
    }

    pub fn hero_type(self) -> HeroType {
        match self {
            Hero::Leader(l) => l.hero_type(),
            Hero::Messiah => Messiah::HERO_TYPE,
            Hero::TreacheryCard(c) => c.hero_type(),
        }
    }

    /// `hero as Leader`
    pub fn as_leader(self) -> Option<Leader> {
        match self {
            Hero::Leader(l) => Some(l),
            _ => None,
        }
    }

    /// `hero as TreacheryCard`
    pub fn as_treachery_card(self) -> Option<TreacheryCard> {
        match self {
            Hero::TreacheryCard(c) => Some(c),
            _ => None,
        }
    }

    /// `hero is Messiah`
    pub fn is_messiah(self) -> bool {
        matches!(self, Hero::Messiah)
    }
}

impl From<Leader> for Hero {
    fn from(l: Leader) -> Hero {
        Hero::Leader(l)
    }
}

impl From<TreacheryCard> for Hero {
    fn from(c: TreacheryCard) -> Hero {
        Hero::TreacheryCard(c)
    }
}

impl IIdentifiable for Hero {
    fn id(&self) -> i32 {
        Hero::id(*self)
    }
}

impl fmt::Debug for Hero {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Hero::Leader(l) => write!(f, "{l:?}"),
            Hero::Messiah => f.write_str("Messiah"),
            Hero::TreacheryCard(c) => write!(f, "{c:?}"),
        }
    }
}

impl serde::Serialize for Hero {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_i32(self.id())
    }
}

/// The text a hero prints as when no describer is set (the engine never sets
/// one): the full .NET type name. Used in dictionary exception messages such
/// as `LeaderState[h]` on a missing key.
impl dotnet_compat::exception::CsToString for Hero {
    fn cs_to_string(&self) -> String {
        match self {
            Hero::Leader(_) => "Treachery.Shared.Leader".to_string(),
            Hero::Messiah => "Treachery.Shared.Messiah".to_string(),
            Hero::TreacheryCard(_) => "Treachery.Shared.TreacheryCard".to_string(),
        }
    }
}
