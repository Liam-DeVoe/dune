use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MetheorPlayed {
}

impl MetheorPlayed {
    pub fn may_play_metheor(g: &Game, p: PlayerId) -> bool {
        g.turn.current_turn > Turn::FIRST
            && g.player(p).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Metheor)
            && MetheorPlayed::has_forces_at_or_near_shield_wall(g, p)
    }

    pub fn has_forces_at_or_near_shield_wall(g: &Game, p: PlayerId) -> bool {
        let pl = g.player(p);
        if pl.occupies_territory(g, Some(g.board.map.shield_wall())) {
            return true;
        }

        for shieldwall_location in g.board.map.shield_wall().locations().iter().copied() {
            //checks locations that are immediately adjacent to the shield wall
            if shieldwall_location.neighbours(&g.board.map).iter().any(|l| pl.occupies(g, Some(*l))) {
                return true;
            }

            //checks locations that are in a territory adjacent to the shield wall (but may be further away and separated by storm)
            if g.board.map
                .find_neighbours(shieldwall_location, 1, false, pl.faction.expect("faction"), g, false)
                .iter()
                .any(|l| pl.occupies(g, Some(*l)))
            {
                return true;
            }
        }

        false
    }
}

impl EventRules for MetheorPlayed {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !MetheorPlayed::may_play_metheor(game, by.player()) {
            return Err(Rejection(express!("You cannot use ", TreacheryCardType::Metheor)));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let me = by.player();
        let card = game.player(me).card(TreacheryCardType::Metheor);
        game.stone(Milestone::MetheorUsed);
        game.board.shield_wall_destroyed = true;
        if let Some(card) = card {
            crate::iter::remove_first(&mut game.player_mut(me).treachery_cards, &card);
        }
        game.decks.removed_treachery_cards.push(card.required("a Metheor card")?);
        let m = self.describe(game, by).with_log(FamilyAtomicsUsed { faction: by.faction.required("faction")?, card });
        game.log(vec![m.into()]);

        for i in 0..game.players.len() {
            let p = game.player_at(i);
            let mut forces = 0i32;
            let mut special_forces = 0i32;
            // Lazy Where: each location is tested after the previous one was cleared.
            for location in game.board.map.shield_wall().locations().iter().copied() {
                if !(game.player(p).any_forces_in(Some(location)) > 0) {
                    continue;
                }

                game.reveal_current_no_field(p, Some(location))?;
                forces += game.player(p).forces_in(Some(location));
                special_forces += game.player(p).special_forces_in(Some(location));
                Player::kill_all_forces(game, p, location, false)?;
            }

            if (forces + special_forces) > 0 {
                let faction = game.player(p).faction;
                let m = express!(
                    TreacheryCardType::Metheor,
                    " kills ",
                    forces + special_forces,
                    faction,
                    " forces on the ",
                    game.board.map.shield_wall()
                )
                .with_log(FamilyAtomicsKilled { faction: faction.required("faction")?, forces, special_forces });
                game.log(vec![m.into()]);
            }
        }
        Ok(())
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        express!(by.faction, " use ", TreacheryCardType::Metheor, " to destroy the ", game.board.map.shield_wall(), "!")
    }
}
