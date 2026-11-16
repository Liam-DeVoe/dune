use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ChoamDiscarded {
    pub card: Option<TreacheryCard>,
}

impl Default for ChoamDiscarded {
    fn default() -> Self {
        ChoamDiscarded { card: TreacheryCardManager::get(0) }
    }
}

impl ChoamDiscarded {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_cards(game: &Game, p: PlayerId) -> impl Iterator<Item = TreacheryCard> {
        let pl = game.player(p);
        pl.treachery_cards
            .iter()
            .copied()
            .filter(move |c| {
                (c.type_() == TreacheryCardType::Useless && !pl.has_any_high_threshold(game))
                    || (c.type_() != TreacheryCardType::Projectile
                        && c.type_() != TreacheryCardType::Poison
                        && pl.treachery_cards.iter().filter(move |to_count| to_count.type_() == c.type_()).count() > 1)
            })
    }
}

impl EventRules for ChoamDiscarded {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        let card = self.card;
        if !ChoamDiscarded::valid_cards(game, by.player()).into_iter().any(|c| Some(c) == card) {
            return Err(Rejection(express!("Invalid card")));
        }
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let Some(card) = self.card else {
            game_error!("Card is null");
        };

        game.discard_card(Some(card), true)?;
        let m = self.describe(game, by);
        game.log_event(m);

        let p = game.player_mut(by.player());
        if card.type_() == TreacheryCardType::Useless {
            p.resources += 2;
        } else {
            p.resources += 3;
        }

        game.stone(Milestone::ResourcesReceived);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.card.map(|c| c.type_()) == Some(TreacheryCardType::Useless) {
            express!(by.faction, " get ", Payment::of(2), " by discarding a ", TreacheryCardType::Useless, " card")
        } else {
            express!(by.faction, " get ", Payment::of(3), " by discarding a duplicate ", self.card)
        }
    }
}
