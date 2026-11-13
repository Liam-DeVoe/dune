use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct AllianceOffered {
    pub target: Faction,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for AllianceOffered {
    fn default() -> Self {
        AllianceOffered {
            target: Default::default(),
            base: Default::default(),
        }
    }
}

impl AllianceOffered {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        // ValidTargets dereferences a null Player only inside its lazy predicate, per element.
        if !AllianceOffered::valid_targets(game, self.player_opt().unwrap_or(PlayerIndex::NULL)).any(|v| v == self.target) {
            return Some(express!("Invalid faction"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_targets(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Faction> {
        (0..g.players.len())
            .map(move |i| g.player_at(i))
            .filter(move |other| {
                p != *other && !g.player(p).have_forces_on_each_others_home_worlds(g, *other) && !AllianceOffered::has_passed_in_alliance_phase(g, *other)
            })
            .map(move |other| g.player(other).faction)
    }

    fn has_passed_in_alliance_phase(g: &Game, p: PlayerIndex) -> bool {
        matches!(g.current_phase, Phase::AllianceA | Phase::AllianceB) && g.has_acted_or_passed.contains(&g.player(p).faction)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let initiator = self.initiator();
        let matching_offer =
            game.current_alliance_offers.iter().find(|x| x.initiator() == self.target && x.target == initiator).cloned();
        if matching_offer.is_some() {
            game.make_alliance(initiator, self.target);

            loop {
                let invalid_offer = game.current_alliance_offers.iter().position(|x| x.by(initiator) || x.initiator() == self.target);
                let Some(i) = invalid_offer else { break };
                // List.Remove(invalidOffer): the first occurrence of that object, which is at i.
                game.current_alliance_offers.remove(i);
            }

            // Offers to the new allies lapse too; otherwise their makers would wait for an answer that never comes.
            loop {
                let lapsed_offer =
                    game.current_alliance_offers.iter().position(|x| x.target == initiator || x.target == self.target);
                let Some(i) = lapsed_offer else { break };
                game.current_alliance_offers.remove(i);
            }

            game.has_acted_or_passed.push(initiator);
            game.has_acted_or_passed.push(self.target);

            game.end_nexus_if_everyone_is_done();
        } else {
            let m = self
                .get_message(game)
                .with_log(NexusLog::new(None, Some(NexusAlliance::new(NexusAllianceChange::Offered, initiator, self.target))));
            self.log_objects(game, vec![m.into()]);
            game.current_alliance_offers.push(self.clone());
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " offer an alliance to ", self.target)
    }

}

impl GameEventMethods for AllianceOffered {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::AllianceOffered
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::AllianceOffered(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        AllianceOffered::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        AllianceOffered::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        AllianceOffered::get_message(self, game)
    }

}
