//! Cards are created once, in `TreacheryCardManager`, and compared by
//! reference (the `Equals` override is commented out). A `TreacheryCard` is a
//! `Copy` handle (index into the manager's table); handle equality is that
//! reference equality.

use std::fmt;

use crate::compat::cs::{CsNull, NullRef};
use crate::enumerations::{Faction, HeroType, Rule, TreacheryCardType};
use crate::heroes::hero::Hero;
use crate::identity::IIdentifiable;
use crate::cards::treachery_card_manager::ITEMS;

/// Constructor arguments of a card.
#[derive(Debug)]
pub struct TreacheryCardData {
    pub id: i32,
    pub skin_id: i32,
    pub type_: TreacheryCardType,
    pub rules: &'static [Rule],
}

/// A card, or the null card (`TreacheryCard::NULL`, the null handle, which can
/// appear in a collection; see `cs::CsNull`).
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct TreacheryCard(u8);

impl CsNull for TreacheryCard {
    const NULL: TreacheryCard = TreacheryCard::NULL;
}

impl TreacheryCard {
    pub const NONE: i32 = -2;
    pub const UNKNOWN: i32 = -1;
    /// The null handle. Every member throws NullReferenceException.
    pub const NULL: TreacheryCard = TreacheryCard(u8::MAX);

    pub(crate) const fn from_index(i: usize) -> TreacheryCard {
        TreacheryCard(i as u8)
    }

