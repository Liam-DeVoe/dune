use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct RaiseDeadPlayed {
    pub amount_of_forces: u16,
    pub amount_of_special_forces: u16,
    pub assign_skill: bool,
    pub hero: Option<Hero>,
    pub number_of_special_forces_in_location: u16,
    pub location: Option<Location>,
    /// The fields of `Event` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for RaiseDeadPlayed {
    fn default() -> Self {
        RaiseDeadPlayed {
            amount_of_forces: 0,
            amount_of_special_forces: 0,
            assign_skill: false,
            hero: TreacheryCardManager::get(0).map(Hero::TreacheryCard),
            number_of_special_forces_in_location: Default::default(),
            location: None,
            base: Default::default(),
        }
    }
}

impl RaiseDeadPlayed {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn to(&self, _game: &Game) -> Option<Location> {
        self.location
    }

    pub fn total_amount_of_forces_added_to_location(&self, _game: &Game) -> i32 {
        i32::from(self.number_of_special_forces_in_location)
    }

    pub fn forces_added_to_location(&self, _game: &Game) -> i32 {
        0
    }

    pub fn special_forces_added_to_location(&self, _game: &Game) -> i32 {
        i32::from(self.number_of_special_forces_in_location)
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let p_opt = self.player_opt();
        let initiator = self.initiator();
        let p = p_opt.expect("player");
        let pl = game.player(p);
        if i32::from(self.amount_of_forces) > i32::from(pl.forces_killed) {
            return Some(express!("You can't revive that many"));
        }
        if i32::from(self.amount_of_special_forces) > i32::from(pl.special_forces_killed) {
            return Some(express!("You can't revive that many"));
        }
        if (i32::from(self.amount_of_forces) + i32::from(self.amount_of_special_forces)) > 5 {
            return Some(express!("You can't revive that many"));
        }
        if initiator != Some(Faction::Ixian) && i32::from(self.amount_of_special_forces) > 1 {
            return Some(express!("You can only revive one ", pl.special_force(), " per turn"));
        }
        if i32::from(self.amount_of_special_forces) > 0
            && initiator != Some(Faction::Ixian)
            && game.revival.factions_that_revived_special_forces_this_turn.contains_opt(initiator)
        {
            return Some(express!("You already revived one ", pl.special_force(), " this turn"));
        }
        if (i32::from(self.amount_of_forces) + i32::from(self.amount_of_special_forces)) > 0 && self.hero.is_some() {
            return Some(express!("You can't revive both forces and a leader"));
        }
        if self.hero.is_some() && !Self::valid_heroes(game, p).contains(&self.hero.expect("hero")) {
            return Some(express!("Invalid leader"));
        }

        if self.assign_skill && self.hero.is_none() {
            return Some(express!("You must revive a leader to assign a skill to"));
        }
        if self.assign_skill && !Revival::may_assign_skill(game, p, self.hero) {
            return Some(express!("You can't assign a skill to this leader"));
        }

        if self.location.is_some() {
            let location = self.location;
            if !Revival::valid_revived_force_locations(game, p).any(|l| Some(l) == location) {
                return Some(express!("You can't place revived forces there"));
            }

            if i32::from(self.number_of_special_forces_in_location)
                > Revival::number_of_special_forces_that_may_be_placed_on_planet(game, p, i32::from(self.amount_of_special_forces))
            {
                return Some(express!("You can't place that many forces directly on the planet"));
            }
        }

        None
    }

    pub fn valid_heroes(game: &Game, player: PlayerId) -> Vec<Hero> {
        game.killed_heroes(player)
    }

    pub fn may_select_location_for_revived_forces(game: &Game, player: PlayerId, special_forces: i32) -> bool {
        let pl = game.player(player);
        pl.is(Faction::Fremen)
            && special_forces >= 1
            && pl.has_any_high_threshold(game)
            && Revival::valid_revived_force_locations(game, player).next().is_some()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.stone(Milestone::RaiseDead);
        // The revival log draws this as a row with the card in the cost column. A face-down
        // leader keeps the plain line so the table doesn't name them.
        let me = self.player();
        let initiator = self.initiator();
        let hero = self.hero;
        let card = game.player(me).treachery_cards.iter().copied().find(|c| c.type_() == TreacheryCardType::RaiseDead);
        let mut message = self.get_message(game);
        if hero.is_none() || !game.is_face_down_dead(hero.required("hero")?) {
            let mut row = RevivalRow::new(
                initiator.required("faction")?,
                if hero.is_none() { i32::from(self.amount_of_forces) } else { 0 },
                if hero.is_none() { i32::from(self.amount_of_special_forces) } else { 0 },
                0,
            );
            row.hero = hero;
            row.card = card;
            message = message.with_log(row);
        }
        game.current_report_mut().express_message(message);
        game.discard(me, TreacheryCardType::RaiseDead)?;

        let tleilaxu = self.get_player(game, Some(Faction::Tleilaxu));
        if let Some(tleilaxu) = tleilaxu {
            let r = &mut game.player_mut(tleilaxu).resources;
            *r += 1;
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
            Player::revive_forces(game, me, i32::from(self.amount_of_forces))?;
            Player::revive_special_forces(game, me, i32::from(self.amount_of_special_forces))?;

            if i32::from(self.amount_of_special_forces) > 0 {
                game.revival.factions_that_revived_special_forces_this_turn.push(initiator.required("faction")?);
            }
        }

        if self.location.is_some() && initiator == Some(Faction::Fremen) {
            let to = self.location.required("location")?;
            Player::ship_special_forces(game, me, to, 1)?;
            let location = self.location;
            self.log_objects(
                game,
                vec![initiator.into(), " place ".into(), FactionSpecialForce::Fremen.into(), " in ".into(), location.into()],
            );
        }
        Ok(())
    }

    pub fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        if let Some(hero) = self.hero {
            if !game.leaders.leader_state.contains_key(&hero) {
                game_error!("{} is not in this game", dotnet_compat::exception::CsToString::cs_to_string(&hero));
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
            express_part_if!(i32::from(self.amount_of_forces) > 0; i32::from(self.amount_of_forces), " ", pl.force()),
            express_part_if!(i32::from(self.amount_of_forces) > 0 && i32::from(self.amount_of_special_forces) > 0; " and "),
            express_part_if!(i32::from(self.amount_of_special_forces) > 0; i32::from(self.amount_of_special_forces), " ", pl.special_force())
        ))
    }

    pub fn get_message(&self, game: &Game) -> Message {
        self.try_get_message(game).expect("the message of an applicable RaiseDeadPlayed event")
    }

    pub fn valid_max_amount(g: &Game, p: PlayerId, special_forces: bool) -> i32 {
        let pl = g.player(p);
        if special_forces {
            if pl.faction == Some(Faction::Emperor) || pl.faction == Some(Faction::Fremen) {
                if g.revival.factions_that_revived_special_forces_this_turn.contains_opt(pl.faction) {
                    return 0;
                }
                return std::cmp::min(i32::from(pl.special_forces_killed), 1);
            }

            return std::cmp::min(i32::from(pl.special_forces_killed), 5);
        }

        std::cmp::min(i32::from(pl.forces_killed), 5)
    }

}

impl GameEventMethods for RaiseDeadPlayed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> EventType {
        EventType::RaiseDeadPlayed
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::RaiseDeadPlayed(self)
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

impl crate::events::location_event::LocationEvent for RaiseDeadPlayed {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self).expect("faction")
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
