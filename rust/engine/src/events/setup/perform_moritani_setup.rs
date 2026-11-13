use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct PerformMoritaniSetup {
    #[canon(name = "_targetId")]
    pub target_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for PerformMoritaniSetup {
    fn default() -> Self {
        PerformMoritaniSetup {
            target_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl PerformMoritaniSetup {
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

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_locations(g: &Game) -> impl Iterator<Item = Location> {
        let hms = g.map.hidden_mobile_stronghold().location();
        g.map.locations(false).filter(move |l| *l != hms && !g.any_forces_in(l.territory()))
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        // Player is read before Target, as in C#.
        let me = self.player();
        // `Target` may be null without validation: ShipForces passes it on.
        let target = self.target(game).or_null();
        Player::ship_forces(game, me, target, 6);
        self.log(game);
        // Game.Enter(bool, Action, Action) is static in C#; inlined here.
        if game.treachery_cards_before_traitors() {
            game.enter_storm_phase();
        } else {
            game.deal_starting_treachery_cards();
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " have set up forces")
    }

}

impl GameEventMethods for PerformMoritaniSetup {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::PerformMoritaniSetup
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::PerformMoritaniSetup(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        PerformMoritaniSetup::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        PerformMoritaniSetup::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        PerformMoritaniSetup::get_message(self, game)
    }

}
