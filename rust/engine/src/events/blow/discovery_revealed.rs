use crate::prelude::*;

/// `params object?[]` element list for `Log(...)`.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

#[derive(Clone, Debug)]
pub struct DiscoveryRevealed {
    pub location_id: i32,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for DiscoveryRevealed {
    fn default() -> Self {
        DiscoveryRevealed {
            location_id: Default::default(),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl DiscoveryRevealed {
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
        if !Self::get_locations(game, self.player()).any(|l| Some(l) == location) {
            return Some(express!("Invalid location"));
        }

        None
    }

    pub fn applicable(g: &Game, p: PlayerIndex) -> bool {
        g.current_main_phase == MainPhase::Collection && !Self::get_locations(g, p).next().is_none()
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn get_locations(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Location> {
        g.discoveries_on_planet
            .iter()
            .filter(move |(k, v)| g.pending_discoveries.contains(&v.token) && g.player(p).occupies_territory(g, Some(k.territory())))
            .map(move |(k, _)| *k)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        // Location!: a null key would be ArgumentNullException; Validate guarantees a valid location.
        let Some(location) = self.location(game) else {
            game_error!("Value cannot be null. (Parameter 'key')")
        };
        let discovery = game.discoveries_on_planet[&location].clone();
        if let Some(i) = game.pending_discoveries.iter().position(|t| *t == discovery.token) {
            game.pending_discoveries.remove(i);
        }

        if !self.passed {
            game.stone(Milestone::DiscoveryRevealed);
            game.discoveries_on_planet.remove(&discovery.location);

            self.log_objects(game, elems![self.initiator(), " reveal ", discovery.token, " in ", discovery.location.territory()]);

            match discovery.token {
                DiscoveryToken::Jacurutu
                | DiscoveryToken::Shrine
                | DiscoveryToken::TestingStation
                | DiscoveryToken::Cistern
                | DiscoveryToken::ProcessingStation => {
                    let sh = game.map.get_discovery_stronghold(discovery.token);
                    sh.point_at(&mut game.map, discovery.location);
                    game.just_revealed_discovery_strongholds.push(sh);
                }

                DiscoveryToken::CardStash => {
                    let card = game.draw_treachery_card();
                    let me = self.player();
                    // C# adds card! even when it is null.
                    game.player_mut(me).treachery_cards.push(card.required("card")?);
                    self.log_objects(game, elems![self.initiator(), " draw a treachery card"]);
                    if game.player(me).hand_size_exceeded(game) {
                        game.let_factions_discard_surplus_cards();
                    }
                }

                DiscoveryToken::ResourceStash => {
                    let me = self.player();
                    let r = &mut game.player_mut(me).resources;
                    *r += 7;
                    self.log_objects(game, elems![self.initiator(), " get ", Payment::of(7)]);
                }

                DiscoveryToken::Flight => {
                    game.owner_of_flight_discovery = Some(self.initiator());
                    self.log_objects(game, elems![self.initiator(), " gain the ", discovery.token, " discovery token"]);
                }

                _ => {}
            }
        }
        Ok(())
    }

    pub fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        Ok(if self.passed {
            express!(self.initiator(), " don't reveal a discovery")
        } else {
            express!(self.initiator(), " reveal a discovery in ", self.location(game).required("location")?.territory())
        })
    }

    pub fn get_message(&self, game: &Game) -> Message {
        self.try_get_message(game).expect("the message of an applicable DiscoveryRevealed event")
    }

}

impl GameEventMethods for DiscoveryRevealed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::DiscoveryRevealed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::DiscoveryRevealed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        DiscoveryRevealed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        DiscoveryRevealed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        DiscoveryRevealed::get_message(self, game)
    }

    fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        DiscoveryRevealed::try_get_message(self, game)
    }

}

impl PassableGameEventMethods for DiscoveryRevealed {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
