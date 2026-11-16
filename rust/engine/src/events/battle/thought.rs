use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Thought {
    pub card: Option<TreacheryCard>,
}

impl Default for Thought {
    fn default() -> Self {
        Thought {
            card: TreacheryCardManager::get(0),
        }
    }
}

impl Thought {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_cards(g: &Game) -> impl Iterator<Item = TreacheryCard> {
        TreacheryCardManager::get_cards_in_play(g)
            .into_iter()
            .filter(move |c| c.is_weapon() && c.type_() != TreacheryCardType::Chemistry)
    }

    pub fn may_be_used(game: &Game, player: PlayerId) -> bool {
        game.has_skill_in_front_of_shield(player, LeaderSkill::Thinker)
            && game.battle.current_battle.is_some()
            && game.battle.current_thought.is_none()
            && game.battle.current_battle.as_ref().expect("current battle").is_aggressor_or_defender(game, player)
    }
}

impl EventRules for Thought {
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        // Enumerable.Contains(null) on a sequence of cards is false.
        let card = self.card;
        if !Self::valid_cards(game).into_iter().any(|c| Some(c) == card) {
            return Err(Rejection(express!("Invalid card")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.battle.current_thought = Some(ThoughtAsked { submitter: by, card: self.card });
        let battle = game.battle.current_battle.clone().required("current battle")?;
        let opponent = game.player(battle.opponent_of_faction(game, by.faction).required("opponent of faction")?).faction;
        game.log(parts![by.faction, " use their ", LeaderSkill::Thinker, " skill to ask ", opponent, " if they have a ", self.card]);
        game.stone(Milestone::Prescience);
        game.enter(Phase::Thought, true);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " use their ", LeaderSkill::Thinker, " skill to ask a question")
    }
}
