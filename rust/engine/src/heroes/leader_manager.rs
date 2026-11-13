use crate::enumerations::{Faction, HeroType};
use crate::identity::IFetcher;
use crate::heroes::hero::Hero;
use crate::heroes::leader::{Leader, LeaderData};
use crate::cards::treachery_card_manager::TreacheryCardManager;

pub struct LeaderManager;

/// `LeaderManager.Leaders`, in order.
pub(crate) static LEADERS: [LeaderData; 62] = [
    LeaderData { id: 1001, value: 5, faction: Faction::Atreides, hero_type: HeroType::Normal },
    LeaderData { id: 1002, value: 5, faction: Faction::Atreides, hero_type: HeroType::Normal },
    LeaderData { id: 1003, value: 4, faction: Faction::Atreides, hero_type: HeroType::Normal },
    LeaderData { id: 1004, value: 2, faction: Faction::Atreides, hero_type: HeroType::Normal },
    LeaderData { id: 1005, value: 1, faction: Faction::Atreides, hero_type: HeroType::Normal },
    LeaderData { id: 1006, value: 5, faction: Faction::BeneGesserit, hero_type: HeroType::Normal },
    LeaderData { id: 1007, value: 5, faction: Faction::BeneGesserit, hero_type: HeroType::Normal },
    LeaderData { id: 1008, value: 5, faction: Faction::BeneGesserit, hero_type: HeroType::Normal },
    LeaderData { id: 1009, value: 5, faction: Faction::BeneGesserit, hero_type: HeroType::Normal },
    LeaderData { id: 1010, value: 5, faction: Faction::BeneGesserit, hero_type: HeroType::Normal },
    LeaderData { id: 1011, value: 6, faction: Faction::Emperor, hero_type: HeroType::Normal },
    LeaderData { id: 1012, value: 5, faction: Faction::Emperor, hero_type: HeroType::Normal },
    LeaderData { id: 1013, value: 3, faction: Faction::Emperor, hero_type: HeroType::Normal },
    LeaderData { id: 1014, value: 3, faction: Faction::Emperor, hero_type: HeroType::Normal },
    LeaderData { id: 1015, value: 2, faction: Faction::Emperor, hero_type: HeroType::Normal },
    LeaderData { id: 1016, value: 7, faction: Faction::Fremen, hero_type: HeroType::Normal },
    LeaderData { id: 1017, value: 6, faction: Faction::Fremen, hero_type: HeroType::Normal },
    LeaderData { id: 1018, value: 5, faction: Faction::Fremen, hero_type: HeroType::Normal },
    LeaderData { id: 1019, value: 3, faction: Faction::Fremen, hero_type: HeroType::Normal },
    LeaderData { id: 1020, value: 2, faction: Faction::Fremen, hero_type: HeroType::Normal },
    LeaderData { id: 1021, value: 5, faction: Faction::Guild, hero_type: HeroType::Normal },
    LeaderData { id: 1022, value: 3, faction: Faction::Guild, hero_type: HeroType::Normal },
    LeaderData { id: 1023, value: 3, faction: Faction::Guild, hero_type: HeroType::Normal },
    LeaderData { id: 1024, value: 2, faction: Faction::Guild, hero_type: HeroType::Normal },
    LeaderData { id: 1025, value: 1, faction: Faction::Guild, hero_type: HeroType::Normal },
    LeaderData { id: 1026, value: 6, faction: Faction::Harkonnen, hero_type: HeroType::Normal },
    LeaderData { id: 1027, value: 4, faction: Faction::Harkonnen, hero_type: HeroType::Normal },
    LeaderData { id: 1028, value: 3, faction: Faction::Harkonnen, hero_type: HeroType::Normal },
    LeaderData { id: 1029, value: 2, faction: Faction::Harkonnen, hero_type: HeroType::Normal },
    LeaderData { id: 1030, value: 1, faction: Faction::Harkonnen, hero_type: HeroType::Normal },
    LeaderData { id: 1031, value: 4, faction: Faction::Ixian, hero_type: HeroType::Normal },
    LeaderData { id: 1032, value: 5, faction: Faction::Ixian, hero_type: HeroType::Normal },
    LeaderData { id: 1033, value: 5, faction: Faction::Ixian, hero_type: HeroType::Normal },
    LeaderData { id: 1034, value: 2, faction: Faction::Ixian, hero_type: HeroType::Normal },
    LeaderData { id: 1035, value: 1, faction: Faction::Ixian, hero_type: HeroType::Normal },
    LeaderData { id: 1036, value: 0, faction: Faction::Tleilaxu, hero_type: HeroType::VariableValue },
    LeaderData { id: 1037, value: 4, faction: Faction::Tleilaxu, hero_type: HeroType::Normal },
    LeaderData { id: 1038, value: 3, faction: Faction::Tleilaxu, hero_type: HeroType::Normal },
    LeaderData { id: 1039, value: 2, faction: Faction::Tleilaxu, hero_type: HeroType::Normal },
    LeaderData { id: 1040, value: 1, faction: Faction::Tleilaxu, hero_type: HeroType::Normal },
    LeaderData { id: 1041, value: 2, faction: Faction::Choam, hero_type: HeroType::Normal },
    LeaderData { id: 1042, value: 3, faction: Faction::Choam, hero_type: HeroType::Normal },
    LeaderData { id: 1043, value: 3, faction: Faction::Choam, hero_type: HeroType::Normal },
    LeaderData { id: 1044, value: 4, faction: Faction::Choam, hero_type: HeroType::Normal },
    LeaderData { id: 1045, value: 4, faction: Faction::Choam, hero_type: HeroType::Normal },
    LeaderData { id: 1046, value: 2, faction: Faction::Choam, hero_type: HeroType::Auditor },
    LeaderData { id: 1047, value: 2, faction: Faction::Richese, hero_type: HeroType::Normal },
    LeaderData { id: 1048, value: 2, faction: Faction::Richese, hero_type: HeroType::Normal },
    LeaderData { id: 1049, value: 3, faction: Faction::Richese, hero_type: HeroType::Normal },
    LeaderData { id: 1050, value: 4, faction: Faction::Richese, hero_type: HeroType::Normal },
    LeaderData { id: 1051, value: 5, faction: Faction::Richese, hero_type: HeroType::Normal },
    LeaderData { id: 1052, value: 4, faction: Faction::Ecaz, hero_type: HeroType::Normal },
    LeaderData { id: 1053, value: 3, faction: Faction::Ecaz, hero_type: HeroType::Normal },
    LeaderData { id: 1054, value: 3, faction: Faction::Ecaz, hero_type: HeroType::Normal },
    LeaderData { id: 1055, value: 2, faction: Faction::Ecaz, hero_type: HeroType::Normal },
    LeaderData { id: 1056, value: 4, faction: Faction::Ecaz, hero_type: HeroType::Normal },
    LeaderData { id: 1057, value: 6, faction: Faction::Ecaz, hero_type: HeroType::Vidal },
    LeaderData { id: 1058, value: 5, faction: Faction::Moritani, hero_type: HeroType::Normal },
    LeaderData { id: 1059, value: 4, faction: Faction::Moritani, hero_type: HeroType::Normal },
    LeaderData { id: 1060, value: 2, faction: Faction::Moritani, hero_type: HeroType::Normal },
    LeaderData { id: 1061, value: 4, faction: Faction::Moritani, hero_type: HeroType::Normal },
    LeaderData { id: 1062, value: 1, faction: Faction::Moritani, hero_type: HeroType::Normal },
];

