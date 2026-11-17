use crate::prelude::*;
use crate::messages::part::Part;
use crate::game::TriggeredBureaucracy;

/// The bidding phase (`PhaseState::Bidding`): its step, the auctions (Black Market, Richese and
/// the regular one), the bids and the card just won. It is dropped when the phase ends, and
/// nothing of it reaches the next phase.
#[derive(Debug)]
pub struct BiddingState {
    pub step: BiddingStep,
    pub bid_sequence: Option<PlayerSequence>,
    pub cards_on_auction: Option<Deck<TreacheryCard>>,
    pub current_auction_type: Option<AuctionType>,
    pub card_number: i32,
    pub current_bid: Option<PlacedBid>,
    // In the order the bids were made (a new bid of a faction replaces its old one and goes last).
    pub bids: IndexMap<Faction, PlacedBid>,
    pub card_just_won: Option<TreacheryCard>,
    pub winning_bid: Option<PlacedBid>,
    pub bidding_round_was_started: bool,
    pub regular_bidding_is_done: bool,
    pub card_that_must_be_kept_or_given_to_ally: Option<TreacheryCard>,
    pub number_of_cards_on_auction: i32,
    pub bidding_triggered_bureaucracy: Option<TriggeredBureaucracy>,
    pub ixian_swapped_card_on_bid: bool,
    pub faction_that_may_replace_bought_card: Option<Faction>,
    pub replacing_bought_card_using_nexus: bool,
    pub black_market_auction_type: Option<AuctionType>,
    pub card_sold_on_black_market: Option<TreacheryCard>,
    pub richese_auction_should_still_happen: bool,
    pub richese_bidding_just_finished: bool,
    pub richese_occupier_specified_card: bool,
    pub latest_bid_by_atreides_or_atreides_ally_was_passed: bool,
}

impl BiddingState {
    /// A new bidding phase at `step`, without a bid sequence yet.
    pub fn new(step: BiddingStep) -> Self {
        BiddingState {
            step,
            bid_sequence: None,
            cards_on_auction: Some(Deck::empty()),
            current_auction_type: None,
            card_number: 0,
            current_bid: None,
            bids: IndexMap::new(),
            card_just_won: None,
            winning_bid: None,
            bidding_round_was_started: false,
            regular_bidding_is_done: false,
            card_that_must_be_kept_or_given_to_ally: None,
            number_of_cards_on_auction: 0,
            bidding_triggered_bureaucracy: None,
            ixian_swapped_card_on_bid: false,
            faction_that_may_replace_bought_card: None,
            replacing_bought_card_using_nexus: false,
            black_market_auction_type: None,
            card_sold_on_black_market: None,
            richese_auction_should_still_happen: false,
            richese_bidding_just_finished: false,
            richese_occupier_specified_card: false,
            latest_bid_by_atreides_or_atreides_ally_was_passed: false,
        }
    }
}

/// Where the bidding phase is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BiddingStep {
    /// Before the auctions, while the host may still wait for players.
    Beginning,
    /// Richese decide whether to sell a card on the Black Market.
    BlackMarketAnnouncement,
    BlackMarketBidding,
    /// Richese choose to auction a cache card first or last.
    RicheseAnnouncingAuction,
    /// Richese (or the occupier of their homeworld) choose the cache card and the auction type.
    RicheseSpecifyingAuction,
    /// Nobody bid on the Richese card: Richese keep it or remove it.
    RicheseKeepingUnsoldCard,
    /// Ixian put one of the drawn cards back on the deck.
    IxianRemovingCard,
    /// Ixian may swap the next card on auction with one of theirs.
    IxianSwappingCard,
    Bidding,
    /// The winner may replace the card they just won.
    ReplacingCardJustWon,
    /// Before the next card is put on auction (Atreides look at it).
    WaitingForNextRound,
    Report,
}

