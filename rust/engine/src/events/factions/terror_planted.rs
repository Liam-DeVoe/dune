use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct TerrorPlanted {
    #[serde(rename = "terrorType")]
    pub type_: Option<TerrorType>,
    pub stronghold: Option<Territory>,
    #[serde(skip)]
    pub passed: bool,
}

impl Default for TerrorPlanted {
    fn default() -> Self {
        TerrorPlanted {
            type_: Default::default(),
            stronghold: Territory::from_id(0),
            passed: Default::default(),
        }
    }
}

impl TerrorPlanted {
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
            || (g.phase() == Phase::Contemplate
                && !TerrorPlanted::valid_terror_types(g, false).next().is_none()
                && !TerrorPlanted::valid_strongholds(g, p).next().is_none()
                && !g.contemplate().is_some_and(|c| c.moritani_has_performed_terror)
                && !g.prevented(FactionAdvantage::MoritaniPlantingTerror))
    }

    pub fn get_verbose_message(&self, game: &Game, by: By) -> Message {
        if self.passed {
            return express!(by.faction, " don't plant terror");
        }

        if let Some(stronghold) = self.stronghold {
            if let Some(territory) = self.type_.and_then(|t| game.board.terror_on_planet.get(&t).copied()) {
                return express!(by.faction, " move terror from ", territory, " to ", stronghold);
            }
            return express!(by.faction, " plant terror in ", stronghold);
        }

        express!(by.faction, " remove a terror token to get ", Payment::of(4))
    }
}

impl EventRules for TerrorPlanted {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if self.passed {
            return Ok(());
        }

        match self.stronghold {
            None => {
                if !self.type_.is_some_and(|t| game.board.terror_on_planet.contains_key(&t)) {
                    return Err(Rejection(express!("You can't remove a token that is not on the board")));
                }
            }
            Some(stronghold) => {
                if !TerrorPlanted::valid_strongholds(game, by.player()).any(|v| v == stronghold) {
                    return Err(Rejection(express!("Invalid Stronghold")));
                }
                if !TerrorPlanted::valid_terror_types(game, false).any(|v| Some(v) == self.type_) {
                    return Err(Rejection(express!(self.type_, " not available")));
                }
            }
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.get_verbose_message(game, by);
        game.log(parts![m]);

        // Outside the Mentat Pause (Moritani remove a token in the collection phase) this
        // wrote a flag that the next pause reset before reading.
        if let Some(c) = game.contemplate_mut() {
            c.moritani_has_performed_terror = true;
        }

        if !self.passed {
            let type_ = self.type_.required("terror type")?;
            game.stone(Milestone::TerrorPlanted);

            match self.stronghold {
                None => {
                    game.board.terror_on_planet.shift_remove(&type_);
                    game.board.unplaced_terror_tokens.push(type_);
                    let me = game.player_mut(by.player());
                    me.resources += 4;
                }
                Some(stronghold) => {
                    let me = by.player();
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

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.passed {
            return express!(by.faction, " don't plant terror");
        }
        if let Some(stronghold) = self.stronghold {
            return express!(by.faction, " plant terror in ", stronghold);
        }
        express!(by.faction, " remove a terror token to get ", Payment::of(4))
    }
}
