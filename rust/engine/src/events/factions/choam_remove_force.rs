use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct ChoamRemoveForce {
    pub target: Faction,
    pub special_force: bool,
    pub location_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for ChoamRemoveForce {
    fn default() -> Self {
        ChoamRemoveForce {
            target: Default::default(),
            special_force: Default::default(),
            location_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl ChoamRemoveForce {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn location(&self, game: &Game) -> Option<Location> {
        game.map.location_lookup().find(self.location_id)
    }

    pub fn set_location(&mut self, game: &Game, value: Option<Location>) {
        self.location_id = game.map.location_lookup().get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let location = self.location(game);
        let Some(l) = location else {
            return Some(express!("Invalid location"));
        };
        if !ChoamRemoveForce::valid_locations(game).any(|v| v == l) {
            return Some(express!("Invalid location"));
        }
        // Location is not null here, so ValidFactions never reads Player.
        if !ChoamRemoveForce::valid_factions(game, self.player_opt(), location).any(|v| v == self.target) {
            return Some(express!("Invalid faction"));
        }
        if !ChoamRemoveForce::valid_special_force_choices(game, location, self.target).contains(&self.special_force) {
            return Some(express!("Invalid type of forces"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_locations(g: &Game) -> impl Iterator<Item = Location> {
        // The dictionary is built at the call, as in C#; Where and Distinct run as the caller pulls.
        let keys: Vec<Location> = g.forces_on_planet_excluding_empty_locations(false).keys().copied().collect();
        crate::iter::unique(keys.into_iter().filter(move |l| !g.is_in_storm(*l)))
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_factions(g: &Game, p: Option<PlayerIndex>, l: Option<Location>) -> impl Iterator<Item = Faction> {
        use either::Either;
        if let Some(l) = l {
            return Either::Left(g.players.iter().filter(move |x| x.any_forces_in(Some(l)) > 0).map(|x| x.faction));
        }
        Either::Right(g.players_other_than(p.expect("player")))
    }

    pub fn valid_special_force_choices(g: &Game, l: Option<Location>, f: Faction) -> Vec<bool> {
        let mut result = Vec::new();
        let player_to_check = g.get_player(Some(f));

        match (l, player_to_check) {
            (Some(l), Some(pc)) => {
                if g.player(pc).forces_in(Some(l)) > 0 {
                    result.push(false);
                }

                if g.player(pc).special_forces_in(Some(l)) > 0 {
                    result.push(true);
                }
            }
            _ => {
                result.push(false);
                result.push(true);
            }
        }

        result
    }

    pub fn can_be_played_by(g: &Game, p: PlayerIndex) -> bool {
        let pl = g.player(p);
        pl.faction == Faction::Choam
            && ((!g.prevented(FactionAdvantage::ChoamDiscarding) && ChoamRemoveForce::card_to_use(g, p).is_some())
                || (NexusPlayed::can_use_cunning(g, p) && !pl.treachery_cards.is_empty()))
    }

    pub fn card_to_use(game: &Game, p: PlayerIndex) -> Option<TreacheryCard> {
        game.player(p).treachery_cards.iter().copied().find(|c| c.id() == TreacheryCardManager::CARD_TRIP_TO_GAMONT)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);

        let me = self.player();
        let card = ChoamRemoveForce::card_to_use(game, me);
        if card.is_none() && NexusPlayed::can_use_cunning(game, me) {
            game.play_nexus_card(me, vec!["send a force back to reserves".into()]);
            game.let_player_discard_treachery_card_of_choice(self.initiator());
        } else {
            game.discard_treachery_card_boolean(card, true)?;
        }

        let target = self.get_player(game, Some(self.target));

        let target = target.required("target")?;
        let location = self.location(game);
        if self.special_force {
            match location {
                Some(location) => Player::special_forces_to_reserves(game, target, location, 1),
                None => {
                    // The reserves part of `Player::special_forces_to_reserves`.
                    let faction = game.player(target).faction;
                    if faction == Faction::BeneGesserit {
                        Player::add_forces_to_reserves(game, target, 1);
                    } else if faction != Faction::Richese {
                        Player::add_special_forces_to_reserves(game, target, 1);
                    }
                    game_error!("No location to remove a force from");
                }
            }
        } else {
            match location {
                Some(location) => Player::forces_to_reserves_location_int32(game, target, location, 1),
                None => {
                    // The reserves part of `Player::forces_to_reserves_location_int32`.
                    Player::add_forces_to_reserves(game, target, 1);
                    game_error!("No location to remove a force from");
                }
            }
        }

        game.flip_bene_gesserit_advisors_when_alone();
        game.stone(Milestone::SpecialUselessPlayed);
        Ok(())
    }

    pub fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        let target_player = game.get_player(Some(self.target)).required("target player")?;
        let force: ExpressionElement = if self.special_force {
            game.player(target_player).special_force().into()
        } else {
            game.player(target_player).force().into()
        };
        Ok(express!(self.initiator(), " remove ", 1, force, " from ", self.location(game)))
    }

    pub fn get_message(&self, game: &Game) -> Message {
        self.try_get_message(game).expect("the message of an applicable ChoamRemoveForce event")
    }

}

impl GameEventMethods for ChoamRemoveForce {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::ChoamRemoveForce
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::ChoamRemoveForce(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ChoamRemoveForce::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        ChoamRemoveForce::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ChoamRemoveForce::get_message(self, game)
    }

    fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        ChoamRemoveForce::try_get_message(self, game)
    }

}
