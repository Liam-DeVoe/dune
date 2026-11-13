use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct TerrorPlanted {
    pub type_: TerrorType,
    #[canon(name = "_territoryId")]
    pub territory_id: i32,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for TerrorPlanted {
    fn default() -> Self {
        TerrorPlanted {
            type_: Default::default(),
            territory_id: Default::default(),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl TerrorPlanted {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn stronghold(&self, game: &Game) -> Option<Territory> {
        game.map.territory_lookup().find(self.territory_id)
    }

    pub fn set_stronghold(&mut self, game: &Game, value: Option<Territory>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.territory_id = game.map.territory_lookup().get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.passed {
            return None;
        }

        match self.stronghold(game) {
            None => {
                if !game.terror_on_planet.contains_key(&self.type_) {
                    return Some(express!("You can't remove a token that is not on the board"));
                }
            }
            Some(stronghold) => {
                if !TerrorPlanted::valid_strongholds(game, self.player()).any(|v| v == stronghold) {
                    return Some(express!("Invalid Stronghold"));
                }
                if !TerrorPlanted::valid_terror_types(game, false).any(|v| v == self.type_) {
                    return Some(express!(self.type_, " not available"));
                }
            }
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_strongholds(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Territory> {
        use either::Either;
        use std::iter::empty;
        if g.current_main_phase != MainPhase::Contemplate {
            return Either::Left(empty());
        }

        let hms_territory = g.map.hidden_mobile_stronghold().territory();
        Either::Right(g.map.territories(false).filter(move |t| {
            t.is_visible(&g.map)
                && t.is_stronghold()
                && *t != hms_territory
                // !TerrorIn(t).Any(): stops at the first token.
                && (g.terror_in(*t).next().is_none() || TerrorPlanted::may_place_at_existing_token(g, p))
        }))
    }

    pub fn may_remove_tokens(g: &Game, p: PlayerIndex) -> bool {
        g.current_main_phase == MainPhase::Collection
            && g.player(p).has_high_threshold(g, World::Moritani)
            && !g.terror_on_planet.is_empty()
    }

    pub fn may_place_at_existing_token(game: &Game, p: PlayerIndex) -> bool {
        game.player(p).has_high_threshold(game, World::Moritani)
            || (game.player(p).nexus == Faction::Moritani && NexusPlayed::can_use_cunning(game, p))
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_terror_types(g: &Game, to_remove: bool) -> impl Iterator<Item = TerrorType> {
        use either::Either;
        if to_remove {
            // The live KeyCollection: no callbacks, so iterating it is the same as Contains/Any on it.
            return Either::Left(g.terror_on_planet.keys().copied());
        }

        Either::Right(crate::iter::unique(g.unplaced_terror_tokens.iter().copied().chain(g.terror_on_planet.keys().copied())))
    }

    pub fn is_applicable(g: &Game, p: PlayerIndex) -> bool {
        TerrorPlanted::may_remove_tokens(g, p)
            || (g.current_phase == Phase::Contemplate
                && !TerrorPlanted::valid_terror_types(g, false).next().is_none()
                && !TerrorPlanted::valid_strongholds(g, p).next().is_none()
                && !g.moritani_has_performed_terror
                && !g.prevented(FactionAdvantage::MoritaniPlantingTerror))
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let m = self.get_verbose_message(game);
        self.log_objects(game, vec![m.into()]);

        game.moritani_has_performed_terror = true;

        if !self.passed {
            game.stone(Milestone::TerrorPlanted);

            match self.stronghold(game) {
                None => {
                    game.terror_on_planet.remove(&self.type_);
                    game.unplaced_terror_tokens.push(self.type_);
                    let me = game.player_mut(self.player());
                    me.resources = me.resources.wrapping_add(4);
                }
                Some(stronghold) => {
                    let me = self.player();
                    if !game.terror_in(stronghold).next().is_none() && !game.player(me).has_high_threshold(game, World::Moritani) {
                        game.play_nexus_card(
                            me,
                            vec!["plant additional Terror in ".into(), stronghold.into()],
                        );
                    }

                    if game.unplaced_terror_tokens.contains(&self.type_) {
                        game.terror_on_planet.add_with(self.type_, stronghold, |k| ExpressionElement::from(*k).to_cs_string());
                        // List.Remove: the first equal element
                        if let Some(pos) = game.unplaced_terror_tokens.iter().position(|t| *t == self.type_) {
                            game.unplaced_terror_tokens.remove(pos);
                        }
                    } else {
                        game.terror_on_planet.remove(&self.type_);
                        game.terror_on_planet.add_with(self.type_, stronghold, |k| ExpressionElement::from(*k).to_cs_string());
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_verbose_message(&self, game: &Game) -> Message {
        if self.passed {
            return express!(self.initiator(), " don't plant terror");
        }

        if let Some(stronghold) = self.stronghold(game) {
            if let Some(territory) = game.terror_on_planet.get(&self.type_).copied() {
                return express!(self.initiator(), " move terror from ", territory, " to ", stronghold);
            }
            return express!(self.initiator(), " plant terror in ", stronghold);
        }

        express!(self.initiator(), " remove a terror token to get ", Payment::of(4))
    }

    pub fn get_message(&self, game: &Game) -> Message {
        if self.passed {
            return express!(self.initiator(), " don't plant terror");
        }
        if let Some(stronghold) = self.stronghold(game) {
            return express!(self.initiator(), " plant terror in ", stronghold);
        }
        express!(self.initiator(), " remove a terror token to get ", Payment::of(4))
    }
}

impl GameEventMethods for TerrorPlanted {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::TerrorPlanted
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::TerrorPlanted(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        TerrorPlanted::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        TerrorPlanted::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        TerrorPlanted::get_message(self, game)
    }

}

impl PassableGameEventMethods for TerrorPlanted {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
