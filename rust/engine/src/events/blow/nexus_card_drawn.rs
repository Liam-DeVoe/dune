use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct NexusCardDrawn {
    #[serde(skip)]
    pub passed: bool,
}

impl NexusCardDrawn {
    pub fn may_draw(g: &Game, p: PlayerId) -> bool {
        !g.blow().is_some_and(|b| b.factions_that_drew_nexus_card.contains_opt(g.player(p).faction)) || g.player(p).faction == g.player(p).nexus
    }

    pub fn applicable(g: &Game, p: PlayerId) -> bool {
        g.blow().is_some_and(|b| b.factions_that_may_draw_nexus_card.contains_opt(g.player(p).faction))
    }

    pub fn deal_nexus_card(&self, game: &mut Game, by: By) {
        let me = by.player();
        game.discard_nexus_card(me);

        if game.decks.nexus_card_deck.as_ref().expect("nexus card deck").is_empty() {
            let pile = game.decks.nexus_discard_pile.clone();
            game.decks.nexus_card_deck.as_mut().expect("nexus card deck").items.extend(pile);
            game.decks.nexus_discard_pile.clear();
            game.decks.nexus_card_deck.as_mut().expect("nexus card deck").shuffle(&mut game.rng);
            game.stone(Milestone::Shuffled);
            game.log(parts!["The Nexus Card discard pile was shuffled into a new Nexus Card deck"]);
        }

        let faction = game.player(by.player()).faction;
        game.log(parts![faction, " draw a Nexus Card"]);
        let card = game.decks.nexus_card_deck.as_mut().expect("nexus card deck").draw();
        game.player_mut(by.player()).nexus = Some(card);
    }
}

impl EventRules for NexusCardDrawn {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !self.passed && !Self::may_draw(game, by.player()) {
            return Err(Rejection(express!("You're not allowed to draw a Nexus Card")));
        }
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        if !self.passed {
            self.deal_nexus_card(game, by);
            game.blow_state().factions_that_drew_nexus_card.push(by.faction.required("faction")?);
        }

        if self.passed {
            let initiator = by.faction;
            if let Some(i) = game.blow_state().factions_that_may_draw_nexus_card.iter().position(|f| Some(*f) == initiator) {
                game.blow_state().factions_that_may_draw_nexus_card.remove(i);
            }
        }

        if game.blow_state().factions_that_may_draw_nexus_card.is_empty() {
            game.end_blow_phase();
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, express_part_if!(self.passed; " don't"), " draw a Nexus card")
    }
}
