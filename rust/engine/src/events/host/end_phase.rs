use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct EndPhase {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for EndPhase {
    fn default() -> Self {
        EndPhase {
            base: Default::default(),
        }
    }
}

impl EndPhase {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        match game.current_phase {
            Phase::SelectingFactions => game.assign_factions_and_enter_faction_trade(),

            Phase::TradingFactions => self.establish_decks(game),

            Phase::BeginningOfStorm => game.move_hms_before_dialling_storm(),

            Phase::MetheorAndStormSpell => self.enter_normal_storm_phase(game),

            Phase::StormReport => game.enter_spice_blow_phase(),

            Phase::Thumper => game.enter_blow_a(),

            Phase::HarvesterA | Phase::HarvesterB => game.move_to_next_phase_after_resource_blow(),

            Phase::AllianceA | Phase::AllianceB => game.end_nexus(),

            Phase::BlowReport => {
                if game.applicable(Rule::HasCharityPhase) {
                    self.enter_charity_phase(game);
                } else {
                    game.enter_bidding_phase();
                }
            }

            Phase::BeginningOfCharity => self.start_claiming_charity(game),

            Phase::ClaimingCharity => self.end_charity_phase(game),

            Phase::CharityReport => game.enter_bidding_phase(),

            Phase::BeginningOfBidding => game.start_bidding_phase(),

            Phase::ReplacingCardJustWon => {
                if game.card_just_won == game.card_sold_on_black_market {
                    game.enter_richese_bidding();
                } else {
                    game.determine_next_step_after_card_was_sold();
                }
            }

            Phase::WaitingForNextBiddingRound => game.put_next_card_on_auction(),

            Phase::BiddingReport => self.enter_revival_phase(game),

            Phase::BeginningOfResurrection => self.continue_to_revivals(game),

            Phase::Resurrection => self.end_resurrection_phase(game),

            Phase::ResurrectionReport => self.enter_shipment_and_move_phase(game),

            Phase::BeginningOfShipAndMove => game.start_ship_and_move_sequence(),

            Phase::ShipmentAndMoveConcluded => self.enter_battle_phase(game),

            Phase::BeginningOfBattle => game.enter(Phase::BattlePhase, true),

            Phase::CancellingTraitor => {
                game.enter(Phase::CallTraitorOrPass, true);
                game.handle_revealed_battle_plans()?;
            }

            Phase::BattleReport => {
                self.reset_battle(game);
                let condition = game.next_player_to_battle().is_some();
                game.enter_boolean_phase_action(condition, Phase::BattlePhase, enter_spice_collection_phase);
            }

            Phase::BeginningOfCollection => game.start_collection(),

            Phase::CollectionReport => game.enter_contemplate_phase(),

            Phase::Extortion => game.end_contemplate_pause(),

            Phase::Contemplate => game.continue_contemplate_phase(),

            Phase::TurnConcluded => {
                game.enter_storm_phase();
                game.flip_bene_gesserit_advisors_when_alone();
            }

            _ => {}
        }
        Ok(())
    }

    pub fn continue_to_revivals(&self, game: &mut Game) {
        game.enter(Phase::Resurrection, true);

        // foreach over a lazy Where: the filter runs right before each body (hazards 4.1)
        for i in 0..game.players.len() {
            let p = game.player_at(i);
            if game.is_automated(AutomationRuleType::RevivalAutoClaimFreeRevival, p) {
                game.claim_free_revival(p);
            }
        }
    }

    pub fn establish_decks(&self, game: &mut Game) {
        if game.is_playing(Faction::Richese) {
            game.richese_cache = TreacheryCardManager::get_richese_cards();
        }

        let condition = game.applicable(Rule::CustomDecks);
        game.enter_boolean_phase_action(condition, Phase::CustomizingDecks, Game::enter_setup_phase);
    }

    pub fn enter_normal_storm_phase(&self, game: &mut Game) {
        let m = express!("The storm moves ", game.next_storm_moves, " sectors")
            .with_log(StormMoved::new(game.sector_in_storm, game.next_storm_moves, Faction::None, None));
        self.log_objects(game, vec![m.into()]);
        let moves = game.next_storm_moves;
        game.move_storm_and_determine_next(moves);
        game.end_storm_phase();
    }

    pub fn enter_charity_phase(&self, game: &mut Game) {
        game.main_phase_start(MainPhase::Charity, true);
        game.has_acted_or_passed.clear();
        game.monsters.clear();
        game.resource_tech_token_income = false;
        game.allow(FactionAdvantage::FremenControlsMonster);
        game.allow(FactionAdvantage::FremenProtectedFromMonster);

        game.enter(Phase::BeginningOfCharity, true);
    }

    pub fn start_claiming_charity(&self, game: &mut Game) {
        if !game.prevented(FactionAdvantage::ChoamControllingCharity) {
            let choam = self.get_player(game, Some(Faction::Choam));
            if let Some(choam) = choam {
                let to_collect = (game.players.len() as i32) * 2 * game.current_charity_multiplier();
                let b = game.player_mut(choam);
                b.resources += to_collect;
                self.log_objects(game, vec![Faction::Choam.into(), " collect ".into(), Payment::of(to_collect).into()]);
            }
        } else {
            game.log_prevention_by_karma(FactionAdvantage::ChoamControllingCharity);
        }

        let bene_gesserit = self.get_player(game, Some(Faction::BeneGesserit));
        if let Some(bene_gesserit) = bene_gesserit {
            if game.applicable(Rule::BeneGesseritAutoCharity) {
                if !game.charity_is_cancelled() {
                    if !game.prevented(FactionAdvantage::BeneGesseritCharity) {
                        game.has_acted_or_passed.push(Faction::BeneGesserit);
                        let amount = 2i32 * game.current_charity_multiplier();
                        game.give_charity(bene_gesserit, amount);
                        game.stone(Milestone::CharityClaimed);
                    } else {
                        game.log_prevention_by_karma(FactionAdvantage::BeneGesseritCharity);
                        if !game.applicable(Rule::FullPhaseKarma) {
                            game.allow(FactionAdvantage::BeneGesseritCharity);
                        }
                    }
                }
            }
        }

        // foreach over a lazy Where: the filter runs right before each body, so a claim
        // paid earlier in the loop can make a later player eligible (hazards 4.1)
        for i in 0..game.players.len() {
            let p = game.player_at(i);
            if CharityClaimed::can_be_played(game, p) && game.is_automated(AutomationRuleType::CharityAutoClaim, p) {
                game.claim_charity(p);
            }
        }

        game.main_phase_middle();
        game.enter(Phase::ClaimingCharity, true);
    }

    pub fn end_charity_phase(&self, game: &mut Game) {
        if game.resource_tech_token_income {
            game.receive_tech_income(TechToken::Resources);
        }

        game.main_phase_end();
        game.enter(Phase::CharityReport, true);
    }

    pub fn enter_revival_phase(&self, game: &mut Game) {
        game.card_just_won = None;
        game.main_phase_start(MainPhase::Resurrection, true);
        game.allow(FactionAdvantage::HarkonnenFreeCard);
        game.allow(FactionAdvantage::EmperorReceiveBid);
        game.allow(FactionAdvantage::IxianAllyDiscardingCard);
        game.revival_tech_token_income = false;
        game.ambassadors_placed_this_turn = 0;
        game.free_revivals_this_turn.clear();
        game.total_revivals_this_turn.clear();
        game.leader_revivals_this_turn.clear();
        //Game.HasActedOrPassed.Clear();
        game.tleilaxu_started_revival_with_low_threshold = game.has_low_threshold(Faction::Tleilaxu);

        game.enter(Phase::BeginningOfResurrection, true);
    }

    pub fn end_resurrection_phase(&self, game: &mut Game) {
        game.allow(FactionAdvantage::TleilaxuReviveGhola);
        if game.revival_tech_token_income {
            game.receive_tech_income(TechToken::Graveyard);
        }
        game.current_karma_revival_prevention = None;
        game.current_fremen_secret_ally = None;
        game.current_recruits_played = None;

        game.main_phase_end();
        game.enter(Phase::ResurrectionReport, true);
    }

    pub fn enter_shipment_and_move_phase(&self, game: &mut Game) {
        game.main_phase_start(MainPhase::ShipmentAndMove, true);
        game.factions_with_ornithopters_at_start_of_movement = (0..game.players.len())
            .map(|i| game.player_at(i))
            .filter(|p| game.occupies_arrakeen_or_carthag(*p))
            .map(|p| game.player(p).faction)
            .collect();
        game.recent_moves.clear();
        game.beginning_of_shipment_and_move_phase = true;
        game.factions_with_increased_revival_limits = Some(Vec::new());
        game.early_revivals_offers.clear();

        game.ships_tech_token_income = false;
        game.current_free_revival_prevention = None;
        game.allow(FactionAdvantage::BeneGesseritAnnouncesBattle);
        game.allow(FactionAdvantage::EmperorLetAllyReviveExtraForces);
        game.allow(FactionAdvantage::TleilaxuReceiveRevive);
        game.allow(FactionAdvantage::ChoamRevival);

        game.has_acted_or_passed.clear();
        game.last_shipment_or_movement = None;

        game.shipment_and_move_sequence = Some(PlayerSequence::new_game(game));

        game.enter(Phase::BeginningOfShipAndMove, true);
    }

    pub fn enter_battle_phase(&self, game: &mut Game) {
        game.main_phase_start(MainPhase::Battle, true);
        game.nr_of_battles_fought = 0;
        game.battle_sequence = Some(PlayerSequence::new_game(game));
        game.current_guild_cunning = None;
        game.current_guild_secret_ally = None;
        if game.karma_hms_moves_left != 2 {
            game.karma_hms_moves_left = 0;
        }
        self.reset_battle(game);
        // Arguments are evaluated before the call (NextPlayerToBattle mutates BattleSequence).
        if game.next_player_to_battle().is_none() {
            enter_spice_collection_phase(game);
        } else {
            game.enter(Phase::BeginningOfBattle, true);
        }
    }

    pub fn reset_battle(&self, game: &mut Game) {
        game.facedancer_was_cancelled = false;
        game.current_battle = None;
        game.current_prescience = None;
        game.prescience_seen_plans.clear();
        game.current_thought = None;
        game.current_voice = None;
        game.current_nexus_prescience = None;
        game.current_bene_gesserit_cunning = None;
        game.current_emperor_cunning = None;
        game.current_ixian_cunning = None;
        game.harkonnen_victim = None;
        game.aggressor_plan = None;
        game.defender_plan = None;
        game.previous_aggressor_plan = None;
        game.previous_defender_plan = None;
        game.aggressor_traitor_action = None;
        game.defender_traitor_action = None;
        game.poison_tooth_cancelled = false;
        game.ixian_special_force_losses_to_take = 0;
        game.battle_winner = Some(Faction::None);
        game.battle_loser = Some(Faction::None);
        game.has_acted_or_passed.clear();
        game.battle_triggered_bureaucracy = None;
        game.cards_to_be_discarded_by_loser_after_battle.clear();
        game.battle_was_concluded_by_winner = false;
        game.loser_may_try_to_assassinate = false;
        game.battle_winner_may_choose_to_discard = true;
        game.battle_about_to_start = None;
        game.current_ecaz_or_ally_fighter = Some(Faction::None);
        game.current_ecaz_battle_contribution = 0;
        game.atreides_karma = false;
        game.ecaz_karma_bonus = 0;
    }

    pub fn enter_spice_collection_phase(&self, game: &mut Game) {
        enter_spice_collection_phase(game)
    }

    pub fn call_heroes_home(&self, game: &mut Game) {
        call_heroes_home(game)
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!("Phase ended by ", self.initiator())
    }

}

impl GameEventMethods for EndPhase {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::EndPhase
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::EndPhase(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        EndPhase::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        EndPhase::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        EndPhase::get_message(self, game)
    }

}

/// `EnterSpiceCollectionPhase`. A free function so it can be passed to `Game.Enter` as
/// the action (it only touches `Game`).
fn enter_spice_collection_phase(game: &mut Game) {
    game.main_phase_start(MainPhase::Collection, true);
    game.resources_collected_by_fremen = 0;
    game.resources_collected_by_harkonnen_from_desert_or_homeworld = 0;
    call_heroes_home(game);
    game.pending_discoveries = game
        .discoveries_on_planet
        .iter()
        .filter(|kvp| game.any_forces_in(kvp.0.territory()))
        .map(|kvp| kvp.1.token)
        .collect();

    game.enter(Phase::BeginningOfCollection, true);
}

/// `CallHeroesHome`
fn call_heroes_home(game: &mut Game) {
    for ls in game.leader_state.values_mut() {
        ls.current_territory = None;
    }
}
