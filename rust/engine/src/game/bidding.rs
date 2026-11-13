use crate::messages::message::LogRecord;
use crate::prelude::*;
use crate::game::TriggeredBureaucracy;

/// `list.Remove(item)` on a `List<TreacheryCard>`: removes the first occurrence, if any.
/// `None` is null, which removes the first null card.
pub(crate) fn remove_card(list: &mut Vec<TreacheryCard>, card: Option<TreacheryCard>) -> bool {
    let card = card.and_then(crate::compat::cs::CsNull::opt);
    cs::list_remove(list, card)
}

impl Game {
    /// Runs `f` on `BidSequence` (which C# mutates in place while the game is
    /// also in use). `BidSequence!` throws NullReferenceException when it is
    /// null. If `f` throws, the sequence is put back with whatever changes were
    /// made before the throw, as in C#, and the exception propagates.
    pub(crate) fn with_bid_sequence<R>(&mut self, f: impl FnOnce(&mut PlayerSequence, &Game) -> R) -> R {
        let mut seq = self.bid_sequence.take().nre();
        let result = cs::catch(|| f(&mut seq, &*self));
        // C# parity: the sequence object stays in the game, also after a throw.
        self.bid_sequence = Some(seq);
        match result {
            Ok(r) => r,
            Err(e) => std::panic::resume_unwind(Box::new(e)),
        }
    }

    pub fn enter_bidding_phase(&mut self) {
        self.main_phase_start(MainPhase::Bidding, true);
        self.allow(FactionAdvantage::ChoamControllingCharity);
        self.allow(FactionAdvantage::BeneGesseritCharity);
        let random = match &self.random {
            Some(r) => r.clone(),
            None => throw!(Exception, "Random generator is not initialized"),
        };
        self.cards_on_auction = Some(Deck::empty(Some(random)));
        self.ixian_swapped_card_on_bid = false;
        self.card_sold_on_black_market = None;
        self.black_market_auction_type = AuctionType::None;
        self.regular_bidding_is_done = false;
        self.bidding_round_was_started = false;
        self.richese_auction_should_still_happen = false;
        self.current_auction_type = AuctionType::None;
        self.bidding_triggered_bureaucracy = None;
        self.bid_sequence = Some(PlayerSequence::new_game(self));
        self.winning_bid = None;

        self.enter(Phase::BeginningOfBidding, true);
    }

    pub fn start_bidding_phase(&mut self) {
        let richese = self.get_player(Some(Faction::Richese));
        let condition = richese.is_some()
            && self.applicable(Rule::RicheseBlackMarket)
            && !self.prevented(FactionAdvantage::RicheseBlackMarket)
            && self.player(richese.nre()).treachery_cards.len() > 0;
        self.enter_boolean_phase_action(condition, Phase::BlackMarketAnnouncement, Game::enter_richese_bidding);
    }

    pub fn enter_richese_bidding(&mut self) {
        self.number_of_cards_on_auction = self.players_that_can_bid().len() as i32;

        if self.card_sold_on_black_market.is_some() {
            self.number_of_cards_on_auction -= 1;
        }

        //CardSoldOnBlackMarket = null;

        if self.number_of_cards_on_auction == 0 {
            self.log(vec![express!("Bidding is skipped because no faction is able to buy cards").with_log(BiddingSkipped::default()).into()]);
            self.end_bidding_phase();
        } else {
            let condition = self.is_playing(Faction::Richese) && self.richese_cache.len() > 0 && !self.prevented(FactionAdvantage::RicheseAuction);
            self.enter_boolean_phase_action(condition, Phase::RicheseAnnouncingAuction, Game::start_regular_bidding);
        }
    }

    pub fn start_regular_bidding(&mut self) {
        self.regular_bidding_is_done = true;
        let mut number_of_cards_to_draw = self.number_of_cards_on_auction;

        if number_of_cards_to_draw > 0 && self.is_playing(Faction::Ixian) && !self.prevented(FactionAdvantage::IxianSelectingCardsOnAuction) {
            number_of_cards_to_draw += 1;
        }

        self.log(vec![express!(number_of_cards_to_draw, " cards were drawn for bidding").with_log(BiddingDrawn::new(number_of_cards_to_draw)).into()]);

        let mut i = 0;
        while i < number_of_cards_to_draw {
            let card = self.draw_treachery_card();
            if let Some(card) = card {
                if let Some(cards_on_auction) = self.cards_on_auction.as_mut() {
                    cards_on_auction.put_on_bottom(card);
                }
            }
            i += 1;
        }

        self.start_bid_sequence_and_auction_type(AuctionType::Normal, None, 1);

        let condition = self.is_playing(Faction::Ixian) && !self.prevented(FactionAdvantage::IxianSelectingCardsOnAuction);
        self.enter_boolean_phase_action(condition, Phase::IxianRemovingCardFromBid, Game::start_bidding_round);
    }

