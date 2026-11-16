use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Discarded {
    pub card: Option<TreacheryCard>,
}

impl Default for Discarded {
    fn default() -> Self {
        Discarded {
            card: TreacheryCardManager::get(0),
        }
    }
}

impl EventRules for Discarded {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if self.card.is_none() {
            return Err(Rejection(express!("Choose a card to discard")));
        }
        if !game.player(by.player()).has(self.card) {
            return Err(Rejection(express!("Invalid card")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let initiator = by.faction;
        if let Some(factions) = game.factions_that_must_discard_mut() {
            if let Some(i) = factions.iter().position(|f| Some(*f) == initiator) {
                factions.remove(i);
            }
        }
        let m = self.describe(game, by);
        let mut fact = BattleFact::new(BattleFactKind::Discard, initiator.required("faction")?);
        fact.card = self.card;
        let m = m.with_fact(fact).with_log(AsideLog::aside());
        game.log(vec![m.into()]);
        game.discard_card_of(by.player, self.card, false);

        if game.factions_that_must_discard().is_none_or(|f| f.is_empty()) {
            game.resume_phase(Phase::Discarding, true)?;
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " discard ", self.card)
    }
}
