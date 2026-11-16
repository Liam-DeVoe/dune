use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Retreat {
    pub forces: u16,
    pub special_forces: u16,
    pub location: Option<Location>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for Retreat {
    fn default() -> Self {
        Retreat {
            forces: Default::default(),
            special_forces: Default::default(),
            location: Location::from_id(0),
            base: Default::default(),
        }
    }
}

impl Retreat {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let p = self.player();
        let mut valid_targets = Retreat::valid_targets(game, p);
        let location = self.location;
        if !valid_targets.any(|v| Some(v) == location) {
            return Some(express!("Invalid location"));
        }
        if i32::from(self.forces) > Retreat::max_forces(game, p) {
            return Some(express!("You selected too many ", game.player(p).force()));
        }
        if i32::from(self.special_forces) > Retreat::max_special_forces(game, p) {
            return Some(express!("You selected too many ", game.player(p).special_force()));
        }
        if (i32::from(self.forces) + i32::from(self.special_forces)) > Retreat::max_total_forces(game, p) {
            return Some(express!("You selected too many forces"));
        }
        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Location> {
        let territory = g.battle.current_battle.as_ref().expect("current battle").territory.expect("territory");
        let battalions = g.player(p).battalions_in(territory);
        PlacementEvent::valid_targets_for_forces(g, p, &battalions)
            .into_iter()
            .filter(move |t| !g.any_forces_in(t.territory()) && !t.is_stronghold())
    }

    pub fn max_total_forces(g: &Game, p: PlayerId) -> i32 {
        let current_battle = g.battle.current_battle.as_ref().expect("current battle");
        let plan = current_battle.plan_of(g, Some(p));
        let opponent_plan = current_battle.plan_of_opponent(g, Some(p));
        // plan.Hero.ValueInCombatAgainst(opponentPlan.Hero): NullReferenceException when
        // either plan or the hero is null.
        let hero = plan.expect("plan").hero;
        let opponent_hero = opponent_plan.expect("opponent plan").hero;
        hero.expect("hero").value_in_combat_against(opponent_hero)
    }

    pub fn max_forces(g: &Game, p: PlayerId) -> i32 {
        let current_battle = g.battle.current_battle.as_ref().expect("current battle");
        let plan = current_battle.plan_of(g, Some(p));
        let forces_in = g.player(p).forces_in_territory(current_battle.territory);
        let plan = plan.expect("plan");
        (forces_in - i32::from(plan.forces)) - i32::from(plan.forces_at_half_strength)
    }

    pub fn max_special_forces(g: &Game, p: PlayerId) -> i32 {
        let current_battle = g.battle.current_battle.as_ref().expect("current battle");
        let plan = current_battle.plan_of(g, Some(p));
        let special_forces_in = g.player(p).special_forces_in_territory(current_battle.territory);
        let plan = plan.expect("plan");
        (special_forces_in - i32::from(plan.special_forces)) - i32::from(plan.special_forces_at_half_strength)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let me = self.player();
        let mut forces_to_move = i32::from(self.forces);
        let territory = game.battle.current_battle.as_ref().required("current battle")?.territory.required("territory")?;
        let sources: Vec<Location> = territory.locations().iter().copied().filter(|l| game.player(me).forces_in(Some(*l)) > 0).collect();
        for l in sources {
            if forces_to_move == 0 {
                break;
            }
            let to_move_from_here = std::cmp::min(forces_to_move, game.player(me).forces_in(Some(l)));
            let to = self.location.required("location")?;
            Player::move_forces(game, me, l, to, to_move_from_here)?;
            forces_to_move -= to_move_from_here;
        }

        let mut special_forces_to_move = i32::from(self.special_forces);
        let territory = game.battle.current_battle.as_ref().required("current battle")?.territory.required("territory")?;
        let sources: Vec<Location> =
            territory.locations().iter().copied().filter(|l| game.player(me).special_forces_in(Some(*l)) > 0).collect();
        for l in sources {
            if special_forces_to_move == 0 {
                break;
            }
            let to_move_from_here = std::cmp::min(special_forces_to_move, game.player(me).special_forces_in(Some(l)));
            let to = self.location.required("location")?;
            Player::move_special_forces(game, me, l, to, to_move_from_here)?;
            special_forces_to_move -= to_move_from_here;
        }

        self.log(game);
        game.handle_losses()?;
        game.flip_bene_gesserit_advisors_when_alone()?;
        game.determine_how_to_proceed_after_revealing_battle_plans()?;
        Ok(())
    }

    pub fn get_message(&self, game: &Game) -> Message {
        if i32::from(self.forces) > 0 || i32::from(self.special_forces) > 0 {
            let force = game.player(self.player()).force();
            let special_force = game.player(self.player()).special_force();
            return express!(
                self.initiator(),
                " retreat ",
                express_part_if!(i32::from(self.forces) > 0; i32::from(self.forces), " ", force),
                express_part_if!(i32::from(self.special_forces) > 0; i32::from(self.special_forces), " ", special_force),
                " to ",
                self.location
            );
        }
        express!(self.initiator(), " don't retreat")
    }

}

impl GameEventMethods for Retreat {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::Retreat
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Retreat(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Retreat::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        Retreat::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Retreat::get_message(self, game)
    }

}
