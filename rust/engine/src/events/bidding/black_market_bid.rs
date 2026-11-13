use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct BlackMarketBid {
    pub amount: i32,
    pub ally_contribution_amount: i32,
    pub emperor_contribution_amount: i32,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for BlackMarketBid {
    fn default() -> Self {
        BlackMarketBid {
            amount: Default::default(),
            ally_contribution_amount: Default::default(),
            emperor_contribution_amount: Default::default(),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl BlackMarketBid {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn uses_emperor_secret_ally(&self) -> bool {
        false
    }

    pub fn total_amount(&self) -> i32 {
        // C# int addition wraps (hazards 1.6).
        self.amount.wrapping_add(self.ally_contribution_amount).wrapping_add(self.emperor_contribution_amount)
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

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.with_bid_sequence(|seq, g| {
            seq.next_player(g);
            g.skip_players_that_cant_bid(seq);
        });
        game.stone(Milestone::Bid);
        let initiator = self.initiator();
        game.bids.remove(&initiator);
        game.bids.add_with(initiator, self.clone().into_event(), |k| k.to_string());
        let as_event = self.clone().into_event();
        game.check_if_atreides_or_atreides_ally_passed_bid(&as_event);

        if !self.passed {
            game.current_bid = Some(self.clone().into_event());
        }

        match game.current_auction_type {
            AuctionType::BlackMarketNormal => self.handle_black_market_normal_bid(game),
            AuctionType::BlackMarketOnceAround | AuctionType::BlackMarketSilent => self.handle_black_market_special_bid(game),
            _ => Ok(()),
        }
    }

    pub fn handle_black_market_normal_bid(&self, game: &mut Game) -> GameResult {
        if self.passed {
            let winning_bid = game.current_bid.clone();

            if let Some(winning_bid) = winning_bid.as_ref().filter(|wb| game.bid_sequence.as_ref().nre().current_faction(game) == wb.i_bid_initiator()) {
                if game.cards_on_auction.is_none() {
                    return Ok(());
                }

                let card = game.win_by_highest_bid(
                    winning_bid.i_bid_player().required("i bid player")?,
                    winning_bid,
                    winning_bid.i_bid_amount(),
                    winning_bid.i_bid_ally_contribution_amount(),
                    winning_bid.i_bid_emperor_contribution_amount(),
                    if winning_bid.i_bid_initiator() != Faction::Richese { Faction::Richese } else { Faction::Emperor },
                    false,
                )?;

                self.finish_black_market_bid(game, winning_bid.i_bid_player().required("i bid player")?, winning_bid, card);
            } else if winning_bid.is_none() && game.bids.len() as i32 >= game.players_that_can_bid().len() as i32 {
                self.richese_keeps_card(game);
            }
        }
        Ok(())
    }

    /// The shared "no faction bid" branch of both handlers.
    fn richese_keeps_card(&self, game: &mut Game) {
        let richese = self.get_player(game, Some(Faction::Richese));
        let card = game.cards_on_auction.as_mut().map(|d| d.draw());
        if let (Some(richese), Some(card)) = (richese, card) {
            game.player_mut(richese).treachery_cards.push(card);
        }
        self.log_objects(game, vec![Faction::Richese.into(), " keep their card as no faction bid on it".into()]);
        game.enter_richese_bidding();
    }

    pub fn handle_black_market_special_bid(&self, game: &mut Game) -> GameResult {
        let players_with_room = || game.players.iter().filter(|p| p.has_room_for_cards(game)).count() as i32;
        let is_last_bid = (game.current_auction_type == AuctionType::BlackMarketSilent && players_with_room() == game.bids.len() as i32)
            || (game.current_auction_type == AuctionType::BlackMarketOnceAround && self.initiator() == Faction::Richese)
            || (self.get_player(game, Some(Faction::Richese)).map(|w| game.player(w).has_room_for_cards(game)) == Some(false)
                && game.bid_sequence.as_ref().required("bid sequence")?.has_passed_richese(game));

        if is_last_bid {
            if game.current_auction_type == AuctionType::BlackMarketSilent {
                // A List<MessagePart> is one element of the params array.
                let parts: Vec<MessagePart> = game.bids.iter().map(|(k, v)| express_part!(*k, Payment::of(v.i_bid_total_amount()), " ")).collect();
                self.log_objects(game, vec![parts.into()]);
            }

            let highest_bid = game.determine_highest_bid(&game.bids);
            match highest_bid {
                Some(highest_bid) if highest_bid.i_bid_total_amount() > 0 => {
                    if game.cards_on_auction.is_none() {
                        return Ok(());
                    }

                    let card = game.win_by_highest_bid(
                        highest_bid.i_bid_player().required("i bid player")?,
                        &highest_bid,
                        highest_bid.i_bid_amount(),
                        highest_bid.i_bid_ally_contribution_amount(),
                        highest_bid.i_bid_emperor_contribution_amount(),
                        if highest_bid.i_bid_initiator() != Faction::Richese { Faction::Richese } else { Faction::Emperor },
                        false,
                    )?;

                    self.finish_black_market_bid(game, highest_bid.i_bid_player().required("i bid player")?, &highest_bid, card);
                }
                _ => {
                    self.richese_keeps_card(game);
                }
            }
        }
        Ok(())
    }

    pub fn finish_black_market_bid(&self, game: &mut Game, winner: PlayerIndex, winning_bid: &GameEvent, card: TreacheryCard) {
        game.card_just_won = Some(card);
        game.winning_bid = Some(winning_bid.clone());
        game.card_sold_on_black_market = Some(card);
        game.current_bid = None;
        game.faction_that_may_replace_bought_card = Faction::None;

        game.bids.clear();
        game.latest_bid_by_atreides_or_atreides_ally_was_passed = false;

        let mut enter_replacing_card_just_won = game.player(winner).nexus != Faction::None;

        if game.player(winner).ally == Faction::Ixian && game.ixian_allows_replacing_cards {
            if !game.prevented(FactionAdvantage::IxianAllyDiscardingCard) {
                game.faction_that_may_replace_bought_card = game.player(winner).faction;
                enter_replacing_card_just_won = true;
            } else {
                if !game.applicable(Rule::FullPhaseKarma) {
                    game.allow(FactionAdvantage::IxianAllyDiscardingCard);
                }

                if game.nexus_allows_replacing_bought_cards(winner) {
                    game.faction_that_may_replace_bought_card = game.player(winner).faction;
                    enter_replacing_card_just_won = true;
                    game.replacing_bought_card_using_nexus = true;
                }
            }
        } else if game.nexus_allows_replacing_bought_cards(winner) {
            game.faction_that_may_replace_bought_card = game.player(winner).faction;
            enter_replacing_card_just_won = true;
            game.replacing_bought_card_using_nexus = true;
        }

        game.enter_boolean_phase_action(enter_replacing_card_just_won, Phase::ReplacingCardJustWon, Game::enter_richese_bidding);

        if let Some(b) = game.bidding_triggered_bureaucracy.clone() {
            game.apply_bureaucracy(b.payment_from, b.payment_to);
            game.bidding_triggered_bureaucracy = None;
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if !self.passed {
            return express!(self.initiator(), " bid");
        }

        express!(self.initiator(), " pass")
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.passed {
            return None;
        }

        if game.current_auction_type != AuctionType::BlackMarketSilent && self.total_amount() < 1 {
            return Some(express!("Bid must be higher than 0"));
        }
        if game.current_auction_type != AuctionType::BlackMarketSilent {
            if let Some(current_bid) = &game.current_bid {
                if self.total_amount() <= current_bid.i_bid_total_amount() {
                    return Some(express!("Bid not high enough"));
                }
            }
        }

        // `Player` is dereferenced (p.Faction) inside ResourcesYourAllyCanPay: NullReferenceException when null.
        if self.ally_contribution_amount > BlackMarketBid::valid_max_ally_amount(game, self.player()) {
            return Some(express!("your ally won't pay that much"));
        }

        let emperor = game.get_player(Some(Faction::Emperor));
        if self.emperor_contribution_amount > 0 && self.emperor_contribution_amount > emperor.map_or(0, |r| game.player(r).resources) {
            return Some(express!(Faction::Emperor, " won't pay that much"));
        }

        if game.current_auction_type == AuctionType::BlackMarketSilent && self.total_amount() > game.player(self.player()).resources {
            return Some(express!("In a Silent auction, you can't bid more than you have"));
        }

        None
    }

    pub fn valid_max_amount(game: &Game, p: PlayerIndex) -> i32 {
        game.player(p).resources
    }

    pub fn valid_max_ally_amount(g: &Game, p: PlayerIndex) -> i32 {
        g.resources_your_ally_can_pay(p)
    }

    pub fn players_to_bid(g: &Game) -> Vec<SequenceElement> {
        // GetPlayersInSequence runs now (PlayersInOrder); its selector runs only in the arms that return it.
        let players_in_sequence = match &g.bid_sequence {
            Some(s) => either::Either::Left(s.get_players_in_sequence(g)),
            None => either::Either::Right(std::iter::empty()),
        };
        match g.current_auction_type {
            AuctionType::BlackMarketNormal | AuctionType::BlackMarketOnceAround => players_in_sequence.collect(),
            AuctionType::BlackMarketSilent => g
                .players
                .iter()
                .enumerate()
                .map(|(i, p)| SequenceElement { player: Some(g.player_at(i)), has_turn: p.has_room_for_cards(g) && !g.bids.contains_key(&p.faction) })
                .collect(),
            _ => Vec::new(),
        }
    }

    pub fn may_be_played(game: &Game, player: PlayerIndex) -> bool {
        (game.current_auction_type == AuctionType::BlackMarketSilent && !game.bids.contains_key(&game.player(player).faction) && game.player(player).has_room_for_cards(game))
            || (game.current_auction_type != AuctionType::BlackMarketSilent && game.bid_sequence.as_ref().map(|s| s.current_player(game)) == Some(Some(player)))
    }
}

impl GameEventMethods for BlackMarketBid {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::BlackMarketBid
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::BlackMarketBid(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        BlackMarketBid::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        BlackMarketBid::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        BlackMarketBid::get_message(self, game)
    }

}

impl PassableGameEventMethods for BlackMarketBid {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}

impl crate::events::bid_trait::IBid for BlackMarketBid {
    fn total_amount(&self) -> i32 {
        BlackMarketBid::total_amount(self)
    }

    fn amount(&self) -> i32 {
        self.amount
    }

    fn ally_contribution_amount(&self) -> i32 {
        self.ally_contribution_amount
    }

    fn emperor_contribution_amount(&self) -> i32 {
        self.emperor_contribution_amount
    }

    fn uses_emperor_secret_ally(&self) -> bool {
        BlackMarketBid::uses_emperor_secret_ally(self)
    }

    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self)
    }

    fn player(&self) -> Option<PlayerIndex> {
        GameEventMethods::player_opt(self)
    }

    fn passed(&self) -> bool {
        self.passed
    }

    fn using_karma_to_remove_bid_limit(&self) -> bool {
        BlackMarketBid::using_karma_to_remove_bid_limit(self)
    }

    fn karma_card(&self) -> Option<TreacheryCard> {
        BlackMarketBid::karma_card(self)
    }

    fn get_message(&self, game: &Game) -> Message {
        BlackMarketBid::get_message(self, game)
    }

    fn karma_bid(&self) -> bool {
        BlackMarketBid::karma_bid(self)
    }

}
