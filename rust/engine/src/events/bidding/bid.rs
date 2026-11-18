use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Bid {
    pub amount: u16,
    pub ally_contribution_amount: u16,
    pub emperor_contribution_amount: u16,
    pub uses_emperor_secret_ally: bool,
    pub karma_card: Option<TreacheryCard>,
    pub karma_bid: bool,
    #[serde(skip)]
    pub passed: bool,
}

impl Bid {
    /// The bid as the auction keeps it, numbered as the game's next bid.
    pub fn placed(&self, game: &mut Game, by: By) -> PlacedBid {
        PlacedBid {
            number: game.number_bid(),
            type_: EventType::Bid,
            initiator: by.faction,
            player: by.player,
            amount: self.amount,
            ally_contribution_amount: self.ally_contribution_amount,
            emperor_contribution_amount: self.emperor_contribution_amount,
            uses_emperor_secret_ally: self.uses_emperor_secret_ally,
            karma_card: self.karma_card,
            karma_bid: self.karma_bid,
            passed: self.passed,
        }
    }

    /// What the bid offers: nothing for a pass. Validation doesn't check a pass's amounts, so
    /// counting them would let a pass win a silent or once-around auction and pay more than the
    /// bidder has (behavior-changes.md, "Turns, sectors and amounts out of range").
    pub fn total_amount(&self) -> i32 {
        if self.passed {
            return 0;
        }
        (i32::from(self.amount) + i32::from(self.ally_contribution_amount)) + i32::from(self.emperor_contribution_amount)
    }

    pub fn using_karma_to_remove_bid_limit(&self) -> bool {
        self.karma_card.is_some() && !self.karma_bid
    }

    fn validate_bid(&self, game: &Game, by: By) -> Option<Message> {
        if (game.in_bidding().current_auction_type == Some(AuctionType::BlackMarketSilent) || game.in_bidding().current_auction_type == Some(AuctionType::RicheseSilent)) && self.passed {
            return Some(express!("You cannot pass a silent bid"));
        }

        // A Karama bid wins at once (`win_with_karma`), so with `Passed` it would win the card, and
        // without a Karama card of the bidder's it threw halfway through, after changing the game.
        // Those changes aren't in the history, so an undo later replayed a different game (fuzz seed
        // 42 game 110, escapes.md). The client never sends either.
        if self.passed && self.karma_bid {
            return Some(express!("You can't pass and use ", TreacheryCardType::Karma, " in the same bid"));
        }

        if self.passed {
            return None;
        }

        let is_special_auction = game.in_bidding().current_auction_type == Some(AuctionType::RicheseOnceAround) || game.in_bidding().current_auction_type == Some(AuctionType::RicheseSilent);
        if self.karma_bid && is_special_auction {
            return Some(express!("You can't use ", TreacheryCardType::Karma, " in Once Around or Silent bidding"));
        }

        if self.karma_bid && !Bid::can_karma(game, by.player) {
            return Some(express!("You can't use ", TreacheryCardType::Karma, " for this bid"));
        }

        if self.karma_bid {
            if !self.karma_card.is_some_and(|c| Bid::valid_karma_cards(game, by.player).any(|v| v == c)) {
                return Some(express!("Invalid ", TreacheryCardType::Karma, " card"));
            }
            return None;
        }

        let p = game.get_player(by.faction);
        if self.total_amount() < 1 && game.in_bidding().current_auction_type != Some(AuctionType::RicheseSilent) {
            return Some(express!("Bid must be higher than 0"));
        }
        if let Some(current_bid) = &game.in_bidding().current_bid {
            if self.total_amount() <= current_bid.bid_total_amount() && game.in_bidding().current_auction_type != Some(AuctionType::RicheseSilent) {
                return Some(express!("Bid not high enough"));
            }
        }

        if i32::from(self.ally_contribution_amount) > Bid::valid_max_ally_amount(game, by.player) {
            return Some(express!("your ally won't pay that much"));
        }

        // The Emperor's support is limited to what the Emperor offered, not just to their spice
        // (behavior-changes.md, "Turns, sectors and amounts out of range").
        if i32::from(self.emperor_contribution_amount) > 0 && i32::from(self.emperor_contribution_amount) > by.faction.map_or(0, |f| game.spice_for_bids_emperor_can_pay(f)) {
            return Some(express!(Faction::Emperor, " won't pay that much"));
        }

        if !self.using_karma_to_remove_bid_limit() && i32::from(self.amount) > i32::from(game.player(by.player()).resources) {
            return Some(express!("You can't pay ", Payment::of(i32::from(self.amount))));
        }
        if let Some(karma_card) = self.karma_card {
            if p.is_none() || !Karma::valid_karma_cards(game, p.expect("player")).any(|v| v == karma_card) {
                return Some(express!("Invalid ", TreacheryCardType::Karma, " card"));
            }
        }

        // Player is not null here: a null Player has thrown above or failed the KarmaCard check.
        if self.uses_emperor_secret_ally && !Bid::may_use_emperor_secret_ally_opt(game, by.player) {
            return Some(express!("You can't use ", Faction::Emperor, " cunning"));
        }

        if game.in_bidding().current_auction_type == Some(AuctionType::RicheseSilent) && self.total_amount() > i32::from(game.player(by.player()).resources) {
            return Some(express!("In a Silent auction, you can't bid more than you have"));
        }

        None
    }

