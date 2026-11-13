use crate::prelude::*;
use crate::compat::lazy::LazyQuery;

#[derive(Clone, Debug, Canon)]
pub struct FactionSelected {
    pub initiator_player_name: Option<String>,
    pub seat: i32,
    pub faction: Faction,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for FactionSelected {
    fn default() -> Self {
        FactionSelected {
            initiator_player_name: Default::default(),
            seat: Default::default(),
            faction: Default::default(),
            base: Default::default(),
        }
    }
}

impl FactionSelected {
    pub fn new(game: &Game) -> Self {
        let e = Self::default();
        e.initialize(game, Faction::None);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.faction != Faction::None && !Self::valid_factions(game).cs_contains(&self.faction) {
            return Some(express!("Faction not available"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_factions(g: &Game) -> impl Iterator<Item = Faction> {
        // The list is shared (RefCell), so take its items now. Nothing changes it while a caller enumerates.
        let factions: Vec<Faction> = g.factions_in_play.borrow().clone();
        factions.into_iter().filter(move |f| g.players.iter().all(|p| p.faction != *f))
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        let initiator = game.get_player_by_seat(self.seat);

        if let Some(initiator) = initiator {
            game.player_mut(initiator).faction = self.faction;

            self.log(game);
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator_player_name.clone(), " plays ", self.faction)
    }

}

impl GameEventMethods for FactionSelected {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::FactionSelected
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::FactionSelected(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        FactionSelected::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        FactionSelected::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        FactionSelected::get_message(self, game)
    }

}
