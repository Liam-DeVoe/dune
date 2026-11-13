use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct RaiseDeadPlayed {
    pub amount_of_forces: i32,
    pub amount_of_special_forces: i32,
    pub assign_skill: bool,
    #[canon(name = "_heroId")]
    pub hero_id: i32,
    pub number_of_special_forces_in_location: i32,
    #[canon(name = "_locationId")]
    pub location_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for RaiseDeadPlayed {
    fn default() -> Self {
        RaiseDeadPlayed {
            amount_of_forces: 0,
            amount_of_special_forces: 0,
            assign_skill: false,
            hero_id: Default::default(),
            number_of_special_forces_in_location: Default::default(),
            location_id: -1,
            base: Default::default(),
        }
    }
}

impl RaiseDeadPlayed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn hero(&self) -> Option<Hero> {
        LeaderManager::hero_lookup().find(self.hero_id)
    }

    pub fn set_hero(&mut self, value: Option<Hero>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.hero_id = LeaderManager::hero_lookup().get_id(value);
    }

    pub fn location(&self, game: &Game) -> Option<Location> {
        game.map.location_lookup().find(self.location_id)
    }

    pub fn set_location(&mut self, game: &Game, value: Option<Location>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.location_id = game.map.location_lookup().get_id(value);
    }

    pub fn to(&self, game: &Game) -> Option<Location> {
        self.location(game)
    }

    pub fn total_amount_of_forces_added_to_location(&self, _game: &Game) -> i32 {
        self.number_of_special_forces_in_location
    }

    pub fn forces_added_to_location(&self, _game: &Game) -> i32 {
        0
    }

    pub fn special_forces_added_to_location(&self, _game: &Game) -> i32 {
        self.number_of_special_forces_in_location
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let p_opt = self.player_opt();
        let initiator = self.initiator();
        if self.amount_of_forces < 0 || self.amount_of_special_forces < 0 {
            return Some(express!("You can't revive a negative amount of forces"));
        }
        let p = p_opt.nre();
        let pl = game.player(p);
        if self.amount_of_forces > pl.forces_killed {
            return Some(express!("You can't revive that many"));
        }
        if self.amount_of_special_forces > pl.special_forces_killed {
            return Some(express!("You can't revive that many"));
        }
        if self.amount_of_forces.wrapping_add(self.amount_of_special_forces) > 5 {
            return Some(express!("You can't revive that many"));
        }
        if initiator != Faction::Ixian && self.amount_of_special_forces > 1 {
            return Some(express!("You can only revive one ", pl.special_force(), " per turn"));
        }
        if self.amount_of_special_forces > 0
            && initiator != Faction::Ixian
            && game.factions_that_revived_special_forces_this_turn.contains(&initiator)
        {
            return Some(express!("You already revived one ", pl.special_force(), " this turn"));
        }
        if self.amount_of_forces.wrapping_add(self.amount_of_special_forces) > 0 && self.hero().is_some() {
            return Some(express!("You can't revive both forces and a leader"));
        }
        if self.hero().is_some() && !Self::valid_heroes(game, p).contains(&self.hero().nre()) {
            return Some(express!("Invalid leader"));
        }

        if self.assign_skill && self.hero().is_none() {
            return Some(express!("You must revive a leader to assign a skill to"));
        }
        if self.assign_skill && !Revival::may_assign_skill(game, p, self.hero()) {
            return Some(express!("You can't assign a skill to this leader"));
        }

        if self.location(game).is_some() {
            let location = self.location(game);
            if !Revival::valid_revived_force_locations(game, p).any(|l| Some(l) == location) {
                return Some(express!("You can't place revived forces there"));
            }

            if self.number_of_special_forces_in_location
                > Revival::number_of_special_forces_that_may_be_placed_on_planet(game, p, self.amount_of_special_forces)
            {
                return Some(express!("You can't place that many forces directly on the planet"));
            }
        }

        None
    }

    pub fn valid_heroes(game: &Game, player: PlayerIndex) -> Vec<Hero> {
        game.killed_heroes(player)
    }

    pub fn may_select_location_for_revived_forces(game: &Game, player: PlayerIndex, special_forces: i32) -> bool {
        let pl = game.player(player);
        pl.is(Faction::Fremen)
            && special_forces >= 1
            && pl.has_high_threshold_0(game)
            && Revival::valid_revived_force_locations(game, player).next().is_some()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.stone(Milestone::RaiseDead);
        // The revival log draws this as a row with the card in the cost column. A face-down
        // leader keeps the plain line so the table doesn't name them.
        let me = self.player();
        let initiator = self.initiator();
        let hero = self.hero();
        let card = game.player(me).treachery_cards.iter().copied().find(|c| c.type_() == TreacheryCardType::RaiseDead);
        let mut message = self.get_message(game);
        if hero.is_none() || !game.is_face_down_dead(hero.required("hero")?) {
            let mut row = RevivalRow::new(
                initiator,
                if hero.is_none() { self.amount_of_forces } else { 0 },
                if hero.is_none() { self.amount_of_special_forces } else { 0 },
                0,
            );
            row.hero = hero;
            row.card = card;
            // C# WithLog mutates the message in place; its result is discarded there.
            message = message.with_log(row);
        }
        game.current_report_mut().express_message(message);
        game.discard(me, TreacheryCardType::RaiseDead)?;

        let tleilaxu = self.get_player(game, Some(Faction::Tleilaxu));
        if let Some(tleilaxu) = tleilaxu {
            let r = &mut game.player_mut(tleilaxu).resources;
            *r = r.wrapping_add(1);
            self.log_objects(
                game,
                vec![
                    Faction::Tleilaxu.into(),
                    " get ".into(),
                    Payment::of(1).into(),
                    " for revival by ".into(),
                    TreacheryCardType::RaiseDead.into(),
                ],
            );
        }

        if let Some(hero) = hero {
            // `Game.Revive(Player, Hero as Leader)` and `Game.Revive(Player, Hero)` pass the same object.
            game.revive(me, hero);

            if self.assign_skill {
                game.prepare_skill_assignment_to_revived_leader(me, hero.as_leader());
            }
        } else {
            Player::revive_forces(game, me, self.amount_of_forces);
            Player::revive_special_forces(game, me, self.amount_of_special_forces);

            if self.amount_of_special_forces > 0 {
                game.factions_that_revived_special_forces_this_turn.push(initiator);
            }
        }

        if self.location(game).is_some() && initiator == Faction::Fremen {
            let to = self.location(game).required("location")?;
            Player::ship_special_forces(game, me, to, 1);
            let location = self.location(game);
            self.log_objects(
                game,
                vec![initiator.into(), " place ".into(), FactionSpecialForce::Fremen.into(), " in ".into(), location.into()],
            );
        }
        Ok(())
    }

    pub fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        if let Some(hero) = self.hero() {
            if !game.leader_state.contains_key(&hero) {
                game_error!("{} is not in this game", ExpressionElement::from(hero).to_cs_string());
            }
            if !game.is_face_down_dead(hero) {
                return Ok(express!("Using ", TreacheryCardType::RaiseDead, ", ", self.initiator(), " revive ", hero));
            }
            return Ok(express!("Using ", TreacheryCardType::RaiseDead, ", ", self.initiator(), " revive a face down leader"));
        }

        let pl = game.player(self.player_opt().required("player")?);
        Ok(express!(
            "Using ",
            TreacheryCardType::RaiseDead,
            ", ",
            self.initiator(),
            " revive ",
            express_part_if!(self.amount_of_forces > 0; self.amount_of_forces, " ", pl.force()),
            express_part_if!(self.amount_of_forces > 0 && self.amount_of_special_forces > 0; " and "),
            express_part_if!(self.amount_of_special_forces > 0; self.amount_of_special_forces, " ", pl.special_force())
        ))
    }

    pub fn get_message(&self, game: &Game) -> Message {
        self.try_get_message(game).expect("the message of an applicable RaiseDeadPlayed event")
    }

    pub fn valid_max_amount(g: &Game, p: PlayerIndex, special_forces: bool) -> i32 {
        let pl = g.player(p);
        if special_forces {
            if pl.faction == Faction::Emperor || pl.faction == Faction::Fremen {
                if g.factions_that_revived_special_forces_this_turn.contains(&pl.faction) {
                    return 0;
                }
                return std::cmp::min(pl.special_forces_killed, 1);
            }

            return std::cmp::min(pl.special_forces_killed, 5);
        }

        std::cmp::min(pl.forces_killed, 5)
    }

}

impl GameEventMethods for RaiseDeadPlayed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::RaiseDeadPlayed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::RaiseDeadPlayed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        RaiseDeadPlayed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        RaiseDeadPlayed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        RaiseDeadPlayed::get_message(self, game)
    }

    fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        RaiseDeadPlayed::try_get_message(self, game)
    }

}

impl crate::events::location_event::ILocationEvent for RaiseDeadPlayed {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self)
    }

    fn to(&self, game: &Game) -> Option<Location> {
        RaiseDeadPlayed::to(self, game)
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        RaiseDeadPlayed::total_amount_of_forces_added_to_location(self, game)
    }

    fn forces_added_to_location(&self, game: &Game) -> i32 {
        RaiseDeadPlayed::forces_added_to_location(self, game)
    }

    fn special_forces_added_to_location(&self, game: &Game) -> i32 {
        RaiseDeadPlayed::special_forces_added_to_location(self, game)
    }

}
