use crate::prelude::*;

impl ClassicBot {
    pub fn determine_bid(&self, game: &Game) -> Rc<Bid> {
        Self::log_info("DetermineBid()", vec![]);

        let me = self.player;
        let other_players = game.player_indices().filter(|&p| p != me && game.player(p).has_room_for_cards(game));
        let bidding_players: Vec<PlayerIndex> = other_players
            .filter(|&p| (game.player(p).treachery_cards.len() as i32) < game.player(p).maximum_number_of_cards(game))
            .collect();
        let max_bid_of_other_player = if bidding_players.is_empty() {
            0
        } else {
            bidding_players.iter().map(|&p| game.player(p).resources).max().unwrap_or(0)
        };

        let current_bid = game.current_bid.as_ref().map(|b| b.i_bid_total_amount()).unwrap_or(0);
        let current_bid_is_from_ally = game.current_bid.as_ref().is_some_and(|b| b.initiator() == self.ally(game));
        // Game.CardsOnAuction.Top, read where C# reads it (CardsOnAuction is never null while bidding).
        let top = || game.cards_on_auction.as_ref().nre().top().or_null();
        let is_known_card = game.has_bidding_prescience(me)
            || (game.player(me).has_ally() && game.has_bidding_prescience(self.allied_player(game).or_null()))
            || game.known_cards(me).contains(&top());

        let this_card_is_useless = is_known_card && !self.may_use_useless_as_karma(game) && top().type_() == TreacheryCardType::Useless;
        let this_card_is_crappy = is_known_card && !self.wanna_have(game, top());
        let this_card_is_perfect = is_known_card && self.card_quality(game, top(), Some(me)) == 5;

        let resources_to_keep = if this_card_is_perfect {
            self.param.bidding_resources_to_keep_when_card_is_perfect
        } else {
            self.param.bidding_resources_to_keep_when_card_isnt_perfect
        };
        let resources_available = 0i32
            .max(self.resources(game).wrapping_sub(resources_to_keep))
            .wrapping_add(self.resources_from_ally(game))
            .wrapping_add(self.resources_from_emperor(game));
        let player = game.player(me);
        let could_use_karma_for_bid = game.current_auction_type == AuctionType::Normal
            && !game.karma_prevented(self.faction(game))
            && self.ally(game) != Faction::Emperor
            && (player.special_karma_power_used
                || !self.param.karma_save_card_to_use_special_karma_ability
                || game.current_turn >= 5
                || player.maximum_number_of_cards(game).wrapping_sub(player.treachery_cards.len() as i32) <= 1);
        let karma_card_to_use_for_bidding = if could_use_karma_for_bid
            && ((self.may_use_useless_as_karma(game) && current_bid > 2) || current_bid > 4 || resources_available <= current_bid)
        {
            player.treachery_cards.iter().copied().find(|c| {
                c.type_() == TreacheryCardType::Karma || (self.may_use_useless_as_karma(game) && c.type_() == TreacheryCardType::Useless)
            })
        } else {
            None
        };
        let karma_worth = if karma_card_to_use_for_bidding.is_none() { 0 } else { 8 };
        let maximum_i_will_spend = self.d(game, 1, resources_available.min(12).wrapping_add(karma_worth));
        let amount_to_bid_in_silent_or_once_around = self.d(game, 1, resources_available.min(8));

        // LogInfo's arguments are plain values here: nothing to evaluate.
        Self::log_info("currentBidIsFromAlly: {0}, PlayerCardIsUseless: {1}, PlayerCardIsCrappy: {2}, resourcesAvailable:, {3}, karmaWorth: {4}, maximumIWillSpend: {5}, karmaCardToUseForBidding: {6}.", vec![]);

        let auction = game.current_auction_type;
        if (self.faction(game) == Faction::Richese || self.ally(game) == Faction::Richese)
            && (auction == AuctionType::BlackMarketSilent || auction == AuctionType::BlackMarketNormal || auction == AuctionType::BlackMarketOnceAround)
        {
            Self::log_info("Player is my own card", vec![]);
            if auction == AuctionType::BlackMarketSilent {
                return self.create_bid_using_ally_and_emperor_spice(game, 0, None);
            }
            return self.passed_bid(game);
        }

        if auction == AuctionType::BlackMarketSilent || auction == AuctionType::RicheseSilent {
            if this_card_is_useless || this_card_is_crappy {
                let to_bid = if this_card_is_useless || 0i32.max(self.resources(game).wrapping_sub(resources_to_keep)) < 2 {
                    0
                } else {
                    self.d(game, 1, 2)
                };
                return self.create_bid_using_ally_and_emperor_spice(game, to_bid, None);
            }

            return self.create_bid_using_ally_and_emperor_spice(
                game,
                0i32.max(self.resources(game).wrapping_sub(resources_to_keep)).min(amount_to_bid_in_silent_or_once_around),
                None,
            );
        }

        if current_bid_is_from_ally
            || (this_card_is_useless && current_bid > 0)
            || (this_card_is_crappy && self.d(game, 1, 1i32.wrapping_add(2i32.wrapping_mul(current_bid))) > 1)
        {
            Self::log_info("currentBidIsFromAlly or PlayerCardIsUseless or PlayerCardIsCrappy", vec![]);
            return self.passed_bid(game);
        }

        if auction == AuctionType::BlackMarketOnceAround || auction == AuctionType::RicheseOnceAround {
            return if amount_to_bid_in_silent_or_once_around > current_bid {
                self.create_bid_using_ally_and_emperor_spice(game, amount_to_bid_in_silent_or_once_around, None)
            } else {
                self.passed_bid(game)
            };
        }

        if current_bid == 0 && self.resources_including_ally_and_emperor_contribution(game) > 0 {
            Self::log_info("always bid at least 1 if possible", vec![]);
            return self.create_bid_using_ally_and_emperor_spice(game, 1, None);
        }

        if self.ally(game) != Faction::Emperor && current_bid > self.param.bidding_passing_treshold.wrapping_add(maximum_i_will_spend) {
            Self::log_info("Ally != Faction.Emperor && currentBid ({0}) > Param.Bidding_PassingThreshold ({1}) + maximumIWillSpend (D(1,{2}+{3}) => {4})", vec![]);
            return self.passed_bid(game);
        }

        if current_bid.wrapping_add(2) == max_bid_of_other_player
            && (karma_card_to_use_for_bidding.is_some() || current_bid.wrapping_add(2) <= resources_available)
        {
            Self::log_info("currentBid ({0}) + 2 == maxBidOfOtherPlayer ({1}) && (karmaCardToUseForBidding ({2}) != null || currentBid + 2 <= resourcesAvailable ({3}))", vec![]);
            return self.create_bid_using_ally_and_emperor_spice(game, max_bid_of_other_player, karma_card_to_use_for_bidding);
        }

        if current_bid.wrapping_add(1) <= resources_available || karma_card_to_use_for_bidding.is_some() {
            Self::log_info("currentBid ({0}) + 1 <= resourcesAvailable ({1}) || karmaCardToUseForBidding ({2}) != null", vec![]);
            return self.create_bid_using_ally_and_emperor_spice(game, current_bid.wrapping_add(1), karma_card_to_use_for_bidding);
        }

        Self::log_info("Not enough spice available for bid", vec![]);
        self.passed_bid(game)
    }

