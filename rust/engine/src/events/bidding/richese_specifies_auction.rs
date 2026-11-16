use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct RicheseSpecifiesAuction {
    pub auction_type: Option<AuctionType>,
    pub direction: i32,
    pub card: Option<TreacheryCard>,
}

impl Default for RicheseSpecifiesAuction {
    fn default() -> Self {
        RicheseSpecifiesAuction {
            auction_type: Default::default(),
            direction: Default::default(),
            card: TreacheryCardManager::get(0),
        }
    }
}

impl RicheseSpecifiesAuction {
    pub fn valid_cards(g: &Game) -> Vec<TreacheryCard> {
        g.decks.richese_cache.clone()
    }

    pub fn may_specify_card(g: &Game, p: PlayerId) -> bool {
        let occupier_of_richese_homeworld = g.occupier_of(World::Richese);
        (occupier_of_richese_homeworld.is_none() && g.player(p).faction == Some(Faction::Richese))
            || (occupier_of_richese_homeworld.is_some() && !g.bidding.richese_occupier_specified_card && g.player(occupier_of_richese_homeworld.expect("occupier of richese homeworld")).is(g.player(p).faction))
    }

    pub fn may_specify_type_of_auction(g: &Game, p: PlayerId) -> bool {
        let occupier_of_richese_homeworld = g.occupier_of(World::Richese);
        (occupier_of_richese_homeworld.is_none() && g.player(p).faction == Some(Faction::Richese))
            || (occupier_of_richese_homeworld.is_some() && g.bidding.richese_occupier_specified_card && g.player(p).faction == Some(Faction::Richese))
    }
}

impl EventRules for RicheseSpecifiesAuction {
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        if !game.bidding.richese_occupier_specified_card && !RicheseSpecifiesAuction::valid_cards(game).iter().any(|c| Some(*c) == self.card) {
            return Err(Rejection(express!("Invalid card")));
        }

        if self.auction_type != Some(AuctionType::RicheseOnceAround) && self.auction_type != Some(AuctionType::RicheseSilent) {
            return Err(Rejection(express!("Invalid auction type")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        if !game.bidding.richese_occupier_specified_card {
            game.bidding.richese_auction_should_still_happen = false;
            let card = self.card.required("card")?;
            game.bidding.cards_on_auction.as_mut().required("cards on auction")?.put_on_top(card);
            crate::game::remove_card(&mut game.decks.richese_cache, Some(card));
            game.drop_copy_in_treachery_deck(card);
            game.register_known(card);

            let occupier_of_richese_homeworld = game.occupier_of(World::Richese);
            if occupier_of_richese_homeworld.is_none() {
                let m = self.describe(game, by);
                game.log_event(m);
                game.start_bid_sequence_and_auction_type(self.auction_type, by.player, self.direction);
                game.start_bidding_round();
            } else {
                game.log(vec![by.faction.into(), " put ".into(), card.into(), " on auction".into()]);
                game.bidding.richese_occupier_specified_card = true;
            }
        } else {
            let mut direction_text = "";
            if self.auction_type == Some(AuctionType::RicheseOnceAround) {
                if self.direction == 1 {
                    direction_text = " (counter-clockwise)";
                } else {
                    direction_text = " (clockwise)";
                }
            }

            game.log(vec![by.faction.into(), " select a ".into(), self.auction_type.into(), " auction".into(), direction_text.into()]);

            game.start_bid_sequence_and_auction_type(self.auction_type, by.player, self.direction);
            game.start_bidding_round();
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        let mut direction_text = "";
        if self.auction_type == Some(AuctionType::RicheseOnceAround) {
            if self.direction == 1 {
                direction_text = " (counter-clockwise)";
            } else {
                direction_text = " (clockwise)";
            }
        }

        express!(by.faction, " put a ", Faction::Richese, " card on ", self.auction_type, " auction", direction_text)
    }
}
