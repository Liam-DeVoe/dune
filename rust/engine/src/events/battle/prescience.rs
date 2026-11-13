use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct Prescience {
    pub aspect: PrescienceAspect,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for Prescience {
    fn default() -> Self {
        Prescience {
            aspect: Default::default(),
            base: Default::default(),
        }
    }
}

impl Prescience {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn valid_aspects(g: &Game, p: PlayerIndex) -> Vec<PrescienceAspect> {
        let battle = g.current_battle.as_ref().nre();
        let opponent = battle.opponent_of(g, p).nre();
        // HasNoFieldIn(Territory) with a null territory: SpecialForcesIn(null) is 0, so false.
        let has_no_field = match battle.territory(g) {
            Some(t) => g.player(opponent).has_no_field_in(t),
            None => false,
        };
        if has_no_field {
            return vec![PrescienceAspect::Leader, PrescienceAspect::Weapon, PrescienceAspect::Defense];
        }

        vec![PrescienceAspect::Dial, PrescienceAspect::Leader, PrescienceAspect::Weapon, PrescienceAspect::Defense]
    }

    pub fn may_use_prescience(g: &Game, p: PlayerIndex) -> bool {
        if let Some(battle) = g.current_battle.as_ref() {
            if g.current_prescience.is_none() && !g.prevented(FactionAdvantage::AtreidesBattlePlanPrescience) {
                if g.player(p).faction == Faction::Atreides {
                    return battle.is_involved(g, p);
                }
                if g.player(p).ally == Faction::Atreides && g.atreides_shares_prescience {
                    return battle.is_aggressor_or_defender(g, p);
                }
            }
        }

        false
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.current_prescience = Some(self.clone());
        self.log(game);
        game.stone(Milestone::Prescience);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " use Prescience to see the enemy ", self.aspect)
    }
}

impl GameEventMethods for Prescience {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::Prescience
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Prescience(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Prescience::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        Prescience::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Prescience::get_message(self, game)
    }

}
