use crate::prelude::*;

/// An open offer of `initiator` to trade factions with `target` (`FactionTradeOffered`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TradeOffer {
    pub initiator: Option<Faction>,
    pub target: Option<Faction>,
}

/// The setup (`PhaseState::Setup`), from choosing factions until the first storm: its step, the
/// faction trade offers and the starting treachery cards. It is dropped when the first storm
/// phase starts.
pub struct SetupState {
    pub step: SetupStep,
    pub current_trade_offers: Vec<TradeOffer>,
    pub starting_treachery_cards: Option<Deck<TreacheryCard>>,
    pub extra_starting_card_for_harkonnen: Option<TreacheryCard>,
}

impl SetupState {
    pub fn new(step: SetupStep) -> Self {
        SetupState {
            step,
            current_trade_offers: Vec::new(),
            starting_treachery_cards: None,
            extra_starting_card_for_harkonnen: None,
        }
    }
}

/// Where the setup is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupStep {
    SelectingFactions,
    TradingFactions,
    CustomizingDecks,
    BeneGesseritPredicting,
    HarkonnenMulligan,
    AssigningInitialSkills,
    SelectingTraitors,
    PerformCustomSetup,
    FremenSettingUp,
    BeneGesseritSettingUp,
    MoritaniSettingUp,
    IxianSelectingCard,
}

impl SetupStep {
    pub fn phase(self) -> Phase {
        match self {
            SetupStep::SelectingFactions => Phase::SelectingFactions,
            SetupStep::TradingFactions => Phase::TradingFactions,
            SetupStep::CustomizingDecks => Phase::CustomizingDecks,
            SetupStep::BeneGesseritPredicting => Phase::BeneGesseritPredicting,
            SetupStep::HarkonnenMulligan => Phase::HarkonnenMulligan,
            SetupStep::AssigningInitialSkills => Phase::AssigningInitialSkills,
            SetupStep::SelectingTraitors => Phase::SelectingTraitors,
            SetupStep::PerformCustomSetup => Phase::PerformCustomSetup,
            SetupStep::FremenSettingUp => Phase::FremenSettingUp,
            SetupStep::BeneGesseritSettingUp => Phase::BeneGesseritSettingUp,
            SetupStep::MoritaniSettingUp => Phase::MoritaniSettingUp,
            SetupStep::IxianSelectingCard => Phase::IxianSelectingCard,
        }
    }

    pub fn of(phase: Phase) -> Option<SetupStep> {
        let step = match phase {
            Phase::SelectingFactions => SetupStep::SelectingFactions,
            Phase::TradingFactions => SetupStep::TradingFactions,
            Phase::CustomizingDecks => SetupStep::CustomizingDecks,
            Phase::BeneGesseritPredicting => SetupStep::BeneGesseritPredicting,
            Phase::HarkonnenMulligan => SetupStep::HarkonnenMulligan,
            Phase::AssigningInitialSkills => SetupStep::AssigningInitialSkills,
            Phase::SelectingTraitors => SetupStep::SelectingTraitors,
            Phase::PerformCustomSetup => SetupStep::PerformCustomSetup,
            Phase::FremenSettingUp => SetupStep::FremenSettingUp,
            Phase::BeneGesseritSettingUp => SetupStep::BeneGesseritSettingUp,
            Phase::MoritaniSettingUp => SetupStep::MoritaniSettingUp,
            Phase::IxianSelectingCard => SetupStep::IxianSelectingCard,
            _ => return None,
        };
        Some(step)
    }
}

impl Game {
    /// The setup's state, while the game is in it (an interrupt may suspend it).
    pub fn setup(&self) -> Option<&SetupState> {
        match &self.phase {
            PhaseState::Setup(s) => Some(s),
            _ => None,
        }
    }

    pub fn setup_mut(&mut self) -> Option<&mut SetupState> {
        match &mut self.phase {
            PhaseState::Setup(s) => Some(s),
            _ => None,
        }
    }

