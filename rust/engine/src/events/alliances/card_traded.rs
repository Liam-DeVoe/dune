use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CardTraded {
    pub target: Option<Faction>,
    pub card: Option<TreacheryCard>,
    pub requested_card: Option<TreacheryCard>,
}

impl Default for CardTraded {
    fn default() -> Self {
        CardTraded {
            target: Default::default(),
            card: TreacheryCardManager::get(0),
            requested_card: None,
        }
    }
}

impl CardTraded {
    pub fn valid_cards(game: &Game, p: PlayerId) -> Vec<TreacheryCard> {
        game.player(p).treachery_cards.clone()
    }
}

impl EventRules for CardTraded {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        let me = by.player();
        // List.Contains(null) is true when the list holds a null card.
        let valid_cards = CardTraded::valid_cards(game, me);
        if !self.card.is_some_and(|x| valid_cards.contains(&x)) {
            return Err(Rejection(express!("Invalid card")));
        }
        let ally = game.player(me).allied_player(game).expect("allied player");
        // An ally without cards couldn't answer, and the game would wait forever.
        if game.player(ally).treachery_cards.is_empty() {
            return Err(Rejection(express!("Your ally does not have cards to trade")));
        }

        let target_player = game.get_player(self.target);
        let Some(target_player) = target_player else { return Err(Rejection(express!("Invalid target player"))) };
        // Only the ally can be offered a trade. An offer to another faction would put the game in
        // TradingCards waiting for an answer that can't be validated without an ally, and the game
        // would stop there (fuzz seed 1 game 31, rust/docs/escapes.md).
        if self.target != game.player(me).ally {
            return Err(Rejection(express!("You can only trade cards with your ally")));
        }

        if self.requested_card.is_some() && !game.is_bot(game.player(me).allied_player(game).expect("allied player")) {
            return Err(Rejection(express!("You can only select a card from a Bot ally")));
        }
        if let Some(requested) = self.requested_card {
            if !CardTraded::valid_cards(game, target_player).contains(&requested) {
                return Err(Rejection(express!("Invalid requested card")));
            }
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        if game.card_trade_offer().is_none() {
            let m = self.describe(game, by);
            game.log_event(m);
            let offer = CardTradeOffer {
                initiator: by.faction,
                player: by.player,
                target: self.target,
                card: self.card,
                requested_card: self.requested_card,
            };
            game.interrupt(Interrupt::CardTrade { offer });
        } else {
            let offer = game.card_trade_offer().cloned().required("current card trade offer")?;
            game.log(parts![by.faction, " and ", offer.initiator, " exchange a card"]);
            let other_player = offer.player;

            let other_count = game.player(other_player.required("other player")?).treachery_cards.len();
            let me = by.player();
            if other_count > 1 || game.player(me).treachery_cards.len() > 1 {
                let other_player = other_player.required("other player")?;
                for i in 0..game.players.len() {
                    let p = game.player_at(i);
                    if p != other_player && p != me {
                        let other_cards = game.player(other_player).treachery_cards.clone();
                        Game::unregister_all_known_by_player(game, p, &other_cards);
                        let my_cards = game.player(me).treachery_cards.clone();
                        Game::unregister_all_known_by_player(game, p, &my_cards);
                    }
                }
            }

            let other_player = other_player.required("other player")?;
            let card = self.card.required("card")?;
            game.player_mut(other_player).treachery_cards.push(card);
            crate::iter::remove_first(&mut game.player_mut(me).treachery_cards, &card);
            let offered_card = offer.card.required("offered card")?;
            game.player_mut(me).treachery_cards.push(offered_card);
            crate::iter::remove_first(&mut game.player_mut(other_player).treachery_cards, &offered_card);
            game.stone(Milestone::CardTraded);
            game.alliances.last_turn_card_was_traded = Some(game.turn.current_turn);
            game.resume_phase(Phase::TradingCards, true)?;
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " offer their ally a card trade")
    }
}
