use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct SkillAssigned {
    pub skill: LeaderSkill,
    #[canon(name = "_leaderId")]
    pub leader_id: i32,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for SkillAssigned {
    fn default() -> Self {
        SkillAssigned {
            skill: Default::default(),
            leader_id: Default::default(),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl SkillAssigned {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn leader(&self) -> Option<Leader> {
        LeaderManager::leader_lookup().find(self.leader_id)
    }

    pub fn set_leader(&mut self, value: Option<Leader>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.leader_id = LeaderManager::leader_lookup().get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.passed && game.current_phase == Phase::AssigningInitialSkills {
            return Some(express!("You must assign a leader skill"));
        }

        None
    }

    pub fn players_must_choose_leader_skills(g: &Game) -> bool {
        g.players.iter().any(|p| !p.skills_to_choose_from.is_empty())
    }

    pub fn valid_skills(game: &Game, p: PlayerIndex) -> Vec<LeaderSkill> {
        game.player(p).skills_to_choose_from.clone()
    }

    /// Returns `Option` elements because `p.MostRecentlyRevivedLeader` can be null.
    pub fn valid_leaders(g: &Game, p: PlayerIndex) -> Vec<Option<Leader>> {
        if g.current_phase == Phase::AssigningInitialSkills {
            return g.player(p).leaders.iter().copied().filter(|l| l.hero_type() != HeroType::Auditor).map(Some).collect();
        }
        vec![g.player(p).most_recently_revived_leader]
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        // C# parity (hazards 18.2): Passed is ignored.
        self.log(game);
        let leader = match self.leader() {
            Some(l) => l,
            None => game_error!("No leader was chosen for the skill"),
        };
        game.set_skill(leader, self.skill);
        let me = self.player();
        let skills = &mut game.player_mut(me).skills_to_choose_from;
        if let Some(i) = skills.iter().position(|s| *s == self.skill) {
            skills.remove(i);
        }
        game.set_in_front_of_shield(leader, true);
        // SkillDeck is dereferenced before Player.SkillsToChooseFrom is read.
        game.skill_deck.as_ref().required("skill deck")?;
        // PutOnTop(IEnumerable) reverses its input (hazards 2.3).
        let remaining = game.player(me).skills_to_choose_from.clone();
        game.skill_deck.as_mut().required("skill deck")?.put_on_top_all(remaining);
        game.player_mut(me).skills_to_choose_from.clear();

        if !Self::players_must_choose_leader_skills(game) {
            game.skill_deck.as_mut().required("skill deck")?.shuffle();
            let c1 = game.current_phase != Phase::AssigningInitialSkills;
            let phase = game.phase_before_skill_assignment;
            let c2 = game.treachery_cards_before_traitors();
            game.enter_boolean_phase_boolean_action_action(c1, phase, c2, Game::deal_traitors, Game::setup_spice_and_forces);
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.leader(), " becomes ", self.skill)
    }

}

impl GameEventMethods for SkillAssigned {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::SkillAssigned
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::SkillAssigned(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        SkillAssigned::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        SkillAssigned::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        SkillAssigned::get_message(self, game)
    }

}

impl PassableGameEventMethods for SkillAssigned {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
