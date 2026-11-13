use crate::compat::random_ext::RandomExt;
use crate::prelude::*;

impl Game {
    pub fn next_faction_to_perform_custom_setup(&self) -> Faction {
        // FirstOrDefault on a value type: default(Faction) is Faction.None.
        self.players
            .iter()
            .map(|p| p.faction)
            .find(|f| !self.has_acted_or_passed.contains(f))
            .unwrap_or_default()
    }

    pub fn enter_phase_awaiting_players(&mut self) {
        self.players = Vec::new();
        self.current_turn = 0;
        self.enter(Phase::AwaitingPlayers, true);
    }

    pub fn assign_factions_and_enter_faction_trade(&mut self) {
        // The Deck constructor copies its source, so evaluating the filter here is equivalent.
        let all = self.factions_in_play.borrow().clone();
        let available_factions: Vec<Faction> = all.into_iter().filter(|f| !self.is_playing(*f)).collect();
        let mut selectable = Deck::new(available_factions, self.random.clone());
        self.stone(Milestone::Shuffled);
        selectable.shuffle();

        // foreach over a lazy Where: the predicate is checked per player, before its body.
        for i in 0..self.players.len() {
            if self.players[i].faction == Faction::None {
                self.players[i].faction = selectable.draw();
            }
        }

        self.enter_phase_trading_factions();
    }

    pub fn determine_positions_at_table(&mut self) {
        let mut seats: Deck<i32> = Deck::empty(self.random.clone());
        let mut i = 0;
        while i < self.settings.as_ref().nre().borrow().number_of_players {
            seats.put_on_top(i);
            i += 1;
        }

        seats.shuffle();

        //Seat players
        for p in 0..self.players.len() {
            self.players[p].seat = seats.draw();
        }
    }

    pub fn enter_phase_trading_factions(&mut self) {
        self.current_trade_offers.clear();
        self.enter(Phase::TradingFactions, true);
    }

    pub fn enter_setup_phase(&mut self) {
        // C# parity: this Enter is overwritten by the Enter at the end of this method, but
        // its side effects (deal cleanup, or EnterPhaseTradingFactions) still happen.
        let condition = self.applicable(Rule::CustomDecks);
        self.enter_boolean_phase_action(condition, Phase::CustomizingDecks, Game::enter_phase_trading_factions);

        self.current_trade_offers.clear();

        // Players.OrderBy(p => p.Seat) is a stable sort.
        let mut by_seat: Vec<(i32, Faction)> = self.players.iter().map(|p| (p.seat, p.faction)).collect();
        by_seat.sort_by_key(|(seat, _)| *seat);
        let first = by_seat.iter().cs_first().1;
        self.has_battle_wheel.push(first);
        let last = by_seat.iter().cs_last().1;
        self.has_battle_wheel.push(last);

        for p in 0..self.players.len() {
            Player::assign_leaders(self, self.player_at(p));
        }

        let ecaz = self.get_player(Some(Faction::Ecaz));
        if let Some(ecaz) = ecaz {
            self.assign_initial_ambassadors(ecaz);

            if self.applicable(Rule::EcazLoyalty) {
                let leaders = self.player(ecaz).leaders.clone();
                self.ecaz_loyal_leader = leaders.into_iter().random_or_default(self.random.as_ref().nre());
                let pll = self.ecaz_loyal_leader;
                self.log(vec![pll.into(), " is forever loyal to ".into(), Faction::Ecaz.into()]);
            }
        }

        let c1 = self.is_playing(Faction::BeneGesserit);
        let c2 = self.treachery_cards_before_traitors();
        self.enter_boolean_phase_boolean_action_action(
            c1,
            Phase::BeneGesseritPredicting,
            c2,
            Game::deal_starting_treachery_cards,
            Game::deal_traitors,
        );
    }

