use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct HarvesterPlayed {
}

impl EventRules for HarvesterPlayed {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // Only reached outside the phase when events are replayed without validation.
        if game.blow().is_none() {
            game_error!("Not in the spice blow phase");
        }
        game.discard(by.player(), TreacheryCardType::Harvester)?;
        let last_resource_card = if game.phase() == Phase::HarvesterA { game.board.latest_spice_card_a } else { game.board.latest_spice_card_b };
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
