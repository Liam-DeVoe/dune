use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct CardTraded {
    pub target: Faction,
    #[canon(name = "_cardId")]
    pub card_id: i32,
    #[canon(name = "_requestedCardId")]
    pub requested_card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for CardTraded {
    fn default() -> Self {
        CardTraded {
            target: Default::default(),
            card_id: Default::default(),
            requested_card_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl CardTraded {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::get(self.card_id)
    }

    pub fn set_card(&mut self, value: Option<TreacheryCard>) {
        self.card_id = TreacheryCardManager::get_id(value);
    }

    pub fn requested_card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::get(self.requested_card_id)
    }

    pub fn set_requested_card(&mut self, value: Option<TreacheryCard>) {
        self.requested_card_id = TreacheryCardManager::get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        // List.Contains(null) is true when the list holds a null card.
        let valid_cards = CardTraded::valid_cards(game, self.player());
        if !self.card().is_some_and(|x| valid_cards.contains(&x)) {
            return Some(express!("Invalid card"));
        }
        let ally = game.player(self.player()).allied_player(game).expect("allied player");
        if !game.player(ally).treachery_cards.iter().any(|_| true) {
            // C# parity: the message is built but never returned, so this check has no effect.
            let _ = express!("Your ally does not have cards to trade");
        }

        let target_player = game.get_player(Some(self.target));
        let Some(target_player) = target_player else { return Some(express!("Invalid target player")) };
        // Not in C#, which accepted an offer to any faction. The client always names the ally, but an
        // offer to another faction put the game in TradingCards waiting for a player whose answer
        // can't be validated without an ally, so the game stopped there (fuzz seed 1 game 31,
        // rust/docs/escapes.md).
        if self.target != game.player(self.player()).ally {
            return Some(express!("You can only trade cards with your ally"));
        }

        if self.requested_card().is_some() && !game.is_bot(game.player(self.player()).allied_player(game).expect("allied player")) {
            return Some(express!("You can only select a card from a Bot ally"));
        }
        if let Some(requested) = self.requested_card() {
            if !CardTraded::valid_cards(game, target_player).contains(&requested) {
                return Some(express!("Invalid requested card"));
            }
        }

        None
    }

    pub fn valid_cards(game: &Game, p: PlayerIndex) -> Vec<TreacheryCard> {
        game.player(p).treachery_cards.clone()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        if game.current_card_trade_offer.is_none() {
            self.log(game);
            game.current_card_trade_offer = Some(self.clone());
            game.phase_before_card_trade = Some(game.current_phase);
            game.enter(Phase::TradingCards, true);
        } else {
            let offer = game.current_card_trade_offer.clone().required("current card trade offer")?;
            self.log_objects(game, vec![self.initiator().into(), " and ".into(), offer.initiator().into(), " exchange a card".into()]);
            let other_player = offer.player_opt();

            let other_count = game.player(other_player.required("other player")?).treachery_cards.len();
            if other_count > 1 || game.player(self.player()).treachery_cards.len() > 1 {
                let other_player = other_player.required("other player")?;
                let me = self.player();
                for i in 0..game.players.len() {
                    let p = game.player_at(i);
                    if p != other_player && p != me {
                        let other_cards = game.player(other_player).treachery_cards.clone();
                        Game::unregister_known_player_i_enumerable_treachery_card(game, p, &other_cards);
                        let my_cards = game.player(me).treachery_cards.clone();
                        Game::unregister_known_player_i_enumerable_treachery_card(game, p, &my_cards);
                    }
                }
            }

            let other_player = other_player.required("other player")?;
            let me = self.player();
            // C# adds `Card!` even when it is null (only reachable without validation).
            let card = self.card().required("card")?;
            game.player_mut(other_player).treachery_cards.push(card);
            crate::iter::remove_first(&mut game.player_mut(me).treachery_cards, &card);
            let offered_card = offer.card().required("offered card")?;
            game.player_mut(me).treachery_cards.push(offered_card);
            crate::iter::remove_first(&mut game.player_mut(other_player).treachery_cards, &offered_card);
            game.current_card_trade_offer = None;
            game.stone(Milestone::CardTraded);
            game.last_turn_card_was_traded = game.current_turn;
            if let Some(phase) = game.phase_before_card_trade {
                game.enter(phase, true);
            }
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " offer their ally a card trade")
    }

}

impl GameEventMethods for CardTraded {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::CardTraded
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::CardTraded(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        CardTraded::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        CardTraded::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        CardTraded::get_message(self, game)
    }

}