    pub fn deal_traitors(&mut self) {
        self.stone(Milestone::Shuffled);
        let random = self.random.clone();
        self.traitor_deck = Some(self.create_and_shuffle_traitor_deck(random));

        if self.applicable(Rule::HarkonnenMulligan) && self.is_playing(Faction::Harkonnen) {
            self.deal_harkonnen_traitor_cards();
            self.enter(Phase::HarkonnenMulligan, true);
        } else {
            for _i in 1..=4 {
                for p in 0..self.players.len() {
                    if self.players[p].faction != Faction::Tleilaxu {
                        let traitor = self.traitor_deck.as_mut().nre().draw();
                        self.players[p].traitors.push(traitor);
                    }
                }
            }

            self.enter_select_traitors();
        }
    }

    /// `random` is `Option` because `Game.Random` may be null; only the
    /// shuffle would then throw.
    pub fn create_and_shuffle_traitor_deck(&mut self, random: Option<SharedRandom>) -> Deck<Hero> {
        let mut result = Deck::new(self.traitors_in_play(), random);

        if let Some(pll) = self.ecaz_loyal_leader {
            let hero = Hero::Leader(pll);
            if let Some(i) = result.items.iter().position(|h| *h == hero) {
                result.items.remove(i);
            }
            for p in self.players.iter_mut() {
                p.known_non_traitors.add(hero);
            }
        }

        if let Some(vidal) = self.vidal() {
            let hero = Hero::Leader(vidal);
            for p in self.players.iter_mut() {
                p.known_non_traitors.add(hero);
            }
        }

        result.shuffle();
        result
    }

    pub fn deal_harkonnen_traitor_cards(&mut self) {
        let Some(harkonnen) = self.get_player(Some(Faction::Harkonnen)) else {
            return;
        };
        for _i in 1..=4 {
            let traitor = self.traitor_deck.as_mut().nre().draw();
            self.player_mut(harkonnen).traitors.push(traitor);
        }
    }

    pub fn enter_select_traitors(&mut self) {
        self.has_acted_or_passed.clear();

        if self.is_playing(Faction::Harkonnen) {
            self.has_acted_or_passed.push(Faction::Harkonnen);
        }

        if self.is_playing(Faction::Tleilaxu) {
            self.has_acted_or_passed.push(Faction::Tleilaxu);
        }

        let condition = self.players.iter().any(|p| !(p.is(Faction::Harkonnen) || p.is(Faction::Tleilaxu)));
        self.enter_boolean_phase_action(condition, Phase::SelectingTraitors, Game::assign_face_dancers);
    }

    pub fn assign_face_dancers(&mut self) {
        let tleilaxu = self.get_player(Some(Faction::Tleilaxu));
        if let Some(tleilaxu) = tleilaxu {
            self.traitor_deck.as_mut().nre().shuffle();
            for _i in 0..3 {
                let leader = self.traitor_deck.as_mut().nre().draw();
                self.player_mut(tleilaxu).face_dancers.push(leader);
                self.player_mut(tleilaxu).known_non_traitors.add(leader);
            }
        }

        // Game.Enter(bool, Action, Action) is static in C#; inlined here.
        if self.treachery_cards_before_traitors() {
            self.setup_spice_and_forces();
        } else {
            self.assign_leader_skills();
        }
    }

    pub fn assign_leader_skills(&mut self) {
        if self.applicable(Rule::LeaderSkills) {
            self.skill_deck = Some(Deck::new(get_values_except_default(LeaderSkill::None), self.random.clone()));
            self.skill_deck.as_mut().nre().shuffle();
            self.stone(Milestone::Shuffled);

            let nr_of_skills_to_assign = if self.players.len() <= 7 { 2 } else { 1 };
            for _i in 0..nr_of_skills_to_assign {
                for p in 0..self.players.len() {
                    let skill = self.skill_deck.as_mut().nre().draw();
                    self.players[p].skills_to_choose_from.push(skill);
                }
            }

            self.enter(Phase::AssigningInitialSkills, true);
        } else {
            // C# parity (hazards 18.3): TreacheryCardsBeforeTraitors requires LeaderSkills, so
            // this always calls SetupSpiceAndForces. Game.Enter(bool, Action, Action) inlined.
            if self.treachery_cards_before_traitors() {
                self.deal_traitors();
            } else {
                self.setup_spice_and_forces();
            }
        }
    }

