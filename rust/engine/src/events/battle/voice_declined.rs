use crate::prelude::*;

/// A player who may use Voice in this battle says they won't, so Prescience can be used. Voice
/// must be used before Prescience (rulebook FAQ), and Prescience waits for this or a Voice. One
/// decline closes Voice for the battle, also for the other player who could have used it.
#[derive(Clone, Debug)]
pub struct VoiceDeclined {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for VoiceDeclined {
    fn default() -> Self {
        VoiceDeclined { base: Default::default() }
    }
}

impl VoiceDeclined {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    /// Offered to a player who openly may use Voice, while Prescience is waiting for it. A
    /// secret ally isn't offered it: declining would show that they hold the Nexus card.
    pub fn may_decline(g: &Game, p: PlayerId) -> bool {
        Voice::may_use_voice(g, p) && Voice::voice_right(g, Some(p)) == Some(VoiceRight::Open) && Voice::prescience_waits_for_voice(g)
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !self.player_opt().is_some_and(|p| Self::may_decline(game, p)) {
            return Some(express!("You cannot decline Voice now"));
        }

        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.battle.voice_declined = true;
        self.log(game);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " don't use Voice")
    }
}

impl GameEventMethods for VoiceDeclined {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::VoiceDeclined
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::VoiceDeclined(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        VoiceDeclined::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        VoiceDeclined::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        VoiceDeclined::get_message(self, game)
    }
}
