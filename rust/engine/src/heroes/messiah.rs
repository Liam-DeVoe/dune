//! There is one `Messiah` object (`LeaderManager.Messiah`); it is
//! [`Hero::Messiah`](crate::heroes::hero::Hero::Messiah) in Rust. Its `Id` and
//! `SkinId` are set once by `LeaderManager` and never change.

use crate::enumerations::{Faction, HeroType};
use crate::heroes::hero::Hero;

pub struct Messiah;

impl Messiah {
    pub const ID: i32 = crate::heroes::leader_manager::LeaderManager::FIRST_ID;
    pub const SKIN_ID: i32 = 1098;
    pub const VALUE: i32 = 2;
    pub const FACTION: Faction = Faction::Atreides;
    pub const HERO_TYPE: HeroType = HeroType::Messiah;

    /// `ValueInCombatAgainst(_) => Value`
    pub fn value_in_combat_against(_opposing_hero: Option<Hero>) -> i32 {
        let _opposing_hero = _opposing_hero.and_then(crate::compat::cs::CsNull::opt);
        Self::VALUE
    }

    /// `CostToRevive => Value`
    pub const COST_TO_REVIVE: i32 = Self::VALUE;
}
