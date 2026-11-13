use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct PerformHmsPlacement {
    #[canon(name = "_targetId")]
    pub target_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for PerformHmsPlacement {
    fn default() -> Self {
        PerformHmsPlacement {
            target_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl PerformHmsPlacement {
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
        // A null Player throws NullReferenceException in ValidLocations (hazards 1.4).
        if !Self::valid_locations(game, self.player()).any(|l| Some(l) == self.target(game)) {
            return Some(express!("Invalid location"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_locations(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Location> {
        use either::Either;
        if g.player(p).faction != Faction::Ixian {
            return Either::Left(g.map.locations(false).filter(|l| !l.territory().is_stronghold()));
        }
        Either::Right(g.map.locations(false).filter(move |l| !l.territory().is_stronghold() && l.sector(&g.map) != g.sector_in_storm))
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        // A null Target (only without validation): PointAt removes the HMS from its old host,
        // then throws NullReferenceException on `newLocation.Neighbours`.
        let target = self.target(game).or_null();
        let hms = game.map.hidden_mobile_stronghold();
        hms.point_at(&mut game.map, target);
        self.log(game);
        game.end_storm_phase();
        game.stone(Milestone::HmsMovement);
    }

    pub fn get_message(&self, game: &Game) -> Message {
        express!(self.initiator(), " position the ", game.map.hidden_mobile_stronghold(), " above ", self.target(game))
    }

}

impl GameEventMethods for PerformHmsPlacement {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::PerformHmsPlacement
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::PerformHmsPlacement(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        PerformHmsPlacement::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        PerformHmsPlacement::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        PerformHmsPlacement::get_message(self, game)
    }

}
