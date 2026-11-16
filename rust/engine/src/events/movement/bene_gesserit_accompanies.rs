use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct BeneGesseritAccompanies {
    pub location: Option<Location>,
    pub accompanies: bool,
    pub extra_advisor: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for BeneGesseritAccompanies {
    fn default() -> Self {
        BeneGesseritAccompanies {
            location: Location::from_id(0),
            accompanies: Default::default(),
            extra_advisor: Default::default(),
            base: Default::default(),
        }
    }
}

impl BeneGesseritAccompanies {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn to(&self, _game: &Game) -> Option<Location> {
        self.location
    }

    pub fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        self.special_forces_added_to_location(game)
    }

    pub fn forces_added_to_location(&self, _game: &Game) -> i32 {
        0
    }

    pub fn special_forces_added_to_location(&self, _game: &Game) -> i32 {
        (if self.accompanies { 1 } else { 0 }) + (if self.extra_advisor { 1 } else { 0 })
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.ship_move.bene_gesserit_may_accompany = false;

        if self.accompanies {
            let me = self.player();
            let location = self.location.required("location")?;
            let amount = self.special_forces_added_to_location(game);
            if game.player(me).occupies_territory(game, Some(location.territory()))
                || !game.is_occupied_territory(location.territory())
                || !game.applicable(Rule::BeneGesseritAdvisors)
            {
                Player::ship_forces(game, me, location, amount);
            } else {
                Player::ship_advisors(game, me, location, amount);
            }

            let m = express!(self.initiator(), " accompany to ", location)
                .with_route(self.initiator().required("faction")?, Vec::<Location>::new(), Some(location))
                .with_log(BeneGesseritReaction::new(BeneGesseritReactionKind::Accompany, location.into(), false));
            self.log_objects(game, vec![m.into()]);
            game.ship_move.last_shipment_or_movement = Some(self.clone().into_event());
            game.check_intrusion(&self.clone().into_event())?;
        } else {
            let m = express!(self.initiator(), " don't accompany")
                .with_log(BeneGesseritReaction::new(BeneGesseritReactionKind::DontAccompany, Option::<Location>::None.into(), false));
            self.log_objects(game, vec![m.into()]);
        }

        game.determine_next_shipment_and_move_sub_phase()?;
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.accompanies {
            return express!(self.initiator(), " accompany shipment to ", self.location);
        }
        express!(self.initiator(), " don't accompany shipment")
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.accompanies {
            if game.player(self.player()).forces_in_reserve() <= 0 {
                return Some(express!("You don't have forces in reserve"));
            }
            let location = self.location;
            if !location.is_some_and(|l| BeneGesseritAccompanies::valid_targets(game, self.player()).contains(&l)) {
                return Some(express!("You can't accompany there"));
            }
            if self.initiator() != Some(Faction::BeneGesserit) {
                return Some(express!("You can't accompany shipments"));
            }
            let Some(location) = location else {
                return Some(express!("To not selected"));
            };
            // C# parity: unreachable, the first check already returned.
            if self.accompanies && game.player(self.player()).forces_in_reserve() == 0 {
                return Some(express!("No forces available"));
            }
            if self.extra_advisor && !BeneGesseritAccompanies::may_send_extra_advisor(game, self.player(), location) {
                return Some(express!("You cannot send an extra advisor"));
            }
        }

        None
    }

    pub fn bene_gesserit_may_accompany_to_shipment_location(g: &Game) -> bool {
        g.applicable(Rule::BeneGesseritAccompaniesToShipmentLocation) || g.applicable(Rule::BeneGesseritAdvisors)
    }

    pub fn valid_targets(g: &Game, p: PlayerId) -> Vec<Location> {
        let last_to = g.ship_move.last_shipment_or_movement.as_ref().and_then(|e| e.to_location(g));
        let Some(last_to) = last_to else {
            panic!("LastShipmentOrMovement is null");
        };

        let mut result = Vec::new();
        if last_to != g.board.map.polar_sink()
            && last_to.as_homeworld().is_none()
            && Some(last_to.territory()) != g.board.atomics_aftermath
            && !g.player(p).occupies_territory(g, Some(last_to.territory()))
            && BeneGesseritAccompanies::bene_gesserit_may_accompany_to_shipment_location(g)
            && !BeneGesseritAccompanies::ally_prevents_accompanying_to_shipment_location(g, p)
        {
            result.extend(last_to.territory().locations().iter().copied().filter(|l| {
                l.sector(&g.board.map) != g.board.sector_in_storm && (g.applicable(Rule::BeneGesseritAdvisors) || g.is_not_full(p, *l))
            }));
        }

        result.push(g.board.map.polar_sink());
        result
    }

    pub fn ally_prevents_accompanying_to_shipment_location(g: &Game, p: PlayerId) -> bool {
        let ally = g.get_player(g.player(p).ally);

        !g.applicable(Rule::AdvisorsDontConflictWithAlly)
            && ally.is_some_and(|a| {
                let t = g.ship_move.last_shipment_or_movement.as_ref().and_then(|e| e.to_location(g)).map(|l| l.territory());
                g.player(a).any_forces_in_territory(t) != 0
            })
    }

    pub fn may_send_extra_advisor(g: &Game, p: PlayerId, l: Location) -> bool {
        let pl = g.player(p);
        pl.has_any_high_threshold(g) && l == g.board.map.polar_sink() && pl.forces_in_reserve() >= 2
    }

}

impl GameEventMethods for BeneGesseritAccompanies {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::BeneGesseritAccompanies
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::BeneGesseritAccompanies(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        BeneGesseritAccompanies::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        BeneGesseritAccompanies::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        BeneGesseritAccompanies::get_message(self, game)
    }

}

impl crate::events::location_event::LocationEvent for BeneGesseritAccompanies {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self).expect("faction")
    }

    fn to(&self, game: &Game) -> Option<Location> {
        BeneGesseritAccompanies::to(self, game)
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        BeneGesseritAccompanies::total_amount_of_forces_added_to_location(self, game)
    }

    fn forces_added_to_location(&self, game: &Game) -> i32 {
        BeneGesseritAccompanies::forces_added_to_location(self, game)
    }

    fn special_forces_added_to_location(&self, game: &Game) -> i32 {
        BeneGesseritAccompanies::special_forces_added_to_location(self, game)
    }

}