static LEADER_HANDLES: [Leader; 62] = {
    let mut a = [Leader::from_index(0); 62];
    let mut i = 0;
    while i < 62 {
        a[i] = Leader::from_index(i);
        i += 1;
    }
    a
};

impl LeaderManager {
    pub const FIRST_ID: i32 = 1000;

    /// `LeaderManager.Leaders`
    pub fn leaders() -> &'static [Leader] {
        &LEADER_HANDLES
    }

    /// `LeaderManager.Messiah`
    pub fn messiah() -> Hero {
        Hero::Messiah
    }

    /// `LeaderManager.LeaderLookup`
    pub fn leader_lookup() -> LeaderFetcher {
        LeaderFetcher
    }

    /// `LeaderManager.HeroLookup`
    pub fn hero_lookup() -> HeroFetcher {
        HeroFetcher
    }

    /// `GetLeaders(faction)`
    pub fn get_leaders(faction: Faction) -> impl Iterator<Item = Leader> + Clone {
        Self::leaders().iter().copied().filter(move |l| l.faction() == faction)
    }
}

/// `LeaderManager.LeaderFetcher`
#[derive(Clone, Copy, Debug)]
pub struct LeaderFetcher;

impl IFetcher<Leader> for LeaderFetcher {
    fn find(&self, id: i32) -> Option<Leader> {
        if id == -1 {
            return None;
        }
        LeaderManager::leaders().iter().copied().find(|t| t.id() == id)
    }

    fn get_id(&self, obj: Option<Leader>) -> i32 {
        let obj = obj.and_then(crate::compat::cs::CsNull::opt);
        obj.and_then(crate::compat::cs::CsNull::opt).map_or(-1, |l| l.id())
    }
}

/// `LeaderManager.HeroFetcher`
#[derive(Clone, Copy, Debug)]
pub struct HeroFetcher;

impl IFetcher<Hero> for HeroFetcher {
    fn find(&self, id: i32) -> Option<Hero> {
        if id < 0 {
            return None;
        }
        if id == crate::heroes::messiah::Messiah::ID {
            return Some(Hero::Messiah);
        }
        if id >= LeaderManager::FIRST_ID {
            return LeaderManager::leaders()
                .iter()
                .copied()
                .find(|t| t.id() == id)
                .map(Hero::Leader);
        }
        TreacheryCardManager::lookup().find(id).map(Hero::TreacheryCard)
    }

    fn get_id(&self, obj: Option<Hero>) -> i32 {
        let obj = obj.and_then(crate::compat::cs::CsNull::opt);
        obj.and_then(crate::compat::cs::CsNull::opt).map_or(-1, |h| h.id())
    }
}
