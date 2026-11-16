use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct NexusPlayed {
    pub faction: Option<Faction>,
    pub atreides_prescience_aspect: Option<PrescienceAspect>,
    pub tleilaxu_forces: u16,
    pub tleilaxu_special_forces: u16,
    pub tleilaxu_hero: Option<Hero>,
    pub tleilaxu_assign_skill: bool,
    pub choam_card: Option<TreacheryCard>,
    pub ecaz_territory: Option<Territory>,
    pub ecaz_faction: Option<Faction>,
    pub moritani_territory: Option<Territory>,
    pub tleilaxu_number_of_special_forces_in_location: u16,
    pub tleilaxu_location: Option<Location>,
}

impl Default for NexusPlayed {
    fn default() -> Self {
        NexusPlayed {
            faction: Default::default(),
            atreides_prescience_aspect: Default::default(),
            tleilaxu_forces: Default::default(),
            tleilaxu_special_forces: Default::default(),
            tleilaxu_hero: None,
            tleilaxu_assign_skill: false,
            choam_card: TreacheryCardManager::get(0),
            ecaz_territory: Territory::from_id(0),
            ecaz_faction: Default::default(),
            moritani_territory: Territory::from_id(0),
            tleilaxu_number_of_special_forces_in_location: Default::default(),
            tleilaxu_location: None,
        }
    }
}

macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(Part::from($e)),*] };
}

impl NexusPlayed {
    pub fn to(&self, _game: &Game) -> Option<Location> {
        self.tleilaxu_location
    }

    pub fn total_amount_of_forces_added_to_location(&self, _game: &Game) -> i32 {
        i32::from(self.tleilaxu_number_of_special_forces_in_location)
    }

    pub fn forces_added_to_location(&self, _game: &Game) -> i32 {
        0
    }

    pub fn special_forces_added_to_location(&self, _game: &Game) -> i32 {
        i32::from(self.tleilaxu_number_of_special_forces_in_location)
    }

    pub fn is_cunning(&self, initiator: Option<Faction>) -> bool {
        initiator == self.faction
    }

    pub fn is_secret_ally(&self, game: &Game) -> bool {
        !game.is_playing(self.faction)
    }

    pub fn is_betrayal(&self, game: &Game, initiator: Option<Faction>) -> bool {
        !(self.is_cunning(initiator) || self.is_secret_ally(game))
    }

    pub fn can_use_cunning(game: &Game, p: PlayerId) -> bool {
        let p = game.player(p);
        p.nexus.is_some() && p.nexus == p.faction
    }

    pub fn can_use_secret_ally(g: &Game, p: PlayerId) -> bool {
        let p = g.player(p);
        p.nexus != None && !g.is_playing(p.nexus)
    }

    pub fn can_use_betrayal(g: &Game, p: PlayerId) -> bool {
        let p = g.player(p);
        p.nexus.is_some() && p.nexus != p.faction && g.is_playing(p.nexus)
    }

