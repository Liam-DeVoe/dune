use crate::prelude::*;

/// A player who may use Voice in this battle says they won't, so Prescience can be used. Voice
/// must be used before Prescience (rulebook FAQ), and Prescience waits for this or a Voice. One
/// decline closes Voice for the battle, also for the other player who could have used it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VoiceDeclined {}

impl VoiceDeclined {
    /// Offered to a player who openly may use Voice, while Prescience is waiting for it. A
    /// secret ally isn't offered it: declining would show that they hold the Nexus card.
    pub fn may_decline(g: &Game, p: PlayerId) -> bool {
        Voice::may_use_voice(g, p) && Voice::voice_right(g, Some(p)) == Some(VoiceRight::Open) && Voice::prescience_waits_for_voice(g)
    }
}

impl EventRules for VoiceDeclined {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !by.player.is_some_and(|p| Self::may_decline(game, p)) {
            return Err(Rejection(express!("You cannot decline Voice now")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.battle.voice_declined = true;
        let m = self.describe(game, by);
        game.log_event(m);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " don't use Voice")
    }
}
