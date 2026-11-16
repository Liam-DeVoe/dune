use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct JuicePlayed {
    pub type_: Option<JuiceType>,
    /// The fields of `Event` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for JuicePlayed {
    fn default() -> Self {
        JuicePlayed {
            type_: Default::default(),
            base: Default::default(),
        }
    }
}

impl JuicePlayed {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn valid_types(g: &Game, p: PlayerId) -> Vec<JuiceType> {
        let mut result = Vec::new();

        // `BattleWinner == Faction.None` is false while BattleWinner is null.
        if let Some(battle) = g.battle.current_battle.as_ref() {
            if battle.is_aggressor_or_defender(g, p) && g.battle.battle_winner == Some(None) {
                result.push(JuiceType::Aggressor);
            }
        }

        if (g.turn.current_main_phase == MainPhase::Bidding && g.bidding.bids.len() == 0)
            || g.turn.current_phase == Phase::BeginningOfShipAndMove
            || (g.turn.current_main_phase == MainPhase::Battle && g.battle.current_battle.is_none())
            || g.turn.current_phase == Phase::Contemplate
        {
            result.push(JuiceType::GoFirst);
        }

        result.push(JuiceType::GoLast);

        result
    }

    pub fn can_be_played_by(g: &Game, player: PlayerId) -> bool {
        let applicable_phase = g.turn.current_main_phase == MainPhase::Bidding
            || g.turn.current_phase == Phase::BeginningOfShipAndMove
            || g.turn.current_phase == Phase::GuildShip
            || g.turn.current_phase == Phase::NonGuildShip
            || g.turn.current_main_phase == MainPhase::Battle
            || g.turn.current_phase == Phase::Contemplate;

        applicable_phase && g.player(player).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Juice)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);

        let aggressor_before_juice_is_played = game.battle.current_battle.clone().and_then(|b| b.aggressive_player(game));
        game.turn.current_juice = Some(self.clone());
        game.discard(self.player(), TreacheryCardType::Juice)?;

        if let Some(battle) = game.battle.current_battle.clone() {
            if self.type_ == Some(JuiceType::Aggressor) && battle.aggressive_player(game) != aggressor_before_juice_is_played {
                std::mem::swap(&mut game.battle.defender_plan, &mut game.battle.aggressor_plan);
                std::mem::swap(&mut game.battle.defender_traitor_action, &mut game.battle.aggressor_traitor_action);
            }
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " use ", TreacheryCardType::Juice, " to ", self.type_)
    }
}

impl GameEventMethods for JuicePlayed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> EventType {
        EventType::JuicePlayed
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::JuicePlayed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        JuicePlayed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        JuicePlayed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        JuicePlayed::get_message(self, game)
    }

}