    pub fn start_bidding_round(&mut self) {
        self.bidding_round_was_started = true;
        self.with_bid_sequence(|seq, g| g.skip_players_that_cant_bid(seq));
        self.enter(Phase::Bidding, true);
        self.current_bid = None;
        self.bids.clear();
        self.card_number = 1;
    }

    pub fn execute_bid(&mut self, bid: &Rc<Bid>) {
        if !bid.passed {
            self.return_karma_card_used_for_bid();
            if bid.using_karma_to_remove_bid_limit() {
                let p = GameEventMethods::player(&**bid);
                let karma_card = bid.karma_card();
                remove_card(&mut self.player_mut(p).treachery_cards, karma_card);
            }
            self.current_bid = Some(bid.clone().into_event());
            self.stone(Milestone::Bid);
        }

        self.with_bid_sequence(|seq, g| {
            seq.next_player(g);
            g.skip_players_that_cant_bid(seq);
        });
        let initiator = GameEventMethods::initiator(&**bid);
        self.bids.remove(&initiator);
        self.bids.add_with(initiator, bid.clone().into_event(), |k| k.to_string());
        let as_event = bid.clone().into_event();
        self.check_if_atreides_or_atreides_ally_passed_bid(&as_event);

        match self.current_auction_type {
            AuctionType::Normal => {
                bid.handle_normal_bid(self);
            }
            AuctionType::RicheseOnceAround | AuctionType::RicheseSilent => {
                bid.handle_richese_bid(self);
            }
            _ => {}
        }
    }

    pub fn check_if_atreides_or_atreides_ally_passed_bid(&mut self, bid: &GameEvent) {
        let atreides = match self.get_player(Some(Faction::Atreides)) {
            Some(g) => g,
            None => return,
        };

        let initiator = bid.i_bid_initiator();
        if initiator == self.player(atreides).faction || initiator == self.player(atreides).ally {
            self.latest_bid_by_atreides_or_atreides_ally_was_passed = bid.i_bid_passed();
        }
    }

    pub fn return_karma_card_used_for_bid(&mut self) {
        if let Some(current_bid) = self.current_bid.clone() {
            if current_bid.i_bid_using_karma_to_remove_bid_limit() {
                let p = current_bid.i_bid_player().nre();
                // UsingKarmaToRemoveBidLimit implies KarmaCard != null.
                let card = current_bid.i_bid_karma_card().nre();
                self.player_mut(p).treachery_cards.push(card);
            }
        }
    }

    pub fn log_bid(&mut self, initiator: PlayerIndex, bid_amount: i32, bid_ally_contribution_amount: i32, bid_emperor_contribution_amount: i32, receiver_income: &MessagePart, paid_to: Faction, free: bool) {
        let bid_total_amount = bid_amount.wrapping_add(bid_ally_contribution_amount).wrapping_add(bid_emperor_contribution_amount);
        let card_number = express_part_if!(self.current_auction_type == AuctionType::Normal; self.card_number);

        let initiator_ally = self.player(initiator).ally;
        let initiator_faction = self.player(initiator).faction;

        let mut shares: Vec<BidShare> = Vec::new();
        if bid_ally_contribution_amount > 0 {
            shares.push(BidShare::new(initiator_ally, bid_ally_contribution_amount));
        }
        if bid_emperor_contribution_amount > 0 {
            shares.push(BidShare::new(Faction::Emperor, bid_emperor_contribution_amount));
        }

        let record: Option<Rc<dyn LogRecord>> = if free {
            None
        } else {
            Some(Rc::new(CardSold::new(self.card_number, initiator_faction, bid_total_amount, paid_to, shares, None)))
        };

        let expression = vec![
            ExpressionElement::from(initiator_faction),
            ExpressionElement::from(" get card "),
            ExpressionElement::from(card_number),
            ExpressionElement::from(" for "),
            ExpressionElement::from(Payment::of(bid_total_amount)),
            ExpressionElement::from(express_part_if!(
                bid_ally_contribution_amount > 0 || bid_emperor_contribution_amount > 0;
                "(",
                Payment::of_by(bid_ally_contribution_amount, initiator_ally),
                express_part_if!(bid_emperor_contribution_amount > 0; " and ", Payment::of_by(bid_emperor_contribution_amount, Faction::Emperor)),
                ") "
            )),
            ExpressionElement::from(receiver_income.clone()),
        ];

        self.log_bidding(record, expression);
    }

