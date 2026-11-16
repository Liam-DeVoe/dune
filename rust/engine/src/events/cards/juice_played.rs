use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct JuicePlayed {
    #[serde(rename = "juiceType")]
    pub type_: Option<JuiceType>,
}

impl JuicePlayed {
    pub fn valid_types(g: &Game, p: PlayerId) -> Vec<JuiceType> {
        let mut result = Vec::new();
        // `BattleWinner == Faction.None` is false while BattleWinner is null.
        if let Some(battle) = g.battle.current_battle.as_ref() {
            if battle.is_aggressor_or_defender(g, p) && g.battle.battle_winner == Some(None) {
                result.push(JuiceType::Aggressor);
            }
        }

        if (g.turn.current_main_phase == MainPhase::Bidding && g.bidding.bids.len() == 0)
            || g.phase() == Phase::BeginningOfShipAndMove
            || (g.turn.current_main_phase == MainPhase::Battle && g.battle.current_battle.is_none())
            || g.phase() == Phase::Contemplate
        {
            result.push(JuiceType::GoFirst);
        }

        result.push(JuiceType::GoLast);
        result
    }

    pub fn can_be_played_by(g: &Game, player: PlayerId) -> bool {
        let applicable_phase = g.turn.current_main_phase == MainPhase::Bidding
            || g.phase() == Phase::BeginningOfShipAndMove
            || g.phase() == Phase::GuildShip
            || g.phase() == Phase::NonGuildShip
            || g.turn.current_main_phase == MainPhase::Battle
            || g.phase() == Phase::Contemplate;
        applicable_phase && g.player(player).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Juice)
    }
}

impl EventRules for JuicePlayed {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        let aggressor_before_juice_is_played = game.battle.current_battle.clone().and_then(|b| b.aggressive_player(game));
        game.turn.current_juice = Some(JuiceUsed { submitter: by, type_: self.type_ });
        game.discard(by.player(), TreacheryCardType::Juice)?;

        if let Some(battle) = game.battle.current_battle.clone() {
            if self.type_ == Some(JuiceType::Aggressor) && battle.aggressive_player(game) != aggressor_before_juice_is_played {
                std::mem::swap(&mut game.battle.defender_plan, &mut game.battle.aggressor_plan);
                std::mem::swap(&mut game.battle.defender_traitor_action, &mut game.battle.aggressor_traitor_action);
            }
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " use ", TreacheryCardType::Juice, " to ", self.type_)
    }
}
