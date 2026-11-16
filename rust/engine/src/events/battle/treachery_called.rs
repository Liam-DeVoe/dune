use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct TreacheryCalled {
    pub traitor_called: bool,
}

impl TreacheryCalled {
    pub fn may_call_treachery(g: &Game, p: PlayerId) -> bool {
        if let (Some(aggressor_plan), Some(defender_plan)) = (&g.battle.aggressor_plan, &g.battle.defender_plan) {
            let territory = g.battle.current_battle.as_ref().expect("current battle").territory.expect("territory");
            let pl = g.player(p);
            if !territory.is_homeworld() || pl.is_native(territory) {
                if !defender_plan.messiah {
                    if (aggressor_plan.initiator() == pl.faction && TreacheryCalled::has_traitor(g, p, defender_plan.hero))
                        || (pl.is(Faction::Harkonnen)
                            && pl.ally == aggressor_plan.initiator()
                            && !g.prevented(FactionAdvantage::HarkonnenCallTraitorForAlly)
                            && TreacheryCalled::has_traitor(g, p, defender_plan.hero))
                    {
                        return true;
                    }
                }

                if !aggressor_plan.messiah {
                    if (defender_plan.initiator() == pl.faction && TreacheryCalled::has_traitor(g, p, aggressor_plan.hero))
                        || (pl.is(Faction::Harkonnen)
                            && pl.ally == defender_plan.initiator()
                            && !g.prevented(FactionAdvantage::HarkonnenCallTraitorForAlly)
                            && TreacheryCalled::has_traitor(g, p, aggressor_plan.hero))
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// `h` is the opponent's hero, which can be null. Every `IsTraitor` implementation
    /// returns false for null.
    pub fn has_traitor(g: &Game, p: PlayerId, h: Option<Hero>) -> bool {
        let pl = g.player(p);
        pl.traitors.iter().any(|t| h.is_some_and(|h| t.is_traitor(h)))
            || (h.is_some() && g.applicable(Rule::CapturedLeadersAreTraitorsToOwnFaction) && Some(h.expect("hero").faction()) == Some(pl.faction))
    }
}

impl EventRules for TreacheryCalled {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !self.traitor_called {
            return Ok(());
        }

        if !TreacheryCalled::may_call_treachery(game, by.player()) {
            return Err(Rejection(express!("You cannot call TREACHERY")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let aggressor_plan = game.battle.aggressor_plan.clone().required("aggressor plan")?;
        if aggressor_plan.by(by.faction) || (by.is(Faction::Harkonnen) && game.are_allies(aggressor_plan.initiator(), Faction::Harkonnen)) {
            game.battle.aggressor_traitor_action = Some(TraitorCall { submitter: by, traitor_called: self.traitor_called });
            if self.traitor_called {
                let m = self.describe(game, by);
                game.log_event(m);
                game.stone(Milestone::TreacheryCalled);
                let hero = game.battle.defender_plan.as_ref().required("defender plan")?.hero;
                game.player_mut(by.player()).revealed_traitors.insert(hero.required("hero")?);
            }
        }

        let defender_plan = game.battle.defender_plan.clone().required("defender plan")?;
        if defender_plan.by(by.faction) || (by.is(Faction::Harkonnen) && game.are_allies(defender_plan.initiator(), Faction::Harkonnen)) {
            game.battle.defender_traitor_action = Some(TraitorCall { submitter: by, traitor_called: self.traitor_called });
            if self.traitor_called {
                let m = self.describe(game, by);
                game.log_event(m);
                game.stone(Milestone::TreacheryCalled);
                let hero = game.battle.aggressor_plan.as_ref().required("aggressor plan")?.hero;
                game.player_mut(by.player()).revealed_traitors.insert(hero.required("hero")?);
            }
        }

        if game.battle.aggressor_traitor_action.is_some() && game.battle.defender_traitor_action.is_some() {
            let treachery = game.battle.current_battle.clone().required("current battle")?.treachery_of_faction(game, Faction::Harkonnen);

            if game.applicable(Rule::NexusCards) && treachery.as_ref().is_some_and(|t| t.initiator() == Some(Faction::Harkonnen) && t.traitor_called) {
                game.enter(Phase::CancellingTraitor, true);
            } else {
                game.handle_revealed_battle_plans()?;
            }
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.traitor_called {
            return express!(by.faction, " call TREACHERY!");
        }
        express!(by.faction, " don't call treachery")
    }
}
