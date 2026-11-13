use crate::prelude::*;

// C# enum `NexusAllianceChange` is generated into enumerations.rs (tools/gen_enumerations.py).

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct BlowDeckLog {
    pub deck: String,
}

impl Default for BlowDeckLog {
    fn default() -> Self {
        BlowDeckLog {
            deck: Default::default(),
        }
    }
}

impl BlowDeckLog {
    pub fn new(deck: String) -> Self {
        BlowDeckLog { deck }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct BlowCardLog {
    pub card: ResourceCard,
    pub location: Location,
    pub amount: i32,
    pub lost: bool,
    pub doubled: bool,
}

impl BlowCardLog {
    pub fn new(card: ResourceCard, location: Location, amount: i32, lost: bool, doubled: bool) -> Self {
        BlowCardLog { card, location, amount, lost, doubled }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct WormAppearedLog {
    pub territory: Territory,
    pub nth: i32,
}

impl WormAppearedLog {
    pub fn new(territory: Territory, nth: i32) -> Self {
        WormAppearedLog { territory, nth }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct WormSentLog {
    pub by: Faction,
    pub territory: Territory,
}

impl WormSentLog {
    pub fn new(by: Faction, territory: Territory) -> Self {
        WormSentLog { by, territory }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct WormDevouredForcesLog {
    pub faction: Faction,
    pub forces: i32,
    pub special_forces: i32,
    pub location: Location,
}

impl WormDevouredForcesLog {
    pub fn new(faction: Faction, forces: i32, special_forces: i32, location: Location) -> Self {
        WormDevouredForcesLog { faction, forces, special_forces, location }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct WormSurvivedLog {
    pub faction: Faction,
    pub location: Location,
}

impl WormSurvivedLog {
    pub fn new(faction: Faction, location: Location) -> Self {
        WormSurvivedLog { faction, location }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct WormDevouredSpiceLog {
    pub amount: i32,
    pub territory: Territory,
}

impl WormDevouredSpiceLog {
    pub fn new(amount: i32, territory: Territory) -> Self {
        WormDevouredSpiceLog { amount, territory }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct WormIgnoredLog {
}

impl Default for WormIgnoredLog {
    fn default() -> Self {
        WormIgnoredLog {
        }
    }
}

impl WormIgnoredLog {
}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct BlowReshuffleLog {
    pub cards: i32,
}

impl Default for BlowReshuffleLog {
    fn default() -> Self {
        BlowReshuffleLog {
            cards: Default::default(),
        }
    }
}

impl BlowReshuffleLog {
    pub fn new(cards: i32) -> Self {
        BlowReshuffleLog { cards }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct NexusLog {
    pub ride: Option<NexusRide>,
    pub alliance: Option<NexusAlliance>,
}

impl Default for NexusLog {
    fn default() -> Self {
        NexusLog {
            ride: Default::default(),
            alliance: Default::default(),
        }
    }
}

impl NexusLog {
    pub fn new(ride: Option<NexusRide>, alliance: Option<NexusAlliance>) -> Self {
        NexusLog { ride, alliance }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct NexusAlliance {
    pub change: NexusAllianceChange,
    pub from: Faction,
    pub to: Faction,
}

impl Default for NexusAlliance {
    fn default() -> Self {
        NexusAlliance {
            change: Default::default(),
            from: Default::default(),
            to: Default::default(),
        }
    }
}

impl NexusAlliance {
    pub fn new(change: NexusAllianceChange, from: Faction, to: Faction) -> Self {
        NexusAlliance { change, from, to }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct NexusRide {
    pub faction: Faction,
    pub forces: i32,
    pub special_forces: i32,
    pub from: Option<Location>,
    pub to: Location,
}

impl NexusRide {
    pub fn new(faction: Faction, forces: i32, special_forces: i32, from: Option<Location>, to: Location) -> Self {
        let from = from.and_then(crate::compat::cs::CsNull::opt);
        NexusRide { faction, forces, special_forces, from, to }
    }

}
