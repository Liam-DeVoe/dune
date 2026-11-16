use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct KarmaShipmentPrevention {
    pub target: Option<Faction>,
}

impl KarmaShipmentPrevention {
    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn get_valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Faction> {
        g.players_other_than(p).filter(|f| *f != Faction::Fremen)
    }
}

impl EventRules for KarmaShipmentPrevention {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !KarmaShipmentPrevention::get_valid_targets(game, by.player()).any(|v| Some(v) == self.target) {
            return Err(Rejection(express!("Invalid target")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.ship_move.current_karma_shipment_prevention = Some(ShipmentPrevention { target: self.target });
        let me = by.player();
        let card = Karma::valid_karma_cards(game, me).into_iter().next();
        game.play_karma(me, card, vec!["prevent shipment by ".into(), self.target.into()]);
        game.player_mut(me).special_karma_power_used = true;
        game.stone(Milestone::Karma);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!("Using ", TreacheryCardType::Karma, ", ", by.faction, " prevent shipment by ", self.target)
    }
}
