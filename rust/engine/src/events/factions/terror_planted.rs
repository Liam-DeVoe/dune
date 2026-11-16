use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct TerrorPlanted {
    pub type_: Option<TerrorType>,
    pub stronghold: Option<Territory>,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `Event` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for TerrorPlanted {
    fn default() -> Self {
        TerrorPlanted {
            type_: Default::default(),
            stronghold: Territory::from_id(0),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl TerrorPlanted {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.passed {
            return None;
        }

        match self.stronghold {
            None => {
                if !self.type_.is_some_and(|t| game.board.terror_on_planet.contains_key(&t)) {
                    return Some(express!("You can't remove a token that is not on the board"));
                }
            }
            Some(stronghold) => {
                if !TerrorPlanted::valid_strongholds(game, self.player()).any(|v| v == stronghold) {
                    return Some(express!("Invalid Stronghold"));
                }
                if !TerrorPlanted::valid_terror_types(game, false).any(|v| Some(v) == self.type_) {
                    return Some(express!(self.type_, " not available"));
                }
            }
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_strongholds(g: &Game, p: PlayerId) -> impl Iterator<Item = Territory> {
        use either::Either;
        use std::iter::empty;
        if g.turn.current_main_phase != MainPhase::Contemplate {
            return Either::Left(empty());
        }

        let hms_territory = g.board.map.hidden_mobile_stronghold().territory();
        Either::Right(g.board.map.territories(false).filter(move |t| {
            t.is_visible(&g.board.map)
                && t.is_stronghold()
                && *t != hms_territory
                // !TerrorIn(t).Any(): stops at the first token.
                && (g.terror_in(*t).next().is_none() || TerrorPlanted::may_place_at_existing_token(g, p))
        }))
    }

    pub fn may_remove_tokens(g: &Game, p: PlayerId) -> bool {
        g.turn.current_main_phase == MainPhase::Collection
            && g.player(p).has_high_threshold(g, World::Moritani)
            && !g.board.terror_on_planet.is_empty()
    }

    pub fn may_place_at_existing_token(game: &Game, p: PlayerId) -> bool {
        game.player(p).has_high_threshold(game, World::Moritani)
            || (game.player(p).nexus == Some(Faction::Moritani) && NexusPlayed::can_use_cunning(game, p))
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_terror_types(g: &Game, to_remove: bool) -> impl Iterator<Item = TerrorType> {
        use either::Either;
        if to_remove {
            // The live KeyCollection: no callbacks, so iterating it is the same as Contains/Any on it.
            return Either::Left(g.board.terror_on_planet.keys().copied());
        }

        Either::Right(crate::iter::unique(g.board.unplaced_terror_tokens.iter().copied().chain(g.board.terror_on_planet.keys().copied())))
    }

    pub fn is_applicable(g: &Game, p: PlayerId) -> bool {
        TerrorPlanted::may_remove_tokens(g, p)
            || (g.turn.current_phase == Phase::Contemplate
                && !TerrorPlanted::valid_terror_types(g, false).next().is_none()
                && !TerrorPlanted::valid_strongholds(g, p).next().is_none()
                && !g.contemplate.moritani_has_performed_terror
                && !g.prevented(FactionAdvantage::MoritaniPlantingTerror))
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let m = self.get_verbose_message(game);
        self.log_objects(game, vec![m.into()]);

        game.contemplate.moritani_has_performed_terror = true;

        if !self.passed {
            let type_ = self.type_.required("terror type")?;
            game.stone(Milestone::TerrorPlanted);

            match self.stronghold {
                None => {
                    game.board.terror_on_planet.shift_remove(&type_);
                    game.board.unplaced_terror_tokens.push(type_);
                    let me = game.player_mut(self.player());
                    me.resources += 4;
                }
                Some(stronghold) => {
                    let me = self.player();
                    if !game.terror_in(stronghold).next().is_none() && !game.player(me).has_high_threshold(game, World::Moritani) {
                        game.play_nexus_card(
                            me,
                            vec!["plant additional Terror in ".into(), stronghold.into()],
                        );
                    }

                    if game.board.unplaced_terror_tokens.contains(&type_) {
                        game.board.terror_on_planet.insert_new(type_, stronghold);
                        // List.Remove: the first equal element
                        if let Some(pos) = game.board.unplaced_terror_tokens.iter().position(|t| *t == type_) {
                            game.board.unplaced_terror_tokens.remove(pos);
                        }
                    } else {
                        game.board.terror_on_planet.shift_remove(&type_);
                        game.board.terror_on_planet.insert_new(type_, stronghold);
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

        if let Some(stronghold) = self.stronghold {
            if let Some(territory) = self.type_.and_then(|t| game.board.terror_on_planet.get(&t).copied()) {
                return express!(self.initiator(), " move terror from ", territory, " to ", stronghold);
            }
            return express!(self.initiator(), " plant terror in ", stronghold);
        }

        express!(self.initiator(), " remove a terror token to get ", Payment::of(4))
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.passed {
            return express!(self.initiator(), " don't plant terror");
        }
        if let Some(stronghold) = self.stronghold {
            return express!(self.initiator(), " plant terror in ", stronghold);
        }
        express!(self.initiator(), " remove a terror token to get ", Payment::of(4))
    }
}

impl GameEventMethods for TerrorPlanted {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> EventType {
        EventType::TerrorPlanted
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::TerrorPlanted(self)
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