    pub fn setup_spice_and_forces(&mut self) {
        for p in 0..self.players.len() {
            self.setup_player_homeworld(self.player_at(p));
        }

        if self.applicable(Rule::CustomInitialForcesAndResources) {
            self.has_acted_or_passed.clear();
            self.enter(Phase::PerformCustomSetup, true);
        } else {
            for p in 0..self.players.len() {
                self.setup_player_spice_and_forces_on_planet(self.player_at(p));
            }

            let method_after_setting_up: fn(&mut Game) = if self.treachery_cards_before_traitors() {
                Game::enter_storm_phase
            } else {
                Game::deal_starting_treachery_cards
            };

            // All conditions are evaluated before the call, as in C#.
            let c1 = self.is_playing(Faction::Fremen);
            let c2 = self.is_playing(Faction::BeneGesserit) && PerformBeneGesseritPlacement::bene_gesserit_may_place_first_force_in_any_territory(self);
            let c3 = self.is_playing(Faction::Moritani);
            self.enter_boolean_phase_boolean_phase_boolean_phase_action(
                c1,
                Phase::FremenSettingUp,
                c2,
                Phase::BeneGesseritSettingUp,
                c3,
                Phase::MoritaniSettingUp,
                method_after_setting_up,
            );
        }
    }

    pub fn setup_player_homeworld(&mut self, p: PlayerIndex) {
        let faction = self.player(p).faction;
        let normal_force_world = self
            .map
            .homeworlds()
            .cs_first_where(|w| w.faction() == faction && w.is_home_of_normal_forces());

        match faction {
            Faction::Fremen => {
                let normal = if self.applicable(Rule::FremenSpecialForces) { 17 } else { 20 };
                let special = if self.applicable(Rule::FremenSpecialForces) { 3 } else { 0 };
                Player::initialize_homeworld(self, p, normal_force_world, normal, special);
            }

            Faction::Emperor => {
                let normal = if self.applicable(Rule::EmperorSpecialForces) { 15 } else { 20 };
                Player::initialize_homeworld(self, p, normal_force_world, normal, 0);
                if self.applicable(Rule::EmperorSpecialForces) {
                    let faction = self.player(p).faction;
                    let special_world = self
                        .map
                        .homeworlds()
                        .cs_first_where(|w| w.faction() == faction && w.is_home_of_special_forces());
                    Player::initialize_homeworld(self, p, special_world, 0, 5);
                }
            }

            Faction::Ixian => {
                Player::initialize_homeworld(self, p, normal_force_world, 13, 7);
            }

            Faction::Atreides
            | Faction::Harkonnen
            | Faction::Guild
            | Faction::BeneGesserit
            | Faction::Tleilaxu
            | Faction::Choam
            | Faction::Richese
            | Faction::Ecaz
            | Faction::Moritani => {
                Player::initialize_homeworld(self, p, normal_force_world, 20, 0);
            }

            _ => {}
        }
    }

    pub fn setup_player_spice_and_forces_on_planet(&mut self, p: PlayerIndex) {
        match self.player(p).faction {
            Faction::Fremen => {
                self.player_mut(p).resources = 3;
            }

            Faction::Atreides => {
                self.player_mut(p).resources = 10;
                let l = self.map.arrakeen();
                Player::add_forces(self, p, l, 10, true);
            }

            Faction::Harkonnen => {
                self.player_mut(p).resources = 10;
                let l = self.map.carthag();
                Player::add_forces(self, p, l, 10, true);
            }

            Faction::Emperor => {
                self.player_mut(p).resources = 10;
            }

            Faction::Guild => {
                self.player_mut(p).resources = 5;
                let l = self.map.tueks_sietch();
                Player::add_forces(self, p, l, 5, true);
            }

            Faction::BeneGesserit => {
                self.player_mut(p).resources = 5;
                if !PerformBeneGesseritPlacement::bene_gesserit_may_place_first_force_in_any_territory(self) {
                    let l = self.map.polar_sink();
                    Player::add_forces(self, p, l, 1, true);
                }
            }

            Faction::Ixian => {
                self.player_mut(p).resources = 10;
                let l: Location = self.map.hidden_mobile_stronghold().into();
                Player::add_forces(self, p, l, 3, true);
                Player::add_special_forces(self, p, l, 3, true);
            }

            Faction::Tleilaxu => {
                self.player_mut(p).resources = 5;
            }

            Faction::Choam => {
                self.player_mut(p).resources = 2;
            }

            Faction::Richese => {
                self.player_mut(p).resources = 5;
            }

            Faction::Ecaz => {
                self.player_mut(p).resources = 12;
                let l = self.map.imperial_basin().middle_location();
                Player::add_forces(self, p, l, 6, true);
            }

            Faction::Moritani => {
                self.player_mut(p).resources = 12;
            }

            _ => {}
        }
    }

