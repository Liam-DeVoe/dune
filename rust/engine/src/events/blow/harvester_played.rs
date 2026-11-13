use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct HarvesterPlayed {
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for HarvesterPlayed {
    fn default() -> Self {
        HarvesterPlayed {
            base: Default::default(),
        }
    }
}

impl HarvesterPlayed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.discard(self.player(), TreacheryCardType::Harvester)?;
        let last_resource_card = if game.current_phase == Phase::HarvesterA { game.latest_spice_card_a } else { game.latest_spice_card_b };
        // lastResourceCard!.Location!: a null card is a NullReferenceException; a null Location
        // (a Shai-Hulud card) reaches TryGetValue, which throws ArgumentNullException.
        let Some(location) = last_resource_card.required("last resource card")?.location() else {
            game_error!("Value cannot be null. (Parameter 'key')")
        };
        if let Some(current_amount_of_spice) = game.resources_on_planet.get(&location).copied() {
            game.change_resources_on_planet(location, current_amount_of_spice);
        }

        self.log(game);
        game.move_to_next_phase_after_resource_blow();
        game.stone(Milestone::Harvester);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " use a ", TreacheryCardType::Harvester)
    }

}

impl GameEventMethods for HarvesterPlayed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::HarvesterPlayed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::HarvesterPlayed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        HarvesterPlayed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        HarvesterPlayed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        HarvesterPlayed::get_message(self, game)
    }

}
