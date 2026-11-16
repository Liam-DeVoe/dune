use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct BeneGesseritBattleAnnouncement {
    pub territory: Option<Territory>,
    /// The fields of `Event` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for BeneGesseritBattleAnnouncement {
    fn default() -> Self {
        BeneGesseritBattleAnnouncement {
            territory: Territory::from_id(0),
            base: Default::default(),
        }
    }
}

impl BeneGesseritBattleAnnouncement {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.initiator() != Some(Faction::BeneGesserit) {
            return Some(express!("Your faction can't announce battle"));
        }
        // p.SpecialForcesIn(Territory): the argument (a lookup that may throw) is evaluated
        // before the call dereferences a null p.
        let territory = self.territory;
        let p = self.player();
        if game.player(p).special_forces_in_territory(territory) == 0 {
            return Some(express!("You don't have advisors there"));
        }
        let ally = game.get_player(game.player(p).ally);
        if let Some(ally) = ally {
            if game.player(ally).any_forces_in_territory(self.territory) > 0 {
                return Some(express!("Can't announce battle due to ally presence"));
            }
        }
        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_territories(g: &Game, p: PlayerId) -> impl Iterator<Item = Territory> {
        let player = g.player(p);
        g.board.map
            .territories(g.applicable(Rule::Homeworlds))
            .filter(move |t| {
                let t = *t;
                (!player.has_ally() || g.player(player.allied_player(g).expect("allied player")).any_forces_in_territory(Some(t)) == 0)
                    && player.special_forces_in_territory(Some(t)) > 0
                    && (!t.is_stronghold() || g.nr_of_occupants_of_territory_excluding(t, player.faction) <= 1)
                    && !t.locations().iter().any(|l| Sector::in_storm(l.sector(&g.board.map), g.board.sector_in_storm) && player.special_forces_in(Some(*l)) > 0)
            })
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let initiator = self.get_player(game, self.initiator());
        let Some(initiator) = initiator else { return Ok(()) };
        let Some(territory) = self.territory else { return Ok(()) };
        Player::flip_forces_in_territory(game, initiator, territory, false)?;
        self.log(game);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " flip to ", FactionForce::BeneGesserit, " in ", self.territory)
    }
}

impl GameEventMethods for BeneGesseritBattleAnnouncement {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> EventType {
        EventType::BeneGesseritBattleAnnouncement
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::BeneGesseritBattleAnnouncement(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        BeneGesseritBattleAnnouncement::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        BeneGesseritBattleAnnouncement::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        BeneGesseritBattleAnnouncement::get_message(self, game)
    }

}
