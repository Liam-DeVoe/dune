use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct NexusVoted {
    #[serde(skip)]
    pub passed: bool,
}

impl EventRules for NexusVoted {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        game.blow.nexus_votes.push(NexusVote { initiator: by.faction, passed: self.passed });
        if game.blow.nexus_votes.len() == game.players.len() {
            let passed = game.blow.nexus_votes.iter().filter(|v| v.passed).count() as i32;
            if (2i32 * passed) >= game.players.len() as i32 {
                let condition = game.phase() == Phase::VoteAllianceA && game.applicable(Rule::IncreasedResourceFlow);
                if condition { game.enter_blow_b()? } else { game.start_nexus_card_phase() }
            } else {
                if game.applicable(Rule::BreakAlliancesOnAlliancePhase) {
                    game.break_all_alliances();
                }
                let condition = game.phase() == Phase::VoteAllianceA;
                game.enter_alliance_phase(condition);
            }
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " vote ", if self.passed { "No" } else { "Yes" })
    }
}
