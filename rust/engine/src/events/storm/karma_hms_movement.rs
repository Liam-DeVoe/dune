use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct KarmaHmsMovement {
    pub passed: bool,
    pub target: Option<Location>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for KarmaHmsMovement {
    fn default() -> Self {
        KarmaHmsMovement {
            passed: Default::default(),
            target: Location::from_id(0),
            base: Default::default(),
        }
    }
}

impl KarmaHmsMovement {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !KarmaHmsMovement::valid_locations(game).any(|v| Some(v) == self.target) {
            return Some(express!("Invalid location"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_locations(g: &Game) -> impl Iterator<Item = Location> {
        // AttachedToLocation is null only when the HMS was never placed; FindNeighboursForHmsMovement then throws.
        let start = g.board.map.hidden_mobile_stronghold().attached_to_location(&g.board.map).expect("attached to location");
        g.board.map
            .find_neighbours_for_hms_movement(start, 1, false, g.board.sector_in_storm)
            .into_iter()
            .filter(move |l| !l.is_stronghold())
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let me = self.player();
        let hms = game.board.map.hidden_mobile_stronghold();
        let collection_rate = game.player(me).any_forces_in(Some(hms.location())) * 2;
        if !game.player(me).special_karma_power_used {
            let card = Karma::valid_karma_cards(game, me).into_iter().next();
            if self.passed {
                game.play_karma(me, card, vec!["pass movement of the ".into(), hms.into()]);
            } else {
                let target = self.target;
                game.play_karma(me, card, vec!["move the ".into(), hms.into(), " to ".into(), target.into()]);
            }

            game.player_mut(me).special_karma_power_used = true;
        } else {
            self.log(game);
        }

        let current_location = hms.attached_to_location(&game.board.map).required("the location of the HMS")?;
        game.collect_spice_from(self.initiator().required("faction")?, current_location, collection_rate)?;

        if !self.passed {
            let target = self.target.required("target")?;
            hms.point_at(&mut game.board.map, target);
            game.collect_spice_from(self.initiator().required("faction")?, target, collection_rate)?;
            game.storm.karma_hms_moves_left -= 1;
            game.stone(Milestone::HmsMovement);
        }

        if self.passed {
            game.storm.karma_hms_moves_left = 0;
        }
        Ok(())
    }

    pub fn get_message(&self, game: &Game) -> Message {
        let hms = game.board.map.hidden_mobile_stronghold();
        if !self.passed {
            return express!("Using ", TreacheryCardType::Karma, ", ", self.initiator(), " move the ", hms, " to ", self.target);
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

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        KarmaHmsMovement::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        KarmaHmsMovement::get_message(self, game)
    }

}
