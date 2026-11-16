use crate::prelude::*;
use crate::messages::part::Part;

/// State of the Mentat Pause (contemplate) phase: extortion, terror and the game's winners.
pub struct ContemplateState {
    pub extortion_to_be_returned: bool,
    pub moritani_has_performed_terror: bool,
    pub winners: Vec<PlayerId>,
    pub win_method: Option<WinMethod>,
}

impl ContemplateState {
    pub fn new() -> Self {
        ContemplateState {
            extortion_to_be_returned: Default::default(),
            moritani_has_performed_terror: Default::default(),
            winners: Vec::new(),
            win_method: Default::default(),
        }
    }
}

impl Game {
    pub fn enter_contemplate_phase(&mut self) {
        for player in self.players.iter_mut() {
            player.transferable_resources = 0;
        }

        self.main_phase_start(MainPhase::Contemplate, true);
        self.allow_all_prevented_faction_advantages(&[]);
        self.handle_economics();
        self.add_bribes_to_player_resources();
        self.contemplate.moritani_has_performed_terror = false;

        for i in 0..self.players.len() {
            let p = self.player_at(i);
            if i32::from(self.player(p).banked_resources) > 0 {
                let banked = i32::from(self.player(p).banked_resources);
                {
                    let pl = self.player_mut(p);
                    crate::game::amounts::gain(&mut pl.resources, banked, "spice").expect("banked");
                }
                let faction = self.player(p).faction;
                let banked = i32::from(self.player(p).banked_resources);
                self.log(vec![
                    faction.into(),
                    " add ".into(),
                    Payment::of(banked).into(),
                    " received as ".into(),
                    LeaderSkill::Banker.into(),
                    " to their reserves".into(),
                ]);
                self.player_mut(p).banked_resources = 0;
            }
        }

        self.enter_contemplate_pause();
    }

    pub fn handle_economics(&mut self) {
        match self.charity.economics_status {
            Some(ChoamEconomicsStatus::Double) => {
                self.charity.economics_status = Some(ChoamEconomicsStatus::CancelFlipped);
                self.log(vec![Faction::Choam.into(), " The Economics Token flips to Cancel.".into()]);
                self.stone(Milestone::Economics);
            }

            Some(ChoamEconomicsStatus::Cancel) => {
                self.charity.economics_status = Some(ChoamEconomicsStatus::DoubleFlipped);
                self.log(vec![Faction::Choam.into(), " The Economics Token flips to Double.".into()]);
                self.stone(Milestone::Economics);
            }

            Some(ChoamEconomicsStatus::DoubleFlipped) | Some(ChoamEconomicsStatus::CancelFlipped) => {
                self.charity.economics_status = Some(ChoamEconomicsStatus::RemovedFromGame);
                self.log(vec![Faction::Choam.into(), " The Economics Token has been removed from the game.".into()]);
                self.stone(Milestone::Economics);
            }

            _ => {}
        }
    }

    pub fn enter_contemplate_pause(&mut self) {
        self.determine_if_moritani_draws_new_traitor();
        self.contemplate.extortion_to_be_returned = self.players.iter().any(|p| i32::from(p.extortion) > 0);
        self.gain_extortions();

        if self.turn.harkonnen_traitor_was_cancelled {
            self.turn.harkonnen_traitor_was_cancelled = false;
            // `?.` skips the draw when Harkonnen is not playing.
            if let Some(harkonnen) = self.get_player(Some(Faction::Harkonnen)) {
                let traitor = self.decks.traitor_deck.as_mut().expect("traitor deck").draw();
                self.player_mut(harkonnen).traitors.push(traitor);
            }
            self.log(vec![Faction::Harkonnen.into(), " drew a new traitor after having been betrayed".into()]);
        }

        let condition = self.contemplate.extortion_to_be_returned;
        self.enter_or_else(condition, Phase::Extortion, Game::end_contemplate_pause);
    }