    pub fn is_applicable(g: &Game, p: PlayerId) -> bool {
        if g.phase() == Phase::NexusCards || g.current_phase_cannot_be_interrupted() {
            return false;
        }

        let cunning = NexusPlayed::can_use_cunning(g, p);
        let secret_ally = NexusPlayed::can_use_secret_ally(g, p);
        let betrayal = NexusPlayed::can_use_betrayal(g, p);

        let game_is_in_battle = g.phase() == Phase::BattlePhase && g.current_battle().is_some();
        let is_currently_formulating_battle_plan = game_is_in_battle
            && g.current_battle().expect("current battle").is_aggressor_or_defender(g, p)
            && (g.defender_plan().is_none() || g.aggressor_plan().is_none());

        let battle = || g.current_battle().expect("current battle");

        match g.player(p).nexus {
            Some(Faction::Atreides) if betrayal => game_is_in_battle && battle().is_aggressor_or_defender_faction(g, Faction::Atreides),
            Some(Faction::Atreides) if cunning || secret_ally => is_currently_formulating_battle_plan,

            Some(Faction::Harkonnen) if betrayal => g.phase() == Phase::CancellingTraitor,
            Some(Faction::Harkonnen) if cunning => true,
            Some(Faction::Harkonnen) if secret_ally => g.turn.current_main_phase == MainPhase::Contemplate,

            Some(Faction::Fremen) if betrayal => {
                g.turn.current_main_phase == MainPhase::Blow || g.turn.current_main_phase == MainPhase::ShipmentAndMove
            }
            Some(Faction::Fremen) if secret_ally => {
                g.turn.current_main_phase == MainPhase::Blow || g.turn.current_main_phase == MainPhase::Resurrection
            }

            Some(Faction::Emperor) if betrayal => {
                g.turn.current_main_phase == MainPhase::Bidding
                    || (game_is_in_battle
                        && g.applicable(Rule::EmperorSpecialForces)
                        && battle().is_aggressor_or_defender_faction(g, Faction::Emperor))
            }
            Some(Faction::Emperor) if cunning => is_currently_formulating_battle_plan,

            Some(Faction::Guild) if betrayal => {
                g.turn.current_main_phase == MainPhase::ShipmentAndMove && g.has_recent_payment_for(EventType::Shipment)
            }
            Some(Faction::Guild) if cunning => g.phase() == Phase::GuildMove && !g.in_guild_cunning_shipment(),
            Some(Faction::Guild) if secret_ally => g.phase() == Phase::NonGuildShip,

            Some(Faction::BeneGesserit) if betrayal => game_is_in_battle && battle().is_involved_faction(g, Faction::BeneGesserit),
            Some(Faction::BeneGesserit) if cunning => g.turn.current_main_phase == MainPhase::ShipmentAndMove,

            Some(Faction::Ixian) if betrayal => g.turn.current_main_phase == MainPhase::Bidding && g.phase() < Phase::BiddingReport,
            Some(Faction::Ixian) if cunning => is_currently_formulating_battle_plan,

            Some(Faction::Tleilaxu) if betrayal => g.phase() == Phase::Facedancing,
            Some(Faction::Tleilaxu) if cunning => true,
            Some(Faction::Tleilaxu) if secret_ally => g.phase() == Phase::Resurrection,

            Some(Faction::Choam) if betrayal => true,
            Some(Faction::Choam) if secret_ally => {
                (g.turn.current_main_phase == MainPhase::Collection && !NexusPlayed::valid_choam_cards(g, p).next().is_none())
                    || (g.phase() == Phase::BattleConclusion
                        && g.current_battle().is_some()
                        && g.player(p).faction == g.battle_winner())
            }

            Some(Faction::Richese) if betrayal && g.turn.current_main_phase == MainPhase::Bidding => {
                g.bidding().is_some_and(|b| b.richese_bidding_just_finished || b.card_sold_on_black_market.is_some())
                    && (g.has_quite_recent_payment_to(Faction::Richese)
                        || (g.card_just_won().is_some()
                            && g.get_player(Some(Faction::Richese)).map(|w| g.player(w).has(g.card_just_won())) == Some(true)))
            }

            Some(Faction::Ecaz) if betrayal => {
                g.turn.current_main_phase < MainPhase::ShipmentAndMove && !NexusPlayed::valid_ecaz_territories(g).next().is_none()
            }
            Some(Faction::Ecaz) if cunning || secret_ally => true,

            Some(Faction::Moritani) if betrayal => !NexusPlayed::valid_moritani_territories(g).next().is_none(),

            _ => false,
        }
    }

    pub fn valid_tleilaxu_max_amount(g: &Game, p: PlayerId, special_forces: bool) -> i32 {
        let p = g.player(p);
        if special_forces {
            if p.faction == Some(Faction::Emperor) || p.faction == Some(Faction::Fremen) {
                if g.turn.revivals.factions_that_revived_special_forces_this_turn.contains_opt(p.faction) {
                    return 0;
                }
                return std::cmp::min(i32::from(p.special_forces_killed), 1);
            }

            return std::cmp::min(i32::from(p.special_forces_killed), 5);
        }

        std::cmp::min(i32::from(p.forces_killed), 5)
    }