    pub fn log_bidding(&mut self, record: Option<Rc<dyn LogRecord>>, expression: Vec<ExpressionElement>) {
        let mut message = Message::express(expression);
        if record.is_some() && self.current_auction_type == AuctionType::Normal {
            message.log = record;
        }
        self.current_report_mut().express_message(message);
    }

    pub fn log_bidding_to(&mut self, faction: Faction, record: Rc<dyn LogRecord>, expression: Vec<ExpressionElement>) {
        let mut message = Message::express_to(faction, expression);
        if self.current_auction_type == AuctionType::Normal {
            message.log = Some(record);
        }
        self.current_report_mut().express_message(message);
    }

    /// `toDrawFrom` is always `Game.CardsOnAuction` in C# (every caller passes it after
    /// checking it is not null), so this draws from `self.cards_on_auction`.
    pub fn win_by_highest_bid(&mut self, winner: PlayerIndex, bid: &GameEvent, bid_amount: i32, bid_ally_contribution_amount: i32, bid_emperor_contribution_amount: i32, payment_receiver: Faction, uses_emperor_secret_ally: bool) -> TreacheryCard {
        let mut receiver_income_message = express_part!();

        if !uses_emperor_secret_ally {
            let paid_to = self.pay_for_card(winner, bid, bid_amount, bid_ally_contribution_amount, bid_emperor_contribution_amount, payment_receiver, &mut receiver_income_message);
            self.log_bid(winner, bid_amount, bid_ally_contribution_amount, bid_emperor_contribution_amount, &receiver_income_message, paid_to, false);
        } else {
            self.play_nexus_card(winner, vec!["Secret Ally".into(), "get this card for free".into()]);
            self.log_bid(winner, 0, 0, 0, &receiver_income_message, Faction::None, true);
        }

        self.stone(Milestone::AuctionWon);
        let card = self.cards_on_auction.as_mut().nre().draw();
        self.register_won_card_as_known(card);
        self.player_mut(winner).treachery_cards.push(card);
        let winner_faction = self.player(winner).faction;
        self.log_bidding_to(winner_faction, Rc::new(CardWonShown::new(card)), vec!["You won: ".into(), card.into()]);
        self.give_player_extra_card_if_applicable(winner);
        card
    }

    pub fn register_won_card_as_known(&mut self, card: TreacheryCard) {
        let players: Vec<PlayerIndex> = self.player_indices().filter(|p| self.has_bidding_prescience(*p)).collect();
        for p in players {
            Game::register_known_player_treachery_card(self, p, Some(card));
        }
    }

    /// Takes the sequence separately from the game; for `BidSequence`, call it
    /// through [`Game::with_bid_sequence`].
    pub fn skip_players_that_cant_bid(&self, sequence: &mut PlayerSequence) {
        let mut i = 0;
        while i < self.players.len() {
            if self.player(sequence.current_player(self).nre()).has_room_for_cards(self) {
                break;
            }
            sequence.next_player(self);
            i += 1;
        }
    }

