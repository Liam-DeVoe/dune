use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct BattleConcluded {
    pub kill: bool,
    pub capture: bool,
    pub traitor_to_replace: Option<Hero>,
    pub new_traitor: Option<Hero>,
    pub special_force_losses_replaced: u16,
    #[serde(with = "crate::events::wire::id_list")]
    #[ts(as = "Option<Vec<i32>>")]
    pub card_ids: Option<String>,
    pub stolen_token: Option<TechToken>,
    pub add_extra_force: bool,
}

impl Default for BattleConcluded {
    fn default() -> Self {
        BattleConcluded {
            kill: Default::default(),
            capture: true,
            traitor_to_replace: None,
            new_traitor: None,
            special_force_losses_replaced: Default::default(),
            card_ids: Default::default(),
            stolen_token: Default::default(),
            add_extra_force: Default::default(),
        }
    }
}

impl BattleConcluded {
    pub fn decision_to_capture(&self) -> CaptureDecision {
        if !self.capture {
            return CaptureDecision::DontCapture;
        }

        if self.kill {
            return CaptureDecision::Kill;
        }
        CaptureDecision::Capture
    }

    pub fn set_decision_to_capture(&mut self, value: Option<CaptureDecision>) {
        if matches!(value, None | Some(CaptureDecision::DontCapture)) {
            self.capture = false;
        } else {
            self.capture = true;
            self.kill = value == Some(CaptureDecision::Kill);
        }
    }

    pub fn discarded_cards(&self) -> Vec<TreacheryCard> {
        Event::id_string_to_objects(Some(self.card_ids.as_deref().unwrap_or("")), &TreacheryCardManager::lookup())
    }

    pub fn set_discarded_cards(&mut self, value: &[TreacheryCard]) {
        self.card_ids = Some(Event::objects_to_id_string(value, &TreacheryCardManager::lookup()));
    }

    pub fn decipherer_replaces_traitors(&self, game: &mut Game, by: By) {
        let to_replace = if by.faction == Some(Faction::Tleilaxu) { "facedancer " } else { "traitor " };

        game.log(vec![
                by.faction.into(),
                " replaced ".into(),
                to_replace.into(),
                self.traitor_to_replace.into(),
                " with another leader from the traitor deck".into(),
            ],
        );

        let me = by.player();
        let tleilaxu = by.faction == Some(Faction::Tleilaxu);

        let (Some(new_traitor), Some(traitor_to_replace)) = (self.new_traitor, self.traitor_to_replace) else {
            return;
        };

        {
            let pl = game.player_mut(me);
            let currently_held = if tleilaxu { &mut pl.face_dancers } else { &mut pl.traitors };
            currently_held.push(new_traitor);
        }
        if let Some(pos) = game.fight_mut().traitors_decipherer_can_look_at.iter().position(|h| *h == new_traitor) {
            game.fight_mut().traitors_decipherer_can_look_at.remove(pos);
        }

        {
            let pl = game.player_mut(me);
            let currently_held = if tleilaxu { &mut pl.face_dancers } else { &mut pl.traitors };
            if let Some(pos) = currently_held.iter().position(|h| *h == traitor_to_replace) {
                currently_held.remove(pos);
            }
        }
        let Some(traitor_deck) = game.decks.traitor_deck.as_mut() else { return };
        traitor_deck.put_on_top(traitor_to_replace);

        game.stone(Milestone::Shuffled);
        game.decks.traitor_deck.as_mut().expect("traitor deck").shuffle(&mut game.rng);
    }

    pub fn decide_fate_of_captured_leader(&self, game: &mut Game, by: By) {
        let Some(current_battle) = game.current_battle().cloned() else { return };

        if !by.is(Faction::Harkonnen) || !game.applicable(Rule::HarkonnenCapturesOrKillsLeaders) || game.fight().map_or(true, |c| c.harkonnen_victim.is_none()) {
            return;
        }

        if game.prevented(FactionAdvantage::HarkonnenCaptureLeader) {
            game.log_prevention_by_karma(FactionAdvantage::HarkonnenCaptureLeader);
        } else {
            let target = current_battle.opponent_of(game, by.player());
            if let Some(target) = target {
                self.capture_or_assassinate_leader(game, by.player(), target);
            }
        }
    }