    pub fn passed_bid(&self, game: &Game) -> Rc<Bid> {
        let mut bid = Bid::new(game, self.faction(game));
        bid.passed = true;
        Rc::new(bid)
    }

    pub fn create_bid_using_ally_and_emperor_spice(&self, game: &Game, amount: i32, karma_card: Option<TreacheryCard>) -> Rc<Bid> {
        let karma_card = karma_card.and_then(treachery_engine::compat::cs::CsNull::opt);
        if let Some(karma_card) = karma_card {
            let mut bid = Bid::new(game, self.faction(game));
            bid.amount = amount;
            bid.karma_bid = false;
            bid.set_karma_card(Some(karma_card));
            bid.passed = false;
            return Rc::new(bid);
        }

        let use_emperor_secret_ally =
            amount > 6 && game.player(self.player).forces_killed < 12 && Bid::may_use_emperor_secret_ally(game, self.player);

        let mut spice_left_to_pay = amount;
        let emperor_contribution = spice_left_to_pay.min(game.spice_for_bids_emperor_can_pay(self.faction(game)));
        spice_left_to_pay = spice_left_to_pay.wrapping_sub(emperor_contribution);

        let ally_contribution = spice_left_to_pay.min(game.resources_your_ally_can_pay(self.player));
        spice_left_to_pay = spice_left_to_pay.wrapping_sub(ally_contribution);

        let mut bid = Bid::new(game, self.faction(game));
        bid.amount = spice_left_to_pay;
        bid.ally_contribution_amount = ally_contribution;
        bid.emperor_contribution_amount = emperor_contribution;
        bid.karma_bid = false;
        bid.set_karma_card(None);
        bid.passed = false;
        bid.uses_emperor_secret_ally = use_emperor_secret_ally;
        Rc::new(bid)
    }

    pub fn wanna_have(&self, game: &Game, c: TreacheryCard) -> bool {
        let player = game.player(self.player);
        let cards = &player.treachery_cards;
        c.is_laser()
            || c.type_() == TreacheryCardType::ProjectileAndPoison
            || c.type_() == TreacheryCardType::ShieldAndAntidote
            || c.type_() == TreacheryCardType::Chemistry
            || c.type_() == TreacheryCardType::WeirdingWay
            || c.type_() == TreacheryCardType::ArtilleryStrike
            || c.type_() == TreacheryCardType::PoisonTooth
            || c.type_() == TreacheryCardType::Amal
            || (c.is_projectile_weapon() && !cards.iter().any(|x| x.is_projectile_weapon()))
            || (c.is_poison_weapon() && !cards.iter().any(|x| x.is_poison_weapon() && x.type_() != TreacheryCardType::Chemistry))
            || (c.is_projectile_defense() && !cards.iter().any(|x| x.is_projectile_defense() && x.type_() != TreacheryCardType::WeirdingWay))
            || (c.is_poison_defense() && !cards.iter().any(|x| x.is_poison_defense()))
            || c.type_() == TreacheryCardType::Karma
            || (c.type_() == TreacheryCardType::Mercenary && player.leaders.iter().filter(|&&l| game.is_alive(Hero::Leader(l))).count() <= 1)
            || (c.type_() == TreacheryCardType::RaiseDead && player.forces_killed > 7)
            || c.type_() == TreacheryCardType::SearchDiscarded
            || c.type_() == TreacheryCardType::TakeDiscarded
            || c.type_() == TreacheryCardType::PortableAntidote
            || c.type_() == TreacheryCardType::Rockmelter
    }

    pub fn determine_black_market_bid(&self, game: &Game) -> Rc<BlackMarketBid> {
        let bid = self.determine_bid(game);
        let mut result = BlackMarketBid::new(game, self.faction(game));
        result.amount = bid.amount;
        result.ally_contribution_amount = bid.ally_contribution_amount;
        result.emperor_contribution_amount = bid.emperor_contribution_amount;
        result.passed = bid.passed;
        Rc::new(result)
    }
}