impl BiddingStep {
    pub fn phase(self) -> Phase {
        match self {
            BiddingStep::Beginning => Phase::BeginningOfBidding,
            BiddingStep::BlackMarketAnnouncement => Phase::BlackMarketAnnouncement,
            BiddingStep::BlackMarketBidding => Phase::BlackMarketBidding,
            BiddingStep::RicheseAnnouncingAuction => Phase::RicheseAnnouncingAuction,
            BiddingStep::RicheseSpecifyingAuction => Phase::RicheseSpecifyingAuction,
            BiddingStep::RicheseKeepingUnsoldCard => Phase::RicheseKeepingUnsoldCard,
            BiddingStep::IxianRemovingCard => Phase::IxianRemovingCardFromBid,
            BiddingStep::IxianSwappingCard => Phase::IxianSwappingCard,
            BiddingStep::Bidding => Phase::Bidding,
            BiddingStep::ReplacingCardJustWon => Phase::ReplacingCardJustWon,
            BiddingStep::WaitingForNextRound => Phase::WaitingForNextBiddingRound,
            BiddingStep::Report => Phase::BiddingReport,
        }
    }

}

impl Game {
    /// The bidding phase's state, while the game is in it (an interrupt may suspend it).
    pub fn bidding(&self) -> Option<&BiddingState> {
        match &self.phase {
            PhaseState::Bidding(b) => Some(b),
            _ => None,
        }
    }

    pub fn bidding_mut(&mut self) -> Option<&mut BiddingState> {
        match &mut self.phase {
            PhaseState::Bidding(b) => Some(b),
            _ => None,
        }
    }

    /// The bidding phase's state, for code that only runs in it.
    #[track_caller]
    pub(crate) fn in_bidding(&self) -> &BiddingState {
        self.bidding().expect("in the bidding phase")
    }

    #[track_caller]
    pub(crate) fn in_bidding_mut(&mut self) -> &mut BiddingState {
        self.bidding_mut().expect("in the bidding phase")
    }

    /// The type of the current (or, between auctions, the last) auction of this bidding phase.
    /// None outside bidding.
    pub fn current_auction_type(&self) -> Option<AuctionType> {
        self.bidding().and_then(|b| b.current_auction_type)
    }

    /// The highest bid of the current auction. None outside bidding.
    pub fn current_bid(&self) -> Option<&PlacedBid> {
        self.bidding().and_then(|b| b.current_bid.as_ref())
    }

    /// The bids of the current auction. Empty outside bidding.
    pub fn bids(&self) -> &IndexMap<Faction, PlacedBid> {
        static NO_BIDS: std::sync::LazyLock<IndexMap<Faction, PlacedBid>> = std::sync::LazyLock::new(IndexMap::new);
        match self.bidding() {
            Some(b) => &b.bids,
            None => &NO_BIDS,
        }
    }

    /// The card just won in this bidding phase.
    pub fn card_just_won(&self) -> Option<TreacheryCard> {
        self.bidding().and_then(|b| b.card_just_won)
    }

    /// The cards drawn for the auction, in the bidding phase.
    pub fn cards_on_auction(&self) -> Option<&Deck<TreacheryCard>> {
        self.bidding().and_then(|b| b.cards_on_auction.as_ref())
    }

    /// Starts the bidding phase (`enter_bidding_phase` does the rest).
    pub fn begin_bidding(&mut self) {
        let mut state = BiddingState::new(BiddingStep::Beginning);
        state.bid_sequence = Some(PlayerSequence::starting_with_first_player(self));
        self.enter_state(PhaseState::Bidding(state));
    }

    /// Moves the bidding phase to `step`, dropping any interrupt, as entering a phase did.
    pub fn enter_bidding(&mut self, step: BiddingStep) {
        match self.bidding_mut() {
            Some(b) => {
                b.step = step;
                self.interrupts.clear();
                self.remove_ended_deals(step.phase());
            }
            None => {
                let state = BiddingState::new(step);
                self.enter_state(PhaseState::Bidding(state));
            }
        }
    }

    /// Shuffles the cards on auction with the game's random draws.
    #[track_caller]
    pub(crate) fn shuffle_cards_on_auction(&mut self) {
        let PhaseState::Bidding(b) = &mut self.phase else { panic!("in the bidding phase") };
        b.cards_on_auction.as_mut().expect("cards on auction").shuffle(&mut self.rng);
    }

    /// The latest Karama hand swap interrupt, while Harkonnen are returning cards.
    pub fn karma_hand_swap_interrupt(&self) -> Option<Interrupt> {
        self.interrupts.iter().rev().cloned().find(|i| matches!(i, Interrupt::KarmaHandSwap { .. }))
    }

