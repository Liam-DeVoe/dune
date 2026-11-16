use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FactionSelected {
    pub initiator_player_name: Option<String>,
    pub seat: i32,
    pub faction: Option<Faction>,
}

impl FactionSelected {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_factions(g: &Game) -> impl Iterator<Item = Faction> {
        // The list is shared (RefCell), so take its items now. Nothing changes it while a caller enumerates.
        let factions: Vec<Faction> = g.rules.factions_in_play.clone();
        factions.into_iter().filter(move |f| g.players.iter().all(|p| p.faction != Some(*f)))
    }
}

impl EventRules for FactionSelected {
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        if self.faction != None && !Self::valid_factions(game).any(|v| Some(v) == self.faction) {
            return Err(Rejection(express!("Faction not available")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let initiator = Seat::from_number(self.seat).and_then(|s| game.get_player_by_seat(s));

        if let Some(initiator) = initiator {
            game.player_mut(initiator).faction = self.faction;

            let m = self.describe(game, by);
        game.log_event(m);
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, _by: By) -> Message {
        express!(self.initiator_player_name.clone(), " plays ", self.faction)
    }
}