    pub fn valid_max_amount(game: &Game, p: PlayerId, using_karma: bool) -> i32 {
        if using_karma {
            return 100;
        }
        i32::from(game.player(p).resources)
    }

    pub fn valid_max_ally_amount(g: &Game, p: Option<PlayerId>) -> i32 {
        match p {
            None => 0,
            Some(p) => g.resources_your_ally_can_pay(p),
        }
    }

    pub fn players_to_bid(g: &Game) -> Vec<SequenceElement> {
        // GetPlayersInSequence runs now (PlayersInOrder); its selector runs only in the arms that return it.
        let sequence_players = match &g.in_bidding().bid_sequence {
            Some(s) => either::Either::Left(s.get_players_in_sequence(g)),
            None => either::Either::Right(std::iter::empty()),
        };
        match g.in_bidding().current_auction_type {
            Some(AuctionType::Normal) | Some(AuctionType::RicheseOnceAround) => sequence_players.collect(),
            Some(AuctionType::RicheseSilent) => g
                .players
                .iter()
                .enumerate()
                .map(|(i, p)| SequenceElement { player: Some(g.player_at(i)), has_turn: p.has_room_for_cards(g) && !g.in_bidding().bids.keys().any(|k| Some(*k) == p.faction) })
                .collect(),
            _ => Vec::new(),
        }
    }

    pub fn valid_karma_cards(g: &Game, p: Option<PlayerId>) -> impl Iterator<Item = TreacheryCard> {
        use either::Either;
        use std::iter::empty;
        let Some(p) = p else { return Either::Left(empty()) };
        if g.in_bidding().current_auction_type == Some(AuctionType::Normal) {
            return Either::Right(Karma::valid_karma_cards(g, p));
        }
        Either::Left(empty())
    }

    pub fn can_karma(g: &Game, p: Option<PlayerId>) -> bool {
        // Any() stops at the first match.
        Bid::valid_karma_cards(g, p).next().is_some()
    }

    pub fn may_be_played(game: &Game, player: Option<PlayerId>) -> bool {
        let Some(player) = player else { return false };
        (game.in_bidding().current_auction_type == Some(AuctionType::RicheseSilent) && !game.in_bidding().bids.contains_key_opt(game.player(player).faction) && game.player(player).has_room_for_cards(game))
            || (game.in_bidding().current_auction_type != Some(AuctionType::RicheseSilent) && game.in_bidding().bid_sequence.as_ref().map(|s| s.current_player(game)) == Some(Some(player)))
    }

    pub fn may_use_emperor_secret_ally(game: &Game, player: PlayerId) -> bool {
        game.in_bidding().current_auction_type == Some(AuctionType::Normal) && game.player(player).nexus == Some(Faction::Emperor) && NexusPlayed::can_use_secret_ally(game, player)
    }

