use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct BlackMarketBid {
    pub amount: u16,
    pub ally_contribution_amount: u16,
    pub emperor_contribution_amount: u16,
    #[serde(skip)]
    pub passed: bool,
}

impl BlackMarketBid {
    /// The bid as the auction keeps it, numbered as the game's next bid.
    pub fn placed(&self, game: &mut Game, by: By) -> PlacedBid {
        PlacedBid {
            number: game.number_bid(),
            type_: EventType::BlackMarketBid,
            initiator: by.faction,
            player: by.player,
            amount: self.amount,
            ally_contribution_amount: self.ally_contribution_amount,
            emperor_contribution_amount: self.emperor_contribution_amount,
            uses_emperor_secret_ally: self.uses_emperor_secret_ally(),
            karma_card: self.karma_card(),
            karma_bid: self.karma_bid(),
            passed: self.passed,
        }
    }

    pub fn uses_emperor_secret_ally(&self) -> bool {
        false
    }

    /// What the bid offers: nothing for a pass. Not in C#, where a pass in a silent or once-around
    /// auction counted its amounts, which validation never checks, so a pass could win and pay
    /// more than the bidder had (behavior-changes.md, "Turns, sectors and amounts out of range").
    pub fn total_amount(&self) -> i32 {
        if self.passed {
            return 0;
        }
        (i32::from(self.amount) + i32::from(self.ally_contribution_amount)) + i32::from(self.emperor_contribution_amount)
    }

    pub fn using_karma_to_remove_bid_limit(&self) -> bool {
        false
    }

    pub fn karma_card(&self) -> Option<TreacheryCard> {
        None
    }

    pub fn karma_bid(&self) -> bool {
        false
    }

    fn execute(&self, game: &mut Game, by: By) -> GameResult {
        let this = self.placed(game, by);
        game.with_bid_sequence(|seq, g| {
            seq.next_player(g);
            g.skip_players_that_cant_bid(seq);
        });
        game.stone(Milestone::Bid);
        let initiator = by.faction;
        game.in_bidding_mut().bids.shift_remove_opt(initiator);
        game.in_bidding_mut().bids.insert_new(initiator.required("faction")?, this.clone());
        game.check_if_atreides_or_atreides_ally_passed_bid(&this);

        if !self.passed {
            game.in_bidding_mut().current_bid = Some(this);
        }

        match game.in_bidding().current_auction_type {
            Some(AuctionType::BlackMarketNormal) => self.handle_black_market_normal_bid(game),
            Some(AuctionType::BlackMarketOnceAround) | Some(AuctionType::BlackMarketSilent) => self.handle_black_market_special_bid(game, by),
            _ => Ok(()),
        }
    }

    fn handle_black_market_normal_bid(&self, game: &mut Game) -> GameResult {
        if self.passed {
            let winning_bid = game.in_bidding().current_bid.clone();

            if let Some(winning_bid) = winning_bid.as_ref().filter(|wb| game.in_bidding().bid_sequence.as_ref().expect("bid sequence").current_faction(game) == wb.bid_initiator()) {
                if game.in_bidding().cards_on_auction.is_none() {
                    return Ok(());
                }

                let card = game.win_by_highest_bid(
                    winning_bid.bid_player().required("i bid player")?,
                    winning_bid,
                    winning_bid.bid_amount(),
                    winning_bid.bid_ally_contribution_amount(),
                    winning_bid.bid_emperor_contribution_amount(),
                    if winning_bid.bid_initiator() != Faction::Richese { Faction::Richese } else { Faction::Emperor },
                    false,
                )?;

                BlackMarketBid::finish_black_market_bid(game, winning_bid.bid_player().required("i bid player")?, winning_bid, card);
            } else if winning_bid.is_none() && game.in_bidding().bids.len() as i32 >= game.players_that_can_bid().len() as i32 {
                BlackMarketBid::richese_keeps_card(game);
            }
        }
        Ok(())
    }

    /// The shared "no faction bid" branch of both handlers.
    fn richese_keeps_card(game: &mut Game) {
        let richese = game.get_player(Some(Faction::Richese));
        let card = game.in_bidding_mut().cards_on_auction.as_mut().map(|d| d.draw());
        if let (Some(richese), Some(card)) = (richese, card) {
            game.player_mut(richese).treachery_cards.push(card);
        }
        game.log(vec![Faction::Richese.into(), " keep their card as no faction bid on it".into()]);
        game.enter_richese_bidding();
    }

