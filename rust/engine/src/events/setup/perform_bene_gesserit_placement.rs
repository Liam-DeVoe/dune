use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct PerformBeneGesseritPlacement {
    pub target: Option<Location>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for PerformBeneGesseritPlacement {
    fn default() -> Self {
        PerformBeneGesseritPlacement {
            target: Location::from_id(0),
            base: Default::default(),
        }
    }
}

impl PerformBeneGesseritPlacement {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let target = self.target;
        if !Self::valid_locations(game).any(|l| Some(l) == target) {
            return Some(express!("Invalid location"));
        }

        None
    }

    pub fn bene_gesserit_may_place_first_force_in_any_territory(g: &Game) -> bool {
        g.applicable(Rule::BeneGesseritFirstForceInAnyTerritory) || g.applicable(Rule::BeneGesseritAdvisors)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_locations(g: &Game) -> impl Iterator<Item = Location> {
        use either::Either;
        if Self::bene_gesserit_may_place_first_force_in_any_territory(g) {
            let hms = g.board.map.hidden_mobile_stronghold().location();
            return Either::Left(g.board.map.locations(false).filter(move |l| *l != hms));
        }
        Either::Right(std::iter::once(g.board.map.polar_sink()))
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        // A null Target (only without validation) is passed on as in C#: IsOccupied(null)
        // is false, ShipForces(null, 1) finds no homeworld to ship from, and `Target.Territory` throws.
        let condition = game.is_occupied_territory(self.target.required("target")?.territory());
        if condition {
            // Player is read before Target.
            let me = self.player();
            Player::ship_advisors(game, me, self.target.required("target")?, 1)?;
        } else {
            let me = self.player();
            Player::ship_forces(game, me, self.target.required("target")?, 1)?;
        }

        self.log(game);
        let c1 = self.is_playing(game, Faction::Moritani);
        let c2 = game.treachery_cards_before_traitors();
        game.enter_first(&[(c1, Step::Enter(Phase::MoritaniSettingUp)), (c2, Step::Run(Game::enter_storm_phase))], Step::Run(Game::deal_starting_treachery_cards))?;
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " position themselves in ", self.target)
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
