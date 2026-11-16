use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct MulliganPerformed {
    #[serde(skip)]
    pub passed: bool,
}

impl MulliganPerformed {
    pub fn may_mulligan(game: &Game, p: PlayerId) -> bool {
        let faction = game.player(p).faction;
        game.player(p).traitors.iter().filter(|l| l.is(faction)).count() >= 2
    }

    pub fn deal_non_harkonnen_traitor_cards(&self, game: &mut Game) {
        for _i in 1..=4 {
            // foreach over a lazy Where: the predicate is checked per player, before its body.
            for p in 0..game.players.len() {
                let f = game.players[p].faction;
                if f != Some(Faction::Harkonnen) && f != Some(Faction::Tleilaxu) {
                    let traitor = game.decks.traitor_deck.as_mut().expect("traitor deck").draw();
                    game.players[p].traitors.push(traitor);
                }
            }
        }
    }
}

impl EventRules for MulliganPerformed {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !self.passed && !Self::may_mulligan(game, by.player()) {
            return Err(Rejection(express!("You can't take a mulligan")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        if !self.passed {
            let me = by.player();
            // TraitorDeck is dereferenced before Player.Traitors is read.
            game.decks.traitor_deck.as_ref().required("traitor deck")?;
            let traitors = game.player(me).traitors.clone();
            game.decks.traitor_deck.as_mut().required("traitor deck")?.items.extend(traitors);
            game.player_mut(me).traitors.clear();
            game.decks.traitor_deck.as_mut().required("traitor deck")?.shuffle(&mut game.rng);
            game.stone(Milestone::Shuffled);
            game.deal_harkonnen_traitor_cards();
            game.enter(Phase::HarkonnenMulligan, true);
        } else {
            self.deal_non_harkonnen_traitor_cards(game);
            game.enter_select_traitors()?;
        }

        let m = self.describe(game, by);
        game.log_event(m);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, if self.passed { " pass" } else { " take" }, " a mulligan")
    }
}