    pub fn determine_if_moritani_draws_new_traitor(&mut self) {
        let Some(moritani) = self.get_player(Some(Faction::Moritani)) else { return };

        // ToList: snapshot before the loop body changes RevealedTraitors.
        let to_replace: Vec<Leader> = self
            .leaders.assassinated
            .iter()
            .copied()
            .filter(|l| self.player(moritani).revealed_traitors.contains(&Hero::Leader(*l)))
            .collect();
        for l in to_replace {
            self.log(vec![Faction::Moritani.into(), " draw a new traitor to replace ".into(), l.into()]);
            let hero = Hero::Leader(l);
            {
                let traitors = &mut self.player_mut(moritani).traitors;
                if let Some(i) = traitors.iter().position(|t| *t == hero) {
                    traitors.remove(i);
                }
            }
            self.player_mut(moritani).revealed_traitors.shift_remove(&hero);
            let traitor = self.decks.traitor_deck.as_mut().expect("traitor deck").draw();
            self.player_mut(moritani).traitors.push(traitor);
        }
    }

    pub fn end_contemplate_pause(&mut self) {
        if self.contemplate.extortion_to_be_returned {
            self.log(vec![Faction::Moritani.into(), " regain their ".into(), TerrorType::Extortion.into(), " token".into()]);
            self.board.unplaced_terror_tokens.push(TerrorType::Extortion);
        }

        self.enter(Phase::Contemplate, true);
        self.determine_occupation_at_start_or_end_of_turn();
    }

    pub fn gain_extortions(&mut self) {
        let players: Vec<PlayerId> =
            self.player_indices().filter(|p| i32::from(self.player(*p).extortion) > 0).collect();
        for p in players {
            let extortion = i32::from(self.player(p).extortion);
            {
                let pl = self.player_mut(p);
                crate::game::amounts::gain(&mut pl.resources, extortion, "spice").expect("extortion");
            }
            let faction = self.player(p).faction;
            let extortion = i32::from(self.player(p).extortion);
            self.log(vec![
                faction.into(),
                " add ".into(),
                Payment::of(extortion).into(),
                " gained from ".into(),
                TerrorType::Extortion.into(),
                " to their reserves".into(),
            ]);
            self.player_mut(p).extortion = 0;
        }
    }

    pub fn continue_contemplate_phase(&mut self) {
        self.check_normal_win();
        self.check_bene_gesserit_prediction();
        self.check_final_turn_win();

        if self.contemplate.winners.len() > 0 || self.turn.current_turn == self.maximum_turns() {
            self.turn.current_main_phase = MainPhase::Ended;
            self.enter(Phase::GameEnded, true);
            self.stone(Milestone::GameWon);
            let winner_factions: Vec<Faction> = self.contemplate.winners.iter().filter_map(|w| self.player(*w).faction).collect();
            self.log(vec![express!("The game has ended.").with_log(GameEnded::new(winner_factions)).into()]);

            let winners = self.contemplate.winners.clone();
            for w in winners {
                let faction = self.player(w).faction;
                self.log(vec![express!(faction, " win!").with_log(GameWinner::new(faction.expect("faction"))).into()]);
            }
        } else {
            let condition = self.is_playing(Faction::Tleilaxu) && !self.prevented(FactionAdvantage::TleilaxuReplacingFaceDancer);
            self.enter_either(condition, Phase::ReplacingFaceDancer, Phase::TurnConcluded);
        }

        self.determine_stronghold_ownership();

        if self.leaders.when_to_set_aside_vidal == Some(VidalMoment::EndOfTurn) {
            self.set_aside_vidal();
        }

        self.main_phase_end();
    }

    pub fn add_bribes_to_player_resources(&mut self) {
        for i in 0..self.players.len() {
            let p = self.player_at(i);
            if i32::from(self.player(p).bribes) > 0 {
                let bribes = i32::from(self.player(p).bribes);
                {
                    let pl = self.player_mut(p);
                    crate::game::amounts::gain(&mut pl.resources, bribes, "spice").expect("bribes");
                }
                let faction = self.player(p).faction;
                let bribes = i32::from(self.player(p).bribes);
                self.log(vec![express!(faction, " add ", Payment::of(bribes), " from bribes to their reserves")
                    .with_log(BribesCollected::new(faction.expect("faction"), bribes))
                    .into()]);
                self.player_mut(p).bribes = 0;
            }
        }
    }

