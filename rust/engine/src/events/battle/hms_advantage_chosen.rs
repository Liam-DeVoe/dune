use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct HMSAdvantageChosen {
    pub advantage: Option<StrongholdAdvantage>,
}

impl HMSAdvantageChosen {
    pub fn can_be_played(g: &Game, p: PlayerId) -> bool {
        let hms = g.board.map.hidden_mobile_stronghold();
        g.battle.chosen_hms_advantage == Some(None)
            && g.battle.current_battle.is_some()
            && g.battle.current_battle.as_ref().expect("current battle").is_aggressor_or_defender(g, p)
            && g.battle.current_battle.as_ref().expect("current battle").territory == Some(hms.location().territory())
            && g.owns_stronghold(g.player(p).faction, hms.location())
            && !HMSAdvantageChosen::valid_advantages(g, p).is_empty()
    }

    pub fn valid_advantages(g: &Game, p: PlayerId) -> Vec<StrongholdAdvantage> {
        let mut result = Vec::new();
        if g.owner_of_location(g.board.map.arrakeen()) == Some(p) {
            result.push(StrongholdAdvantage::FreeResourcesForBattles);
        }
        if g.owner_of_location(g.board.map.carthag()) == Some(p) {
            result.push(StrongholdAdvantage::CountDefensesAsAntidote);
        }
        if g.owner_of_location(g.board.map.sietch_tabr()) == Some(p) {
            result.push(StrongholdAdvantage::CollectResourcesForDial);
        }
        if g.owner_of_location(g.board.map.tueks_sietch()) == Some(p) {
            result.push(StrongholdAdvantage::CollectResourcesForUseless);
        }
        if g.owner_of_location(g.board.map.habbanya_sietch()) == Some(p) {
            result.push(StrongholdAdvantage::WinTies);
        }
        result
    }
}

impl EventRules for HMSAdvantageChosen {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        game.battle.chosen_hms_advantage = Some(self.advantage);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " use the ", self.advantage, " stronghold advantage for this battle")
    }
}
