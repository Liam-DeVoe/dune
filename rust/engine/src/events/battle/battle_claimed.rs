use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct BattleClaimed {
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for BattleClaimed {
    fn default() -> Self {
        BattleClaimed {
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl BattleClaimed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);
        game.current_ecaz_or_ally_fighter = if self.passed { game.get_ally(Some(Faction::Ecaz)) } else { Some(Faction::Ecaz) };
        game.enter(Phase::BattlePhase, true);
        game.initiate_battle();
        self.determine_ecaz_contribution(game);
        Ok(())
    }

    pub fn determine_ecaz_contribution(&self, game: &mut Game) {
        let fighter = game.get_player(game.current_ecaz_or_ally_fighter);

        if fighter.is_some()
            && game.current_battle.is_some()
            && (game.player(fighter.expect("fighter")).is(Faction::Ecaz) || game.player(fighter.expect("fighter")).ally == Faction::Ecaz)
        {
            let ecaz = self.get_player(game, Some(Faction::Ecaz));
            if let Some(ecaz) = ecaz {
                let territory = game.current_battle.as_ref().expect("current battle").territory(game).expect("territory");
                let forces = game.player(ecaz).any_forces_in_territory(Some(territory));
                // (int)Math.Ceiling(0.5f * n): the float widens to double
                game.current_ecaz_battle_contribution = ((0.5f32 * forces as f32) as f64).ceil() as i32;
            }
        } else {
            game.current_ecaz_battle_contribution = 0;
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if !self.passed {
            return express!(Faction::Ecaz, " will fight this battle");
        }
        express!(Faction::Ecaz, "'s ally will fight this battle")
    }

}

impl GameEventMethods for BattleClaimed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::BattleClaimed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::BattleClaimed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        BattleClaimed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        BattleClaimed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        BattleClaimed::get_message(self, game)
    }

}

impl PassableGameEventMethods for BattleClaimed {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
