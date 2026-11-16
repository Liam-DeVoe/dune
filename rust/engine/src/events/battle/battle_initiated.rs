use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct BattleInitiated {
    pub territory: Option<Territory>,
    pub target: Option<Faction>,
    /// The fields of `Event` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for BattleInitiated {
    fn default() -> Self {
        BattleInitiated {
            territory: Territory::from_id(0),
            target: Default::default(),
            base: Default::default(),
        }
    }
}

impl BattleInitiated {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn actual_initiator(&self, game: &Game) -> Option<Faction> {
        match game.battle.current_ecaz_or_ally_fighter.flatten() {
            Some(fighter) if self.initiator() == game.get_ally(Some(fighter)) => Some(fighter),
            _ => self.initiator(),
        }
    }

    pub fn actual_target(&self, game: &Game) -> Option<Faction> {
        match game.battle.current_ecaz_or_ally_fighter.flatten() {
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

    /// `IsTargetByJuice` with a possibly null `initiator`.
    ///
    /// C# parity: the same predicate as IsInitiatorByJuice apart from the Juice type.
    fn is_target_by_juice_opt(g: &Game, initiator: Option<PlayerId>, target: PlayerId) -> bool {
        g.turn.current_juice.as_ref().is_some_and(|j| {
            j.type_ == Some(JuiceType::GoLast)
                && j.player_opt() == initiator
                && PlayerSequence::is_after(g, initiator.expect("initiator"), target)
        })
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

    pub fn plan_of(&self, game: &Game, p: Option<PlayerId>) -> Option<Rc<Battle>> {
        let Some(p) = p else { return None };
        let f = game.player(p).faction;

        if f == self.aggressor(game) {
            return game.battle.aggressor_plan.clone();
        }
        if f == self.defender(game) { game.battle.defender_plan.clone() } else { None }
    }

    pub fn plan_of_opponent(&self, game: &Game, p: Option<PlayerId>) -> Option<Rc<Battle>> {
        match p {
            None => None,
            Some(p) => self.plan_of(game, self.opponent_of(game, p)),
        }
    }

    pub fn plan_of_opponent_faction(&self, game: &Game, f: Faction) -> Option<Rc<Battle>> {
        self.plan_of(game, self.opponent_of_faction(game, f))
    }

    pub fn plan_of_faction(&self, game: &Game, f: impl Into<Option<Faction>>) -> Option<Rc<Battle>> {
        let f: Option<Faction> = f.into();
        if f == self.aggressor(game) {
            return game.battle.aggressor_plan.clone();
        }

        if f == self.defender(game) { game.battle.defender_plan.clone() } else { None }
    }

    pub fn previous_plan_of(&self, game: &Game, f: Faction) -> Option<Rc<Battle>> {
        if Some(f) == self.aggressor(game) {
            return game.battle.previous_aggressor_plan.clone();
        }

        if Some(f) == self.defender(game) { game.battle.previous_defender_plan.clone() } else { None }
    }

    pub fn treachery_of_opponent(&self, game: &Game, p: PlayerId) -> Option<Rc<TreacheryCalled>> {
        self.treachery_of(game, self.opponent_of(game, p))
    }

    pub fn treachery_of(&self, game: &Game, p: Option<PlayerId>) -> Option<Rc<TreacheryCalled>> {
        let Some(p) = p else { return None };
        let pl = game.player(p);

        if let Some(a) = &game.battle.aggressor_traitor_action {
            if a.initiator() == pl.faction || a.initiator() == pl.ally {
                return Some(a.clone());
            }
        }
        if let Some(d) = &game.battle.defender_traitor_action {
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
        let Some(territory) = self.territory else {
            return Some(express!("Territory not selected"));
        };

        if !game.player(self.player()).occupies_territory(game, Some(territory)) {
            return Some(express!("You have no forces in this territory"));
        }
        if !Battle::battles_to_be_fought(game, self.player(), false)
            .iter()
            .any(|f| Some(f.faction) == self.target && Some(f.territory) == self.territory)
        {
            return Some(express!("Invalid battle selected"));
        }

        let opponent = game.get_player(self.defender(game));
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
        let territory = self.territory;
        let report = game.start_report(MainPhase::Battle);
        report.aggressor = aggressor;
        report.defender = defender;
        report.territory = territory;
        game.battle.battle_about_to_start = Some(self.clone());

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
        game.enter_or_else(condition, Phase::ClaimingBattle, Game::initiate_battle);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        let territory = self.territory;
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

    fn event_type(&self) -> EventType {
        EventType::BattleInitiated
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::BattleInitiated(self)
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
