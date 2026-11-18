use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct FaceDanced {
    #[serde(with = "crate::events::wire::force_list")]
    #[ts(as = "Option<Vec<crate::events::wire::ForceEntry>>")]
    pub target_force_locations: Option<String>,
    pub forces_from_reserve: u16,
    pub face_dancer_called: bool,
    // ---- inherited from PlacementEvent ----
    pub to: Option<Location>,
    #[serde(with = "crate::events::wire::force_list")]
    #[ts(as = "Option<Vec<crate::events::wire::ForceEntry>>")]
    pub force_locations: Option<String>,
    #[serde(skip)]
    pub passed: bool,
}

impl Default for FaceDanced {
    fn default() -> Self {
        FaceDanced {
            target_force_locations: Some("".to_string()),
            forces_from_reserve: Default::default(),
            face_dancer_called: Default::default(),
            to: Location::from_id(0),
            force_locations: Some("".to_string()),
            passed: Default::default(),
        }
    }
}

impl FaceDanced {
    pub fn target_force_locations(&self, game: &Game, by: By) -> IndexMap<Location, Battalion> {
        // ParseForceLocations(Game, Player.Faction, _targetForceLocations): Player is dereferenced
        // first; a null string throws at `.Length`.
        let faction = game.player(by.player()).faction;
        let s = self.target_force_locations.as_deref().expect("target force locations");
        PlacementEvent::parse_force_locations(game, faction.expect("faction"), s)
    }

    pub fn set_target_force_locations(&mut self, game: &Game, value: &IndexMap<Location, Battalion>) {
        self.target_force_locations = Some(PlacementEvent::force_locations_string(game, value));
    }

    pub fn maximum_number_of_forces(g: &Game, p: PlayerId) -> i32 {
        let ally = g.player(p).allied_player(g);
        let territory = || g.current_battle().expect("current battle").territory;
        let condition = match ally {
            None => true,
            Some(ally) => g.battle_winner() == g.player(p).ally || g.player(ally).any_forces_in_territory(territory()) == 0,
        };
        if condition {
            let winner = g.get_player(g.battle_winner());
            let coocupying_player = if g.battle_winner() == g.fight().and_then(|c| c.ecaz_or_ally_fighter) {
                g.player(winner.expect("winner")).allied_player(g)
            } else {
                None
            };
            let mut nr_of_forces = g.player(winner.expect("winner")).any_forces_in_territory(territory());
            if let Some(co) = coocupying_player {
                nr_of_forces += g.player(co).any_forces_in_territory(territory());
            }
            return nr_of_forces;
        }

        0
    }

    pub fn may_call_face_dancer(g: &Game, p: PlayerId) -> bool {
        Self::may_call_face_dancer_opt(g, Some(p))
    }

