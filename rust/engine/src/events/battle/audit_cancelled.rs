use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct AuditCancelled {
    pub cancelled: bool,
}

impl AuditCancelled {
    pub fn cost(&self, game: &Game) -> i32 {
        Self::cost_of_cancelling(game)
    }

    pub fn cost_of_cancelling(g: &Game) -> i32 {
        Self::get_number_of_cards_that_may_be_audited(g)
    }

    pub fn get_number_of_cards_that_may_be_audited(g: &Game) -> i32 {
        std::cmp::min(if g.battle.auditor_survived_battle { 2 } else { 1 }, Self::get_cards_that_may_be_audited(g).len() as i32)
    }

    pub fn get_cards_that_may_be_audited(g: &Game) -> Vec<TreacheryCard> {
        let Some(auditee) = g.auditee() else { return Vec::new() };
        let recent_battle_plan = g.battle.current_battle.as_ref().expect("current battle").plan_of(g, Some(auditee));
        if let Some(plan) = recent_battle_plan {
            let (weapon, defense, hero) = (plan.weapon(), plan.defense(), plan.hero);
            return g
                .player(auditee)
                .treachery_cards
                .iter()
                .copied()
                // c != Hero compares a card with an IHero by reference.
                .filter(|c| Some(*c) != weapon && Some(*c) != defense && Some(Hero::TreacheryCard(*c)) != hero)
                .collect();
        }
        Vec::new()
    }
}

impl EventRules for AuditCancelled {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if self.cancelled && i32::from(game.player(by.player()).resources) < Self::get_number_of_cards_that_may_be_audited(game) {
            return Err(Rejection(express!("You can't pay enough to prevent the audit")));
        }
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        if self.cancelled {
            let cost = Payment::of(Self::get_number_of_cards_that_may_be_audited(game));
            let n = Self::get_number_of_cards_that_may_be_audited(game);
            game.log(parts![by.faction, " pay ", Faction::Choam, cost, " to cancel an audit of ", n, " cards"]);
        } else {
            let n = Self::get_number_of_cards_that_may_be_audited(game);
            game.log(parts![by.faction, " don't cancel an audit targeting ", n, " of their cards"]);
        }

        if self.cancelled {
            let me = by.player();
            let cost = self.cost(game);
            crate::game::amounts::spend(&mut game.player_mut(me).resources, cost, "spice")?;
            let choam = game.get_player(Some(Faction::Choam));
            if let Some(choam) = choam {
                let cost = self.cost(game);
                crate::game::amounts::gain(&mut game.player_mut(choam).resources, cost, "spice")?;
            }
        }

        if !self.cancelled {
            game.enter(Phase::Auditing, true);
            // A copy: the next audit clears and refills `audited_cards`, and this message
            // should keep showing the cards of this audit.
            let audited = Part::list(game.battle.audited_cards.iter().copied());
            game.log_to(by.faction.required("faction")?, parts![Faction::Choam, " see: ", audited]);
        } else {
            let winner_is_none = game.battle.battle_winner == Some(None);
            let harkonnen_must_decide = game.harkonnen_must_decide_to_capture();
            // Enter(condition, FinishBattle, ...), with FinishBattle's failure passed on.
            if winner_is_none {
                game.finish_battle()?;
            } else {
                game.enter_either(harkonnen_must_decide, Phase::CaptureDecision, Phase::BattleConclusion);
            }
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.cancelled {
            return express!(by.faction, " pay to cancel an audit");
        }
        express!(by.faction, " don't pay to cancel an audit")
    }
}