    /// The setup's state, for code that only runs during the setup.
    pub(crate) fn setup_state(&mut self) -> &mut SetupState {
        self.setup_mut().expect("the setup")
    }

    /// Moves the setup to `step`, dropping any interrupt, as entering a phase did. Starts the
    /// setup if the game isn't in it yet.
    pub fn enter_setup(&mut self, step: SetupStep) {
        match self.setup_mut() {
            Some(s) => {
                s.step = step;
                self.interrupts.clear();
                self.remove_ended_deals(step.phase());
            }
            None => self.enter_state(PhaseState::Setup(SetupState::new(step))),
        }
    }

    /// Enters setup step `step` if `condition` holds, or else runs `otherwise`.
    pub fn enter_setup_or_else(&mut self, condition: bool, step: SetupStep, otherwise: fn(&mut Game) -> GameResult) -> GameResult {
        if condition {
            self.enter_setup(step);
            Ok(())
        } else {
            otherwise(self)
        }
    }
}

impl Game {
    pub fn next_faction_to_perform_custom_setup(&self) -> Option<Faction> {
        self.players.iter().filter_map(|p| p.faction).find(|f| !self.turn.has_acted_or_passed.contains(f))
    }

    pub fn enter_phase_awaiting_players(&mut self) {
        self.players = Vec::new();
        self.turn.current_turn = Turn::BEFORE_START;
        self.enter_state(PhaseState::AwaitingPlayers);
    }

    pub fn assign_factions_and_enter_faction_trade(&mut self) {
        // The Deck constructor copies its source, so evaluating the filter here is equivalent.
        let all = self.rules.factions_in_play.clone();
        let available_factions: Vec<Faction> = all.into_iter().filter(|f| !self.is_playing(*f)).collect();
        let mut selectable = Deck::new(available_factions);
        self.stone(Milestone::Shuffled);
        selectable.shuffle(&mut self.rng);

        // foreach over a lazy Where: the predicate is checked per player, before its body.
        for i in 0..self.players.len() {
            if self.players[i].faction == None {
                self.players[i].faction = Some(selectable.draw());
            }
        }

        self.enter_phase_trading_factions();
    }

    pub fn determine_positions_at_table(&mut self) {
        let mut seats: Deck<Seat> = Deck::empty();
        let mut i = 0;
        while i < self.rules.settings.as_ref().expect("settings").number_of_players {
            seats.put_on_top(Seat::from_number(i).expect("at most 255 players"));
            i += 1;
        }

        seats.shuffle(&mut self.rng);

        //Seat players
        for p in 0..self.players.len() {
            self.players[p].seat = Some(seats.draw());
        }
    }

    pub fn enter_phase_trading_factions(&mut self) {
        self.enter_setup(SetupStep::TradingFactions);
        self.setup_state().current_trade_offers.clear();
    }

    pub fn enter_setup_phase(&mut self) -> GameResult {
        self.setup_state().current_trade_offers.clear();

        // Players.OrderBy(p => p.Seat) is a stable sort.
        let mut by_seat: Vec<(Option<Seat>, Option<Faction>)> = self.players.iter().map(|p| (p.seat, p.faction)).collect();
        by_seat.sort_by_key(|(seat, _)| *seat);
        let first = by_seat.first().expect("a player").1.expect("faction");
        self.board.has_battle_wheel.push(first);
        let last = by_seat.last().expect("a player").1.expect("faction");
        self.board.has_battle_wheel.push(last);

        for p in 0..self.players.len() {
            Player::assign_leaders(self, self.player_at(p));
        }

        let ecaz = self.get_player(Some(Faction::Ecaz));
        if let Some(ecaz) = ecaz {
            self.assign_initial_ambassadors(ecaz);

            if self.applicable(Rule::EcazLoyalty) {
                let leaders = self.player(ecaz).leaders.clone();
                self.leaders.ecaz_loyal_leader = leaders.into_iter().random_item(&mut self.rng);
                let pll = self.leaders.ecaz_loyal_leader;
                self.log(vec![pll.into(), " is forever loyal to ".into(), Faction::Ecaz.into()]);
            }
        }

        if self.is_playing(Faction::BeneGesserit) {
            self.enter_setup(SetupStep::BeneGesseritPredicting);
            Ok(())
        } else if self.treachery_cards_before_traitors() {
            self.deal_starting_treachery_cards()
        } else {
            self.deal_traitors()
        }
    }

