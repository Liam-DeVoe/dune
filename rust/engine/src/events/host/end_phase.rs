use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct EndPhase {
}

impl EndPhase {
    pub fn continue_to_revivals(&self, game: &mut Game) -> GameResult {
        game.enter_revival(RevivalStep::Reviving);

        // foreach over a lazy Where: the filter runs right before each body
        for i in 0..game.players.len() {
            let p = game.player_at(i);
            if game.is_automated(AutomationRuleType::RevivalAutoClaimFreeRevival, p) {
                game.claim_free_revival(p)?;
            }
        }
        Ok(())
    }

    pub fn establish_decks(&self, game: &mut Game) -> GameResult {
        if game.is_playing(Faction::Richese) {
            game.decks.richese_cache = TreacheryCardManager::get_richese_cards();
        }

        let condition = game.applicable(Rule::CustomDecks);
        game.enter_or_else_try(condition, Phase::CustomizingDecks, Game::enter_setup_phase)?;
        Ok(())
    }

    pub fn enter_normal_storm_phase(&self, game: &mut Game) -> GameResult {
        let m = express!("The storm moves ", game.board.next_storm_moves, " sectors")
            .with_log(StormMoved::new(game.board.sector_in_storm, game.board.next_storm_moves, None, None));
        game.log(vec![m.into()]);
        let moves = game.board.next_storm_moves;
        game.move_storm_and_determine_next(moves)?;
        game.end_storm_phase();
        Ok(())
    }

    pub fn enter_charity_phase(&self, game: &mut Game) {
        game.main_phase_start(MainPhase::Charity, true);
        game.turn.has_acted_or_passed.clear();
        game.blow.monsters.clear();
        game.allow(FactionAdvantage::FremenControlsMonster);
        game.allow(FactionAdvantage::FremenProtectedFromMonster);

        game.begin_charity();
    }

