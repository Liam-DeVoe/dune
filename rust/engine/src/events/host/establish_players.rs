use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct EstablishPlayers {
    pub settings: Option<Shared<GameSettings>>,
    pub seed: i32,
    pub maximum_number_of_players: i32,
    pub maximum_turns: i32,
    pub applicable_rules: Option<Vec<Rule>>,
    #[canon(name = "_players")]
    pub players: Option<String>,
    #[canon(name = "_factionsInPlay")]
    pub factions_in_play: Option<String>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for EstablishPlayers {
    fn default() -> Self {
        EstablishPlayers {
            settings: Some(Shared::new(Default::default())),
            seed: Default::default(),
            maximum_number_of_players: 6,
            maximum_turns: 10,
            applicable_rules: Some(Default::default()),
            players: Some(String::new()),
            factions_in_play: Some("".to_string()),
            base: Default::default(),
        }
    }
}

impl EstablishPlayers {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn players(&self) -> Vec<String> {
        // `_players == ""` is false for null, so a null `_players` throws on Split.
        let players = self.players.as_deref();
        if players == Some("") {
            return Vec::new();
        }
        players.expect("players").split('>').map(|s| s.to_string()).collect()
    }

    pub fn set_players(&mut self, value: &[String]) {
        self.players = Some(value.join(">"));
    }

    pub fn factions_in_play(&self) -> Vec<Faction> {
        match self.factions_in_play.as_deref() {
            None | Some("") => Vec::new(),
            Some(s) => s
                .split(',')
                .map(|part| Faction(dotnet_compat::parse::enum_parse(Some(part), Faction::INFO.declared)))
                .collect(),
        }
    }

    pub fn set_factions_in_play(&mut self, value: &[Faction]) {
        self.factions_in_play = Some(value.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(","));
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if game.current_phase != Phase::AwaitingPlayers {
            return Some(express!("Invalid game phase"));
        }
        let settings = self.settings.as_ref().expect("settings").borrow();
        if settings.number_of_players < 2 {
            return Some(express!("At least two players required"));
        }
        if settings.number_of_players > settings.allowed_factions_in_play.as_ref().expect("allowed factions in play").borrow().len() as i32 {
            return Some(express!("More factions required"));
        }
        if game.participation.borrow().seated_players.len() as i32 > settings.number_of_players {
            return Some(express!("More players than seats at the table"));
        }
        if settings
            .allowed_factions_in_play
            .as_ref()
            .expect("allowed factions in play")
            .borrow()
            .iter()
            .any(|f| !Self::available_factions().contains(f))
        {
            return Some(express!("Invalid faction"));
        }

        None
    }

    pub fn get_maximum_number_of_players() -> i32 {
        Self::available_factions().len() as i32
    }

    pub fn get_maximum_number_of_turns() -> i32 {
        20
    }

    pub fn available_factions() -> Vec<Faction> {
        let mut result = Vec::new();

        if Game::EXPANSION_LEVEL >= 0 {
            result.push(Faction::Atreides);
            result.push(Faction::Harkonnen);
            result.push(Faction::Fremen);
            result.push(Faction::Emperor);
            result.push(Faction::Guild);
            result.push(Faction::BeneGesserit);
        }

        if Game::EXPANSION_LEVEL >= 1 {
            result.push(Faction::Ixian);
            result.push(Faction::Tleilaxu);
        }

        if Game::EXPANSION_LEVEL >= 2 {
            result.push(Faction::Choam);
            result.push(Faction::Richese);
        }

        if Game::EXPANSION_LEVEL >= 3 {
            result.push(Faction::Ecaz);
            result.push(Faction::Moritani);
        }

        result
    }

    pub fn available_rule_groups() -> Vec<RuleGroup> {
        let mut result = Vec::new();

        if Game::EXPANSION_LEVEL >= 0 {
            result.push(RuleGroup::CoreAdvanced);
            result.push(RuleGroup::CoreBasicExceptions);
            result.push(RuleGroup::CoreAdvancedExceptions);
        }

        if Game::EXPANSION_LEVEL >= 1 {
            result.push(RuleGroup::ExpansionIxAndBtBasic);
            result.push(RuleGroup::ExpansionIxAndBtAdvanced);
        }

        if Game::EXPANSION_LEVEL >= 2 {
            result.push(RuleGroup::ExpansionChoamAndRicheseBasic);
            result.push(RuleGroup::ExpansionChoamAndRicheseAdvanced);
        }

        if Game::EXPANSION_LEVEL >= 3 {
            result.push(RuleGroup::ExpansionEcazAndMoritaniBasic);
            result.push(RuleGroup::ExpansionEcazAndMoritaniAdvanced);
        }

        if Game::EXPANSION_LEVEL >= 0 {
            result.push(RuleGroup::House);
        }

        result
    }

    /// Overrides `GameEvent.Execute` (hazards 1.2): no try/catch, no applicability check,
    /// `RecentMilestones` is not cleared.
    pub fn try_execute(self: &Rc<Self>, game: &mut Game, perform_validation: bool, _is_host: bool) -> GameResult<Option<Message>> {
        if perform_validation {
            if let Some(refusal) = self.validate(game) {
                return Ok(Some(refusal));
            }
        }

        let ev = self.clone().into_event();
        game.perform_pre_event_tasks(&ev);
        self.execute_concrete_event(game)?;
        game.perform_post_event_tasks(&ev, true);
        Ok(None)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.start_report(MainPhase::Setup);
        self.log_objects(game, vec!["Game started!".into()]);
        game.stone(Milestone::GameStarted);

        game.current_main_phase = MainPhase::Setup;
        game.seed = self.seed;
        game.random = Some(SharedRandom::from_seed(self.seed));

        //Settings

        // Shares the event's own settings object (hazards 5.1). A null Settings is stored,
        // and the next statement throws NullReferenceException on Game.Settings.InitialRules.
        game.settings = self.settings.clone();

        //Rules

        // AllRules shares Settings.InitialRules; nothing mutates either afterwards, so a copy
        // is equivalent (engine-conventions.md). A null InitialRules is stored; the Where below
        // throws ArgumentNullException.
        game.all_rules = game.settings.as_ref().required("settings")?.borrow().initial_rules.clone();

        game.rules = game.all_rules.as_ref().expect("rules").iter().copied().filter(|r| Game::get_rule_group(*r) != RuleGroup::Bots).collect();
        game.rules_for_bots =
            game.all_rules.as_ref().expect("rules").iter().copied().filter(|r| Game::get_rule_group(*r) == RuleGroup::Bots).collect();
        let core_basic = Game::get_rules_in_group(RuleGroup::CoreBasic, Game::EXPANSION_LEVEL);
        game.rules.extend(core_basic);

        let allowed = game.settings.as_ref().required("settings")?.borrow().allowed_factions_in_play.clone().required("allowed factions in play")?;
        game.ruleset = Game::determine_approximate_ruleset(&allowed.borrow(), &game.rules, Game::EXPANSION_LEVEL);
        let ruleset = game.ruleset;
        self.log_objects(game, vec!["Ruleset: ".into(), ruleset.into()]);

        let custom_rules: Vec<Rule> = game.get_custom_rules();
        self.log_if(game, !custom_rules.is_empty(), vec!["House rules: ".into(), custom_rules.into()]);

        if game.applicable(Rule::ExpansionTreacheryCards) {
            if !game.rules.contains(&Rule::ExpansionTreacheryCardsExceptPBandSSandAmal) {
                game.rules.push(Rule::ExpansionTreacheryCardsExceptPBandSSandAmal);
            }
            if !game.rules.contains(&Rule::ExpansionTreacheryCardsPBandSs) {
                game.rules.push(Rule::ExpansionTreacheryCardsPBandSs);
            }
            if !game.rules.contains(&Rule::ExpansionTreacheryCardsAmal) {
                game.rules.push(Rule::ExpansionTreacheryCardsAmal);
            }
        }

        //Decks and card piles

        game.resource_card_deck = Some(self.create_and_shuffle_resource_card_deck(game));
        game.treachery_deck = Some(TreacheryCardManager::create_treachery_deck(&*game, game.random.clone()));
        self.create_discovery_tokens(game);

        if !game.applicable(Rule::CustomDecks) {
            game.treachery_deck.as_mut().required("treachery deck")?.shuffle();
            game.stone(Milestone::Shuffled);
        }

        game.treachery_discard_pile = Some(Deck::empty(game.random.clone()));
        game.resource_card_discard_pile_a = Some(Deck::empty(game.random.clone()));
        game.resource_card_discard_pile_b = Some(Deck::empty(game.random.clone()));

        if game.applicable(Rule::NexusCards) {
            self.create_nexus_deck(game);
        }

        self.create_terror_tokens(game);

        game.unassigned_ambassadors = Some(Deck::new(
            Self::available_factions().into_iter().filter(|f| *f != Faction::Moritani).map(Game::ambassador_of),
            game.random.clone(),
        ));

        //Initial alliance settings

        game.guild_allows_shipping_discount = true;
        game.tleilaxu_allows_revival_discount = true;
        game.ixian_allows_replacing_cards = true;
        game.emperor_will_pay_for_extra_revival = 0;
        game.fremen_will_protect_from_monster = true;
        game.fremen_allows_three_free_revivals = true;
        game.fremen_shares_prescience = true;
        game.fremen_refunds_battle_dial = true;
        game.atreides_shares_prescience = true;
        game.bene_gesserit_allows_use_of_voice = true;
        game.richese_allows_use_of_no_field = true;

        // The same list object as Settings.AllowedFactionsInPlay (hazards 5.1). It cannot be
        // null here: DetermineApproximateRuleset above would have thrown.
        game.factions_in_play = game.settings.as_ref().required("settings")?.borrow().allowed_factions_in_play.clone().required("allowed factions in play")?;

        self.fill_seats(game);
        self.remove_claimed_factions(game);

        game.determine_positions_at_table();

        let condition = game.applicable(Rule::PlayersChooseFactions);
        game.enter_boolean_phase_action(condition, Phase::SelectingFactions, Game::assign_factions_and_enter_faction_trade);
        Ok(())
    }

    pub fn create_terror_tokens(&self, game: &mut Game) {
        game.unplaced_terror_tokens = vec![
            TerrorType::Assassination,
            TerrorType::Atomics,
            TerrorType::Extortion,
            TerrorType::Robbery,
            TerrorType::Sabotage,
            TerrorType::SneakAttack,
        ];
    }

    pub fn create_nexus_deck(&self, game: &mut Game) {
        game.nexus_card_deck = Some(Deck::new(Self::available_factions(), game.random.clone()));
        game.nexus_card_deck.as_mut().expect("nexus card deck").shuffle();
    }

    pub fn remove_claimed_factions(&self, game: &mut Game) {
        // The body only changes the factions list, not Players, so collecting first is equivalent.
        let claimed: Vec<Faction> = game.players.iter().filter(|p| !p.is(Faction::None)).map(|p| p.faction).collect();
        for f in claimed {
            let list = game.settings.as_ref().expect("settings").borrow().allowed_factions_in_play.clone().expect("allowed factions in play");
            let mut list = list.borrow_mut();
            if let Some(i) = list.iter().position(|x| *x == f) {
                list.remove(i);
            }
        }
    }

    pub fn create_and_shuffle_resource_card_deck(&self, game: &mut Game) -> Deck<ResourceCard> {
        let mut result = Deck::empty(game.random.clone());
        // PutOnTop one by one reverses the source order (hazards 2.3).
        for c in Map::get_resource_cards_in_play(&*game) {
            result.put_on_top(c);
        }

        game.stone(Milestone::Shuffled);
        result.shuffle();
        result
    }

    pub fn create_discovery_tokens(&self, game: &mut Game) {
        game.fremen_discovery_tokens = Some(Deck::empty(game.random.clone()));
        game.guild_discovery_tokens = Some(Deck::empty(game.random.clone()));

        if game.applicable(Rule::DiscoveryTokens) {
            let fremen = game.fremen_discovery_tokens.as_mut().expect("fremen discovery tokens");
            fremen.items.push(DiscoveryToken::Jacurutu);
            fremen.items.push(DiscoveryToken::Shrine);
            fremen.items.push(DiscoveryToken::TestingStation);
            fremen.items.push(DiscoveryToken::Cistern);
            fremen.shuffle();

            let guild = game.guild_discovery_tokens.as_mut().expect("guild discovery tokens");
            guild.items.push(DiscoveryToken::ProcessingStation);
            guild.items.push(DiscoveryToken::CardStash);
            guild.items.push(DiscoveryToken::ResourceStash);
            guild.items.push(DiscoveryToken::Flight);
            guild.shuffle();
        }
    }

    pub fn fill_seats(&self, game: &mut Game) {
        while (game.players.len() as i32) < game.settings.as_ref().expect("settings").borrow().number_of_players {
            let player = Player::new(Faction::None);
            game.players.push(player);
        }
    }
}

impl GameEventMethods for EstablishPlayers {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::EstablishPlayers
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::EstablishPlayers(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        EstablishPlayers::validate(self, game)
    }

    fn try_execute(self: &Rc<Self>, game: &mut Game, perform_validation: bool, is_host: bool) -> GameResult<Option<Message>> {
        EstablishPlayers::try_execute(self, game, perform_validation, is_host)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        EstablishPlayers::execute_concrete_event(self, game)
    }

}