    fn handle_black_market_special_bid(&self, game: &mut Game, by: By) -> GameResult {
        let players_with_room = || game.players.iter().filter(|p| p.has_room_for_cards(game)).count() as i32;
        let is_last_bid = (game.in_bidding().current_auction_type == Some(AuctionType::BlackMarketSilent) && players_with_room() == game.in_bidding().bids.len() as i32)
            || (game.in_bidding().current_auction_type == Some(AuctionType::BlackMarketOnceAround) && by.faction == Some(Faction::Richese))
            || (game.get_player(Some(Faction::Richese)).map(|w| game.player(w).has_room_for_cards(game)) == Some(false)
                && game.in_bidding().bid_sequence.as_ref().required("bid sequence")?.has_passed_richese(game));

        if is_last_bid {
            if game.in_bidding().current_auction_type == Some(AuctionType::BlackMarketSilent) {
                // A List<MessagePart> is one element of the params array.
                let parts: Vec<MessagePart> = game.in_bidding().bids.iter().map(|(k, v)| express_part!(*k, Payment::of(v.bid_total_amount()), " ")).collect();
                game.log(vec![parts.into()]);
            }

            let highest_bid = game.determine_highest_bid(&game.in_bidding().bids);
            match highest_bid {
                Some(highest_bid) if highest_bid.bid_total_amount() > 0 => {
                    if game.in_bidding().cards_on_auction.is_none() {
                        return Ok(());
                    }

                    let card = game.win_by_highest_bid(
                        highest_bid.bid_player().required("i bid player")?,
                        &highest_bid,
                        highest_bid.bid_amount(),
                        highest_bid.bid_ally_contribution_amount(),
                        highest_bid.bid_emperor_contribution_amount(),
                        if highest_bid.bid_initiator() != Faction::Richese { Faction::Richese } else { Faction::Emperor },
                        false,
                    )?;

                    BlackMarketBid::finish_black_market_bid(game, highest_bid.bid_player().required("i bid player")?, &highest_bid, card);
                }
                _ => {
                    BlackMarketBid::richese_keeps_card(game);
                }
            }
        }
        Ok(())
    }

    fn finish_black_market_bid(game: &mut Game, winner: PlayerId, winning_bid: &PlacedBid, card: TreacheryCard) {
        game.in_bidding_mut().card_just_won = Some(card);
        game.in_bidding_mut().winning_bid = Some(winning_bid.clone());
        game.in_bidding_mut().card_sold_on_black_market = Some(card);
        game.in_bidding_mut().current_bid = None;
        game.in_bidding_mut().faction_that_may_replace_bought_card = None;

        game.in_bidding_mut().bids.clear();
        game.in_bidding_mut().latest_bid_by_atreides_or_atreides_ally_was_passed = false;

        let mut enter_replacing_card_just_won = game.winner_might_replace_with_nexus();

        if game.player(winner).ally == Some(Faction::Ixian) && game.alliances.ixian_allows_replacing_cards {
            if !game.prevented(FactionAdvantage::IxianAllyDiscardingCard) {
                game.in_bidding_mut().faction_that_may_replace_bought_card = game.player(winner).faction;
                enter_replacing_card_just_won = true;
            } else {
                if !game.applicable(Rule::FullPhaseKarma) {
                    game.allow(FactionAdvantage::IxianAllyDiscardingCard);
                }

                if game.nexus_allows_replacing_bought_cards(winner) {
                    game.in_bidding_mut().faction_that_may_replace_bought_card = game.player(winner).faction;
                    enter_replacing_card_just_won = true;
                    game.in_bidding_mut().replacing_bought_card_using_nexus = true;
                }
            }
        } else if game.nexus_allows_replacing_bought_cards(winner) {
            game.in_bidding_mut().faction_that_may_replace_bought_card = game.player(winner).faction;
            enter_replacing_card_just_won = true;
            game.in_bidding_mut().replacing_bought_card_using_nexus = true;
        }

        if enter_replacing_card_just_won {

            game.enter_bidding(BiddingStep::ReplacingCardJustWon);

        } else {

            game.enter_richese_bidding();

        }

        if let Some(b) = game.in_bidding().bidding_triggered_bureaucracy.clone() {
            game.apply_bureaucracy(b.payment_from, b.payment_to);
            game.in_bidding_mut().bidding_triggered_bureaucracy = None;
        }
    }

