use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct TakeLosses {
    pub force_amount: u16,
    pub special_force_amount: u16,
    pub use_useless_card: bool,
}

impl TakeLosses {
    pub fn losses_to_take(g: &Game) -> LossToTake {
        if g.turn.storm_losses_to_take.len() > 0 {
            return g.turn.storm_losses_to_take[0].clone();
        }
        LossToTake::new()
    }

    pub fn valid_max_force_amount(g: &Game, p: PlayerId) -> i32 {
        g.player(p).forces_in(TakeLosses::losses_to_take(g).location)
    }

    pub fn valid_max_special_force_amount(g: &Game, p: PlayerId) -> i32 {
        g.player(p).special_forces_in(TakeLosses::losses_to_take(g).location)
    }

    pub fn valid_useless_card_to_prevent_losses(g: &Game, p: PlayerId) -> Option<TreacheryCard> {
        let pl = g.player(p);
        if pl.faction == Some(Faction::Choam) && !g.prevented(FactionAdvantage::ChoamDiscarding) {
            return pl.treachery_cards.iter().copied().find(|c| c.id() == TreacheryCardManager::CARD_JUBBA_CLOAK);
        }

        None
    }

    pub fn can_prevent_losses(g: &Game, p: PlayerId) -> bool {
        g.player(p).is(Faction::Choam)
            && ((!g.prevented(FactionAdvantage::ChoamDiscarding) && TakeLosses::valid_useless_card_to_prevent_losses(g, p).is_some())
                || (NexusPlayed::can_use_cunning(g, p) && !g.player(p).treachery_cards.is_empty()))
    }

    pub fn half_of(amount_of_forces: i32, amount_of_special_forces: i32) -> i32 {
        // The int addition wraps, as in C#.
        half_rounded_up(amount_of_forces.wrapping_add(amount_of_special_forces))
    }
}

impl EventRules for TakeLosses {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if self.use_useless_card && !TakeLosses::can_prevent_losses(game, by.player()) {
            return Err(Rejection(express!("You can't use a card to prevent force losses")));
        }
        if self.use_useless_card {
            return Ok(());
        }


        let value_to_be_killed = TakeLosses::losses_to_take(game).amount;
        if (i32::from(self.force_amount) + i32::from(self.special_force_amount)) < value_to_be_killed {
            return Err(Rejection(express!("Select a total value of at least ", value_to_be_killed, " to be killed")));
        }

        if i32::from(self.force_amount) > TakeLosses::valid_max_force_amount(game, by.player()) {
            return Err(Rejection(express!("Invalid amount of forces")));
        }
        if i32::from(self.special_force_amount) > TakeLosses::valid_max_special_force_amount(game, by.player()) {
            return Err(Rejection(express!("Invalid amount of forces")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let player = game.get_player(by.faction);
        let mut must_discard = false;

        if self.use_useless_card {
            let card = TakeLosses::valid_useless_card_to_prevent_losses(game, by.player());
            if card.is_none() && NexusPlayed::can_use_cunning(game, player.required("player")?) {
                let location = game.turn.storm_losses_to_take[0].location;
                game.play_nexus_card(player.required("player")?, vec!["prevent losing forces in ".into(), location.into()]);
                must_discard = true;
            } else {
                game.discard_card_of(Some(by.player()), card, true);
                let location = game.turn.storm_losses_to_take[0].location;
                game.log(vec![by.faction.into(), " prevent losing forces in ".into(), location.into()]);
            }
            game.turn.storm_losses_to_take.remove(0);
            game.stone(Milestone::SpecialUselessPlayed);
        } else {
            let location = TakeLosses::losses_to_take(game).location;
            let player = player.required("player")?;
            // KillForces(null, ...): GetAndCreateIfNeeded looks up a null key.
            let Some(kill_location) = location else {
                game_error!("Value cannot be null. (Parameter 'key')");
            };
            Player::kill_forces(game, player, kill_location, i32::from(self.force_amount), i32::from(self.special_force_amount), false)?;
            game.turn.storm_losses_to_take.remove(0);
            let message = self.describe(game, by).with_log(StormKilled::new(by.faction.required("faction")?, kill_location, i32::from(self.force_amount), i32::from(self.special_force_amount)));
            game.log(vec![message.into()]);
        }

        if game.is_interrupted_by(Interrupt::StormLosses) {
            // Losses of a shipment or move: back to it, which asks for the next loss if any.
            game.resume(Interrupt::StormLosses, true)?;
            game.determine_next_shipment_and_move_sub_phase()?;
        } else if !game.turn.storm_losses_to_take.is_empty() {
            game.enter_storm(StormStep::Losses);
        } else {
            game.end_storm_phase();
        }

        if must_discard {
            game.let_player_discard_treachery_card_of_choice(by.faction.required("faction")?);
        }
        Ok(())
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        self.try_describe(game, by).expect("the message of an applicable TakeLosses event")
    }

    fn try_describe(&self, game: &Game, by: By) -> GameResult<Message> {
        let p = by.player.required("player")?;
        let force = game.player(p).force();
        let special_force = game.player(p).special_force();
        Ok(express!(
            "The storm kills ",
            express_part_if!(i32::from(self.force_amount) > 0; i32::from(self.force_amount), force),
            express_part_if!(i32::from(self.force_amount) > 0 && i32::from(self.special_force_amount) > 0; " and "),
            express_part_if!(i32::from(self.special_force_amount) > 0; i32::from(self.special_force_amount), special_force)
        ))
    }
}
