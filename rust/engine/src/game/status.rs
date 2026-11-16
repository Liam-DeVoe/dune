use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug)]
pub struct GameStatus {
    /// `Option` elements: `[game.CurrentBattle!.Territory!]` can add a null (the `!` does not throw).
    pub highlighted_territories: Vec<Option<Territory>>,
    pub waiting_in_sequence: Vec<SequenceElement>,
    /// `Option` elements: `PlayersThatNeedToMakeABattlePlan`/`PlayersThatNeedToCallTraitor` add
    /// `AggressivePlayer!`/`DefendingPlayer!`, which can be null (the `!` does not throw).
    pub waiting_for_players: Vec<Option<PlayerId>>,
    pub flash_info: Vec<FlashInfo>,
    pub timed_event: Option<HistoryEntry>,
    pub waiting_for_host: bool,
    pub description_when_awaited: Message,
    pub description_when_waiting: Message,
}

impl GameStatus {
    pub fn for_player(message_when_awaited: Message, message_when_waiting: Message, waiting_for_player: Option<PlayerId>, timed_event: Option<HistoryEntry>) -> Self {
        let list = if waiting_for_player.is_some() { vec![waiting_for_player] } else { vec![] };
        Self::for_players(message_when_awaited, message_when_waiting, list, timed_event)
    }

