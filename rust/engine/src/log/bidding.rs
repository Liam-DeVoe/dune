use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct BiddingDrawn {
    pub count: i32,
}

impl BiddingDrawn {
    pub fn new(count: i32) -> Self {
        BiddingDrawn { count }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct TreacheryReshuffled {
    pub count: i32,
}

impl TreacheryReshuffled {
    pub fn new(count: i32) -> Self {
        TreacheryReshuffled { count }
    }

}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct BiddingSkipped {
}

impl BiddingSkipped {
}

#[derive(Clone, Debug, PartialEq)]
pub struct BidShare {
    pub by: Option<Faction>,
    pub amount: i32,
}

impl BidShare {
    pub fn new(by: impl Into<Option<Faction>>, amount: i32) -> Self {
        BidShare { by: by.into(), amount }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct CardSold {
    pub card_number: i32,
    pub winner: Faction,
    pub price: i32,
    pub paid_to: Option<Faction>,
    pub shares: Vec<BidShare>,
    pub karama_card: Option<TreacheryCard>,
}

impl CardSold {
    pub fn new(card_number: i32, winner: Faction, price: i32, paid_to: Option<Faction>, shares: Vec<BidShare>, karama_card: Option<TreacheryCard>) -> Self {
        CardSold { card_number, winner, price, paid_to, shares, karama_card }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct CardWonShown {
    pub card: TreacheryCard,
}

impl CardWonShown {
    pub fn new(card: TreacheryCard) -> Self {
        CardWonShown { card }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct ExtraCardReceived {
    pub receiver: Faction,
}

impl ExtraCardReceived {
    pub fn new(receiver: Faction) -> Self {
        ExtraCardReceived { receiver }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct ExtraCardShown {
    pub card: TreacheryCard,
}

impl ExtraCardShown {
    pub fn new(card: TreacheryCard) -> Self {
        ExtraCardShown { card }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub struct BiddingPassedByAll {
    pub card_number: i32,
}

impl BiddingPassedByAll {
    pub fn new(card_number: i32) -> Self {
        BiddingPassedByAll { card_number }
    }

}
