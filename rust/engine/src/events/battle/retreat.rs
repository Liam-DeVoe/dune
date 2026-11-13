use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct Retreat {
    pub forces: i32,
    pub special_forces: i32,
    #[canon(name = "_targetId")]
    pub target_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for Retreat {
    fn default() -> Self {
        Retreat {
            forces: Default::default(),
            special_forces: Default::default(),
            target_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl Retreat {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn location(&self, game: &Game) -> Option<Location> {
        game.map.location_lookup().find(self.target_id)
    }

    pub fn set_location(&mut self, game: &Game, value: Option<Location>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.target_id = game.map.location_lookup().get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let p = self.player();
        let mut valid_targets = Retreat::valid_targets(game, p);
        let location = self.location(game);
        if !valid_targets.any(|v| v == location.or_null()) {
            return Some(express!("Invalid location"));
        }
        if self.forces > Retreat::max_forces(game, p) {
            return Some(express!("You selected too many ", game.player(p).force()));
        }
        if self.special_forces > Retreat::max_special_forces(game, p) {
            return Some(express!("You selected too many ", game.player(p).special_force()));
        }
        if self.forces.wrapping_add(self.special_forces) > Retreat::max_total_forces(game, p) {
            return Some(express!("You selected too many forces"));
        }
        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_targets(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Location> {
        let territory = g.current_battle.as_ref().nre().territory(g).nre();
        let battalions = g.player(p).battalions_in(territory);
        PlacementEvent::valid_targets_game_player_dictionary_location_battalion(g, p, &battalions)
            .into_iter()
            .filter(move |t| !g.any_forces_in(t.territory()) && !t.is_stronghold())
    }

    pub fn max_total_forces(g: &Game, p: PlayerIndex) -> i32 {
        let current_battle = g.current_battle.as_ref().nre();
        let plan = current_battle.plan_of(g, Some(p));
        let opponent_plan = current_battle.plan_of_opponent(g, Some(p));
        // plan.Hero.ValueInCombatAgainst(opponentPlan.Hero): NullReferenceException when
        // either plan or the hero is null (hazards 1.4).
        let hero = plan.nre().hero();
        let opponent_hero = opponent_plan.nre().hero();
        hero.nre().value_in_combat_against(opponent_hero)
    }

    pub fn max_forces(g: &Game, p: PlayerIndex) -> i32 {
        let current_battle = g.current_battle.as_ref().nre();
        let plan = current_battle.plan_of(g, Some(p));
        let forces_in = g.player(p).forces_in_territory(current_battle.territory(g));
        let plan = plan.nre();
        forces_in.wrapping_sub(plan.forces).wrapping_sub(plan.forces_at_half_strength)
    }

    pub fn max_special_forces(g: &Game, p: PlayerIndex) -> i32 {
        let current_battle = g.current_battle.as_ref().nre();
        let plan = current_battle.plan_of(g, Some(p));
        let special_forces_in = g.player(p).special_forces_in_territory(current_battle.territory(g));
        let plan = plan.nre();
        special_forces_in.wrapping_sub(plan.special_forces).wrapping_sub(plan.special_forces_at_half_strength)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let me = self.player();
        let mut forces_to_move = self.forces;
        let territory = game.current_battle.as_ref().required("current battle")?.territory(game).required("territory")?;
        let sources: Vec<Location> = territory.locations().iter().copied().filter(|l| game.player(me).forces_in(Some(*l)) > 0).collect();
        for l in sources {
            if forces_to_move == 0 {
                break;
            }
            let to_move_from_here = std::cmp::min(forces_to_move, game.player(me).forces_in(Some(l)));
            // A null Location (only without validation): MoveForces empties `l`, then throws
            // ArgumentNullException on the null key.
            let to = self.location(game).or_null();
            Player::move_forces(game, me, l, to, to_move_from_here);
            forces_to_move = forces_to_move.wrapping_sub(to_move_from_here);
        }

        let mut special_forces_to_move = self.special_forces;
        let territory = game.current_battle.as_ref().required("current battle")?.territory(game).required("territory")?;
        let sources: Vec<Location> =
            territory.locations().iter().copied().filter(|l| game.player(me).special_forces_in(Some(*l)) > 0).collect();
        for l in sources {
            if special_forces_to_move == 0 {
                break;
            }
            let to_move_from_here = std::cmp::min(special_forces_to_move, game.player(me).special_forces_in(Some(l)));
            let to = self.location(game).or_null();
            Player::move_special_forces(game, me, l, to, to_move_from_here);
            special_forces_to_move = special_forces_to_move.wrapping_sub(to_move_from_here);
        }

        self.log(game);
        game.handle_losses()?;
        game.flip_bene_gesserit_advisors_when_alone();
        game.determine_how_to_proceed_after_revealing_battle_plans()?;
        Ok(())
    }

    pub fn get_message(&self, game: &Game) -> Message {
        if self.forces > 0 || self.special_forces > 0 {
            let force = game.player(self.player()).force();
            let special_force = game.player(self.player()).special_force();
            return express!(
                self.initiator(),
                " retreat ",
                express_part_if!(self.forces > 0; self.forces, " ", force),
                express_part_if!(self.special_forces > 0; self.special_forces, " ", special_force),
                " to ",
                self.location(game)
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
