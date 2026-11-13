use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct JuicePlayed {
    pub type_: JuiceType,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
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
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn valid_types(g: &Game, p: PlayerIndex) -> Vec<JuiceType> {
        let mut result = Vec::new();

        // `BattleWinner == Faction.None` is false while BattleWinner is null (hazards 10.1).
        if let Some(battle) = g.current_battle.as_ref() {
            if battle.is_aggressor_or_defender(g, p) && g.battle_winner == Some(Faction::None) {
                result.push(JuiceType::Aggressor);
            }
        }

        if (g.current_main_phase == MainPhase::Bidding && g.bids.len() == 0)
            || g.current_phase == Phase::BeginningOfShipAndMove
            || (g.current_main_phase == MainPhase::Battle && g.current_battle.is_none())
            || g.current_phase == Phase::Contemplate
        {
            result.push(JuiceType::GoFirst);
        }

        result.push(JuiceType::GoLast);

        result
    }

    pub fn can_be_played_by(g: &Game, player: PlayerIndex) -> bool {
        let applicable_phase = g.current_main_phase == MainPhase::Bidding
            || g.current_phase == Phase::BeginningOfShipAndMove
            || g.current_phase == Phase::GuildShip
            || g.current_phase == Phase::NonGuildShip
            || g.current_main_phase == MainPhase::Battle
            || g.current_phase == Phase::Contemplate;

        applicable_phase && g.player(player).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Juice)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        self.log(game);

        let aggressor_before_juice_is_played = game.current_battle.clone().and_then(|b| b.aggressive_player(game));
        game.current_juice = Some(self.clone());
        game.discard(self.player(), TreacheryCardType::Juice);

        if let Some(battle) = game.current_battle.clone() {
            if self.type_ == JuiceType::Aggressor && battle.aggressive_player(game) != aggressor_before_juice_is_played {
                std::mem::swap(&mut game.defender_plan, &mut game.aggressor_plan);
                std::mem::swap(&mut game.defender_traitor_action, &mut game.aggressor_traitor_action);
            }
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " use ", TreacheryCardType::Juice, " to ", self.type_)
    }
}

impl GameEventMethods for JuicePlayed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::JuicePlayed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::JuicePlayed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        JuicePlayed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        JuicePlayed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        JuicePlayed::get_message(self, game)
    }

}
