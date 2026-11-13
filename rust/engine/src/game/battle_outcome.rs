use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct BattleOutcome {
    pub agg_hero_skill_bonus: i32,
    pub agg_activated_bonus_skill: LeaderSkill,
    pub def_hero_skill_bonus: i32,
    pub def_activated_bonus_skill: LeaderSkill,
    pub agg_battle_penalty: i32,
    pub agg_activated_penalty_skill: LeaderSkill,
    pub def_battle_penalty: i32,
    pub def_activated_penalty_skill: LeaderSkill,
    pub agg_messiah_contribution: i32,
    pub def_messiah_contribution: i32,
    pub agg_homeworld_contribution: i32,
    pub def_homeworld_contribution: i32,
    pub agg_reinforcements_contribution: i32,
    pub def_reinforcements_contribution: i32,
    pub winner: Option<PlayerIndex>,
    pub loser: Option<PlayerIndex>,
    pub winner_battle_plan: Option<Rc<Battle>>,
    pub loser_battle_plan: Option<Rc<Battle>>,
    pub agg_hero_killed: bool,
    pub agg_hero_cause_of_death: TreacheryCardType,
    pub agg_hero_effective_strength: i32,
    pub def_hero_killed: bool,
    pub def_hero_cause_of_death: TreacheryCardType,
    pub def_hero_effective_strength: i32,
    pub agg_undialed_forces: i32,
    pub def_undialed_forces: i32,
    pub agg_total: f32,
    pub def_total: f32,
    pub agg_saved_by_carthag: bool,
    pub def_saved_by_carthag: bool,
    pub aggressor: PlayerIndex,
    pub defender: PlayerIndex,
}

impl BattleOutcome {
    /// An outcome with only `aggressor` and `defender` set; every other member has its default.
    pub fn new(aggressor: PlayerIndex, defender: PlayerIndex) -> Self {
        BattleOutcome {
            agg_hero_skill_bonus: 0,
            agg_activated_bonus_skill: Default::default(),
            def_hero_skill_bonus: 0,
            def_activated_bonus_skill: Default::default(),
            agg_battle_penalty: 0,
            agg_activated_penalty_skill: Default::default(),
            def_battle_penalty: 0,
            def_activated_penalty_skill: Default::default(),
            agg_messiah_contribution: 0,
            def_messiah_contribution: 0,
            agg_homeworld_contribution: 0,
            def_homeworld_contribution: 0,
            agg_reinforcements_contribution: 0,
            def_reinforcements_contribution: 0,
            winner: None,
            loser: None,
            winner_battle_plan: None,
            loser_battle_plan: None,
            agg_hero_killed: false,
            agg_hero_cause_of_death: Default::default(),
            agg_hero_effective_strength: 0,
            def_hero_killed: false,
            def_hero_cause_of_death: Default::default(),
            def_hero_effective_strength: 0,
            agg_undialed_forces: 0,
            def_undialed_forces: 0,
            agg_total: 0.0,
            def_total: 0.0,
            agg_saved_by_carthag: false,
            def_saved_by_carthag: false,
            aggressor,
            defender,
        }
    }

    pub fn loser_hero_killed(&self) -> bool {
        if self.loser == Some(self.aggressor) { self.agg_hero_killed } else { self.def_hero_killed }
    }

    pub fn winner_hero_killed(&self) -> bool {
        if self.winner == Some(self.aggressor) { self.agg_hero_killed } else { self.def_hero_killed }
    }

    pub fn get_message(&self, game: &Game) -> Message {
        express!(
            "Aggressor (",
            game.player(self.aggressor).faction,
            ") ",
            if self.winner == Some(self.aggressor) { "win" } else { "lose" },
            ". Total strength: ",
            self.agg_total,
            ", leader ",
            if self.agg_hero_killed { "killed" } else { "survives" },
            " by ",
            self.agg_hero_cause_of_death,
            ". Defender (",
            game.player(self.defender).faction,
            ") ",
            if self.winner == Some(self.defender) { "win" } else { "lose" },
            ". Total strength: ",
            self.def_total,
            ", leader ",
            if self.def_hero_killed { "killed" } else { "survives" },
            " by ",
            self.def_hero_cause_of_death,
            "."
        )
    }

}
