use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct TraitorDiscarded {
    pub traitor: Option<Hero>,
}

impl Default for TraitorDiscarded {
    fn default() -> Self {
        TraitorDiscarded {
            traitor: TreacheryCardManager::get(0).map(Hero::TreacheryCard),
        }
    }
}

impl TraitorDiscarded {
    pub fn valid_traitors(game: &Game, p: PlayerId) -> Vec<Hero> {
        game.player(p).traitors.clone()
    }
}

impl EventRules for TraitorDiscarded {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        let valid = Self::valid_traitors(game, by.player());
        // List<IHero>.Contains(null) is false: the list holds no nulls.
        let traitor = self.traitor;
        if !valid.iter().any(|t| Some(*t) == traitor) {
            return Err(Rejection(express!("Invalid traitor")));
        }
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        // C# parity: the message says the traitor is shuffled into the deck, but the deck is not shuffled.
        // A null Traitor (only reachable without validation) is added to the deck as null.
        let traitor = self.traitor;
        let traitor = traitor.required("traitor")?;
        game.decks.traitor_deck.as_mut().required("traitor deck")?.items.push(traitor);
        let me = by.player();
        crate::iter::remove_first(&mut game.player_mut(me).traitors, &traitor);
        game.turn.number_of_traitors_to_discard -= 1;
        if game.turn.number_of_traitors_to_discard == 0 {
            let phase = game.turn.phase_before_discarding_traitor.required("phase before discarding traitor")?;
            game.enter(phase, true);
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " shuffle a traitor into the Traitor deck")
    }
}
