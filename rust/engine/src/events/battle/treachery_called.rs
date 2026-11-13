use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct TreacheryCalled {
    pub traitor_called: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for TreacheryCalled {
    fn default() -> Self {
        TreacheryCalled {
            traitor_called: Default::default(),
            base: Default::default(),
        }
    }
}

impl TreacheryCalled {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn succeeded(&self, game: &Game) -> bool {
        self.traitor_called && (self.initiator() != Faction::Harkonnen || !game.harkonnen_traitor_was_cancelled)
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !self.traitor_called {
            return None;
        }

        if !TreacheryCalled::may_call_treachery(game, self.player()) {
            return Some(express!("You cannot call TREACHERY"));
        }

        None
    }

    pub fn may_call_treachery(g: &Game, p: PlayerIndex) -> bool {
        if let (Some(aggressor_plan), Some(defender_plan)) = (&g.aggressor_plan, &g.defender_plan) {
            let territory = g.current_battle.as_ref().nre().territory(g).nre();
            let pl = g.player(p);
            if !territory.is_homeworld() || pl.is_native(territory) {
                if !defender_plan.messiah {
                    if (aggressor_plan.initiator() == pl.faction && TreacheryCalled::has_traitor(g, p, defender_plan.hero()))
                        || (pl.is(Faction::Harkonnen)
                            && pl.ally == aggressor_plan.initiator()
                            && !g.prevented(FactionAdvantage::HarkonnenCallTraitorForAlly)
                            && TreacheryCalled::has_traitor(g, p, defender_plan.hero()))
                    {
                        return true;
                    }
                }

                if !aggressor_plan.messiah {
                    if (defender_plan.initiator() == pl.faction && TreacheryCalled::has_traitor(g, p, aggressor_plan.hero()))
                        || (pl.is(Faction::Harkonnen)
                            && pl.ally == defender_plan.initiator()
                            && !g.prevented(FactionAdvantage::HarkonnenCallTraitorForAlly)
                            && TreacheryCalled::has_traitor(g, p, aggressor_plan.hero()))
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
    pub fn has_traitor(g: &Game, p: PlayerIndex, h: Option<Hero>) -> bool {
        let h = h.and_then(crate::compat::cs::CsNull::opt);
        let pl = g.player(p);
        pl.traitors.iter().any(|t| h.is_some_and(|h| t.is_traitor(h)))
            || (h.is_some() && g.applicable(Rule::CapturedLeadersAreTraitorsToOwnFaction) && h.nre().faction() == pl.faction)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        let aggressor_plan = game.aggressor_plan.clone().nre();
        if aggressor_plan.by(self.initiator()) || (self.by(Faction::Harkonnen) && game.are_allies(aggressor_plan.initiator(), Faction::Harkonnen)) {
            game.aggressor_traitor_action = Some(self.clone());
            if self.traitor_called {
                self.log(game);
                game.stone(Milestone::TreacheryCalled);
                let hero = game.defender_plan.as_ref().nre().hero();
                // C# adds the defender's hero, which can be null.
                game.player_mut(self.player()).revealed_traitors.add(hero.or_null());
            }
        }

        let defender_plan = game.defender_plan.clone().nre();
        if defender_plan.by(self.initiator()) || (self.by(Faction::Harkonnen) && game.are_allies(defender_plan.initiator(), Faction::Harkonnen)) {
            game.defender_traitor_action = Some(self.clone());
            if self.traitor_called {
                self.log(game);
                game.stone(Milestone::TreacheryCalled);
                let hero = game.aggressor_plan.as_ref().nre().hero();
                game.player_mut(self.player()).revealed_traitors.add(hero.or_null());
            }
        }

        if game.aggressor_traitor_action.is_some() && game.defender_traitor_action.is_some() {
            let treachery = game.current_battle.clone().nre().treachery_of_faction(game, Faction::Harkonnen);

            if game.applicable(Rule::NexusCards) && treachery.as_ref().is_some_and(|t| t.initiator() == Faction::Harkonnen && t.traitor_called) {
                game.enter(Phase::CancellingTraitor, true);
            } else {
                game.handle_revealed_battle_plans();
            }
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.traitor_called {
            return express!(self.initiator(), " call TREACHERY!");
        }
        express!(self.initiator(), " don't call treachery")
    }

}

impl GameEventMethods for TreacheryCalled {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::TreacheryCalled
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::TreacheryCalled(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        TreacheryCalled::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        TreacheryCalled::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        TreacheryCalled::get_message(self, game)
    }

}
