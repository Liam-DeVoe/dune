use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct ResourcesAudited {
    pub target: Faction,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for ResourcesAudited {
    fn default() -> Self {
        ResourcesAudited {
            target: Default::default(),
            base: Default::default(),
        }
    }
}

impl ResourcesAudited {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !ResourcesAudited::valid_factions(game, self.player()).any(|v| v == self.target) {
            return Some(express!("Invalid faction"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_factions(game: &Game, player: PlayerIndex) -> impl Iterator<Item = Faction> {
        (0..game.players.len())
            .map(move |i| game.player_at(i))
            .filter(move |opp| {
                let o = game.player(*opp);
                *opp != player
                    && !game.resource_audited_factions.contains(&o.faction)
                    && (game.player(player).home_worlds.iter().any(|hw| o.any_forces_in(Some(hw.location())) > 0)
                        || o.home_worlds.iter().any(|hw| game.player(player).any_forces_in(Some(hw.location())) > 0))
            })
            .map(move |p| game.player(p).faction)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.resource_audited_factions.push(self.target);
        let target = self.get_player(game, Some(self.target));
        self.log(game);
        let t = game.player(target.required("target")?);
        let resources = t.resources;
        let weapons = t.treachery_cards.iter().filter(|tc| tc.is_weapon()).count() as i32;
        let defenses = t.treachery_cards.iter().filter(|tc| tc.is_defense()).count() as i32;
        self.log_to(
            game,
            self.initiator(),
            vec![
                self.target.into(),
                " own ".into(),
                Payment::of(resources).into(),
                ", ".into(),
                weapons.into(),
                " weapons and ".into(),
                defenses.into(),
                " defenses".into(),
            ],
        );
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(
            self.initiator(),
            " force ",
            self.target,
            " to reveal amount of weapons, defenses and ",
            Concept::Resource
        )
    }
}

impl GameEventMethods for ResourcesAudited {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::ResourcesAudited
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::ResourcesAudited(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ResourcesAudited::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        ResourcesAudited::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ResourcesAudited::get_message(self, game)
    }

}
