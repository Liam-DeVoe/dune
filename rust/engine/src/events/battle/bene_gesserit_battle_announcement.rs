use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct BeneGesseritBattleAnnouncement {
    #[canon(name = "_territoryId")]
    pub territory_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for BeneGesseritBattleAnnouncement {
    fn default() -> Self {
        BeneGesseritBattleAnnouncement {
            territory_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl BeneGesseritBattleAnnouncement {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn territory(&self, game: &Game) -> Option<Territory> {
        game.map.territory_lookup().find(self.territory_id)
    }

    pub fn set_territory(&mut self, game: &Game, value: Option<Territory>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.territory_id = game.map.territory_lookup().get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.initiator() != Faction::BeneGesserit {
            return Some(express!("Your faction can't announce battle"));
        }
        // p.SpecialForcesIn(Territory): the argument (a lookup that may throw) is evaluated
        // before the call dereferences a null p.
        let territory = self.territory(game);
        let p = self.player();
        if game.player(p).special_forces_in_territory(territory) == 0 {
            return Some(express!("You don't have advisors there"));
        }
        let ally = game.get_player(Some(game.player(p).ally));
        if let Some(ally) = ally {
            if game.player(ally).any_forces_in_territory(self.territory(game)) > 0 {
                return Some(express!("Can't announce battle due to ally presence"));
            }
        }
        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_territories(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Territory> {
        let player = g.player(p);
        g.map
            .territories(g.applicable(Rule::Homeworlds))
            .filter(move |t| {
                let t = *t;
                (!player.has_ally() || g.player(player.allied_player(g).nre()).any_forces_in_territory(Some(t)) == 0)
                    && player.special_forces_in_territory(Some(t)) > 0
                    && (!t.is_stronghold() || g.nr_of_occupants_excluding_faction_territory_faction(t, player.faction) <= 1)
                    && !t.locations().iter().any(|l| l.sector(&g.map) == g.sector_in_storm && player.special_forces_in(Some(*l)) > 0)
            })
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        let initiator = self.get_player(game, Some(self.initiator()));
        let Some(initiator) = initiator else { return };
        let Some(territory) = self.territory(game) else { return };
        Player::flip_forces_territory_boolean(game, initiator, territory, false);
        self.log(game);
    }

    pub fn get_message(&self, game: &Game) -> Message {
        express!(self.initiator(), " flip to ", FactionForce::BeneGesserit, " in ", self.territory(game))
    }
}

impl GameEventMethods for BeneGesseritBattleAnnouncement {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::BeneGesseritBattleAnnouncement
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::BeneGesseritBattleAnnouncement(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        BeneGesseritBattleAnnouncement::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        BeneGesseritBattleAnnouncement::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        BeneGesseritBattleAnnouncement::get_message(self, game)
    }

}