    /// `MayCallFaceDancer` with a possibly null `p` (`Validate` passes `Player`):
    /// `p` is only dereferenced where it is needed, so a null `p` throws only there.
    pub(crate) fn may_call_face_dancer_opt(g: &Game, p: Option<PlayerId>) -> bool {
        // `BattleWinner == Faction.None` is false while BattleWinner is null.
        if g.battle_winner().is_none() {
            return false;
        }
        let Some(battle) = g.current_battle() else { return false };

        let territory = battle.territory.expect("territory");
        if territory.is_homeworld() && !g.player(p.expect("player")).is_native(territory) {
            return false;
        }

        let winner_hero = g.winner_hero();
        if let (Some(winner_hero), Some(battle_winner)) = (winner_hero, g.battle_winner()) {
            if !g.is_occupied_by_faction_or_their_ally(World::Tleilaxu, battle_winner) {
                let player = g.player(p.expect("player"));
                return player
                    .face_dancers
                    .iter()
                    .any(|t| t.is_face_dancer(winner_hero) && !player.revealed_face_dancers.contains(t));
            }
        }

        false
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_source_locations(g: &Game, p: PlayerId) -> impl Iterator<Item = Location> {
        g.locations_with_any_forces_not_in_storm(p).into_iter().filter(move |l| !l.is_homeworld())
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_target_locations(g: &Game) -> impl Iterator<Item = Location> {
        let territory = g.current_battle().expect("current battle").territory.expect("territory");
        territory
            .locations()
            .iter()
            .copied()
            .filter(move |l| g.applicable(Rule::BattlesUnderStorm) || !Sector::in_storm(l.sector(&g.board.map), g.board.sector_in_storm))
    }

    pub fn replace_forces(&self, game: &mut Game, by: By) -> GameResult {
        let Some(battle) = game.current_battle().cloned() else { return Ok(()) };

        let winner = game.get_player(game.battle_winner());
        let mut nr_of_removed_forces = game.player(winner.expect("winner")).any_forces_in_territory(battle.territory);
        let winner = winner.expect("winner");
        let co_occupying_player = if game.battle_winner() == game.fight().and_then(|c| c.ecaz_or_ally_fighter) {
            game.player(winner).allied_player(game)
        } else {
            None
        };
        if let Some(co) = co_occupying_player {
            nr_of_removed_forces += game.player(co).any_forces_in_territory(battle.territory);
        }

        if nr_of_removed_forces > 0 {
            Player::forces_in_territory_to_reserves(game, winner, battle.territory.expect("territory"))?;
            if let Some(co) = co_occupying_player {
                Player::forces_in_territory_to_reserves(game, co, battle.territory.expect("territory"))?;
            }
            Player::add_forces_to_reserves(game, by.player(), -i32::from(self.forces_from_reserve))?;

            for (location, battalion) in self.force_locations(game, by).iter() {
                let location = *location;
                Player::remove_forces(game, by.player(), location, battalion.forces())?;
                Player::remove_special_forces(game, by.player(), location, battalion.special_forces())?;
            }

            for (location, battalion) in self.target_force_locations(game, by).iter() {
                let location = *location;
                Player::add_forces(game, by.player(), location, battalion.forces(), false)?;
                Player::add_special_forces(game, by.player(), location, battalion.special_forces(), false)?;
            }

            let winner_faction = game.player(winner).faction;
            let co_faction = co_occupying_player.and_then(|co| game.player(co).faction);
            let replaced_by =
                self.target_force_locations(game, by).iter().map(|(_, b)| b.total_amount_of_forces()).sum::<i32>();
            let force = game.player(by.player()).force();
            let source_locations = self.determine_source_locations(game, by);
            game.log(parts![
                nr_of_removed_forces,
                " ",
                winner_faction,
                express_part_if!(co_occupying_player.is_some(); co_faction),
                " forces go back to reserves and are replaced by ",
                replaced_by,
                force,
                " (",
                i32::from(self.forces_from_reserve),
                " from reserves",
                source_locations,
                ")"
            ]);
        }
        Ok(())
    }

    pub fn replace_facedancers(&self, game: &mut Game, by: By) {
        let me = by.player();
        let face_dancers = game.player(me).face_dancers.clone();
        game.decks.traitor_deck.as_mut().expect("traitor deck").items.extend(face_dancers);
        game.player_mut(me).face_dancers.clear();
        game.player_mut(me).revealed_face_dancers.clear();
        game.decks.traitor_deck.as_mut().expect("traitor deck").shuffle(&mut game.rng);
        game.stone(Milestone::Shuffled);
        for _ in 0..3 {
            let drawn = game.decks.traitor_deck.as_mut().expect("traitor deck").draw();
            game.player_mut(me).face_dancers.push(drawn);
        }
        game.log(parts![by.faction, " draw 3 new Face Dancers."]);
    }

    pub fn determine_source_locations(&self, game: &Game, by: By) -> MessagePart {
        // ForceLocations is parsed once for Count and once more for the query.
        let any = self.force_locations(game, by).len() > 0;
        let query = Part::list(
            self.force_locations(game, by)
                .iter()
                .filter(|(_, b)| b.total_amount_of_forces() > 0)
                .map(|(l, b)| express_part!(", ", b.amount_of_forces, " from ", *l)),
        );
        express_part_if!(any; query)
    }
}

impl FaceDanced {
    /// Where the forces that replace the loser's come from: the reserves (`forces_from_reserve`)
    /// and these locations of the submitter `by`.
    pub fn force_locations(&self, game: &Game, by: By) -> IndexMap<Location, Battalion> {
        // ParseForceLocations(Game, Player.Faction, _forceLocations): Player is dereferenced
        // first; a null string throws at `.Length`.
        let faction = game.player(by.player()).faction;
        let s = self.force_locations.as_deref().expect("field force locations");
        PlacementEvent::parse_force_locations(game, faction.expect("faction"), s)
    }

    pub fn set_force_locations(&mut self, game: &Game, value: &IndexMap<Location, Battalion>) {
        self.force_locations = Some(PlacementEvent::force_locations_string(game, value));
    }

    pub fn total_amount_of_forces_added_to_location(&self, game: &Game, by: By) -> i32 {
        let forces: i32 = self.force_locations(game, by).values().map(|b| b.forces()).sum();
        let special_forces: i32 = self.force_locations(game, by).values().map(|b| b.special_forces()).sum();
        forces + special_forces
    }
}

impl EventRules for FaceDanced {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        let p = by.player;
        if !Self::may_call_face_dancer_opt(game, p) {
            return Err(Rejection(express!("You can't reveal a Face Dancer")));
        }

        let p = p.expect("player");
        let maximum_forces = Self::maximum_number_of_forces(game, p);
        let amount_of_placed_forces =
            self.target_force_locations(game, by).values().map(|b| b.total_amount_of_forces()).sum::<i32>();
        if amount_of_placed_forces > maximum_forces {
            return Err(Rejection(express!("Place ", maximum_forces, " or fewer forces")));
        }

        // The forces must come from reserves and locations that hold them (escapes.md, "Forces
        // below zero").
        let pl = game.player(p);
        if i32::from(self.forces_from_reserve) > pl.forces_in_reserve() {
            return Err(Rejection(express!("Invalid number of forces from reserves")));
        }
        if self.force_locations(game, by).iter().any(|(l, b)| b.forces() > pl.forces_in(Some(*l)) || b.special_forces() > pl.special_forces_in(Some(*l))) {
            return Err(Rejection(express!("Invalid number of forces")));
        }

        let amount_of_forces = i32::from(self.forces_from_reserve) + self.force_locations(game, by).values().map(|b| b.total_amount_of_forces()).sum::<i32>();
        if amount_of_forces != amount_of_placed_forces {
            return Err(Rejection(express!(
                "The amount of forces you selected from the planet and from reserves (",
                amount_of_forces,
                ") should equal the amount you wish to put in ",
                game.current_battle().expect("current battle").territory,
                " (",
                amount_of_placed_forces,
                ")"
            )));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let me = by.player();
        // A Game Error when no face dancer matches the winner (or there is no winner).
        let winner = game.winner_hero();
        let facedancer = game
            .player(me)
            .face_dancers
            .iter()
            .copied()
            .find(|f| winner.is_some_and(|w| w.is_face_dancer(*f)))
            .required("The face dancer of the winner")?;

        if matches!(facedancer, Hero::Leader(_)) && game.is_alive(facedancer) {
            game.kill_hero(facedancer);
        }

        for i in 0..game.players.len() {
            game.players[i].known_non_traitors.insert(facedancer);
        }

        game.player_mut(by.player()).revealed_face_dancers.insert(facedancer);

        if game.player(by.player()).unrevealed_face_dancers().next().is_none() {
            self.replace_facedancers(game, by);
        }

        self.replace_forces(game, by)?;
        game.flip_bene_gesserit_advisors_when_alone()?;

        game.finish_battle()?;
        Ok(())
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        express!(
            by.faction,
            " replace ",
            self.total_amount_of_forces_added_to_location(game, by),
            " forces with their own"
        )
    }
}