    /// The Karama hand swap under way: how many cards Harkonnen return, and to whom.
    pub fn karma_hand_swap(&self) -> Option<(i32, Option<Faction>)> {
        match self.karma_hand_swap_interrupt()? {
            Interrupt::KarmaHandSwap { number_of_cards, target } => Some((number_of_cards, target)),
            _ => None,
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
        let mut seq = self.in_bidding_mut().bid_sequence.take().expect("a bid sequence during bidding");
        let result = f(&mut seq, &*self);
        self.in_bidding_mut().bid_sequence = Some(seq);
        result
    }

    pub fn enter_bidding_phase(&mut self) {
        self.main_phase_start(MainPhase::Bidding, true);
        self.allow(FactionAdvantage::ChoamControllingCharity);
        self.allow(FactionAdvantage::BeneGesseritCharity);
        self.begin_bidding();
    }

    pub fn start_bidding_phase(&mut self) {
        let richese = self.get_player(Some(Faction::Richese));
        let condition = richese.is_some()
            && self.applicable(Rule::RicheseBlackMarket)
            && !self.prevented(FactionAdvantage::RicheseBlackMarket)
            && self.player(richese.expect("richese")).treachery_cards.len() > 0;
        if condition {
            self.enter_bidding(BiddingStep::BlackMarketAnnouncement);
        } else {
            self.enter_richese_bidding();
        }
    }

    pub fn enter_richese_bidding(&mut self) {
        self.in_bidding_mut().number_of_cards_on_auction = self.players_that_can_bid().len() as i32;

        if self.in_bidding().card_sold_on_black_market.is_some() {
            self.in_bidding_mut().number_of_cards_on_auction -= 1;
        }

        //CardSoldOnBlackMarket = null;

        if self.in_bidding().number_of_cards_on_auction == 0 {
            self.log(vec![express!("Bidding is skipped because no faction is able to buy cards").with_log(BiddingSkipped::default()).into()]);
            self.end_bidding_phase();
        } else {
            let condition = self.is_playing(Faction::Richese) && self.decks.richese_cache.len() > 0 && !self.prevented(FactionAdvantage::RicheseAuction);
            if condition {
                self.enter_bidding(BiddingStep::RicheseAnnouncingAuction);
            } else {
                self.start_regular_bidding();
            }
        }
    }

    pub fn start_regular_bidding(&mut self) {
        self.in_bidding_mut().regular_bidding_is_done = true;
        let mut number_of_cards_to_draw = self.in_bidding().number_of_cards_on_auction;

        if number_of_cards_to_draw > 0 && self.is_playing(Faction::Ixian) && !self.prevented(FactionAdvantage::IxianSelectingCardsOnAuction) {
            number_of_cards_to_draw += 1;
        }

        self.log(vec![express!(number_of_cards_to_draw, " cards were drawn for bidding").with_log(BiddingDrawn::new(number_of_cards_to_draw)).into()]);

        let mut i = 0;
        while i < number_of_cards_to_draw {
            let card = self.draw_treachery_card();
            if let Some(card) = card {
                if let Some(cards_on_auction) = self.in_bidding_mut().cards_on_auction.as_mut() {
                    cards_on_auction.put_on_bottom(card);
                }
            }
            i += 1;
        }

        self.start_bid_sequence_and_auction_type(Some(AuctionType::Normal), None, 1);

        let condition = self.is_playing(Faction::Ixian) && !self.prevented(FactionAdvantage::IxianSelectingCardsOnAuction);
        if condition {
            self.enter_bidding(BiddingStep::IxianRemovingCard);
        } else {
            self.start_bidding_round();
        }
    }

    pub fn start_bidding_round(&mut self) {
        self.in_bidding_mut().bidding_round_was_started = true;
        self.with_bid_sequence(|seq, g| g.skip_players_that_cant_bid(seq));
        self.enter_bidding(BiddingStep::Bidding);
        let b = self.in_bidding_mut();
        b.current_bid = None;
        b.bids.clear();
        b.card_number = 1;
    }

    /// The next bid's number (`PlacedBid::number`).
    pub(crate) fn number_bid(&mut self) -> u32 {
        self.counters.bids += 1;
        self.counters.bids
    }

    pub fn execute_bid(&mut self, bid: &PlacedBid) -> GameResult {
        if !bid.passed {
            self.return_karma_card_used_for_bid();
            if bid.bid_using_karma_to_remove_bid_limit() {
                let p = bid.player.expect("the event has a player");
                let karma_card = bid.karma_card;
                remove_card(&mut self.player_mut(p).treachery_cards, karma_card);
            }
            self.in_bidding_mut().current_bid = Some(bid.clone());
            self.stone(Milestone::Bid);
        }

        self.with_bid_sequence(|seq, g| {
            seq.next_player(g);
            g.skip_players_that_cant_bid(seq);
        });
        let initiator = bid.initiator;
        self.in_bidding_mut().bids.shift_remove_opt(initiator);
        self.in_bidding_mut().bids.insert_new(initiator.required("faction")?, bid.clone());
        self.check_if_atreides_or_atreides_ally_passed_bid(bid);

        match self.in_bidding().current_auction_type {
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
            self.in_bidding_mut().latest_bid_by_atreides_or_atreides_ally_was_passed = bid.bid_passed();
        }
    }

    pub fn return_karma_card_used_for_bid(&mut self) {
        if let Some(current_bid) = self.in_bidding().current_bid.clone() {
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
        let card_number: Vec<Part> = if self.in_bidding().current_auction_type == Some(AuctionType::Normal) {
            vec![" get card ".into(), express_part!(self.in_bidding().card_number).into(), " for ".into()]
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
            Some(CardSold::new(self.in_bidding().card_number, initiator_faction.expect("faction"), bid_total_amount, paid_to, shares, None).into())
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
        if let Some(record) = record.filter(|_| self.in_bidding().current_auction_type == Some(AuctionType::Normal)) {
            message = message.with_log(record);
        }
        self.current_report_mut().express_message(message);
    }

    pub fn log_bidding_to(&mut self, faction: Faction, record: impl Into<LogRecord>, expression: Vec<Part>) {
        let mut message = Message::express_to(Some(faction), expression);
        if self.in_bidding().current_auction_type == Some(AuctionType::Normal) {
            message = message.with_log(record);
        }
        self.current_report_mut().express_message(message);
    }

    /// Draws from `self.in_bidding().cards_on_auction`; callers check that it is not null first.
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
        let card = self.in_bidding_mut().cards_on_auction.as_mut().required("cards on auction")?.try_draw()?;
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
        self.spend_bid(initiator, bid_amount, bid_ally_contribution_amount, bid_emperor_contribution_amount)?;
        let mut receiver_profit = 0;

        let receiver = self.get_player(Some(payment_receiver));
        let receiver_and_ally_are_richese = self.player(initiator).ally == Some(Faction::Richese) && payment_receiver == Faction::Richese;
        let total_amount = bid_amount + if receiver_and_ally_are_richese { 0 } else { bid_ally_contribution_amount };

        let (total_amount_received_by_emperor_instead_of_richese, emperor_received_instead_message) =
            self.pay_emperor_instead_of_richese(initiator, bid, receiver, receiver_and_ally_are_richese, bid_ally_contribution_amount)?;

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
                    self.in_bidding_mut().bidding_triggered_bureaucracy = Some(TriggeredBureaucracy { payment_from: initiator_faction, payment_to: Some(payment_receiver) });
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

    /// Takes a bid's spice from the bidder, from their ally for the ally's part, and from the
    /// Emperor for theirs.
    fn spend_bid(&mut self, initiator: PlayerId, bid_amount: i32, bid_ally_contribution_amount: i32, bid_emperor_contribution_amount: i32) -> GameResult {
        {
            let p = self.player_mut(initiator);
            crate::game::amounts::spend(&mut p.resources, bid_amount, "spice")?;
        }

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
        Ok(())
    }

    /// When Richese are paid for a card that their ally helped pay for, the Emperor gets the
    /// ally's part instead, unless Karama prevents it. Returns what the Emperor got and the
    /// message part saying so.
    fn pay_emperor_instead_of_richese(&mut self, initiator: PlayerId, bid: &PlacedBid, receiver: Option<PlayerId>, receiver_and_ally_are_richese: bool, bid_ally_contribution_amount: i32) -> GameResult<(i32, MessagePart)> {
        let emperor = self.get_player(Some(Faction::Emperor));
        let mut total_amount_received_by_emperor_instead_of_richese =
            if receiver_and_ally_are_richese && emperor.is_some() && !self.prevented(FactionAdvantage::EmperorReceiveBid) {
                bid_ally_contribution_amount
            } else {
                0
            };
        // C# parity: records a recent payment of receiverProfit while it is still 0.
        let receiver_profit = 0;

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
                self.in_bidding_mut().bidding_triggered_bureaucracy = Some(TriggeredBureaucracy { payment_from: Some(Faction::Richese), payment_to: Some(Faction::Emperor) });
            }

            let initiator_faction = self.player(initiator).faction;
            let receiver_faction = self.player(receiver.required("The payment receiver")?).faction;
            self.set_recent_payment(receiver_profit, initiator_faction, receiver_faction, bid.type_);
        }
        Ok((total_amount_received_by_emperor_instead_of_richese, emperor_received_instead_message))
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
                        self.in_bidding_mut().card_that_must_be_kept_or_given_to_ally = Some(extra_card);
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
        self.in_bidding_mut().card_just_won = Some(card);
        self.in_bidding_mut().winning_bid = winning_bid.cloned();
        self.in_bidding_mut().card_sold_on_black_market = None;
        self.in_bidding_mut().current_bid = None;
        self.in_bidding_mut().faction_that_may_replace_bought_card = None;
        self.in_bidding_mut().latest_bid_by_atreides_or_atreides_ally_was_passed = false;

        self.in_bidding_mut().bids.clear();

        if !self.applicable(Rule::FullPhaseKarma) {
            self.allow(FactionAdvantage::AtreidesBiddingPrescience);
        }

        let not_richese = !matches!(self.in_bidding().current_auction_type, Some(AuctionType::RicheseSilent) | Some(AuctionType::RicheseOnceAround));
        let enter_replacing_card_just_won = might_replace && self.offer_card_replacement(winner.expect("winner"), not_richese);

        if enter_replacing_card_just_won {
            self.enter_bidding(BiddingStep::ReplacingCardJustWon);
        } else {
            self.determine_next_step_after_card_was_sold();
        }

        if let Some(b) = self.in_bidding().bidding_triggered_bureaucracy.clone() {
            self.apply_bureaucracy(b.payment_from, b.payment_to);
            self.in_bidding_mut().bidding_triggered_bureaucracy = None;
        }
    }

    /// Whether the winner of a card might replace it: as Ixian's ally, when the alliance allows it
    /// and `ally_may_replace` (not for a Richese card), or with the Ixian Nexus card, or because
    /// anyone might hold that Nexus card ([`Game::winner_might_replace_with_nexus`]). Records
    /// who may replace it, and whether that uses the Nexus card.
    pub(crate) fn offer_card_replacement(&mut self, winner: PlayerId, ally_may_replace: bool) -> bool {
        let mut replacing = self.winner_might_replace_with_nexus();
        if self.player(winner).ally == Some(Faction::Ixian) && self.alliances.ixian_allows_replacing_cards && ally_may_replace {
            if !self.prevented(FactionAdvantage::IxianAllyDiscardingCard) {
                self.in_bidding_mut().faction_that_may_replace_bought_card = self.player(winner).faction;
                replacing = true;
            } else {
                if !self.applicable(Rule::FullPhaseKarma) {
                    self.allow(FactionAdvantage::IxianAllyDiscardingCard);
                }

                if self.nexus_allows_replacing_bought_cards(winner) {
                    self.in_bidding_mut().faction_that_may_replace_bought_card = self.player(winner).faction;
                    replacing = true;
                    self.in_bidding_mut().replacing_bought_card_using_nexus = true;
                }
            }
        } else if self.nexus_allows_replacing_bought_cards(winner) {
            self.in_bidding_mut().faction_that_may_replace_bought_card = self.player(winner).faction;
            replacing = true;
            self.in_bidding_mut().replacing_bought_card_using_nexus = true;
        }
        replacing
    }

    pub fn determine_next_step_after_card_was_sold(&mut self) {
        self.in_bidding_mut().richese_bidding_just_finished =
            self.in_bidding().current_auction_type == Some(AuctionType::RicheseOnceAround) || self.in_bidding().current_auction_type == Some(AuctionType::RicheseSilent);

        // `CardsOnAuction?.IsEmpty is not true`: also taken when CardsOnAuction is null.
        if self.in_bidding().cards_on_auction.as_ref().map(|d| d.is_empty()) != Some(true) {
            self.move_to_next_card_on_auction();
        } else if self.in_bidding().regular_bidding_is_done {
            if self.in_bidding().richese_auction_should_still_happen {
                self.enter_bidding(BiddingStep::RicheseSpecifyingAuction);
            } else {
                self.end_bidding_phase();
            }
        } else {
            self.start_regular_bidding();
        }
    }

    pub fn move_to_next_card_on_auction(&mut self) {
        self.in_bidding_mut().card_number += 1;
        self.in_bidding_mut().card_that_must_be_kept_or_given_to_ally = None;

        if self.ixian_may_swap_card_on_bid() {
            if !self.prevented(FactionAdvantage::IxianSwappingCard) {
                self.enter_bidding(BiddingStep::IxianSwappingCard);
            } else {
                self.log_prevention_by_karma(FactionAdvantage::IxianSwappingCard);
                if !self.applicable(Rule::FullPhaseKarma) {
                    self.allow(FactionAdvantage::IxianSwappingCard);
                }
                self.enter_bidding(BiddingStep::WaitingForNextRound);
            }
        } else {
            self.enter_bidding(BiddingStep::WaitingForNextRound);
        }
    }

    pub fn put_next_card_on_auction(&mut self) {
        if !self.in_bidding().richese_bidding_just_finished {
            self.with_bid_sequence(|seq, g| seq.next_round(g));
        }

        self.with_bid_sequence(|seq, g| g.skip_players_that_cant_bid(seq));
        self.in_bidding_mut().winning_bid = None;
        self.enter_bidding(BiddingStep::Bidding);
    }

    pub fn continue_after_ixian_card_swap_decision(&mut self) {
        if !self.in_bidding().bidding_round_was_started {
            self.start_bidding_round();
        } else {
            let condition = self.is_playing(Faction::Atreides);
            if condition {
                self.enter_bidding(BiddingStep::WaitingForNextRound);
            } else {
                self.put_next_card_on_auction();
            }
        }
    }

    pub fn start_bid_sequence_and_auction_type(&mut self, auction_type: Option<AuctionType>, richese_player: Option<PlayerId>, direction: i32) {
        match auction_type {
            Some(AuctionType::BlackMarketNormal) => {
                self.in_bidding_mut().black_market_auction_type = Some(AuctionType::BlackMarketNormal);
                self.in_bidding_mut().bid_sequence = Some(PlayerSequence::starting_with_first_player_options(self, true, 1));
            }
            Some(AuctionType::BlackMarketOnceAround) => {
                self.in_bidding_mut().black_market_auction_type = Some(AuctionType::BlackMarketOnceAround);
                self.in_bidding_mut().bid_sequence = Some(PlayerSequence::new(self, true, direction, richese_player, true));
            }
            Some(AuctionType::BlackMarketSilent) => {
                self.in_bidding_mut().bid_sequence = Some(PlayerSequence::starting_with_first_player_options(self, true, 1));
                self.in_bidding_mut().black_market_auction_type = Some(AuctionType::BlackMarketSilent);
            }
            Some(AuctionType::RicheseOnceAround) => {
                self.in_bidding_mut().bid_sequence = Some(PlayerSequence::new(self, true, direction, richese_player, true));
            }
            Some(AuctionType::RicheseSilent) => {
                self.in_bidding_mut().bid_sequence = Some(PlayerSequence::starting_with_first_player_options(self, true, 1));
            }
            Some(AuctionType::Normal) => {
                self.in_bidding_mut().bid_sequence = Some(PlayerSequence::starting_with_first_player_options(self, true, 1));
                if self.in_bidding().black_market_auction_type == Some(AuctionType::BlackMarketNormal) {
                    //Continue where black market bidding left off
                    self.with_bid_sequence(|seq, g| seq.next_round(g));
                    self.in_bidding_mut().winning_bid = None;
                }
            }
            None => {}
        }

        self.in_bidding_mut().current_auction_type = auction_type;
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
        self.in_bidding_mut().card_that_must_be_kept_or_given_to_ally = None;
        self.turn.current_ixian_cunning = None;
        if let Some(emperor) = self.get_player(Some(Faction::Emperor)) {
            let r = self.player_mut(emperor);
            crate::game::amounts::gain(&mut r.resources, i32::from(r.resources_after_bidding), "spice").expect("the Emperor's spice after bidding");
            r.resources_after_bidding = 0;
        }

        self.main_phase_end();
        self.enter_bidding(BiddingStep::Report);
    }

    pub fn number_of_cards_on_regular_auction(&self) -> i32 {
        (self.in_bidding().card_number + (self.in_bidding().cards_on_auction.as_ref().expect("cards on auction").items.len() as i32)) - 1
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
        match self.current_bid() {
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
        let pending: Vec<&PlacedBid> = match self.current_auction_type() {
            Some(AuctionType::RicheseSilent | AuctionType::BlackMarketSilent) => self.bids().values().collect(),
            _ => self.current_bid().into_iter().collect(),
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
            && !self.in_bidding().ixian_swapped_card_on_bid
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
        match self.current_bid() {
            Some(current_bid) if current_bid.bid_player() == Some(p) && current_bid.bid_using_karma_to_remove_bid_limit() => current_bid.bid_karma_card(),
            _ => None,
        }
    }
}

impl GameStatus {
    /// The status in the bidding phase, or `None` in another phase.
    pub(crate) fn bidding_status(game: &Game, _me: Option<PlayerId>, phase: Phase) -> Option<GameStatus> {
        let g = game;
        Some(match phase {
            Phase::BlackMarketAnnouncement => {
                let a = express!("You may now select a card to sell on the Black Market, or pass.");
                let w = express!(Faction::Richese, " are thinking about selling one of their cards on the Black Market...");
                Self::faction_status(game, a, w, Some(Faction::Richese), None)
            }

            Phase::BlackMarketBidding if game.in_bidding().current_auction_type != Some(AuctionType::BlackMarketSilent) => {
                let a = express!("Please bid or pass.");
                let w = express!(g.in_bidding().bid_sequence.as_ref().expect("bid sequence").current_faction(g), " are thinking about their bid...");
                let seq = BlackMarketBid::players_to_bid(g);
                let latest = g.latest_event_0();
                Self::sequence_status(a, w, seq, latest)
            }

            Phase::BlackMarketBidding if game.in_bidding().current_auction_type == Some(AuctionType::BlackMarketSilent) => {
                let players = Self::players_with_room_and_no_bid(g);
                let latest = g.latest_event(EventType::RicheseAnnouncesBlackMarket);
                Self::players_status("Please bid.", "Factions are thinking about their bids...", players, latest)
            }

            Phase::RicheseAnnouncingAuction => {
                let a = express!("Please decide if you will auction a card from your cache First or Last.");
                let w = express!(Faction::Richese, " are deciding to auction a card from their cache First or Last...");
                Self::faction_status(game, a, w, Some(Faction::Richese), None)
            }

            Phase::RicheseSpecifyingAuction => {
                let a = express!("Please select a ", Faction::Richese, " card to put on auction.");
                let w = express!(Self::faction_or_occupier(g, Faction::Richese, World::Richese), " are putting a ", Faction::Richese, " card on auction...");
                let f = Self::faction_or_occupier(g, Faction::Richese, World::Richese);
                Self::faction_status(g, a, w, f, None)
            }

            Phase::RicheseKeepingUnsoldCard => {
                let a = express!("Please decide if you wish to keep this unsold card.");
                let w = express!(Self::faction_or_occupier(g, Faction::Richese, World::Richese), " are deciding about keeping the unsold card...");
                let f = Self::faction_or_occupier(g, Faction::Richese, World::Richese);
                Self::faction_status(g, a, w, f, None)
            }

            Phase::IxianRemovingCardFromBid => {
                let a = express!("Please put one card from the auction on top or at the bottom of the Treachery Card deck.");
                let w = express!(Self::faction_or_occupier(g, Faction::Ixian, World::Ixian), " are putting one card from the auction on top or at the bottom of the Treachery Card deck...");
                let f = Self::faction_or_occupier(g, Faction::Ixian, World::Ixian);
                Self::faction_status(g, a, w, f, None)
            }

            Phase::IxianSwappingCard => {
                let a = express!("You may swap the next card on auction with a card from your hand, or pass.");
                let w = express!(Faction::Ixian, " are thinking about swapping the next card on auction with a card from their hand...");
                Self::faction_status(game, a, w, Some(Faction::Ixian), None)
            }

            Phase::Bidding if game.in_bidding().current_auction_type != Some(AuctionType::RicheseSilent) => {
                let a = express!("Please bid or pass.");
                let w = express!(g.in_bidding().bid_sequence.as_ref().expect("bid sequence").current_faction(g), " are thinking about their bid...");
                let seq = Bid::players_to_bid(g);
                let latest = g.latest_event_0();
                Self::sequence_status(a, w, seq, latest)
            }

            Phase::Bidding if game.in_bidding().current_auction_type == Some(AuctionType::RicheseSilent) => {
                let players = Self::players_with_room_and_no_bid(g);
                let latest = g.latest_event(EventType::RicheseSpecifiesAuction);
                Self::players_status("Please bid.", "Factions are thinking about their bids...", players, latest)
            }

            Phase::ReplacingCardJustWon => {
                let a = express!("You might discard the card you just won and draw a new card instead, in case you have the necessary Nexus or alliance advantage.");
                let w = express!(g.in_bidding().winning_bid.as_ref().map(|b| b.initiator), " might replace the card they just won with a card from the deck in case they have the required Nexus or alliance advantage...");
                let p = g.get_player(g.in_bidding().winning_bid.as_ref().map(|b| b.initiator).flatten());
                Self::player_status(a, w, p, None)
            }

            Phase::WaitingForNextBiddingRound => Self::status("Factions are waiting for the next card to be put on auction...", None),

            _ => return None,
        })
    }
}

impl Game {
    /// What `player` may do in the bidding phase (see `add_player_actions`).
    pub(crate) fn add_bidding_actions(&self, player: PlayerId, is_host: bool, result: &mut Vec<EventType>) {
        let faction = self.player(player).faction;
        match self.phase() {
            Phase::BlackMarketAnnouncement => {
                if faction == Some(Faction::Richese) {
                    result.push(EventType::RicheseAnnouncesBlackMarket);
                }
            }

            Phase::BlackMarketBidding => {
                if BlackMarketBid::may_be_played(self, player) {
                    result.push(EventType::BlackMarketBid);
                }
                if faction == Some(Faction::Emperor) && self.applicable(Rule::EmperorSupportingNonAllyBids) {
                    result.push(EventType::EmperorBidSupport);
                }
            }

            Phase::RicheseAnnouncingAuction => {
                if faction == Some(Faction::Richese) {
                    result.push(EventType::RicheseAnnouncesAuction);
                }
            }

            Phase::RicheseSpecifyingAuction => {
                if RicheseSpecifiesAuction::may_specify_card(self, player) || RicheseSpecifiesAuction::may_specify_type_of_auction(self, player) {
                    result.push(EventType::RicheseSpecifiesAuction);
                }
            }

            Phase::Bidding => {
                if Bid::may_be_played(self, Some(player)) {
                    result.push(EventType::Bid);
                }
                if faction == Some(Faction::Emperor) && self.applicable(Rule::EmperorSupportingNonAllyBids) {
                    result.push(EventType::EmperorBidSupport);
                }
            }

            Phase::RicheseKeepingUnsoldCard => {
                if faction == Some(Faction::Richese) {
                    result.push(EventType::RicheseKeepsUnsoldCard);
                }
            }

            Phase::IxianRemovingCardFromBid => {
                let occupier_of_ixian_homeworld = self.occupier_of(World::Ixian);
                if (occupier_of_ixian_homeworld.is_none() && faction == Some(Faction::Ixian)) || occupier_of_ixian_homeworld == Some(player) {
                    result.push(EventType::IxianRemovedCardFromAuction);
                }
            }

            Phase::IxianSwappingCard => {
                if faction == Some(Faction::Ixian) {
                    result.push(EventType::IxianSwappedCardOnBid);
                }
            }

            Phase::ReplacingCardJustWon => {
                if faction == self.in_bidding().faction_that_may_replace_bought_card {
                    result.push(EventType::ReplacedCardWon);
                }
            }

            Phase::WaitingForNextBiddingRound => {
                if !is_host && faction == Some(Faction::Atreides) {
                    result.push(EventType::EndPhase);
                }
            }

            Phase::BiddingReport => {
                if faction == Some(Faction::Tleilaxu)
                    && self.players.len() > 1
                    && !self.prevented(FactionAdvantage::TleilaxuIncreasingRevivalLimits)
                {
                    result.push(EventType::SetIncreasedRevivalLimits);
                }
                // Kaitain's high threshold. Salusa Secundus has no discard.
                if faction == Some(Faction::Emperor) && self.player(player).has_high_threshold(self, World::Emperor) && i32::from(self.player(player).resources) >= 2 {
                    result.push(EventType::EmperorDiscarded);
                }
            }

            _ => {}
        }
    }
}
