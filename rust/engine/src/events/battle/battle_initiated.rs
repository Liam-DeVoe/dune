use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct BattleInitiated {
    #[canon(name = "_territoryId")]
    pub territory_id: i32,
    pub target: Faction,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for BattleInitiated {
    fn default() -> Self {
        BattleInitiated {
            territory_id: Default::default(),
            target: Default::default(),
            base: Default::default(),
        }
    }
}

impl BattleInitiated {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn territory(&self, game: &Game) -> Option<Territory> {
        game.map.territory_lookup().find(self.territory_id)
    }

    pub fn set_territory(&mut self, game: &Game, value: Option<Territory>) {
        self.territory_id = game.map.territory_lookup().get_id(value);
    }

    pub fn actual_initiator(&self, game: &Game) -> Faction {
        match game.current_ecaz_or_ally_fighter {
            Some(fighter) if fighter != Faction::None && Some(self.initiator()) == game.get_ally(Some(fighter)) => fighter,
            _ => self.initiator(),
        }
    }

    pub fn actual_target(&self, game: &Game) -> Faction {
        match game.current_ecaz_or_ally_fighter {
            Some(fighter) if fighter != Faction::None && Some(self.target) == game.get_ally(Some(fighter)) => fighter,
            _ => self.target,
        }
    }

    pub fn defender(&self, game: &Game) -> Faction {
        if self.actual_initiator(game) == self.aggressor(game) {
            self.actual_target(game)
        } else {
            self.actual_initiator(game)
        }
    }

    pub fn defending_player(&self, game: &Game) -> Option<PlayerIndex> {
        game.get_player(Some(self.defender(game)))
    }

    pub fn aggressor(&self, game: &Game) -> Faction {
        let target = self.actual_target(game);
        let target_player = game.get_player(Some(target));

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

    pub fn aggressive_player(&self, game: &Game) -> Option<PlayerIndex> {
        game.get_player(Some(self.aggressor(game)))
    }

    pub fn is_aggressor_by_juice(g: &Game, f: Faction) -> bool {
        g.current_juice.as_ref().is_some_and(|j| j.type_ == JuiceType::Aggressor && j.initiator() == f)
    }

    pub fn is_initiator_by_juice(g: &Game, initiator: PlayerIndex, target: PlayerIndex) -> bool {
        BattleInitiated::is_initiator_by_juice_opt(g, Some(initiator), target)
    }

    /// `IsInitiatorByJuice` with a possibly null `initiator` (as `Aggressor` passes `Player`).
    fn is_initiator_by_juice_opt(g: &Game, initiator: Option<PlayerIndex>, target: PlayerIndex) -> bool {
        g.current_juice.as_ref().is_some_and(|j| {
            j.type_ == JuiceType::GoFirst
                && j.player_opt() == initiator
                && PlayerSequence::is_after(g, initiator.expect("initiator"), target)
        })
    }

    pub fn is_target_by_juice(g: &Game, initiator: PlayerIndex, target: PlayerIndex) -> bool {
        BattleInitiated::is_target_by_juice_opt(g, Some(initiator), target)
    }

    /// `IsTargetByJuice` with a possibly null `initiator`.
    ///
    /// C# parity: the same predicate as IsInitiatorByJuice apart from the Juice type (hazards 18.2).
    fn is_target_by_juice_opt(g: &Game, initiator: Option<PlayerIndex>, target: PlayerIndex) -> bool {
        g.current_juice.as_ref().is_some_and(|j| {
            j.type_ == JuiceType::GoLast
                && j.player_opt() == initiator
                && PlayerSequence::is_after(g, initiator.expect("initiator"), target)
        })
    }

    pub fn is_involved(&self, game: &Game, p: PlayerIndex) -> bool {
        let pl = game.player(p);
        self.initiator() == pl.faction || self.target == pl.faction || self.initiator() == pl.ally || self.target == pl.ally
    }

    pub fn is_involved_faction(&self, game: &Game, f: Faction) -> bool {
        self.initiator() == f
            || self.target == f
            || game.player(self.player()).ally == f
            || game.get_ally(Some(self.target)) == Some(f)
    }

    pub fn is_aggressor_or_defender(&self, game: &Game, p: PlayerIndex) -> bool {
        let f = game.player(p).faction;
        self.aggressor(game) == f || self.defender(game) == f
    }

    pub fn is_aggressor_or_defender_faction(&self, game: &Game, f: Faction) -> bool {
        self.aggressor(game) == f || self.defender(game) == f
    }

    pub fn opponent_of(&self, game: &Game, p: PlayerIndex) -> Option<PlayerIndex> {
        let pl = game.player(p);
        if pl.faction == self.initiator() || pl.ally == self.initiator() {
            return game.get_player(Some(self.actual_target(game)));
        }

        if pl.faction == self.target || pl.ally == self.target {
            return game.get_player(Some(self.actual_initiator(game)));
        }

        None
    }

    pub fn opponent_of_faction(&self, game: &Game, f: Faction) -> Option<PlayerIndex> {
        let player = game.get_player(Some(f));
        match player {
            None => None,
            Some(player) => self.opponent_of(game, player),
        }
    }

    pub fn plan_of(&self, game: &Game, p: Option<PlayerIndex>) -> Option<Rc<Battle>> {
        let Some(p) = p else { return None };
        let f = game.player(p).faction;

        if f == self.aggressor(game) {
            return game.aggressor_plan.clone();
        }
        if f == self.defender(game) { game.defender_plan.clone() } else { None }
    }

    pub fn plan_of_opponent(&self, game: &Game, p: Option<PlayerIndex>) -> Option<Rc<Battle>> {
        match p {
            None => None,
            Some(p) => self.plan_of(game, self.opponent_of(game, p)),
        }
    }

    pub fn plan_of_opponent_faction(&self, game: &Game, f: Faction) -> Option<Rc<Battle>> {
        self.plan_of(game, self.opponent_of_faction(game, f))
    }

    pub fn plan_of_faction(&self, game: &Game, f: Faction) -> Option<Rc<Battle>> {
        if f == self.aggressor(game) {
            return game.aggressor_plan.clone();
        }

        if f == self.defender(game) { game.defender_plan.clone() } else { None }
    }

    pub fn previous_plan_of(&self, game: &Game, f: Faction) -> Option<Rc<Battle>> {
        if f == self.aggressor(game) {
            return game.previous_aggressor_plan.clone();
        }

        if f == self.defender(game) { game.previous_defender_plan.clone() } else { None }
    }

    pub fn treachery_of_opponent(&self, game: &Game, p: PlayerIndex) -> Option<Rc<TreacheryCalled>> {
        self.treachery_of(game, self.opponent_of(game, p))
    }

    pub fn treachery_of(&self, game: &Game, p: Option<PlayerIndex>) -> Option<Rc<TreacheryCalled>> {
        let Some(p) = p else { return None };
        let pl = game.player(p);

        if let Some(a) = &game.aggressor_traitor_action {
            if a.initiator() == pl.faction || a.initiator() == pl.ally {
                return Some(a.clone());
            }
        }
        if let Some(d) = &game.defender_traitor_action {
            if d.initiator() == pl.faction || d.initiator() == pl.ally {
                return Some(d.clone());
            }
        }
        None
    }

    pub fn treachery_of_faction(&self, game: &Game, f: Faction) -> Option<Rc<TreacheryCalled>> {
        self.treachery_of(game, game.get_player(Some(f)))
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let Some(territory) = self.territory(game) else {
            return Some(express!("Territory not selected"));
        };

        if !game.player(self.player()).occupies_territory(game, Some(territory)) {
            return Some(express!("You have no forces in this territory"));
        }
        if !Battle::battles_to_be_fought(game, self.player(), false)
            .iter()
            .any(|f| f.faction == self.target && f.territory.id() == self.territory_id)
        {
            return Some(express!("Invalid battle selected"));
        }

        let opponent = game.get_player(Some(self.defender(game)));
        let Some(opponent) = opponent else {
            return Some(express!("Opponent not selected"));
        };
        if !game.player(opponent).occupies_territory(game, Some(territory)) {
            return Some(express!("Opponent has no forces in this territory"));
        }

        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        // Not logged: the report panel shows who attacks whom where in the battle's heading.
        // Aggressor, Defender and Territory only read state that StartReport doesn't touch.
        let aggressor = self.aggressor(game);
        let defender = self.defender(game);
        let territory = self.territory(game);
        let report = game.start_report(MainPhase::Battle);
        report.aggressor = aggressor;
        report.defender = defender;
        report.territory = territory;
        game.battle_about_to_start = Some(self.clone());

        let ecaz = self.get_player(game, Some(Faction::Ecaz));
        let condition = match ecaz {
            Some(ecaz) => {
                self.is_involved_faction(game, Faction::Ecaz)
                    && territory.is_some()
                    && game.player(ecaz).occupies_territory(game, territory)
                    && game.player(ecaz).has_ally()
                    && game.player(game.player(ecaz).allied_player(game).required("allied player")?).occupies_territory(game, territory)
            }
            None => false,
        };
        game.enter_boolean_phase_action(condition, Phase::ClaimingBattle, Game::initiate_battle);
        Ok(())
    }

    pub fn get_message(&self, game: &Game) -> Message {
        let territory = self.territory(game);
        express!(
            self.initiator(),
            " initiate battle against ",
            self.target,
            if territory.is_some_and(|t| t.is_homeworld()) { " on " } else { " in " },
            territory
        )
    }

}

impl GameEventMethods for BattleInitiated {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::BattleInitiated
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::BattleInitiated(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        BattleInitiated::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        BattleInitiated::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        BattleInitiated::get_message(self, game)
    }

}