    /// `MayUseEmperorSecretAlly` with a possibly null player: the NullReferenceException
    /// happens only after the auction type check.
    pub(crate) fn may_use_emperor_secret_ally_opt(game: &Game, player: Option<PlayerId>) -> bool {
        game.in_bidding().current_auction_type == Some(AuctionType::Normal) && game.player(player.expect("player")).nexus == Some(Faction::Emperor) && NexusPlayed::can_use_secret_ally(game, player.expect("player"))
    }

    fn execute(&self, game: &mut Game, by: By) -> GameResult {
        let bid = self.placed(game, by);
        game.execute_bid(&bid)?;

        // Each player passes automatically at most once per bid. Once a card is won, no rule
        // applies again until someone really bids, so a second automatic pass by the same player
        // means the auction went around without reaching the high bidder. That happened when
        // the high bidder's hand filled up during the auction, so the sequence skipped them
        // forever (fuzz seed 1 game 176, rust/docs/escapes.md).
        let mut auto_passed: Vec<PlayerId> = Vec::new();
        let mut player_to_bid_with_pass_normal_bid = Bid::first_auto_passing_player(game);
        while let Some(p) = player_to_bid_with_pass_normal_bid.filter(|p| !auto_passed.contains(p)) {
            auto_passed.push(p);
            // Executed through `execute_bid`, not as a submitted event: no history entry, because
            // replaying the bid that triggered it passes again.
            let faction = game.player(p).faction;
            let auto_by = By::new(game, faction.required("faction")?);
            let auto_bid = Bid { passed: true, ..Bid::default() }.placed(game, auto_by);
            game.execute_bid(&auto_bid)?;
            player_to_bid_with_pass_normal_bid = Bid::first_auto_passing_player(game);
        }
        Ok(())
    }

    /// The first player whose automation rule passes for them now. Only during `Phase::Bidding`,
    /// the only phase that offers `Bid`. `may_be_played` alone stays true after bidding ends.
    /// When everyone passed on a card while the next bidder's rule was active, every automatic
    /// pass would end the bidding phase again, and the loop would never stop (fuzz seed 1 game 2,
    /// rust/docs/escapes.md).
    fn first_auto_passing_player(game: &Game) -> Option<PlayerId> {
        if game.phase() != Phase::Bidding {
            return None;
        }
        (0..game.players.len())
            .map(|i| game.player_at(i))
            .find(|p| Bid::may_be_played(game, Some(*p)) && game.is_auto_passed_bid(game.player(*p).faction.expect("faction")))
    }

    pub(crate) fn handle_normal_bid(game: &mut Game, this: &PlacedBid) -> GameResult {
        if this.passed || this.karma_bid {
            if this.karma_bid {
                //Immediate Karma
                let card = Bid::win_with_karma(game, this)?;
                game.finish_bid(this.player, Some(this), card, true);
            } else if let Some(winning_bid) = game.in_bidding().current_bid.clone().filter(|cb| {
                cb.type_ == EventType::Bid && game.in_bidding().bid_sequence.as_ref().map(|s| s.current_faction(game)) == Some(cb.bid_initiator())
            }) {
                if winning_bid.bid_using_karma_to_remove_bid_limit() {
                    //Karma was used to bid any amount
                    game.return_karma_card_used_for_bid();
                    let card = Bid::win_with_karma(game, &winning_bid)?;
                    game.finish_bid(winning_bid.player, Some(&winning_bid), card, true);
                } else {
                    if game.in_bidding().cards_on_auction.is_none() {
                        return Ok(());
                    }

                    let receiver = Faction::Emperor;
                    let card = game.win_by_highest_bid(
                        winning_bid.player.required("player")?,
                        &winning_bid,
                        i32::from(winning_bid.amount),
                        i32::from(winning_bid.ally_contribution_amount),
                        i32::from(winning_bid.emperor_contribution_amount),
                        receiver,
                        winning_bid.uses_emperor_secret_ally,
                    )?;

                    game.finish_bid(winning_bid.player, Some(&winning_bid), card, true);
                }
            } else if game.in_bidding().current_bid.is_none() && game.in_bidding().bids.len() as i32 >= game.players_that_can_bid().len() as i32 {
                Bid::everyone_passed_bid(game);
            }
        } else if game.in_bidding().bid_sequence.as_ref().map(|s| s.current_faction(game)) == this.initiator {
            if game.in_bidding().cards_on_auction.is_none() || game.in_bidding().current_bid.is_none() {
                return Ok(());
            }

            let card = Bid::bid_won_by_only_player(game, this, Faction::Emperor)?;
            let current_bid_player = game.in_bidding().current_bid.as_ref().required("current bid")?.bid_player();
            game.finish_bid(current_bid_player, Some(this), card, true);
        }
        Ok(())
    }