    pub fn pay_for_card(&mut self, initiator: PlayerIndex, bid: &GameEvent, bid_amount: i32, bid_ally_contribution_amount: i32, bid_emperor_contribution_amount: i32, payment_receiver: Faction, message: &mut MessagePart) -> Faction {
        let mut paid_to = Faction::None;
        {
            let p = self.player_mut(initiator);
            p.resources = p.resources.wrapping_sub(bid_amount);
        }

        let mut receiver_profit = 0;

        if bid_ally_contribution_amount > 0 && self.player(initiator).has_ally() {
            let ally = self.player(initiator).allied_player(self).nre();
            {
                let a = self.player_mut(ally);
                a.resources = a.resources.wrapping_sub(bid_ally_contribution_amount);
            }
            let initiator_faction = self.player(initiator).faction;
            self.decrease_permitted_use_of_ally_spice(initiator_faction, bid_ally_contribution_amount);
        }

        if bid_emperor_contribution_amount > 0 {
            let emperor = self.get_player(Some(Faction::Emperor)).nre();
            let r = self.player_mut(emperor);
            r.resources = r.resources.wrapping_sub(bid_emperor_contribution_amount);
        }

        let receiver = self.get_player(Some(payment_receiver));
        let receiver_and_ally_are_richese = self.player(initiator).ally == Faction::Richese && payment_receiver == Faction::Richese;
        let total_amount = bid_amount.wrapping_add(if receiver_and_ally_are_richese { 0 } else { bid_ally_contribution_amount });

        let emperor = self.get_player(Some(Faction::Emperor));
        let mut total_amount_received_by_emperor_instead_of_richese =
            if receiver_and_ally_are_richese && emperor.is_some() && !self.prevented(FactionAdvantage::EmperorReceiveBid) {
                bid_ally_contribution_amount
            } else {
                0
            };

        let mut emperor_received_instead_message = express_part!();
        if emperor.is_some() && total_amount_received_by_emperor_instead_of_richese > 0 {
            let emperor = emperor.nre();
            // C# parity: Richese's homeworld threshold and occupier are applied to Emperor's income (hazards 18.2).
            self.modify_income_because_of_low_threshold_or_occupation(receiver.nre(), &mut total_amount_received_by_emperor_instead_of_richese);
            {
                let r = self.player_mut(emperor);
                r.resources = r.resources.wrapping_add(total_amount_received_by_emperor_instead_of_richese);
            }
            emperor_received_instead_message = express_part!(" and ", Faction::Emperor, " get ", Payment::of(total_amount_received_by_emperor_instead_of_richese));

            if total_amount_received_by_emperor_instead_of_richese >= 5 {
                //This should also take into account payments received by an occupier
                self.bidding_triggered_bureaucracy = Some(TriggeredBureaucracy { payment_from: Faction::Richese, payment_to: Faction::Emperor });
            }

            // C# parity: records a recent payment of receiverProfit while it is still 0 (hazards 18.2).
            let initiator_faction = self.player(initiator).faction;
            let receiver_faction = self.player(receiver.nre()).faction;
            self.set_recent_payment(receiver_profit, initiator_faction, receiver_faction, bid);
        }

        if receiver.is_some() && self.player(initiator).faction != payment_receiver {
            let receiver = receiver.nre();
            if payment_receiver == Faction::Emperor && self.prevented(FactionAdvantage::EmperorReceiveBid) {
                *message = express_part!(" → ", payment_receiver, " do not receive ", Concept::Resource, " for this card");
                if !self.applicable(Rule::FullPhaseKarma) {
                    self.allow(FactionAdvantage::EmperorReceiveBid);
                }
            } else {
                receiver_profit = total_amount;
                self.modify_income_because_of_low_threshold_or_occupation(receiver, &mut receiver_profit);
                {
                    let r = self.player_mut(receiver);
                    r.resources = r.resources.wrapping_add(receiver_profit);
                    r.resources_after_bidding = r.resources_after_bidding.wrapping_add(bid_emperor_contribution_amount);
                }
                let receiver_faction = self.player(receiver).faction;
                paid_to = receiver_faction;
                *message = express_part!(
                    " → ",
                    receiver_faction,
                    " get ",
                    Payment::of(receiver_profit),
                    emperor_received_instead_message,
                    express_part_if!(bid_emperor_contribution_amount > 0; " immediately and ", Payment::of(bid_emperor_contribution_amount), " at the end of the bidding phase")
                );

                if receiver_profit >= 5 {
                    //This should also take into account payments received by an occupier
                    let initiator_faction = self.player(initiator).faction;
                    self.bidding_triggered_bureaucracy = Some(TriggeredBureaucracy { payment_from: initiator_faction, payment_to: payment_receiver });
                }

                let initiator_faction = self.player(initiator).faction;
                self.set_recent_payment(receiver_profit, initiator_faction, receiver_faction, bid);
            }
        } else if total_amount_received_by_emperor_instead_of_richese > 0 {
            *message = express_part!(" → ", Faction::Emperor, " get ", Payment::of(total_amount_received_by_emperor_instead_of_richese));
            paid_to = Faction::Emperor;
        }

        if total_amount.wrapping_sub(receiver_profit).wrapping_sub(total_amount_received_by_emperor_instead_of_richese) >= 4 {
            self.activate_banker(initiator);
        }

        paid_to
    }