    pub fn take_tech_token(&self, game: &mut Game, by: By) {
        if let Some(stolen_token) = self.stolen_token {
            let loser = game.get_player(game.battle_loser());
            if let Some(loser) = loser {
                if game.player(loser).tech_tokens.contains(&stolen_token) {
                    let tokens = &mut game.player_mut(loser).tech_tokens;
                    if let Some(pos) = tokens.iter().position(|t| *t == stolen_token) {
                        tokens.remove(pos);
                    }
                    game.player_mut(by.player()).tech_tokens.push(stolen_token);
                    let battle_loser = game.battle_loser();
                    game.log(vec![by.faction.into(), " steal ".into(), stolen_token.into(), " from ".into(), battle_loser.into()]);
                }
            }
        }
    }

    pub fn process_ixian_force_losses_and_substitutions(&self, game: &mut Game, by: By) -> GameResult {
        if game.fight().map_or(0, |c| c.ixian_special_force_losses_to_take) > 0 {
            let Some(current_battle) = game.current_battle().cloned() else { return Ok(()) };

            let plan = current_battle.plan_of(game, Some(by.player()));
            let territory = current_battle.territory;
            let (Some(plan), Some(territory)) = (plan, territory) else { return Ok(()) };

            let Some(winner_gambit) = game.winner_battle_action() else { return Ok(()) };
            let forces_to_lose = i32::from(winner_gambit.forces) + i32::from(winner_gambit.forces_at_half_strength) + i32::from(self.special_force_losses_replaced);
            let special_forces_to_lose = i32::from(winner_gambit.special_forces) + i32::from(winner_gambit.special_forces_at_half_strength)
                - i32::from(self.special_force_losses_replaced);

            let force_supplier_of_winner = Battle::determine_force_supplier(game, by.player());

            let faction = game.player(by.player()).faction;
            let special_force = game.player(force_supplier_of_winner).special_force();
            let force = game.player(force_supplier_of_winner).force();
            game.log(vec![
                    faction.into(),
                    " substitute ".into(),
                    i32::from(self.special_force_losses_replaced).into(),
                    special_force.into(),
                    " losses by ".into(),
                    force.into(),
                    " losses".into(),
                ],
            );

            let mut special_forces_to_save_to_reserves = 0;
            let mut forces_to_save_to_reserves = 0;
            let mut special_forces_to_save_in_territory = 0;
            let mut forces_to_save_in_territory = 0;

            if plan.hero.is_some() && game.skilled_as(plan.hero.expect("hero"), LeaderSkill::Graduate) {
                special_forces_to_save_in_territory = std::cmp::min(special_forces_to_lose, 1);
                forces_to_save_in_territory = std::cmp::max(0, std::cmp::min(forces_to_lose, 1 - special_forces_to_save_in_territory));

                special_forces_to_save_to_reserves = std::cmp::max(
                    0,
                    std::cmp::min(
                        (special_forces_to_lose - special_forces_to_save_in_territory) - forces_to_save_in_territory,
                        2,
                    ),
                );
                forces_to_save_to_reserves = std::cmp::max(
                    0,
                    std::cmp::min(forces_to_lose - forces_to_save_in_territory, 2 - special_forces_to_save_to_reserves),
                );
            } else if game.has_skill_in_front_of_shield(by.player(), LeaderSkill::Graduate) {
                special_forces_to_save_to_reserves = std::cmp::min(special_forces_to_lose, 1);
                forces_to_save_to_reserves = std::cmp::max(0, std::cmp::min(forces_to_lose, 1 - special_forces_to_save_to_reserves));
            }

            if special_forces_to_save_in_territory + forces_to_save_in_territory + special_forces_to_save_to_reserves + forces_to_save_to_reserves > 0 {
                if special_forces_to_save_to_reserves > 0 {
                    Player::some_forces_in_territory_to_reserves(game, force_supplier_of_winner, territory, special_forces_to_save_to_reserves, true)?;
                }

                if forces_to_save_to_reserves > 0 {
                    Player::some_forces_in_territory_to_reserves(game, force_supplier_of_winner, territory, forces_to_save_to_reserves, false)?;
                }

                let force = game.player(force_supplier_of_winner).force();
                let special_force = game.player(force_supplier_of_winner).special_force();
                game.log(vec![
                        LeaderSkill::Graduate.into(),
                        " saves ".into(),
                        express_part_if!(forces_to_save_in_territory > 0; forces_to_save_in_territory, force).into(),
                        express_part_if!(special_forces_to_save_in_territory > 0; special_forces_to_save_in_territory, special_force).into(),
                        express_part_if!(forces_to_save_in_territory > 0 || special_forces_to_save_in_territory > 0; " in ", territory).into(),
                        express_part_if!(forces_to_save_to_reserves > 0 || special_forces_to_save_to_reserves > 0; " and ").into(),
                        express_part_if!(forces_to_save_to_reserves > 0; forces_to_save_to_reserves, force).into(),
                        express_part_if!(special_forces_to_save_to_reserves > 0; special_forces_to_save_to_reserves, special_force).into(),
                        express_part_if!(forces_to_save_to_reserves > 0 || special_forces_to_save_to_reserves > 0; " to reserves").into(),
                    ],
                );
            }

            game.handle_force_losses(
                territory,
                force_supplier_of_winner,
                (forces_to_lose - forces_to_save_to_reserves) - forces_to_save_in_territory,
                (special_forces_to_lose - special_forces_to_save_to_reserves) - special_forces_to_save_in_territory,
            )?;
        }
        Ok(())
    }