    pub fn deal_traitors(&mut self) -> GameResult {
        self.stone(Milestone::Shuffled);
        self.decks.traitor_deck = Some(self.create_and_shuffle_traitor_deck());

        if self.applicable(Rule::HarkonnenMulligan) && self.is_playing(Faction::Harkonnen) {
            self.deal_harkonnen_traitor_cards();
            self.enter_setup(SetupStep::HarkonnenMulligan);
        } else {
            for _i in 1..=4 {
                for p in 0..self.players.len() {
                    if self.players[p].faction != Some(Faction::Tleilaxu) {
                        let traitor = self.decks.traitor_deck.as_mut().expect("traitor deck").draw();
                        self.players[p].traitors.push(traitor);
                    }
                }
            }

            self.enter_select_traitors()?;
        }
        Ok(())
    }

    pub fn create_and_shuffle_traitor_deck(&mut self) -> Deck<Hero> {
        let mut result = Deck::new(self.traitors_in_play());

        if let Some(pll) = self.leaders.ecaz_loyal_leader {
            let hero = Hero::Leader(pll);
            if let Some(i) = result.items.iter().position(|h| *h == hero) {
                result.items.remove(i);
            }
            for p in self.players.iter_mut() {
                p.known_non_traitors.insert(hero);
            }
        }

        if let Some(vidal) = self.vidal() {
            let hero = Hero::Leader(vidal);
            for p in self.players.iter_mut() {
                p.known_non_traitors.insert(hero);
            }
        }

        result.shuffle(&mut self.rng);
        result
    }

    pub fn deal_harkonnen_traitor_cards(&mut self) {
        let Some(harkonnen) = self.get_player(Some(Faction::Harkonnen)) else {
            return;
        };
        for _i in 1..=4 {
            let traitor = self.decks.traitor_deck.as_mut().expect("traitor deck").draw();
            self.player_mut(harkonnen).traitors.push(traitor);
        }
    }

    pub fn enter_select_traitors(&mut self) -> GameResult {
        self.turn.has_acted_or_passed.clear();

        if self.is_playing(Faction::Harkonnen) {
            self.turn.has_acted_or_passed.push(Faction::Harkonnen);
        }

        if self.is_playing(Faction::Tleilaxu) {
            self.turn.has_acted_or_passed.push(Faction::Tleilaxu);
        }

        let condition = self.players.iter().any(|p| !(p.is(Faction::Harkonnen) || p.is(Faction::Tleilaxu)));
        self.enter_setup_or_else(condition, SetupStep::SelectingTraitors, Game::assign_face_dancers)
    }

    pub fn assign_face_dancers(&mut self) -> GameResult {
        let tleilaxu = self.get_player(Some(Faction::Tleilaxu));
        if let Some(tleilaxu) = tleilaxu {
            self.decks.traitor_deck.as_mut().expect("traitor deck").shuffle(&mut self.rng);
            for _i in 0..3 {
                let leader = self.decks.traitor_deck.as_mut().expect("traitor deck").draw();
                self.player_mut(tleilaxu).face_dancers.push(leader);
                self.player_mut(tleilaxu).known_non_traitors.insert(leader);
            }
        }

        if self.treachery_cards_before_traitors() {
            self.setup_spice_and_forces()?;
        } else {
            self.assign_leader_skills()?;
        }
        Ok(())
    }