    pub fn determine_stronghold_ownership(&mut self) {
        if self.applicable(Rule::StrongholdBonus) {
            let arrakeen = self.board.map.arrakeen();
            self.determine_stronghold_ownership_location(arrakeen);
            let carthag = self.board.map.carthag();
            self.determine_stronghold_ownership_location(carthag);
            let sietch_tabr = self.board.map.sietch_tabr();
            self.determine_stronghold_ownership_location(sietch_tabr);
            let habbanya_sietch = self.board.map.habbanya_sietch();
            self.determine_stronghold_ownership_location(habbanya_sietch);
            let tueks_sietch = self.board.map.tueks_sietch();
            self.determine_stronghold_ownership_location(tueks_sietch);
            let hms = self.board.map.hidden_mobile_stronghold().location();
            self.determine_stronghold_ownership_location(hms);
        }
    }

    pub fn determine_stronghold_ownership_location(&mut self, location: Location) {
        let current_owner = self.board.stronghold_ownership.get(&location).copied();
        let contested = self.applicable(Rule::ContestedStongholdsCountAsOccupied);
        let new_owning_player = self.players.iter().find(|p| p.controls(self, location, contested));
        let new_owner = new_owning_player.map_or(None, |p| p.faction);

        if current_owner != new_owner {
            if current_owner != None && new_owner != None {
                self.board.stronghold_ownership.remove(&location);
                self.board.stronghold_ownership.insert_new(location, new_owner.expect("faction"));
                self.log(vec![
                    new_owner.into(),
                    " take control over ".into(),
                    location.into(),
                    " from ".into(),
                    current_owner.into(),
                ]);
            } else if current_owner != None {
                self.board.stronghold_ownership.remove(&location);
                self.log(vec![current_owner.into(), " lose control over ".into(), location.into()]);
            } else if new_owner != None {
                self.board.stronghold_ownership.insert_new(location, new_owner.expect("faction"));
                self.log(vec![new_owner.into(), " take control over ".into(), location.into()]);
            }
        }
    }

    pub fn check_final_turn_win(&mut self) {
        if self.turn.current_turn != self.maximum_turns() {
            return;
        }

        if self.contemplate.winners.len() == 0 {
            self.check_special_win_conditions();
        }

        if self.contemplate.winners.len() != 0 {
            return;
        }

        self.check_other_win_conditions();
        self.check_bene_gesserit_prediction();
    }

    pub fn check_bene_gesserit_prediction(&mut self) {
        let bene_gesserit = self.get_player(Some(Faction::BeneGesserit));
        if let Some(bene_gesserit) = bene_gesserit {
            let predicted_faction = self.player(bene_gesserit).predicted_faction;
            let predicted_turn = self.player(bene_gesserit).predicted_turn;
            if predicted_turn == Some(self.turn.current_turn) && self.contemplate.winners.iter().any(|w| self.player(*w).faction == predicted_faction) {
                self.log_win_reason(
                    WinMethod::Prediction,
                    vec![
                        Faction::BeneGesserit.into(),
                        " predicted ".into(),
                        predicted_faction.into(),
                        " victory in turn ".into(),
                        self.turn.current_turn.into(),
                        "! They had everything planned...".into(),
                    ],
                );
                self.contemplate.win_method = Some(WinMethod::Prediction);
                self.contemplate.winners.clear();
                self.contemplate.winners.push(bene_gesserit);
            }
        }
    }

    pub fn check_normal_win(&mut self) {
        let mut check_win_sequence = PlayerSequence::starting_with_first_player(self);

        let mut i = 0;
        while i < self.players.len() {
            let p = check_win_sequence.current_player(self);

            let contested = self.applicable(Rule::ContestedStongholdsCountAsOccupied);
            let p = p.expect("player");
            if self.meets_normal_victory_condition(p, contested) {
                self.contemplate.win_method = Some(WinMethod::Strongholds);

                self.contemplate.winners.push(p);

                let ally = self.get_player(self.player(p).ally);
                if let Some(ally) = ally {
                    self.contemplate.winners.push(ally);
                }
                self.log_normal_win(p, ally);
            }

            if self.contemplate.winners.len() > 0 {
                break;
            }

            check_win_sequence.next_player(self);
            i += 1;
        }
    }