    pub fn capture_or_assassinate_leader(&self, game: &mut Game, harkonnen: PlayerId, target: PlayerId) {
        if self.decision_to_capture() == CaptureDecision::Capture {
            game.log(vec![Faction::Harkonnen.into(), " capture a leader!".into()]);
            let Some(victim) = game.fight().and_then(|c| c.harkonnen_victim) else { return };
            game.player_mut(harkonnen).leaders.push(victim);
            let leaders = &mut game.player_mut(target).leaders;
            if let Some(pos) = leaders.iter().position(|l| *l == victim) {
                leaders.remove(pos);
            }
            game.set_in_front_of_shield(victim, false);
            let target_faction = game.player(target).faction;
            game.leaders.captured_leaders.insert_new(victim, target_faction.expect("faction"));
        } else if self.decision_to_capture() == CaptureDecision::Kill {
            game.log(vec![Faction::Harkonnen.into(), " kill a leader for ".into(), Payment::of(2).into()]);
            if let Some(victim) = game.fight().and_then(|c| c.harkonnen_victim) {
                game.assassinate_leader(victim);
            }
            let pl = game.player_mut(harkonnen);
            pl.resources += 2;
        } else if self.decision_to_capture() == CaptureDecision::DontCapture {
            game.log(vec![Faction::Harkonnen.into(), " decide not to capture or kill a leader".into()]);
        }
    }

    pub fn valid_replacement_force_amounts(g: &Game, p: PlayerId) -> Vec<i32> {
        if g.fight().map_or(0, |c| c.ixian_special_force_losses_to_take) == 0 {
            return vec![0];
        }

        let territory = g.current_battle().and_then(|b| b.territory);
        let winner_battle_action = g.winner_battle_action();
        let (Some(territory), Some(winner_battle_action)) = (territory, winner_battle_action) else {
            return vec![0];
        };

        let replacement_forces_left = g
            .player(p)
            .forces_in_territory(Some(territory)) - (i32::from(winner_battle_action.forces) + i32::from(winner_battle_action.forces_at_half_strength));

        if replacement_forces_left <= 0 {
            return vec![0];
        }

        // Enumerable.Range(0, count) checks count when called.
        let count = 1i32 + std::cmp::min(replacement_forces_left, g.fight().map_or(0, |c| c.ixian_special_force_losses_to_take));
        if count < 0 {
            panic!("Specified argument was out of the range of valid values. (Parameter 'count')");
        }
        (0..count).collect()
    }

    pub fn valid_max_replacement_force_amount(g: &Game, p: PlayerId) -> i32 {
        if g.fight().map_or(0, |c| c.ixian_special_force_losses_to_take) == 0 {
            return 0;
        }

        let territory = g.current_battle().and_then(|b| b.territory);
        let winner_battle_action = g.winner_battle_action();
        let (Some(territory), Some(winner_battle_action)) = (territory, winner_battle_action) else {
            return 0;
        };

        let replacement_forces_left = g
            .player(p)
            .forces_in_territory(Some(territory)) - (i32::from(winner_battle_action.forces) + i32::from(winner_battle_action.forces_at_half_strength));

        if replacement_forces_left <= 0 {
            return 0;
        }

        std::cmp::min(replacement_forces_left, g.fight().map_or(0, |c| c.ixian_special_force_losses_to_take))
    }

    pub fn may_capture_or_kill(g: &Game, p: Option<PlayerId>) -> bool {
        let Some(p) = p else { return false };
        g.player(p).faction == Some(Faction::Harkonnen)
            && g.battle_winner() == Some(Faction::Harkonnen)
            && !g.prevented(FactionAdvantage::HarkonnenCaptureLeader)
            && g.applicable(Rule::HarkonnenCapturesOrKillsLeaders)
            && g.fight().is_some_and(|c| c.harkonnen_victim.is_some())
    }