    pub fn for_players(message_when_awaited: Message, message_when_waiting: Message, waiting_for_players: Vec<Option<PlayerId>>, timed_event: Option<HistoryEntry>) -> Self {
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

    pub fn for_sequence(message_when_awaited: Message, message_when_waiting: Message, waiting_in_sequence: Vec<SequenceElement>, timed_event: Option<HistoryEntry>) -> Self {
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

    pub fn for_host(message_when_awaited: Message, message_when_waiting: Message, timed_event: Option<HistoryEntry>) -> Self {
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

    pub fn waiting_for_me(&self, player: PlayerId, is_host: bool) -> bool {
        (self.waiting_for_host && is_host)
            || self.waiting_for_players.contains(&Some(player))
            || self.waiting_in_sequence.iter().any(|se| se.player == Some(player) && se.has_turn)
    }

    pub fn waiting_for_others(&self, player: PlayerId, is_host: bool) -> bool {
        !self.waiting_for_me(player, is_host)
    }

    pub fn get_message(&self, player: PlayerId, is_host: bool) -> Message {
        if self.waiting_for_me(player, is_host) { self.description_when_awaited.clone() } else { self.description_when_waiting.clone() }
    }

    /// The arms below are checked in order; each guard is evaluated
    /// only when its pattern matches, and again for each later arm with the same phase.
    pub fn determine_status(game: &Game, me_: Option<PlayerId>, is_player: bool) -> GameStatus {
        let g = game;
        let phase = game.phase();
        let result = match phase {
            /* Phase Beginnings */
            Phase::BeginningOfStorm | Phase::Thumper | Phase::BeginningOfCharity | Phase::BeginningOfBidding | Phase::BeginningOfResurrection | Phase::BeginningOfCollection => Self::host_status(
                express!("You may now continue with the ", game.turn.current_main_phase, " phase..."),
                express!("Waiting for the host to continue the ", game.turn.current_main_phase, " phase..."),
                None,
            ),

            /* Phase Endings */
            Phase::StormReport | Phase::BlowReport | Phase::CharityReport | Phase::BiddingReport | Phase::ResurrectionReport | Phase::ShipmentAndMoveConcluded | Phase::CollectionReport | Phase::TurnConcluded => {
                Self::status(express!(game.turn.current_main_phase, " phase ended."), None)
            }

            Phase::AssigningInitialSkills | Phase::AssigningSkill => {
                let players: Vec<Option<PlayerId>> = (0..g.players.len()).filter(|i| g.players[*i].skills_to_choose_from.len() > 0).map(|i| Some(g.player_at(i))).collect();
                Self::players_status("You may now assign a skill to a leader.", "Waiting for factions to assign leader skills...", players, None)
            }

            // DiscardingTraitor, the one phase C# had no status for, and the guarded phases above.
            // Each phase state has its own function, in the file of its topic.
            _ => Self::setup_status(game, me_, phase)
                .or_else(|| Self::storm_status(game, me_, phase))
                .or_else(|| Self::blow_status(game, me_, phase))
                .or_else(|| Self::charity_status(game, me_, phase))
                .or_else(|| Self::bidding_status(game, me_, phase))
                .or_else(|| Self::revival_status(game, me_, phase))
                .or_else(|| Self::ship_move_status(game, me_, phase))
                .or_else(|| Self::battle_status(game, me_, phase))
                .or_else(|| Self::collection_status(game, me_, phase))
                .or_else(|| Self::contemplate_status(game, me_, phase))
                .or_else(|| Self::interrupt_status(game, me_, phase))
                .unwrap_or_else(|| Self::status(express!(format!("Unknown phase: {}", game.phase())), None)),
        };

        let mut result = result;
        let my_faction = match me_ {
            Some(me) => game.player(me).faction,
            None => None,
        };
        result.flash_info = Self::determine_flash(game, my_faction, is_player);
        result.highlighted_territories = Self::determine_highlights(game, my_faction);

        result
    }

    /// The `Status(Message, Message, game.ShipmentAndMoveSequence.GetPlayersInSequence().ToList(),
    /// game.FindMostRecentEvent(typeof(EndPhase), typeof(GuildDelay), typeof(Move)))` tail shared by the
    /// Ship & Move arms. Arguments are evaluated in the order written.
    pub(crate) fn status_ship_and_move(g: &Game, a: Message, w: Message) -> GameStatus {
        let seq = g.shipment_and_move_sequence().expect("shipment and move sequence").get_players_in_sequence(g).collect();
        let latest = g.find_most_recent_event(vec![EventType::EndPhase, EventType::GuildDelay, EventType::Move]);
        Self::sequence_status(a, w, seq, latest)
    }

    /// Shared by two bidding arms.
    pub(crate) fn players_with_room_and_no_bid(g: &Game) -> Vec<Option<PlayerId>> {
        (0..g.players.len())
            .map(|i| g.player_at(i))
            .filter(|p| g.player(*p).has_room_for_cards(g) && !g.bids().contains_key_opt(g.player(*p).faction))
            .map(Some)
            .collect()
    }

    pub fn faction_or_occupier(g: &Game, f: Faction, w: World) -> Option<Faction> {
        let occupier = g.occupier_of(w);
        if let Some(occupier) = occupier {
            return g.player(occupier).faction;
        }
        Some(f)
    }

    pub fn player_and_ally(g: &Game, f: Faction) -> Vec<Option<PlayerId>> {
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
        // C# parity: the ally is read even when the condition is false, so a missing player panics.
        express_part_if!(cond; g.player(player.expect("player")).ally)
    }

    pub fn determine_highlights(game: &Game, my_faction: Option<Faction>) -> Vec<Option<Territory>> {
        match game.phase() {
            Phase::FremenSettingUp if my_faction == Some(Faction::Fremen) => vec![Some(game.board.map.sietch_tabr().territory()), Some(game.board.map.false_wall_south()), Some(game.board.map.false_wall_west())],

            Phase::StormLosses => vec![Some(TakeLosses::losses_to_take(game).location.expect("location").territory())],

            Phase::HarvesterA => vec![Some(game.blow.latest_spice_card_a.expect("latest spice card a").location().expect("location").territory())],
            Phase::HarvesterB => vec![Some(game.blow.latest_spice_card_b.expect("latest spice card b").location().expect("location").territory())],

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
                if game.turn.last_shipment_or_movement.is_some() =>
            {
                let e = game.turn.last_shipment_or_movement.as_ref().expect("last shipment or movement");
                vec![Some(e.to_location(game).expect("to location").territory())]
            }

            Phase::BattlePhase
            | Phase::MeltingRock
            | Phase::CallTraitorOrPass
            | Phase::CaptureDecision
            | Phase::BattleConclusion
            | Phase::AvoidingAudit
            | Phase::Auditing
            | Phase::RevealingFacedancer
                if game.current_battle().is_some() =>
            {
                // `Territory!` does not throw: a null territory becomes a null element.
                vec![game.current_battle().expect("current battle").territory]
            }

            _ => vec![],
        }
    }

    pub fn determine_battle_status(game: &Game, me_: Option<PlayerId>) -> GameStatus {
        let g = game;
        if game.current_battle().is_none() {
            let a = express!("Please choose whom and where to battle.");
            let next = game.next_player_to_battle().expect("next player to battle");
            let w = express!(game.player(next).faction, " are deciding whom and where to battle...");
            let p = game.next_player_to_battle();
            return Self::player_status(a, w, p, None);
        }

        let latest_battle_event = g.latest_event(EventType::BattleInitiated);
        let mut to_make_plan = Self::players_that_need_to_make_a_battle_plan(g);
        let battle = g.current_battle().cloned().expect("current battle");
        let my_faction = me_.map(|me| g.player(me).faction);

        // Prescience waits for Voice, so whoever may use or decline it is waited for too.
        let voice_deciders: Vec<PlayerId> = g.player_indices().filter(|&p| VoiceDeclined::may_decline(g, p)).collect();
        for &p in &voice_deciders {
            if !to_make_plan.contains(&Some(p)) {
                to_make_plan.push(Some(p));
            }
        }
        if me_.is_some_and(|me| voice_deciders.contains(&me) && !battle.is_aggressor_or_defender(g, me)) {
            let message = express!(battle.defender(g), " are defending against ", battle.aggressor(g), " aggression in ", battle.territory, "...");
            return Self::players_status(express!("Please decide whether to use Voice. Prescience waits for it."), message, to_make_plan, latest_battle_event);
        }

        if battle.aggressor(g) == my_faction.flatten() {
            return Self::players_status(
                express!("You are aggressor against ", battle.defender(g), " in ", battle.territory, "! Please confirm your Battle Plan."),
                express!("You are waiting for ", battle.defender(g), " to defend ", battle.territory, "..."),
                to_make_plan,
                latest_battle_event,
            );
        }
        if battle.defender(g) == my_faction.flatten() {
            return Self::players_status(
                express!("You must defend against ", battle.aggressor(g), " in ", battle.territory, "! Please confirm your Battle Plan."),
                express!("You are waiting for ", battle.aggressor(g), " to attack ", battle.territory, "..."),
                to_make_plan,
                latest_battle_event,
            );
        }
        let message = express!(battle.defender(g), " are defending against ", battle.aggressor(g), " aggression in ", battle.territory, "...");
        Self::players_status(message.clone(), message, to_make_plan, latest_battle_event)
    }

    pub fn players_that_have_not_acted_or_passed(game: &Game) -> Vec<Option<PlayerId>> {
        (0..game.players.len())
            .map(|i| game.player_at(i))
            .filter(|p| !game.turn.has_acted_or_passed.contains_opt(game.player(*p).faction))
            .map(Some)
            .collect()
    }

    pub fn players_that_need_to_make_a_battle_plan(game: &Game) -> Vec<Option<PlayerId>> {
        let mut result = vec![];
        // `AggressivePlayer!` does not throw: a null player becomes a null element.
        if game.aggressor_plan().is_none() {
            result.push(game.current_battle().expect("current battle").aggressive_player(game));
        }
        if game.defender_plan().is_none() {
            result.push(game.current_battle().expect("current battle").defending_player(game));
        }
        result
    }

    pub fn players_that_need_to_call_traitor(game: &Game) -> Vec<Option<PlayerId>> {
        let mut result = vec![];
        if game.fight().map_or(true, |c| c.aggressor_traitor_action.is_none()) {
            result.push(game.current_battle().expect("current battle").aggressive_player(game));
        }
        if game.fight().map_or(true, |c| c.defender_traitor_action.is_none()) {
            result.push(game.current_battle().expect("current battle").defending_player(game));
        }
        result
    }

    /// Waiting for the host, with the same text for everyone.
    pub fn status(message: impl Into<Message>, timed_event: Option<HistoryEntry>) -> GameStatus {
        let message = message.into();
        Self::for_host(message.clone(), message, timed_event)
    }

    pub fn host_status(message_when_awaited: impl Into<Message>, message_when_waiting: impl Into<Message>, timed_event: Option<HistoryEntry>) -> GameStatus {
        let a = message_when_awaited.into();
        let w = message_when_waiting.into();
        Self::for_host(a, w, timed_event)
    }

    pub fn players_status(message_when_awaited: impl Into<Message>, message_when_waiting: impl Into<Message>, waiting_for_players: Vec<Option<PlayerId>>, timed_event: Option<HistoryEntry>) -> GameStatus {
        let a = message_when_awaited.into();
        let w = message_when_waiting.into();
        Self::for_players(a, w, waiting_for_players, timed_event)
    }

    pub fn sequence_status(message_when_awaited: Message, message_when_waiting: Message, waiting_in_sequence: Vec<SequenceElement>, timed_event: Option<HistoryEntry>) -> GameStatus {
        Self::for_sequence(message_when_awaited, message_when_waiting, waiting_in_sequence, timed_event)
    }

    /// Waiting for the players of these factions. Factions nobody plays are left out.
    pub fn factions_status(game: &Game, message_when_awaited: impl Into<Message>, message_when_waiting: impl Into<Message>, waiting_for_factions: &[Faction], timed_event: Option<HistoryEntry>) -> GameStatus {
        let a = message_when_awaited.into();
        let w = message_when_waiting.into();
        let players: Vec<Option<PlayerId>> = waiting_for_factions.iter().filter_map(|f| game.get_player(Some(*f))).map(Some).collect();
        Self::for_players(a, w, players, timed_event)
    }

    pub fn player_status(message_when_awaited: impl Into<Message>, message_when_waiting: impl Into<Message>, waiting_for_player: Option<PlayerId>, timed_event: Option<HistoryEntry>) -> GameStatus {
        let a = message_when_awaited.into();
        let w = message_when_waiting.into();
        Self::for_player(a, w, waiting_for_player, timed_event)
    }

    pub fn faction_status(game: &Game, message_when_awaited: impl Into<Message>, message_when_waiting: impl Into<Message>, waiting_for_faction: Option<Faction>, timed_event: Option<HistoryEntry>) -> GameStatus {
        let a = message_when_awaited.into();
        let w = message_when_waiting.into();
        Self::for_player(a, w, game.get_player(waiting_for_faction), timed_event)
    }

    pub fn determine_flash(g: &Game, my_faction: Option<Faction>, is_player: bool) -> Vec<FlashInfo> {
        let mut result: Vec<FlashInfo> = vec![];

        if g.phase() == Phase::GameEnded {
            for p in g.winners().iter().copied() {
                let f = g.player(p).faction;
                Self::flash(&mut result, express!(f, " win!"), f);
            }
            return result;
        }

        // C# parity: BattleWinner is a Faction?, so a null winner passes this check and flashes "null win!".
        if let Some(winner) = g.battle_winner().filter(|_| g.phase() == Phase::BattleConclusion) {
            Self::flash(&mut result, express!(Some(winner), " win!"), Some(winner));
            return result;
        }

        let latest_event = g.history.entries.last().cloned();

        if let Some(latest) = &latest_event {
            match &latest.event {
                // Show treachery card played
                Event::RaiseDeadPlayed(_) => Self::flash_card_type_played(g, &mut result, latest, TreacheryCardType::RaiseDead),
                Event::MetheorPlayed(_) => Self::flash_card_type_played(g, &mut result, latest, TreacheryCardType::Metheor),
                Event::StormSpellPlayed(_) => Self::flash_card_type_played(g, &mut result, latest, TreacheryCardType::StormSpell),
                Event::ClairVoyancePlayed(_) => Self::flash_card_type_played(g, &mut result, latest, TreacheryCardType::Clairvoyance),
                Event::AmalPlayed(_) => Self::flash_card_type_played(g, &mut result, latest, TreacheryCardType::Amal),
                Event::HarvesterPlayed(_) => Self::flash_card_type_played(g, &mut result, latest, TreacheryCardType::Harvester),
                Event::ThumperPlayed(_) => Self::flash_card_type_played(g, &mut result, latest, TreacheryCardType::Thumper),
                Event::ResidualPlayed(_) => Self::flash_card_type_played(g, &mut result, latest, TreacheryCardType::Residual),
                Event::FlightUsed(_) => Self::flash_card_type_played(g, &mut result, latest, TreacheryCardType::Flight),
                Event::DistransUsed(_) => Self::flash_card_type_played(g, &mut result, latest, TreacheryCardType::Distrans),
                Event::Karma(_)
                | Event::KarmaFreeRevival(_)
                | Event::KarmaHandSwapInitiated(_)
                | Event::KarmaHmsMovement(_)
                | Event::KarmaMonster(_)
                | Event::KarmaPrescience(_)
                | Event::KarmaRevivalPrevention(_)
                | Event::KarmaShipmentPrevention(_)
                | Event::KarmaRicheseBuy(_)
                | Event::KarmaChoamDiscard(_) => Self::flash_card_type_played(g, &mut result, latest, TreacheryCardType::Karma),
                Event::PortableAntidoteUsed(_) => Self::flash_card_type_played(g, &mut result, latest, TreacheryCardType::PortableAntidote),
                Event::RockWasMelted(_) => Self::flash_card_type_played(g, &mut result, latest, TreacheryCardType::Rockmelter),
                Event::DiscardedTaken(_) => Self::flash_card_type_played(g, &mut result, latest, TreacheryCardType::TakeDiscarded),
                Event::DiscardedSearched(_) => Self::flash_card_type_played(g, &mut result, latest, TreacheryCardType::SearchDiscarded),
                Event::JuicePlayed(_) => Self::flash_card_type_played(g, &mut result, latest, TreacheryCardType::Juice),

                // Show Leader skill used
                Event::Retreat(_) | Event::Diplomacy(_) => Self::flash_skill_used(g, &mut result, latest, LeaderSkill::Diplomat),
                Event::Bureaucracy(e) if !e.passed => Self::flash_skill_used(g, &mut result, latest, LeaderSkill::Bureaucrat),
                Event::Planetology(_) => Self::flash_skill_used(g, &mut result, latest, LeaderSkill::Planetologist),
                Event::BattleConcluded(_) if g.fight().map_or(0, |c| c.traitors_decipherer_can_look_at.len()) > 0 => {
                    Self::flash_skill_used(g, &mut result, latest, LeaderSkill::Decipherer)
                }
                Event::Thought(_) => Self::flash_skill_used(g, &mut result, latest, LeaderSkill::Thinker),

                // Show Event description
                Event::RicheseSpecifiesAuction(_)
                | Event::BeneGesseritBattleAnnouncement(_)
                | Event::Voice(_)
                | Event::Prescience(_)
                | Event::IxianRemovedCardFromAuction(_)
                | Event::ChoamDiscarded(_)
                | Event::CardTraded(_)
                | Event::ChoamEconomics(_)
                | Event::SwitchedSkilledLeader(_) => Self::flash_event(g, &mut result, latest_event.as_ref()),
                Event::RicheseAnnouncesBlackMarket(e) if !e.passed => Self::flash_event(g, &mut result, latest_event.as_ref()),
                Event::IxianSwappedCardOnBid(e) if !e.passed => Self::flash_event(g, &mut result, latest_event.as_ref()),
                Event::ReplacedCardWon(e) if !e.passed => Self::flash_event(g, &mut result, latest_event.as_ref()),
                Event::FaceDancerReplaced(e) if !e.passed => Self::flash_event(g, &mut result, latest_event.as_ref()),

                // Show Faction
                Event::EstablishPlayers(_) if g.phase() != Phase::SelectingFactions && is_player => Self::flash_faction_played(&mut result, my_faction),
                Event::FactionTradeOffered(fto) if (latest.by == my_faction || fto.target == my_faction) && g.setup.current_trade_offers.iter().all(|t| t.initiator != my_faction) => {
                    Self::flash_faction_played(&mut result, my_faction)
                }

                // Show Nexus card played
                Event::NexusPlayed(np) => Self::flash(&mut result, express!(latest.by, " play a Nexus card"), np.faction.map(Faction::to_nexus)),
                Event::Revival(nexus_revival) if nexus_revival.uses_emperor_secret_ally => {
                    Self::flash(&mut result, express!(latest.by, " play a Nexus card"), Some(Faction::Emperor.to_nexus()))
                }

                _ => {}
            }
        }

        let nr_of_spice_blows = g.turn.recent_milestones.iter().filter(|m| **m == Milestone::Resource).count();
        let mut resource_card_already_added = false;

        for m in g.turn.recent_milestones.iter().copied() {
            match m {
                Milestone::TreacheryCalled | Milestone::FaceDanced => {
                    Self::flash_event(g, &mut result, latest_event.as_ref());
                }
                Milestone::BabyMonster => {
                    let c = Map::get_resource_cards_in_play(g).into_iter().find(|c| c.is_sand_trout());
                    Self::flash(&mut result, express!(Concept::BabyMonster, " detected!"), c);
                }
                Milestone::Karma if matches!(latest_event, Some(HistoryEntry { event: Event::Bid(_), .. })) => {
                    if g.decks.treachery_discard_pile.as_ref().expect("treachery discard pile").top().is_some() {
                        let top = g.decks.treachery_discard_pile.as_ref().expect("treachery discard pile").top();
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
                            card_to_show = g.blow.latest_spice_card_a;
                            resource_card_already_added = true;
                        } else {
                            card_to_show = g.blow.latest_spice_card_b;
                        }
                    } else if g.applicable(Rule::IncreasedResourceFlow) && matches!(g.phase(), Phase::AllianceB | Phase::HarvesterB | Phase::BlowReport) {
                        card_to_show = g.blow.latest_spice_card_b;
                    } else {
                        card_to_show = g.blow.latest_spice_card_a;
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

    pub fn flash<T: Into<Part>>(flashes: &mut Vec<FlashInfo>, message: Message, to_show: T) {
        flashes.push(FlashInfo { message, to_show: to_show.into() });
    }

    pub fn flash_event(game: &Game, flashes: &mut Vec<FlashInfo>, e: Option<&HistoryEntry>) {
        let Some(e) = e else { return };

        match &e.event {
            Event::TreacheryCalled(_) => {
                let victim = game.current_battle().expect("current battle").opponent_of_faction(game, e.by);
                let victim_plan = game.current_battle().expect("current battle").plan_of(game, victim);
                let msg = express!(victim_plan.as_ref().and_then(|p| p.hero), " is a ", e.by, " traitor!");
                let hero = victim_plan.as_ref().expect("victim plan").hero;
                Self::flash(flashes, msg, hero);
            }

            Event::FaceDanced(_) => {
                let dancer = game.winner_hero();
                Self::flash(flashes, express!(dancer, " is a ", e.by, " facedancer!"), dancer);
            }

            Event::SwitchedSkilledLeader(_) => {
                let leader = SwitchedSkilledLeader::switchable_leader(game, e.by_in(game).player());
                let hero = leader.map(Hero::Leader);
                let msg = express!(
                    e.by,
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
                let msg = e.describe(game);
                Self::flash(flashes, msg, e.by);
            }
        }
    }

    pub fn flash_card_type_played(game: &Game, flashes: &mut Vec<FlashInfo>, e: &HistoryEntry, t: TreacheryCardType) {
        let pile = game.decks.treachery_discard_pile.as_ref().expect("treachery discard pile");
        if pile.top().expect("a discarded card").type_() == t || (t == TreacheryCardType::Karma && pile.top().expect("a discarded card").type_() == TreacheryCardType::Useless) {
            let msg = e.describe(game);
            Self::flash(flashes, msg, pile.top());
        } else {
            let msg = e.describe(game);
            let card = TreacheryCardManager::get_cards_in_and_outside_play().iter().copied().find(|card| card.type_() == t).expect("a card of this type");
            Self::flash(flashes, msg, card);
        }
    }

    pub fn flash_skill_used(game: &Game, flashes: &mut Vec<FlashInfo>, e: &HistoryEntry, s: LeaderSkill) {
        let msg = if s == LeaderSkill::Decipherer { express!(e.by, " use their ", s, " skill") } else { e.describe(game) };
        Self::flash(flashes, msg, s)
    }

    pub fn flash_faction_played(flashes: &mut Vec<FlashInfo>, f: Option<Faction>) {
        Self::flash(flashes, express!("You play ", f), f)
    }

    /// The calls above use `express!` directly, which is the same `Message.Express(elements)`.
    pub fn express(elements: Vec<Part>) -> Message {
        Message::express(elements)
    }
}

/// Every instance is created by `Flash<T>` with both members set, so there is no `new()`.
#[derive(Clone, Debug)]
pub struct FlashInfo {
    pub to_show: Part,
    pub message: Message,
}

