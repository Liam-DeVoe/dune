use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct GameStatus {
    /// `Option` elements: `[game.CurrentBattle!.Territory!]` can add a null (the `!` does not throw).
    pub highlighted_territories: Vec<Option<Territory>>,
    pub waiting_in_sequence: Vec<SequenceElement>,
    /// `Option` elements: `PlayersThatNeedToMakeABattlePlan`/`PlayersThatNeedToCallTraitor` add
    /// `AggressivePlayer!`/`DefendingPlayer!`, which can be null (the `!` does not throw).
    pub waiting_for_players: Vec<Option<PlayerIndex>>,
    pub flash_info: Vec<FlashInfo>,
    pub timed_event: Option<GameEvent>,
    pub waiting_for_host: bool,
    pub description_when_awaited: Message,
    pub description_when_waiting: Message,
}

impl GameStatus {
    pub fn new(message_when_awaited: Message, message_when_waiting: Message, waiting_for_player: Option<PlayerIndex>, timed_event: Option<GameEvent>) -> Self {
        let waiting_for_player = waiting_for_player.and_then(crate::compat::cs::CsNull::opt);
        let list = if waiting_for_player.is_some() { vec![waiting_for_player] } else { vec![] };
        Self::new_message_message_list_player_game_event(message_when_awaited, message_when_waiting, list, timed_event)
    }

    pub fn new_message_message_list_player_game_event(message_when_awaited: Message, message_when_waiting: Message, waiting_for_players: Vec<Option<PlayerIndex>>, timed_event: Option<GameEvent>) -> Self {
        GameStatus {
            highlighted_territories: vec![],
            waiting_in_sequence: vec![],
            waiting_for_players,
            flash_info: vec![],
            timed_event,
            waiting_for_host: false,
            description_when_awaited: message_when_awaited,
            description_when_waiting: message_when_waiting,
        }
    }

    pub fn new_message_message_list_sequence_element_game_event(message_when_awaited: Message, message_when_waiting: Message, waiting_in_sequence: Vec<SequenceElement>, timed_event: Option<GameEvent>) -> Self {
        GameStatus {
            highlighted_territories: vec![],
            waiting_in_sequence,
            waiting_for_players: vec![],
            flash_info: vec![],
            timed_event,
            waiting_for_host: false,
            description_when_awaited: message_when_awaited,
            description_when_waiting: message_when_waiting,
        }
    }

    pub fn new_message_message_game_event(message_when_awaited: Message, message_when_waiting: Message, timed_event: Option<GameEvent>) -> Self {
        GameStatus {
            highlighted_territories: vec![],
            waiting_in_sequence: vec![],
            waiting_for_players: vec![],
            flash_info: vec![],
            timed_event,
            waiting_for_host: true,
            description_when_awaited: message_when_awaited,
            description_when_waiting: message_when_waiting,
        }
    }

    pub fn new_message_game_event(message: Message, timed_event: Option<GameEvent>) -> Self {
        Self::new_message_message_game_event(message.clone(), message, timed_event)
    }

    pub fn waiting_for_me(&self, player: PlayerIndex, is_host: bool) -> bool {
        (self.waiting_for_host && is_host)
            || self.waiting_for_players.contains(&Some(player))
            || self.waiting_in_sequence.iter().any(|se| se.player == Some(player) && se.has_turn)
    }

    pub fn waiting_for_others(&self, player: PlayerIndex, is_host: bool) -> bool {
        !self.waiting_for_me(player, is_host)
    }

    pub fn player_should_be_highlighted(&self, p: PlayerIndex) -> bool {
        self.waiting_for_players.contains(&Some(p))
    }

    pub fn get_message(&self, player: PlayerIndex, is_host: bool) -> Message {
        if self.waiting_for_me(player, is_host) { self.description_when_awaited.clone() } else { self.description_when_waiting.clone() }
    }

