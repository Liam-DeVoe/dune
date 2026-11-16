use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ChoamKarmaPrevention {
    pub target: Option<Faction>,
}

impl ChoamKarmaPrevention {
    pub fn can_be_played_by(g: &Game, p: PlayerId) -> bool {
        g.player(p).faction == Some(Faction::Choam)
            && ((!g.prevented(FactionAdvantage::ChoamDiscarding) && ChoamKarmaPrevention::card_to_use(g, p).is_some())
                || (NexusPlayed::can_use_cunning(g, p) && !g.player(p).treachery_cards.is_empty()))
    }

    pub fn card_to_use(game: &Game, p: PlayerId) -> Option<TreacheryCard> {
        game.player(p).treachery_cards.iter().copied().find(|c| c.id() == TreacheryCardManager::CARD_KULL_WAHAD)
    }
}

impl EventRules for ChoamKarmaPrevention {
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
            let card = ChoamKarmaPrevention::card_to_use(game, me);
            game.discard_card(card, true)?;
        }

        game.turn.current_karma_prevention = Some(KarmaPrevention { target: self.target });
        game.stone(Milestone::SpecialUselessPlayed);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " prevent ", self.target, " from using ", TreacheryCardType::Karma, " this phase")
    }
}
