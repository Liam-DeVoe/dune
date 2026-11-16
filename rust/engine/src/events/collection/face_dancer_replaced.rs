use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct FaceDancerReplaced {
    pub selected_dancer: Option<Hero>,
    pub passed: bool,
}

impl Default for FaceDancerReplaced {
    fn default() -> Self {
        FaceDancerReplaced {
            selected_dancer: TreacheryCardManager::get(0).map(Hero::TreacheryCard),
            passed: Default::default(),
        }
    }
}

impl FaceDancerReplaced {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_face_dancers(game: &Game, p: PlayerId) -> impl Iterator<Item = Hero> {
        game.player(p).unrevealed_face_dancers()
    }
}

impl EventRules for FaceDancerReplaced {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !self.passed {
            let Some(selected_dancer) = self.selected_dancer else {
                return Err(Rejection(express!("Select a leader")));
            };
            let p = game.player(by.player());
            if p.revealed_face_dancers.contains(&selected_dancer) {
                return Err(Rejection(express!("You can't replace a revealed Face Dancer")));
            }
            if !p.face_dancers.contains(&selected_dancer) {
                return Err(Rejection(express!("Invalid Face Dancer")));
            }
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        if !self.passed {
            if let Some(selected_dancer) = self.selected_dancer {
                let player = by.player();
                // List.Remove: the first match, if any.
                if let Some(i) = game.player(player).face_dancers.iter().position(|d| *d == selected_dancer) {
                    game.player_mut(player).face_dancers.remove(i);
                }
                game.decks.traitor_deck.as_mut().required("traitor deck")?.put_on_top(selected_dancer);
                game.decks.traitor_deck.as_mut().required("traitor deck")?.shuffle(&mut game.rng);
                game.stone(Milestone::Shuffled);
                let leader = game.decks.traitor_deck.as_mut().required("traitor deck")?.try_draw()?;
                game.player_mut(player).face_dancers.push(leader);
                game.player_mut(player).known_non_traitors.insert(leader);
            }
        }

        let m = self.describe(game, by);
        game.log_event(m);
        game.enter(Phase::TurnConcluded, true);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, express_part_if!(self.passed; " don't"), " replace a Face Dancer")
    }
}
