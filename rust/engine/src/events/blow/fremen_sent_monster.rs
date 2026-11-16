use crate::log::blow::WormSentLog;
use crate::game::MonsterAppearance;
use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct FremenSentMonster {
    pub territory: Option<Territory>,
}

impl Default for FremenSentMonster {
    fn default() -> Self {
        FremenSentMonster {
            territory: Territory::from_id(0),
        }
    }
}

impl FremenSentMonster {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game) -> impl Iterator<Item = Territory> {
        g.board.map.territories(false).filter(move |t| !t.is_protected_from_worm())
    }
}

impl EventRules for FremenSentMonster {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        if self.territory.is_none() {
            return Err(Rejection(express!("No territory selected")));
        }
        if self.territory.expect("territory").is_protected_from_worm() {
            return Err(Rejection(express!("Selected territory is protected")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // A null Territory (only without validation) is stored in WormSentLog and the
        // MonsterAppearance; PerformMonster then throws on `m.Territory.Locations`.
        let msg = self.describe(game, by);
        let territory = self.territory.required("territory")?;
        let msg = msg.with_log(WormSentLog::new(by.faction.required("faction")?, territory));
        game.log(parts![msg]);
        let monster = MonsterAppearance::new(territory, false);
        game.board.monsters.push(monster.clone());
        game.perform_monster(&monster)?;
        let condition = game.phase() == Phase::FremenSendingMonsterA;
        game.enter_blow(BlowStep::either(condition, BlowStep::BlowA, BlowStep::BlowB));
        game.draw_resource_card()?;
        game.let_factions_discard_surplus_cards();
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " send ", Concept::Monster, " to ", self.territory)
    }
}
