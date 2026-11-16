use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResourcesAudited {
    pub target: Option<Faction>,
}

impl ResourcesAudited {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_factions(game: &Game, player: PlayerId) -> impl Iterator<Item = Faction> {
        (0..game.players.len())
            .map(move |i| game.player_at(i))
            .filter(move |opp| {
                let o = game.player(*opp);
                *opp != player
                    && !game.turn.resource_audited_factions.contains_opt(o.faction)
                    && (game.player(player).home_worlds.iter().any(|hw| o.any_forces_in(Some(hw.location())) > 0)
                        || o.home_worlds.iter().any(|hw| game.player(player).any_forces_in(Some(hw.location())) > 0))
            })
            .filter_map(move |p| game.player(p).faction)
    }
}

impl EventRules for ResourcesAudited {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !ResourcesAudited::valid_factions(game, by.player()).any(|v| Some(v) == self.target) {
            return Err(Rejection(express!("Invalid faction")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.turn.resource_audited_factions.push(self.target.required("faction")?);
        let target = game.get_player(self.target);
        let m = self.describe(game, by);
        game.log_event(m);
        let t = game.player(target.required("target")?);
        let resources = i32::from(t.resources);
        let weapons = t.treachery_cards.iter().filter(|tc| tc.is_weapon()).count() as i32;
        let defenses = t.treachery_cards.iter().filter(|tc| tc.is_defense()).count() as i32;
        game.log_to(
            by.faction.required("faction")?,
            parts![self.target, " own ", Payment::of(resources), ", ", weapons, " weapons and ", defenses, " defenses"],
        );
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " force ", self.target, " to reveal the amount of weapons, defenses and ", Concept::Resource)
    }
}
