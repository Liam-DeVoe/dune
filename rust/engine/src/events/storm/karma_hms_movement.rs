use crate::prelude::*;
use crate::compat::lazy::LazyQuery;

#[derive(Clone, Debug, Canon)]
pub struct KarmaHmsMovement {
    pub passed: bool,
    #[canon(name = "_targetId")]
    pub target_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for KarmaHmsMovement {
    fn default() -> Self {
        KarmaHmsMovement {
            passed: Default::default(),
            target_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl KarmaHmsMovement {
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
        if !KarmaHmsMovement::valid_locations(game).cs_contains(&self.target(game).or_null()) {
            return Some(express!("Invalid location"));
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

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        let me = self.player();
        let hms = game.map.hidden_mobile_stronghold();
        let collection_rate = game.player(me).any_forces_in(Some(hms.location())).wrapping_mul(2);
        if !game.player(me).special_karma_power_used {
            let card = Karma::valid_karma_cards(game, me).into_iter().next();
            if self.passed {
                game.play_karma(me, card, vec!["pass movement of the ".into(), hms.into()]);
            } else {
                let target = self.target(game);
                game.play_karma(me, card, vec!["move the ".into(), hms.into(), " to ".into(), target.into()]);
            }

            game.player_mut(me).special_karma_power_used = true;
        } else {
            self.log(game);
        }

        // A null AttachedToLocation throws ArgumentNullException in CollectSpiceFrom (TryGetValue(null)).
        let current_location = hms.attached_to_location(&game.map).or_null();
        game.collect_spice_from(self.initiator(), current_location, collection_rate);

        if !self.passed {
            // A null Target (only without validation): PointAt removes the HMS from its old
            // host's neighbours, then throws NullReferenceException on `newLocation.Neighbours`.
            let target = self.target(game).or_null();
            hms.point_at(&mut game.map, target);
            game.collect_spice_from(self.initiator(), target, collection_rate);
            game.karma_hms_moves_left = game.karma_hms_moves_left.wrapping_sub(1);
            game.stone(Milestone::HmsMovement);
        }

        if self.passed {
            game.karma_hms_moves_left = 0;
        }
    }

    pub fn get_message(&self, game: &Game) -> Message {
        let hms = game.map.hidden_mobile_stronghold();
        if !self.passed {
            return express!("Using ", TreacheryCardType::Karma, ", ", self.initiator(), " move the ", hms, " to ", self.target(game));
        }
        express!(self.initiator(), " pass (further) movement of the ", hms, " using ", TreacheryCardType::Karma)
    }

}

impl GameEventMethods for KarmaHmsMovement {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::KarmaHmsMovement
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::KarmaHmsMovement(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        KarmaHmsMovement::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        KarmaHmsMovement::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        KarmaHmsMovement::get_message(self, game)
    }

}
