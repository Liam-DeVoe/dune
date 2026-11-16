use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct TraitorsSelected {
    pub selected_traitor: Option<Hero>,
}

impl Default for TraitorsSelected {
    fn default() -> Self {
        TraitorsSelected {
            selected_traitor: TreacheryCardManager::get(0).map(Hero::TreacheryCard),
        }
    }
}

impl EventRules for TraitorsSelected {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        let selected = self.selected_traitor;
        if !game.player(by.player()).traitors.iter().any(|t| Some(*t) == selected) {
            return Err(Rejection(express!("Invalid traitor")));
        }
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // Only reached outside the phase when events are replayed without validation.
        if game.setup().is_none() {
            game_error!("Not in the setup phase");
        }
        let me = by.player();
        let selected = self.selected_traitor;
        // Snapshot (ToList).
        let to_remove: Vec<Hero> = game.player(me).traitors.iter().copied().filter(|l| Some(*l) != selected).collect();

        for l in to_remove {
            game.decks.traitor_deck.as_mut().required("traitor deck")?.items.push(l);
            let traitors = &mut game.player_mut(me).traitors;
            if let Some(i) = traitors.iter().position(|t| *t == l) {
                traitors.remove(i);
            }
            game.player_mut(me).discarded_traitors.insert(l);
            game.player_mut(me).known_non_traitors.insert(l);
        }

        game.turn.has_acted_or_passed.push(by.faction.required("faction")?);
        let m = self.describe(game, by);
        game.log_event(m);

        if game.everyone_acted_or_passed() {
            game.assign_face_dancers()?;
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " pick their traitor")
    }
}
