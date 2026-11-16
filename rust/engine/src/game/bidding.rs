use crate::prelude::*;
use crate::messages::part::Part;
use crate::game::TriggeredBureaucracy;

/// State of the bidding phase: the auction, the bids, Black Market and Richese auctions, and Karama
/// hand swaps.
pub struct BiddingState {
    pub bid_sequence: Option<PlayerSequence>,
    pub cards_on_auction: Option<Deck<TreacheryCard>>,
    pub current_auction_type: Option<AuctionType>,
    pub card_number: i32,
    pub current_bid: Option<PlacedBid>,
    // In the order the bids were made (a new bid of a faction replaces its old one and goes last).
    pub bids: IndexMap<Faction, PlacedBid>,
    pub card_just_won: Option<TreacheryCard>,
    pub winning_bid: Option<PlacedBid>,
    /// How many bids were placed in this game, automatic passes included: the next bid's number.
    pub bids_placed: u32,
    pub bidding_round_was_started: bool,
    pub regular_bidding_is_done: bool,
    pub card_that_must_be_kept_or_given_to_ally: Option<TreacheryCard>,
    pub number_of_cards_on_auction: i32,
    pub bidding_triggered_bureaucracy: Option<TriggeredBureaucracy>,
    pub ixian_swapped_card_on_bid: bool,
    pub faction_that_may_replace_bought_card: Option<Faction>,
    pub replacing_bought_card_using_nexus: bool,
    pub karma_hand_swap_number_of_cards: i32,
    pub karma_hand_swap_target: Option<Faction>,
    pub black_market_auction_type: Option<AuctionType>,
    pub card_sold_on_black_market: Option<TreacheryCard>,
    pub richese_auction_should_still_happen: bool,
    pub richese_bidding_just_finished: bool,
    pub richese_occupier_specified_card: bool,
    pub latest_bid_by_atreides_or_atreides_ally_was_passed: bool,
}

impl BiddingState {
    pub fn new() -> Self {
        BiddingState {
            bid_sequence: None,
            cards_on_auction: None,
            current_auction_type: Default::default(),
            card_number: Default::default(),
            current_bid: None,
            bids: Default::default(),
            card_just_won: None,
            winning_bid: None,
            bids_placed: 0,
            bidding_round_was_started: Default::default(),
            regular_bidding_is_done: Default::default(),
            card_that_must_be_kept_or_given_to_ally: None,
            number_of_cards_on_auction: Default::default(),
            bidding_triggered_bureaucracy: None,
            ixian_swapped_card_on_bid: Default::default(),
            faction_that_may_replace_bought_card: Default::default(),
            replacing_bought_card_using_nexus: Default::default(),
            karma_hand_swap_number_of_cards: Default::default(),
            karma_hand_swap_target: Default::default(),
            black_market_auction_type: Default::default(),
            card_sold_on_black_market: None,
            richese_auction_should_still_happen: Default::default(),
            richese_bidding_just_finished: Default::default(),
            richese_occupier_specified_card: Default::default(),
            latest_bid_by_atreides_or_atreides_ally_was_passed: Default::default(),
        }
    }
}

/// A bid (`Bid` or `BlackMarketBid`) as the auction keeps it: what was offered and by whom.
///
/// The `bid_*` accessors keep the names the code outside bidding used on the stored event.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedBid {
    /// Numbers the bids of a game, automatic passes included, so two bids with the same values
    /// stay apart, as the stored C# events did (the story compares winning bids).
    pub number: u32,
    /// `Bid` or `BlackMarketBid`.
    pub type_: EventType,
    pub initiator: Option<Faction>,
    /// The initiator's player when the bid was submitted.
    pub player: Option<PlayerId>,
    pub amount: u16,
    pub ally_contribution_amount: u16,
    pub emperor_contribution_amount: u16,
    pub uses_emperor_secret_ally: bool,
    pub karma_card: Option<TreacheryCard>,
    pub karma_bid: bool,
    pub passed: bool,
}

impl PlacedBid {
    /// What the bid offers: nothing for a pass (see [`Bid::total_amount`]).
    pub fn bid_total_amount(&self) -> i32 {
        if self.passed {
            return 0;
        }
        i32::from(self.amount) + i32::from(self.ally_contribution_amount) + i32::from(self.emperor_contribution_amount)
    }

    pub fn bid_amount(&self) -> i32 {
        i32::from(self.amount)
    }

    pub fn bid_ally_contribution_amount(&self) -> i32 {
        i32::from(self.ally_contribution_amount)
    }

    pub fn bid_emperor_contribution_amount(&self) -> i32 {
        i32::from(self.emperor_contribution_amount)
    }

