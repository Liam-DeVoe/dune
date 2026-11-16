use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct PerformHmsPlacement {
    pub target: Option<Location>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for PerformHmsPlacement {
    fn default() -> Self {
        PerformHmsPlacement {
            target: Location::from_id(0),
            base: Default::default(),
        }
    }
}

impl PerformHmsPlacement {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        // A null Player throws NullReferenceException in ValidLocations.
        if !Self::valid_locations(game, self.player()).any(|l| Some(l) == self.target) {
            return Some(express!("Invalid location"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_locations(g: &Game, p: PlayerId) -> impl Iterator<Item = Location> {
        use either::Either;
        if g.player(p).faction != Some(Faction::Ixian) {
            return Either::Left(g.board.map.locations(false).filter(|l| !l.territory().is_stronghold()));
        }
        Either::Right(g.board.map.locations(false).filter(move |l| !l.territory().is_stronghold() && !Sector::in_storm(l.sector(&g.board.map), g.board.sector_in_storm)))
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let target = self.target.required("target")?;
        let hms = game.board.map.hidden_mobile_stronghold();
        hms.point_at(&mut game.board.map, target);
        self.log(game);
        game.end_storm_phase();
        game.stone(Milestone::HmsMovement);
        Ok(())
    }

    pub fn get_message(&self, game: &Game) -> Message {
        express!(self.initiator(), " position the ", game.board.map.hidden_mobile_stronghold(), " above ", self.target)
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

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        PerformHmsPlacement::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        PerformHmsPlacement::get_message(self, game)
    }

}
