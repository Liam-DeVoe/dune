use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ChoamMovePrevention {
    pub territory: Option<Territory>,
}

impl Default for ChoamMovePrevention {
    fn default() -> Self {
        ChoamMovePrevention { territory: Territory::from_id(0) }
    }
}

impl ChoamMovePrevention {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_territories(game: &Game, p: PlayerId) -> impl Iterator<Item = Territory> {
        game.player(p).territories_with_forces(game)
    }

    pub fn can_be_played_by(g: &Game, p: PlayerId) -> bool {
        let pl = g.player(p);
        pl.faction == Some(Faction::Choam)
            && !ChoamMovePrevention::valid_territories(g, p).next().is_none()
            && ((!g.prevented(FactionAdvantage::ChoamDiscarding) && ChoamMovePrevention::card_to_use(g, p).is_some())
                || (NexusPlayed::can_use_cunning(g, p) && !pl.treachery_cards.is_empty()))
    }

    pub fn card_to_use(game: &Game, p: PlayerId) -> Option<TreacheryCard> {
        game.player(p).treachery_cards.iter().copied().find(|c| c.id() == TreacheryCardManager::CARD_BALISET)
    }
}

impl EventRules for ChoamMovePrevention {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        let territory = self.territory;
        if !ChoamMovePrevention::valid_territories(game, by.player()).any(|v| Some(v) == territory) {
            return Err(Rejection(express!("Invalid territory")));
        }
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
            let card = ChoamMovePrevention::card_to_use(game, me);
            game.discard_card(card, true)?;
        }

        // `Territory!` does not throw: a null territory (only without validation) is stored.
        game.ship_move.current_blocked_territories.push(self.territory.required("territory")?);
        game.stone(Milestone::SpecialUselessPlayed);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " prevent forces moving into ", self.territory)
    }
}
