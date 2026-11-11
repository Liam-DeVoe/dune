//! Port of Model/IHero.cs.
//!
//! `IHero` has three implementations: `Leader`, `Messiah` and `TreacheryCard`
//! (mercenary cards). [`Hero`] is the closed union of them. Equality is C#
//! reference equality: same leader, the one messiah, or the same card.

use std::fmt;

use crate::cs::CsNull;
use crate::enumerations::{Faction, HeroType};
use crate::i_identifiable::IIdentifiable;
use crate::leader::Leader;
use crate::messiah::Messiah;
use crate::treachery_card::TreacheryCard;

/// `Hero::NULL` is a C# null `IHero` (see `cs::CsNull`). It is the null card
/// wrapped as a hero, and `From<Leader>`/`From<TreacheryCard>` map a null
/// leader or card to it, so every null hero is the same value. `as_leader`,
/// `as_treachery_card` and `is_messiah` are false for it, like C# `is`.
#[derive(Copy, Clone)]
pub enum Hero {
    Leader(Leader),
    Messiah,
    TreacheryCard(TreacheryCard),
}

impl CsNull for Hero {
    const NULL: Hero = Hero::NULL;

    fn is_null(self) -> bool {
        match self {
            Hero::Leader(l) => l.is_null(),
            Hero::Messiah => false,
            Hero::TreacheryCard(c) => c.is_null(),
        }
    }
}

/// Reference equality, where `Hero::Leader(Leader::NULL)` is the same null as
/// `Hero::NULL`.
impl PartialEq for Hero {
    fn eq(&self, other: &Hero) -> bool {
        match (self.normalized(), other.normalized()) {
            (Hero::Leader(a), Hero::Leader(b)) => a == b,
            (Hero::Messiah, Hero::Messiah) => true,
            (Hero::TreacheryCard(a), Hero::TreacheryCard(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Hero {}

impl std::hash::Hash for Hero {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self.normalized() {
            Hero::Leader(l) => (0u8, l).hash(state),
            Hero::Messiah => 1u8.hash(state),
            Hero::TreacheryCard(c) => (2u8, c).hash(state),
        }
    }
}

impl Hero {
    /// C# null. Every member throws NullReferenceException.
    pub const NULL: Hero = Hero::TreacheryCard(TreacheryCard::NULL);

    /// Every null hero as `Hero::NULL`.
    fn normalized(self) -> Hero {
        if self.is_null() { Hero::NULL } else { self }
    }

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
        let opposing_hero = opposing_hero.and_then(crate::cs::CsNull::opt);
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
            Hero::Leader(l) => l.opt(),
            _ => None,
        }
    }

    /// `hero as TreacheryCard`
    pub fn as_treachery_card(self) -> Option<TreacheryCard> {
        match self {
            Hero::TreacheryCard(c) => c.opt(),
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
        if l.is_null() {
            return Hero::NULL;
        }
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
        if self.is_null() {
            return s.serialize_none();
        }
        s.serialize_i32(self.id())
    }
}

/// C# `ToString()` with `Message.DefaultDescriber` null (the engine never sets
/// it): `object.ToString()`, the full type name. Used in dictionary exception
/// messages such as `LeaderState[h]` on a missing key.
impl dotnet_compat::exception::CsToString for Hero {
    fn cs_to_string(&self) -> String {
        match self {
            h if h.is_null() => crate::cs::null_reference(),
            Hero::Leader(_) => "Treachery.Shared.Leader".to_string(),
            Hero::Messiah => "Treachery.Shared.Messiah".to_string(),
            Hero::TreacheryCard(_) => "Treachery.Shared.TreacheryCard".to_string(),
        }
    }
}