    pub fn assign_leader_skills(&mut self) -> GameResult {
        if self.applicable(Rule::LeaderSkills) {
            self.decks.skill_deck = Some(Deck::new(get_values::<LeaderSkill>().collect::<Vec<_>>()));
            self.decks.skill_deck.as_mut().expect("skill deck").shuffle(&mut self.rng);
            self.stone(Milestone::Shuffled);

            let nr_of_skills_to_assign = if self.players.len() <= 7 { 2 } else { 1 };
            for _i in 0..nr_of_skills_to_assign {
                for p in 0..self.players.len() {
                    let skill = self.decks.skill_deck.as_mut().expect("skill deck").draw();
                    self.players[p].skills_to_choose_from.push(skill);
                }
            }

            self.enter_setup(SetupStep::AssigningInitialSkills);
        } else {
            // C# parity: TreacheryCardsBeforeTraitors requires LeaderSkills, so
            // this always calls SetupSpiceAndForces. Game.Enter(bool, Action, Action) inlined.
            if self.treachery_cards_before_traitors() {
                self.deal_traitors()?;
            } else {
                self.setup_spice_and_forces()?;
            }
        }
        Ok(())
    }

    pub fn setup_spice_and_forces(&mut self) -> GameResult {
        for p in 0..self.players.len() {
            self.setup_player_homeworld(self.player_at(p))?;
        }

        if self.applicable(Rule::CustomInitialForcesAndResources) {
            self.turn.has_acted_or_passed.clear();
            self.enter_setup(SetupStep::PerformCustomSetup);
        } else {
            for p in 0..self.players.len() {
                self.setup_player_spice_and_forces_on_planet(self.player_at(p))?;
            }

            let method_after_setting_up: fn(&mut Game) -> GameResult = if self.treachery_cards_before_traitors() {
                Game::enter_storm_phase
            } else {
                Game::deal_starting_treachery_cards
            };

            // All conditions are evaluated before the call.
            let c1 = self.is_playing(Faction::Fremen);
            let c2 = self.is_playing(Faction::BeneGesserit) && PerformBeneGesseritPlacement::bene_gesserit_may_place_first_force_in_any_territory(self);
            let c3 = self.is_playing(Faction::Moritani);
            if c1 {
                self.enter_setup(SetupStep::FremenSettingUp);
            } else if c2 {
                self.enter_setup(SetupStep::BeneGesseritSettingUp);
            } else if c3 {
                self.enter_setup(SetupStep::MoritaniSettingUp);
            } else {
                method_after_setting_up(self)?;
            }
        }
        Ok(())
    }

    pub fn setup_player_homeworld(&mut self, p: PlayerId) -> GameResult {
        let faction = self.player(p).faction;
        let normal_force_world = self
            .board.map
            .homeworlds()
            .find(|w| Some(w.faction()) == faction && w.is_home_of_normal_forces()).expect("a normal force homeworld");

        match faction {
            Some(Faction::Fremen) => {
                let normal = if self.applicable(Rule::FremenSpecialForces) { 17 } else { 20 };
                let special = if self.applicable(Rule::FremenSpecialForces) { 3 } else { 0 };
                Player::initialize_homeworld(self, p, normal_force_world, normal, special)?;
            }

            Some(Faction::Emperor) => {
                let normal = if self.applicable(Rule::EmperorSpecialForces) { 15 } else { 20 };
                Player::initialize_homeworld(self, p, normal_force_world, normal, 0)?;
                if self.applicable(Rule::EmperorSpecialForces) {
                    let faction = self.player(p).faction;
                    let special_world = self
                        .board.map
                        .homeworlds()
                        .find(|w| Some(w.faction()) == faction && w.is_home_of_special_forces()).expect("a special force homeworld");
                    Player::initialize_homeworld(self, p, special_world, 0, 5)?;
                }
            }

            Some(Faction::Ixian) => {
                Player::initialize_homeworld(self, p, normal_force_world, 13, 7)?;
            }

            Some(Faction::Atreides)
            | Some(Faction::Harkonnen)
            | Some(Faction::Guild)
            | Some(Faction::BeneGesserit)
            | Some(Faction::Tleilaxu)
            | Some(Faction::Choam)
            | Some(Faction::Richese)
            | Some(Faction::Ecaz)
            | Some(Faction::Moritani) => {
                Player::initialize_homeworld(self, p, normal_force_world, 20, 0)?;
            }

            _ => {}
        }
        Ok(())
    }

