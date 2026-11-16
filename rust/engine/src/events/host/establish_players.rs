use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct EstablishPlayers {
    pub settings: Option<GameSettings>,
    pub seed: i32,
    pub maximum_number_of_players: u16,
    pub maximum_turns: Turn,
    pub applicable_rules: Option<Vec<Rule>>,
    #[serde(with = "crate::events::wire::name_list")]
    #[ts(as = "Option<Vec<String>>")]
    pub players: Option<String>,
    #[serde(with = "crate::events::wire::faction_list")]
    #[ts(as = "Option<Vec<Faction>>")]
    pub factions_in_play: Option<String>,
}

impl Default for EstablishPlayers {
    fn default() -> Self {
        EstablishPlayers {
            settings: Some(Default::default()),
            seed: Default::default(),
            maximum_number_of_players: 6,
            maximum_turns: Turn::new(10),
            applicable_rules: Some(Default::default()),
            players: Some(String::new()),
            factions_in_play: Some("".to_string()),
        }
    }
}

impl EstablishPlayers {
    pub fn players(&self) -> Vec<String> {
        // `_players == ""` is false for null, so a null `_players` throws on Split.
        let players = self.players.as_deref();
        if players == Some("") {
            return Vec::new();
        }
        players.expect("players").split('>').map(|s| s.to_string()).collect()
    }

    pub fn factions_in_play(&self) -> Vec<Faction> {
        match self.factions_in_play.as_deref() {
            None | Some("") => Vec::new(),
            Some(s) => s
                .split(',')
                // "None" and numbers that name no faction are left out.
                .filter_map(|part| <Faction as crate::enum_type::GameEnum>::from_value(dotnet_compat::parse::enum_parse(Some(part), Faction::INFO.declared)))
                .collect(),
        }
    }

    /// [`EstablishPlayers::validate`], and with `seated_users` also whether that many users fit
    /// at the table. Only the server knows how many users hold or wait for a seat, so it checks
    /// this before it submits the event.
    pub fn validate_for_table(&self, game: &Game, seated_users: Option<i32>) -> Option<Message> {
        if game.phase() != Phase::AwaitingPlayers {
            return Some(express!("Invalid game phase"));
        }
        let settings = self.settings.as_ref().expect("settings");
        if settings.number_of_players < 2 {
            return Some(express!("At least two players required"));
        }
        if settings.number_of_players > settings.allowed_factions_in_play.as_ref().expect("allowed factions in play").len() as i32 {
            return Some(express!("More factions required"));
        }
        // Not in C#: a game of 0 turns never ended (behavior-changes.md, "Turns, sectors and
        // amounts out of range").
        if settings.maximum_turns < Turn::FIRST {
            return Some(express!("At least one turn required"));
        }
        if seated_users.is_some_and(|n| n > settings.number_of_players) {
            return Some(express!("More players than seats at the table"));
        }
        if settings
            .allowed_factions_in_play
            .as_ref()
            .expect("allowed factions in play")
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

    pub fn create_terror_tokens(&self, game: &mut Game) {
        game.board.unplaced_terror_tokens = vec![
            TerrorType::Assassination,
            TerrorType::Atomics,
            TerrorType::Extortion,
            TerrorType::Robbery,
            TerrorType::Sabotage,
            TerrorType::SneakAttack,
        ];
    }

    pub fn create_nexus_deck(&self, game: &mut Game) {
        game.decks.nexus_card_deck = Some(Deck::new(Self::available_factions()));
        game.decks.nexus_card_deck.as_mut().expect("nexus card deck").shuffle(&mut game.rng);
    }

    /// Takes the factions players already have out of the factions in play. A validated
    /// `EstablishPlayers` comes before anyone has a faction, so only a savegame replayed without
    /// validation can get here with claimed factions. The settings' list and the game's list were
    /// one object in the C# engine, so both lose the faction. The event's own settings don't.
    pub fn remove_claimed_factions(&self, game: &mut Game) {
        let claimed: Vec<Faction> = game.players.iter().filter(|p| !p.is(None)).filter_map(|p| p.faction).collect();
        for f in claimed {
            let settings = game.rules.settings.as_mut().expect("settings");
            for list in [settings.allowed_factions_in_play.as_mut().expect("allowed factions in play"), &mut game.rules.factions_in_play] {
                if let Some(i) = list.iter().position(|x| *x == f) {
                    list.remove(i);
                }
            }
        }
    }

    pub fn create_and_shuffle_resource_card_deck(&self, game: &mut Game) -> Deck<ResourceCard> {
        let mut result = Deck::empty();
        // PutOnTop one by one reverses the source order.
        for c in Map::get_resource_cards_in_play(&*game) {
            result.put_on_top(c);
        }

        game.stone(Milestone::Shuffled);
        result.shuffle(&mut game.rng);
        result
    }

    pub fn create_discovery_tokens(&self, game: &mut Game) {
        game.decks.fremen_discovery_tokens = Some(Deck::empty());
        game.decks.guild_discovery_tokens = Some(Deck::empty());

        if game.applicable(Rule::DiscoveryTokens) {
            let fremen = game.decks.fremen_discovery_tokens.as_mut().expect("fremen discovery tokens");
            fremen.items.push(DiscoveryToken::Jacurutu);
            fremen.items.push(DiscoveryToken::Shrine);
            fremen.items.push(DiscoveryToken::TestingStation);
            fremen.items.push(DiscoveryToken::Cistern);
            fremen.shuffle(&mut game.rng);

            let guild = game.decks.guild_discovery_tokens.as_mut().expect("guild discovery tokens");
            guild.items.push(DiscoveryToken::ProcessingStation);
            guild.items.push(DiscoveryToken::CardStash);
            guild.items.push(DiscoveryToken::ResourceStash);
            guild.items.push(DiscoveryToken::Flight);
            guild.shuffle(&mut game.rng);
        }
    }

    pub fn fill_seats(&self, game: &mut Game) {
        while (game.players.len() as i32) < game.rules.settings.as_ref().expect("settings").number_of_players {
            let player = Player::new(None);
            game.players.push(player);
        }
    }
}

impl EventRules for EstablishPlayers {
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        match self.validate_for_table(game, None) {
            None => Ok(()),
            Some(m) => Err(Rejection(m)),
        }
    }

