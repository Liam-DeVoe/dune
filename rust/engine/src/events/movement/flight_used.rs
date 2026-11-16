use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlightUsed {
    pub move_three_territories: bool,
}

impl FlightUsed {
    pub fn extra_move(&self) -> bool {
        !self.move_three_territories
    }

    pub fn is_available(game: &Game, p: PlayerId) -> bool {
        game.player(p).treachery_cards.iter().any(|c| c.type_() == TreacheryCardType::Flight)
    }
}

impl EventRules for FlightUsed {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        game.discard(by.player(), TreacheryCardType::Flight)?;
        game.ship_move.current_flight_used = Some(FlightUse { by, move_three_territories: self.move_three_territories });
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(
            by.faction,
            " use an ",
            TreacheryCardType::Flight,
            " to ",
            if self.extra_move() { "move an additional group of forces" } else { "gain movement speed" }
        )
    }
}
