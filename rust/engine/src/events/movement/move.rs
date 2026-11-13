use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Move {
    pub as_advisors: bool,
    // ---- inherited from PlacementEvent ----
    pub to_id: i32,
    pub force_locations: Option<String>,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for Move {
    fn default() -> Self {
        Move {
            as_advisors: Default::default(),
            to_id: Default::default(),
            force_locations: Some("".to_string()),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl Move {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !self.passed && game.in_guild_cunning_shipment {
            return Some(express!("You cannot move after Cunning shipment"));
        }

        self.validate_move(game, self.as_advisors)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.storm_losses_to_take.clear();

        game.current_player_may_perform_extra_move = match &game.current_flight_used {
            Some(fu) => fu.initiator() == self.initiator() && fu.extra_move(),
            None => false,
        };

        if !game.current_player_may_perform_extra_move && !game.in_guild_cunning_shipment {
            game.has_acted_or_passed.push(self.initiator());

            if game.current_phase == Phase::NonGuildMove {
                let mut seq = game.shipment_and_move_sequence.take().required("shipment and move sequence")?;
                seq.next_player(game);
                game.shipment_and_move_sequence = Some(seq);

                let current_is_guild = game.shipment_and_move_sequence.as_ref().required("shipment and move sequence")?.current_faction(game) == Faction::Guild;
                if current_is_guild && game.guild_may_ship_out_of_turn_order() {
                    let mut seq = game.shipment_and_move_sequence.take().required("shipment and move sequence")?;
                    seq.next_player(game);
                    game.shipment_and_move_sequence = Some(seq);
                }
            }
        }

        if game.in_guild_cunning_shipment {
            game.current_guild_cunning = None;
            game.in_guild_cunning_shipment = false;
        }

        if !self.passed {
            game.recent_moves.push(self.clone().into_event());

            let me = self.player();
            let to = PlacementEventMethods::to(&**self, game);
            if contains_conflicting_ally_opt(game, me, to)? {
                game.chosen_destinations_with_allies.push(to.required("to")?.territory());
            }

            let force_locations = PlacementEventMethods::force_locations(&**self, game);
            let as_advisors = self.initiator() != Faction::BeneGesserit || self.as_advisors;
            game.perform_move_from_locations(me, &force_locations, &self.clone().into_event(), as_advisors, false);
            game.check_intrusion(&self.clone().into_event())?;
        } else {
            let m = express!(self.initiator(), " pass movement").with_log(MovePassed { faction: self.initiator() });
            self.log_objects(game, vec![m.into()]);
        }

        game.determine_next_shipment_and_move_sub_phase()?;
        game.check_if_forces_should_be_destroyed_by_ally_presence(self.player());
        game.flip_bene_gesserit_advisors_when_alone();

        if !game.applicable(Rule::FullPhaseKarma) {
            game.allow(FactionAdvantage::FremenExtraMove);
        }
        if !game.applicable(Rule::FullPhaseKarma) {
            game.allow(FactionAdvantage::IxianCyborgExtraMove);
        }

        game.current_flight_used = None;
        game.current_flight_discovery_used = None;
        game.current_planetology = None;
        Ok(())
    }

    pub fn get_message(&self, game: &Game) -> Message {
        if self.passed {
            return express!(self.initiator(), " pass move");
        }
        express!(self.initiator(), " move to ", PlacementEventMethods::to(self, game))
    }

}

/// `Game.ContainsConflictingAlly(Player, To)` where `To` may be null.
///
/// `to` is only dereferenced after the early returns, so a null
/// `To` gives `false` there and fails the event after them (C#: NullReferenceException).
pub(crate) fn contains_conflicting_ally_opt(game: &Game, initiator: PlayerIndex, to: Option<Location>) -> GameResult<bool> {
    match to {
        Some(to) => Ok(game.contains_conflicting_ally(initiator, to)),
        None => {
            let ini = game.player(initiator);
            if !ini.has_ally() || ini.faction == Faction::Ecaz || ini.ally == Faction::Ecaz {
                return Ok(false);
            }
            game_error!("There is no destination")
        }
    }
}

impl GameEventMethods for Move {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::Move
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Move(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Move::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        Move::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Move::get_message(self, game)
    }

}

impl PassableGameEventMethods for Move {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}

impl PlacementEventMethods for Move {
    fn field_to_id(&self) -> &i32 {
        &self.to_id
    }

    fn field_force_locations(&self) -> &Option<String> {
        &self.force_locations
    }

    fn field_to_id_mut(&mut self) -> &mut i32 {
        &mut self.to_id
    }

    fn field_force_locations_mut(&mut self) -> &mut Option<String> {
        &mut self.force_locations
    }

}

impl crate::events::location_event::ILocationEvent for Move {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self)
    }

    fn to(&self, game: &Game) -> Option<Location> {
        <Self as PlacementEventMethods>::to(self, game)
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        <Self as PlacementEventMethods>::total_amount_of_forces_added_to_location(self, game)
    }

    fn forces_added_to_location(&self, game: &Game) -> i32 {
        <Self as PlacementEventMethods>::forces_added_to_location(self, game)
    }

    fn special_forces_added_to_location(&self, game: &Game) -> i32 {
        <Self as PlacementEventMethods>::special_forces_added_to_location(self, game)
    }

}

impl crate::events::placement::IPlacement for Move {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self)
    }

    fn force_locations(&self, game: &Game) -> IndexMap<Location, Battalion> {
        <Self as PlacementEventMethods>::force_locations(self, game)
    }

    fn to(&self, game: &Game) -> Option<Location> {
        <Self as PlacementEventMethods>::to(self, game)
    }

}
