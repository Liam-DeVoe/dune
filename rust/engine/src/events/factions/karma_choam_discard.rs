use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct KarmaChoamDiscard {
    pub card_to_use: Option<TreacheryCard>,
    #[serde(with = "crate::events::wire::id_list")]
    #[ts(as = "Option<Vec<i32>>")]
    pub card_ids: Option<String>,
}

impl Default for KarmaChoamDiscard {
    fn default() -> Self {
        KarmaChoamDiscard {
            card_to_use: TreacheryCardManager::get(0),
            card_ids: Default::default(),
        }
    }
}

impl KarmaChoamDiscard {
    pub fn cards(&self) -> Vec<TreacheryCard> {
        Event::id_string_to_objects(self.card_ids.as_deref(), &TreacheryCardManager::lookup())
    }

    pub fn set_cards(&mut self, value: &[TreacheryCard]) {
        self.card_ids = Some(Event::objects_to_id_string(value, &TreacheryCardManager::lookup()));
    }

    /// `Cards.Select(c => MessagePart.Express(" ", c, " "))` as one message element.
    fn cards_parts(cards: &[TreacheryCard]) -> Part {
        Part::list(cards.iter().map(|c| express_part!(" ", *c, " ")))
    }
}

impl EventRules for KarmaChoamDiscard {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        let Some(karma_card_to_use) = self.card_to_use else {
            return Err(Rejection(express!("You must select a ", TreacheryCardType::Karma, " card to use")));
        };

        if self.cards().contains(&karma_card_to_use) {
            return Err(Rejection(express!("You can't discard the card you need to play to use this power")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let me = by.player();
        // C# parity: plays the first valid Karama card, not the validated CardToUse.
        let card = Karma::valid_karma_cards(game, me).into_iter().next();
        let cards = self.cards();
        let parts = Self::cards_parts(&cards);
        let amount = self.cards().len() as i32 * 3;
        game.play_karma(me, card, parts!["discard ", parts, "to get ", Payment::of(amount)]);

        for card in self.cards() {
            game.discard_card_of(Some(me), Some(card), true);
        }

        let gain = self.cards().len() as i32 * 3;
        let p = game.player_mut(me);
        crate::game::amounts::gain(&mut p.resources, gain, "spice")?;
        p.special_karma_power_used = true;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        let cards = self.cards();
        express!(
            by.faction,
            " using ",
            TreacheryCardType::Karma,
            ", ",
            by.faction,
            " discard ",
            Self::cards_parts(&cards),
            "to get ",
            Payment::of(self.cards().len() as i32 * 3)
        )
    }
}
