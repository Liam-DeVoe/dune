use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct BattleConcluded {
    pub kill: bool,
    pub capture: bool,
    #[canon(name = "_traitorToReplaceId")]
    pub traitor_to_replace_id: i32,
    #[canon(name = "_newTraitorId")]
    pub new_traitor_id: i32,
    pub special_force_losses_replaced: i32,
    #[canon(name = "_cardIds")]
    pub card_ids: Option<String>,
    pub stolen_token: TechToken,
    pub add_extra_force: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for BattleConcluded {
    fn default() -> Self {
        BattleConcluded {
            kill: Default::default(),
            capture: true,
            traitor_to_replace_id: -1,
            new_traitor_id: -1,
            special_force_losses_replaced: Default::default(),
            card_ids: Default::default(),
            stolen_token: Default::default(),
            add_extra_force: Default::default(),
            base: Default::default(),
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

    pub fn set_decision_to_capture(&mut self, value: CaptureDecision) {
        if value == CaptureDecision::DontCapture || value == CaptureDecision::None {
            self.capture = false;
        } else {
            self.capture = true;
            self.kill = value == CaptureDecision::Kill;
        }
    }

    pub fn traitor_to_replace(&self) -> Option<Hero> {
        LeaderManager::hero_lookup().find(self.traitor_to_replace_id)
    }

    pub fn set_traitor_to_replace(&mut self, value: Option<Hero>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.traitor_to_replace_id = LeaderManager::hero_lookup().get_id(value);
    }

    pub fn new_traitor(&self) -> Option<Hero> {
        LeaderManager::hero_lookup().find(self.new_traitor_id)
    }

    pub fn set_new_traitor(&mut self, value: Option<Hero>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.new_traitor_id = LeaderManager::hero_lookup().get_id(value);
    }

    pub fn discarded_cards(&self) -> Vec<TreacheryCard> {
        GameEvent::id_string_to_objects(Some(self.card_ids.as_deref().unwrap_or("")), &TreacheryCardManager::lookup())
    }

    pub fn set_discarded_cards(&mut self, value: &[TreacheryCard]) {
        self.card_ids = Some(GameEvent::objects_to_id_string(value, &TreacheryCardManager::lookup()));
    }

    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let p = self.player();
        if self.special_force_losses_replaced > 0 && game.prevented(FactionAdvantage::IxianReplacingSpecialForces) {
            return Some(express!(TreacheryCardType::Karma, " prevents replacing ", FactionSpecialForce::Ixian, " losses"));
        }
        if self.special_force_losses_replaced > 0
            && !BattleConcluded::valid_replacement_force_amounts(game, p).contains(&self.special_force_losses_replaced)
        {
            return Some(express!("Invalid amount of replacement forces"));
        }

        if game.decipherer_may_replace_traitor {
            if self.new_traitor().is_some() && self.traitor_to_replace().is_none() {
                return Some(express!("Select a traitor to be replaced by ", self.new_traitor()));
            }
            if self.new_traitor().is_none() && self.traitor_to_replace().is_some() {
                return Some(express!("Select a traitor to replace ", self.traitor_to_replace()));
            }
        }

        if !BattleConcluded::may_choose_to_discard_cards(game) && !self.discarded_cards().is_empty() {
            return Some(express!("You are not allowed to choose which cards to discard"));
        }

        if self.add_extra_force && !BattleConcluded::may_add_extra_force(game, self.player()) {
            return Some(express!("You cannot add a force"));
        }

        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let Some(current_battle) = game.current_battle.clone() else { return Ok(()) };

        game.battle_was_concluded_by_winner = true;

        if self.add_extra_force {
            let territory = current_battle.territory(game);
            let to = territory.and_then(|t| t.locations().iter().copied().find(|l| game.player(self.player()).any_forces_in(Some(*l)) > 0));
            if let Some(to) = to {
                Player::ship_forces(game, self.player(), to, 1);
            }

            if let Some(territory) = territory {
                self.log_objects(game, vec![self.initiator().into(), " place ".into(), 1.into(), " extra force in ".into(), territory.into()]);
            }
        }

        for c in self.discarded_cards() {
            self.log_objects(game, vec![self.initiator().into(), " discard ".into(), c.into()]);
            let me = self.player();
            let cards = &mut game.player_mut(me).treachery_cards;
            if let Some(pos) = cards.iter().position(|x| *x == c) {
                cards.remove(pos);
            }
            game.treachery_discard_pile.as_mut().required("treachery discard pile")?.put_on_top(c);
        }

        if game.traitors_decipherer_can_look_at.len() > 0 {
            let count = game.traitors_decipherer_can_look_at.len() as i32;
            self.log_objects(game, vec![self.initiator().into(), " look at ".into(), count.into(), " leaders in the traitor deck".into()]);
        }

        if self.traitor_to_replace().is_some() && self.new_traitor().is_some() && game.decipherer_may_replace_traitor {
            self.decipherer_replaces_traitors(game);
        }

        self.decide_fate_of_captured_leader(game);
        self.take_tech_token(game);
        self.process_ixian_force_losses_and_substitutions(game);

        let loser = self.get_player(game, game.battle_loser);
        if !LoserConcluded::is_applicable(game, loser) {
            let condition1 = !game.is_playing(Faction::Tleilaxu) || game.battle_winner == Some(Faction::Tleilaxu);
            if condition1 {
                Game::finish_battle(game)?;
            } else {
                game.enter(Phase::RevealingFacedancer, true);
            }
        }
        Ok(())
    }

    pub fn decipherer_replaces_traitors(&self, game: &mut Game) {
        let to_replace = if self.initiator() == Faction::Tleilaxu { "facedancer " } else { "traitor " };

        self.log_objects(
            game,
            vec![
                self.initiator().into(),
                " replaced ".into(),
                to_replace.into(),
                self.traitor_to_replace().into(),
                " with another leader from the traitor deck".into(),
            ],
        );

        let me = self.player();
        let tleilaxu = self.initiator() == Faction::Tleilaxu;

        let (Some(new_traitor), Some(traitor_to_replace)) = (self.new_traitor(), self.traitor_to_replace()) else {
            return;
        };

        {
            let pl = game.player_mut(me);
            let currently_held = if tleilaxu { &mut pl.face_dancers } else { &mut pl.traitors };
            currently_held.push(new_traitor);
        }
        if let Some(pos) = game.traitors_decipherer_can_look_at.iter().position(|h| *h == new_traitor) {
            game.traitors_decipherer_can_look_at.remove(pos);
        }

        {
            let pl = game.player_mut(me);
            let currently_held = if tleilaxu { &mut pl.face_dancers } else { &mut pl.traitors };
            if let Some(pos) = currently_held.iter().position(|h| *h == traitor_to_replace) {
                currently_held.remove(pos);
            }
        }
        let Some(traitor_deck) = game.traitor_deck.as_mut() else { return };
        traitor_deck.put_on_top(traitor_to_replace);

        game.stone(Milestone::Shuffled);
        game.traitor_deck.as_mut().nre().shuffle();
    }

    pub fn decide_fate_of_captured_leader(&self, game: &mut Game) {
        let Some(current_battle) = game.current_battle.clone() else { return };

        if !self.by(Faction::Harkonnen) || !game.applicable(Rule::HarkonnenCapturesOrKillsLeaders) || game.harkonnen_victim.is_none() {
            return;
        }

        if game.prevented(FactionAdvantage::HarkonnenCaptureLeader) {
            game.log_prevention_by_karma(FactionAdvantage::HarkonnenCaptureLeader);
        } else {
            let target = current_battle.opponent_of(game, self.player());
            if let Some(target) = target {
                self.capture_or_assassinate_leader(game, self.player(), target);
            }
        }
    }

    pub fn take_tech_token(&self, game: &mut Game) {
        if self.stolen_token != TechToken::None {
            let loser = self.get_player(game, game.battle_loser);
            if let Some(loser) = loser {
                if game.player(loser).tech_tokens.contains(&self.stolen_token) {
                    let tokens = &mut game.player_mut(loser).tech_tokens;
                    if let Some(pos) = tokens.iter().position(|t| *t == self.stolen_token) {
                        tokens.remove(pos);
                    }
                    game.player_mut(self.player()).tech_tokens.push(self.stolen_token);
                    let battle_loser = game.battle_loser;
                    self.log_objects(game, vec![self.initiator().into(), " steal ".into(), self.stolen_token.into(), " from ".into(), battle_loser.into()]);
                }
            }
        }
    }

    pub fn process_ixian_force_losses_and_substitutions(&self, game: &mut Game) {
        if game.ixian_special_force_losses_to_take > 0 {
            let Some(current_battle) = game.current_battle.clone() else { return };

            let plan = current_battle.plan_of(game, Some(self.player()));
            let territory = current_battle.territory(game);
            let (Some(plan), Some(territory)) = (plan, territory) else { return };

            let Some(winner_gambit) = game.winner_battle_action() else { return };
            let forces_to_lose = winner_gambit
                .forces
                .wrapping_add(winner_gambit.forces_at_half_strength)
                .wrapping_add(self.special_force_losses_replaced);
            let special_forces_to_lose = winner_gambit
                .special_forces
                .wrapping_add(winner_gambit.special_forces_at_half_strength)
                .wrapping_sub(self.special_force_losses_replaced);

            let force_supplier_of_winner = Battle::determine_force_supplier(game, self.player());

            let faction = game.player(self.player()).faction;
            let special_force = game.player(force_supplier_of_winner).special_force();
            let force = game.player(force_supplier_of_winner).force();
            self.log_objects(
                game,
                vec![
                    faction.into(),
                    " substitute ".into(),
                    self.special_force_losses_replaced.into(),
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

            if plan.hero().is_some() && game.skilled_as(plan.hero().nre(), LeaderSkill::Graduate) {
                special_forces_to_save_in_territory = std::cmp::min(special_forces_to_lose, 1);
                forces_to_save_in_territory = std::cmp::max(0, std::cmp::min(forces_to_lose, 1 - special_forces_to_save_in_territory));

                special_forces_to_save_to_reserves = std::cmp::max(
                    0,
                    std::cmp::min(
                        special_forces_to_lose
                            .wrapping_sub(special_forces_to_save_in_territory)
                            .wrapping_sub(forces_to_save_in_territory),
                        2,
                    ),
                );
                forces_to_save_to_reserves = std::cmp::max(
                    0,
                    std::cmp::min(forces_to_lose.wrapping_sub(forces_to_save_in_territory), 2 - special_forces_to_save_to_reserves),
                );
            } else if game.skilled_as_player_leader_skill(self.player(), LeaderSkill::Graduate) {
                special_forces_to_save_to_reserves = std::cmp::min(special_forces_to_lose, 1);
                forces_to_save_to_reserves = std::cmp::max(0, std::cmp::min(forces_to_lose, 1 - special_forces_to_save_to_reserves));
            }

            if special_forces_to_save_in_territory + forces_to_save_in_territory + special_forces_to_save_to_reserves + forces_to_save_to_reserves > 0 {
                if special_forces_to_save_to_reserves > 0 {
                    Player::forces_to_reserves_territory_int32_boolean(game, force_supplier_of_winner, territory, special_forces_to_save_to_reserves, true);
                }

                if forces_to_save_to_reserves > 0 {
                    Player::forces_to_reserves_territory_int32_boolean(game, force_supplier_of_winner, territory, forces_to_save_to_reserves, false);
                }

                let force = game.player(force_supplier_of_winner).force();
                let special_force = game.player(force_supplier_of_winner).special_force();
                self.log_objects(
                    game,
                    vec![
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
                forces_to_lose.wrapping_sub(forces_to_save_to_reserves).wrapping_sub(forces_to_save_in_territory),
                special_forces_to_lose
                    .wrapping_sub(special_forces_to_save_to_reserves)
                    .wrapping_sub(special_forces_to_save_in_territory),
            );
        }
    }

    pub fn capture_or_assassinate_leader(&self, game: &mut Game, harkonnen: PlayerIndex, target: PlayerIndex) {
        if self.decision_to_capture() == CaptureDecision::Capture {
            self.log_objects(game, vec![Faction::Harkonnen.into(), " capture a leader!".into()]);
            let Some(victim) = game.harkonnen_victim else { return };
            game.player_mut(harkonnen).leaders.push(victim);
            let leaders = &mut game.player_mut(target).leaders;
            if let Some(pos) = leaders.iter().position(|l| *l == victim) {
                leaders.remove(pos);
            }
            game.set_in_front_of_shield(victim, false);
            let target_faction = game.player(target).faction;
            game.captured_leaders.add_with(victim, target_faction, |k| ExpressionElement::from(*k).to_cs_string());
        } else if self.decision_to_capture() == CaptureDecision::Kill {
            self.log_objects(game, vec![Faction::Harkonnen.into(), " kill a leader for ".into(), Payment::of(2).into()]);
            if let Some(victim) = game.harkonnen_victim {
                game.assassinate_leader(victim);
            }
            let pl = game.player_mut(harkonnen);
            pl.resources = pl.resources.wrapping_add(2);
        } else if self.decision_to_capture() == CaptureDecision::DontCapture {
            self.log_objects(game, vec![Faction::Harkonnen.into(), " decide not to capture or kill a leader".into()]);
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " conclude the battle")
    }

    pub fn valid_replacement_force_amounts(g: &Game, p: PlayerIndex) -> Vec<i32> {
        if g.ixian_special_force_losses_to_take == 0 {
            return vec![0];
        }

        let territory = g.current_battle.as_ref().and_then(|b| b.territory(g));
        let winner_battle_action = g.winner_battle_action();
        let (Some(territory), Some(winner_battle_action)) = (territory, winner_battle_action) else {
            return vec![0];
        };

        let replacement_forces_left = g
            .player(p)
            .forces_in_territory(Some(territory))
            .wrapping_sub(winner_battle_action.forces.wrapping_add(winner_battle_action.forces_at_half_strength));

        if replacement_forces_left <= 0 {
            return vec![0];
        }

        // Enumerable.Range(0, count) checks count when called.
        let count = 1i32.wrapping_add(std::cmp::min(replacement_forces_left, g.ixian_special_force_losses_to_take));
        if count < 0 {
            panic!("Specified argument was out of the range of valid values. (Parameter 'count')");
        }
        (0..count).collect()
    }

    pub fn valid_max_replacement_force_amount(g: &Game, p: PlayerIndex) -> i32 {
        if g.ixian_special_force_losses_to_take == 0 {
            return 0;
        }

        let territory = g.current_battle.as_ref().and_then(|b| b.territory(g));
        let winner_battle_action = g.winner_battle_action();
        let (Some(territory), Some(winner_battle_action)) = (territory, winner_battle_action) else {
            return 0;
        };

        let replacement_forces_left = g
            .player(p)
            .forces_in_territory(Some(territory))
            .wrapping_sub(winner_battle_action.forces.wrapping_add(winner_battle_action.forces_at_half_strength));

        if replacement_forces_left <= 0 {
            return 0;
        }

        std::cmp::min(replacement_forces_left, g.ixian_special_force_losses_to_take)
    }

    pub fn may_capture_or_kill(g: &Game, p: Option<PlayerIndex>) -> bool {
        let p = p.and_then(crate::compat::cs::CsNull::opt);
        let Some(p) = p else { return false };
        g.player(p).faction == Faction::Harkonnen
            && g.battle_winner == Some(Faction::Harkonnen)
            && !g.prevented(FactionAdvantage::HarkonnenCaptureLeader)
            && g.applicable(Rule::HarkonnenCapturesOrKillsLeaders)
            && g.harkonnen_victim.is_some()
    }

    pub fn valid_capture_decisions(_g: &Game) -> Vec<CaptureDecision> {
        vec![CaptureDecision::Capture, CaptureDecision::Kill]
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_traitors_to_replace(game: &Game, p: PlayerIndex) -> impl Iterator<Item = Hero> {
        use either::Either;
        let pl = game.player(p);
        if pl.faction == Faction::Tleilaxu {
            return Either::Left(pl.face_dancers.iter().copied().filter(move |h| !pl.revealed_face_dancers.contains(h)));
        }
        Either::Right(pl.traitors.iter().copied().filter(move |h| !pl.revealed_traitors.contains(h)))
    }

    pub fn may_choose_to_discard_cards(g: &Game) -> bool {
        g.battle_winner_may_choose_to_discard
    }

    pub fn may_add_extra_force(g: &Game, p: PlayerIndex) -> bool {
        let Some(territory) = g.current_battle.as_ref().and_then(|b| b.territory(g)) else {
            return false;
        };
        let pl = g.player(p);
        pl.is(Faction::Atreides) && pl.has_high_threshold_0(g) && pl.forces_in_reserve() >= 1 && pl.any_forces_in_territory(Some(territory)) > 0
    }

}

impl GameEventMethods for BattleConcluded {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::BattleConcluded
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::BattleConcluded(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        BattleConcluded::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        BattleConcluded::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        BattleConcluded::get_message(self, game)
    }

}