    fn win_with_karma(game: &mut Game, bid: &PlacedBid) -> GameResult<TreacheryCard> {
        let winner = game.get_player(bid.initiator);
        if winner.is_none() || bid.karma_card.is_none() || game.in_bidding().cards_on_auction.is_none() {
            game_error!("Cannot resolve karma bid winner or auction deck");
        }
        let winner = winner.required("winner")?;
        let karma_card = bid.karma_card.required("karma card")?;

        game.discard_card(Some(karma_card), true)?;

        let sold = CardSold::new(game.in_bidding().card_number, bid.initiator.required("faction")?, 0, None, vec![], Some(karma_card));
        if karma_card.type_() == TreacheryCardType::Karma {
            let card_number = game.in_bidding().card_number;
            game.log_bidding(
                Some(sold.into()),
                vec![bid.initiator.into(), " get card ".into(), card_number.into(), " using ".into(), TreacheryCardType::Karma.into()],
            );
        } else {
            let card_number = game.in_bidding().card_number;
            game.log_bidding(
                Some(sold.into()),
                vec![
                    bid.initiator.into(),
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

        let card = game.in_bidding_mut().cards_on_auction.as_mut().required("cards on auction")?.try_draw()?;
        game.player_mut(winner).treachery_cards.push(card);
        game.register_won_card_as_known(card);
        let winner_faction = game.player(winner).faction;
        game.log_bidding_to(winner_faction.required("faction")?, CardWonShown::new(card), vec!["You won: ".into(), card.into()]);
        game.give_player_extra_card_if_applicable(winner);
        Ok(card)
    }

    fn everyone_passed_bid(game: &mut Game) {
        let card_number = game.in_bidding().card_number;
        game.log_bidding(Some(BiddingPassedByAll::new(card_number).into()), vec!["Bidding ends because everyone passed".into()]);
        game.stone(Milestone::AuctionWon);

        if game.in_bidding().cards_on_auction.is_none() {
            return;
        }

        while !game.in_bidding().cards_on_auction.as_ref().expect("cards on auction").is_empty() {
            game.shuffle_cards_on_auction();

            let card = game.in_bidding_mut().cards_on_auction.as_mut().expect("cards on auction").draw();
            game.decks.treachery_deck.as_mut().expect("treachery deck").put_on_top(card);
        }

        game.end_bidding_phase();
    }

    /// The only caller passes `Game.CardsOnAuction` as `toDrawFrom`, so this draws from it.
    fn bid_won_by_only_player(game: &mut Game, bid: &PlacedBid, payment_receiver: Faction) -> GameResult<TreacheryCard> {
        game.in_bidding_mut().current_bid = Some(bid.clone());
        let winner = game.get_player(bid.initiator);
        let Some(winner) = winner else {
            game_error!("Cannot resolve winner for single-player bid");
        };
        let mut receiver_income_message = express_part!();

        if !bid.uses_emperor_secret_ally {
            let paid_to = game.pay_for_card(
                winner,
                bid,
                i32::from(bid.amount),
                i32::from(bid.ally_contribution_amount),
                i32::from(bid.emperor_contribution_amount),
                payment_receiver,
                &mut receiver_income_message,
            )?;
            game.log_bid(winner, i32::from(bid.amount), i32::from(bid.ally_contribution_amount), i32::from(bid.emperor_contribution_amount), &receiver_income_message, paid_to, false);
        } else {
            game.play_nexus_card(winner, vec!["get this card for free".into()]);
            game.log_bid(winner, 0, 0, 0, &receiver_income_message, None, true);
        }

        game.stone(Milestone::AuctionWon);
        let card = game.in_bidding_mut().cards_on_auction.as_mut().required("cards on auction")?.try_draw()?;
        game.register_won_card_as_known(card);
        game.player_mut(winner).treachery_cards.push(card);
        let winner_faction = game.player(winner).faction;
        game.log_bidding_to(winner_faction.required("faction")?, CardWonShown::new(card), vec!["You won: ".into(), card.into()]);
        game.give_player_extra_card_if_applicable(winner);
        Ok(card)
    }

    pub(crate) fn handle_richese_bid(game: &mut Game, this: &PlacedBid) -> GameResult {
        let bid_sequence_has_passed_richese = game.in_bidding().bid_sequence.as_ref().map(|s| s.has_passed_richese(game)) == Some(true);
        let richese_has_no_room = game.get_player(Some(Faction::Richese)).map(|w| game.player(w).has_room_for_cards(game)) == Some(false);

        let players_with_room = || game.players.iter().filter(|p| p.has_room_for_cards(game)).count() as i32;
        let is_last_bid = (game.in_bidding().current_auction_type == Some(AuctionType::RicheseSilent) && players_with_room() == game.in_bidding().bids.len() as i32)
            || (game.in_bidding().current_auction_type == Some(AuctionType::RicheseOnceAround)
                && (this.initiator == Some(Faction::Richese) || (richese_has_no_room && bid_sequence_has_passed_richese)));

        if is_last_bid {
            if game.in_bidding().current_auction_type == Some(AuctionType::RicheseSilent) {
                let parts: Vec<MessagePart> = game.in_bidding().bids.iter().map(|(k, v)| express_part!(*k, Payment::of(v.bid_total_amount()), " ")).collect();
                game.log(vec!["Bids: ".into(), parts.into()]);
            }

            let highest_bid = game.determine_highest_bid(&game.in_bidding().bids);
            match highest_bid {
                Some(winning_bid) if winning_bid.bid_total_amount() > 0 => {
                    if game.in_bidding().cards_on_auction.is_none() {
                        return Ok(());
                    }

                    let card = game.win_by_highest_bid(
                        winning_bid.bid_player().required("i bid player")?,
                        &winning_bid,
                        winning_bid.bid_amount(),
                        winning_bid.bid_ally_contribution_amount(),
                        winning_bid.bid_emperor_contribution_amount(),
                        if winning_bid.bid_initiator() != Faction::Richese { Faction::Richese } else { Faction::Emperor },
                        false,
                    )?;

                    game.finish_bid(winning_bid.bid_player(), Some(&winning_bid), card, true);
                }
                _ => {
                    game.log(vec!["Card not sold as no faction bid on it".into()]);
                    let richese = game.get_player(Some(Faction::Richese));
                    if richese.map(|w| game.player(w).has_room_for_cards(game)) == Some(true) {
                        game.enter_bidding(BiddingStep::RicheseKeepingUnsoldCard);
                    } else {
                        if game.in_bidding().cards_on_auction.is_none() {
                            return Ok(());
                        }
                        let card = game.in_bidding_mut().cards_on_auction.as_mut().required("cards on auction")?.try_draw()?;
                        game.decks.removed_treachery_cards.push(card);
                        game.register_won_card_as_known(card);
                        game.log(vec![card.into(), " was removed from the game".into()]);
                        game.finish_bid(None, None, card, false);
                    }
                }
            }
        }
        Ok(())
    }

}

impl EventRules for Bid {
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
        if self.passed {
            return express!(by.faction, " pass");
        }

        if self.karma_bid {
            express!(by.faction, " win the bid using ", TreacheryCardType::Karma)
        } else {
            express!(by.faction, " bid")
        }
    }
}
