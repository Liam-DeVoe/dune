use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct BribesCollected {
    pub faction: Faction,
    pub amount: i32,
}

impl Default for BribesCollected {
    fn default() -> Self {
        BribesCollected {
            faction: Default::default(),
            amount: Default::default(),
        }
    }
}

impl BribesCollected {
    pub fn new(faction: Faction, amount: i32) -> Self {
        BribesCollected { faction, amount }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct WinReason {
    pub method: WinMethod,
}

impl Default for WinReason {
    fn default() -> Self {
        WinReason {
            method: Default::default(),
        }
    }
}

impl WinReason {
    pub fn new(method: WinMethod) -> Self {
        WinReason { method }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct GameEnded {
    pub winners: Vec<Faction>,
}

impl Default for GameEnded {
    fn default() -> Self {
        GameEnded {
            winners: Default::default(),
        }
    }
}

impl GameEnded {
    pub fn new(winners: Vec<Faction>) -> Self {
        GameEnded { winners }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct GameWinner {
    pub faction: Faction,
}

impl Default for GameWinner {
    fn default() -> Self {
        GameWinner {
            faction: Default::default(),
        }
    }
}

impl GameWinner {
    pub fn new(faction: Faction) -> Self {
        GameWinner { faction }
    }

}
