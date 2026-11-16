use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ChoamExtraMove {}

impl ChoamExtraMove {
    pub fn can_be_played_by(g: &Game, p: PlayerId) -> bool {
        let pl = g.player(p);
        pl.faction == Some(Faction::Choam)
            && ((!g.prevented(FactionAdvantage::ChoamDiscarding) && ChoamExtraMove::card_to_use(g, p).is_some())
                || (NexusPlayed::can_use_cunning(g, p) && !pl.treachery_cards.is_empty()))
    }

    pub fn card_to_use(game: &Game, p: PlayerId) -> Option<TreacheryCard> {
        game.player(p).treachery_cards.iter().copied().find(|c| c.id() == TreacheryCardManager::CARD_KULON)
    }
}

impl EventRules for ChoamExtraMove {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);

        let me = by.player();
        if NexusPlayed::can_use_cunning(game, me) {
            game.discard_nexus_card(me);
            game.stone(Milestone::NexusPlayed);
            game.let_player_discard_treachery_card_of_choice(by.faction.required("faction")?);
        } else {
            let card = ChoamExtraMove::card_to_use(game, me);
            game.discard_card(card, true)?;
        }

        game.ship_move.choam_has_extra_move = true;
        game.stone(Milestone::SpecialUselessPlayed);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " gain extra movement")
    }
}
