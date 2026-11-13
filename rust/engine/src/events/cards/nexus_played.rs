use crate::prelude::*;
use crate::compat::random_ext::RandomExt;

#[derive(Clone, Debug, Canon)]
pub struct NexusPlayed {
    pub faction: Faction,
    pub atreides_prescience_aspect: PrescienceAspect,
    pub tleilaxu_forces: i32,
    pub tleilaxu_special_forces: i32,
    #[canon(name = "_tleilaxuHeroId")]
    pub tleilaxu_hero_id: i32,
    pub tleilaxu_assign_skill: bool,
    #[canon(name = "_choamCardId")]
    pub choam_card_id: i32,
    #[canon(name = "_ecazTerritoryId")]
    pub ecaz_territory_id: i32,
    pub ecaz_faction: Faction,
    #[canon(name = "_moritaniTerritoryId")]
    pub moritani_territory_id: i32,
    pub tleilaxu_number_of_special_forces_in_location: i32,
    #[canon(name = "_tleilaxuLocationId")]
    pub tleilaxu_location_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for NexusPlayed {
    fn default() -> Self {
        NexusPlayed {
            faction: Default::default(),
            atreides_prescience_aspect: Default::default(),
            tleilaxu_forces: Default::default(),
            tleilaxu_special_forces: Default::default(),
            tleilaxu_hero_id: -1,
            tleilaxu_assign_skill: false,
            choam_card_id: Default::default(),
            ecaz_territory_id: Default::default(),
            ecaz_faction: Default::default(),
            moritani_territory_id: Default::default(),
            tleilaxu_number_of_special_forces_in_location: Default::default(),
            tleilaxu_location_id: -1,
            base: Default::default(),
        }
    }
}

macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

impl NexusPlayed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn tleilaxu_hero(&self) -> Option<Hero> {
        LeaderManager::hero_lookup().find(self.tleilaxu_hero_id)
    }

    pub fn set_tleilaxu_hero(&mut self, value: Option<Hero>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.tleilaxu_hero_id = LeaderManager::hero_lookup().get_id(value);
    }

