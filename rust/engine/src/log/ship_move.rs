use crate::prelude::*;

// The enum `ShipKind` is in enumerations.rs.

// The enum `BeneGesseritReactionKind` is in enumerations.rs.

#[derive(Clone, Debug, PartialEq)]
pub struct ShipLogged {
    pub faction: Faction,
    pub kind: ShipKind,
    pub forces: i32,
    pub special_forces: i32,
    pub from: Option<Location>,
    pub to: Option<Location>,
    pub cost: i32,
    pub ally_share: i32,
    pub ally: Faction,
    pub guild_paid: bool,
    pub karama: Option<TreacheryCard>,
    pub blocked: bool,
    pub blocked_by: Option<TreacheryCard>,
    pub plain: bool,
}

impl Default for ShipLogged {
    fn default() -> Self {
        ShipLogged {
            faction: Default::default(),
            kind: Default::default(),
            forces: Default::default(),
            special_forces: Default::default(),
            from: Default::default(),
            to: Default::default(),
            cost: Default::default(),
            ally_share: Default::default(),
            ally: Default::default(),
            guild_paid: Default::default(),
            karama: Default::default(),
            blocked: Default::default(),
            blocked_by: Default::default(),
            plain: Default::default(),
        }
    }
}

impl ShipLogged {
    pub fn new(faction: Faction, kind: ShipKind, forces: i32, special_forces: i32, from: Option<Location>, to: Option<Location>, cost: i32, ally_share: i32, ally: Faction, guild_paid: bool, karama: Option<TreacheryCard>, blocked: bool, blocked_by: Option<TreacheryCard>, plain: bool) -> Self {
        ShipLogged { faction, kind, forces, special_forces, from, to, cost, ally_share, ally, guild_paid, karama, blocked, blocked_by, plain }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveLogged {
    pub faction: Faction,
    pub move_: i32,
    pub from: Territory,
    pub forces: i32,
    pub special_forces: i32,
    pub to: Location,
    pub hajr: bool,
    pub as_advisors: Option<bool>,
}

impl MoveLogged {
    pub fn new(faction: Faction, move_: i32, from: Territory, forces: i32, special_forces: i32, to: Location, hajr: bool, as_advisors: Option<bool>) -> Self {
        MoveLogged { faction, move_, from, forces, special_forces, to, hajr, as_advisors }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct MovePassed {
    pub faction: Faction,
}

impl Default for MovePassed {
    fn default() -> Self {
        MovePassed {
            faction: Default::default(),
        }
    }
}

impl MovePassed {
    pub fn new(faction: Faction) -> Self {
        MovePassed { faction }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct BeneGesseritReaction {
    pub kind: BeneGesseritReactionKind,
    pub place: ExpressionElement,
    pub advisors: bool,
}

impl BeneGesseritReaction {
    pub fn new(kind: BeneGesseritReactionKind, place: ExpressionElement, advisors: bool) -> Self {
        BeneGesseritReaction { kind, place, advisors }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct ShipMoveLoss {
    pub faction: Faction,
    pub storm: bool,
    pub travelling: bool,
    pub forces: i32,
    pub territory: Option<Territory>,
}

impl Default for ShipMoveLoss {
    fn default() -> Self {
        ShipMoveLoss {
            faction: Default::default(),
            storm: Default::default(),
            travelling: Default::default(),
            forces: Default::default(),
            territory: Default::default(),
        }
    }
}

impl ShipMoveLoss {
    pub fn new(faction: Faction, storm: bool, travelling: bool, forces: i32, territory: Option<Territory>) -> Self {
        ShipMoveLoss { faction, storm, travelling, forces, territory }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct GuildDelayed {
}

impl Default for GuildDelayed {
    fn default() -> Self {
        GuildDelayed {
        }
    }
}

impl GuildDelayed {
}