    fn apply(&self, game: &mut Game, _by: By) -> GameResult {
        game.start_report(MainPhase::Setup);
        game.log(vec!["Game started!".into()]);
        game.stone(Milestone::GameStarted);

        game.turn.current_main_phase = MainPhase::Setup;
        game.seed = self.seed;
        game.rng = crate::random::new_rng(self.seed);

        //Settings

        // A copy of the event's settings. A null Settings is stored, and the next statement
        // fails the event.
        game.rules.settings = self.settings.clone();

        //Rules

        // A null InitialRules is stored, and the filter below panics.
        game.rules.all_rules = game.rules.settings.as_ref().required("settings")?.initial_rules.clone();

        game.rules.active = game.rules.all_rules.as_ref().expect("rules").iter().copied().filter(|r| Game::get_rule_group(*r) != Some(RuleGroup::Bots)).collect();
        game.rules.rules_for_bots =
            game.rules.all_rules.as_ref().expect("rules").iter().copied().filter(|r| Game::get_rule_group(*r) == Some(RuleGroup::Bots)).collect();
        let core_basic = Game::get_rules_in_group(Some(RuleGroup::CoreBasic), Game::EXPANSION_LEVEL);
        game.rules.active.extend(core_basic);

        let allowed = game.rules.settings.as_ref().required("settings")?.allowed_factions_in_play.clone().required("allowed factions in play")?;
        game.rules.ruleset = Some(Game::determine_approximate_ruleset(&allowed, &game.rules.active, Game::EXPANSION_LEVEL));
        let ruleset = game.rules.ruleset;
        game.log(vec!["Ruleset: ".into(), ruleset.into()]);

        let custom_rules: Vec<Rule> = game.get_custom_rules();
        game.log_if(!custom_rules.is_empty(), vec!["House rules: ".into(), custom_rules.into()]);

        if game.applicable(Rule::ExpansionTreacheryCards) {
            if !game.rules.active.contains(&Rule::ExpansionTreacheryCardsExceptPBandSSandAmal) {
                game.rules.active.push(Rule::ExpansionTreacheryCardsExceptPBandSSandAmal);
            }
            if !game.rules.active.contains(&Rule::ExpansionTreacheryCardsPBandSs) {
                game.rules.active.push(Rule::ExpansionTreacheryCardsPBandSs);
            }
            if !game.rules.active.contains(&Rule::ExpansionTreacheryCardsAmal) {
                game.rules.active.push(Rule::ExpansionTreacheryCardsAmal);
            }
        }

        //Decks and card piles

        game.decks.resource_card_deck = Some(self.create_and_shuffle_resource_card_deck(game));
        game.decks.treachery_deck = Some(TreacheryCardManager::create_treachery_deck(&*game));
        self.create_discovery_tokens(game);

        if !game.applicable(Rule::CustomDecks) {
            game.decks.treachery_deck.as_mut().required("treachery deck")?.shuffle(&mut game.rng);
            game.stone(Milestone::Shuffled);
        }

        game.decks.treachery_discard_pile = Some(Deck::empty());
        game.decks.resource_card_discard_pile_a = Some(Deck::empty());
        game.decks.resource_card_discard_pile_b = Some(Deck::empty());

        if game.applicable(Rule::NexusCards) {
            self.create_nexus_deck(game);
        }

        self.create_terror_tokens(game);

        game.decks.unassigned_ambassadors = Some(Deck::new(
            Self::available_factions().into_iter().filter(|f| *f != Faction::Moritani).filter_map(Game::ambassador_of),
        ));

        //Initial alliance settings

        game.alliances.guild_allows_shipping_discount = true;
        game.alliances.tleilaxu_allows_revival_discount = true;
        game.alliances.ixian_allows_replacing_cards = true;
        game.alliances.emperor_will_pay_for_extra_revival = 0;
        game.alliances.fremen_will_protect_from_monster = true;
        game.alliances.fremen_allows_three_free_revivals = true;
        game.alliances.fremen_shares_prescience = true;
        game.alliances.fremen_refunds_battle_dial = true;
        game.alliances.atreides_shares_prescience = true;
        game.alliances.bene_gesserit_allows_use_of_voice = true;
        game.alliances.richese_allows_use_of_no_field = true;

        // A copy of the settings' allowed factions, which can't be null here: the ruleset above
        // needed them.
        game.rules.factions_in_play = game.rules.settings.as_ref().required("settings")?.allowed_factions_in_play.clone().required("allowed factions in play")?;

        self.fill_seats(game);
        self.remove_claimed_factions(game);

        game.determine_positions_at_table();

        if game.applicable(Rule::PlayersChooseFactions) {
            game.enter_setup(SetupStep::SelectingFactions);
        } else {
            game.assign_factions_and_enter_faction_trade();
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(EventType::EstablishPlayers.name(), " by ", by.faction)
    }
}