    #[track_caller]
    pub fn data(self) -> &'static TreacheryCardData {
        &ITEMS[self.nn().0 as usize]
    }

    #[track_caller]
    pub fn id(self) -> i32 {
        self.data().id
    }

    #[track_caller]
    pub fn skin_id(self) -> i32 {
        self.data().skin_id
    }

    #[track_caller]
    pub fn value(self) -> i32 {
        self.nn();
        0
    }

    #[track_caller]
    pub fn is(self, _f: Faction) -> bool {
        self.nn();
        false
    }

    #[track_caller]
    pub fn value_in_combat_against(self, _opposing_hero: Option<Hero>) -> i32 {
        let _opposing_hero = _opposing_hero.and_then(crate::compat::cs::CsNull::opt);
        self.nn();
        0
    }

    #[track_caller]
    pub fn faction(self) -> Faction {
        self.nn();
        Faction::None
    }

    #[track_caller]
    pub fn cost_to_revive(self) -> i32 {
        self.nn();
        0
    }

    /// `IsTraitor(hero) => hero is TreacheryCard`
    #[track_caller]
    pub fn is_traitor(self, hero: Hero) -> bool {
        self.nn();
        hero.as_treachery_card().is_some()
    }

    /// `IsFaceDancer(hero) => hero is TreacheryCard`
    #[track_caller]
    pub fn is_face_dancer(self, hero: Hero) -> bool {
        self.nn();
        hero.as_treachery_card().is_some()
    }

    #[track_caller]
    pub fn rules(self) -> &'static [Rule] {
        self.data().rules
    }

    /// `Type` (a Rust keyword, hence the underscore).
    #[track_caller]
    pub fn type_(self) -> TreacheryCardType {
        self.data().type_
    }

    #[track_caller]
    pub fn hero_type(self) -> HeroType {
        self.nn();
        HeroType::Mercenary
    }

    #[track_caller]
    pub fn is_poison_weapon(self) -> bool {
        matches!(self.type_(), TreacheryCardType::Poison | TreacheryCardType::Chemistry | TreacheryCardType::ProjectileAndPoison)
    }

    #[track_caller]
    pub fn is_projectile_weapon(self) -> bool {
        matches!(self.type_(), TreacheryCardType::Projectile | TreacheryCardType::WeirdingWay | TreacheryCardType::ProjectileAndPoison)
    }

    #[track_caller]
    pub fn is_poison_defense(self) -> bool {
        matches!(
            self.type_(),
            TreacheryCardType::Chemistry
                | TreacheryCardType::Antidote
                | TreacheryCardType::ShieldAndAntidote
                | TreacheryCardType::PortableAntidote
        )
    }

    #[track_caller]
    pub fn is_non_antidote_poison_defense(self) -> bool {
        matches!(self.type_(), TreacheryCardType::Chemistry)
    }

    #[track_caller]
    pub fn is_shield(self) -> bool {
        matches!(self.type_(), TreacheryCardType::Shield | TreacheryCardType::ShieldAndAntidote)
    }

    #[track_caller]
    pub fn is_projectile_defense(self) -> bool {
        matches!(self.type_(), TreacheryCardType::Shield | TreacheryCardType::WeirdingWay | TreacheryCardType::ShieldAndAntidote)
    }

    #[track_caller]
    pub fn is_laser(self) -> bool {
        matches!(self.type_(), TreacheryCardType::Laser)
    }

    #[track_caller]
    pub fn is_poison_tooth(self) -> bool {
        matches!(self.type_(), TreacheryCardType::PoisonTooth)
    }

    #[track_caller]
    pub fn is_artillery(self) -> bool {
        matches!(self.type_(), TreacheryCardType::ArtilleryStrike)
    }

    #[track_caller]
    pub fn is_rock_melter(self) -> bool {
        matches!(self.type_(), TreacheryCardType::Rockmelter)
    }

    #[track_caller]
    pub fn is_mirror_weapon(self) -> bool {
        matches!(self.type_(), TreacheryCardType::MirrorWeapon)
    }

    #[track_caller]
    pub fn is_portable_antidote(self) -> bool {
        matches!(self.type_(), TreacheryCardType::PortableAntidote)
    }

    #[track_caller]
    pub fn is_weapon(self) -> bool {
        self.is_poison_weapon()
            || self.is_projectile_weapon()
            || self.is_laser()
            || self.is_poison_tooth()
            || self.is_artillery()
            || self.is_mirror_weapon()
            || self.is_rock_melter()
    }

    #[track_caller]
    pub fn is_defense(self) -> bool {
        self.is_poison_defense() || self.is_projectile_defense()
    }

    #[track_caller]
    pub fn is_useless(self) -> bool {
        matches!(self.type_(), TreacheryCardType::Useless)
    }

    #[track_caller]
    pub fn is_mercenary(self) -> bool {
        matches!(self.type_(), TreacheryCardType::Mercenary)
    }

    #[track_caller]
    pub fn is_atreides(self) -> bool {
        !(self.is_weapon() || self.is_defense() || self.is_useless() || self.is_mercenary())
    }

    /// `CounteredBy(defense, combinedWithWeapon)`. Callers may pass null for
    /// either (the bot passes a plan's `Defense`, which can be null).
    ///
    /// C# parity: a null `defense` throws NullReferenceException only on the
    /// branches that read it. Each clause tests `Type` first, so only a weapon
    /// of type PoisonTooth, Poison, Chemistry, Projectile, WeirdingWay or
    /// ProjectileAndPoison dereferences it; any other type returns false (a
    /// mirror weapon with no combined weapon returns true). Checked against
    /// .NET in tests/boundary.rs.
    ///
    /// C# parity: a mirror weapon combined with a mirror weapon recurses
    /// forever and overflows the stack, which kills the process.
    #[track_caller]
    pub fn countered_by(self, defense: Option<TreacheryCard>, combined_with_weapon: Option<TreacheryCard>) -> bool {
        let defense = defense.and_then(crate::compat::cs::CsNull::opt);
        let combined_with_weapon = combined_with_weapon.and_then(crate::compat::cs::CsNull::opt);
        let t = self.type_();
        if t == TreacheryCardType::MirrorWeapon {
            return match combined_with_weapon {
                None => true,
                Some(w) => w.countered_by(defense, combined_with_weapon),
            };
        }

        (t == TreacheryCardType::PoisonTooth && defense.nre().is_non_antidote_poison_defense())
            || (t == TreacheryCardType::Poison && defense.nre().is_poison_defense())
            || (t == TreacheryCardType::Chemistry && defense.nre().is_poison_defense())
            || (t == TreacheryCardType::Projectile && defense.nre().is_projectile_defense())
            || (t == TreacheryCardType::WeirdingWay && defense.nre().is_projectile_defense())
            || (t == TreacheryCardType::ProjectileAndPoison && defense.nre().type_() == TreacheryCardType::ShieldAndAntidote)
    }
}

impl IIdentifiable for TreacheryCard {
    fn id(&self) -> i32 {
        TreacheryCard::id(*self)
    }
}

impl fmt::Debug for TreacheryCard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            return f.write_str("TreacheryCard(null)");
        }
        write!(f, "TreacheryCard({})", self.id())
    }
}

impl serde::Serialize for TreacheryCard {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.is_null() {
            return s.serialize_none();
        }
        s.serialize_i32(self.id())
    }
}
