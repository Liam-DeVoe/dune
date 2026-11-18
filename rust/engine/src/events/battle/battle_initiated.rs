use crate::prelude::*;

/// A faction starts one of its battles.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct BattleInitiated {
    pub territory: Option<Territory>,
    pub target: Option<Faction>,
}

impl Default for BattleInitiated {
    fn default() -> Self {
        BattleInitiated { territory: Territory::from_id(0), target: Default::default() }
    }
}

impl BattleInitiated {
    /// The battle this event starts, when `by` submits it as the history entry `id`.
    pub fn battle(&self, by: By, id: EntryId) -> InitiatedBattle {
        InitiatedBattle { submitter: by, territory: self.territory, target: self.target, id }
    }

    pub fn is_aggressor_by_juice(g: &Game, f: Option<Faction>) -> bool {
        g.turn.current_juice.as_ref().is_some_and(|j| j.type_ == Some(JuiceType::Aggressor) && j.initiator() == f)
    }

    /// `IsInitiatorByJuice` with a possibly null `initiator` (as `Aggressor` passes `Player`).
    fn is_initiator_by_juice_opt(g: &Game, initiator: Option<PlayerId>, target: PlayerId) -> bool {
        g.turn.current_juice.as_ref().is_some_and(|j| {
            j.type_ == Some(JuiceType::GoFirst)
                && j.player_opt() == initiator
                && PlayerSequence::is_after(g, initiator.expect("initiator"), target)
        })
    }

    /// Whether the initiator played Juice to go last and comes after `target` in the storm
    /// order. The same check as `is_initiator_by_juice_opt`, for Go Last instead of Go First.
    fn is_target_by_juice_opt(g: &Game, initiator: Option<PlayerId>, target: PlayerId) -> bool {
        g.turn.current_juice.as_ref().is_some_and(|j| {
            j.type_ == Some(JuiceType::GoLast)
                && j.player_opt() == initiator
                && PlayerSequence::is_after(g, initiator.expect("initiator"), target)
        })
    }
}

impl InitiatedBattle {
    pub fn actual_initiator(&self, game: &Game) -> Option<Faction> {
        match game.fight().and_then(|c| c.ecaz_or_ally_fighter) {
            Some(fighter) if self.initiator() == game.get_ally(Some(fighter)) => Some(fighter),
            _ => self.initiator(),
        }
    }

    pub fn actual_target(&self, game: &Game) -> Option<Faction> {
        match game.fight().and_then(|c| c.ecaz_or_ally_fighter) {
            Some(fighter) if self.target == game.get_ally(Some(fighter)) => Some(fighter),
            _ => self.target,
        }
    }

    pub fn defender(&self, game: &Game) -> Option<Faction> {
        if self.actual_initiator(game) == self.aggressor(game) {
            self.actual_target(game)
        } else {
            self.actual_initiator(game)
        }
    }

    pub fn defending_player(&self, game: &Game) -> Option<PlayerId> {
        game.get_player(self.defender(game))
    }

    pub fn aggressor(&self, game: &Game) -> Option<Faction> {
        let target = self.actual_target(game);
        let target_player = game.get_player(target);

        let Some(target_player) = target_player else {
            return self.actual_initiator(game);
        };

        // `Player` is only dereferenced inside the Juice checks, after `CurrentJuice` is known
        // to be set; IsInitiatorByJuice/IsTargetByJuice compare it by reference first.
        if BattleInitiated::is_aggressor_by_juice(game, target)
            || BattleInitiated::is_initiator_by_juice_opt(game, self.player_opt(), target_player)
            || BattleInitiated::is_target_by_juice_opt(game, self.player_opt(), target_player)
        {
            return target;
        }
        self.actual_initiator(game)
    }

    pub fn aggressive_player(&self, game: &Game) -> Option<PlayerId> {
        game.get_player(self.aggressor(game))
    }

    pub fn is_involved(&self, game: &Game, p: PlayerId) -> bool {
        let pl = game.player(p);
        self.initiator() == pl.faction || self.target == pl.faction || self.initiator() == pl.ally || self.target == pl.ally
    }

    pub fn is_involved_faction(&self, game: &Game, f: Faction) -> bool {
        self.initiator() == Some(f)
            || self.target == Some(f)
            || game.player(self.player()).ally == Some(f)
            || game.get_ally(self.target) == Some(f)
    }

    pub fn is_aggressor_or_defender(&self, game: &Game, p: PlayerId) -> bool {
        let f = game.player(p).faction;
        self.aggressor(game) == f || self.defender(game) == f
    }

    pub fn is_aggressor_or_defender_faction(&self, game: &Game, f: Faction) -> bool {
        self.aggressor(game) == Some(f) || self.defender(game) == Some(f)
    }

    pub fn opponent_of(&self, game: &Game, p: PlayerId) -> Option<PlayerId> {
        let pl = game.player(p);
        if pl.faction == self.initiator() || pl.ally == self.initiator() {
            return game.get_player(self.actual_target(game));
        }

        if pl.faction == self.target || pl.ally == self.target {
            return game.get_player(self.actual_initiator(game));
        }

        None
    }

