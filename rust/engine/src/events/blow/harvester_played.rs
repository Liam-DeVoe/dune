use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HarvesterPlayed {
}

impl EventRules for HarvesterPlayed {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.discard(by.player(), TreacheryCardType::Harvester)?;
        let last_resource_card = if game.turn.current_phase == Phase::HarvesterA { game.blow.latest_spice_card_a } else { game.blow.latest_spice_card_b };
        // lastResourceCard!.Location!: a null card is a NullReferenceException; a null Location
        // (a Shai-Hulud card) reaches TryGetValue, which throws ArgumentNullException.
        let Some(location) = last_resource_card.required("last resource card")?.location() else {
            game_error!("Value cannot be null. (Parameter 'key')")
        };
        if let Some(current_amount_of_spice) = game.board.resources_on_planet.get(&location).copied() {
            game.change_resources_on_planet(location, i32::from(current_amount_of_spice))?;
        }

        let m = self.describe(game, by);
        game.log_event(m);
        game.move_to_next_phase_after_resource_blow()?;
        game.stone(Milestone::Harvester);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " use a ", TreacheryCardType::Harvester)
    }
}
