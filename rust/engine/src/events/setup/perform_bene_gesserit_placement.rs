use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct PerformBeneGesseritPlacement {
    #[canon(name = "_targetId")]
    pub target_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for PerformBeneGesseritPlacement {
    fn default() -> Self {
        PerformBeneGesseritPlacement {
            target_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl PerformBeneGesseritPlacement {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn target(&self, game: &Game) -> Option<Location> {
        game.map.location_lookup().find(self.target_id)
    }

    pub fn set_target(&mut self, game: &Game, value: Option<Location>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.target_id = game.map.location_lookup().get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let target = self.target(game);
        if !Self::valid_locations(game).any(|l| Some(l) == target) {
            return Some(express!("Invalid location"));
        }

        None
    }

    pub fn bene_gesserit_may_place_first_force_in_any_territory(g: &Game) -> bool {
        g.applicable(Rule::BeneGesseritFirstForceInAnyTerritory) || g.applicable(Rule::BeneGesseritAdvisors)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_locations(g: &Game) -> impl Iterator<Item = Location> {
        use either::Either;
        if Self::bene_gesserit_may_place_first_force_in_any_territory(g) {
            let hms = g.map.hidden_mobile_stronghold().location();
            return Either::Left(g.map.locations(false).filter(move |l| *l != hms));
        }
        Either::Right(std::iter::once(g.map.polar_sink()))
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        // A null Target (only without validation) is passed on as in C#: IsOccupied(null)
        // is false, ShipForces(null, 1) finds no homeworld to ship from, and `Target.Territory` throws.
        let condition = game.is_occupied_territory(self.target(game).or_null().territory());
        if condition {
            // Player is read before Target, as in C#.
            let me = self.player();
            Player::ship_advisors(game, me, self.target(game).or_null(), 1);
        } else {
            let me = self.player();
            Player::ship_forces(game, me, self.target(game).or_null(), 1);
        }

        self.log(game);
        let c1 = self.is_playing(game, Faction::Moritani);
        let c2 = game.treachery_cards_before_traitors();
        game.enter_boolean_phase_boolean_action_action(
            c1,
            Phase::MoritaniSettingUp,
            c2,
            Game::enter_storm_phase,
            Game::deal_starting_treachery_cards,
        );
        Ok(())
    }

    pub fn get_message(&self, game: &Game) -> Message {
        express!(self.initiator(), " position themselves in ", self.target(game))
    }

}

impl GameEventMethods for PerformBeneGesseritPlacement {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::PerformBeneGesseritPlacement
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::PerformBeneGesseritPlacement(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        PerformBeneGesseritPlacement::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        PerformBeneGesseritPlacement::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        PerformBeneGesseritPlacement::get_message(self, game)
    }

}