    pub fn valid_capture_decisions(_g: &Game) -> Vec<CaptureDecision> {
        vec![CaptureDecision::Capture, CaptureDecision::Kill]
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_traitors_to_replace(game: &Game, p: PlayerId) -> impl Iterator<Item = Hero> {
        use either::Either;
        let pl = game.player(p);
        if pl.faction == Some(Faction::Tleilaxu) {
            return Either::Left(pl.face_dancers.iter().copied().filter(move |h| !pl.revealed_face_dancers.contains(h)));
        }
        Either::Right(pl.traitors.iter().copied().filter(move |h| !pl.revealed_traitors.contains(h)))
    }

    pub fn may_choose_to_discard_cards(g: &Game) -> bool {
        g.fight().map_or(true, |c| c.winner_may_choose_to_discard)
    }

    pub fn may_add_extra_force(g: &Game, p: PlayerId) -> bool {
        let Some(territory) = g.current_battle().and_then(|b| b.territory) else {
            return false;
        };
        let pl = g.player(p);
        pl.is(Faction::Atreides) && pl.has_any_high_threshold(g) && pl.forces_in_reserve() >= 1 && pl.any_forces_in_territory(Some(territory)) > 0
    }
}

impl EventRules for BattleConcluded {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        let p = by.player();
        if i32::from(self.special_force_losses_replaced) > 0 && game.prevented(FactionAdvantage::IxianReplacingSpecialForces) {
            return Err(Rejection(express!(TreacheryCardType::Karma, " prevents replacing ", FactionSpecialForce::Ixian, " losses")));
        }
        if i32::from(self.special_force_losses_replaced) > 0
            && !BattleConcluded::valid_replacement_force_amounts(game, p).contains(&i32::from(self.special_force_losses_replaced))
        {
            return Err(Rejection(express!("Invalid amount of replacement forces")));
        }

        if game.battle_leftovers.decipherer_may_replace_traitor {
            if self.new_traitor.is_some() && self.traitor_to_replace.is_none() {
                return Err(Rejection(express!("Select a traitor to be replaced by ", self.new_traitor)));
            }
            if self.new_traitor.is_none() && self.traitor_to_replace.is_some() {
                return Err(Rejection(express!("Select a traitor to replace ", self.traitor_to_replace)));
            }
        }

        if !BattleConcluded::may_choose_to_discard_cards(game) && !self.discarded_cards().is_empty() {
            return Err(Rejection(express!("You are not allowed to choose which cards to discard")));
        }

        if self.add_extra_force && !BattleConcluded::may_add_extra_force(game, by.player()) {
            return Err(Rejection(express!("You cannot add a force")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let Some(current_battle) = game.current_battle().cloned() else { return Ok(()) };

        game.fight_mut().concluded_by_winner = true;

        if self.add_extra_force {
            let territory = current_battle.territory;
            let to = territory.and_then(|t| t.locations().iter().copied().find(|l| game.player(by.player()).any_forces_in(Some(*l)) > 0));
            if let Some(to) = to {
                Player::ship_forces(game, by.player(), to, 1)?;
            }

            if let Some(territory) = territory {
                game.log(vec![by.faction.into(), " place ".into(), 1.into(), " extra force in ".into(), territory.into()]);
            }
        }

        for c in self.discarded_cards() {
            game.log(vec![by.faction.into(), " discard ".into(), c.into()]);
            let me = by.player();
            let cards = &mut game.player_mut(me).treachery_cards;
            if let Some(pos) = cards.iter().position(|x| *x == c) {
                cards.remove(pos);
            }
            game.decks.treachery_discard_pile.as_mut().required("treachery discard pile")?.put_on_top(c);
        }

        if game.fight().map_or(0, |c| c.traitors_decipherer_can_look_at.len()) > 0 {
            let count = game.fight().map_or(0, |c| c.traitors_decipherer_can_look_at.len()) as i32;
            game.log(vec![by.faction.into(), " look at ".into(), count.into(), " leaders in the traitor deck".into()]);
        }

        if self.traitor_to_replace.is_some() && self.new_traitor.is_some() && game.battle_leftovers.decipherer_may_replace_traitor {
            self.decipherer_replaces_traitors(game, by);
        }

        self.decide_fate_of_captured_leader(game, by);
        self.take_tech_token(game, by);
        self.process_ixian_force_losses_and_substitutions(game, by)?;

        let loser = game.get_player(game.battle_loser());
        if !LoserConcluded::is_applicable(game, loser) {
            let condition1 = !game.is_playing(Faction::Tleilaxu) || game.battle_winner() == Some(Faction::Tleilaxu);
            if condition1 {
                Game::finish_battle(game)?;
            } else {
                game.enter_battle(BattleStep::RevealingFacedancer);
            }
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " conclude the battle")
    }
}