    pub fn opponent_of_faction(&self, game: &Game, f: impl Into<Option<Faction>>) -> Option<PlayerId> {
        let f: Option<Faction> = f.into();
        let player = game.get_player(f);
        match player {
            None => None,
            Some(player) => self.opponent_of(game, player),
        }
    }

    pub fn plan_of(&self, game: &Game, p: Option<PlayerId>) -> Option<BattlePlan> {
        let Some(p) = p else { return None };
        let f = game.player(p).faction;

        if f == self.aggressor(game) {
            return game.aggressor_plan().cloned();
        }
        if f == self.defender(game) { game.defender_plan().cloned() } else { None }
    }

    pub fn plan_of_opponent(&self, game: &Game, p: Option<PlayerId>) -> Option<BattlePlan> {
        match p {
            None => None,
            Some(p) => self.plan_of(game, self.opponent_of(game, p)),
        }
    }

    pub fn plan_of_opponent_faction(&self, game: &Game, f: Faction) -> Option<BattlePlan> {
        self.plan_of(game, self.opponent_of_faction(game, f))
    }

    pub fn plan_of_faction(&self, game: &Game, f: impl Into<Option<Faction>>) -> Option<BattlePlan> {
        let f: Option<Faction> = f.into();
        if f == self.aggressor(game) {
            return game.aggressor_plan().cloned();
        }

        if f == self.defender(game) { game.defender_plan().cloned() } else { None }
    }

    pub fn previous_plan_of(&self, game: &Game, f: Faction) -> Option<BattlePlan> {
        if Some(f) == self.aggressor(game) {
            return game.fight().and_then(|c| c.previous_aggressor_plan.clone());
        }

        if Some(f) == self.defender(game) { game.fight().and_then(|c| c.previous_defender_plan.clone()) } else { None }
    }

    pub fn treachery_of_opponent(&self, game: &Game, p: PlayerId) -> Option<TraitorCall> {
        self.treachery_of(game, self.opponent_of(game, p))
    }

    pub fn treachery_of(&self, game: &Game, p: Option<PlayerId>) -> Option<TraitorCall> {
        let Some(p) = p else { return None };
        let pl = game.player(p);

        if let Some(a) = game.fight().and_then(|c| c.aggressor_traitor_action.as_ref()) {
            if a.initiator() == pl.faction || a.initiator() == pl.ally {
                return Some(a.clone());
            }
        }
        if let Some(d) = game.fight().and_then(|c| c.defender_traitor_action.as_ref()) {
            if d.initiator() == pl.faction || d.initiator() == pl.ally {
                return Some(d.clone());
            }
        }
        None
    }

    pub fn treachery_of_faction(&self, game: &Game, f: Faction) -> Option<TraitorCall> {
        self.treachery_of(game, game.get_player(Some(f)))
    }
}

impl EventRules for BattleInitiated {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        let Some(territory) = self.territory else {
            return Err(Rejection(express!("Territory not selected")));
        };

        if !game.player(by.player()).occupies_territory(game, Some(territory)) {
            return Err(Rejection(express!("You have no forces in this territory")));
        }
        if !Battle::battles_to_be_fought(game, by.player(), false)
            .iter()
            .any(|f| Some(f.faction) == self.target && Some(f.territory) == self.territory)
        {
            return Err(Rejection(express!("Invalid battle selected")));
        }

        let opponent = game.get_player(self.battle(by, EntryId::default()).defender(game));
        let Some(opponent) = opponent else {
            return Err(Rejection(express!("Opponent not selected")));
        };
        if !game.player(opponent).occupies_territory(game, Some(territory)) {
            return Err(Rejection(express!("Opponent has no forces in this territory")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let battle = self.battle(by, game.history.playing);
        // Not logged: the report panel shows who attacks whom where in the battle's heading.
        // Aggressor, Defender and Territory only read state that StartReport doesn't touch.
        let aggressor = battle.aggressor(game);
        let defender = battle.defender(game);
        let territory = self.territory;
        let report = game.start_report(MainPhase::Battle);
        report.aggressor = aggressor;
        report.defender = defender;
        report.territory = territory;
        game.battle_mut().required("battle phase")?.current = Some(CurrentBattle::new(battle.clone()));

        let ecaz = game.get_player(Some(Faction::Ecaz));
        let condition = match ecaz {
            Some(ecaz) => {
                battle.is_involved_faction(game, Faction::Ecaz)
                    && territory.is_some()
                    && game.player(ecaz).occupies_territory(game, territory)
                    && game.player(ecaz).has_ally()
                    && game.player(game.player(ecaz).allied_player(game).required("allied player")?).occupies_territory(game, territory)
            }
            None => false,
        };
        if condition {
            game.enter_battle(BattleStep::Claiming);
        } else {
            game.initiate_battle();
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        let territory = self.territory;
        express!(
            by.faction,
            " initiate battle against ",
            self.target,
            if territory.is_some_and(|t| t.is_homeworld()) { " on " } else { " in " },
            territory
        )
    }
}
