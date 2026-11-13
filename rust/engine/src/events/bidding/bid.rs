use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct Bid {
    pub amount: i32,
    pub ally_contribution_amount: i32,
    pub emperor_contribution_amount: i32,
    pub uses_emperor_secret_ally: bool,
    #[canon(name = "_karmaCardId")]
    pub karma_card_id: i32,
    pub karma_bid: bool,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for Bid {
    fn default() -> Self {
        Bid {
            amount: Default::default(),
            ally_contribution_amount: Default::default(),
            emperor_contribution_amount: Default::default(),
            uses_emperor_secret_ally: Default::default(),
            karma_card_id: -1,
            karma_bid: Default::default(),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl Bid {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn karma_card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::lookup().find(self.karma_card_id)
    }

    pub fn set_karma_card(&mut self, value: Option<TreacheryCard>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.karma_card_id = TreacheryCardManager::get_id(value);
    }

    pub fn total_amount(&self) -> i32 {
        // C# int addition wraps (hazards 1.6).
        self.amount.wrapping_add(self.ally_contribution_amount).wrapping_add(self.emperor_contribution_amount)
    }

    pub fn using_karma_to_remove_bid_limit(&self) -> bool {
        self.karma_card().is_some() && !self.karma_bid
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if (game.current_auction_type == AuctionType::BlackMarketSilent || game.current_auction_type == AuctionType::RicheseSilent) && self.passed {
            return Some(express!("You cannot pass a silent bid"));
        }

        if self.passed {
            return None;
        }

        let is_special_auction = game.current_auction_type == AuctionType::RicheseOnceAround || game.current_auction_type == AuctionType::RicheseSilent;
        if self.karma_bid && is_special_auction {
            return Some(express!("You can't use ", TreacheryCardType::Karma, " in Once Around or Silent bidding"));
        }

        if self.karma_bid && !Bid::can_karma(game, self.player_opt()) {
            return Some(express!("You can't use ", TreacheryCardType::Karma, " for this bid"));
        }

        if self.karma_bid {
            return None;
        }

        let p = game.get_player(Some(self.initiator()));
        if self.total_amount() < 1 && game.current_auction_type != AuctionType::RicheseSilent {
            return Some(express!("Bid must be higher than 0"));
        }
        if let Some(current_bid) = &game.current_bid {
            if self.total_amount() <= current_bid.i_bid_total_amount() && game.current_auction_type != AuctionType::RicheseSilent {
                return Some(express!("Bid not high enough"));
            }
        }

        if self.ally_contribution_amount > Bid::valid_max_ally_amount(game, self.player_opt()) {
            return Some(express!("your ally won't pay that much"));
        }

        let emperor = game.get_player(Some(Faction::Emperor));
        if self.emperor_contribution_amount > 0 && self.emperor_contribution_amount > emperor.map_or(0, |r| game.player(r).resources) {
            return Some(express!(Faction::Emperor, " won't pay that much"));
        }

        if !self.using_karma_to_remove_bid_limit() && self.amount > game.player(self.player()).resources {
            return Some(express!("You can't pay ", Payment::of(self.amount)));
        }
        if let Some(karma_card) = self.karma_card() {
            if p.is_none() || !crate::compat::lazy::contains(Karma::valid_karma_cards(game, p.nre()), &karma_card) {
                return Some(express!("Invalid ", TreacheryCardType::Karma, " card"));
            }
        }

        // Player is not null here: a null Player has thrown above or failed the KarmaCard check.
        if self.uses_emperor_secret_ally && !Bid::may_use_emperor_secret_ally_opt(game, self.player_opt()) {
            return Some(express!("You can't use ", Faction::Emperor, " cunning"));
        }

        if game.current_auction_type == AuctionType::RicheseSilent && self.total_amount() > game.player(self.player()).resources {
            return Some(express!("In a Silent auction, you can't bid more than you have"));
        }

        None
    }

    pub fn valid_max_amount(game: &Game, p: PlayerIndex, using_karma: bool) -> i32 {
        if using_karma {
            return 100;
        }
        game.player(p).resources
    }

    pub fn valid_max_ally_amount(g: &Game, p: Option<PlayerIndex>) -> i32 {
        let p = p.and_then(crate::compat::cs::CsNull::opt);
        match p {
            None => 0,
            Some(p) => g.resources_your_ally_can_pay(p),
        }
    }

    pub fn players_to_bid(g: &Game) -> Vec<SequenceElement> {
        // GetPlayersInSequence runs now (PlayersInOrder); its selector runs only in the arms that return it.
        let sequence_players = match &g.bid_sequence {
            Some(s) => crate::compat::lazy::Either::A(s.get_players_in_sequence(g)),
            None => crate::compat::lazy::Either::B(crate::compat::lazy::empty()),
        };
        match g.current_auction_type {
            AuctionType::Normal | AuctionType::RicheseOnceAround => sequence_players.collect(),
            AuctionType::RicheseSilent => g
                .players
                .iter()
                .enumerate()
                .map(|(i, p)| SequenceElement { player: Some(g.player_at(i)), has_turn: p.has_room_for_cards(g) && !g.bids.keys().any(|k| *k == p.faction) })
                .collect(),
            _ => Vec::new(),
        }
    }

    pub fn valid_karma_cards(g: &Game, p: Option<PlayerIndex>) -> impl Iterator<Item = TreacheryCard> {
        use crate::compat::lazy::{Either, empty};
        let p = p.and_then(crate::compat::cs::CsNull::opt);
        let Some(p) = p else { return Either::A(empty()) };
        if g.current_auction_type == AuctionType::Normal {
            return Either::B(Karma::valid_karma_cards(g, p));
        }
        Either::A(empty())
    }

    pub fn can_karma(g: &Game, p: Option<PlayerIndex>) -> bool {
        let p = p.and_then(crate::compat::cs::CsNull::opt);
        // Any() stops at the first match.
        Bid::valid_karma_cards(g, p).next().is_some()
    }

    pub fn may_be_played(game: &Game, player: Option<PlayerIndex>) -> bool {
        let player = player.and_then(crate::compat::cs::CsNull::opt);
        let Some(player) = player else { return false };
        (game.current_auction_type == AuctionType::RicheseSilent && !game.bids.contains_key(&game.player(player).faction) && game.player(player).has_room_for_cards(game))
            || (game.current_auction_type != AuctionType::RicheseSilent && game.bid_sequence.as_ref().map(|s| s.current_player(game)) == Some(Some(player)))
    }

    pub fn may_use_emperor_secret_ally(game: &Game, player: PlayerIndex) -> bool {
        game.current_auction_type == AuctionType::Normal && game.player(player).nexus == Faction::Emperor && NexusPlayed::can_use_secret_ally(game, player)
    }

    /// `MayUseEmperorSecretAlly` with a possibly null player: the NullReferenceException
    /// happens only after the auction type check.
    pub(crate) fn may_use_emperor_secret_ally_opt(game: &Game, player: Option<PlayerIndex>) -> bool {
        let player = player.and_then(crate::compat::cs::CsNull::opt);
        game.current_auction_type == AuctionType::Normal && game.player(player.nre()).nexus == Faction::Emperor && NexusPlayed::can_use_secret_ally(game, player.nre())
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.execute_bid(self)?;

        let mut player_to_bid_with_pass_normal_bid = Bid::first_auto_passing_player(game);
        while let Some(p) = player_to_bid_with_pass_normal_bid {
            // C# parity: executed through Game.ExecuteBid, not Execute, so no pre/post-event tasks and no History entry (hazards 5.5).
            let faction = game.player(p).faction;
            let mut auto_bid = Bid::new(game, faction);
            auto_bid.passed = true;
            game.execute_bid(&Rc::new(auto_bid))?;
            player_to_bid_with_pass_normal_bid = Bid::first_auto_passing_player(game);
        }
        Ok(())
    }

    /// `Game.Players.FirstOrDefault(p => MayBePlayed(Game, p) && Game.IsAutoPassedBid(p.Faction))`
    fn first_auto_passing_player(game: &Game) -> Option<PlayerIndex> {
        (0..game.players.len())
            .map(|i| game.player_at(i))
            .find(|p| Bid::may_be_played(game, Some(*p)) && game.is_auto_passed_bid(game.player(*p).faction))
    }

    pub fn handle_normal_bid(self: &Rc<Self>, game: &mut Game) -> GameResult {
        if self.passed || self.karma_bid {
            if self.karma_bid {
                //Immediate Karma
                let card = self.win_with_karma(game, self)?;
                let this = self.clone().into_event();
                game.finish_bid(self.player_opt(), Some(&this), card, true);
            } else if let Some(GameEvent::Bid(winning_bid)) = game.current_bid.clone().filter(|cb| {
                matches!(cb, GameEvent::Bid(_)) && game.bid_sequence.as_ref().map(|s| s.current_faction(game)) == Some(cb.i_bid_initiator())
            }) {
                if winning_bid.using_karma_to_remove_bid_limit() {
                    //Karma was used to bid any amount
                    game.return_karma_card_used_for_bid();
                    let card = self.win_with_karma(game, &winning_bid)?;
                    let wb = winning_bid.clone().into_event();
                    game.finish_bid(winning_bid.player_opt(), Some(&wb), card, true);
                } else {
                    if game.cards_on_auction.is_none() {
                        return Ok(());
                    }

                    let receiver = Faction::Emperor;
                    let wb = winning_bid.clone().into_event();
                    let card = game.win_by_highest_bid(
                        winning_bid.player_opt().required("player")?,
                        &wb,
                        winning_bid.amount,
                        winning_bid.ally_contribution_amount,
                        winning_bid.emperor_contribution_amount,
                        receiver,
                        winning_bid.uses_emperor_secret_ally,
                    )?;

                    game.finish_bid(winning_bid.player_opt(), Some(&wb), card, true);
                }
            } else if game.current_bid.is_none() && game.bids.len() as i32 >= game.players_that_can_bid().len() as i32 {
                self.everyone_passed_bid(game);
            }
        } else if game.bid_sequence.as_ref().map(|s| s.current_faction(game)) == Some(self.initiator()) {
            if game.cards_on_auction.is_none() || game.current_bid.is_none() {
                return Ok(());
            }

            let card = self.bid_won_by_only_player(game, self, Faction::Emperor)?;
            let this = self.clone().into_event();
            let current_bid_player = game.current_bid.as_ref().required("current bid")?.i_bid_player();
            game.finish_bid(current_bid_player, Some(&this), card, true);
        }
        Ok(())
    }

    pub fn win_with_karma(&self, game: &mut Game, bid: &Bid) -> GameResult<TreacheryCard> {
        let winner = self.get_player(game, Some(bid.initiator()));
        if winner.is_none() || bid.karma_card().is_none() || game.cards_on_auction.is_none() {
            game_error!("Cannot resolve karma bid winner or auction deck");
        }
        let winner = winner.required("winner")?;
        let karma_card = bid.karma_card().required("karma card")?;

        game.discard_treachery_card_boolean(Some(karma_card), true)?;

        let sold = CardSold::new(game.card_number, bid.initiator(), 0, Faction::None, vec![], Some(karma_card));
        if karma_card.type_() == TreacheryCardType::Karma {
            let card_number = game.card_number;
            game.log_bidding(
                Some(Rc::new(sold)),
                vec![bid.initiator().into(), " get card ".into(), card_number.into(), " using ".into(), TreacheryCardType::Karma.into()],
            );
        } else {
            let card_number = game.card_number;
            game.log_bidding(
                Some(Rc::new(sold)),
                vec![
                    bid.initiator().into(),
                    " get card ".into(),
                    card_number.into(),
                    " using ".into(),
                    karma_card.into(),
                    " for ".into(),
                    TreacheryCardType::Karma.into(),
                ],
            );
        }

        game.stone(Milestone::AuctionWon);
        game.stone(Milestone::Karma);

        let card = game.cards_on_auction.as_mut().required("cards on auction")?.try_draw()?;
        game.player_mut(winner).treachery_cards.push(card);
        game.register_won_card_as_known(card);
        let winner_faction = game.player(winner).faction;
        game.log_bidding_to(winner_faction, Rc::new(CardWonShown::new(card)), vec!["You won: ".into(), card.into()]);
        game.give_player_extra_card_if_applicable(winner);
        Ok(card)
    }

    pub fn everyone_passed_bid(&self, game: &mut Game) {
        let card_number = game.card_number;
        game.log_bidding(Some(Rc::new(BiddingPassedByAll::new(card_number))), vec!["Bidding ends because everyone passed".into()]);
        game.stone(Milestone::AuctionWon);

        if game.cards_on_auction.is_none() {
            return;
        }

        while !game.cards_on_auction.as_ref().nre().is_empty() {
            game.cards_on_auction.as_mut().nre().shuffle();

            let card = game.cards_on_auction.as_mut().nre().draw();
            game.treachery_deck.as_mut().nre().put_on_top(card);
        }

        game.end_bidding_phase();
    }

    /// The only caller passes `Game.CardsOnAuction` as `toDrawFrom`, so this draws from it.
    pub fn bid_won_by_only_player(&self, game: &mut Game, bid: &Rc<Bid>, payment_receiver: Faction) -> GameResult<TreacheryCard> {
        game.current_bid = Some(bid.clone().into_event());
        let winner = self.get_player(game, Some(bid.initiator()));
        let Some(winner) = winner else {
            game_error!("Cannot resolve winner for single-player bid");
        };
        let mut receiver_income_message = express_part!();

        let bid_event = bid.clone().into_event();
        if !bid.uses_emperor_secret_ally {
            let paid_to = game.pay_for_card(
                winner,
                &bid_event,
                bid.amount,
                bid.ally_contribution_amount,
                bid.emperor_contribution_amount,
                payment_receiver,
                &mut receiver_income_message,
            )?;
            game.log_bid(winner, bid.amount, bid.ally_contribution_amount, bid.emperor_contribution_amount, &receiver_income_message, paid_to, false);
        } else {
            game.play_nexus_card(winner, vec!["Secret Ally".into(), "get this card for free".into()]);
            game.log_bid(winner, 0, 0, 0, &receiver_income_message, Faction::None, true);
        }

        game.stone(Milestone::AuctionWon);
        let card = game.cards_on_auction.as_mut().required("cards on auction")?.try_draw()?;
        game.register_won_card_as_known(card);
        game.player_mut(winner).treachery_cards.push(card);
        let winner_faction = game.player(winner).faction;
        game.log_bidding_to(winner_faction, Rc::new(CardWonShown::new(card)), vec!["You won: ".into(), card.into()]);
        game.give_player_extra_card_if_applicable(winner);
        Ok(card)
    }

    pub fn handle_richese_bid(&self, game: &mut Game) -> GameResult {
        let bid_sequence_has_passed_richese = game.bid_sequence.as_ref().map(|s| s.has_passed_richese(game)) == Some(true);
        let richese_has_no_room = self.get_player(game, Some(Faction::Richese)).map(|w| game.player(w).has_room_for_cards(game)) == Some(false);

        let players_with_room = || game.players.iter().filter(|p| p.has_room_for_cards(game)).count() as i32;
        let is_last_bid = (game.current_auction_type == AuctionType::RicheseSilent && players_with_room() == game.bids.len() as i32)
            || (game.current_auction_type == AuctionType::RicheseOnceAround
                && (self.initiator() == Faction::Richese || (richese_has_no_room && bid_sequence_has_passed_richese)));

        if is_last_bid {
            if game.current_auction_type == AuctionType::RicheseSilent {
                let parts: Vec<MessagePart> = game.bids.iter().map(|(k, v)| express_part!(*k, Payment::of(v.i_bid_total_amount()), " ")).collect();
                self.log_objects(game, vec!["Bids: ".into(), parts.into()]);
            }

            let highest_bid = game.determine_highest_bid(&game.bids);
            match highest_bid {
                Some(winning_bid) if winning_bid.i_bid_total_amount() > 0 => {
                    if game.cards_on_auction.is_none() {
                        return Ok(());
                    }

                    let card = game.win_by_highest_bid(
                        winning_bid.i_bid_player().required("i bid player")?,
                        &winning_bid,
                        winning_bid.i_bid_amount(),
                        winning_bid.i_bid_ally_contribution_amount(),
                        winning_bid.i_bid_emperor_contribution_amount(),
                        if winning_bid.i_bid_initiator() != Faction::Richese { Faction::Richese } else { Faction::Emperor },
                        false,
                    )?;

                    game.finish_bid(winning_bid.i_bid_player(), Some(&winning_bid), card, true);
                }
                _ => {
                    self.log_objects(game, vec!["Card not sold as no faction bid on it".into()]);
                    let richese = self.get_player(game, Some(Faction::Richese));
                    if richese.map(|w| game.player(w).has_room_for_cards(game)) == Some(true) {
                        game.enter(Phase::RicheseKeepingUnsoldCard, true);
                    } else {
                        if game.cards_on_auction.is_none() {
                            return Ok(());
                        }
                        let card = game.cards_on_auction.as_mut().required("cards on auction")?.try_draw()?;
                        game.removed_treachery_cards.push(card);
                        game.register_won_card_as_known(card);
                        self.log_objects(game, vec![card.into(), " was removed from the game".into()]);
                        game.finish_bid(None, None, card, false);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.passed {
            return express!(self.initiator(), " pass");
        }

        if self.karma_bid {
            express!(self.initiator(), " win the bid using ", TreacheryCardType::Karma)
        } else {
            express!(self.initiator(), " bid")
        }
    }
}

impl GameEventMethods for Bid {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::Bid
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Bid(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Bid::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        Bid::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Bid::get_message(self, game)
    }

}

impl PassableGameEventMethods for Bid {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}

impl crate::events::bid_trait::IBid for Bid {
    fn total_amount(&self) -> i32 {
        Bid::total_amount(self)
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
        self.uses_emperor_secret_ally
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
        Bid::using_karma_to_remove_bid_limit(self)
    }

    fn karma_card(&self) -> Option<TreacheryCard> {
        Bid::karma_card(self)
    }

    fn get_message(&self, game: &Game) -> Message {
        Bid::get_message(self, game)
    }

    fn karma_bid(&self) -> bool {
        self.karma_bid
    }

}
