use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct KarmaEcazDial {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for KarmaEcazDial {
    fn default() -> Self {
        KarmaEcazDial {
            base: Default::default(),
        }
    }
}

impl KarmaEcazDial {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let me = self.player();
        let karama = Karma::valid_karma_cards(game, me).into_iter().next();
        game.player_mut(me).special_karma_power_used = true;
        game.stone(Milestone::Karma);
        let battle = game.current_battle.clone().required("current battle")?;
        let my_leader = battle.plan_of_faction(game, self.initiator()).required("plan of faction")?.hero();
        let opponent_leader = battle.plan_of_opponent(game, Some(me)).required("plan of opponent")?.hero();
        if let (Some(my_leader), Some(opponent_leader)) = (my_leader, opponent_leader) {
            game.ecaz_karma_bonus =
                dotnet_compat::math::abs_i32(my_leader.value().wrapping_sub(opponent_leader.value_in_combat_against(Some(my_leader))));
        }
        let bonus = game.ecaz_karma_bonus;
        game.play_karma(me, karama, vec!["add ".into(), bonus.into(), " to their dial".into()]);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!("Using ", TreacheryCardType::Karma, ", ", self.initiator(), " add the difference between leader discs to their dial")
    }
}

impl GameEventMethods for KarmaEcazDial {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::KarmaEcazDial
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::KarmaEcazDial(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        KarmaEcazDial::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        KarmaEcazDial::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        KarmaEcazDial::get_message(self, game)
    }

}
