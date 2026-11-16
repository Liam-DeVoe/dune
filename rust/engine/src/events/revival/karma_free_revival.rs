use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct KarmaFreeRevival {
    pub hero: Option<Hero>,
    pub amount_of_forces: u16,
    pub amount_of_special_forces: u16,
    pub assign_skill: bool,
}

impl Default for KarmaFreeRevival {
    fn default() -> Self {
        KarmaFreeRevival {
            hero: TreacheryCardManager::get(0).map(Hero::TreacheryCard),
            amount_of_forces: Default::default(),
            amount_of_special_forces: Default::default(),
            assign_skill: false,
        }
    }
}

impl KarmaFreeRevival {
    pub fn valid_max_amount(game: &Game, p: PlayerId, special_forces: bool) -> i32 {
        if special_forces {
            return std::cmp::min(1, i32::from(game.player(p).special_forces_killed));
        }
        std::cmp::min(3, i32::from(game.player(p).forces_killed))
    }
}

impl EventRules for KarmaFreeRevival {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        let p = by.player;
        let pl = game.player(p.expect("player"));

        if i32::from(self.amount_of_forces) > i32::from(pl.forces_killed) {
            return Err(Rejection(express!("You can't revive that many")));
        }
        if i32::from(self.amount_of_special_forces) > i32::from(pl.special_forces_killed) {
            return Err(Rejection(express!("You can't revive that many")));
        }
        if (i32::from(self.amount_of_forces) + i32::from(self.amount_of_special_forces)) > 3 {
            return Err(Rejection(express!("You can't revive that many")));
        }
        if i32::from(self.amount_of_special_forces) > 1 {
            return Err(Rejection(express!("You can only revive one ", pl.special_force(), " per turn")));
        }
        if i32::from(self.amount_of_special_forces) > 0 && game.revival.factions_that_revived_special_forces_this_turn.contains_opt(by.faction) {
            return Err(Rejection(express!("You already revived one ", pl.special_force(), " this turn.")));
        }
        if (i32::from(self.amount_of_forces) + i32::from(self.amount_of_special_forces)) > 0 && self.hero.is_some() {
            return Err(Rejection(express!("You can't revive both forces and a leader")));
        }
        if self.hero.is_some_and(|h| Revival::is_living_leader(game, h)) {
            return Err(Rejection(express!("You can't revive a leader who isn't dead")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.stone(Milestone::Revival);
        let card = Karma::valid_karma_cards(game, by.player()).into_iter().next();

        // A face down leader stays a plain line, so the log doesn't name them.
        let hero = self.hero;
        if hero.is_some() && game.is_face_down_dead(hero.required("hero")?) {
            let m = self.describe(game, by);
        game.log_event(m);
        } else {
            let mut row = RevivalRow::new(
                by.faction.required("faction")?,
                if hero.is_none() { i32::from(self.amount_of_forces) } else { 0 },
                if hero.is_none() { i32::from(self.amount_of_special_forces) } else { 0 },
                0,
            );
            row.hero = hero;
            row.card = card;
            let message = self.describe(game, by).with_log(row);
            game.log(vec![message.into()]);
        }

        let me = by.player();
        game.discard_card_of(Some(me), card, true);
        game.player_mut(me).special_karma_power_used = true;

        if let Some(hero) = hero {
            game.revive(me, hero);

            if self.assign_skill {
                game.prepare_skill_assignment_to_revived_leader(me, hero.as_leader());
            }
        } else {
            Player::revive_forces(game, me, i32::from(self.amount_of_forces))?;
            Player::revive_special_forces(game, me, i32::from(self.amount_of_special_forces))?;

            if i32::from(self.amount_of_special_forces) > 0 {
                game.revival.factions_that_revived_special_forces_this_turn.push(by.faction.required("faction")?);
            }
        }
        Ok(())
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        self.try_describe(game, by).expect("the message of an applicable KarmaFreeRevival event")
    }

    fn try_describe(&self, game: &Game, by: By) -> GameResult<Message> {
        let p = by.player;
        if let Some(hero) = self.hero {
            if !game.leaders.leader_state.contains_key(&hero) {
                game_error!("{} is not in this game", dotnet_compat::exception::CsToString::cs_to_string(&hero));
            }
            if !game.is_face_down_dead(hero) {
                return Ok(express!("Using ", TreacheryCardType::Karma, ", ", by.faction, " revive ", hero));
            }
            return Ok(express!("Using ", TreacheryCardType::Karma, ", ", by.faction, " revive a face down leader"));
        }

        let pl = game.player(p.required("player")?);
        Ok(express!(
            "Using ",
            TreacheryCardType::Karma,
            ", ",
            by.faction,
            " revive ",
            express_part_if!(i32::from(self.amount_of_forces) > 0; i32::from(self.amount_of_forces), pl.force()),
            express_part_if!(i32::from(self.amount_of_forces) > 0 && i32::from(self.amount_of_special_forces) > 0; " and "),
            express_part_if!(i32::from(self.amount_of_special_forces) > 0; i32::from(self.amount_of_special_forces), pl.special_force())
        ))
    }
}
