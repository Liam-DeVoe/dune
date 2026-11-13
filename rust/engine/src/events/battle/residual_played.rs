use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct ResidualPlayed {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for ResidualPlayed {
    fn default() -> Self {
        ResidualPlayed {
            base: Default::default(),
        }
    }
}

macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

impl ResidualPlayed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn may_play(g: &Game, p: PlayerIndex) -> bool {
        g.current_battle.as_ref().nre().is_aggressor_or_defender(g, p)
            && g.player(p).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Residual)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.discard(self.player(), TreacheryCardType::Residual)?;
        let battle = game.current_battle.clone().required("current battle")?;
        let opponent = battle.opponent_of_faction(game, self.initiator()).required("opponent of faction")?;
        let candidates: Vec<Hero> = game
            .player(opponent)
            .leaders
            .iter()
            .map(|l| Hero::Leader(*l))
            .filter(|l| game.is_alive(*l) && game.can_join_current_battle(*l))
            .collect();
        let mut leaders_to_kill = Deck::new(candidates, game.random.clone());
        leaders_to_kill.shuffle();

        if !leaders_to_kill.is_empty() {
            let to_kill = leaders_to_kill.draw();
            let opponent_plan = battle.plan_of(game, Some(opponent));
            if let Some(opponent_plan) = opponent_plan {
                if opponent_plan.hero() == Some(to_kill) {
                    game.revoke_plan(&opponent_plan);
                }
            }
            game.kill_hero(to_kill);
            game.log(elems![TreacheryCardType::Residual, " kills ", to_kill]);
        } else {
            let faction = game.player(opponent).faction;
            game.log(elems![faction, " have no available leaders to kill"]);
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " use ", TreacheryCardType::Residual, " to kill a random opponent leader")
    }
}

impl GameEventMethods for ResidualPlayed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::ResidualPlayed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::ResidualPlayed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ResidualPlayed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        ResidualPlayed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ResidualPlayed::get_message(self, game)
    }

}
