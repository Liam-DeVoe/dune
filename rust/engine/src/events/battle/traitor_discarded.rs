use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
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
        // Known quirk (known-issues.md): the message says the traitor is shuffled into the deck,
        // but the deck is not shuffled.
        let traitor = self.traitor;
        let traitor = traitor.required("traitor")?;
        game.decks.traitor_deck.as_mut().required("traitor deck")?.items.push(traitor);
        let me = by.player();
        crate::iter::remove_first(&mut game.player_mut(me).traitors, &traitor);
        let remaining = game.traitors_to_discard_mut().required("traitors to discard")?;
        *remaining -= 1;
        let remaining = *remaining;
        if remaining == 0 {
            game.resume_phase(Phase::DiscardingTraitor, true)?;
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " shuffle a traitor into the Traitor deck")
    }
}
