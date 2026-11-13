use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct ReplacedCardWon {
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for ReplacedCardWon {
    fn default() -> Self {
        ReplacedCardWon {
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl ReplacedCardWon {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        if !self.passed {
            let card_just_won = game.card_just_won;
            game.discard_treachery_card_boolean(card_just_won, true)?;
            let initiator = self.get_player(game, Some(self.initiator()));
            let new_card = game.draw_treachery_card();
            // C# adds a null card when both treachery deck and discard pile are empty.
            let initiator = initiator.required("initiator")?;
            game.player_mut(initiator).treachery_cards.push(new_card.required("new_card")?);
            game.stone(Milestone::CardWonSwapped);

            if game.replacing_bought_card_using_nexus {
                game.play_nexus_card(self.player(), vec!["replace the card they just bought".into()]);
                game.replacing_bought_card_using_nexus = false;
            } else {
                self.log(game);
            }

            let initiator_faction = game.player(initiator).faction;
            let card_just_won = game.card_just_won;
            self.log_to(game, initiator_faction, vec!["You replaced your ".into(), card_just_won.into(), " with a ".into(), new_card.into()]);
        }

        if game.card_just_won == game.card_sold_on_black_market {
            game.enter_richese_bidding();
        } else {
            game.determine_next_step_after_card_was_sold();
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.passed {
            return express!(self.initiator(), " keep the card they just won");
        }
        express!(self.initiator(), " discard the card they just won and draw a new card")
    }

}

impl GameEventMethods for ReplacedCardWon {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::ReplacedCardWon
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::ReplacedCardWon(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ReplacedCardWon::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        ReplacedCardWon::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ReplacedCardWon::get_message(self, game)
    }

}

impl PassableGameEventMethods for ReplacedCardWon {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
