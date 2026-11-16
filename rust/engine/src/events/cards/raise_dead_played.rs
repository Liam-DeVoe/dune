use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct RaiseDeadPlayed {
    pub amount_of_forces: u16,
    pub amount_of_special_forces: u16,
    pub assign_skill: bool,
    pub hero: Option<Hero>,
    pub number_of_special_forces_in_location: u16,
    pub location: Option<Location>,
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
        }
    }
}

impl RaiseDeadPlayed {
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

impl EventRules for RaiseDeadPlayed {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        let p_opt = by.player;
        let initiator = by.faction;
        let p = p_opt.expect("player");
        let pl = game.player(p);
        if i32::from(self.amount_of_forces) > i32::from(pl.forces_killed) {
            return Err(Rejection(express!("You can't revive that many")));
        }
        if i32::from(self.amount_of_special_forces) > i32::from(pl.special_forces_killed) {
            return Err(Rejection(express!("You can't revive that many")));
        }
        if (i32::from(self.amount_of_forces) + i32::from(self.amount_of_special_forces)) > 5 {
            return Err(Rejection(express!("You can't revive that many")));
        }
        if initiator != Some(Faction::Ixian) && i32::from(self.amount_of_special_forces) > 1 {
            return Err(Rejection(express!("You can only revive one ", pl.special_force(), " per turn")));
        }
        if i32::from(self.amount_of_special_forces) > 0
            && initiator != Some(Faction::Ixian)
            && game.revival.factions_that_revived_special_forces_this_turn.contains_opt(initiator)
        {
            return Err(Rejection(express!("You already revived one ", pl.special_force(), " this turn")));
        }
        if (i32::from(self.amount_of_forces) + i32::from(self.amount_of_special_forces)) > 0 && self.hero.is_some() {
            return Err(Rejection(express!("You can't revive both forces and a leader")));
        }
        if self.hero.is_some() && !Self::valid_heroes(game, p).contains(&self.hero.expect("hero")) {
            return Err(Rejection(express!("Invalid leader")));
        }

        if self.assign_skill && self.hero.is_none() {
            return Err(Rejection(express!("You must revive a leader to assign a skill to")));
        }
        if self.assign_skill && !Revival::may_assign_skill(game, p, self.hero) {
            return Err(Rejection(express!("You can't assign a skill to this leader")));
        }

        if self.location.is_some() {
            let location = self.location;
            if !Revival::valid_revived_force_locations(game, p).any(|l| Some(l) == location) {
                return Err(Rejection(express!("You can't place revived forces there")));
            }

            if i32::from(self.number_of_special_forces_in_location)
                > Revival::number_of_special_forces_that_may_be_placed_on_planet(game, p, i32::from(self.amount_of_special_forces))
            {
                return Err(Rejection(express!("You can't place that many forces directly on the planet")));
            }
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.stone(Milestone::RaiseDead);
        // The revival log draws this as a row with the card in the cost column. A face-down
        // leader keeps the plain line so the table doesn't name them.
        let me = by.player();
        let initiator = by.faction;
        let hero = self.hero;
        let card = game.player(me).treachery_cards.iter().copied().find(|c| c.type_() == TreacheryCardType::RaiseDead);
        let mut message = self.describe(game, by);
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

        let tleilaxu = game.get_player(Some(Faction::Tleilaxu));
        if let Some(tleilaxu) = tleilaxu {
            let r = &mut game.player_mut(tleilaxu).resources;
            *r += 1;
            game.log(
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
            game.log(
                vec![initiator.into(), " place ".into(), FactionSpecialForce::Fremen.into(), " in ".into(), location.into()],
            );
        }
        Ok(())
    }

    fn try_describe(&self, game: &Game, by: By) -> GameResult<Message> {
        if let Some(hero) = self.hero {
            if !game.leaders.leader_state.contains_key(&hero) {
                game_error!("{} is not in this game", dotnet_compat::exception::CsToString::cs_to_string(&hero));
            }
            if !game.is_face_down_dead(hero) {
                return Ok(express!("Using ", TreacheryCardType::RaiseDead, ", ", by.faction, " revive ", hero));
            }
            return Ok(express!("Using ", TreacheryCardType::RaiseDead, ", ", by.faction, " revive a face down leader"));
        }

        let pl = game.player(by.player.required("player")?);
        Ok(express!(
            "Using ",
            TreacheryCardType::RaiseDead,
            ", ",
            by.faction,
            " revive ",
            express_part_if!(i32::from(self.amount_of_forces) > 0; i32::from(self.amount_of_forces), " ", pl.force()),
            express_part_if!(i32::from(self.amount_of_forces) > 0 && i32::from(self.amount_of_special_forces) > 0; " and "),
            express_part_if!(i32::from(self.amount_of_special_forces) > 0; i32::from(self.amount_of_special_forces), " ", pl.special_force())
        ))
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        self.try_describe(game, by).expect("the message of an applicable RaiseDeadPlayed event")
    }
}

impl crate::events::location_event::LocationEvent for RaiseDeadPlayed {
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
