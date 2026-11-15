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
