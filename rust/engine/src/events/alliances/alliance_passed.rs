use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AlliancePassed {
}

impl EventRules for AlliancePassed {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);

        let initiator = by.faction;
        loop {
            let withdrawn_offer = game.alliances.current_alliance_offers.iter().position(|x| x.initiator == initiator || x.target == initiator);
            let Some(i) = withdrawn_offer else { break };
            // List.Remove(withdrawnOffer): the first occurrence of that object, which is at i.
            game.alliances.current_alliance_offers.remove(i);
        }

        game.turn.has_acted_or_passed.push(initiator.required("faction")?);
        game.end_nexus_if_everyone_is_done()?;
        Ok(())
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        self.try_describe(game, by).expect("the message of an applicable AlliancePassed event")
    }

    fn try_describe(&self, game: &Game, by: By) -> GameResult<Message> {
        let player = game.player(by.player.required("player")?);
        Ok(if player.has_ally() {
            express!(by.faction, " keep their alliance with ", player.ally)
        } else {
            express!(by.faction, " will not make an alliance")
        })
    }
}