    pub fn setup_player_spice_and_forces_on_planet(&mut self, p: PlayerId) -> GameResult {
        match self.player(p).faction {
            Some(Faction::Fremen) => {
                self.player_mut(p).resources = 3;
            }

            Some(Faction::Atreides) => {
                self.player_mut(p).resources = 10;
                let l = self.board.map.arrakeen();
                Player::add_forces(self, p, l, 10, true)?;
            }

            Some(Faction::Harkonnen) => {
                self.player_mut(p).resources = 10;
                let l = self.board.map.carthag();
                Player::add_forces(self, p, l, 10, true)?;
            }

            Some(Faction::Emperor) => {
                self.player_mut(p).resources = 10;
            }

            Some(Faction::Guild) => {
                self.player_mut(p).resources = 5;
                let l = self.board.map.tueks_sietch();
                Player::add_forces(self, p, l, 5, true)?;
            }

            Some(Faction::BeneGesserit) => {
                self.player_mut(p).resources = 5;
                if !PerformBeneGesseritPlacement::bene_gesserit_may_place_first_force_in_any_territory(self) {
                    let l = self.board.map.polar_sink();
                    Player::add_forces(self, p, l, 1, true)?;
                }
            }

            Some(Faction::Ixian) => {
                self.player_mut(p).resources = 10;
                let l: Location = self.board.map.hidden_mobile_stronghold().into();
                Player::add_forces(self, p, l, 3, true)?;
                Player::add_special_forces(self, p, l, 3, true)?;
            }

            Some(Faction::Tleilaxu) => {
                self.player_mut(p).resources = 5;
            }

            Some(Faction::Choam) => {
                self.player_mut(p).resources = 2;
            }

            Some(Faction::Richese) => {
                self.player_mut(p).resources = 5;
            }

            Some(Faction::Ecaz) => {
                self.player_mut(p).resources = 12;
                let l = self.board.map.imperial_basin().middle_location();
                Player::add_forces(self, p, l, 6, true)?;
            }

            Some(Faction::Moritani) => {
                self.player_mut(p).resources = 12;
            }

            _ => {}
        }
        Ok(())
    }

    pub fn assign_initial_ambassadors(&mut self, p: PlayerId) {
        self.player_mut(p).ambassadors.push(Ambassador::Ecaz);
        let items = &mut self.decks.unassigned_ambassadors.as_mut().expect("unassigned ambassadors").items;
        if let Some(i) = items.iter().position(|a| *a == Ambassador::Ecaz) {
            items.remove(i);
        }
        let faction = self.player(p).faction;
        self.log(vec![faction.into(), " receive the ".into(), Faction::Ecaz.into(), " ambassador".into()]);
        self.assign_random_ambassadors(p);
    }

    pub fn assign_random_ambassadors(&mut self, p: PlayerId) {
        let set_aside = self.decks.ambassadors_set_aside.clone();
        for item in set_aside {
            self.decks.unassigned_ambassadors.as_mut().expect("unassigned ambassadors").items.push(item);
        }
        self.decks.ambassadors_set_aside.clear();

        self.decks.unassigned_ambassadors.as_mut().expect("unassigned ambassadors").shuffle(&mut self.rng);
        self.stone(Milestone::Shuffled);
        let faction = self.player(p).faction;
        self.log(vec![faction.into(), " get 5 random Ambassadors".into()]);
        for _i in 0..5 {
            let ambassador = self.decks.unassigned_ambassadors.as_mut().expect("unassigned ambassadors").draw();
            self.player_mut(p).ambassadors.push(ambassador);
        }
    }