    /// The bidder; panics for a bid without a faction, as the C# cast did.
    #[track_caller]
    pub fn bid_initiator(&self) -> Faction {
        self.initiator.expect("faction")
    }

    pub fn bid_player(&self) -> Option<PlayerId> {
        self.player
    }

    pub fn bid_passed(&self) -> bool {
        self.passed
    }

    pub fn bid_using_karma_to_remove_bid_limit(&self) -> bool {
        self.karma_card.is_some() && !self.karma_bid
    }

    pub fn bid_karma_card(&self) -> Option<TreacheryCard> {
        self.karma_card
    }

    pub fn bid_karma_bid(&self) -> bool {
        self.karma_bid
    }
}

/// Removes the first occurrence of `card` from `list`, if any.
pub(crate) fn remove_card(list: &mut Vec<TreacheryCard>, card: Option<TreacheryCard>) -> bool {
    card.is_some_and(|card| crate::iter::remove_first(list, &card))
}

impl Game {
    /// Runs `f` on the bid sequence, which is mutated in place while the game is also in use.
    /// Panics when there is no bid sequence (outside the bidding phase).
    pub(crate) fn with_bid_sequence<R>(&mut self, f: impl FnOnce(&mut PlayerSequence, &Game) -> R) -> R {
        let mut seq = self.bidding.bid_sequence.take().expect("a bid sequence during bidding");
        let result = f(&mut seq, &*self);
        self.bidding.bid_sequence = Some(seq);
        result
    }

    pub fn enter_bidding_phase(&mut self) {
        self.main_phase_start(MainPhase::Bidding, true);
        self.allow(FactionAdvantage::ChoamControllingCharity);
        self.allow(FactionAdvantage::BeneGesseritCharity);
        self.bidding.cards_on_auction = Some(Deck::empty());
        self.bidding.ixian_swapped_card_on_bid = false;
        self.bidding.card_sold_on_black_market = None;
        self.bidding.black_market_auction_type = None;
        self.bidding.regular_bidding_is_done = false;
        self.bidding.bidding_round_was_started = false;
        self.bidding.richese_auction_should_still_happen = false;
        self.bidding.current_auction_type = None;
        self.bidding.bidding_triggered_bureaucracy = None;
        self.bidding.bid_sequence = Some(PlayerSequence::starting_with_first_player(self));
        self.bidding.winning_bid = None;

        self.enter(Phase::BeginningOfBidding, true);
    }

    pub fn start_bidding_phase(&mut self) {
        let richese = self.get_player(Some(Faction::Richese));
        let condition = richese.is_some()
            && self.applicable(Rule::RicheseBlackMarket)
            && !self.prevented(FactionAdvantage::RicheseBlackMarket)
            && self.player(richese.expect("richese")).treachery_cards.len() > 0;
        self.enter_or_else(condition, Phase::BlackMarketAnnouncement, Game::enter_richese_bidding);
    }

    pub fn enter_richese_bidding(&mut self) {
        self.bidding.number_of_cards_on_auction = self.players_that_can_bid().len() as i32;

        if self.bidding.card_sold_on_black_market.is_some() {
            self.bidding.number_of_cards_on_auction -= 1;
        }

        //CardSoldOnBlackMarket = null;

        if self.bidding.number_of_cards_on_auction == 0 {
            self.log(vec![express!("Bidding is skipped because no faction is able to buy cards").with_log(BiddingSkipped::default()).into()]);
            self.end_bidding_phase();
        } else {
            let condition = self.is_playing(Faction::Richese) && self.decks.richese_cache.len() > 0 && !self.prevented(FactionAdvantage::RicheseAuction);
            self.enter_or_else(condition, Phase::RicheseAnnouncingAuction, Game::start_regular_bidding);
        }
    }

    pub fn start_regular_bidding(&mut self) {
        self.bidding.regular_bidding_is_done = true;
        let mut number_of_cards_to_draw = self.bidding.number_of_cards_on_auction;

        if number_of_cards_to_draw > 0 && self.is_playing(Faction::Ixian) && !self.prevented(FactionAdvantage::IxianSelectingCardsOnAuction) {
            number_of_cards_to_draw += 1;
        }

        self.log(vec![express!(number_of_cards_to_draw, " cards were drawn for bidding").with_log(BiddingDrawn::new(number_of_cards_to_draw)).into()]);

        let mut i = 0;
        while i < number_of_cards_to_draw {
            let card = self.draw_treachery_card();
            if let Some(card) = card {
                if let Some(cards_on_auction) = self.bidding.cards_on_auction.as_mut() {
                    cards_on_auction.put_on_bottom(card);
                }
            }
            i += 1;
        }

        self.start_bid_sequence_and_auction_type(Some(AuctionType::Normal), None, 1);

        let condition = self.is_playing(Faction::Ixian) && !self.prevented(FactionAdvantage::IxianSelectingCardsOnAuction);
        self.enter_or_else(condition, Phase::IxianRemovingCardFromBid, Game::start_bidding_round);
    }