    pub fn log_normal_win(&mut self, player: PlayerId, ally: Option<PlayerId>) {
        let faction = self.player(player).faction;
        let player_ally = self.player(player).ally;
        let others_meet_condition = self.player_indices().any(|p| {
            !self.contemplate.winners.contains(&p)
                && self.meets_normal_victory_condition(p, self.applicable(Rule::ContestedStongholdsCountAsOccupied))
        });
        if others_meet_condition {
            if ally.is_none() {
                self.log_win_reason(
                    WinMethod::Strongholds,
                    vec![faction.into(), " are the first in front of the storm with enough victory points to win the game".into()],
                );
            } else {
                self.log_win_reason(
                    WinMethod::Strongholds,
                    vec![
                        faction.into(),
                        " and ".into(),
                        player_ally.into(),
                        " are the first in front of the storm with enough victory points to win the game".into(),
                    ],
                );
            }
        } else if ally.is_none() {
            self.log_win_reason(WinMethod::Strongholds, vec![faction.into(), " have enough victory points to win the game".into()]);
        } else {
            self.log_win_reason(
                WinMethod::Strongholds,
                vec![faction.into(), " and ".into(), player_ally.into(), " have enough victory points to win the game".into()],
            );
        }
    }

    pub fn log_win_reason(&mut self, method: WinMethod, expression: Vec<Part>) {
        self.log(vec![Message::express(expression).with_log(WinReason::new(method)).into()]);
    }

    pub fn meets_normal_victory_condition(&self, p: PlayerId, contested_strongholds_count_as_occupied: bool) -> bool {
        self.meets_ecaz_victory_condition(p, contested_strongholds_count_as_occupied)
            || self.meets_high_threshold_ecaz_victory_condition(p, contested_strongholds_count_as_occupied)
            || self.number_of_victory_points(p, contested_strongholds_count_as_occupied) >= self.threshold_for_win(p)
    }

    pub fn count_challenged_victory_points(&self, p: PlayerId) -> i32 {
        self.number_of_victory_points(p, true) - self.number_of_victory_points(p, false)
    }