    pub fn assign_initial_ambassadors(&mut self, p: PlayerIndex) {
        self.player_mut(p).ambassadors.push(Ambassador::Ecaz);
        let items = &mut self.unassigned_ambassadors.as_mut().nre().items;
        if let Some(i) = items.iter().position(|a| *a == Ambassador::Ecaz) {
            items.remove(i);
        }
        let faction = self.player(p).faction;
        self.log(vec![faction.into(), " receive the ".into(), Faction::Ecaz.into(), " ambassador".into()]);
        self.assign_random_ambassadors(p);
    }

    pub fn assign_random_ambassadors(&mut self, p: PlayerIndex) {
        let set_aside = self.ambassadors_set_aside.clone();
        for item in set_aside {
            self.unassigned_ambassadors.as_mut().nre().items.push(item);
        }
        self.ambassadors_set_aside.clear();

        self.unassigned_ambassadors.as_mut().nre().shuffle();
        self.stone(Milestone::Shuffled);
        let faction = self.player(p).faction;
        self.log(vec![faction.into(), " get 5 random Ambassadors".into()]);
        for _i in 0..5 {
            let ambassador = self.unassigned_ambassadors.as_mut().nre().draw();
            self.player_mut(p).ambassadors.push(ambassador);
        }
    }

    pub fn deal_starting_treachery_cards(&mut self) {
        self.starting_treachery_cards = Some(Deck::empty(self.random.clone()));
        for p in 0..self.players.len() {
            let card = self.treachery_deck.as_mut().nre().draw();
            self.starting_treachery_cards.as_mut().nre().items.push(card);

            if self.players[p].is(Faction::Harkonnen) {
                self.extra_starting_card_for_harkonnen = Some(self.treachery_deck.as_mut().nre().draw());
            }
        }

        let condition = self.is_playing(Faction::Ixian);
        self.enter_boolean_phase_action(
            condition,
            Phase::IxianSelectingCard,
            Game::deal_remaining_starting_treachery_cards_to_non_ixian,
        );
    }

    pub fn deal_remaining_starting_treachery_cards_to_non_ixian(&mut self) {
        for i in 0..self.players.len() {
            if self.players[i].faction == Faction::Ixian {
                continue;
            }
            let p = self.player_at(i);
            let card = self.starting_treachery_cards.as_mut().nre().draw();
            self.player_mut(p).treachery_cards.push(card);
            let faction = self.player(p).faction;
            self.log_to(faction, vec!["Your starting treachery card is: ".into(), card.into()]);

            if self.player(p).is(Faction::Harkonnen) {
                if let Some(extra) = self.extra_starting_card_for_harkonnen {
                    self.player_mut(p).treachery_cards.push(extra);
                    self.log_to(Faction::Harkonnen, vec!["Your extra card is: ".into(), extra.into()]);
                }
            }
        }

        // Game.Enter(bool, Action, Action) is static in C#; inlined here.
        if self.treachery_cards_before_traitors() {
            self.assign_leader_skills();
        } else {
            self.enter_storm_phase();
        }
    }

    pub fn treachery_cards_before_traitors(&self) -> bool {
        self.applicable(Rule::LeaderSkills)
    }
}
