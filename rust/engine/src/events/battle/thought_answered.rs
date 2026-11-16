use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ThoughtAnswered {
    pub card: Option<TreacheryCard>,
}

impl Default for ThoughtAnswered {
    fn default() -> Self {
        ThoughtAnswered {
            card: TreacheryCardManager::get(0),
        }
    }
}

impl ThoughtAnswered {
    pub fn valid_cards(g: &Game, p: PlayerId) -> Vec<TreacheryCard> {
        Self::valid_cards_opt(g, Some(p))
    }

    /// `ValidCards` with a possibly null `p`: `g.CurrentThought.Card` is read before `p.Has`.
    fn valid_cards_opt(g: &Game, p: Option<PlayerId>) -> Vec<TreacheryCard> {
        let card = g.battle.current_thought.as_ref().expect("current thought").card;
        let p = p.expect("player");
        if g.player(p).has(card) {
            return vec![card.expect("card")];
        }

        g.player(p).treachery_cards.clone()
    }
}

impl EventRules for ThoughtAnswered {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if Self::valid_cards_opt(game, by.player).is_empty() {
            return Ok(());
        }
        // Enumerable.Contains(null) on a sequence of cards is false.
        let card = self.card;
        if !Self::valid_cards_opt(game, by.player).into_iter().any(|c| Some(c) == card) {
            return Err(Rejection(express!("Select a valid card to show")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        match self.card {
            None => {
                game.log(parts![by.faction, " don't own any cards"]);
            }
            Some(card) => {
                let thought_initiator = game.battle.current_thought.as_ref().required("current thought")?.initiator();
                game.log_to(thought_initiator.required("faction")?, parts!["In response, ", by.faction, " show you: ", card]);
                let thought_initiator = game.battle.current_thought.as_ref().required("current thought")?.initiator();
                game.log(parts!["In response, ", by.faction, " show ", thought_initiator, " a card"]);
                let thought_initiator = game.battle.current_thought.as_ref().required("current thought")?.initiator();
                game.register_known_by_faction(thought_initiator.required("faction")?, card);
            }
        }

        game.enter(Phase::BattlePhase, true);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.card.is_none() {
            return express!(by.faction, " don't have a card to show");
        }
        express!(by.faction, " show one of their cards")
    }
}