    pub fn valid_tleilaxu_heroes(game: &Game, player: PlayerId) -> Vec<Hero> {
        RaiseDeadPlayed::valid_heroes(game, player)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_choam_cards(game: &Game, player: PlayerId) -> impl Iterator<Item = TreacheryCard> {
        game.player(player).treachery_cards.iter().copied().filter(move |c| c.type_() == TreacheryCardType::Useless)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_ecaz_territories(g: &Game) -> impl Iterator<Item = Territory> {
        use either::Either;
        use std::iter::empty;
        let ecaz = g.get_player(Some(Faction::Ecaz));

        if let Some(ecaz) = ecaz {
            if g.player(ecaz).has_ally() {
                return Either::Left(g.board.map.territories(false).filter(move |t| {
                    g.player(ecaz).any_forces_in_territory(Some(*t)) > 0
                        && g.player(g.player(ecaz).allied_player(g).expect("allied player")).any_forces_in_territory(Some(*t)) > 0
                }));
            }
        }

        Either::Right(empty())
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_ecaz_factions(g: &Game) -> impl Iterator<Item = Faction> {
        g.players
            .iter()
            .filter(move |p| p.faction != Some(Faction::Ecaz) && p.faction != Some(Faction::Tleilaxu))
            .filter_map(move |p| p.faction)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_moritani_territories(g: &Game) -> impl Iterator<Item = Territory> {
        crate::iter::unique(g.board.terror_on_planet.values().copied())
    }

    pub fn may_select_location_for_revived_forces(game: &Game, player: PlayerId, special_forces: i32) -> bool {
        game.player(player).is(Faction::Fremen)
            && special_forces >= 1
            && game.player(player).has_any_high_threshold(game)
            && !Revival::valid_revived_force_locations(game, player).next().is_none()
    }

    pub fn determine_tleilaxu_cost(&self) -> i32 {
        NexusPlayed::tleilaxu_cost_of_forces(i32::from(self.tleilaxu_forces), i32::from(self.tleilaxu_special_forces))
    }

    pub fn tleilaxu_cost_of_forces(forces: i32, special_forces: i32) -> i32 {
        forces + special_forces
    }

    fn handle_betrayal(&self, game: &mut Game, by: By) -> GameResult {
        let Some(actor) = by.player else { return Ok(()) };

        match self.faction {
            Some(Faction::Atreides) => {
                game.play_nexus_card(actor, vec![]);
                game.prevent(by.faction.required("faction")?, FactionAdvantage::AtreidesBattlePlanPrescience, true);
                if let Some(c) = game.battle_mut().and_then(|b| b.current.as_mut()) {
                    c.prescience = None;
                }
            }

            Some(Faction::Harkonnen) => {
                let Some(current_battle) = game.current_battle().cloned() else { return Ok(()) };
                if game.decks.traitor_deck.is_none() {
                    return Ok(());
                }
                let harkonnen = game.get_player(Some(Faction::Harkonnen));
                let traitor = current_battle.plan_of_opponent_faction(game, Faction::Harkonnen).required("plan of opponent faction")?.hero;
                if let (Some(harkonnen), Some(traitor)) = (harkonnen, traitor) {
                    // List.Remove: the first equal element
                    let traitors = &mut game.player_mut(harkonnen).traitors;
                    if let Some(pos) = traitors.iter().position(|t| *t == traitor) {
                        traitors.remove(pos);
                    }
                    let traitor_deck = game.decks.traitor_deck.as_mut().required("traitor deck")?;
                    traitor_deck.items.push(traitor);
                    traitor_deck.shuffle(&mut game.rng);
                    game.stone(Milestone::Shuffled);
                    game.turn.harkonnen_traitor_was_cancelled = true;
                    game.play_nexus_card(actor, elems!["cancel the ", Faction::Harkonnen, " traitor call"]);
                    game.enter_battle(BattleStep::CallingTraitor);
                    game.handle_revealed_battle_plans()?;
                }
            }

            Some(Faction::Fremen) => {
                game.play_nexus_card(actor, vec![]);
                if game.turn.current_main_phase == MainPhase::Blow {
                    game.prevent(by.faction.required("faction")?, FactionAdvantage::FremenRidesMonster, true);
                    if matches!(game.phase(), Phase::FremenRidingMonsterA | Phase::FremenRidingMonsterB) {
                        game.determine_next_shipment_and_move_sub_phase()?;
                    }
                } else if game.turn.current_main_phase == MainPhase::ShipmentAndMove {
                    game.prevent(by.faction.required("faction")?, FactionAdvantage::FremenExtraMove, true);
                }
            }

            Some(Faction::Emperor) => {
                game.play_nexus_card(actor, vec![]);
                if game.turn.current_main_phase == MainPhase::Bidding {
                    game.prevent(by.faction.required("faction")?, FactionAdvantage::EmperorReceiveBid, true);
                } else if game.turn.current_main_phase == MainPhase::Battle && game.applicable(Rule::EmperorSpecialForces) {
                    game.prevent(by.faction.required("faction")?, FactionAdvantage::EmperorSpecialForceBonus, true);
                }
            }

            Some(Faction::Guild) => {
                game.play_nexus_card(actor, vec![]);
                // Nothing in the loop changes StoredRecentlyPaid.
                for p in game.turn.stored_recently_paid.clone() {
                    let from: Part =
                        if p.to == None { "the Bank".into() } else { p.to.into() };
                    let actor_faction = game.player(actor).faction;
                    game.log(elems![actor_faction, " get ", p.amount, " from ", from]);
                    if p.to != None {
                        let get_from = game.get_player(p.to);
                        if let Some(get_from) = get_from {
                            let gf = game.player_mut(get_from);
                            crate::game::amounts::spend(&mut gf.resources, p.amount, "spice")?;
                        }
                    }

                    let me = game.player_mut(actor);
                    crate::game::amounts::gain(&mut me.resources, p.amount, "spice")?;
                }

                // Bureaucracy on the payment Guild just received now falls on whoever took it.
                if let Some(target) = game.target_of_bureaucracy_mut().filter(|t| **t == Some(Faction::Guild)) {
                    *target = by.faction;
                }
            }

            Some(Faction::BeneGesserit) => {
                game.play_nexus_card(actor, vec![]);
                game.prevent(by.faction.required("faction")?, FactionAdvantage::BeneGesseritUsingVoice, true);
                if let Some(c) = game.battle_mut().and_then(|b| b.current.as_mut()) {
                    c.voice = None;
                }
            }

            Some(Faction::Ixian) => {
                game.play_nexus_card(actor, vec![]);
                if game.phase() < Phase::IxianSelectingCard {
                    game.prevent(by.faction.required("faction")?, FactionAdvantage::IxianSelectingCardsOnAuction, true);
                } else if game.phase() > Phase::IxianSelectingCard && game.phase() < Phase::BiddingReport {
                    game.prevent(by.faction.required("faction")?, FactionAdvantage::IxianSwappingCard, true);
                }
            }

            Some(Faction::Tleilaxu) => {
                game.turn.facedancer_was_cancelled = true;
                game.play_nexus_card(actor, elems!["cancel the ", Faction::Tleilaxu, " face dancer reveal"]);
                game.finish_battle()?;
            }

            Some(Faction::Choam) => {
                game.play_nexus_card(
                    actor,
                    elems!["force ", Faction::Choam, " to discard one of their treachery cards at random"],
                );
                let victim_player = game.get_player(Some(Faction::Choam));
                if victim_player.map(|v| !game.player(v).treachery_cards.is_empty()) == Some(true) {
                    let victim_player = victim_player.required("victim player")?;
                    let card = game
                        .player(victim_player)
                        .treachery_cards
                        .iter()
                        .copied()
                        .collect::<Vec<_>>().into_iter().random_item(&mut game.rng);
                    if let Some(card) = card {
                        game.discard_card(Some(card), true)?;
                    }
                } else {
                    game.log(elems![Faction::Choam, " have no treachery cards to discard"]);
                }
            }

            Some(Faction::Richese) => {
                let payment_to_richese = game.quite_recent_payment_to(Faction::Richese);
                let richese = game.get_player(Some(Faction::Richese));
                if let Some(payment_to_richese) = payment_to_richese {
                    let amount_received = payment_to_richese
                        .amount - (if game.turn.was_victim_of_bureaucracy == Some(Faction::Richese) { 2 } else { 0 });
                    game.play_nexus_card(
                        actor,
                        elems!["let ", Faction::Richese, " return the payment of ", Payment::of(amount_received), " they just received"],
                    );
                    if let Some(richese) = richese {
                        let w = game.player_mut(richese);
                        crate::game::amounts::spend(&mut w.resources, amount_received, "spice")?;
                    }
                } else if richese.is_some() && game.card_just_won().is_some() && game.player(richese.required("richese")?).has(game.card_just_won()) {
                    let richese = richese.required("richese")?;
                    game.play_nexus_card(
                        actor,
                        elems!["force ", Faction::Richese, " to discard the card they just won, refunding payment"],
                    );
                    let card_just_won = game.card_just_won();
                    game.discard_card_of(Some(richese), card_just_won, true);
                    let winning_bid = game.bidding().and_then(|b| b.winning_bid.clone());
                    if let Some(winning_bid) = winning_bid {
                        if !winning_bid.bid_karma_bid() && winning_bid.bid_initiator() == Faction::Richese {
                            let w = game.player_mut(richese);
                            crate::game::amounts::gain(&mut w.resources, winning_bid.bid_amount(), "spice")?;
                            let richese_ally = game.player(richese).allied_player(game);
                            if let Some(richese_ally) = richese_ally {
                                if winning_bid.bid_ally_contribution_amount() > 0 {
                                    let a = game.player_mut(richese_ally);
                                    crate::game::amounts::gain(&mut a.resources, winning_bid.bid_ally_contribution_amount(), "spice")?;
                                }
                            }
                            if let Some(richese_ally) = richese_ally {
                                if winning_bid.bid_emperor_contribution_amount() > 0 {
                                    // C# parity: Emperor's contribution is refunded to Richese's ally.
                                    let a = game.player_mut(richese_ally);
                                    crate::game::amounts::gain(&mut a.resources, winning_bid.bid_emperor_contribution_amount(), "spice")?;
                                }
                            }
                        }
                    }
                }
            }

            Some(Faction::Ecaz) => {
                let ecazs_ally = game.get_player(Some(Faction::Ecaz)).and_then(|p| game.player(p).allied_player(game));
                if let (Some(ecazs_ally), Some(ecaz_territory)) = (ecazs_ally, self.ecaz_territory) {
                    Player::forces_in_territory_to_reserves(game, ecazs_ally, ecaz_territory)?;
                    let ally_faction = game.player(ecazs_ally).faction;
                    let ecaz_territory = self.ecaz_territory;
                    game.play_nexus_card(
                        actor,
                        elems!["return all ", ally_faction, " forces in ", ecaz_territory, " to reserves"],
                    );
                    game.flip_bene_gesserit_advisors_when_alone()?;
                }
            }

            Some(Faction::Moritani) => {
                if let Some(moritani_territory) = self.moritani_territory {
                    // Validation only offers a territory with terror. C# took `default(TerrorType)`
                    // (None) from an empty territory and added it to the unplaced tokens.
                    let terror_to_remove = game.terror_in(moritani_territory).into_iter().collect::<Vec<_>>().into_iter().random_item(&mut game.rng);
                    if let Some(terror_to_remove) = terror_to_remove {
                        game.board.terror_on_planet.shift_remove(&terror_to_remove);
                        game.board.unplaced_terror_tokens.push(terror_to_remove);
                    }
                    let moritani_territory = self.moritani_territory;
                    game.play_nexus_card(
                        actor,
                        elems!["return a terror token from ", moritani_territory, " to reserves"],
                    );
                }
            }

            _ => {}
        }
        Ok(())
    }

    fn handle_cunning(&self, game: &mut Game, by: By) {
        let Some(actor) = by.player else { return };

        match self.faction {
            Some(Faction::Atreides) => {
                game.turn.current_nexus_prescience = Some(NexusCardPlayed { submitter: by, atreides_prescience_aspect: self.atreides_prescience_aspect });
                game.play_nexus_card(actor, elems!["see their opponent's ", self.atreides_prescience_aspect]);
            }

            Some(Faction::Harkonnen) => {
                if game.decks.traitor_deck.is_none() {
                    return;
                }
                game.play_nexus_card(actor, elems!["draw a new traitor"]);
                let t = game.decks.traitor_deck.as_mut().expect("traitor deck").draw();
                game.player_mut(actor).traitors.push(t);
                game.interrupt(Interrupt::DiscardingTraitor { faction: by.faction, remaining: 1 });
            }

            Some(Faction::Emperor) => {
                game.turn.current_emperor_cunning = Some(NexusCardPlayed { submitter: by, atreides_prescience_aspect: self.atreides_prescience_aspect });
                game.play_nexus_card(
                    actor,
                    elems!["let 5 ", FactionForce::Emperor, " count as ", FactionSpecialForce::Emperor, " during this battle"],
                );
            }

            Some(Faction::Guild) => {
                game.turn.current_guild_cunning = Some(NexusCardPlayed { submitter: by, atreides_prescience_aspect: self.atreides_prescience_aspect });
                game.play_nexus_card(actor, elems!["perform an extra shipment after their move"]);
            }

            Some(Faction::BeneGesserit) => {
                game.turn.current_bene_gesserit_cunning = Some(NexusCardPlayed { submitter: by, atreides_prescience_aspect: self.atreides_prescience_aspect });
                game.play_nexus_card(
                    actor,
                    elems!["be able to flip advisors to fighters during ", MainPhase::ShipmentAndMove],
                );
            }

            Some(Faction::Ixian) => {
                game.turn.current_ixian_cunning = Some(NexusCardPlayed { submitter: by, atreides_prescience_aspect: self.atreides_prescience_aspect });
                game.play_nexus_card(actor, elems!["let ", FactionForce::Ixian, " be full strength during this battle"]);
            }

            Some(Faction::Tleilaxu) => {
                if game.decks.traitor_deck.is_none() {
                    return;
                }
                let Some(tleilaxu) = game.get_player(Some(Faction::Tleilaxu)) else { return };

                let count = game.player(tleilaxu).revealed_face_dancers.len() as i32;
                game.play_nexus_card(actor, elems!["replace their ", count, " revealed face dancers"]);
                if game.player(tleilaxu).revealed_face_dancers.len() > 0 {
                    let mut i = 0;
                    while i < game.player(tleilaxu).revealed_face_dancers.len() {
                        let d = game.decks.traitor_deck.as_mut().expect("traitor deck").draw();
                        game.player_mut(tleilaxu).face_dancers.push(d);
                        i += 1;
                    }

                    // HashSet iteration order
                    let revealed: Vec<Hero> = game.player(tleilaxu).revealed_face_dancers.iter().copied().collect();
                    let traitor_deck = game.decks.traitor_deck.as_mut().expect("traitor deck");
                    traitor_deck.items.extend(revealed.iter().copied());
                    traitor_deck.shuffle(&mut game.rng);

                    for dancer in &revealed {
                        let face_dancers = &mut game.player_mut(tleilaxu).face_dancers;
                        if let Some(pos) = face_dancers.iter().position(|d| d == dancer) {
                            face_dancers.remove(pos);
                        }
                    }
                    game.player_mut(tleilaxu).revealed_face_dancers.clear();

                    game.stone(Milestone::Shuffled);
                }
            }

            Some(Faction::Ecaz) => {
                if game.vidal().is_some() && !game.is_alive(Hero::Leader(game.vidal().expect("vidal"))) {
                    let vidal = game.vidal().expect("vidal");
                    game.revive(actor, Hero::Leader(vidal));

                    if self.tleilaxu_assign_skill {
                        let vidal = game.vidal();
                        game.prepare_skill_assignment_to_revived_leader(actor, vidal);
                    }
                }
                game.take_vidal(actor, VidalMoment::EndOfTurn);
                let vidal = game.vidal();
                game.play_nexus_card(actor, elems!["take ", vidal, " this turn"]);
            }

            _ => {}
        }
    }

    fn handle_secret_ally(&self, game: &mut Game, by: By) -> GameResult {
        let Some(actor) = by.player else { return Ok(()) };

        match self.faction {
            Some(Faction::Atreides) => {
                game.turn.current_nexus_prescience = Some(NexusCardPlayed { submitter: by, atreides_prescience_aspect: self.atreides_prescience_aspect });
                game.play_nexus_card(actor, elems!["see their opponent's ", self.atreides_prescience_aspect]);
            }

            Some(Faction::Harkonnen) => {
                if game.decks.traitor_deck.is_none() {
                    return Ok(());
                }
                game.play_nexus_card(actor, elems!["draw two new traitors"]);
                let t = game.decks.traitor_deck.as_mut().expect("traitor deck").draw();
                game.player_mut(actor).traitors.push(t);
                let t = game.decks.traitor_deck.as_mut().expect("traitor deck").draw();
                game.player_mut(actor).traitors.push(t);
                game.interrupt(Interrupt::DiscardingTraitor { faction: by.faction, remaining: 2 });
            }

            Some(Faction::Fremen) => {
                game.turn.current_fremen_secret_ally = Some(NexusCardPlayed { submitter: by, atreides_prescience_aspect: self.atreides_prescience_aspect });
                if game.turn.current_main_phase == MainPhase::Blow {
                    game.play_nexus_card(
                        actor,
                        elems!["prevent their forces from being devoured by ", Concept::Monster],
                    );
                } else if game.turn.current_main_phase == MainPhase::Resurrection {
                    game.play_nexus_card(actor, elems!["increase their free revival to 3"]);
                }
            }

            Some(Faction::Guild) => {
                game.turn.current_guild_secret_ally = Some(NexusCardPlayed { submitter: by, atreides_prescience_aspect: self.atreides_prescience_aspect });
                game.play_nexus_card(actor, elems!["be able to ship as ", Faction::Guild]);
            }

            Some(Faction::Tleilaxu) => {
                game.stone(Milestone::RaiseDead);
                let Some(player) = game.get_player(by.faction) else { return Ok(()) };

                Player::revive_forces(game, player, i32::from(self.tleilaxu_forces))?;
                Player::revive_special_forces(game, player, i32::from(self.tleilaxu_special_forces))?;

                if i32::from(self.tleilaxu_special_forces) > 0 {
                    game.turn.revivals.factions_that_revived_special_forces_this_turn.push(by.faction.expect("faction"));
                }

                if let Some(tleilaxu_hero) = self.tleilaxu_hero {
                    game.revive(actor, tleilaxu_hero);

                    if self.tleilaxu_assign_skill {
                        if let Some(revived_leader) = self.tleilaxu_hero.expect("tleilaxu hero").as_leader() {
                            game.prepare_skill_assignment_to_revived_leader(player, Some(revived_leader));
                        }
                    }
                }

                let total = i32::from(self.tleilaxu_forces) + i32::from(self.tleilaxu_special_forces);
                let force = game.player(player).force();
                let special_force = game.player(player).special_force();
                game.play_nexus_card(
                    actor,
                    elems![
                        "revive ",
                        express_part_if!(self.tleilaxu_hero.is_some(); self.tleilaxu_hero),
                        express_part_if!(self.tleilaxu_hero.is_some() && total > 0; " and "),
                        express_part_if!(i32::from(self.tleilaxu_forces) > 0; i32::from(self.tleilaxu_forces), " ", force),
                        express_part_if!(i32::from(self.tleilaxu_forces) > 0 && i32::from(self.tleilaxu_special_forces) > 0; " and "),
                        express_part_if!(i32::from(self.tleilaxu_special_forces) > 0; i32::from(self.tleilaxu_special_forces), " ", special_force)
                    ],
                );

                if let Some(tleilaxu_location) = self.tleilaxu_location {
                    if by.faction == Some(Faction::Fremen) {
                        Player::ship_special_forces(game, player, tleilaxu_location, 1)?;
                        let tleilaxu_location = self.tleilaxu_location;
                        game.log(
                            elems![by.faction, " place ", FactionSpecialForce::Fremen, " in ", tleilaxu_location],
                        );
                    }
                }
            }

            Some(Faction::Choam) => {
                if game.turn.current_main_phase == MainPhase::Collection {
                    game.play_nexus_card(
                        actor,
                        elems!["discard a ", TreacheryCardType::Useless, " card to get ", Payment::of(2)],
                    );
                    let Some(choam_card) = self.choam_card else { return Ok(()) };
                    // C# parity: the card is not validated here.
                    game.discard_card_of(Some(actor), Some(choam_card), true);
                    let me = game.player_mut(actor);
                    me.resources += 2;
                } else if game.phase() == Phase::BattleConclusion {
                    let Some(current_battle) = game.current_battle().cloned() else { return Ok(()) };

                    let Some(auditee) = current_battle.opponent_of_faction(game, by.faction) else { return Ok(()) };
                    let Some(recent_battle_plan) = current_battle.plan_of(game, Some(auditee)) else { return Ok(()) };
                    let weapon = recent_battle_plan.weapon();
                    let defense = recent_battle_plan.defense();
                    let hero = recent_battle_plan.hero;
                    let auditable_cards: Vec<TreacheryCard> = game
                        .player(auditee)
                        .treachery_cards
                        .iter()
                        .copied()
                        .filter(|c| Some(*c) != weapon && Some(*c) != defense && Some(Hero::TreacheryCard(*c)) != hero)
                        .collect();

                    let auditee_faction = game.player(auditee).faction;
                    game.play_nexus_card(
                        actor,
                        elems!["see a random treachery card in the ", auditee_faction, " hand"],
                    );

                    if !auditable_cards.is_empty() {
                        let audited_card = auditable_cards.iter().copied().random_item(&mut game.rng);
                        if let Some(audited_card) = audited_card {
                            Game::register_known_by_player(game, actor, Some(audited_card));
                            game.log_to(by.faction.expect("faction"), elems!["You see: ", audited_card]);
                            game.log_to(auditee_faction.expect("faction"), elems!["You showed them: ", audited_card]);
                        }
                    } else {
                        game.log(elems![auditee_faction, " don't have cards to audit"]);
                    }
                }
            }

            Some(Faction::Ecaz) => {
                game.play_nexus_card(
                    actor,
                    elems!["force ", self.ecaz_faction, " to reveal if they have an ", Faction::Ecaz, " traitor"],
                );
                game.log(
                    elems![self.ecaz_faction, " reveal to ", by.faction, " if they have a ", by.faction, " traitor"],
                );
                let initiator = by.faction;
                let has_traitor = game
                    .get_player(self.ecaz_faction)
                    .map(|p| game.player(p).traitors.iter().any(|t| Some(t.faction()) == Some(initiator)))
                    == Some(true);
                game.log_to(
                    by.faction.expect("faction"),
                    elems![
                        self.ecaz_faction,
                        " reveal that they ",
                        if has_traitor { "DO" } else { "DON'T" },
                        " have a ",
                        by.faction,
                        " traitor "
                    ],
                );
                game.log_to(
                    self.ecaz_faction.expect("faction"),
                    elems![
                        " you revealed to them that you ",
                        if has_traitor { "DO" } else { "DON'T" },
                        " have a ",
                        by.faction,
                        " traitor "
                    ],
                );
            }

            _ => {}
        }
        Ok(())
    }
}

impl EventRules for NexusPlayed {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        match self.faction {
            None => return Err(Rejection(express!("Invalid Nexus faction"))),

            Some(Faction::Atreides) => {
                if (self.is_cunning(by.faction) || self.is_secret_ally(game)) && self.atreides_prescience_aspect == None {
                    return Err(Rejection(express!("Invalid battle plan element")));
                }
            }

            Some(Faction::Tleilaxu) => {
                if self.is_secret_ally(game) {
                    let me = by.player();
                    if self.tleilaxu_hero.is_some()
                        && !NexusPlayed::valid_tleilaxu_heroes(game, me).contains(&self.tleilaxu_hero.expect("tleilaxu hero"))
                    {
                        return Err(Rejection(express!("Invalid leader")));
                    }
                    if i32::from(self.tleilaxu_forces) > NexusPlayed::valid_tleilaxu_max_amount(game, me, false) {
                        return Err(Rejection(express!("You can't revive that many ", game.player(me).force())));
                    }
                    if i32::from(self.tleilaxu_special_forces) > NexusPlayed::valid_tleilaxu_max_amount(game, me, true) {
                        return Err(Rejection(express!("You can't revive that many ", game.player(me).special_force())));
                    }
                    if self.determine_tleilaxu_cost() > i32::from(game.player(me).resources) {
                        return Err(Rejection(express!("You can't pay that much")));
                    }
                    if (i32::from(self.tleilaxu_forces) + i32::from(self.tleilaxu_special_forces)) > 5 {
                        return Err(Rejection(express!("You can't revive that many forces")));
                    }
                    if self.tleilaxu_assign_skill && self.tleilaxu_hero.is_none() {
                        return Err(Rejection(express!("You must revive a leader to assign a skill to")));
                    }
                    if self.tleilaxu_assign_skill
                        && (self.tleilaxu_hero.is_none() || !Revival::may_assign_skill(game, me, self.tleilaxu_hero))
                    {
                        return Err(Rejection(express!("You can't assign a skill to this leader")));
                    }

                    if let Some(tleilaxu_location) = self.tleilaxu_location {
                        if !Revival::valid_revived_force_locations(game, me).any(|v| v == tleilaxu_location) {
                            return Err(Rejection(express!("You can't place revived forces there")));
                        }

                        if i32::from(self.tleilaxu_number_of_special_forces_in_location)
                            > Revival::number_of_special_forces_that_may_be_placed_on_planet(game, me, i32::from(self.tleilaxu_special_forces))
                        {
                            return Err(Rejection(express!("You can't place that many forces directly on the planet")));
                        }
                    }
                }
            }

            Some(Faction::Choam) => {
                if self.is_cunning(by.faction) {
                    if self.choam_card.is_none() {
                        return Err(Rejection(express!("Select a ", TreacheryCardType::Useless, " card to discard")));
                    }
                    if self.choam_card.is_some()
                        && !NexusPlayed::valid_choam_cards(game, by.player()).any(|v| v == self.choam_card.expect("choam card"))
                    {
                        return Err(Rejection(express!("Invalid card")));
                    }
                }
            }

            Some(Faction::Ecaz) => {
                if self.is_betrayal(game, by.faction) {
                    let Some(ecaz_territory) = self.ecaz_territory else {
                        return Err(Rejection(express!("Select a territory")));
                    };
                    if !NexusPlayed::valid_ecaz_territories(game).any(|v| v == ecaz_territory) {
                        return Err(Rejection(express!("Invalid territory")));
                    }
                } else if self.is_secret_ally(game) {
                    if !NexusPlayed::valid_ecaz_factions(game).any(|v| Some(v) == self.ecaz_faction) {
                        return Err(Rejection(express!("Invalid faction")));
                    }
                }
            }

            Some(Faction::Moritani) => {
                if self.is_betrayal(game, by.faction)
                    && (self.moritani_territory.is_none()
                        || !NexusPlayed::valid_moritani_territories(game).any(|v| v == self.moritani_territory.expect("moritani territory")))
                {
                    return Err(Rejection(express!("This territory contains no terror token")));
                }
            }

            _ => {}
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        if self.is_cunning(by.faction) {
            self.handle_cunning(game, by);
        } else if self.is_secret_ally(game) {
            self.handle_secret_ally(game, by)?;
        } else {
            self.handle_betrayal(game, by)?;
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " play a Nexus card")
    }
}

impl crate::events::location_event::LocationEvent for NexusPlayed {
    fn to(&self, game: &Game) -> Option<Location> {
        NexusPlayed::to(self, game)
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        NexusPlayed::total_amount_of_forces_added_to_location(self, game)
    }

    fn forces_added_to_location(&self, game: &Game) -> i32 {
        NexusPlayed::forces_added_to_location(self, game)
    }

    fn special_forces_added_to_location(&self, game: &Game) -> i32 {
        NexusPlayed::special_forces_added_to_location(self, game)
    }
}