    pub fn start_claiming_charity(&self, game: &mut Game) {
        if !game.prevented(FactionAdvantage::ChoamControllingCharity) {
            let choam = game.get_player(Some(Faction::Choam));
            if let Some(choam) = choam {
                let to_collect = (game.players.len() as i32) * 2 * game.current_charity_multiplier();
                let b = game.player_mut(choam);
                crate::game::amounts::gain(&mut b.resources, to_collect, "spice").expect("CHOAM's charity");
                game.log(vec![Faction::Choam.into(), " collect ".into(), Payment::of(to_collect).into()]);
            }
        } else {
            game.log_prevention_by_karma(FactionAdvantage::ChoamControllingCharity);
        }

        let bene_gesserit = game.get_player(Some(Faction::BeneGesserit));
        if let Some(bene_gesserit) = bene_gesserit {
            if game.applicable(Rule::BeneGesseritAutoCharity) {
                if !game.charity_is_cancelled() {
                    if !game.prevented(FactionAdvantage::BeneGesseritCharity) {
                        game.turn.has_acted_or_passed.push(Faction::BeneGesserit);
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
        // paid earlier in the loop can make a later player eligible
        for i in 0..game.players.len() {
            let p = game.player_at(i);
            if CharityClaimed::can_be_played(game, p) && game.is_automated(AutomationRuleType::CharityAutoClaim, p) {
                game.claim_charity(p);
            }
        }

        game.main_phase_middle();
        game.enter_charity(CharityStep::Claiming);
    }

    pub fn end_charity_phase(&self, game: &mut Game) {
        if game.charity().expect("in the charity phase").resource_tech_token_income {
            game.receive_tech_income(TechToken::Resources);
        }

        game.main_phase_end();
        game.enter_charity(CharityStep::Report);
    }

    pub fn enter_revival_phase(&self, game: &mut Game) {
        game.leave_bidding();
        game.main_phase_start(MainPhase::Resurrection, true);
        game.allow(FactionAdvantage::HarkonnenFreeCard);
        game.allow(FactionAdvantage::EmperorReceiveBid);
        game.allow(FactionAdvantage::IxianAllyDiscardingCard);
        game.turn.revivals.ambassadors_placed_this_turn = 0;
        game.turn.revivals.free_revivals_this_turn.clear();
        game.turn.revivals.total_revivals_this_turn.clear();
        game.turn.revivals.leader_revivals_this_turn.clear();
        //Game.HasActedOrPassed.Clear();

        game.begin_revival();
    }

    pub fn end_resurrection_phase(&self, game: &mut Game) {
        game.allow(FactionAdvantage::TleilaxuReviveGhola);
        if game.revival().expect("in the revival phase").revival_tech_token_income {
            game.receive_tech_income(TechToken::Graveyard);
        }
        game.turn.revivals.current_karma_revival_prevention = None;
        game.turn.current_fremen_secret_ally = None;
        game.revival_mut().expect("in the revival phase").current_recruits_played = false;

        game.main_phase_end();
        game.enter_revival(RevivalStep::Report);
    }

    pub fn enter_shipment_and_move_phase(&self, game: &mut Game) {
        game.main_phase_start(MainPhase::ShipmentAndMove, true);
        game.ship_move.factions_with_ornithopters_at_start_of_movement = (0..game.players.len())
            .map(|i| game.player_at(i))
            .filter(|p| game.occupies_arrakeen_or_carthag(*p))
            .filter_map(|p| game.player(p).faction)
            .collect();
        game.ship_move.recent_moves.clear();
        game.ship_move.beginning_of_shipment_and_move_phase = true;
        game.turn.revivals.factions_with_increased_revival_limits = Some(Vec::new());

        game.ship_move.ships_tech_token_income = false;
        game.allow(FactionAdvantage::BeneGesseritAnnouncesBattle);
        game.allow(FactionAdvantage::EmperorLetAllyReviveExtraForces);
        game.allow(FactionAdvantage::TleilaxuReceiveRevive);
        game.allow(FactionAdvantage::ChoamRevival);

        game.turn.has_acted_or_passed.clear();
        game.ship_move.last_shipment_or_movement = None;

        game.ship_move.shipment_and_move_sequence = Some(PlayerSequence::starting_with_first_player(game));

        game.enter(Phase::BeginningOfShipAndMove, true);
    }

    pub fn enter_battle_phase(&self, game: &mut Game) {
        game.main_phase_start(MainPhase::Battle, true);
        game.turn.current_guild_cunning = None;
        game.turn.current_guild_secret_ally = None;
        if game.storm.karma_hms_moves_left != 2 {
            game.storm.karma_hms_moves_left = 0;
        }
        self.reset_battle(game);
        if !game.begin_battle() {
            enter_spice_collection_phase(game);
        }
    }

    /// Resets the turn's state that lasts one battle. The battle's own state is dropped with it
    /// (`BattlePhase::current`).
    pub fn reset_battle(&self, game: &mut Game) {
        game.turn.facedancer_was_cancelled = false;
        game.turn.current_nexus_prescience = None;
        game.turn.current_bene_gesserit_cunning = None;
        game.turn.current_emperor_cunning = None;
        game.turn.current_ixian_cunning = None;
        game.turn.has_acted_or_passed.clear();
    }

    pub fn enter_spice_collection_phase(&self, game: &mut Game) {
        enter_spice_collection_phase(game)
    }

    pub fn call_heroes_home(&self, game: &mut Game) {
        call_heroes_home(game)
    }
}

impl EventRules for EndPhase {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, _by: By) -> GameResult {
        match game.phase() {
            Phase::SelectingFactions => game.assign_factions_and_enter_faction_trade(),

            Phase::TradingFactions => self.establish_decks(game)?,

            Phase::BeginningOfStorm => game.move_hms_before_dialling_storm(),

            Phase::MetheorAndStormSpell => self.enter_normal_storm_phase(game)?,

            Phase::StormReport => game.enter_spice_blow_phase(),

            Phase::Thumper => game.enter_blow_a()?,

            Phase::HarvesterA | Phase::HarvesterB => game.move_to_next_phase_after_resource_blow()?,

            Phase::AllianceA | Phase::AllianceB => game.end_nexus()?,

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
                let b = game.bidding().expect("in the bidding phase");
                if b.card_just_won == b.card_sold_on_black_market {
                    game.enter_richese_bidding();
                } else {
                    game.determine_next_step_after_card_was_sold();
                }
            }

            Phase::WaitingForNextBiddingRound => game.put_next_card_on_auction(),

            Phase::BiddingReport => self.enter_revival_phase(game),

            Phase::BeginningOfResurrection => self.continue_to_revivals(game)?,

            Phase::Resurrection => self.end_resurrection_phase(game),

            Phase::ResurrectionReport => self.enter_shipment_and_move_phase(game),

            Phase::BeginningOfShipAndMove => game.start_ship_and_move_sequence()?,

            Phase::ShipmentAndMoveConcluded => self.enter_battle_phase(game),

            Phase::BeginningOfBattle => game.enter_battle(BattleStep::Planning),

            Phase::CancellingTraitor => {
                game.enter_battle(BattleStep::CallingTraitor);
                game.handle_revealed_battle_plans()?;
            }

            Phase::BattleReport => {
                self.reset_battle(game);
                if game.next_battle_or_end() {
                    game.enter_battle(BattleStep::Planning);
                } else {
                    enter_spice_collection_phase(game);
                }
            }

            Phase::BeginningOfCollection => game.start_collection(),

            Phase::CollectionReport => game.enter_contemplate_phase(),

            Phase::Extortion => game.end_contemplate_pause(),

            Phase::Contemplate => game.continue_contemplate_phase(),

            Phase::TurnConcluded => {
                game.enter_storm_phase()?;
                game.flip_bene_gesserit_advisors_when_alone()?;
            }

            _ => {}
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!("Phase ended by ", by.faction)
    }
}


/// `EnterSpiceCollectionPhase`. A free function so it can be passed to `Game.Enter` as
/// the action (it only touches `Game`).
fn enter_spice_collection_phase(game: &mut Game) {
    game.main_phase_start(MainPhase::Collection, true);
    call_heroes_home(game);
    game.board.pending_discoveries = game
        .board.discoveries_on_planet
        .iter()
        .filter(|kvp| game.any_forces_in(kvp.0.territory()))
        .map(|kvp| kvp.1.token)
        .collect();

    game.begin_collection();
}

fn call_heroes_home(game: &mut Game) {
    for ls in game.leaders.leader_state.values_mut() {
        ls.current_territory = None;
    }
}