    pub fn deal_starting_treachery_cards(&mut self) -> GameResult {
        self.setup_state().starting_treachery_cards = Some(Deck::empty());
        for p in 0..self.players.len() {
            let card = self.decks.treachery_deck.as_mut().expect("treachery deck").draw();
            self.drop_copy_in_richese_cache(card);
            self.setup_state().starting_treachery_cards.as_mut().expect("starting treachery cards").items.push(card);

            if self.players[p].is(Faction::Harkonnen) {
                let extra = self.decks.treachery_deck.as_mut().expect("treachery deck").draw();
                self.drop_copy_in_richese_cache(extra);
                self.setup_state().extra_starting_card_for_harkonnen = Some(extra);
            }
        }

        let condition = self.is_playing(Faction::Ixian);
        self.enter_setup_or_else(condition, SetupStep::IxianSelectingCard, Game::deal_remaining_starting_treachery_cards_to_non_ixian)
    }

    pub fn deal_remaining_starting_treachery_cards_to_non_ixian(&mut self) -> GameResult {
        for i in 0..self.players.len() {
            if self.players[i].faction == Some(Faction::Ixian) {
                continue;
            }
            let p = self.player_at(i);
            let card = self.setup_state().starting_treachery_cards.as_mut().expect("starting treachery cards").draw();
            self.player_mut(p).treachery_cards.push(card);
            let faction = self.player(p).faction;
            self.log_to(faction.expect("faction"), vec!["Your starting treachery card is: ".into(), card.into()]);

            if self.player(p).is(Faction::Harkonnen) {
                if let Some(extra) = self.setup_state().extra_starting_card_for_harkonnen {
                    self.player_mut(p).treachery_cards.push(extra);
                    self.log_to(Faction::Harkonnen, vec!["Your extra card is: ".into(), extra.into()]);
                }
            }
        }

        if self.treachery_cards_before_traitors() {
            self.assign_leader_skills()?;
        } else {
            self.enter_storm_phase()?;
        }
        Ok(())
    }

    pub fn treachery_cards_before_traitors(&self) -> bool {
        self.applicable(Rule::LeaderSkills)
    }
}

