use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct FaceDanced {
    #[canon(name = "_targetForceLocations")]
    pub target_force_locations: Option<String>,
    pub forces_from_reserve: i32,
    pub face_dancer_called: bool,
    // ---- inherited from PlacementEvent ----
    #[canon(name = "_toId")]
    pub to_id: i32,
    #[canon(name = "_forceLocations")]
    pub force_locations: Option<String>,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for FaceDanced {
    fn default() -> Self {
        FaceDanced {
            target_force_locations: Some("".to_string()),
            forces_from_reserve: Default::default(),
            face_dancer_called: Default::default(),
            to_id: Default::default(),
            force_locations: Some("".to_string()),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

/// The type name shown when the collection built in `DetermineSourceLocations` is rendered as text.
/// Kept deliberately so the message text stays the same.
const SOURCE_LOCATIONS_QUERY_TYPE: &str = "System.Linq.Enumerable+IEnumerableWhereSelectIterator`2[System.Collections.Generic.KeyValuePair`2[Treachery.Shared.Location,Treachery.Shared.Battalion],Treachery.Shared.MessagePart]";

impl FaceDanced {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn target_force_locations(&self, game: &Game) -> NetDictionary<Location, Battalion> {
        // ParseForceLocations(Game, Player.Faction, _targetForceLocations): Player is dereferenced
        // first; a null string throws at `.Length`.
        let faction = game.player(self.player()).faction;
        let s = self.target_force_locations.as_deref().nre();
        PlacementEvent::parse_force_locations(game, faction, s)
    }

    pub fn set_target_force_locations(&mut self, game: &Game, value: &NetDictionary<Location, Battalion>) {
        self.target_force_locations = Some(PlacementEvent::force_locations_string(game, value));
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let p = self.player_opt();
        if !Self::may_call_face_dancer_opt(game, p) {
            return Some(express!("You can't reveal a Face Dancer"));
        }

        let p = p.nre();
        let maximum_forces = Self::maximum_number_of_forces(game, p);
        let amount_of_placed_forces =
            dotnet_compat::linq::sum_i32(self.target_force_locations(game).values().map(|b| b.total_amount_of_forces()));
        if amount_of_placed_forces > maximum_forces {
            return Some(express!("Place ", maximum_forces, " or less forces"));
        }

        let amount_of_forces = self
            .forces_from_reserve
            .wrapping_add(dotnet_compat::linq::sum_i32(self.force_locations(game).values().map(|b| b.total_amount_of_forces())));
        if amount_of_forces != amount_of_placed_forces {
            return Some(express!(
                "The amount of forces you selected from the planet and from reserves (",
                amount_of_forces,
                ") should equal the amount you wish to put in ",
                game.current_battle.as_ref().nre().territory(game),
                " (",
                amount_of_placed_forces,
                ")"
            ));
        }

        None
    }

    pub fn maximum_number_of_forces(g: &Game, p: PlayerIndex) -> i32 {
        let ally = g.player(p).allied_player(g);
        let territory = || g.current_battle.as_ref().nre().territory(g);
        let condition = match ally {
            None => true,
            Some(ally) => g.battle_winner == Some(g.player(p).ally) || g.player(ally).any_forces_in_territory(territory()) == 0,
        };
        if condition {
            let winner = g.get_player(g.battle_winner);
            let coocupying_player = if g.battle_winner == g.current_ecaz_or_ally_fighter {
                g.player(winner.nre()).allied_player(g)
            } else {
                None
            };
            let mut nr_of_forces = g.player(winner.nre()).any_forces_in_territory(territory());
            if let Some(co) = coocupying_player {
                nr_of_forces = nr_of_forces.wrapping_add(g.player(co).any_forces_in_territory(territory()));
            }
            return nr_of_forces;
        }

        0
    }

    pub fn may_call_face_dancer(g: &Game, p: PlayerIndex) -> bool {
        Self::may_call_face_dancer_opt(g, Some(p))
    }

    /// `MayCallFaceDancer` with a possibly null `p` (`Validate` passes `Player`, hazards 1.8):
    /// `p` is only dereferenced where it is needed, so a null `p` throws only there.
    pub(crate) fn may_call_face_dancer_opt(g: &Game, p: Option<PlayerIndex>) -> bool {
        let p = p.and_then(crate::compat::cs::CsNull::opt);
        // `BattleWinner == Faction.None` is false while BattleWinner is null (hazards 10.1).
        if g.battle_winner == Some(Faction::None) {
            return false;
        }
        let Some(battle) = g.current_battle.as_ref() else { return false };

        let territory = battle.territory(g).nre();
        if territory.is_homeworld() && !g.player(p.nre()).is_native(territory) {
            return false;
        }

        let winner_hero = g.winner_hero();
        if let (Some(winner_hero), Some(battle_winner)) = (winner_hero, g.battle_winner) {
            if !g.is_occupied_by_faction_or_their_ally_world_faction(World::Tleilaxu, battle_winner) {
                let player = g.player(p.nre());
                return player
                    .face_dancers
                    .iter()
                    .any(|t| t.is_face_dancer(winner_hero) && !player.revealed_face_dancers.contains(t));
            }
        }

        false
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_source_locations(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Location> {
        g.locations_with_any_forces_not_in_storm(p).into_iter().filter(move |l| !l.is_homeworld())
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_target_locations(g: &Game) -> impl Iterator<Item = Location> {
        let territory = g.current_battle.as_ref().nre().territory(g).nre();
        territory
            .locations()
            .iter()
            .copied()
            .filter(move |l| g.applicable(Rule::BattlesUnderStorm) || l.sector(&g.map) != g.sector_in_storm)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        let me = self.player();
        let facedancer = game
            .player(me)
            .face_dancers
            .iter()
            .copied()
            .cs_first_where(|f| game.winner_hero().nre().is_face_dancer(*f));

        if matches!(facedancer, Hero::Leader(_)) && game.is_alive(facedancer) {
            game.kill_hero(facedancer);
        }

        for i in 0..game.players.len() {
            game.players[i].known_non_traitors.add(facedancer);
        }

        game.player_mut(self.player()).revealed_face_dancers.add(facedancer);

        if game.player(self.player()).unrevealed_face_dancers().next().is_none() {
            self.replace_facedancers(game);
        }

        self.replace_forces(game);
        game.flip_bene_gesserit_advisors_when_alone();

        game.finish_battle();
    }

    pub fn replace_forces(&self, game: &mut Game) {
        let Some(battle) = game.current_battle.clone() else { return };

        let winner = self.get_player(game, game.battle_winner);
        let mut nr_of_removed_forces = game.player(winner.nre()).any_forces_in_territory(battle.territory(game));
        let winner = winner.nre();
        let co_occupying_player = if game.battle_winner == game.current_ecaz_or_ally_fighter {
            game.player(winner).allied_player(game)
        } else {
            None
        };
        if let Some(co) = co_occupying_player {
            nr_of_removed_forces = nr_of_removed_forces.wrapping_add(game.player(co).any_forces_in_territory(battle.territory(game)));
        }

        if nr_of_removed_forces > 0 {
            Player::forces_to_reserves_territory(game, winner, battle.territory(game).nre());
            if let Some(co) = co_occupying_player {
                Player::forces_to_reserves_territory(game, co, battle.territory(game).nre());
            }
            Player::add_forces_to_reserves(game, self.player(), self.forces_from_reserve.wrapping_neg());

            for (location, battalion) in self.force_locations(game).iter() {
                let location = *location;
                Player::remove_forces(game, self.player(), location, battalion.amount_of_forces);
                Player::remove_special_forces(game, self.player(), location, battalion.amount_of_special_forces);
            }

            for (location, battalion) in self.target_force_locations(game).iter() {
                let location = *location;
                Player::add_forces(game, self.player(), location, battalion.amount_of_forces, false);
                Player::add_special_forces(game, self.player(), location, battalion.amount_of_special_forces, false);
            }

            let winner_faction = game.player(winner).faction;
            let co_faction = co_occupying_player.map(|co| game.player(co).faction);
            let replaced_by =
                dotnet_compat::linq::sum_i32(self.target_force_locations(game).iter().map(|(_, b)| b.total_amount_of_forces()));
            let force = game.player(self.player()).force();
            let source_locations = self.determine_source_locations(game);
            game.log(elems![
                nr_of_removed_forces,
                " ",
                winner_faction,
                express_part_if!(co_occupying_player.is_some(); co_faction),
                " forces go back to reserves and are replaced by ",
                replaced_by,
                force,
                " (",
                self.forces_from_reserve,
                " from reserves",
                source_locations,
                ")"
            ]);
        }
    }

    pub fn replace_facedancers(&self, game: &mut Game) {
        let me = self.player();
        let face_dancers = game.player(me).face_dancers.clone();
        game.traitor_deck.as_mut().nre().items.extend(face_dancers);
        game.player_mut(me).face_dancers.clear();
        game.player_mut(me).revealed_face_dancers.clear();
        game.traitor_deck.as_mut().nre().shuffle();
        game.stone(Milestone::Shuffled);
        for _ in 0..3 {
            let drawn = game.traitor_deck.as_mut().nre().draw();
            game.player_mut(me).face_dancers.push(drawn);
        }
        game.log(elems![self.initiator(), " draw 3 new Face Dancers."]);
    }

    pub fn determine_source_locations(&self, game: &Game) -> MessagePart {
        // ForceLocations is parsed once for Count and once more for the query.
        let any = self.force_locations(game).len() > 0;
        let query = ExpressionElement::collection(
            SOURCE_LOCATIONS_QUERY_TYPE,
            self.force_locations(game)
                .iter()
                .filter(|(_, b)| b.total_amount_of_forces() > 0)
                .map(|(l, b)| express_part!(", ", b.amount_of_forces, " from ", *l)),
        );
        express_part_if!(any; query)
    }

    pub fn get_message(&self, game: &Game) -> Message {
        express!(
            self.initiator(),
            " replace ",
            <Self as PlacementEventMethods>::total_amount_of_forces_added_to_location(self, game),
            " forces by their own"
        )
    }
}

impl GameEventMethods for FaceDanced {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::FaceDanced
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::FaceDanced(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        FaceDanced::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        FaceDanced::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        FaceDanced::get_message(self, game)
    }

}

impl PassableGameEventMethods for FaceDanced {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}

impl PlacementEventMethods for FaceDanced {
    fn field_to_id(&self) -> &i32 {
        &self.to_id
    }

    fn field_force_locations(&self) -> &Option<String> {
        &self.force_locations
    }

    fn field_to_id_mut(&mut self) -> &mut i32 {
        &mut self.to_id
    }

    fn field_force_locations_mut(&mut self) -> &mut Option<String> {
        &mut self.force_locations
    }

}

impl crate::events::location_event::ILocationEvent for FaceDanced {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self)
    }

    fn to(&self, game: &Game) -> Option<Location> {
        <Self as PlacementEventMethods>::to(self, game)
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        <Self as PlacementEventMethods>::total_amount_of_forces_added_to_location(self, game)
    }

    fn forces_added_to_location(&self, game: &Game) -> i32 {
        <Self as PlacementEventMethods>::forces_added_to_location(self, game)
    }

    fn special_forces_added_to_location(&self, game: &Game) -> i32 {
        <Self as PlacementEventMethods>::special_forces_added_to_location(self, game)
    }

}

impl crate::events::placement::IPlacement for FaceDanced {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self)
    }

    fn force_locations(&self, game: &Game) -> NetDictionary<Location, Battalion> {
        <Self as PlacementEventMethods>::force_locations(self, game)
    }

    fn to(&self, game: &Game) -> Option<Location> {
        <Self as PlacementEventMethods>::to(self, game)
    }

}