    /// `&mut Game` because `Game.NextPlayerToBattle` advances `BattleSequence` when read (hazards §4.3).
    /// The arms below follow the C# switch expression in order; each `when` guard is evaluated
    /// only when its pattern matches, and again for each later arm with the same phase.
    pub fn determine_status(game: &mut Game, me_: Option<PlayerIndex>, is_player: bool) -> GameStatus {
        let me_ = me_.and_then(crate::compat::cs::CsNull::opt);
        let phase = game.current_phase;
        let result = match phase {
            /* Phase Beginnings */
            Phase::BeginningOfStorm | Phase::Thumper | Phase::BeginningOfCharity | Phase::BeginningOfBidding | Phase::BeginningOfResurrection | Phase::BeginningOfCollection => Self::status_message_message_game_event(
                express!("You may now continue with the ", game.current_main_phase, " phase..."),
                express!("Waiting for the host to continue the ", game.current_main_phase, " phase..."),
                None,
            ),

            /* Phase Endings */
            Phase::StormReport | Phase::BlowReport | Phase::CharityReport | Phase::BiddingReport | Phase::ResurrectionReport | Phase::ShipmentAndMoveConcluded | Phase::CollectionReport | Phase::TurnConcluded => {
                Self::status_message_game_event(express!(game.current_main_phase, " phase ended."), None)
            }

            /* Non-mainphase-related phases */
            Phase::Clairvoyance => {
                let g: &Game = game;
                let a = express!("Please answer a question from ", g.latest_clairvoyance.as_ref().map(|c| c.initiator()), " by ", TreacheryCardType::Clairvoyance, ".");
                let w = express!("Waiting for an answer to a ", TreacheryCardType::Clairvoyance, " question...");
                let f = g.latest_clairvoyance.as_ref().nre().target;
                Self::status_game_message_message_faction_game_event(g, a, w, Some(f), None)
            }

            Phase::SearchingDiscarded => {
                let g: &Game = game;
                let p = g.owner_of_treachery_card_type(TreacheryCardType::SearchDiscarded);
                Self::status_string_string_player_game_event("Please take a card from the treachery discard pile.", "Waiting for a card to be taken from the treachery discard pile...", p, None)
            }

            Phase::TradingCards => {
                let g: &Game = game;
                let offer_player = g.current_card_trade_offer.as_ref().nre().player();
                let f = g.player(offer_player).ally;
                Self::status_game_string_string_faction_game_event(g, "Please select which card to give to your ally.", "Waiting for a card to be returned...", f, None)
            }

            Phase::Bureaucracy => {
                let g: &Game = game;
                let p = g.player_skilled_as(LeaderSkill::Bureaucrat);
                Self::status_string_string_player_game_event("Please decide whether to apply Bureaucracy to the latest payment.", "Waiting for Bureaucracy to be applied to the latest payment...", p, None)
            }

            Phase::Discarding => {
                let g: &Game = game;
                Self::status_game_string_string_list_faction_game_event(g, "Please decide which card to discard.", "Waiting factions to discard...", &g.factions_that_must_discard, None)
            }

            /* Setup */
            Phase::AwaitingPlayers => Self::status_string_string_game_event(
                "Configure the game and wait for players to join. Start the game when ready.",
                "Waiting for the host to configure and start the game...",
                None,
            ),

            Phase::SelectingFactions => Self::status("Players may now choose a faction...", None),

            Phase::TradingFactions => Self::status("Players may now trade factions...", None),

            Phase::AssigningInitialSkills | Phase::AssigningSkill => {
                let g: &Game = game;
                let players: Vec<Option<PlayerIndex>> = (0..g.players.len()).filter(|i| g.players[*i].skills_to_choose_from.len() > 0).map(|i| Some(g.player_at(i))).collect();
                Self::status_string_string_list_player_game_event("You may now assign a skill to a leader.", "Waiting for factions to assign leader skills...", players, None)
            }

            Phase::BeneGesseritPredicting => {
                let a = express!("Please predict who will win the game and when.");
                let w = express!(Faction::BeneGesserit, " are predicting who will win the game and when...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::BeneGesserit), None)
            }

            Phase::HarkonnenMulligan => {
                let a = express!("You may draw a new set of traitors if you were dealt two or more of your own leaders.");
                let w = express!(Faction::Harkonnen, " may draw a new set of traitors if they were dealt two or more of their own leaders...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Harkonnen), None)
            }

            Phase::SelectingTraitors => {
                let players = Self::players_that_have_not_acted_or_passed(game);
                Self::status_string_string_list_player_game_event("Please select one leader to keep as a traitor.", "Factions are selecting traitors...", players, None)
            }

            Phase::CustomizingDecks => Self::status_string_string_game_event("Please select which cards will be in play.", "The host is selecting which cards will be in play...", None),

            Phase::PerformCustomSetup => {
                let a = express!("Please set up ", game.next_faction_to_perform_custom_setup(), " starting ", Concept::Resource, " and force positions.");
                let w = express!("The host is setting up ", game.next_faction_to_perform_custom_setup(), " starting ", Concept::Resource, " and force positions...");
                Self::status_message_message_game_event(a, w, None)
            }

            Phase::FremenSettingUp => {
                let a = express!("Please choose your initial force positions.");
                let w = express!(Faction::Fremen, " are setting up their starting force positions...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Fremen), None)
            }

            Phase::BeneGesseritSettingUp => {
                let a = express!("Please select your starting force position.");
                let w = express!(Faction::BeneGesserit, " are selecting their starting force position...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::BeneGesserit), None)
            }

            Phase::MoritaniSettingUp => {
                let a = express!("Please select your starting force position.");
                let w = express!(Faction::Moritani, " are selecting their starting force position...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Moritani), None)
            }

            Phase::IxianSelectingCard => {
                let a = express!("Please select your starting Treachery Card.");
                let w = express!(Faction::Ixian, " are selecting their starting Treachery Card...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Ixian), None)
            }

            Phase::HmsPlacement => {
                let a = express!("Please position the Hidden Mobile Stronghold.");
                let w = express!(Faction::Ixian, " are positioning the Hidden Mobile Stronghold...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Ixian), None)
            }

            /* Storm */
            Phase::HmsMovement => {
                let a = express!("You may move the Hidden Mobile Stronghold to a sector in an adjacent territory, or pass.");
                let w = express!(Faction::Ixian, " are moving the Hidden Mobile Stronghold...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Ixian), None)
            }

            Phase::DiallingStorm => {
                let g: &Game = game;
                let factions: Vec<Faction> = g.factions_in_play.borrow().iter().copied().filter(|f| g.has_battle_wheel.contains(f) && !g.has_acted_or_passed.contains(f)).collect();
                Self::status_game_string_string_list_faction_game_event(g, "Please dial a number to determine storm movement.", "Storm movement is being determined...", &factions, None)
            }

            Phase::MetheorAndStormSpell => Self::status_message_game_event(express!("Factions may now use ", TreacheryCardType::Metheor, " or ", TreacheryCardType::StormSpell, "..."), None),

            Phase::StormLosses => {
                let g: &Game = game;
                let a = express!("Please decide which forces were killed by the storm in ", TakeLosses::losses_to_take(g).location, ".");
                let w = express!(TakeLosses::losses_to_take(g).faction, " are deciding which forces were killed by the storm in ", TakeLosses::losses_to_take(g).location, "...");
                let f = cs::index(&g.storm_losses_to_take, 0).faction;
                Self::status_game_message_message_faction_game_event(g, a, w, Some(f), None)
            }

            /* Spice Blow */
            Phase::FremenSendingMonsterA | Phase::FremenSendingMonsterB => {
                let a = express!("Please decide where to let ", Concept::Monster, " appear.");
                let w = express!(Faction::Fremen, " are deciding where to let ", Concept::Monster, " appear...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Fremen), None)
            }

            Phase::FremenRidingMonsterA | Phase::FremenRidingMonsterB => {
                let a = express!("Please decide where to travel with ", Concept::Monster, ".");
                let w = express!(Faction::Fremen, " are deciding where to travel with ", Concept::Monster, "...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Fremen), None)
            }

            // C# parity: these two phases are listed again in the BeneGesseritIntrudedBy... arm below, which
            // is unreachable for them; this arm (with `LastShipmentOrMovement!`) wins.
            Phase::BeneGesseritIntrudedByFremenRidingMonsterA | Phase::BeneGesseritIntrudedByFremenRidingMonsterB => {
                let g: &Game = game;
                let a = express!("Please decide what to do in response to an intrusion of ", g.last_shipment_or_movement.as_ref().nre().to_location(g).map(|l| l.territory()), "; be fighters or advisors?");
                let w = express!(Faction::BeneGesserit, " are deciding what to do in response to an intrusion of ", g.last_shipment_or_movement.as_ref().nre().to_location(g).map(|l| l.territory()), "...");
                Self::status_game_message_message_faction_game_event(g, a, w, Some(Faction::BeneGesserit), None)
            }

            Phase::BlowA | Phase::HarvesterA => {
                let t = game.latest_spice_card_a.nre().location().nre().territory();
                Self::status_message_game_event(express!("Factions may use a ", TreacheryCardType::Harvester, " to double the ", Concept::Resource, " blow in ", t, "..."), None)
            }
            Phase::BlowB | Phase::HarvesterB => {
                let t = game.latest_spice_card_b.nre().location().nre().territory();
                Self::status_message_game_event(express!("Factions may use a ", TreacheryCardType::Harvester, " to double the ", Concept::Resource, " blow in ", t, "..."), None)
            }

            Phase::VoteAllianceA | Phase::VoteAllianceB => {
                let players = Self::players_that_have_not_acted_or_passed(game);
                Self::status_string_string_list_player_game_event("Please vote Yes or No to a Nexus.", "Factions are voting on about a Nexus...", players, None)
            }

            Phase::AllianceA | Phase::AllianceB => {
                let players = Self::players_that_have_not_acted_or_passed(game);
                Self::status_string_string_list_player_game_event(
                    "You may now make or break an alliance.",
                    "Factions are making and breaking alliances...",
                    players,
                    None,
                )
            }

            Phase::NexusCards => {
                let g: &Game = game;
                let players: Vec<Option<PlayerIndex>> = g.player_indices().filter(|p| NexusCardDrawn::applicable(g, *p)).map(Some).collect();
                Self::status_string_string_list_player_game_event("Do you wish to draw a Nexus card in case you have none or have your own faction?", "Factions are thinking about drawing Nexus cards...", players, None)
            }

            /* Charity */
            Phase::ClaimingCharity => Self::status_message_game_event(express!("Factions may now claim charity if eligible."), None),

            /* Bidding */
            Phase::BlackMarketAnnouncement => {
                let a = express!("You may now select a card to sell on the Harkonnen Markt, or pass.");
                let w = express!(Faction::Richese, " are thinking about selling one of their cards on the Black Market...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Richese), None)
            }

            Phase::BlackMarketBidding if game.current_auction_type != AuctionType::BlackMarketSilent => {
                let g: &Game = game;
                let a = express!("Please bid or pass.");
                let w = express!(g.bid_sequence.as_ref().nre().current_faction(g), " are thinking about their bid...");
                let seq = BlackMarketBid::players_to_bid(g);
                let latest = g.latest_event_0();
                Self::status_message_message_list_sequence_element_game_event(a, w, seq, latest)
            }

            Phase::BlackMarketBidding if game.current_auction_type == AuctionType::BlackMarketSilent => {
                let g: &Game = game;
                let players = Self::players_with_room_and_no_bid(g);
                let latest = g.latest_event(GameEventType::RicheseAnnouncesBlackMarket);
                Self::status_string_string_list_player_game_event("Please bid.", "Factions are thinking about their bids...", players, latest)
            }

            Phase::RicheseAnnouncingAuction => {
                let a = express!("Please decide if you will auction a card from your cache First or Last.");
                let w = express!(Faction::Richese, " are deciding to auction a card from their cache First or Last...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Richese), None)
            }

            Phase::RicheseSpecifyingAuction => {
                let g: &Game = game;
                let a = express!("Please select a ", Faction::Richese, " card to put on auction.");
                let w = express!(Self::faction_or_occupier(g, Faction::Richese, World::Richese), " are putting a ", Faction::Richese, " card on auction...");
                let f = Self::faction_or_occupier(g, Faction::Richese, World::Richese);
                Self::status_game_message_message_faction_game_event(g, a, w, Some(f), None)
            }

            Phase::RicheseKeepingUnsoldCard => {
                let g: &Game = game;
                let a = express!("Please decide if you wish to keep this unsold card.");
                let w = express!(Self::faction_or_occupier(g, Faction::Richese, World::Richese), " are deciding about keeping the unsold card...");
                let f = Self::faction_or_occupier(g, Faction::Richese, World::Richese);
                Self::status_game_message_message_faction_game_event(g, a, w, Some(f), None)
            }

            Phase::IxianRemovingCardFromBid => {
                let g: &Game = game;
                let a = express!("Please put one card from the auction on top or at the bottom of the Treachery Card deck.");
                let w = express!(Self::faction_or_occupier(g, Faction::Ixian, World::Ixian), " are putting one card from the auction on top or at the bottom of the Treachery Card deck...");
                let f = Self::faction_or_occupier(g, Faction::Ixian, World::Ixian);
                Self::status_game_message_message_faction_game_event(g, a, w, Some(f), None)
            }

            Phase::IxianSwappingCard => {
                let a = express!("You may swap the next card on auction with a card from your hand, or pass.");
                let w = express!(Faction::Ixian, " are thinking about swapping the next card on auction with a card from their hand...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Ixian), None)
            }

            Phase::Bidding if game.current_auction_type != AuctionType::RicheseSilent => {
                let g: &Game = game;
                let a = express!("Please bid or pass.");
                let w = express!(g.bid_sequence.as_ref().nre().current_faction(g), " are thinking about their bid...");
                let seq = Bid::players_to_bid(g);
                let latest = g.latest_event_0();
                Self::status_message_message_list_sequence_element_game_event(a, w, seq, latest)
            }

            Phase::Bidding if game.current_auction_type == AuctionType::RicheseSilent => {
                let g: &Game = game;
                let players = Self::players_with_room_and_no_bid(g);
                let latest = g.latest_event(GameEventType::RicheseSpecifiesAuction);
                Self::status_string_string_list_player_game_event("Please bid.", "Factions are thinking about their bids...", players, latest)
            }

            Phase::ReplacingCardJustWon => {
                let g: &Game = game;
                let a = express!("You might discard the card you just won and draw a new card instead, in case you have the necessary Nexus or alliance advantage.");
                let w = express!(g.winning_bid.as_ref().map(|b| b.initiator()), " might replace the card they just won with a card from the deck in case they have the required Nexus or alliance advantage...");
                let p = g.get_player(g.winning_bid.as_ref().map(|b| b.initiator()));
                Self::status_message_message_player_game_event(a, w, p, None)
            }

            Phase::WaitingForNextBiddingRound => Self::status("Factions are waiting for the next card to be put on auction...", None),

            Phase::PerformingKarmaHandSwap => {
                let a = express!("Please decide which ", game.karma_hand_swap_number_of_cards, " cards to return to ", game.karma_hand_swap_target, ".");
                let w = express!(Faction::Harkonnen, " are deciding which ", game.karma_hand_swap_number_of_cards, " cards to return to ", game.karma_hand_swap_target, "...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Harkonnen), None)
            }

            /* Revival */
            Phase::Resurrection => Self::status("Factions may now reclaim forces and leaders.", None),

            /* Ship & Move */
            Phase::BeginningOfShipAndMove => Self::status_message_message_game_event(
                express!("You may now start the Ship & Move sequence..."),
                express!("Waiting for the host to start the Ship & Move sequence..."),
                None,
            ),

            Phase::NonGuildShip if game.shipment_and_move_sequence.as_ref().nre().current_faction(game) == Faction::Fremen => {
                let g: &Game = game;
                let a = express!("Please decide to rally forces or pass.");
                let w = express!(Faction::Fremen, " are thinking about rallying forces...");
                Self::status_ship_and_move(g, a, w)
            }

            Phase::NonGuildShip if game.shipment_and_move_sequence.as_ref().nre().current_faction(game) != Faction::Fremen => {
                let g: &Game = game;
                let a = express!("Please decide to ship forces or pass.");
                let w = express!(g.shipment_and_move_sequence.as_ref().nre().current_faction(g), " are thinking about shipping forces...");
                Self::status_ship_and_move(g, a, w)
            }

            Phase::GuildShip if game.guild_may_delay() => {
                let g: &Game = game;
                let a = express!("Please decide to ship now or delay your turn and let other factions go first.");
                let w = express!(Faction::Guild, " are deciding about taking their turn now...");
                Self::status_ship_and_move(g, a, w)
            }

            Phase::GuildShip if !game.guild_may_delay() => {
                let g: &Game = game;
                let a = express!("Please decide to ship forces or pass.");
                let w = express!(Faction::Guild, " are thinking about shipping forces...");
                Self::status_ship_and_move(g, a, w)
            }

            Phase::BeneGesseritAccompaniesNonGuildShip | Phase::BeneGesseritAccompaniesGuildShip => {
                let a = express!("Do you wish to accompany the latest shipment?");
                let w = express!(Faction::BeneGesserit, " are thinking about accompanying the latest shipment...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::BeneGesserit), None)
            }

            Phase::TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip
            | Phase::TerrorTriggeredByBeneGesseritAccompaniesGuildShip
            | Phase::TerrorTriggeredByFremenRidingMonsterA
            | Phase::TerrorTriggeredByFremenRidingMonsterB
            | Phase::TerrorTriggeredByGuildShip
            | Phase::TerrorTriggeredByNonGuildShip
            | Phase::TerrorTriggeredByNonGuildMove
            | Phase::TerrorTriggeredByGuildMove
            | Phase::TerrorTriggeredByCaravan
            | Phase::TerrorTriggeredByRevival => {
                let a = express!("Do you wish to respond to this intrusion with terror?");
                let w = express!(Faction::Moritani, " are thinking about resorting to terror...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Moritani), None)
            }

            Phase::AllianceByTerror => {
                let a = express!("Please decide about entering an alliance offered to you by ", Faction::Moritani);
                let w = express!(game.alliance_by_terror_offered_to, " are considering entering an alliance with ", Faction::Moritani, "...");
                let f = game.alliance_by_terror_offered_to;
                Self::status_game_message_message_faction_game_event(game, a, w, Some(f), None)
            }

            Phase::AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip
            | Phase::AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip
            | Phase::AmbassadorTriggeredByFremenRidingMonsterA
            | Phase::AmbassadorTriggeredByFremenRidingMonsterB
            | Phase::AmbassadorTriggeredByGuildShip
            | Phase::AmbassadorTriggeredByNonGuildShip
            | Phase::AmbassadorTriggeredByNonGuildMove
            | Phase::AmbassadorTriggeredByGuildMove
            | Phase::AmbassadorTriggeredByCaravan
            | Phase::AmbassadorTriggeredByRevival => {
                let g: &Game = game;
                let a = express!("Do you wish to activate your ambassador?");
                let w = express!(Faction::Ecaz, Self::ally(g, Faction::Ecaz), " are thinking about activating their ambassador...");
                let players = Self::player_and_ally(g, Faction::Ecaz);
                Self::status_message_message_list_player_game_event(a, w, players, None)
            }

            Phase::AllianceByAmbassador => {
                let a = express!("Please decide about entering an alliance offered to you by ", Faction::Ecaz);
                let w = express!(game.alliance_by_ambassador_offered_to, " are considering entering an alliance with ", Faction::Ecaz, "...");
                let f = game.alliance_by_ambassador_offered_to;
                Self::status_game_message_message_faction_game_event(game, a, w, Some(f), None)
            }

            // BeneGesseritIntrudedByFremenRidingMonsterA/B are also listed here in C#, but the arm above
            // already takes them.
            Phase::BeneGesseritIntrudedByNonGuildShip
            | Phase::BeneGesseritIntrudedByGuildShip
            | Phase::BeneGesseritIntrudedByNonGuildMove
            | Phase::BeneGesseritIntrudedByGuildMove
            | Phase::BeneGesseritIntrudedByCaravan
            | Phase::BeneGesseritIntrudedByRevival => {
                let g: &Game = game;
                let a = express!("Please decide how to respond to the intrusion in ", g.last_shipment_or_movement.as_ref().and_then(|e| e.to_location(g)).map(|l| l.territory()), "; be fighters or advisors?");
                let w = express!(Faction::BeneGesserit, " are deciding how to respond to the intrusion in ", g.last_shipment_or_movement.as_ref().and_then(|e| e.to_location(g)).map(|l| l.territory()), "...");
                Self::status_game_message_message_faction_game_event(g, a, w, Some(Faction::BeneGesserit), None)
            }

            Phase::NonGuildMove => {
                let g: &Game = game;
                let a = express!("Please decide to move forces or pass.");
                let w = express!(g.shipment_and_move_sequence.as_ref().nre().current_faction(g), " are thinking about moving forces.");
                Self::status_ship_and_move(g, a, w)
            }

            Phase::GuildMove => {
                let g: &Game = game;
                let a = express!("Please decide to move forces or pass.");
                let w = express!(Faction::Guild, " are thinking about moving forces.");
                Self::status_ship_and_move(g, a, w)
            }

            /* Battle */
            Phase::BeginningOfBattle => Self::status_string_string_game_event("You may proceed to the first battle when ready.", "Waiting for the host to proceed to the first battle...", None),

            Phase::ClaimingBattle => {
                let g: &Game = game;
                let a = express!("You may now decide who will fight in ", g.battle_about_to_start.as_ref().nre().territory(g));
                let battle = g.battle_about_to_start.clone().nre();
                let f: Option<Faction> = if g.has_low_threshold(Faction::Ecaz) { battle.opponent_of_faction(g, Faction::Ecaz).map(|p| g.player(p).faction) } else { Some(Faction::Ecaz) };
                let w = express!(f, " are deciding who will fight in ", battle.territory(g), "...");
                let p = if g.has_low_threshold(Faction::Ecaz) { battle.opponent_of_faction(g, Faction::Ecaz) } else { g.get_player(Some(Faction::Ecaz)) };
                Self::status_message_message_player_game_event(a, w, p, None)
            }

            Phase::Thought => {
                let g: &Game = game;
                let a = express!(g.current_thought.as_ref().nre().initiator(), " asked you a question and are waiting for your answer.");
                let battle = g.current_battle.clone().nre();
                let thought_initiator = g.current_thought.as_ref().nre().initiator();
                let w = express!("Waiting for ", battle.opponent_of_faction(g, thought_initiator).map(|p| g.player(p).faction), " to answer a question...");
                let battle = g.current_battle.clone().nre();
                let p = battle.opponent_of_faction(g, g.current_thought.as_ref().nre().initiator());
                Self::status_message_message_player_game_event(a, w, p, None)
            }

            Phase::BattlePhase => Self::determine_battle_status(game, me_),

            Phase::MeltingRock => {
                let g: &Game = game;
                let a = express!("Please decide how to use your ", TreacheryCardType::Rockmelter, ".");
                let w = express!("Waiting for a decision on how they wish to use their ", TreacheryCardType::Rockmelter, "...");
                let p = if g.aggressor_plan.as_ref().nre().has_rock_melter() {
                    Some(g.current_battle.as_ref().nre().player())
                } else {
                    g.current_battle.as_ref().nre().defending_player(g)
                };
                Self::status_message_message_player_game_event(a, w, p, None)
            }

            Phase::CallTraitorOrPass => {
                let players = Self::players_that_need_to_call_traitor(game);
                Self::status_string_string_list_player_game_event("You may now call TREACHERY if the enemy leader is a traitor under your command.", "Waiting for a faction to call TREACHERY...", players, None)
            }

            Phase::CancellingTraitor => Self::status_string_string_game_event("You may proceed when done waiting for players to cancel a traitor call.", "Players may now try to cancel a traitor call...", None),

            Phase::Retreating => {
                let g: &Game = game;
                let a = express!("Please decide about retreating forces from ", g.current_battle.as_ref().nre().territory(g), ".");
                let w = express!(g.battle_loser, " are thinking about retreating forces from ", g.current_battle.as_ref().nre().territory(g), ".");
                Self::status_game_message_message_faction_game_event(g, a, w, g.battle_loser, None)
            }

            Phase::AvoidingAudit => {
                let g: &Game = game;
                let a = express!("Please decide if you wish to avoid being audited.");
                let w = express!(g.player(g.auditee().nre()).faction, " are thinking about avoiding a scheduled audit...");
                let p = g.auditee();
                Self::status_message_message_player_game_event(a, w, p, None)
            }

            Phase::CaptureDecision => {
                let a = express!("Please decide if you wish to capture a leader.");
                let w = express!(Faction::Harkonnen, " are deciding about capturing a leader...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Harkonnen), None)
            }

            Phase::Auditing => {
                let a = express!("Please conclude the audit when done inspecting your opponents cards.");
                let w = express!(Faction::Choam, " are performing their audit...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Choam), None)
            }

            Phase::BattleConclusion => {
                let a = express!("You won!");
                let w = express!(game.battle_winner, " are celebrating their victory in battle...");
                let f = game.battle_winner;
                Self::status_game_message_message_faction_game_event(game, a, w, f, None)
            }

            Phase::RevealingFacedancer => {
                let a = express!("You may reveal a leader to be one of your face dancers.");
                let w = express!("Waiting for ", Faction::Tleilaxu, " to reveal a face dancer...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Tleilaxu), None)
            }

            Phase::Facedancing => {
                let a = express!("You may now replace opponent forces by your own.");
                let w = express!("Waiting for ", Faction::Tleilaxu, " to replace forces...");
                Self::status_game_message_message_faction_game_event(game, a, w, Some(Faction::Tleilaxu), None)
            }

            // Each guard reads NextPlayerToBattle, which advances BattleSequence (hazards §4.3).
            // When the first guard fails, C# reads it again for the second guard, and if that
            // read is not null either, falls through to the default arm.
            Phase::BattleReport if game.next_player_to_battle().is_some() => {
                Self::status_message_game_event(express!("Factions may now review the battle report before the next battle begins..."), None)
            }
            Phase::BattleReport if game.next_player_to_battle().is_none() => Self::status_message_game_event(express!(game.current_main_phase, " phase ended."), None),

            /* Collection */
            Phase::DividingCollectedResources => {
                let g: &Game = game;
                let a = express!("Please make a proposal about how to divide collected ", Concept::Resource, ".");
                let w = express!("Waiting for ", g.collected_resources_to_be_divided.first().map(|r| r.first_faction), " to propose how to divide collected ", Concept::Resource);
                let f = g.collected_resources_to_be_divided.first().map(|r| r.first_faction);
                Self::status_game_message_message_faction_game_event(g, a, w, f, None)
            }

            Phase::AcceptingResourceDivision => {
                let g: &Game = game;
                let a = express!("Please decide about the proposed division of ", Concept::Resource, ".");
                let w = express!(g.collected_resources_to_be_divided.first().map(|r| r.other_faction), " are thinking about the proposed division");
                let f = g.collected_resources_to_be_divided.first().map(|r| r.other_faction);
                Self::status_game_message_message_faction_game_event(g, a, w, f, None)
            }

            /* Mentat */
            Phase::Extortion => Self::status_message_message_game_event(
                express!("You may proceed when done waiting for players to avoid future {0} by {1}", TerrorType::Extortion, Faction::Moritani),
                express!("Players may pay {0} to avoid future {1} by {2}...", Concept::Resource, TerrorType::Extortion, Faction::Moritani),
                None,
            ),

            Phase::ReplacingFaceDancer => Self::status_game_string_string_faction_game_event(
                game,
                "You may replace an unrevealed Face Dancer with a new one from the Traitor Deck.",
                "Waiting for a Face Dancer to be replaced...",
                Faction::Tleilaxu,
                None,
            ),

            Phase::Contemplate => Self::status_string_string_game_event("Check for victories?", "Waiting for the host to start determining victories...", None),

            Phase::GameEnded => Self::status("The game has ended.", None),

            _ => Self::status_message_game_event(express!(format!("Unknown phase: {}", game.current_phase)), None),
        };

        let mut result = result;
        let my_faction = match me_ {
            Some(me) => game.player(me).faction,
            None => Faction::None,
        };
        result.flash_info = Self::determine_flash(game, my_faction, is_player);
        result.highlighted_territories = Self::determine_highlights(game, my_faction);

        result
    }

    /// The `Status(Message, Message, game.ShipmentAndMoveSequence.GetPlayersInSequence().ToList(),
    /// game.FindMostRecentEvent(typeof(EndPhase), typeof(GuildDelay), typeof(Move)))` tail shared by the
    /// Ship & Move arms (Model/GameStatus.cs:329-413). Rust-only helper; arguments are evaluated in C# order.
    fn status_ship_and_move(g: &Game, a: Message, w: Message) -> GameStatus {
        let seq = g.shipment_and_move_sequence.as_ref().nre().get_players_in_sequence(g).collect();
        let latest = g.find_most_recent_event(vec![GameEventType::EndPhase, GameEventType::GuildDelay, GameEventType::Move]);
        Self::status_message_message_list_sequence_element_game_event(a, w, seq, latest)
    }

    /// `game.Players.Where(p => p.HasRoomForCards && !game.Bids.ContainsKey(p.Faction)).ToList()`
    /// (Model/GameStatus.cs:267/302). Rust-only helper.
    fn players_with_room_and_no_bid(g: &Game) -> Vec<Option<PlayerIndex>> {
        (0..g.players.len())
            .map(|i| g.player_at(i))
            .filter(|p| g.player(*p).has_room_for_cards(g) && !g.bids.contains_key(&g.player(*p).faction))
            .map(Some)
            .collect()
    }

    pub fn faction_or_occupier(g: &Game, f: Faction, w: World) -> Faction {
        let occupier = g.occupier_of(w);
        if let Some(occupier) = occupier {
            return g.player(occupier).faction;
        }
        f
    }

    pub fn player_and_ally(g: &Game, f: Faction) -> Vec<Option<PlayerIndex>> {
        let Some(player) = g.get_player(Some(f)) else { return vec![] };

        let mut result = vec![Some(player)];
        if g.player(player).has_ally() {
            result.push(g.player(player).allied_player(g));
        }
        result
    }

    pub fn ally(g: &Game, f: Faction) -> MessagePart {
        let player = g.get_player(Some(f));
        let cond = player.is_some_and(|p| g.player(p).has_ally());
        // `player!.Ally` is evaluated even when the condition is false: NullReferenceException when absent.
        express_part_if!(cond; g.player(player.nre()).ally)
    }

    pub fn determine_highlights(game: &Game, my_faction: Faction) -> Vec<Option<Territory>> {
        match game.current_phase {
            Phase::FremenSettingUp if my_faction == Faction::Fremen => vec![Some(game.map.sietch_tabr().territory()), Some(game.map.false_wall_south()), Some(game.map.false_wall_west())],

            Phase::StormLosses => vec![Some(TakeLosses::losses_to_take(game).location.nre().territory())],

            Phase::HarvesterA => vec![Some(game.latest_spice_card_a.nre().location().nre().territory())],
            Phase::HarvesterB => vec![Some(game.latest_spice_card_b.nre().location().nre().territory())],

            Phase::TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip
            | Phase::TerrorTriggeredByBeneGesseritAccompaniesGuildShip
            | Phase::TerrorTriggeredByGuildShip
            | Phase::TerrorTriggeredByNonGuildShip
            | Phase::TerrorTriggeredByNonGuildMove
            | Phase::TerrorTriggeredByGuildMove
            | Phase::TerrorTriggeredByCaravan
            | Phase::TerrorTriggeredByRevival
            | Phase::AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip
            | Phase::AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip
            | Phase::AmbassadorTriggeredByGuildShip
            | Phase::AmbassadorTriggeredByNonGuildShip
            | Phase::AmbassadorTriggeredByNonGuildMove
            | Phase::AmbassadorTriggeredByGuildMove
            | Phase::AmbassadorTriggeredByCaravan
            | Phase::AmbassadorTriggeredByRevival
            | Phase::BeneGesseritIntrudedByCaravan
            | Phase::BeneGesseritIntrudedByRevival
            | Phase::BeneGesseritIntrudedByNonGuildMove
            | Phase::BeneGesseritIntrudedByNonGuildShip
            | Phase::BeneGesseritIntrudedByGuildMove
            | Phase::BeneGesseritIntrudedByGuildShip
            | Phase::BeneGesseritIntrudedByFremenRidingMonsterA
            | Phase::BeneGesseritIntrudedByFremenRidingMonsterB
            | Phase::BeneGesseritAccompaniesGuildShip
            | Phase::BeneGesseritAccompaniesNonGuildShip
                if game.last_shipment_or_movement.is_some() =>
            {
                let e = game.last_shipment_or_movement.as_ref().nre();
                vec![Some(e.to_location(game).nre().territory())]
            }

            Phase::BattlePhase
            | Phase::MeltingRock
            | Phase::CallTraitorOrPass
            | Phase::CaptureDecision
            | Phase::BattleConclusion
            | Phase::AvoidingAudit
            | Phase::Auditing
            | Phase::RevealingFacedancer
                if game.current_battle.is_some() =>
            {
                // `Territory!` does not throw: a null territory becomes a null element.
                vec![game.current_battle.as_ref().nre().territory(game)]
            }

            _ => vec![],
        }
    }

    pub fn determine_battle_status(game: &mut Game, me_: Option<PlayerIndex>) -> GameStatus {
        let me_ = me_.and_then(crate::compat::cs::CsNull::opt);
        if game.current_battle.is_none() {
            let a = express!("Please choose whom and where to battle.");
            let next = game.next_player_to_battle().nre();
            let w = express!(game.player(next).faction, " are deciding whom and where to battle...");
            let p = game.next_player_to_battle();
            return Self::status_message_message_player_game_event(a, w, p, None);
        }

        let g: &Game = game;
        let latest_battle_event = g.latest_event(GameEventType::BattleInitiated);
        let to_make_plan = Self::players_that_need_to_make_a_battle_plan(g);
        let battle = g.current_battle.clone().nre();
        let my_faction = me_.map(|me| g.player(me).faction);

        if Some(battle.aggressor(g)) == my_faction {
            return Self::status_message_message_list_player_game_event(
                express!("You are aggressor against ", battle.defender(g), " in ", battle.territory(g), "! Please confirm your Battle Plan."),
                express!("You are waiting for ", battle.defender(g), " to defend ", battle.territory(g), "..."),
                to_make_plan,
                latest_battle_event,
            );
        }
        if Some(battle.defender(g)) == my_faction {
            return Self::status_message_message_list_player_game_event(
                express!("You must defend against ", battle.aggressor(g), " in ", battle.territory(g), "! Please confirm your Battle Plan."),
                express!("You are waiting for ", battle.aggressor(g), " to attack ", battle.territory(g), "..."),
                to_make_plan,
                latest_battle_event,
            );
        }
        Self::status_message_list_player_game_event(
            express!(battle.defender(g), " are defending against ", battle.aggressor(g), " aggression in ", battle.territory(g), "..."),
            to_make_plan,
            latest_battle_event,
        )
    }

    pub fn players_that_have_not_acted_or_passed(game: &Game) -> Vec<Option<PlayerIndex>> {
        (0..game.players.len())
            .map(|i| game.player_at(i))
            .filter(|p| !game.has_acted_or_passed.contains(&game.player(*p).faction))
            .map(Some)
            .collect()
    }

    pub fn players_that_need_to_make_a_battle_plan(game: &Game) -> Vec<Option<PlayerIndex>> {
        let mut result = vec![];
        // `AggressivePlayer!` does not throw: a null player becomes a null element.
        if game.aggressor_plan.is_none() {
            result.push(game.current_battle.as_ref().nre().aggressive_player(game));
        }
        if game.defender_plan.is_none() {
            result.push(game.current_battle.as_ref().nre().defending_player(game));
        }
        result
    }

    pub fn players_that_need_to_call_traitor(game: &Game) -> Vec<Option<PlayerIndex>> {
        let mut result = vec![];
        if game.aggressor_traitor_action.is_none() {
            result.push(game.current_battle.as_ref().nre().aggressive_player(game));
        }
        if game.defender_traitor_action.is_none() {
            result.push(game.current_battle.as_ref().nre().defending_player(game));
        }
        result
    }

    pub fn status(message: &str, timed_event: Option<GameEvent>) -> GameStatus {
        Self::new_message_game_event(express!(message), timed_event)
    }

    pub fn status_message_game_event(message: Message, timed_event: Option<GameEvent>) -> GameStatus {
        Self::new_message_game_event(message, timed_event)
    }

    pub fn status_message_list_player_game_event(message: Message, waiting_for_players: Vec<Option<PlayerIndex>>, timed_event: Option<GameEvent>) -> GameStatus {
        Self::new_message_message_list_player_game_event(message.clone(), message, waiting_for_players, timed_event)
    }

    pub fn status_string_string_game_event(message_awaited: &str, message_when_waiting: &str, timed_event: Option<GameEvent>) -> GameStatus {
        Self::new_message_message_game_event(express!(message_awaited), express!(message_when_waiting), timed_event)
    }

    pub fn status_message_message_game_event(message_awaited: Message, message_when_waiting: Message, timed_event: Option<GameEvent>) -> GameStatus {
        Self::new_message_message_game_event(message_awaited, message_when_waiting, timed_event)
    }

    pub fn status_string_string_list_player_game_event(message_when_awaited: &str, message_when_waiting: &str, waiting_for_players: Vec<Option<PlayerIndex>>, timed_event: Option<GameEvent>) -> GameStatus {
        Self::new_message_message_list_player_game_event(express!(message_when_awaited), express!(message_when_waiting), waiting_for_players, timed_event)
    }

    pub fn status_message_message_list_player_game_event(message_when_awaited: Message, message_when_waiting: Message, waiting_for_players: Vec<Option<PlayerIndex>>, timed_event: Option<GameEvent>) -> GameStatus {
        Self::new_message_message_list_player_game_event(message_when_awaited, message_when_waiting, waiting_for_players, timed_event)
    }

    pub fn status_message_message_list_sequence_element_game_event(message_when_awaited: Message, message_when_waiting: Message, waiting_in_sequence: Vec<SequenceElement>, timed_event: Option<GameEvent>) -> GameStatus {
        Self::new_message_message_list_sequence_element_game_event(message_when_awaited, message_when_waiting, waiting_in_sequence, timed_event)
    }

    pub fn status_game_string_string_list_faction_game_event(game: &Game, message_when_awaited: &str, message_when_waiting: &str, waiting_for_factions: &[Faction], timed_event: Option<GameEvent>) -> GameStatus {
        let a = express!(message_when_awaited);
        let w = express!(message_when_waiting);
        // `.OfType<Player>()` drops the nulls.
        let players: Vec<Option<PlayerIndex>> = waiting_for_factions.iter().filter_map(|f| game.get_player(Some(*f))).map(Some).collect();
        Self::new_message_message_list_player_game_event(a, w, players, timed_event)
    }

    pub fn status_string_string_player_game_event(message_when_awaited: &str, message_when_waiting: &str, waiting_for_player: Option<PlayerIndex>, timed_event: Option<GameEvent>) -> GameStatus {
        let waiting_for_player = waiting_for_player.and_then(crate::compat::cs::CsNull::opt);
        Self::new(express!(message_when_awaited), express!(message_when_waiting), waiting_for_player, timed_event)
    }

    pub fn status_message_message_player_game_event(message_when_awaited: Message, message_when_waiting: Message, waiting_for_player: Option<PlayerIndex>, timed_event: Option<GameEvent>) -> GameStatus {
        let waiting_for_player = waiting_for_player.and_then(crate::compat::cs::CsNull::opt);
        Self::new(message_when_awaited, message_when_waiting, waiting_for_player, timed_event)
    }

    pub fn status_game_string_string_faction_game_event(game: &Game, message_when_awaited: &str, message_when_waiting: &str, waiting_for_faction: Faction, timed_event: Option<GameEvent>) -> GameStatus {
        let a = express!(message_when_awaited);
        let w = express!(message_when_waiting);
        Self::new(a, w, game.get_player(Some(waiting_for_faction)), timed_event)
    }

    pub fn status_game_message_message_faction_game_event(game: &Game, message_when_awaited: Message, message_when_waiting: Message, waiting_for_faction: Option<Faction>, timed_event: Option<GameEvent>) -> GameStatus {
        Self::new(message_when_awaited, message_when_waiting, game.get_player(waiting_for_faction), timed_event)
    }

    pub fn determine_flash(g: &Game, my_faction: Faction, is_player: bool) -> Vec<FlashInfo> {
        let mut result: Vec<FlashInfo> = vec![];

        if g.current_phase == Phase::GameEnded {
            for p in g.winners.iter().copied() {
                let f = g.player(p).faction;
                Self::flash(&mut result, express!(f, " win!"), f);
            }
            return result;
        }

        // C# parity: BattleWinner is a Faction?, so a null winner passes this check and flashes "null win!".
        if g.current_phase == Phase::BattleConclusion && g.battle_winner != Some(Faction::None) {
            Self::flash(&mut result, express!(g.battle_winner, " win!"), g.battle_winner);
            return result;
        }

        let latest_event = g.history.last().cloned();

        if let Some(latest) = &latest_event {
            match latest {
                // Show treachery card played
                GameEvent::RaiseDeadPlayed(_) => Self::flash_i_list_flash_info_game_event_treachery_card_type(g, &mut result, latest, TreacheryCardType::RaiseDead),
                GameEvent::MetheorPlayed(_) => Self::flash_i_list_flash_info_game_event_treachery_card_type(g, &mut result, latest, TreacheryCardType::Metheor),
                GameEvent::StormSpellPlayed(_) => Self::flash_i_list_flash_info_game_event_treachery_card_type(g, &mut result, latest, TreacheryCardType::StormSpell),
                GameEvent::ClairVoyancePlayed(_) => Self::flash_i_list_flash_info_game_event_treachery_card_type(g, &mut result, latest, TreacheryCardType::Clairvoyance),
                GameEvent::AmalPlayed(_) => Self::flash_i_list_flash_info_game_event_treachery_card_type(g, &mut result, latest, TreacheryCardType::Amal),
                GameEvent::HarvesterPlayed(_) => Self::flash_i_list_flash_info_game_event_treachery_card_type(g, &mut result, latest, TreacheryCardType::Harvester),
                GameEvent::ThumperPlayed(_) => Self::flash_i_list_flash_info_game_event_treachery_card_type(g, &mut result, latest, TreacheryCardType::Thumper),
                GameEvent::ResidualPlayed(_) => Self::flash_i_list_flash_info_game_event_treachery_card_type(g, &mut result, latest, TreacheryCardType::Residual),
                GameEvent::FlightUsed(_) => Self::flash_i_list_flash_info_game_event_treachery_card_type(g, &mut result, latest, TreacheryCardType::Flight),
                GameEvent::DistransUsed(_) => Self::flash_i_list_flash_info_game_event_treachery_card_type(g, &mut result, latest, TreacheryCardType::Distrans),
                GameEvent::Karma(_)
                | GameEvent::KarmaFreeRevival(_)
                | GameEvent::KarmaHandSwapInitiated(_)
                | GameEvent::KarmaHmsMovement(_)
                | GameEvent::KarmaMonster(_)
                | GameEvent::KarmaPrescience(_)
                | GameEvent::KarmaRevivalPrevention(_)
                | GameEvent::KarmaShipmentPrevention(_)
                | GameEvent::KarmaRicheseBuy(_)
                | GameEvent::KarmaChoamDiscard(_) => Self::flash_i_list_flash_info_game_event_treachery_card_type(g, &mut result, latest, TreacheryCardType::Karma),
                GameEvent::PortableAntidoteUsed(_) => Self::flash_i_list_flash_info_game_event_treachery_card_type(g, &mut result, latest, TreacheryCardType::PortableAntidote),
                GameEvent::RockWasMelted(_) => Self::flash_i_list_flash_info_game_event_treachery_card_type(g, &mut result, latest, TreacheryCardType::Rockmelter),
                GameEvent::DiscardedTaken(_) => Self::flash_i_list_flash_info_game_event_treachery_card_type(g, &mut result, latest, TreacheryCardType::TakeDiscarded),
                GameEvent::DiscardedSearched(_) => Self::flash_i_list_flash_info_game_event_treachery_card_type(g, &mut result, latest, TreacheryCardType::SearchDiscarded),
                GameEvent::JuicePlayed(_) => Self::flash_i_list_flash_info_game_event_treachery_card_type(g, &mut result, latest, TreacheryCardType::Juice),

                // Show Leader skill used
                GameEvent::Retreat(_) | GameEvent::Diplomacy(_) => Self::flash_i_list_flash_info_game_event_leader_skill(g, &mut result, latest, LeaderSkill::Diplomat),
                GameEvent::Bureaucracy(e) if !e.passed => Self::flash_i_list_flash_info_game_event_leader_skill(g, &mut result, latest, LeaderSkill::Bureaucrat),
                GameEvent::Planetology(_) => Self::flash_i_list_flash_info_game_event_leader_skill(g, &mut result, latest, LeaderSkill::Planetologist),
                GameEvent::BattleConcluded(_) if g.traitors_decipherer_can_look_at.len() > 0 => {
                    Self::flash_i_list_flash_info_game_event_leader_skill(g, &mut result, latest, LeaderSkill::Decipherer)
                }
                GameEvent::Thought(_) => Self::flash_i_list_flash_info_game_event_leader_skill(g, &mut result, latest, LeaderSkill::Thinker),

                // Show Event description
                GameEvent::RicheseSpecifiesAuction(_)
                | GameEvent::BeneGesseritBattleAnnouncement(_)
                | GameEvent::Voice(_)
                | GameEvent::Prescience(_)
                | GameEvent::IxianRemovedCardFromAuction(_)
                | GameEvent::ChoamDiscarded(_)
                | GameEvent::CardTraded(_)
                | GameEvent::ChoamEconomics(_)
                | GameEvent::SwitchedSkilledLeader(_) => Self::flash_i_list_flash_info_game_event(g, &mut result, latest_event.as_ref()),
                GameEvent::RicheseAnnouncesBlackMarket(e) if !e.passed => Self::flash_i_list_flash_info_game_event(g, &mut result, latest_event.as_ref()),
                GameEvent::IxianSwappedCardOnBid(e) if !e.passed => Self::flash_i_list_flash_info_game_event(g, &mut result, latest_event.as_ref()),
                GameEvent::ReplacedCardWon(e) if !e.passed => Self::flash_i_list_flash_info_game_event(g, &mut result, latest_event.as_ref()),
                GameEvent::FaceDancerReplaced(e) if !e.passed => Self::flash_i_list_flash_info_game_event(g, &mut result, latest_event.as_ref()),

                // Show Faction
                GameEvent::EstablishPlayers(_) if g.current_phase != Phase::SelectingFactions && is_player => Self::flash_i_list_flash_info_faction(&mut result, my_faction),
                GameEvent::FactionTradeOffered(fto) if (fto.initiator() == my_faction || fto.target == my_faction) && g.current_trade_offers.iter().all(|t| t.initiator() != my_faction) => {
                    Self::flash_i_list_flash_info_faction(&mut result, my_faction)
                }

                // Show Nexus card played
                GameEvent::NexusPlayed(np) => Self::flash(&mut result, express!(np.initiator(), " play a Nexus card"), np.faction.to_nexus()),
                GameEvent::Revival(nexus_revival) if nexus_revival.uses_emperor_secret_ally => {
                    Self::flash(&mut result, express!(nexus_revival.initiator(), " play a Nexus card"), Faction::Emperor.to_nexus())
                }

                _ => {}
            }
        }

        let nr_of_spice_blows = g.recent_milestones.iter().filter(|m| **m == Milestone::Resource).count();
        let mut resource_card_already_added = false;

        for m in g.recent_milestones.iter().copied() {
            match m {
                Milestone::TreacheryCalled | Milestone::FaceDanced => {
                    Self::flash_i_list_flash_info_game_event(g, &mut result, latest_event.as_ref());
                }
                Milestone::BabyMonster => {
                    let c = Map::get_resource_cards_in_play(g).into_iter().find(|c| c.is_sand_trout());
                    Self::flash(&mut result, express!(Concept::BabyMonster, " detected!"), c);
                }
                Milestone::Karma if matches!(latest_event, Some(GameEvent::Bid(_))) => {
                    if g.treachery_discard_pile.as_ref().nre().top().is_some() {
                        let top = g.treachery_discard_pile.as_ref().nre().top();
                        Self::flash(&mut result, express!("Card was won using ", TreacheryCardType::Karma), top);
                    }
                }
                Milestone::Monster => {
                    let c = Map::get_resource_cards_in_play(g).into_iter().find(|c| c.is_shai_hulud());
                    Self::flash(&mut result, express!(Concept::Monster, " detected!"), c);
                }
                Milestone::GreatMonster => {
                    let c = Map::get_resource_cards_in_play(g).into_iter().find(|c| c.is_great_maker());
                    Self::flash(&mut result, express!(Concept::GreatMonster, " detected!"), c);
                }
                Milestone::Resource => {
                    let card_to_show: Option<ResourceCard>;

                    if nr_of_spice_blows == 2 {
                        if !resource_card_already_added {
                            card_to_show = g.latest_spice_card_a;
                            resource_card_already_added = true;
                        } else {
                            card_to_show = g.latest_spice_card_b;
                        }
                    } else if g.applicable(Rule::IncreasedResourceFlow) && matches!(g.current_phase, Phase::AllianceB | Phase::HarvesterB | Phase::BlowReport) {
                        card_to_show = g.latest_spice_card_b;
                    } else {
                        card_to_show = g.latest_spice_card_a;
                    }

                    if let Some(card) = card_to_show {
                        Self::flash(&mut result, express!(Concept::Resource, " in ", card), card);
                    }
                }
                _ => {}
            }
        }

        result
    }

    pub fn flash<T: Into<ExpressionElement>>(flashes: &mut Vec<FlashInfo>, message: Message, to_show: T) {
        flashes.push(FlashInfo { message, to_show: to_show.into() });
    }

    /// C# reads `e.Game`: after an undo, another game than the one the status is for.
    pub fn flash_i_list_flash_info_game_event(game: &Game, flashes: &mut Vec<FlashInfo>, e: Option<&GameEvent>) {
        let Some(e) = e else { return };
        let own = game.enter_own_game(e.base().game.get()); // e.Game
        let game: &Game = &own;

        match e {
            GameEvent::TreacheryCalled(t) => {
                let victim = game.current_battle.as_ref().nre().opponent_of_faction(game, t.initiator());
                let victim_plan = game.current_battle.as_ref().nre().plan_of(game, victim);
                let msg = express!(victim_plan.as_ref().and_then(|p| p.hero()), " is a ", t.initiator(), " traitor!");
                let hero = victim_plan.as_ref().nre().hero();
                Self::flash(flashes, msg, hero);
            }

            GameEvent::FaceDanced(_) => {
                let dancer = game.winner_hero();
                Self::flash(flashes, express!(dancer, " is a ", e.initiator(), " facedancer!"), dancer);
            }

            GameEvent::SwitchedSkilledLeader(_) => {
                let leader = SwitchedSkilledLeader::switchable_leader(game, e.player());
                let hero = leader.map(Hero::Leader);
                let msg = express!(
                    e.initiator(),
                    " place ",
                    game.skill(hero),
                    " ",
                    leader,
                    if game.is_in_front_of_shield(hero) { " in front of" } else { " behind" },
                    " their shield"
                );
                Self::flash(flashes, msg, leader);
            }

            _ => {
                let msg = e.get_message(game);
                Self::flash(flashes, msg, e.initiator());
            }
        }
    }

    pub fn flash_i_list_flash_info_game_event_treachery_card_type(game: &Game, flashes: &mut Vec<FlashInfo>, e: &GameEvent, t: TreacheryCardType) {
        let own = game.enter_own_game(e.base().game.get()); // e.Game
        let game: &Game = &own;
        let pile = game.treachery_discard_pile.as_ref().nre();
        if pile.top().nre().type_() == t || (t == TreacheryCardType::Karma && pile.top().nre().type_() == TreacheryCardType::Useless) {
            let msg = e.get_message(game);
            Self::flash(flashes, msg, pile.top());
        } else {
            let msg = e.get_message(game);
            let card = TreacheryCardManager::get_cards_in_and_outside_play().iter().copied().cs_first_where(|card| card.type_() == t);
            Self::flash(flashes, msg, card);
        }
    }

    pub fn flash_i_list_flash_info_game_event_leader_skill(game: &Game, flashes: &mut Vec<FlashInfo>, e: &GameEvent, s: LeaderSkill) {
        let own = game.enter_own_game(e.base().game.get()); // e.Game (through GetMessage)
        let game: &Game = &own;
        let msg = if s == LeaderSkill::Decipherer { express!(e.initiator(), " use their ", s, " skill") } else { e.get_message(game) };
        Self::flash(flashes, msg, s)
    }

    pub fn flash_i_list_flash_info_faction(flashes: &mut Vec<FlashInfo>, f: Faction) {
        Self::flash(flashes, express!("You play ", f), f)
    }

    /// The calls above use `express!` directly, which is the same `Message.Express(elements)`.
    pub fn express(elements: Vec<ExpressionElement>) -> Message {
        Message::express(elements)
    }
}

/// C# `default(FlashInfo)` (null `Message`) is never created; every instance comes from
/// `Flash<T>`'s object initializer, which sets both members. So there is no `new()`.
#[derive(Clone, Debug, Canon)]
pub struct FlashInfo {
    pub to_show: ExpressionElement,
    pub message: Message,
}
