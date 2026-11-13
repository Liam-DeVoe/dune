use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct KarmaFreeRevival {
    #[canon(name = "_heroId")]
    pub hero_id: i32,
    pub amount_of_forces: i32,
    pub amount_of_special_forces: i32,
    pub assign_skill: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for KarmaFreeRevival {
    fn default() -> Self {
        KarmaFreeRevival {
            hero_id: Default::default(),
            amount_of_forces: Default::default(),
            amount_of_special_forces: Default::default(),
            assign_skill: false,
            base: Default::default(),
        }
    }
}

impl KarmaFreeRevival {
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

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let p = self.player_opt();
        let pl = game.player(p.nre());

        if self.amount_of_forces > pl.forces_killed {
            return Some(express!("You can't revive that many"));
        }
        if self.amount_of_special_forces > pl.special_forces_killed {
            return Some(express!("You can't revive that many"));
        }
        if self.amount_of_forces.wrapping_add(self.amount_of_special_forces) > 3 {
            return Some(express!("You can't revive that many"));
        }
        if self.amount_of_special_forces > 1 {
            return Some(express!("You can only revive one ", pl.special_force(), " per turn"));
        }
        if self.amount_of_special_forces > 0 && game.factions_that_revived_special_forces_this_turn.contains(&self.initiator()) {
            return Some(express!("You already revived one ", pl.special_force(), " this turn."));
        }
        if self.amount_of_forces.wrapping_add(self.amount_of_special_forces) > 0 && self.hero().is_some() {
            return Some(express!("You can't revive both forces and a leader"));
        }

        None
    }

    pub fn valid_max_amount(game: &Game, p: PlayerIndex, special_forces: bool) -> i32 {
        if special_forces {
            return std::cmp::min(1, game.player(p).special_forces_killed);
        }
        std::cmp::min(3, game.player(p).forces_killed)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        game.stone(Milestone::Revival);
        let card = Karma::valid_karma_cards(game, self.player()).into_iter().next();

        // A face down leader stays a plain line, so the log doesn't name them.
        let hero = self.hero();
        if hero.is_some() && game.is_face_down_dead(hero.nre()) {
            self.log(game);
        } else {
            let mut row = RevivalRow::new(
                self.initiator(),
                if hero.is_none() { self.amount_of_forces } else { 0 },
                if hero.is_none() { self.amount_of_special_forces } else { 0 },
                0,
            );
            row.hero = hero;
            row.card = card;
            let message = self.get_message(game).with_log(row);
            self.log_objects(game, vec![message.into()]);
        }

        let me = self.player();
        game.discard_player_treachery_card_boolean(Some(me), card, true);
        game.player_mut(me).special_karma_power_used = true;

        if let Some(hero) = hero {
            game.revive(me, hero);

            if self.assign_skill {
                game.prepare_skill_assignment_to_revived_leader(me, hero.as_leader());
            }
        } else {
            Player::revive_forces(game, me, self.amount_of_forces);
            Player::revive_special_forces(game, me, self.amount_of_special_forces);

            if self.amount_of_special_forces > 0 {
                game.factions_that_revived_special_forces_this_turn.push(self.initiator());
            }
        }
    }

    pub fn get_message(&self, game: &Game) -> Message {
        let p = self.player_opt();
        if let Some(hero) = self.hero() {
            if !game.is_face_down_dead(hero) {
                return express!("Using ", TreacheryCardType::Karma, ", ", self.initiator(), " revive ", hero);
            }
            return express!("Using ", TreacheryCardType::Karma, ", ", self.initiator(), " revive a face down leader");
        }

        let pl = game.player(p.nre());
        express!(
            "Using ",
            TreacheryCardType::Karma,
            ", ",
            self.initiator(),
            " revive ",
            express_part_if!(self.amount_of_forces > 0; self.amount_of_forces, pl.force()),
            express_part_if!(self.amount_of_forces > 0 && self.amount_of_special_forces > 0; " and "),
            express_part_if!(self.amount_of_special_forces > 0; self.amount_of_special_forces, pl.special_force())
        )
    }

}

impl GameEventMethods for KarmaFreeRevival {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::KarmaFreeRevival
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::KarmaFreeRevival(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        KarmaFreeRevival::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        KarmaFreeRevival::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        KarmaFreeRevival::get_message(self, game)
    }

}
