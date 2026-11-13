use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct BiddingDrawn {
    pub count: i32,
}

impl Default for BiddingDrawn {
    fn default() -> Self {
        BiddingDrawn {
            count: Default::default(),
        }
    }
}

impl BiddingDrawn {
    pub fn new(count: i32) -> Self {
        BiddingDrawn { count }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct TreacheryReshuffled {
    pub count: i32,
}

impl Default for TreacheryReshuffled {
    fn default() -> Self {
        TreacheryReshuffled {
            count: Default::default(),
        }
    }
}

impl TreacheryReshuffled {
    pub fn new(count: i32) -> Self {
        TreacheryReshuffled { count }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct BiddingSkipped {
}

impl Default for BiddingSkipped {
    fn default() -> Self {
        BiddingSkipped {
        }
    }
}

impl BiddingSkipped {
}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct BidShare {
    pub by: Faction,
    pub amount: i32,
}

impl Default for BidShare {
    fn default() -> Self {
        BidShare {
            by: Default::default(),
            amount: Default::default(),
        }
    }
}

impl BidShare {
    pub fn new(by: Faction, amount: i32) -> Self {
        BidShare { by, amount }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct CardSold {
    pub card_number: i32,
    pub winner: Faction,
    pub price: i32,
    pub paid_to: Faction,
    pub shares: Vec<BidShare>,
    pub karama_card: Option<TreacheryCard>,
}

impl Default for CardSold {
    fn default() -> Self {
        CardSold {
            card_number: Default::default(),
            winner: Default::default(),
            price: Default::default(),
            paid_to: Default::default(),
            shares: Default::default(),
            karama_card: Default::default(),
        }
    }
}

impl CardSold {
    pub fn new(card_number: i32, winner: Faction, price: i32, paid_to: Faction, shares: Vec<BidShare>, karama_card: Option<TreacheryCard>) -> Self {
        CardSold { card_number, winner, price, paid_to, shares, karama_card }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct CardWonShown {
    pub card: TreacheryCard,
}

impl CardWonShown {
    pub fn new(card: TreacheryCard) -> Self {
        CardWonShown { card }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct ExtraCardReceived {
    pub receiver: Faction,
}

impl Default for ExtraCardReceived {
    fn default() -> Self {
        ExtraCardReceived {
            receiver: Default::default(),
        }
    }
}

impl ExtraCardReceived {
    pub fn new(receiver: Faction) -> Self {
        ExtraCardReceived { receiver }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct ExtraCardShown {
    pub card: TreacheryCard,
}

impl ExtraCardShown {
    pub fn new(card: TreacheryCard) -> Self {
        ExtraCardShown { card }
    }

}

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct BiddingPassedByAll {
    pub card_number: i32,
}

impl Default for BiddingPassedByAll {
    fn default() -> Self {
        BiddingPassedByAll {
            card_number: Default::default(),
        }
    }
}

impl BiddingPassedByAll {
    pub fn new(card_number: i32) -> Self {
        BiddingPassedByAll { card_number }
    }

}
