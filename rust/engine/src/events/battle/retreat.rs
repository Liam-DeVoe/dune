use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Retreat {
    pub forces: u16,
    pub special_forces: u16,
    pub location: Option<Location>,
}

impl Default for Retreat {
    fn default() -> Self {
        Retreat {
            forces: Default::default(),
            special_forces: Default::default(),
            location: Location::from_id(0),
        }
    }
}

impl Retreat {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Location> {
        let territory = g.current_battle().expect("current battle").territory.expect("territory");
        let battalions = g.player(p).battalions_in(territory);
        PlacementEvent::valid_targets_for_forces(g, p, &battalions)
            .into_iter()
            .filter(move |t| !g.any_forces_in(t.territory()) && !t.is_stronghold())
    }

    pub fn max_total_forces(g: &Game, p: PlayerId) -> i32 {
        let current_battle = g.current_battle().expect("current battle");
        let plan = current_battle.plan_of(g, Some(p));
        let opponent_plan = current_battle.plan_of_opponent(g, Some(p));
        // plan.Hero.ValueInCombatAgainst(opponentPlan.Hero): NullReferenceException when
        // either plan or the hero is null.
        let hero = plan.expect("plan").hero;
        let opponent_hero = opponent_plan.expect("opponent plan").hero;
        hero.expect("hero").value_in_combat_against(opponent_hero)
    }

    pub fn max_forces(g: &Game, p: PlayerId) -> i32 {
        let current_battle = g.current_battle().expect("current battle");
        let plan = current_battle.plan_of(g, Some(p));
        let forces_in = g.player(p).forces_in_territory(current_battle.territory);
        let plan = plan.expect("plan");
        (forces_in - i32::from(plan.forces)) - i32::from(plan.forces_at_half_strength)
    }

    pub fn max_special_forces(g: &Game, p: PlayerId) -> i32 {
        let current_battle = g.current_battle().expect("current battle");
        let plan = current_battle.plan_of(g, Some(p));
        let special_forces_in = g.player(p).special_forces_in_territory(current_battle.territory);
        let plan = plan.expect("plan");
        (special_forces_in - i32::from(plan.special_forces)) - i32::from(plan.special_forces_at_half_strength)
    }
}

impl EventRules for Retreat {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        let p = by.player();
        let mut valid_targets = Retreat::valid_targets(game, p);
        let location = self.location;
        if !valid_targets.any(|v| Some(v) == location) {
            return Err(Rejection(express!("Invalid location")));
        }
        if i32::from(self.forces) > Retreat::max_forces(game, p) {
            return Err(Rejection(express!("You selected too many ", game.player(p).force())));
        }
        if i32::from(self.special_forces) > Retreat::max_special_forces(game, p) {
            return Err(Rejection(express!("You selected too many ", game.player(p).special_force())));
        }
        if (i32::from(self.forces) + i32::from(self.special_forces)) > Retreat::max_total_forces(game, p) {
            return Err(Rejection(express!("You selected too many forces")));
        }
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let me = by.player();
        let mut forces_to_move = i32::from(self.forces);
        let territory = game.current_battle().required("current battle")?.territory.required("territory")?;
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
        let territory = game.current_battle().required("current battle")?.territory.required("territory")?;
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

        let m = self.describe(game, by);
        game.log_event(m);
        game.handle_losses()?;
        game.flip_bene_gesserit_advisors_when_alone()?;
        game.determine_how_to_proceed_after_revealing_battle_plans()?;
        Ok(())
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        if i32::from(self.forces) > 0 || i32::from(self.special_forces) > 0 {
            let force = game.player(by.player()).force();
            let special_force = game.player(by.player()).special_force();
            return express!(
                by.faction,
                " retreat ",
                express_part_if!(i32::from(self.forces) > 0; i32::from(self.forces), " ", force),
                express_part_if!(i32::from(self.special_forces) > 0; i32::from(self.special_forces), " ", special_force),
                " to ",
                self.location
            );
        }
        express!(by.faction, " don't retreat")
    }
}
