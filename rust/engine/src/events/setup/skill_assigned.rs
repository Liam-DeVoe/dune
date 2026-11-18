use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct SkillAssigned {
    pub skill: Option<LeaderSkill>,
    pub leader: Option<Leader>,
    #[serde(skip)]
    pub passed: bool,
}

impl Default for SkillAssigned {
    fn default() -> Self {
        SkillAssigned {
            skill: Default::default(),
            leader: None,
            passed: Default::default(),
        }
    }
}

impl SkillAssigned {
    pub fn players_must_choose_leader_skills(g: &Game) -> bool {
        g.players.iter().any(|p| !p.skills_to_choose_from.is_empty())
    }

    pub fn valid_skills(game: &Game, p: PlayerId) -> Vec<LeaderSkill> {
        game.player(p).skills_to_choose_from.clone()
    }

    /// Returns `Option` elements because `p.MostRecentlyRevivedLeader` can be null.
    pub fn valid_leaders(g: &Game, p: PlayerId) -> Vec<Option<Leader>> {
        if g.phase() == Phase::AssigningInitialSkills {
            return g.player(p).leaders.iter().copied().filter(|l| l.hero_type() != HeroType::Auditor).map(Some).collect();
        }
        vec![g.player(p).most_recently_revived_leader]
    }
}

impl EventRules for SkillAssigned {
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        if self.passed && game.phase() == Phase::AssigningInitialSkills {
            return Err(Rejection(express!("You must assign a leader skill")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // `passed` isn't read: a pass has no leader, so it fails below.
        let m = self.describe(game, by);
        game.log_event(m);
        let leader = match self.leader {
            Some(l) => l,
            None => game_error!("No leader was chosen for the skill"),
        };
        game.set_skill(leader, self.skill);
        let me = by.player();
        let skills = &mut game.player_mut(me).skills_to_choose_from;
        if let Some(i) = skills.iter().position(|s| Some(*s) == self.skill) {
            skills.remove(i);
        }
        game.set_in_front_of_shield(leader, true);
        // SkillDeck is dereferenced before Player.SkillsToChooseFrom is read.
        game.decks.skill_deck.as_ref().required("skill deck")?;
        // PutOnTop(IEnumerable) reverses its input.
        let remaining = game.player(me).skills_to_choose_from.clone();
        game.decks.skill_deck.as_mut().required("skill deck")?.put_on_top_all(remaining);
        game.player_mut(me).skills_to_choose_from.clear();

        if !Self::players_must_choose_leader_skills(game) {
            game.decks.skill_deck.as_mut().required("skill deck")?.shuffle(&mut game.rng);
            if game.phase() != Phase::AssigningInitialSkills {
                game.resume(Interrupt::SkillAssignment, true)?;
            } else {
                if game.treachery_cards_before_traitors() {
                    game.deal_traitors()?;
                } else {
                    game.setup_spice_and_forces()?;
                }
            }
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, _by: By) -> Message {
        express!(self.leader, " becomes ", self.skill)
    }
}