    pub fn give_player_extra_card_if_applicable(&mut self, winner: PlayerIndex) {
        if self.player(winner).is(Faction::Harkonnen) {
            let mut extra_card_must_be_decided_about = false;
            let mut receiver: Option<PlayerIndex> = None;

            let occupier_of_harkonnen_homeworld = self.occupier_of(World::Harkonnen);
            if let Some(occupier) = occupier_of_harkonnen_homeworld {
                if self.player(occupier).has_room_for_cards(self) {
                    receiver = Some(occupier);
                    extra_card_must_be_decided_about = self.player(occupier).has_ally()
                        && self.player(self.player(occupier).allied_player(self).nre()).has_room_for_cards(self);
                } else if self.player(occupier).has_ally() && self.player(self.player(occupier).allied_player(self).nre()).has_room_for_cards(self) {
                    receiver = self.player(occupier).allied_player(self);
                }
            } else if self.player(winner).has_room_for_cards(self) {
                if !self.prevented(FactionAdvantage::HarkonnenFreeCard) {
                    receiver = Some(winner);
                } else {
                    self.log_prevention_by_karma(FactionAdvantage::HarkonnenFreeCard);
                    if !self.applicable(Rule::FullPhaseKarma) {
                        self.allow(FactionAdvantage::HarkonnenFreeCard);
                    }
                }
            }

            if let Some(receiver) = receiver {
                let extra_card = self.draw_treachery_card();

                if let Some(extra_card) = extra_card {
                    self.player_mut(receiver).treachery_cards.push(extra_card);
                    let receiver_faction = self.player(receiver).faction;
                    self.log_bidding(Some(Rc::new(ExtraCardReceived::new(receiver_faction))), vec![receiver_faction.into(), " receive an extra treachery card".into()]);
                    self.log_bidding_to(receiver_faction, Rc::new(ExtraCardShown::new(extra_card)), vec!["Your extra card is: ".into(), extra_card.into()]);

                    if extra_card_must_be_decided_about {
                        self.card_that_must_be_kept_or_given_to_ally = Some(extra_card);
                    }
                }
            }
        }
    }

    pub fn draw_treachery_card(&mut self) -> Option<TreacheryCard> {
        if self.treachery_deck.as_ref().map(|d| d.is_empty()) == Some(true) {
            let count = self.treachery_discard_pile.as_ref().nre().items.len() as i32;
            self.log(vec![express!("Shuffled ", count, " cards from the treachery discard pile into a new deck")
                .with_log(TreacheryReshuffled::new(count))
                .into()]);

            let discarded: Vec<TreacheryCard> = self.treachery_discard_pile.as_ref().nre().items.clone();
            for i in discarded {
                self.treachery_deck.as_mut().nre().items.push(i);
                self.unregister_known(i);
            }

            self.treachery_discard_pile.as_mut().nre().clear();
            self.treachery_deck.as_mut().nre().shuffle();
            self.stone(Milestone::Shuffled);
        }

        // `TreacheryDeck?.Items.Count > 0`: null > 0 is false.
        if self.treachery_deck.as_ref().map_or(false, |d| d.items.len() > 0) {
            // A null card in the deck comes back as None (engine-conventions.md, "Null handles").
            self.treachery_deck.as_mut().nre().draw().opt()
        } else {
            None
        }
    }

