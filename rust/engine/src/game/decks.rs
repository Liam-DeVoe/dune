use crate::prelude::*;

/// The card decks, token piles and discard piles.
pub struct Decks {
    pub treachery_deck: Option<Deck<TreacheryCard>>,
    pub treachery_discard_pile: Option<Deck<TreacheryCard>>,
    pub removed_treachery_cards: Vec<TreacheryCard>,
    pub recently_discarded: NetDictionary<TreacheryCard, Faction>,
    pub resource_card_deck: Option<Deck<ResourceCard>>,
    pub resource_card_discard_pile_a: Option<Deck<ResourceCard>>,
    pub resource_card_discard_pile_b: Option<Deck<ResourceCard>>,
    pub traitor_deck: Option<Deck<Hero>>,
    pub skill_deck: Option<Deck<LeaderSkill>>,
    pub nexus_card_deck: Option<Deck<Faction>>,
    pub nexus_discard_pile: Vec<Faction>,
    pub richese_cache: Vec<TreacheryCard>,
    pub unassigned_ambassadors: Option<Deck<Ambassador>>,
    pub ambassadors_set_aside: Vec<Ambassador>,
    pub fremen_discovery_tokens: Option<Deck<DiscoveryToken>>,
    pub guild_discovery_tokens: Option<Deck<DiscoveryToken>>,
}

impl Decks {
    pub fn new() -> Self {
        Decks {
            treachery_deck: None,
            treachery_discard_pile: None,
            removed_treachery_cards: Vec::new(),
            recently_discarded: Default::default(),
            resource_card_deck: None,
            resource_card_discard_pile_a: None,
            resource_card_discard_pile_b: None,
            traitor_deck: None,
            skill_deck: None,
            nexus_card_deck: None,
            nexus_discard_pile: Vec::new(),
            richese_cache: Vec::new(),
            unassigned_ambassadors: None,
            ambassadors_set_aside: Vec::new(),
            fremen_discovery_tokens: None,
            guild_discovery_tokens: None,
        }
    }
}

impl Game {
    /// Discards the first card of `card_type` in the player's hand (for Bene Gesserit, a Useless
    /// card can stand in for Karma). Fails if they have none.
    pub fn discard(&mut self, player: PlayerIndex, card_type: TreacheryCardType) -> GameResult {
        let bene_gesserit_karma = card_type == TreacheryCardType::Karma && self.player(player).is(Faction::BeneGesserit);
        let card = self
            .player(player)
            .treachery_cards
            .iter()
            .copied()
            .find(|x| if bene_gesserit_karma { x.type_() == TreacheryCardType::Karma || x.type_() == TreacheryCardType::Useless } else { x.type_() == card_type })
            .ok_or_else(|| GameError::new(format!("{} have no {} card to discard", self.player(player).faction, card_type)))?;

        self.discard_card_of(Some(player), Some(card), true);
        Ok(())
    }

    /// `log` is false when the caller logs the card's play itself, such as PlayKarma.
    /// Discards `card` from the hand that holds it. Fails if more than one hand holds it.
    pub fn discard_card(&mut self, card: Option<TreacheryCard>, log: bool) -> GameResult {
        let Some(c) = card else { return Ok(()) };

        let mut holders = self.player_indices().filter(|p| self.player(*p).has(Some(c)));
        let player = holders.next();
        if holders.next().is_some() {
            game_error!("Card {} is in more than one hand", c.id());
        }
        self.discard_card_of(player, Some(c), log);
        self.register_known(c);
        Ok(())
    }

    pub fn discard_card_of(&mut self, player: Option<PlayerIndex>, card: Option<TreacheryCard>, log: bool) {
        if let (Some(player), Some(card)) = (player, card) {
            let faction = self.player(player).faction;
            if log {
                let mut fact = BattleFact::new(BattleFactKind::Discard, faction);
                fact.card = Some(card);
                self.log(vec![express!(faction, " discard ", card).with_fact(fact).into()]);
            }
            // List.Remove: the first equal element
            let cards = &mut self.player_mut(player).treachery_cards;
            if let Some(pos) = cards.iter().position(|c| *c == card) {
                cards.remove(pos);
            }
            self.decks.treachery_discard_pile.as_mut().expect("treachery discard pile").put_on_top(card);
            self.register_known(card);
            self.decks.recently_discarded.insert_new(card, faction);
            self.stone(Milestone::Discard);

            if !matches!(
                card.type_(),
                TreacheryCardType::Poison | TreacheryCardType::ProjectileAndPoison | TreacheryCardType::PoisonTooth
            ) {
                return;
            }

            let ecaz = self.get_player(Some(Faction::Ecaz));
            let Some(ecaz) = ecaz else { return };
            if !self.player(ecaz).has_any_high_threshold(self) {
                return;
            }
            self.log(vec![
                Faction::Ecaz.into(),
                " get ".into(),
                Payment::of(3).into(),
                " from the discarded ".into(),
                card.into(),
            ]);
            let p = self.player_mut(ecaz);
            p.resources += 3;
        }
    }
}
