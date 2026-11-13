use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct TakeLosses {
    pub force_amount: i32,
    pub special_force_amount: i32,
    pub use_useless_card: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for TakeLosses {
    fn default() -> Self {
        TakeLosses {
            force_amount: Default::default(),
            special_force_amount: Default::default(),
            use_useless_card: Default::default(),
            base: Default::default(),
        }
    }
}

impl TakeLosses {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.use_useless_card && !TakeLosses::can_prevent_losses(game, self.player()) {
            return Some(express!("You can't use a card to prevent force losses"));
        }
        if self.use_useless_card {
            return None;
        }

        let value_to_be_killed = TakeLosses::losses_to_take(game).amount;
        if (self.force_amount + self.special_force_amount) < value_to_be_killed {
            return Some(express!("Select a total value of at least ", value_to_be_killed, " to be killed"));
        }

        if self.force_amount > TakeLosses::valid_max_force_amount(game, self.player()) {
            return Some(express!("Invalid amount of forces"));
        }
        if self.special_force_amount > TakeLosses::valid_max_special_force_amount(game, self.player()) {
            return Some(express!("Invalid amount of forces"));
        }

        None
    }

    pub fn losses_to_take(g: &Game) -> LossToTake {
        if g.storm_losses_to_take.len() > 0 {
            return g.storm_losses_to_take[0].clone();
        }
        LossToTake::new()
    }

    pub fn valid_max_force_amount(g: &Game, p: PlayerIndex) -> i32 {
        g.player(p).forces_in(TakeLosses::losses_to_take(g).location)
    }

    pub fn valid_max_special_force_amount(g: &Game, p: PlayerIndex) -> i32 {
        g.player(p).special_forces_in(TakeLosses::losses_to_take(g).location)
    }

    pub fn valid_useless_card_to_prevent_losses(g: &Game, p: PlayerIndex) -> Option<TreacheryCard> {
        let pl = g.player(p);
        if pl.faction == Faction::Choam && !g.prevented(FactionAdvantage::ChoamDiscarding) {
            return pl.treachery_cards.iter().copied().find(|c| c.id() == TreacheryCardManager::CARD_JUBBA_CLOAK);
        }

        None
    }

    pub fn can_prevent_losses(g: &Game, p: PlayerIndex) -> bool {
        g.player(p).is(Faction::Choam)
            && ((!g.prevented(FactionAdvantage::ChoamDiscarding) && TakeLosses::valid_useless_card_to_prevent_losses(g, p).is_some())
                || (NexusPlayed::can_use_cunning(g, p) && !g.player(p).treachery_cards.is_empty()))
    }

    pub fn half_of(amount_of_forces: i32, amount_of_special_forces: i32) -> i32 {
        // double arithmetic; the int addition wraps first.
        (0.5f64 * (amount_of_forces + amount_of_special_forces) as f64).ceil() as i32
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let player = self.get_player(game, Some(self.initiator()));
        let mut must_discard = false;

        if self.use_useless_card {
            let card = TakeLosses::valid_useless_card_to_prevent_losses(game, self.player());
            if card.is_none() && NexusPlayed::can_use_cunning(game, player.required("player")?) {
                let location = game.storm_losses_to_take[0].location;
                game.play_nexus_card(player.required("player")?, vec!["prevent losing forces in ".into(), location.into()]);
                must_discard = true;
            } else {
                game.discard_player_treachery_card_boolean(Some(self.player()), card, true);
                let location = game.storm_losses_to_take[0].location;
                self.log_objects(game, vec![self.initiator().into(), " prevent losing forces in ".into(), location.into()]);
            }
            game.storm_losses_to_take.remove(0);
            game.stone(Milestone::SpecialUselessPlayed);
        } else {
            let location = TakeLosses::losses_to_take(game).location;
            let player = player.required("player")?;
            // KillForces(null, ...): GetAndCreateIfNeeded looks up a null key.
            let Some(kill_location) = location else {
                game_error!("Value cannot be null. (Parameter 'key')");
            };
            Player::kill_forces(game, player, kill_location, self.force_amount, self.special_force_amount, false);
            game.storm_losses_to_take.remove(0);
            let message = self.get_message(game).with_log(StormKilled::new(self.initiator(), kill_location, self.force_amount, self.special_force_amount));
            game.log(vec![message.into()]);
        }

        if game.phase_before_storm_loss == Phase::BlowA {
            let condition = game.storm_losses_to_take.len() > 0;
            game.enter_boolean_phase_action(condition, Phase::StormLosses, Game::end_storm_phase);
        } else {
            let phase = game.phase_before_storm_loss;
            game.enter(phase, true);
            game.determine_next_shipment_and_move_sub_phase()?;
        }

        if must_discard {
            game.let_player_discard_treachery_card_of_choice(self.initiator());
        }
        Ok(())
    }

    pub fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        let p = self.player_opt().required("player")?;
        let force = game.player(p).force();
        let special_force = game.player(p).special_force();
        Ok(express!(
            "The storm kills ",
            express_part_if!(self.force_amount > 0; self.force_amount, force),
            express_part_if!(self.force_amount > 0 && self.special_force_amount > 0; " and "),
            express_part_if!(self.special_force_amount > 0; self.special_force_amount, special_force)
        ))
    }

    pub fn get_message(&self, game: &Game) -> Message {
        self.try_get_message(game).expect("the message of an applicable TakeLosses event")
    }

}

impl GameEventMethods for TakeLosses {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::TakeLosses
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::TakeLosses(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        TakeLosses::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        TakeLosses::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        TakeLosses::get_message(self, game)
    }

    fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        TakeLosses::try_get_message(self, game)
    }

}
