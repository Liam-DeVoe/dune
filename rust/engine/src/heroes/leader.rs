//! Leaders are created once, in `LeaderManager`, and compared by identity. A
//! `Leader` is a `Copy` handle (index into `LeaderManager`'s table), so handle
//! equality is identity.

use std::fmt;

use crate::compat::cs::CsNull;
use crate::enumerations::{Faction, HeroType};
use crate::heroes::hero::Hero;
use crate::identity::IIdentifiable;
use crate::heroes::leader_manager::LEADERS;

/// The fields of a leader, as set in `LeaderManager.Initialize`.
#[derive(Debug)]
pub struct LeaderData {
    pub id: i32,
    pub value: i32,
    pub faction: Faction,
    pub hero_type: HeroType,
}

/// A leader, or the null leader (`Leader::NULL`, the null handle, which can
/// appear in a collection; see `cs::CsNull`).
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Leader(u8);

impl CsNull for Leader {
    const NULL: Leader = Leader::NULL;
}

impl Leader {
    /// The null handle. Every member throws NullReferenceException.
    pub const NULL: Leader = Leader(u8::MAX);

    pub(crate) const fn from_index(i: usize) -> Leader {
        Leader(i as u8)
    }

    #[track_caller]
    pub fn data(self) -> &'static LeaderData {
        &LEADERS[self.nn().0 as usize]
    }

    pub fn id(self) -> i32 {
        self.data().id
    }

    pub fn faction(self) -> Faction {
        self.data().faction
    }

    pub fn value(self) -> i32 {
        self.data().value
    }

    pub fn hero_type(self) -> HeroType {
        self.data().hero_type
    }

    /// `Is(f)`
    pub fn is(self, f: Faction) -> bool {
        self.faction() == f
    }

    /// `ValueInCombatAgainst(opposingHero)`
    pub fn value_in_combat_against(self, opposing_hero: Option<Hero>) -> i32 {
        let opposing_hero = opposing_hero.and_then(crate::compat::cs::CsNull::opt);
        if self.hero_type() == HeroType::VariableValue {
            return match opposing_hero {
                None => 0,
                Some(h) => h.value(),
            };
        }
        self.value()
    }

    /// `CostToRevive`
    pub fn cost_to_revive(self) -> i32 {
        if self.hero_type() == HeroType::VariableValue {
            return 6;
        }
        if self.hero_type() == HeroType::Vidal {
            return 5;
        }
        self.value()
    }

    pub fn skin_id(self) -> i32 {
        self.id()
    }

    /// `IsTraitor(hero) => hero == this` (reference equality)
    #[track_caller]
    pub fn is_traitor(self, hero: Hero) -> bool {
        self.nn();
        hero == Hero::Leader(self)
    }

    /// `IsFaceDancer(hero) => hero == this` (reference equality)
    #[track_caller]
    pub fn is_face_dancer(self, hero: Hero) -> bool {
        self.nn();
        hero == Hero::Leader(self)
    }
}

impl IIdentifiable for Leader {
    fn id(&self) -> i32 {
        Leader::id(*self)
    }
}

impl fmt::Debug for Leader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            return f.write_str("Leader(null)");
        }
        write!(f, "Leader({})", self.id())
    }
}

impl serde::Serialize for Leader {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.is_null() {
            return s.serialize_none();
        }
        s.serialize_i32(self.id())
    }
}
