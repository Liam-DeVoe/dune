use crate::prelude::*;
use crate::compat::lazy::LazyQuery;

#[derive(Clone, Debug, Canon)]
pub struct PerformHmsMovement {
    #[canon(name = "_targetId")]
    pub target_id: i32,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for PerformHmsMovement {
    fn default() -> Self {
        PerformHmsMovement {
            target_id: Default::default(),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl PerformHmsMovement {
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
        if !self.passed {
            if !PerformHmsMovement::valid_locations(game).cs_contains(&self.target(game).or_null()) {
                return Some(express!("Invalid location"));
            }
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_locations(g: &Game) -> impl Iterator<Item = Location> {
        // AttachedToLocation is null only when the HMS was never placed; FindNeighboursForHmsMovement then throws.
        let start = g.map.hidden_mobile_stronghold().attached_to_location(&g.map).nre();
        g.map
            .find_neighbours_for_hms_movement(start, 1, false, g.sector_in_storm)
            .into_iter()
            .filter(move |l| !l.is_stronghold())
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let me = self.player();
        let hms = game.map.hidden_mobile_stronghold();
        let collection_rate = game.player(me).any_forces_in(Some(hms.location())).wrapping_mul(2);
        self.log(game);

        // A null AttachedToLocation throws ArgumentNullException in CollectSpiceFrom (TryGetValue(null)).
        let current_location = hms.attached_to_location(&game.map).or_null();
        game.collect_spice_from(self.initiator(), current_location, collection_rate);

        if !self.passed {
            // A null Target (only without validation): PointAt removes the HMS from its old
            // host's neighbours, then throws NullReferenceException on `newLocation.Neighbours`: the event fails there.
            let target = self.target(game).or_null();
            hms.point_at(&mut game.map, target)?;
            game.collect_spice_from(self.initiator(), target, collection_rate);
            game.hms_moves_left = game.hms_moves_left.wrapping_sub(1);
            game.stone(Milestone::HmsMovement);
        }

        if self.passed || game.hms_moves_left == 0 {
            game.determine_storm();
        }
        Ok(())
    }

    pub fn get_message(&self, game: &Game) -> Message {
        let hms = game.map.hidden_mobile_stronghold();
        if !self.passed {
            return express!(self.initiator(), " move the ", hms, " to ", self.target(game));
        }
        express!(self.initiator(), " pass (further) movement of the ", hms)
    }

}

impl GameEventMethods for PerformHmsMovement {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::PerformHmsMovement
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::PerformHmsMovement(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        PerformHmsMovement::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        PerformHmsMovement::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        PerformHmsMovement::get_message(self, game)
    }

}

impl PassableGameEventMethods for PerformHmsMovement {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