impl GameStatus {
    /// The status in the setup, or `None` in another phase.
    pub(crate) fn setup_status(game: &Game, _me: Option<PlayerId>, phase: Phase) -> Option<GameStatus> {
        Some(match phase {
            Phase::AwaitingPlayers => Self::host_status(
                "Configure the game and wait for players to join. Start the game when ready.",
                "Waiting for the host to configure and start the game...",
                None,
            ),

            Phase::SelectingFactions => Self::status("Players may now choose a faction...", None),

            Phase::TradingFactions => Self::status("Players may now trade factions...", None),

            Phase::BeneGesseritPredicting => {
                let a = express!("Please predict who will win the game and when.");
                let w = express!(Faction::BeneGesserit, " are predicting who will win the game and when...");
                Self::faction_status(game, a, w, Some(Faction::BeneGesserit), None)
            }

            Phase::HarkonnenMulligan => {
                let a = express!("You may draw a new set of traitors if you were dealt two or more of your own leaders.");
                let w = express!(Faction::Harkonnen, " may draw a new set of traitors if they were dealt two or more of their own leaders...");
                Self::faction_status(game, a, w, Some(Faction::Harkonnen), None)
            }

            Phase::SelectingTraitors => {
                let players = Self::players_that_have_not_acted_or_passed(game);
                Self::players_status("Please select one leader to keep as a traitor.", "Factions are selecting traitors...", players, None)
            }

            Phase::CustomizingDecks => Self::host_status("Please select which cards will be in play.", "The host is selecting which cards will be in play...", None),

            Phase::PerformCustomSetup => {
                let a = express!("Please set up ", game.next_faction_to_perform_custom_setup(), " starting ", Concept::Resource, " and force positions.");
                let w = express!("The host is setting up ", game.next_faction_to_perform_custom_setup(), " starting ", Concept::Resource, " and force positions...");
                Self::host_status(a, w, None)
            }

            Phase::FremenSettingUp => {
                let a = express!("Please choose your initial force positions.");
                let w = express!(Faction::Fremen, " are setting up their starting force positions...");
                Self::faction_status(game, a, w, Some(Faction::Fremen), None)
            }

            Phase::BeneGesseritSettingUp => {
                let a = express!("Please select your starting force position.");
                let w = express!(Faction::BeneGesserit, " are selecting their starting force position...");
                Self::faction_status(game, a, w, Some(Faction::BeneGesserit), None)
            }

            Phase::MoritaniSettingUp => {
                let a = express!("Please select your starting force position.");
                let w = express!(Faction::Moritani, " are selecting their starting force position...");
                Self::faction_status(game, a, w, Some(Faction::Moritani), None)
            }

            Phase::IxianSelectingCard => {
                let a = express!("Please select your starting Treachery Card.");
                let w = express!(Faction::Ixian, " are selecting their starting Treachery Card...");
                Self::faction_status(game, a, w, Some(Faction::Ixian), None)
            }

            Phase::HmsPlacement => {
                let a = express!("Please position the Hidden Mobile Stronghold.");
                let w = express!(Faction::Ixian, " are positioning the Hidden Mobile Stronghold...");
                Self::faction_status(game, a, w, Some(Faction::Ixian), None)
            }

            _ => return None,
        })
    }
}

impl Game {
    /// What `player` may do in the setup (see `add_player_actions`).
    pub(crate) fn add_setup_actions(&self, player: PlayerId, _is_host: bool, result: &mut Vec<EventType>) {
        let faction = self.player(player).faction;
        match self.phase() {
            Phase::SelectingFactions => {
                if self.player(player).faction == None {
                    result.push(EventType::FactionSelected);
                }
            }

            Phase::TradingFactions => {
                if self.players.len() > 1 {
                    result.push(EventType::FactionTradeOffered);
                }
            }

            Phase::PerformCustomSetup => {}

            Phase::BeneGesseritPredicting => {
                if faction == Some(Faction::BeneGesserit) {
                    result.push(EventType::BeneGesseritPrediction);
                }
            }

            Phase::FremenSettingUp => {
                if faction == Some(Faction::Fremen) {
                    result.push(EventType::PerformFremenSetup);
                }
            }

            Phase::BeneGesseritSettingUp => {
                if faction == Some(Faction::BeneGesserit) {
                    result.push(EventType::PerformBeneGesseritPlacement);
                }
            }

            Phase::MoritaniSettingUp => {
                if faction == Some(Faction::Moritani) {
                    result.push(EventType::PerformMoritaniSetup);
                }
            }

            Phase::HarkonnenMulligan => {
                if faction == Some(Faction::Harkonnen) {
                    result.push(EventType::MulliganPerformed);
                }
            }

            Phase::SelectingTraitors => {
                if faction != Some(Faction::Harkonnen) && faction != Some(Faction::Tleilaxu) && !self.turn.has_acted_or_passed.contains_opt(faction) {
                    result.push(EventType::TraitorsSelected);
                }
            }

            Phase::IxianSelectingCard => {
                if faction == Some(Faction::Ixian) {
                    result.push(EventType::IxianSelectedStartingCard);
                }
            }

            Phase::HmsPlacement => {
                if faction == Some(Faction::Ixian) {
                    result.push(EventType::PerformHmsPlacement);
                }
            }

            _ => {}
        }
    }
}