    fn validate_bid(&self, game: &Game, by: By) -> Option<Message> {
        if self.passed {
            return None;
        }

        if game.in_bidding().current_auction_type != Some(AuctionType::BlackMarketSilent) && self.total_amount() < 1 {
            return Some(express!("Bid must be higher than 0"));
        }
        if game.in_bidding().current_auction_type != Some(AuctionType::BlackMarketSilent) {
            if let Some(current_bid) = &game.in_bidding().current_bid {
                if self.total_amount() <= current_bid.bid_total_amount() {
                    return Some(express!("Bid not high enough"));
                }
            }
        }

        // `Player` is dereferenced (p.Faction) inside ResourcesYourAllyCanPay: NullReferenceException when null.
        if i32::from(self.ally_contribution_amount) > BlackMarketBid::valid_max_ally_amount(game, by.player()) {
            return Some(express!("your ally won't pay that much"));
        }

        // Not in C#, which checked only the Emperor's spice, not what the Emperor offered
        // (behavior-changes.md, "Turns, sectors and amounts out of range").
        if i32::from(self.emperor_contribution_amount) > 0 && i32::from(self.emperor_contribution_amount) > by.faction.map_or(0, |f| game.spice_for_bids_emperor_can_pay(f)) {
            return Some(express!(Faction::Emperor, " won't pay that much"));
        }

        // Not in C#, where a normal Black Market bid could be more than the bidder had, and
        // paying it left them with less than zero (behavior-changes.md, "Turns, sectors and
        // amounts out of range"). `Bid` checks the same.
        if i32::from(self.amount) > i32::from(game.player(by.player()).resources) {
            return Some(express!("You can't pay ", Payment::of(i32::from(self.amount))));
        }
        if game.in_bidding().current_auction_type == Some(AuctionType::BlackMarketSilent) && self.total_amount() > i32::from(game.player(by.player()).resources) {
            return Some(express!("In a Silent auction, you can't bid more than you have"));
        }

        None
    }

    pub fn valid_max_amount(game: &Game, p: PlayerId) -> i32 {
        i32::from(game.player(p).resources)
    }

    pub fn valid_max_ally_amount(g: &Game, p: PlayerId) -> i32 {
        g.resources_your_ally_can_pay(p)
    }

    pub fn players_to_bid(g: &Game) -> Vec<SequenceElement> {
        // GetPlayersInSequence runs now (PlayersInOrder); its selector runs only in the arms that return it.
        let players_in_sequence = match &g.in_bidding().bid_sequence {
            Some(s) => either::Either::Left(s.get_players_in_sequence(g)),
            None => either::Either::Right(std::iter::empty()),
        };
        match g.in_bidding().current_auction_type {
            Some(AuctionType::BlackMarketNormal) | Some(AuctionType::BlackMarketOnceAround) => players_in_sequence.collect(),
            Some(AuctionType::BlackMarketSilent) => g
                .players
                .iter()
                .enumerate()
                .map(|(i, p)| SequenceElement { player: Some(g.player_at(i)), has_turn: p.has_room_for_cards(g) && !g.in_bidding().bids.contains_key_opt(p.faction) })
                .collect(),
            _ => Vec::new(),
        }
    }

    pub fn may_be_played(game: &Game, player: PlayerId) -> bool {
        (game.in_bidding().current_auction_type == Some(AuctionType::BlackMarketSilent) && !game.in_bidding().bids.contains_key_opt(game.player(player).faction) && game.player(player).has_room_for_cards(game))
            || (game.in_bidding().current_auction_type != Some(AuctionType::BlackMarketSilent) && game.in_bidding().bid_sequence.as_ref().map(|s| s.current_player(game)) == Some(Some(player)))
    }
}

impl EventRules for BlackMarketBid {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        match self.validate_bid(game, by) {
            Some(m) => Err(Rejection(m)),
            None => Ok(()),
        }
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        self.execute(game, by)
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if !self.passed {
            return express!(by.faction, " bid");
        }

        express!(by.faction, " pass")
    }
}