    pub fn start_bidding_round(&mut self) {
        self.bidding.bidding_round_was_started = true;
        self.with_bid_sequence(|seq, g| g.skip_players_that_cant_bid(seq));
        self.enter(Phase::Bidding, true);
        self.bidding.current_bid = None;
        self.bidding.bids.clear();
        self.bidding.card_number = 1;
    }

    /// The next bid's number (`PlacedBid::number`).
    pub(crate) fn number_bid(&mut self) -> u32 {
        self.bidding.bids_placed += 1;
        self.bidding.bids_placed
    }

    pub fn execute_bid(&mut self, bid: &PlacedBid) -> GameResult {
        if !bid.passed {
            self.return_karma_card_used_for_bid();
            if bid.bid_using_karma_to_remove_bid_limit() {
                let p = bid.player.expect("the event has a player");
                let karma_card = bid.karma_card;
                remove_card(&mut self.player_mut(p).treachery_cards, karma_card);
            }
            self.bidding.current_bid = Some(bid.clone());
            self.stone(Milestone::Bid);
        }

        self.with_bid_sequence(|seq, g| {
            seq.next_player(g);
            g.skip_players_that_cant_bid(seq);
        });
        let initiator = bid.initiator;
        self.bidding.bids.shift_remove_opt(initiator);
        self.bidding.bids.insert_new(initiator.required("faction")?, bid.clone());
        self.check_if_atreides_or_atreides_ally_passed_bid(bid);

        match self.bidding.current_auction_type {
            Some(AuctionType::Normal) => {
                Bid::handle_normal_bid(self, bid)?;
            }
            Some(AuctionType::RicheseOnceAround) | Some(AuctionType::RicheseSilent) => {
                Bid::handle_richese_bid(self, bid)?;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn check_if_atreides_or_atreides_ally_passed_bid(&mut self, bid: &PlacedBid) {
        let atreides = match self.get_player(Some(Faction::Atreides)) {
            Some(g) => g,
            None => return,
        };

        let initiator = bid.bid_initiator();
        if Some(initiator) == self.player(atreides).faction || Some(initiator) == self.player(atreides).ally {
            self.bidding.latest_bid_by_atreides_or_atreides_ally_was_passed = bid.bid_passed();
        }
    }

    pub fn return_karma_card_used_for_bid(&mut self) {
        if let Some(current_bid) = self.bidding.current_bid.clone() {
            if current_bid.bid_using_karma_to_remove_bid_limit() {
                let p = current_bid.bid_player().expect("i bid player");
                // UsingKarmaToRemoveBidLimit implies KarmaCard != null.
                let card = current_bid.bid_karma_card().expect("i bid karma card");
                self.player_mut(p).treachery_cards.push(card);
            }
        }
    }

    pub fn log_bid(&mut self, initiator: PlayerId, bid_amount: i32, bid_ally_contribution_amount: i32, bid_emperor_contribution_amount: i32, receiver_income: &MessagePart, paid_to: Option<Faction>, free: bool) {
        let bid_total_amount = bid_amount + bid_ally_contribution_amount + bid_emperor_contribution_amount;
        // A Black Market card has no number.
        let card_number: Vec<Part> = if self.bidding.current_auction_type == Some(AuctionType::Normal) {
            vec![" get card ".into(), express_part!(self.bidding.card_number).into(), " for ".into()]
        } else {
            vec![" get card for ".into()]
        };

        let initiator_ally = self.player(initiator).ally;
        let initiator_faction = self.player(initiator).faction;

        let mut shares: Vec<BidShare> = Vec::new();
        if bid_ally_contribution_amount > 0 {
            shares.push(BidShare::new(initiator_ally, bid_ally_contribution_amount));
        }
        if bid_emperor_contribution_amount > 0 {
            shares.push(BidShare::new(Faction::Emperor, bid_emperor_contribution_amount));
        }

        let record: Option<LogRecord> = if free {
            None
        } else {
            Some(CardSold::new(self.bidding.card_number, initiator_faction.expect("faction"), bid_total_amount, paid_to, shares, None).into())
        };

        let mut expression = vec![Part::from(initiator_faction)];
        expression.extend(card_number);
        expression.extend([
            Part::from(Payment::of(bid_total_amount)),
            Part::from(express_part_if!(
                bid_ally_contribution_amount > 0 || bid_emperor_contribution_amount > 0;
                " (",
                Payment::of_by(bid_ally_contribution_amount, initiator_ally),
                express_part_if!(bid_emperor_contribution_amount > 0; " and ", Payment::of_by(bid_emperor_contribution_amount, Some(Faction::Emperor))),
                ")"
            )),
            Part::from(receiver_income.clone()),
        ]);

        self.log_bidding(record, expression);
    }

    pub fn log_bidding(&mut self, record: Option<LogRecord>, expression: Vec<Part>) {
        let mut message = Message::express(expression);
        if let Some(record) = record.filter(|_| self.bidding.current_auction_type == Some(AuctionType::Normal)) {
            message = message.with_log(record);
        }
        self.current_report_mut().express_message(message);
    }

    pub fn log_bidding_to(&mut self, faction: Faction, record: impl Into<LogRecord>, expression: Vec<Part>) {
        let mut message = Message::express_to(Some(faction), expression);
        if self.bidding.current_auction_type == Some(AuctionType::Normal) {
            message = message.with_log(record);
        }
        self.current_report_mut().express_message(message);
    }

    /// Draws from `self.bidding.cards_on_auction`; callers check that it is not null first.
    pub fn win_by_highest_bid(&mut self, winner: PlayerId, bid: &PlacedBid, bid_amount: i32, bid_ally_contribution_amount: i32, bid_emperor_contribution_amount: i32, payment_receiver: Faction, uses_emperor_secret_ally: bool) -> GameResult<TreacheryCard> {
        let mut receiver_income_message = express_part!();

        if !uses_emperor_secret_ally {
            let paid_to = self.pay_for_card(winner, bid, bid_amount, bid_ally_contribution_amount, bid_emperor_contribution_amount, payment_receiver, &mut receiver_income_message)?;
            self.log_bid(winner, bid_amount, bid_ally_contribution_amount, bid_emperor_contribution_amount, &receiver_income_message, paid_to, false);
        } else {
            self.play_nexus_card(winner, vec!["get this card for free".into()]);
            self.log_bid(winner, 0, 0, 0, &receiver_income_message, None, true);
        }

        self.stone(Milestone::AuctionWon);
        let card = self.bidding.cards_on_auction.as_mut().required("cards on auction")?.try_draw()?;
        self.register_won_card_as_known(card);
        self.player_mut(winner).treachery_cards.push(card);
        let winner_faction = self.player(winner).faction;
        self.log_bidding_to(winner_faction.required("faction")?, CardWonShown::new(card), vec!["You won: ".into(), card.into()]);
        self.give_player_extra_card_if_applicable(winner);
        Ok(card)
    }

    pub fn register_won_card_as_known(&mut self, card: TreacheryCard) {
        let players: Vec<PlayerId> = self.player_indices().filter(|p| self.has_bidding_prescience(*p)).collect();
        for p in players {
            Game::register_known_by_player(self, p, Some(card));
        }
    }

    /// Takes the sequence separately from the game; for `BidSequence`, call it
    /// through [`Game::with_bid_sequence`].
    pub fn skip_players_that_cant_bid(&self, sequence: &mut PlayerSequence) {
        let mut i = 0;
        while i < self.players.len() {
            if self.player(sequence.current_player(self).expect("current player")).has_room_for_cards(self) {
                break;
            }
            sequence.next_player(self);
            i += 1;
        }
    }

    pub fn pay_for_card(&mut self, initiator: PlayerId, bid: &PlacedBid, bid_amount: i32, bid_ally_contribution_amount: i32, bid_emperor_contribution_amount: i32, payment_receiver: Faction, message: &mut MessagePart) -> GameResult<Option<Faction>> {
        let mut paid_to = None;
        {
            let p = self.player_mut(initiator);
            crate::game::amounts::spend(&mut p.resources, bid_amount, "spice")?;
        }

        let mut receiver_profit = 0;

        if bid_ally_contribution_amount > 0 && self.player(initiator).has_ally() {
            let ally = self.player(initiator).allied_player(self).required("The ally")?;
            {
                let a = self.player_mut(ally);
                crate::game::amounts::spend(&mut a.resources, bid_ally_contribution_amount, "spice")?;
            }
            let initiator_faction = self.player(initiator).faction;
            self.decrease_permitted_use_of_ally_spice(initiator_faction.required("faction")?, bid_ally_contribution_amount);
        }

        if bid_emperor_contribution_amount > 0 {
            let emperor = self.get_player(Some(Faction::Emperor)).required("The Emperor player")?;
            let r = self.player_mut(emperor);
            crate::game::amounts::spend(&mut r.resources, bid_emperor_contribution_amount, "spice")?;
        }

        let receiver = self.get_player(Some(payment_receiver));
        let receiver_and_ally_are_richese = self.player(initiator).ally == Some(Faction::Richese) && payment_receiver == Faction::Richese;
        let total_amount = bid_amount + if receiver_and_ally_are_richese { 0 } else { bid_ally_contribution_amount };

        let emperor = self.get_player(Some(Faction::Emperor));
        let mut total_amount_received_by_emperor_instead_of_richese =
            if receiver_and_ally_are_richese && emperor.is_some() && !self.prevented(FactionAdvantage::EmperorReceiveBid) {
                bid_ally_contribution_amount
            } else {
                0
            };

        let mut emperor_received_instead_message = express_part!();
        if emperor.is_some() && total_amount_received_by_emperor_instead_of_richese > 0 {
            let emperor = emperor.expect("emperor");
            // C# parity: Richese's homeworld threshold and occupier are applied to Emperor's income.
            self.modify_income_because_of_low_threshold_or_occupation(receiver.required("The payment receiver")?, &mut total_amount_received_by_emperor_instead_of_richese);
            {
                let r = self.player_mut(emperor);
                crate::game::amounts::gain(&mut r.resources, total_amount_received_by_emperor_instead_of_richese, "spice")?;
            }
            emperor_received_instead_message = express_part!(" and ", Faction::Emperor, " get ", Payment::of(total_amount_received_by_emperor_instead_of_richese));

            if total_amount_received_by_emperor_instead_of_richese >= 5 {
                //This should also take into account payments received by an occupier
                self.bidding.bidding_triggered_bureaucracy = Some(TriggeredBureaucracy { payment_from: Some(Faction::Richese), payment_to: Some(Faction::Emperor) });
            }

            // C# parity: records a recent payment of receiverProfit while it is still 0.
            let initiator_faction = self.player(initiator).faction;
            let receiver_faction = self.player(receiver.required("The payment receiver")?).faction;
            self.set_recent_payment(receiver_profit, initiator_faction, receiver_faction, bid.type_);
        }

        if receiver.is_some() && self.player(initiator).faction != Some(payment_receiver) {
            let receiver = receiver.expect("receiver");
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
                    crate::game::amounts::gain(&mut r.resources, receiver_profit, "spice")?;
                    crate::game::amounts::gain(&mut r.resources_after_bidding, bid_emperor_contribution_amount, "spice")?;
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
                    self.bidding.bidding_triggered_bureaucracy = Some(TriggeredBureaucracy { payment_from: initiator_faction, payment_to: Some(payment_receiver) });
                }

                let initiator_faction = self.player(initiator).faction;
                self.set_recent_payment(receiver_profit, initiator_faction, receiver_faction, bid.type_);
            }
        } else if total_amount_received_by_emperor_instead_of_richese > 0 {
            *message = express_part!(" → ", Faction::Emperor, " get ", Payment::of(total_amount_received_by_emperor_instead_of_richese));
            paid_to = Some(Faction::Emperor);
        }

        if (total_amount - receiver_profit - total_amount_received_by_emperor_instead_of_richese) >= 4 {
            self.activate_banker(initiator);
        }

        Ok(paid_to)
    }

    pub fn give_player_extra_card_if_applicable(&mut self, winner: PlayerId) {
        if self.player(winner).is(Faction::Harkonnen) {
            let mut extra_card_must_be_decided_about = false;
            let mut receiver: Option<PlayerId> = None;

            let occupier_of_harkonnen_homeworld = self.occupier_of(World::Harkonnen);
            if let Some(occupier) = occupier_of_harkonnen_homeworld {
                if self.player(occupier).has_room_for_cards(self) {
                    receiver = Some(occupier);
                    extra_card_must_be_decided_about = self.player(occupier).has_ally()
                        && self.player(self.player(occupier).allied_player(self).expect("allied player")).has_room_for_cards(self);
                } else if self.player(occupier).has_ally() && self.player(self.player(occupier).allied_player(self).expect("allied player")).has_room_for_cards(self) {
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
                    self.log_bidding(Some(ExtraCardReceived::new(receiver_faction.expect("faction")).into()), vec![receiver_faction.into(), " receive an extra treachery card".into()]);
                    self.log_bidding_to(receiver_faction.expect("faction"), ExtraCardShown::new(extra_card), vec!["Your extra card is: ".into(), extra_card.into()]);

                    if extra_card_must_be_decided_about {
                        self.bidding.card_that_must_be_kept_or_given_to_ally = Some(extra_card);
                    }
                }
            }
        }
    }

    /// Under `Rule::RicheseTreacheryCards` Richese's cards are in the treachery deck and, with
    /// Richese playing, in its cache too. Once a card is taken from one of the two, it
    /// leaves the other, so it can't be sold twice and end up in two hands (observe bots seed 1
    /// game 163, escapes.md "A Richese card sold twice"). Without the rule nothing is in both.
    pub fn drop_copy_in_richese_cache(&mut self, card: TreacheryCard) {
        crate::game::remove_card(&mut self.decks.richese_cache, Some(card));
    }

    /// See [`drop_copy_in_richese_cache`](Game::drop_copy_in_richese_cache).
    pub fn drop_copy_in_treachery_deck(&mut self, card: TreacheryCard) {
        if let Some(deck) = self.decks.treachery_deck.as_mut() {
            crate::game::remove_card(&mut deck.items, Some(card));
        }
    }

    pub fn draw_treachery_card(&mut self) -> Option<TreacheryCard> {
        if self.decks.treachery_deck.as_ref().map(|d| d.is_empty()) == Some(true) {
            let count = self.decks.treachery_discard_pile.as_ref().expect("treachery discard pile").items.len() as i32;
            self.log(vec![express!("Shuffled ", count, " cards from the treachery discard pile into a new deck")
                .with_log(TreacheryReshuffled::new(count))
                .into()]);

            let discarded: Vec<TreacheryCard> = self.decks.treachery_discard_pile.as_ref().expect("treachery discard pile").items.clone();
            for i in discarded {
                self.decks.treachery_deck.as_mut().expect("treachery deck").items.push(i);
                self.unregister_known(i);
            }

            self.decks.treachery_discard_pile.as_mut().expect("treachery discard pile").clear();
            self.decks.treachery_deck.as_mut().expect("treachery deck").shuffle(&mut self.rng);
            self.stone(Milestone::Shuffled);
        }

        // `TreacheryDeck?.Items.Count > 0`: null > 0 is false.
        if self.decks.treachery_deck.as_ref().map_or(false, |d| d.items.len() > 0) {
            // A null card in the deck comes back as None (engine-conventions.md, "Null handles").
            let card = self.decks.treachery_deck.as_mut().expect("treachery deck").draw();
            self.drop_copy_in_richese_cache(card);
            Some(card)
        } else {
            None
        }
    }

    pub fn finish_bid(&mut self, winner: Option<PlayerId>, winning_bid: Option<&PlacedBid>, card: TreacheryCard, might_replace: bool) {
        self.bidding.card_just_won = Some(card);
        self.bidding.winning_bid = winning_bid.cloned();
        self.bidding.card_sold_on_black_market = None;
        self.bidding.current_bid = None;
        self.bidding.faction_that_may_replace_bought_card = None;
        self.bidding.latest_bid_by_atreides_or_atreides_ally_was_passed = false;

        self.bidding.bids.clear();

        if !self.applicable(Rule::FullPhaseKarma) {
            self.allow(FactionAdvantage::AtreidesBiddingPrescience);
        }

        let mut enter_replacing_card_just_won = might_replace && self.winner_might_replace_with_nexus();

        if might_replace {
            let winner = winner.expect("winner");
            if self.player(winner).ally == Some(Faction::Ixian)
                && self.alliances.ixian_allows_replacing_cards
                && !matches!(self.bidding.current_auction_type, Some(AuctionType::RicheseSilent) | Some(AuctionType::RicheseOnceAround))
            {
                if !self.prevented(FactionAdvantage::IxianAllyDiscardingCard) {
                    self.bidding.faction_that_may_replace_bought_card = self.player(winner).faction;
                    enter_replacing_card_just_won = true;
                } else {
                    if !self.applicable(Rule::FullPhaseKarma) {
                        self.allow(FactionAdvantage::IxianAllyDiscardingCard);
                    }

                    if self.nexus_allows_replacing_bought_cards(winner) {
                        self.bidding.faction_that_may_replace_bought_card = self.player(winner).faction;
                        enter_replacing_card_just_won = true;
                        self.bidding.replacing_bought_card_using_nexus = true;
                    }
                }
            } else if self.nexus_allows_replacing_bought_cards(winner) {
                self.bidding.faction_that_may_replace_bought_card = self.player(winner).faction;
                enter_replacing_card_just_won = true;
                self.bidding.replacing_bought_card_using_nexus = true;
            }
        }

        self.enter_or_else(enter_replacing_card_just_won, Phase::ReplacingCardJustWon, Game::determine_next_step_after_card_was_sold);

        if let Some(b) = self.bidding.bidding_triggered_bureaucracy.clone() {
            self.apply_bureaucracy(b.payment_from, b.payment_to);
            self.bidding.bidding_triggered_bureaucracy = None;
        }
    }

    pub fn determine_next_step_after_card_was_sold(&mut self) {
        self.bidding.richese_bidding_just_finished =
            self.bidding.current_auction_type == Some(AuctionType::RicheseOnceAround) || self.bidding.current_auction_type == Some(AuctionType::RicheseSilent);

        // `CardsOnAuction?.IsEmpty is not true`: also taken when CardsOnAuction is null.
        if self.bidding.cards_on_auction.as_ref().map(|d| d.is_empty()) != Some(true) {
            self.move_to_next_card_on_auction();
        } else if self.bidding.regular_bidding_is_done {
            if self.bidding.richese_auction_should_still_happen {
                self.enter(Phase::RicheseSpecifyingAuction, true);
            } else {
                self.end_bidding_phase();
            }
        } else {
            self.start_regular_bidding();
        }
    }

    pub fn move_to_next_card_on_auction(&mut self) {
        self.bidding.card_number += 1;
        self.bidding.card_that_must_be_kept_or_given_to_ally = None;

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
        if !self.bidding.richese_bidding_just_finished {
            self.with_bid_sequence(|seq, g| seq.next_round(g));
        }

        self.with_bid_sequence(|seq, g| g.skip_players_that_cant_bid(seq));
        self.bidding.winning_bid = None;
        self.enter(Phase::Bidding, true);
    }

    pub fn continue_after_ixian_card_swap_decision(&mut self) {
        if !self.bidding.bidding_round_was_started {
            self.start_bidding_round();
        } else {
            let condition = self.is_playing(Faction::Atreides);
            self.enter_or_else(condition, Phase::WaitingForNextBiddingRound, Game::put_next_card_on_auction);
        }
    }

    pub fn start_bid_sequence_and_auction_type(&mut self, auction_type: Option<AuctionType>, richese_player: Option<PlayerId>, direction: i32) {
        match auction_type {
            Some(AuctionType::BlackMarketNormal) => {
                self.bidding.black_market_auction_type = Some(AuctionType::BlackMarketNormal);
                self.bidding.bid_sequence = Some(PlayerSequence::starting_with_first_player_options(self, true, 1));
            }
            Some(AuctionType::BlackMarketOnceAround) => {
                self.bidding.black_market_auction_type = Some(AuctionType::BlackMarketOnceAround);
                self.bidding.bid_sequence = Some(PlayerSequence::new(self, true, direction, richese_player, true));
            }
            Some(AuctionType::BlackMarketSilent) => {
                self.bidding.bid_sequence = Some(PlayerSequence::starting_with_first_player_options(self, true, 1));
                self.bidding.black_market_auction_type = Some(AuctionType::BlackMarketSilent);
            }
            Some(AuctionType::RicheseOnceAround) => {
                self.bidding.bid_sequence = Some(PlayerSequence::new(self, true, direction, richese_player, true));
            }
            Some(AuctionType::RicheseSilent) => {
                self.bidding.bid_sequence = Some(PlayerSequence::starting_with_first_player_options(self, true, 1));
            }
            Some(AuctionType::Normal) => {
                self.bidding.bid_sequence = Some(PlayerSequence::starting_with_first_player_options(self, true, 1));
                if self.bidding.black_market_auction_type == Some(AuctionType::BlackMarketNormal) {
                    //Continue where black market bidding left off
                    self.with_bid_sequence(|seq, g| seq.next_round(g));
                    self.bidding.winning_bid = None;
                }
            }
            None => {}
        }

        self.bidding.current_auction_type = auction_type;
    }

    pub fn determine_highest_bid(&self, bids: &IndexMap<Faction, PlacedBid>) -> Option<PlacedBid> {
        let highest_bid_value = bids.values().map(|b| b.bid_total_amount()).max().expect("a bid");
        let mut determine_bid_winner_sequence = PlayerSequence::starting_with_first_player_options(self, false, 1);

        let mut i = 0;
        while i < self.maximum_players() {
            let f = determine_bid_winner_sequence.current_faction(self);
            if let Some(value) = bids.get(&f) {
                if value.bid_total_amount() == highest_bid_value {
                    return Some(bids[&f].clone());
                }
            }

            determine_bid_winner_sequence.next_player(self);
            i += 1;
        }

        None
    }

    pub fn end_bidding_phase(&mut self) {
        self.bidding.card_that_must_be_kept_or_given_to_ally = None;
        self.turn.current_ixian_cunning = None;
        if let Some(emperor) = self.get_player(Some(Faction::Emperor)) {
            let r = self.player_mut(emperor);
            crate::game::amounts::gain(&mut r.resources, i32::from(r.resources_after_bidding), "spice").expect("the Emperor's spice after bidding");
            r.resources_after_bidding = 0;
        }

        self.main_phase_end();
        self.enter(Phase::BiddingReport, true);
    }

    pub fn number_of_cards_on_regular_auction(&self) -> i32 {
        (self.bidding.card_number + (self.bidding.cards_on_auction.as_ref().expect("cards on auction").items.len() as i32)) - 1
    }

    pub fn players_that_can_bid(&self) -> Vec<PlayerId> {
        self.player_indices().filter(|p| self.player(*p).has_room_for_cards(self)).collect()
    }

    /// Whether any winner might hold the Ixian Nexus as a secret ally, and so be able to replace
    /// the card they won. Decided on public grounds only: entering `ReplacingCardJustWon` must not
    /// tell anyone which Nexus the winner holds, or whether they hold one at all.
    pub fn winner_might_replace_with_nexus(&self) -> bool {
        self.applicable(Rule::NexusCards) && !self.is_playing(Faction::Ixian)
    }

    pub fn nexus_allows_replacing_bought_cards(&self, p: PlayerId) -> bool {
        let nexus = self.player(p).nexus;
        nexus == Some(Faction::Ixian) && NexusPlayed::can_use_secret_ally(self, p)
    }

    pub fn has_bid_to_pay(&self, p: PlayerId) -> bool {
        match &self.bidding.current_bid {
            Some(current_bid) if self.phase() == Phase::Bidding => {
                Some(current_bid.bid_initiator()) == self.player(p).faction
                    || (Some(current_bid.bid_initiator()) == self.player(p).ally && current_bid.bid_ally_contribution_amount() > 0)
            }
            _ => false,
        }
    }

    /// The most spice `p` may still have to pay for a bid of the current auction: as the bidder,
    /// as the bidder's ally, or as the Emperor supporting it. A silent auction has every bid made
    /// so far to pay, the others only the highest. A bid made with Karama to lift the limit pays
    /// no spice. Spice given away in between was paid anyway,
    /// which left the winner below zero (escapes.md, "Forces below zero").
    pub fn spice_committed_to_bids(&self, p: PlayerId) -> i32 {
        if !matches!(self.phase(), Phase::Bidding | Phase::BlackMarketBidding) {
            return 0;
        }
        let pending: Vec<&PlacedBid> = match self.bidding.current_auction_type {
            Some(AuctionType::RicheseSilent | AuctionType::BlackMarketSilent) => self.bidding.bids.values().collect(),
            _ => self.bidding.current_bid.iter().collect(),
        };
        let me = self.player(p);
        pending
            .into_iter()
            .filter(|bid| !bid.bid_passed() && !bid.bid_using_karma_to_remove_bid_limit())
            .map(|bid| {
                let bidder = Some(bid.bid_initiator());
                let own = if bidder == me.faction { bid.bid_amount() } else { 0 };
                let as_ally = if bidder == me.ally && me.ally.is_some() { bid.bid_ally_contribution_amount() } else { 0 };
                let as_emperor = if me.faction == Some(Faction::Emperor) { bid.bid_emperor_contribution_amount() } else { 0 };
                own + as_ally + as_emperor
            })
            .max()
            .unwrap_or(0)
    }

    /// Whether `p` can give away `amount` spice and still pay their bids.
    pub fn may_give_away_spice(&self, p: PlayerId, amount: i32) -> bool {
        amount <= 0 || i32::from(self.player(p).resources) - amount >= self.spice_committed_to_bids(p)
    }

    pub fn ixian_may_swap_card_on_bid(&self) -> bool {
        // `GetPlayer(Faction.Ixian)?.TreacheryCards.Count > 0`: null > 0 is false.
        self.get_player(Some(Faction::Ixian)).map_or(false, |g| self.player(g).treachery_cards.len() > 0)
            && self.applicable(Rule::IxianSwappingCardOnBid)
            && !self.bidding.ixian_swapped_card_on_bid
    }

    pub fn cards_owned_by(&self, p: PlayerId) -> Vec<TreacheryCard> {
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

    pub fn get_card_set_aside_for_bid(&self, p: PlayerId) -> Option<TreacheryCard> {
        match &self.bidding.current_bid {
            Some(current_bid) if current_bid.bid_player() == Some(p) && current_bid.bid_using_karma_to_remove_bid_limit() => current_bid.bid_karma_card(),
            _ => None,
        }
    }
}