    pub fn finish_bid(&mut self, winner: Option<PlayerIndex>, winning_bid: Option<&GameEvent>, card: TreacheryCard, might_replace: bool) {
        let winner = winner.and_then(crate::compat::cs::CsNull::opt);
        self.card_just_won = Some(card);
        self.winning_bid = winning_bid.cloned();
        self.card_sold_on_black_market = None;
        self.current_bid = None;
        self.faction_that_may_replace_bought_card = Faction::None;
        self.latest_bid_by_atreides_or_atreides_ally_was_passed = false;

        self.bids.clear();

        if !self.applicable(Rule::FullPhaseKarma) {
            self.allow(FactionAdvantage::AtreidesBiddingPrescience);
        }

        let mut enter_replacing_card_just_won = might_replace && self.player(winner.nre()).nexus != Faction::None;

        if might_replace {
            let winner = winner.nre();
            if self.player(winner).ally == Faction::Ixian
                && self.ixian_allows_replacing_cards
                && !matches!(self.current_auction_type, AuctionType::RicheseSilent | AuctionType::RicheseOnceAround)
            {
                if !self.prevented(FactionAdvantage::IxianAllyDiscardingCard) {
                    self.faction_that_may_replace_bought_card = self.player(winner).faction;
                    enter_replacing_card_just_won = true;
                } else {
                    if !self.applicable(Rule::FullPhaseKarma) {
                        self.allow(FactionAdvantage::IxianAllyDiscardingCard);
                    }

                    if self.nexus_allows_replacing_bought_cards(winner) {
                        self.faction_that_may_replace_bought_card = self.player(winner).faction;
                        enter_replacing_card_just_won = true;
                        self.replacing_bought_card_using_nexus = true;
                    }
                }
            } else if self.nexus_allows_replacing_bought_cards(winner) {
                self.faction_that_may_replace_bought_card = self.player(winner).faction;
                enter_replacing_card_just_won = true;
                self.replacing_bought_card_using_nexus = true;
            }
        }

        self.enter_boolean_phase_action(enter_replacing_card_just_won, Phase::ReplacingCardJustWon, Game::determine_next_step_after_card_was_sold);

        if let Some(b) = self.bidding_triggered_bureaucracy.clone() {
            self.apply_bureaucracy(b.payment_from, b.payment_to);
            self.bidding_triggered_bureaucracy = None;
        }
    }

    pub fn determine_next_step_after_card_was_sold(&mut self) {
        self.richese_bidding_just_finished =
            self.current_auction_type == AuctionType::RicheseOnceAround || self.current_auction_type == AuctionType::RicheseSilent;

        // `CardsOnAuction?.IsEmpty is not true`: also taken when CardsOnAuction is null.
        if self.cards_on_auction.as_ref().map(|d| d.is_empty()) != Some(true) {
            self.move_to_next_card_on_auction();
        } else if self.regular_bidding_is_done {
            if self.richese_auction_should_still_happen {
                self.enter(Phase::RicheseSpecifyingAuction, true);
            } else {
                self.end_bidding_phase();
            }
        } else {
            self.start_regular_bidding();
        }
    }

    pub fn move_to_next_card_on_auction(&mut self) {
        self.card_number += 1;
        self.card_that_must_be_kept_or_given_to_ally = None;

        if self.ixian_may_swap_card_on_bid() {
            if !self.prevented(FactionAdvantage::IxianSwappingCard) {
                self.enter(Phase::IxianSwappingCard, true);
            } else {
                self.log_prevention_by_karma(FactionAdvantage::IxianSwappingCard);
                if !self.applicable(Rule::FullPhaseKarma) {
                    self.allow(FactionAdvantage::IxianSwappingCard);
                }
                self.enter(Phase::WaitingForNextBiddingRound, true);
            }
        } else {
            self.enter(Phase::WaitingForNextBiddingRound, true);
        }
    }

    pub fn put_next_card_on_auction(&mut self) {
        if !self.richese_bidding_just_finished {
            self.with_bid_sequence(|seq, g| seq.next_round(g));
        }

        self.with_bid_sequence(|seq, g| g.skip_players_that_cant_bid(seq));
        self.winning_bid = None;
        self.enter(Phase::Bidding, true);
    }

    pub fn continue_after_ixian_card_swap_decision(&mut self) {
        if !self.bidding_round_was_started {
            self.start_bidding_round();
        } else {
            let condition = self.is_playing(Faction::Atreides);
            self.enter_boolean_phase_action(condition, Phase::WaitingForNextBiddingRound, Game::put_next_card_on_auction);
        }
    }

