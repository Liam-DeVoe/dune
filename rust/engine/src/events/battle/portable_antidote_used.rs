use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct PortableAntidoteUsed {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for PortableAntidoteUsed {
    fn default() -> Self {
        PortableAntidoteUsed {
            base: Default::default(),
        }
    }
}

impl PortableAntidoteUsed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn can_be_played(g: &Game, p: PlayerIndex) -> bool {
        let card = g.player(p).card(TreacheryCardType::PortableAntidote);
        if let Some(card) = card {
            let plan = g.current_battle.as_ref().and_then(|b| b.plan_of(g, Some(p)));
            if let Some(plan) = plan {
                if plan.defense().is_none()
                    && g.current_portable_antidote_used.is_none()
                    && Battle::valid_defenses(g, p, plan.weapon(), g.current_battle.as_ref().and_then(|b| b.territory(g)), false)
                        .contains(&Some(card))
                {
                    return true;
                }
            }
        }

        false
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        self.log(game);
        game.current_portable_antidote_used = Some(self.clone());
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " equip a ", TreacheryCardType::PortableAntidote)
    }
}

impl GameEventMethods for PortableAntidoteUsed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::PortableAntidoteUsed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::PortableAntidoteUsed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        PortableAntidoteUsed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        PortableAntidoteUsed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        PortableAntidoteUsed::get_message(self, game)
    }

}