    pub fn threshold_for_win(&self, p: PlayerId) -> i32 {
        if self.player(p).ally != None {
            return 4;
        }
        if self.players.len() <= 2 { 4 } else { 3 }
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn strongholds(&self) -> impl Iterator<Item = Territory> {
        self.board.map.territories(false).filter(move |t| t.is_stronghold() || self.is_special_stronghold(*t))
    }

    pub fn meets_ecaz_victory_condition(&self, p: PlayerId, contested_strongholds_count_as_occupied: bool) -> bool {
        let pl = self.player(p);
        ((pl.is(Faction::Ecaz) && pl.has_ally()) || pl.ally == Some(Faction::Ecaz))
            && self
                .strongholds()
                .filter(|t| {
                    pl.controls_territory(self, *t, contested_strongholds_count_as_occupied)
                        && self
                            .player(pl.allied_player(self).expect("allied player"))
                            .controls_territory(self, *t, contested_strongholds_count_as_occupied)
                })
                .count()
                >= 3
    }

    pub fn meets_high_threshold_ecaz_victory_condition(&self, p: PlayerId, contested_strongholds_count_as_occupied: bool) -> bool {
        let pl = self.player(p);
        self.has_high_threshold(Faction::Ecaz)
            && ((pl.is(Faction::Ecaz) && pl.has_ally()) || pl.ally == Some(Faction::Ecaz))
            && self.strongholds().any(|t| {
                pl.controls_territory(self, t, contested_strongholds_count_as_occupied)
                    && self
                        .player(pl.allied_player(self).expect("allied player"))
                        .controls_territory(self, t, contested_strongholds_count_as_occupied)
            })
            && self.board.homeworld_occupation.values().filter(|f| Some(**f) == pl.faction || Some(**f) == pl.ally).count() >= 2
    }

    pub fn number_of_victory_points(&self, p: PlayerId, contested_strongholds_count_as_occupied: bool) -> i32 {
        let pl = self.player(p);
        if let Some(allied) = pl.allied_player(self) {
            let allied = self.player(allied);
            let tech_token_point = if pl.tech_tokens.len() == 3 || allied.tech_tokens.len() == 3 { 1 } else { 0 };
            let count = self
                .board.map
                .territories(false)
                .filter(|t| t.is_stronghold() || self.is_special_stronghold(*t))
                .filter(|l| {
                    pl.controls_territory(self, *l, contested_strongholds_count_as_occupied)
                        || allied.controls_territory(self, *l, contested_strongholds_count_as_occupied)
                })
                .count() as i32;
            tech_token_point + count
        } else {
            let tech_token_point = if pl.tech_tokens.len() == 3 { 1 } else { 0 };
            tech_token_point + self.number_of_occupied_strongholds(p, contested_strongholds_count_as_occupied)
        }
    }

    pub fn number_of_occupied_strongholds(&self, p: PlayerId, contested_strongholds_count_as_occupied: bool) -> i32 {
        let pl = self.player(p);
        self.board.map
            .territories(false)
            .filter(|t| t.is_stronghold() || self.is_special_stronghold(*t))
            .filter(|l| pl.controls_territory(self, *l, contested_strongholds_count_as_occupied))
            .count() as i32
    }

    pub fn check_special_win_conditions(&mut self) {
        let fremen = self.get_player(Some(Faction::Fremen));
        let guild = self.get_player(Some(Faction::Guild));

        if fremen.is_some() && self.fremen_victory_condition_met() {
            let fremen = fremen.expect("fremen");
            self.log_win_reason(WinMethod::FremenSpecial, vec![Faction::Fremen.into(), " special victory conditions are met!".into()]);
            self.contemplate.win_method = Some(WinMethod::FremenSpecial);
            self.contemplate.winners.push(fremen);
            if self.player(fremen).has_ally() {
                // The ally faction may not be playing (C# then adds a null winner).
                if let Some(ally) = self.player(fremen).allied_player(self) {
                    self.contemplate.winners.push(ally);
                }
            }
        } else if guild.is_some() && !self.applicable(Rule::DisableGuildSpecialVictory) {
            let guild = guild.expect("guild");
            self.log_win_reason(WinMethod::GuildSpecial, vec![Faction::Guild.into(), " special victory conditions are met!".into()]);
            self.contemplate.win_method = Some(WinMethod::GuildSpecial);
            self.contemplate.winners.push(guild);
            if self.player(guild).has_ally() {
                if let Some(ally) = self.player(guild).allied_player(self) {
                    self.contemplate.winners.push(ally);
                }
            }
        } else if fremen.is_some() && !self.applicable(Rule::DisableGuildSpecialVictory) {
            let fremen = fremen.expect("fremen");
            self.log_win_reason(
                WinMethod::GuildSpecial,
                vec![
                    Faction::Fremen.into(),
                    " win because ".into(),
                    Faction::Guild.into(),
                    " are not playing and no one else won".into(),
                ],
            );
            self.contemplate.win_method = Some(WinMethod::GuildSpecial);
            self.contemplate.winners.push(fremen);
            if self.player(fremen).has_ally() {
                if let Some(ally) = self.player(fremen).allied_player(self) {
                    self.contemplate.winners.push(ally);
                }
            }
        }
    }

    pub fn fremen_victory_condition_met(&self) -> bool {
        let sietch_tabr = self.board.map.sietch_tabr();
        let habbanya_sietch = self.board.map.habbanya_sietch();
        let tueks_sietch = self.board.map.tueks_sietch();
        let sietch_tabr_occupied_by_other_than_fremen = self.players.iter().any(|p| {
            p.faction != Some(Faction::Fremen)
                && (p.faction != Some(Faction::Ecaz) || p.ally != Some(Faction::Fremen))
                && p.occupies(self, Some(sietch_tabr))
        });
        let habbanya_sietch_occupied_by_other_than_fremen = self.players.iter().any(|p| {
            p.faction != Some(Faction::Fremen)
                && (p.faction != Some(Faction::Ecaz) || p.ally != Some(Faction::Fremen))
                && p.occupies(self, Some(habbanya_sietch))
        });
        let tueks_sietch_occupied_by_atreides_or_harkonnen_or_emperor_or_richese = self.players.iter().any(|p| {
            (p.is(Faction::Atreides) || p.is(Faction::Harkonnen) || p.is(Faction::Emperor) || p.is(Faction::Richese))
                && p.occupies(self, Some(tueks_sietch))
        });

        self.turn.current_turn == self.maximum_turns()
            && !sietch_tabr_occupied_by_other_than_fremen
            && !habbanya_sietch_occupied_by_other_than_fremen
            && !tueks_sietch_occupied_by_atreides_or_harkonnen_or_emperor_or_richese
    }

    pub fn check_other_win_conditions(&mut self) {
        let mut with_most_points: Option<PlayerId> = None;
        let mut highest_number_of_points = -1;
        let mut check_win_sequence = PlayerSequence::starting_with_first_player(self);

        for _ in 0..self.players.len() {
            // A null CurrentPlayer throws inside NumberOfVictoryPoints.
            let p = check_win_sequence.current_player(self).expect("current player");

            let points_of_player = self.number_of_victory_points(p, self.applicable(Rule::ContestedStongholdsCountAsOccupied));
            if points_of_player > highest_number_of_points {
                with_most_points = Some(p);
                highest_number_of_points = points_of_player;
            }

            check_win_sequence.next_player(self);
        }

        if let Some(with_most_points) = with_most_points {
            self.contemplate.winners.push(with_most_points);
            self.contemplate.win_method = Some(WinMethod::Strongholds);
            let faction = self.player(with_most_points).faction;
            self.log_win_reason(
                WinMethod::Strongholds,
                vec![faction.into(), " are the first after the storm with most victory points".into()],
            );

            if self.player(with_most_points).has_ally() {
                if let Some(ally) = self.player(with_most_points).allied_player(self) {
                    self.contemplate.winners.push(ally);
                }
            }
        }
    }

    pub fn has_stronghold_advantage(&self, f: impl Into<Option<Faction>>, advantage: StrongholdAdvantage, battle_territory: Territory) -> bool {
        let f: Option<Faction> = f.into();
        let hms = self.board.map.hidden_mobile_stronghold().location();
        if battle_territory == hms.territory() && self.owns_stronghold(f, hms) && self.battle.chosen_hms_advantage == Some(Some(advantage)) {
            return true;
        }

        battle_territory.advantage() == Some(advantage) && battle_territory.locations().iter().any(|l| self.owns_stronghold(f, *l))
    }

    pub fn owns_stronghold(&self, f: impl Into<Option<Faction>>, stronghold: Location) -> bool {
        let f: Option<Faction> = f.into();
        match self.board.stronghold_ownership.get(&stronghold) {
            Some(owner) => Some(*owner) == f,
            None => false,
        }
    }

    pub fn is_special_stronghold(&self, t: Territory) -> bool {
        t == self.board.map.shield_wall() && self.applicable(Rule::Ssw) && self.board.number_of_monsters >= 4
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn terror_in(&self, t: Territory) -> impl Iterator<Item = TerrorType> {
        self.board.terror_on_planet.iter().filter(move |(_, v)| **v == t).map(move |(k, _)| *k)
    }
}

impl GameStatus {
    /// The status in the Mentat phase and the end of the game, or `None` in another phase.
    pub(crate) fn contemplate_status(_game: &Game, _me: Option<PlayerId>, phase: Phase) -> Option<GameStatus> {
        Some(match phase {
            Phase::Extortion => Self::host_status(
                express!("You may proceed when done waiting for players to avoid future {0} by {1}", TerrorType::Extortion, Faction::Moritani),
                express!("Players may pay {0} to avoid future {1} by {2}...", Concept::Resource, TerrorType::Extortion, Faction::Moritani),
                None,
            ),

            Phase::Contemplate => Self::host_status("Check for victories?", "Waiting for the host to start determining victories...", None),

            Phase::GameEnded => Self::status("The game has ended.", None),

            _ => return None,
        })
    }
}

impl Game {
    /// What `player` may do in the Mentat phase and the end of the game (see `add_player_actions`).
    pub(crate) fn add_contemplate_actions(&self, player: PlayerId, _is_host: bool, result: &mut Vec<EventType>) {
        let faction = self.player(player).faction;
        match self.phase() {
            Phase::Extortion => {
                if ExtortionPrevented::can_be_played(self, player) {
                    result.push(EventType::ExtortionPrevented);
                }
            }

            Phase::Contemplate => {
                if faction == Some(Faction::Choam)
                    && !self.prevented(FactionAdvantage::ChoamEconomics)
                    && self.charity.economics_status == None
                {
                    result.push(EventType::ChoamEconomics);
                }
                if faction == Some(Faction::Moritani) && TerrorPlanted::is_applicable(self, player) {
                    result.push(EventType::TerrorPlanted);
                }
            }

            _ => {}
        }
    }
}