    pub fn start_bid_sequence_and_auction_type(&mut self, auction_type: AuctionType, richese_player: Option<PlayerIndex>, direction: i32) {
        let richese_player = richese_player.and_then(crate::compat::cs::CsNull::opt);
        match auction_type {
            AuctionType::BlackMarketNormal => {
                self.black_market_auction_type = AuctionType::BlackMarketNormal;
                self.bid_sequence = Some(PlayerSequence::new_game_boolean_int32(self, true, 1));
            }
            AuctionType::BlackMarketOnceAround => {
                self.black_market_auction_type = AuctionType::BlackMarketOnceAround;
                self.bid_sequence = Some(PlayerSequence::new(self, true, direction, richese_player, true));
            }
            AuctionType::BlackMarketSilent => {
                self.bid_sequence = Some(PlayerSequence::new_game_boolean_int32(self, true, 1));
                self.black_market_auction_type = AuctionType::BlackMarketSilent;
            }
            AuctionType::RicheseOnceAround => {
                self.bid_sequence = Some(PlayerSequence::new(self, true, direction, richese_player, true));
            }
            AuctionType::RicheseSilent => {
                self.bid_sequence = Some(PlayerSequence::new_game_boolean_int32(self, true, 1));
            }
            AuctionType::Normal => {
                self.bid_sequence = Some(PlayerSequence::new_game_boolean_int32(self, true, 1));
                if self.black_market_auction_type == AuctionType::BlackMarketNormal {
                    //Continue where black market bidding left off
                    self.with_bid_sequence(|seq, g| seq.next_round(g));
                    self.winning_bid = None;
                }
            }
            _ => {}
        }

        self.current_auction_type = auction_type;
    }

    pub fn determine_highest_bid(&self, bids: &NetDictionary<Faction, GameEvent>) -> Option<GameEvent> {
        let highest_bid_value = dotnet_compat::linq::max(bids.values().map(|b| b.i_bid_total_amount()));
        let mut determine_bid_winner_sequence = PlayerSequence::new_game_boolean_int32(self, false, 1);

        let mut i = 0;
        while i < self.maximum_players() {
            let f = determine_bid_winner_sequence.current_faction(self);
            if let Some(value) = bids.get(&f) {
                if value.i_bid_total_amount() == highest_bid_value {
                    return Some(bids.index_with(&f, |k| k.to_string()).clone());
                }
            }

            determine_bid_winner_sequence.next_player(self);
            i += 1;
        }

        None
    }

    pub fn end_bidding_phase(&mut self) {
        self.card_that_must_be_kept_or_given_to_ally = None;
        self.current_ixian_cunning = None;
        if let Some(emperor) = self.get_player(Some(Faction::Emperor)) {
            let r = self.player_mut(emperor);
            r.resources = r.resources.wrapping_add(r.resources_after_bidding);
            r.resources_after_bidding = 0;
        }

        self.main_phase_end();
        self.enter(Phase::BiddingReport, true);
    }

    pub fn number_of_cards_on_regular_auction(&self) -> i32 {
        self.card_number.wrapping_add(self.cards_on_auction.as_ref().nre().items.len() as i32).wrapping_sub(1)
    }

    pub fn players_that_can_bid(&self) -> Vec<PlayerIndex> {
        self.player_indices().filter(|p| self.player(*p).has_room_for_cards(self)).collect()
    }

    pub fn nexus_allows_replacing_bought_cards(&self, p: PlayerIndex) -> bool {
        let nexus = self.player(p).nexus;
        nexus == Faction::Ixian && NexusPlayed::can_use_secret_ally(self, p)
    }

    pub fn has_bid_to_pay(&self, p: PlayerIndex) -> bool {
        match &self.current_bid {
            Some(current_bid) if self.current_phase == Phase::Bidding => {
                current_bid.i_bid_initiator() == self.player(p).faction
                    || (current_bid.i_bid_initiator() == self.player(p).ally && current_bid.i_bid_ally_contribution_amount() > 0)
            }
            _ => false,
        }
    }

    pub fn ixian_may_swap_card_on_bid(&self) -> bool {
        // `GetPlayer(Faction.Ixian)?.TreacheryCards.Count > 0`: null > 0 is false.
        self.get_player(Some(Faction::Ixian)).map_or(false, |g| self.player(g).treachery_cards.len() > 0)
            && self.applicable(Rule::IxianSwappingCardOnBid)
            && !self.ixian_swapped_card_on_bid
    }

    pub fn cards_owned_by(&self, p: PlayerIndex) -> Vec<TreacheryCard> {
        let card_set_aside = self.get_card_set_aside_for_bid(p);
        match card_set_aside {
            None => self.player(p).treachery_cards.clone(),
            Some(card_set_aside) => {
                let mut result = self.player(p).treachery_cards.clone();
                result.push(card_set_aside);
                result
            }
        }
    }

    pub fn get_card_set_aside_for_bid(&self, p: PlayerIndex) -> Option<TreacheryCard> {
        match &self.current_bid {
            Some(current_bid) if current_bid.i_bid_player() == Some(p) && current_bid.i_bid_using_karma_to_remove_bid_limit() => current_bid.i_bid_karma_card(),
            _ => None,
        }
    }
}
