use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct SwitchedSkilledLeader {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for SwitchedSkilledLeader {
    fn default() -> Self {
        SwitchedSkilledLeader {
            base: Default::default(),
        }
    }
}

impl SwitchedSkilledLeader {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn switchable_leader(game: &Game, player: PlayerIndex) -> Option<Leader> {
        let faction = game.player(player).faction;
        game.player(player).leaders.iter().copied().find(|l| {
            game.is_skilled(Some(Hero::Leader(*l)))
                && !game.captured_leaders.contains_key(l)
                && (faction == Faction::Ecaz || l.hero_type() != HeroType::Vidal)
        })
    }

    pub fn can_be_played(game: &Game, player: PlayerIndex) -> bool {
        if game.current_phase != Phase::BattlePhase {
            return false;
        }
        let Some(battle) = game.current_battle.as_ref() else {
            return false;
        };
        battle.is_aggressor_or_defender(game, player)
            && battle.plan_of(game, Some(player)).is_none()
            && Self::switchable_leader(game, player).is_some()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let leader = Self::switchable_leader(game, self.player());
        let value = !game.is_in_front_of_shield(leader.map(Hero::Leader));
        let leader = match leader {
            Some(l) => l,
            None => game_error!("There is no skilled leader to switch"),
        };
        game.set_in_front_of_shield(leader, value);
        let hero = Some(Hero::Leader(leader));
        let skill = game.skill(hero);
        let in_front = game.is_in_front_of_shield(hero);
        self.log_objects(
            game,
            vec![
                self.initiator().into(),
                " place ".into(),
                skill.into(),
                " ".into(),
                leader.into(),
                if in_front { " in front of" } else { " behind" }.into(),
                " their shield".into(),
            ],
        );
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " switch their skilled leader")
    }

}

impl GameEventMethods for SwitchedSkilledLeader {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::SwitchedSkilledLeader
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::SwitchedSkilledLeader(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        SwitchedSkilledLeader::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        SwitchedSkilledLeader::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        SwitchedSkilledLeader::get_message(self, game)
    }

}