    pub fn choam_card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::get(self.choam_card_id)
    }

    pub fn set_choam_card(&mut self, value: Option<TreacheryCard>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.choam_card_id = TreacheryCardManager::get_id(value);
    }

    pub fn ecaz_territory(&self, game: &Game) -> Option<Territory> {
        game.map.territory_lookup().find(self.ecaz_territory_id)
    }

    pub fn set_ecaz_territory(&mut self, game: &Game, value: Option<Territory>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.ecaz_territory_id = game.map.territory_lookup().get_id(value);
    }

    pub fn moritani_territory(&self, game: &Game) -> Option<Territory> {
        game.map.territory_lookup().find(self.moritani_territory_id)
    }

    pub fn set_moritani_territory(&mut self, game: &Game, value: Option<Territory>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.moritani_territory_id = game.map.territory_lookup().get_id(value);
    }

    pub fn tleilaxu_location(&self, game: &Game) -> Option<Location> {
        game.map.location_lookup().find(self.tleilaxu_location_id)
    }

    pub fn set_tleilaxu_location(&mut self, game: &Game, value: Option<Location>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.tleilaxu_location_id = game.map.location_lookup().get_id(value);
    }

    pub fn to(&self, game: &Game) -> Option<Location> {
        self.tleilaxu_location(game)
    }

    pub fn total_amount_of_forces_added_to_location(&self, _game: &Game) -> i32 {
        self.tleilaxu_number_of_special_forces_in_location
    }

    pub fn forces_added_to_location(&self, _game: &Game) -> i32 {
        0
    }

    pub fn special_forces_added_to_location(&self, _game: &Game) -> i32 {
        self.tleilaxu_number_of_special_forces_in_location
    }

    pub fn is_cunning(&self) -> bool {
        self.initiator() == self.faction
    }

    pub fn is_secret_ally(&self, game: &Game) -> bool {
        !game.is_playing(self.faction)
    }

    pub fn is_betrayal(&self, game: &Game) -> bool {
        !(self.is_cunning() || self.is_secret_ally(game))
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        match self.faction {
            Faction::None => return Some(express!("Invalid Nexus faction")),

            Faction::Atreides => {
                if (self.is_cunning() || self.is_secret_ally(game)) && self.atreides_prescience_aspect == PrescienceAspect::None {
                    return Some(express!("Invalid battle plan element"));
                }
            }

            Faction::Tleilaxu => {
                if self.is_secret_ally(game) {
                    let me = self.player();
                    if self.tleilaxu_hero().is_some()
                        && !NexusPlayed::valid_tleilaxu_heroes(game, me).contains(&self.tleilaxu_hero().nre())
                    {
                        return Some(express!("Invalid leader"));
                    }
                    if self.tleilaxu_forces > NexusPlayed::valid_tleilaxu_max_amount(game, me, false) {
                        return Some(express!("You can't revive that many ", game.player(me).force()));
                    }
                    if self.tleilaxu_special_forces > NexusPlayed::valid_tleilaxu_max_amount(game, me, true) {
                        return Some(express!("You can't revive that many ", game.player(me).special_force()));
                    }
                    if self.determine_tleilaxu_cost() > game.player(me).resources {
                        return Some(express!("You can't pay that much"));
                    }
                    if (self.tleilaxu_forces + self.tleilaxu_special_forces) > 5 {
                        return Some(express!("You can't revive that many forces"));
                    }
                    if self.tleilaxu_assign_skill && self.tleilaxu_hero().is_none() {
                        return Some(express!("You must revive a leader to assign a skill to"));
                    }
                    if self.tleilaxu_assign_skill
                        && (self.tleilaxu_hero().is_none() || !Revival::may_assign_skill(game, me, self.tleilaxu_hero()))
                    {
                        return Some(express!("You can't assign a skill to this leader"));
                    }

                    if let Some(tleilaxu_location) = self.tleilaxu_location(game) {
                        if !Revival::valid_revived_force_locations(game, me).any(|v| v == tleilaxu_location) {
                            return Some(express!("You can't place revived forces there"));
                        }

                        if self.tleilaxu_number_of_special_forces_in_location
                            > Revival::number_of_special_forces_that_may_be_placed_on_planet(game, me, self.tleilaxu_special_forces)
                        {
                            return Some(express!("You can't place that many forces directly on the planet"));
                        }
                    }
                }
            }

            Faction::Choam => {
                if self.is_cunning() {
                    if self.choam_card().is_none() {
                        return Some(express!("Select a ", TreacheryCardType::Useless, " card to discard"));
                    }
                    if self.choam_card().is_some()
                        && !NexusPlayed::valid_choam_cards(game, self.player()).any(|v| v == self.choam_card().nre())
                    {
                        return Some(express!("Invalid card"));
                    }
                }
            }

            Faction::Ecaz => {
                if self.is_betrayal(game) {
                    let Some(ecaz_territory) = self.ecaz_territory(game) else {
                        return Some(express!("Select a territory"));
                    };
                    if !NexusPlayed::valid_ecaz_territories(game).any(|v| v == ecaz_territory) {
                        return Some(express!("Invalid territory"));
                    }
                } else if self.is_secret_ally(game) {
                    if !NexusPlayed::valid_ecaz_factions(game).any(|v| v == self.ecaz_faction) {
                        return Some(express!("Invalid faction"));
                    }
                }
            }

            Faction::Moritani => {
                if self.is_betrayal(game)
                    && (self.moritani_territory(game).is_none()
                        || !NexusPlayed::valid_moritani_territories(game).any(|v| v == self.moritani_territory(game).nre()))
                {
                    return Some(express!("This territory contains no terror token"));
                }
            }

            _ => {}
        }

        None
    }

    pub fn can_use_cunning(game: &Game, p: PlayerIndex) -> bool {
        let p = game.player(p);
        p.nexus != Faction::None && p.nexus == p.faction
    }

    pub fn can_use_secret_ally(g: &Game, p: PlayerIndex) -> bool {
        let p = g.player(p);
        p.nexus != Faction::None && !g.is_playing(p.nexus)
    }

    pub fn can_use_betrayal(g: &Game, p: PlayerIndex) -> bool {
        let p = g.player(p);
        p.nexus != Faction::None && p.nexus != p.faction && g.is_playing(p.nexus)
    }

    pub fn is_applicable(g: &Game, p: PlayerIndex) -> bool {
        if g.current_phase == Phase::NexusCards || g.current_phase_cannot_be_interrupted() {
            return false;
        }

        let cunning = NexusPlayed::can_use_cunning(g, p);
        let secret_ally = NexusPlayed::can_use_secret_ally(g, p);
        let betrayal = NexusPlayed::can_use_betrayal(g, p);

        let game_is_in_battle = g.current_phase == Phase::BattlePhase && g.current_battle.is_some();
        let is_currently_formulating_battle_plan = game_is_in_battle
            && g.current_battle.as_ref().nre().is_aggressor_or_defender(g, p)
            && (g.defender_plan.is_none() || g.aggressor_plan.is_none());

        let battle = || g.current_battle.as_ref().nre();

        match g.player(p).nexus {
            Faction::Atreides if betrayal => game_is_in_battle && battle().is_aggressor_or_defender_faction(g, Faction::Atreides),
            Faction::Atreides if cunning || secret_ally => is_currently_formulating_battle_plan,

            Faction::Harkonnen if betrayal => g.current_phase == Phase::CancellingTraitor,
            Faction::Harkonnen if cunning => true,
            Faction::Harkonnen if secret_ally => g.current_main_phase == MainPhase::Contemplate,

            Faction::Fremen if betrayal => {
                g.current_main_phase == MainPhase::Blow || g.current_main_phase == MainPhase::ShipmentAndMove
            }
            Faction::Fremen if secret_ally => {
                g.current_main_phase == MainPhase::Blow || g.current_main_phase == MainPhase::Resurrection
            }

            Faction::Emperor if betrayal => {
                g.current_main_phase == MainPhase::Bidding
                    || (game_is_in_battle
                        && g.applicable(Rule::EmperorSpecialForces)
                        && battle().is_aggressor_or_defender_faction(g, Faction::Emperor))
            }
            Faction::Emperor if cunning => is_currently_formulating_battle_plan,

            Faction::Guild if betrayal => {
                g.current_main_phase == MainPhase::ShipmentAndMove && g.has_recent_payment_for(GameEventType::Shipment)
            }
            Faction::Guild if cunning => g.current_phase == Phase::GuildMove && !g.in_guild_cunning_shipment,
            Faction::Guild if secret_ally => g.current_phase == Phase::NonGuildShip,

            Faction::BeneGesserit if betrayal => game_is_in_battle && battle().is_involved_faction(g, Faction::BeneGesserit),
            Faction::BeneGesserit if cunning => g.current_main_phase == MainPhase::ShipmentAndMove,

            Faction::Ixian if betrayal => g.current_main_phase == MainPhase::Bidding && g.current_phase < Phase::BiddingReport,
            Faction::Ixian if cunning => is_currently_formulating_battle_plan,

            Faction::Tleilaxu if betrayal => g.current_phase == Phase::Facedancing,
            Faction::Tleilaxu if cunning => true,
            Faction::Tleilaxu if secret_ally => g.current_phase == Phase::Resurrection,

            Faction::Choam if betrayal => true,
            Faction::Choam if secret_ally => {
                (g.current_main_phase == MainPhase::Collection && !NexusPlayed::valid_choam_cards(g, p).next().is_none())
                    || (g.current_phase == Phase::BattleConclusion
                        && g.current_battle.is_some()
                        && Some(g.player(p).faction) == g.battle_winner)
            }

            Faction::Richese if betrayal && g.current_main_phase == MainPhase::Bidding => {
                (g.richese_bidding_just_finished || g.card_sold_on_black_market.is_some())
                    && (g.has_quite_recent_payment_to(Faction::Richese)
                        || (g.card_just_won.is_some()
                            && g.get_player(Some(Faction::Richese)).map(|w| g.player(w).has(g.card_just_won)) == Some(true)))
            }

            Faction::Ecaz if betrayal => {
                g.current_main_phase < MainPhase::ShipmentAndMove && !NexusPlayed::valid_ecaz_territories(g).next().is_none()
            }
            Faction::Ecaz if cunning || secret_ally => true,

            Faction::Moritani if betrayal => !NexusPlayed::valid_moritani_territories(g).next().is_none(),

            _ => false,
        }
    }

    pub fn valid_tleilaxu_max_amount(g: &Game, p: PlayerIndex, special_forces: bool) -> i32 {
        let p = g.player(p);
        if special_forces {
            if p.faction == Faction::Emperor || p.faction == Faction::Fremen {
                if g.factions_that_revived_special_forces_this_turn.contains(&p.faction) {
                    return 0;
                }
                return std::cmp::min(p.special_forces_killed, 1);
            }

            return std::cmp::min(p.special_forces_killed, 5);
        }

        std::cmp::min(p.forces_killed, 5)
    }

    pub fn valid_tleilaxu_heroes(game: &Game, player: PlayerIndex) -> Vec<Hero> {
        RaiseDeadPlayed::valid_heroes(game, player)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_choam_cards(game: &Game, player: PlayerIndex) -> impl Iterator<Item = TreacheryCard> {
        game.player(player).treachery_cards.iter().copied().filter(move |c| c.type_() == TreacheryCardType::Useless)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_ecaz_territories(g: &Game) -> impl Iterator<Item = Territory> {
        use either::Either;
        use std::iter::empty;
        let ecaz = g.get_player(Some(Faction::Ecaz));

        if let Some(ecaz) = ecaz {
            if g.player(ecaz).has_ally() {
                return Either::Left(g.map.territories(false).filter(move |t| {
                    g.player(ecaz).any_forces_in_territory(Some(*t)) > 0
                        && g.player(g.player(ecaz).allied_player(g).nre()).any_forces_in_territory(Some(*t)) > 0
                }));
            }
        }

        Either::Right(empty())
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_ecaz_factions(g: &Game) -> impl Iterator<Item = Faction> {
        g.players
            .iter()
            .filter(move |p| p.faction != Faction::Ecaz && p.faction != Faction::Tleilaxu)
            .map(move |p| p.faction)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_moritani_territories(g: &Game) -> impl Iterator<Item = Territory> {
        crate::iter::unique(g.terror_on_planet.values().copied())
    }

    pub fn may_select_location_for_revived_forces(game: &Game, player: PlayerIndex, special_forces: i32) -> bool {
        game.player(player).is(Faction::Fremen)
            && special_forces >= 1
            && game.player(player).has_high_threshold_0(game)
            && !Revival::valid_revived_force_locations(game, player).next().is_none()
    }

    pub fn determine_tleilaxu_cost(&self) -> i32 {
        NexusPlayed::determine_tleilaxu_cost_int32_int32(self.tleilaxu_forces, self.tleilaxu_special_forces)
    }

    pub fn determine_tleilaxu_cost_int32_int32(forces: i32, special_forces: i32) -> i32 {
        forces + special_forces
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        if self.is_cunning() {
            self.handle_cunning(game);
        } else if self.is_secret_ally(game) {
            self.handle_secret_ally(game);
        } else {
            self.handle_betrayal(game)?;
        }
        Ok(())
    }

    pub fn handle_betrayal(&self, game: &mut Game) -> GameResult {
        let Some(actor) = self.player_opt() else { return Ok(()) };

        match self.faction {
            Faction::Atreides => {
                game.play_nexus_card(actor, vec![]);
                game.prevent(self.initiator(), FactionAdvantage::AtreidesBattlePlanPrescience, true);
                game.current_prescience = None;
            }

            Faction::Harkonnen => {
                let Some(current_battle) = game.current_battle.clone() else { return Ok(()) };
                if game.traitor_deck.is_none() {
                    return Ok(());
                }
                let harkonnen = self.get_player(game, Some(Faction::Harkonnen));
                let traitor = current_battle.plan_of_opponent_faction(game, Faction::Harkonnen).required("plan of opponent faction")?.hero();
                if let (Some(harkonnen), Some(traitor)) = (harkonnen, traitor) {
                    // List.Remove: the first equal element
                    let traitors = &mut game.player_mut(harkonnen).traitors;
                    if let Some(pos) = traitors.iter().position(|t| *t == traitor) {
                        traitors.remove(pos);
                    }
                    let traitor_deck = game.traitor_deck.as_mut().required("traitor deck")?;
                    traitor_deck.items.push(traitor);
                    traitor_deck.shuffle();
                    game.stone(Milestone::Shuffled);
                    game.harkonnen_traitor_was_cancelled = true;
                    game.play_nexus_card(actor, elems!["cancel the ", Faction::Harkonnen, " traitor call"]);
                    game.enter(Phase::CallTraitorOrPass, true);
                    game.handle_revealed_battle_plans()?;
                }
            }

            Faction::Fremen => {
                game.play_nexus_card(actor, vec![]);
                if game.current_main_phase == MainPhase::Blow {
                    game.prevent(self.initiator(), FactionAdvantage::FremenRidesMonster, true);
                    if matches!(game.current_phase, Phase::FremenRidingMonsterA | Phase::FremenRidingMonsterB) {
                        game.determine_next_shipment_and_move_sub_phase()?;
                    }
                } else if game.current_main_phase == MainPhase::ShipmentAndMove {
                    game.prevent(self.initiator(), FactionAdvantage::FremenExtraMove, true);
                }
            }

            Faction::Emperor => {
                game.play_nexus_card(actor, vec![]);
                if game.current_main_phase == MainPhase::Bidding {
                    game.prevent(self.initiator(), FactionAdvantage::EmperorReceiveBid, true);
                } else if game.current_main_phase == MainPhase::Battle && game.applicable(Rule::EmperorSpecialForces) {
                    game.prevent(self.initiator(), FactionAdvantage::EmperorSpecialForceBonus, true);
                }
            }

            Faction::Guild => {
                game.play_nexus_card(actor, vec![]);
                // Nothing in the loop changes StoredRecentlyPaid.
                for p in game.stored_recently_paid.clone() {
                    let from: ExpressionElement =
                        if p.to == Faction::None { "the Bank".into() } else { p.to.into() };
                    // C# parity: logs the Player object itself, which prints as its type name,
                    // with no space before "get" (hazards 11).
                    self.log_objects(game, elems![actor, "get ", p.amount, " from ", from]);
                    if p.to != Faction::None {
                        let get_from = self.get_player(game, Some(p.to));
                        if let Some(get_from) = get_from {
                            let gf = game.player_mut(get_from);
                            gf.resources -= p.amount;
                        }
                    }

                    let me = game.player_mut(actor);
                    me.resources += p.amount;
                }

                if game.target_of_bureaucracy == Faction::Guild {
                    game.target_of_bureaucracy = self.initiator();
                }
            }

            Faction::BeneGesserit => {
                game.play_nexus_card(actor, vec![]);
                game.prevent(self.initiator(), FactionAdvantage::BeneGesseritUsingVoice, true);
                game.current_voice = None;
            }

            Faction::Ixian => {
                game.play_nexus_card(actor, vec![]);
                if game.current_phase < Phase::IxianSelectingCard {
                    game.prevent(self.initiator(), FactionAdvantage::IxianSelectingCardsOnAuction, true);
                } else if game.current_phase > Phase::IxianSelectingCard && game.current_phase < Phase::BiddingReport {
                    game.prevent(self.initiator(), FactionAdvantage::IxianSwappingCard, true);
                }
            }

            Faction::Tleilaxu => {
                game.facedancer_was_cancelled = true;
                game.play_nexus_card(actor, elems!["cancel the ", Faction::Tleilaxu, " face dancer reveal"]);
                game.finish_battle()?;
            }

            Faction::Choam => {
                game.play_nexus_card(
                    actor,
                    elems!["force ", Faction::Choam, " to discard one of their treachery cards at random"],
                );
                let victim_player = self.get_player(game, Some(Faction::Choam));
                if victim_player.map(|v| !game.player(v).treachery_cards.is_empty()) == Some(true) {
                    let victim_player = victim_player.required("victim player")?;
                    let card = game
                        .player(victim_player)
                        .treachery_cards
                        .iter()
                        .copied()
                        .random_or_default(game.random.as_ref().required("random")?);
                    if let Some(card) = card {
                        game.discard_treachery_card_boolean(Some(card), true)?;
                    }
                } else {
                    self.log_objects(game, elems![Faction::Choam, " have no treachery cards to discard"]);
                }
            }

            Faction::Richese => {
                let payment_to_richese = game.quite_recent_payment_to(Faction::Richese);
                let richese = self.get_player(game, Some(Faction::Richese));
                if let Some(payment_to_richese) = payment_to_richese {
                    let amount_received = payment_to_richese
                        .amount - (if game.was_victim_of_bureaucracy == Faction::Richese { 2 } else { 0 });
                    game.play_nexus_card(
                        actor,
                        elems!["let ", Faction::Richese, " return the payment of ", Payment::of(amount_received), " they just received"],
                    );
                    if let Some(richese) = richese {
                        let w = game.player_mut(richese);
                        w.resources -= amount_received;
                    }
                } else if richese.is_some() && game.card_just_won.is_some() && game.player(richese.required("richese")?).has(game.card_just_won) {
                    let richese = richese.required("richese")?;
                    game.play_nexus_card(
                        actor,
                        elems!["force ", Faction::Richese, " to discard the card they just won, refunding payment"],
                    );
                    let card_just_won = game.card_just_won;
                    game.discard_player_treachery_card_boolean(Some(richese), card_just_won, true);
                    let winning_bid = game.winning_bid.clone();
                    if let Some(winning_bid) = winning_bid {
                        if !winning_bid.i_bid_karma_bid() && winning_bid.i_bid_initiator() == Faction::Richese {
                            let w = game.player_mut(richese);
                            w.resources += winning_bid.i_bid_amount();
                            let richese_ally = game.player(richese).allied_player(game);
                            if let Some(richese_ally) = richese_ally {
                                if winning_bid.i_bid_ally_contribution_amount() > 0 {
                                    let a = game.player_mut(richese_ally);
                                    a.resources += winning_bid.i_bid_ally_contribution_amount();
                                }
                            }
                            if let Some(richese_ally) = richese_ally {
                                if winning_bid.i_bid_emperor_contribution_amount() > 0 {
                                    // C# parity: Emperor's contribution is refunded to Richese's ally (hazards 18).
                                    let a = game.player_mut(richese_ally);
                                    a.resources += winning_bid.i_bid_emperor_contribution_amount();
                                }
                            }
                        }
                    }
                }
            }

            Faction::Ecaz => {
                let ecazs_ally = game.get_player(Some(Faction::Ecaz)).and_then(|p| game.player(p).allied_player(game));
                if let (Some(ecazs_ally), Some(ecaz_territory)) = (ecazs_ally, self.ecaz_territory(game)) {
                    Player::forces_to_reserves_territory(game, ecazs_ally, ecaz_territory);
                    let ally_faction = game.player(ecazs_ally).faction;
                    let ecaz_territory = self.ecaz_territory(game);
                    game.play_nexus_card(
                        actor,
                        elems!["return all ", ally_faction, " forces in ", ecaz_territory, " to reserves"],
                    );
                    game.flip_bene_gesserit_advisors_when_alone();
                }
            }

            Faction::Moritani => {
                if let Some(moritani_territory) = self.moritani_territory(game) {
                    // TerrorIn iterates TerrorOnPlanet in .NET slot order (hazards 3.1). An empty
                    // result gives default(TerrorType), which C# then removes and adds.
                    let terror_to_remove = game
                        .terror_in(moritani_territory)
                        .into_iter()
                        .random_or_default(game.random.as_ref().required("random")?)
                        .unwrap_or_default();
                    game.terror_on_planet.remove(&terror_to_remove);
                    game.unplaced_terror_tokens.push(terror_to_remove);
                    let moritani_territory = self.moritani_territory(game);
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

    pub fn handle_cunning(self: &Rc<Self>, game: &mut Game) {
        let Some(actor) = self.player_opt() else { return };

        match self.faction {
            Faction::Atreides => {
                game.current_nexus_prescience = Some(self.clone());
                game.play_nexus_card(actor, elems!["see their opponent's ", self.atreides_prescience_aspect]);
            }

            Faction::Harkonnen => {
                if game.traitor_deck.is_none() {
                    return;
                }
                game.phase_before_discarding_traitor = game.current_phase;
                game.faction_that_must_discard_traitor = self.initiator();
                game.number_of_traitors_to_discard = 1;
                game.play_nexus_card(actor, elems!["draw a new traitor"]);
                let t = game.traitor_deck.as_mut().nre().draw();
                game.player_mut(actor).traitors.push(t);
                game.enter(Phase::DiscardingTraitor, true);
            }

            Faction::Emperor => {
                game.current_emperor_cunning = Some(self.clone());
                game.play_nexus_card(
                    actor,
                    elems!["let 5 ", FactionForce::Emperor, " count as ", FactionSpecialForce::Emperor, " during this battle"],
                );
            }

            Faction::Guild => {
                game.current_guild_cunning = Some(self.clone());
                game.play_nexus_card(actor, elems!["perform an extra shipment after their move"]);
            }

            Faction::BeneGesserit => {
                game.current_bene_gesserit_cunning = Some(self.clone());
                game.play_nexus_card(
                    actor,
                    elems!["be able to flip advisor to fighters during ", MainPhase::ShipmentAndMove],
                );
            }

            Faction::Ixian => {
                game.current_ixian_cunning = Some(self.clone());
                game.play_nexus_card(actor, elems!["let ", FactionForce::Ixian, " be full strength during this battle"]);
            }

            Faction::Tleilaxu => {
                if game.traitor_deck.is_none() {
                    return;
                }
                let Some(tleilaxu) = self.get_player(game, Some(Faction::Tleilaxu)) else { return };

                let count = game.player(tleilaxu).revealed_face_dancers.len() as i32;
                game.play_nexus_card(actor, elems!["replace their ", count, " revealed face dancers"]);
                if game.player(tleilaxu).revealed_face_dancers.len() > 0 {
                    let mut i = 0;
                    while i < game.player(tleilaxu).revealed_face_dancers.len() {
                        let d = game.traitor_deck.as_mut().nre().draw();
                        game.player_mut(tleilaxu).face_dancers.push(d);
                        i += 1;
                    }

                    // HashSet iteration order (hazards 3)
                    let revealed: Vec<Hero> = game.player(tleilaxu).revealed_face_dancers.iter().copied().collect();
                    let traitor_deck = game.traitor_deck.as_mut().nre();
                    traitor_deck.items.extend(revealed.iter().copied());
                    traitor_deck.shuffle();

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

            Faction::Ecaz => {
                if game.vidal().is_some() && !game.is_alive(Hero::Leader(game.vidal().nre())) {
                    let vidal = game.vidal().nre();
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

    pub fn handle_secret_ally(self: &Rc<Self>, game: &mut Game) {
        let Some(actor) = self.player_opt() else { return };

        match self.faction {
            Faction::Atreides => {
                game.current_nexus_prescience = Some(self.clone());
                game.play_nexus_card(actor, elems!["see their opponent's ", self.atreides_prescience_aspect]);
            }

            Faction::Harkonnen => {
                if game.traitor_deck.is_none() {
                    return;
                }
                game.phase_before_discarding_traitor = game.current_phase;
                game.faction_that_must_discard_traitor = self.initiator();
                game.number_of_traitors_to_discard = 2;
                game.play_nexus_card(actor, elems!["draw two new traitors"]);
                let t = game.traitor_deck.as_mut().nre().draw();
                game.player_mut(actor).traitors.push(t);
                let t = game.traitor_deck.as_mut().nre().draw();
                game.player_mut(actor).traitors.push(t);
                game.enter(Phase::DiscardingTraitor, true);
            }

            Faction::Fremen => {
                game.current_fremen_secret_ally = Some(self.clone());
                if game.current_main_phase == MainPhase::Blow {
                    game.play_nexus_card(
                        actor,
                        elems!["prevent their forces from being devoured by ", Concept::Monster],
                    );
                } else if game.current_main_phase == MainPhase::Resurrection {
                    game.play_nexus_card(actor, elems!["increase their free revival to 3"]);
                }
            }

            Faction::Guild => {
                game.current_guild_secret_ally = Some(self.clone());
                game.play_nexus_card(actor, elems!["be able to ship as ", Faction::Guild]);
            }

            Faction::Tleilaxu => {
                game.stone(Milestone::RaiseDead);
                let Some(player) = self.get_player(game, Some(self.initiator())) else { return };

                Player::revive_forces(game, player, self.tleilaxu_forces);
                Player::revive_special_forces(game, player, self.tleilaxu_special_forces);

                if self.tleilaxu_special_forces > 0 {
                    game.factions_that_revived_special_forces_this_turn.push(self.initiator());
                }

                if let Some(tleilaxu_hero) = self.tleilaxu_hero() {
                    game.revive(actor, tleilaxu_hero);

                    if self.tleilaxu_assign_skill {
                        if let Some(revived_leader) = self.tleilaxu_hero().nre().as_leader() {
                            game.prepare_skill_assignment_to_revived_leader(player, Some(revived_leader));
                        }
                    }
                }

                let total = self.tleilaxu_forces + self.tleilaxu_special_forces;
                let force = game.player(player).force();
                let special_force = game.player(player).special_force();
                game.play_nexus_card(
                    actor,
                    elems![
                        "revive ",
                        express_part_if!(self.tleilaxu_hero().is_some(); self.tleilaxu_hero()),
                        express_part_if!(self.tleilaxu_hero().is_some() && total > 0; " and "),
                        express_part_if!(self.tleilaxu_forces > 0; self.tleilaxu_forces, " ", force),
                        express_part_if!(self.tleilaxu_forces > 0 && self.tleilaxu_special_forces > 0; " and "),
                        express_part_if!(self.tleilaxu_special_forces > 0; self.tleilaxu_special_forces, " ", special_force)
                    ],
                );

                if let Some(tleilaxu_location) = self.tleilaxu_location(game) {
                    if self.initiator() == Faction::Fremen {
                        Player::ship_special_forces(game, player, tleilaxu_location, 1);
                        let tleilaxu_location = self.tleilaxu_location(game);
                        self.log_objects(
                            game,
                            elems![self.initiator(), " place ", FactionSpecialForce::Fremen, " in ", tleilaxu_location],
                        );
                    }
                }
            }

            Faction::Choam => {
                if game.current_main_phase == MainPhase::Collection {
                    game.play_nexus_card(
                        actor,
                        elems!["discard a ", TreacheryCardType::Useless, " card to get ", Payment::of(2)],
                    );
                    let Some(choam_card) = self.choam_card() else { return };
                    // C# parity: the card is not validated here (hazards 18).
                    game.discard_player_treachery_card_boolean(Some(actor), Some(choam_card), true);
                    let me = game.player_mut(actor);
                    me.resources += 2;
                } else if game.current_phase == Phase::BattleConclusion {
                    let Some(current_battle) = game.current_battle.clone() else { return };

                    let Some(auditee) = current_battle.opponent_of_faction(game, self.initiator()) else { return };
                    let Some(recent_battle_plan) = current_battle.plan_of(game, Some(auditee)) else { return };
                    let weapon = recent_battle_plan.weapon();
                    let defense = recent_battle_plan.defense();
                    let hero = recent_battle_plan.hero();
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
                        let audited_card = auditable_cards.iter().copied().random_or_default(game.random.as_ref().nre());
                        if let Some(audited_card) = audited_card {
                            Game::register_known_player_treachery_card(game, actor, Some(audited_card));
                            self.log_to(game, self.initiator(), elems!["You see: ", audited_card]);
                            self.log_to(game, auditee_faction, elems!["You showed them: ", audited_card]);
                        }
                    } else {
                        self.log_objects(game, elems![auditee_faction, " don't have cards to audit"]);
                    }
                }
            }

            Faction::Ecaz => {
                game.play_nexus_card(
                    actor,
                    elems!["force ", self.ecaz_faction, " to reveal if they have an ", Faction::Ecaz, " traitor"],
                );
                self.log_objects(
                    game,
                    elems![self.ecaz_faction, " reveal to ", self.initiator(), " if they have a ", self.initiator(), " traitor"],
                );
                let initiator = self.initiator();
                let has_traitor = self
                    .get_player(game, Some(self.ecaz_faction))
                    .map(|p| game.player(p).traitors.iter().any(|t| t.faction() == initiator))
                    == Some(true);
                self.log_to(
                    game,
                    self.initiator(),
                    elems![
                        self.ecaz_faction,
                        " reveal that they ",
                        if has_traitor { "DO" } else { "DON'T" },
                        " have a ",
                        self.initiator(),
                        " traitor "
                    ],
                );
                self.log_to(
                    game,
                    self.ecaz_faction,
                    elems![
                        " you revealed to them that you ",
                        if has_traitor { "DO" } else { "DON'T" },
                        " have a ",
                        self.initiator(),
                        " traitor "
                    ],
                );
            }

            _ => {}
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " play a Nexus card")
    }
}

impl GameEventMethods for NexusPlayed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::NexusPlayed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::NexusPlayed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        NexusPlayed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        NexusPlayed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        NexusPlayed::get_message(self, game)
    }

}

impl crate::events::location_event::ILocationEvent for NexusPlayed {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self)
    }

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
