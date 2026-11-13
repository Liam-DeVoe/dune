use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct MetheorPlayed {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for MetheorPlayed {
    fn default() -> Self {
        MetheorPlayed {
            base: Default::default(),
        }
    }
}

impl MetheorPlayed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !MetheorPlayed::may_play_metheor(game, self.player()) {
            return Some(express!("You cannot use ", TreacheryCardType::Metheor));
        }

        None
    }

    pub fn may_play_metheor(g: &Game, p: PlayerIndex) -> bool {
        g.current_turn > 1
            && g.player(p).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Metheor)
            && MetheorPlayed::has_forces_at_or_near_shield_wall(g, p)
    }

    pub fn has_forces_at_or_near_shield_wall(g: &Game, p: PlayerIndex) -> bool {
        let pl = g.player(p);
        if pl.occupies_territory(g, Some(g.map.shield_wall())) {
            return true;
        }

        for shieldwall_location in g.map.shield_wall().locations().iter().copied() {
            //checks locations that are immediately adjacent to the shield wall
            if shieldwall_location.neighbours(&g.map).iter().any(|l| pl.occupies(g, Some(*l))) {
                return true;
            }

            //checks locations that are in a territory adjacent to the shield wall (but may be further away and separated by storm)
            if g.map
                .find_neighbours(shieldwall_location, 1, false, pl.faction, g, false)
                .iter()
                .any(|l| pl.occupies(g, Some(*l)))
            {
                return true;
            }
        }

        false
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let me = self.player();
        let card = game.player(me).card(TreacheryCardType::Metheor);
        game.stone(Milestone::MetheorUsed);
        game.shield_wall_destroyed = true;
        if let Some(card) = card {
            crate::iter::remove_first(&mut game.player_mut(me).treachery_cards, &card);
        }
        game.removed_treachery_cards.push(card.required("a Metheor card")?);
        let m = self.get_message(game).with_log(FamilyAtomicsUsed { faction: self.initiator(), card });
        self.log_objects(game, vec![m.into()]);

        for i in 0..game.players.len() {
            let p = game.player_at(i);
            let mut forces = 0i32;
            let mut special_forces = 0i32;
            // Lazy Where: each location is tested after the previous one was cleared.
            for location in game.map.shield_wall().locations().iter().copied() {
                if !(game.player(p).any_forces_in(Some(location)) > 0) {
                    continue;
                }

                game.reveal_current_no_field(p, Some(location));
                forces += game.player(p).forces_in(Some(location));
                special_forces += game.player(p).special_forces_in(Some(location));
                Player::kill_all_forces(game, p, location, false);
            }

            if (forces + special_forces) > 0 {
                let faction = game.player(p).faction;
                let m = express!(
                    TreacheryCardType::Metheor,
                    " kills ",
                    forces + special_forces,
                    faction,
                    " forces on the ",
                    game.map.shield_wall()
                )
                .with_log(FamilyAtomicsKilled { faction, forces, special_forces });
                self.log_objects(game, vec![m.into()]);
            }
        }
        Ok(())
    }

    pub fn get_message(&self, game: &Game) -> Message {
        express!(self.initiator(), " use ", TreacheryCardType::Metheor, " to destroy the ", game.map.shield_wall(), "!")
    }
}

impl GameEventMethods for MetheorPlayed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::MetheorPlayed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::MetheorPlayed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        MetheorPlayed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        MetheorPlayed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        MetheorPlayed::get_message(self, game)
    }

}
