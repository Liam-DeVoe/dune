//! Port of TreacheryCardManager.cs.

use crate::cs::CsIterExt;
use crate::deck::Deck;
use crate::enumerations::{Rule, TreacheryCardType};
use crate::i_fetcher::IFetcher;
use crate::logged_random::SharedRandom;
use crate::map::MapGame;
use crate::treachery_card::{TreacheryCard, TreacheryCardData};

pub struct TreacheryCardManager;

/// `TreacheryCardManager.Items`, in order.
pub(crate) static ITEMS: [TreacheryCardData; 63] = [
    // Basic cards
    TreacheryCardData { id: 0, skin_id: 0, type_: TreacheryCardType::Laser, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 1, skin_id: 1, type_: TreacheryCardType::Projectile, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 2, skin_id: 2, type_: TreacheryCardType::Projectile, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 3, skin_id: 3, type_: TreacheryCardType::Projectile, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 4, skin_id: 4, type_: TreacheryCardType::Projectile, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 5, skin_id: 5, type_: TreacheryCardType::Poison, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 6, skin_id: 6, type_: TreacheryCardType::Poison, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 7, skin_id: 7, type_: TreacheryCardType::Poison, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 8, skin_id: 8, type_: TreacheryCardType::Poison, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 9, skin_id: 9, type_: TreacheryCardType::Shield, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 10, skin_id: 9, type_: TreacheryCardType::Shield, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 11, skin_id: 9, type_: TreacheryCardType::Shield, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 12, skin_id: 9, type_: TreacheryCardType::Shield, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 13, skin_id: 13, type_: TreacheryCardType::Antidote, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 14, skin_id: 13, type_: TreacheryCardType::Antidote, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 15, skin_id: 13, type_: TreacheryCardType::Antidote, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 16, skin_id: 13, type_: TreacheryCardType::Antidote, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 17, skin_id: 17, type_: TreacheryCardType::Mercenary, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 18, skin_id: 17, type_: TreacheryCardType::Mercenary, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 19, skin_id: 19, type_: TreacheryCardType::Mercenary, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 20, skin_id: 20, type_: TreacheryCardType::RaiseDead, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 21, skin_id: 21, type_: TreacheryCardType::Metheor, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 22, skin_id: 22, type_: TreacheryCardType::Caravan, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 23, skin_id: 23, type_: TreacheryCardType::Karma, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 24, skin_id: 23, type_: TreacheryCardType::Karma, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 25, skin_id: 25, type_: TreacheryCardType::Clairvoyance, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 26, skin_id: 25, type_: TreacheryCardType::Clairvoyance, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: 27, skin_id: 27, type_: TreacheryCardType::StormSpell, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: TreacheryCardManager::CARD_BALISET, skin_id: TreacheryCardManager::CARD_BALISET, type_: TreacheryCardType::Useless, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: TreacheryCardManager::CARD_JUBBA_CLOAK, skin_id: TreacheryCardManager::CARD_JUBBA_CLOAK, type_: TreacheryCardType::Useless, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: TreacheryCardManager::CARD_KULON, skin_id: TreacheryCardManager::CARD_KULON, type_: TreacheryCardType::Useless, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: TreacheryCardManager::CARD_LALALA, skin_id: TreacheryCardManager::CARD_LALALA, type_: TreacheryCardType::Useless, rules: &[Rule::BasicTreacheryCards] },
    TreacheryCardData { id: TreacheryCardManager::CARD_TRIP_TO_GAMONT, skin_id: TreacheryCardManager::CARD_TRIP_TO_GAMONT, type_: TreacheryCardType::Useless, rules: &[Rule::BasicTreacheryCards] },
    // Ixian & Tleilaxu Expansion Treachery Cards
    TreacheryCardData { id: 33, skin_id: 33, type_: TreacheryCardType::ProjectileAndPoison, rules: &[Rule::ExpansionTreacheryCardsPBandSs] },
    TreacheryCardData { id: 34, skin_id: 34, type_: TreacheryCardType::Projectile, rules: &[Rule::ExpansionTreacheryCardsExceptPBandSSandAmal] },
    TreacheryCardData { id: 35, skin_id: 35, type_: TreacheryCardType::Poison, rules: &[Rule::ExpansionTreacheryCardsExceptPBandSSandAmal] },
    TreacheryCardData { id: 36, skin_id: 36, type_: TreacheryCardType::WeirdingWay, rules: &[Rule::ExpansionTreacheryCardsExceptPBandSSandAmal] },
    TreacheryCardData { id: 37, skin_id: 37, type_: TreacheryCardType::PoisonTooth, rules: &[Rule::ExpansionTreacheryCardsExceptPBandSSandAmal, Rule::Expansion2TreacheryCards] },
    TreacheryCardData { id: 38, skin_id: 38, type_: TreacheryCardType::ShieldAndAntidote, rules: &[Rule::ExpansionTreacheryCardsPBandSs] },
    TreacheryCardData { id: 39, skin_id: 9, type_: TreacheryCardType::Shield, rules: &[Rule::ExpansionTreacheryCardsExceptPBandSSandAmal] },
    TreacheryCardData { id: 40, skin_id: 13, type_: TreacheryCardType::Antidote, rules: &[Rule::ExpansionTreacheryCardsExceptPBandSSandAmal] },
    TreacheryCardData { id: 41, skin_id: 39, type_: TreacheryCardType::Chemistry, rules: &[Rule::ExpansionTreacheryCardsExceptPBandSSandAmal] },
    TreacheryCardData { id: 42, skin_id: 40, type_: TreacheryCardType::ArtilleryStrike, rules: &[Rule::ExpansionTreacheryCardsExceptPBandSSandAmal, Rule::Expansion2TreacheryCards] },
    TreacheryCardData { id: 43, skin_id: 41, type_: TreacheryCardType::Harvester, rules: &[Rule::ExpansionTreacheryCardsExceptPBandSSandAmal] },
    TreacheryCardData { id: 44, skin_id: 42, type_: TreacheryCardType::Thumper, rules: &[Rule::ExpansionTreacheryCardsExceptPBandSSandAmal] },
    TreacheryCardData { id: 45, skin_id: 43, type_: TreacheryCardType::Amal, rules: &[Rule::ExpansionTreacheryCardsAmal] },
    TreacheryCardData { id: TreacheryCardManager::CARD_KULL_WAHAD, skin_id: 44, type_: TreacheryCardType::Useless, rules: &[Rule::ExpansionTreacheryCardsExceptPBandSSandAmal] },
    // Richese Treachery Cards
    TreacheryCardData { id: 47, skin_id: 45, type_: TreacheryCardType::Distrans, rules: &[Rule::RicheseTreacheryCards] },
    TreacheryCardData { id: 48, skin_id: 46, type_: TreacheryCardType::Juice, rules: &[Rule::RicheseTreacheryCards] },
    TreacheryCardData { id: 49, skin_id: 47, type_: TreacheryCardType::MirrorWeapon, rules: &[Rule::RicheseTreacheryCards] },
    TreacheryCardData { id: 50, skin_id: 48, type_: TreacheryCardType::PortableAntidote, rules: &[Rule::RicheseTreacheryCards] },
    TreacheryCardData { id: 51, skin_id: 49, type_: TreacheryCardType::Flight, rules: &[Rule::RicheseTreacheryCards] },
    TreacheryCardData { id: 52, skin_id: 50, type_: TreacheryCardType::SearchDiscarded, rules: &[Rule::RicheseTreacheryCards] },
    TreacheryCardData { id: 53, skin_id: 51, type_: TreacheryCardType::TakeDiscarded, rules: &[Rule::RicheseTreacheryCards] },
    TreacheryCardData { id: 54, skin_id: 52, type_: TreacheryCardType::Residual, rules: &[Rule::RicheseTreacheryCards] },
    TreacheryCardData { id: 55, skin_id: 53, type_: TreacheryCardType::Rockmelter, rules: &[Rule::RicheseTreacheryCards] },
    TreacheryCardData { id: 56, skin_id: 54, type_: TreacheryCardType::Karma, rules: &[Rule::RicheseTreacheryCards] },
    // Expansion 3 Treachery Cards
    TreacheryCardData { id: 57, skin_id: 55, type_: TreacheryCardType::Recruits, rules: &[Rule::Expansion3TreacheryCards] },
    TreacheryCardData { id: 58, skin_id: 56, type_: TreacheryCardType::Reinforcements, rules: &[Rule::Expansion3TreacheryCards] },
    TreacheryCardData { id: 59, skin_id: 57, type_: TreacheryCardType::HarassAndWithdraw, rules: &[Rule::Expansion3TreacheryCards] },
    // 3 extra karma cards
    TreacheryCardData { id: 100, skin_id: 23, type_: TreacheryCardType::Karma, rules: &[Rule::ExtraKaramaCards] },
    TreacheryCardData { id: 101, skin_id: 23, type_: TreacheryCardType::Karma, rules: &[Rule::ExtraKaramaCards] },
    TreacheryCardData { id: 102, skin_id: 23, type_: TreacheryCardType::Karma, rules: &[Rule::ExtraKaramaCards] },
];

static ITEM_HANDLES: [TreacheryCard; 63] = {
    let mut a = [TreacheryCard::from_index(0); 63];
    let mut i = 0;
    while i < 63 {
        a[i] = TreacheryCard::from_index(i);
        i += 1;
    }
    a
};

/// What `TreacheryCardManager.GetCardsInPlay(Game g)` reads from the game.
pub trait CardsGame {
    /// `g.Applicable(rule)`
    fn applicable(&self, rule: Rule) -> bool;
}

impl<T: MapGame> CardsGame for T {
    fn applicable(&self, rule: Rule) -> bool {
        MapGame::applicable(self, rule)
    }
}

impl TreacheryCardManager {
    pub const CARD_BALISET: i32 = 28;
    pub const CARD_JUBBA_CLOAK: i32 = 29;
    pub const CARD_KULON: i32 = 30;
    pub const CARD_LALALA: i32 = 31;
    pub const CARD_TRIP_TO_GAMONT: i32 = 32;
    pub const CARD_KULL_WAHAD: i32 = 46;

    /// `TreacheryCardManager.Items`
    pub fn items() -> &'static [TreacheryCard] {
        &ITEM_HANDLES
    }

    /// `TreacheryCardManager.Lookup`
    pub fn lookup() -> TreacheryCardFetcher {
        TreacheryCardFetcher
    }

    /// `CreateTreacheryDeck(g, random)`
    pub fn create_treachery_deck(g: &impl CardsGame, random: Option<SharedRandom>) -> Deck<TreacheryCard> {
        Deck::new(Self::get_cards_in_play(g), random)
    }

    /// `GetRicheseCards()`
    pub fn get_richese_cards() -> Vec<TreacheryCard> {
        Self::get_cards_in_play_by_rule(Rule::RicheseTreacheryCards).collect()
    }

    /// `GetCardsInPlay(Game g)`
    /// Lazy like the C# query: callers stop early (docs/lazy-queries.md).
    pub fn get_cards_in_play(g: &impl CardsGame) -> impl Iterator<Item = TreacheryCard> {
        Self::items()
            .iter()
            .copied()
            .filter(move |c| {
                c.rules().iter().any(|r| g.applicable(*r))
            })
    }

    /// `GetCardsInPlay(Rule rule)` (private in C#)
    pub fn get_cards_in_play_by_rule(rule: Rule) -> impl Iterator<Item = TreacheryCard> {
        Self::items().iter().copied().filter(move |c| c.rules().contains(&rule))
    }

    /// `GetCardsInAndOutsidePlay()`
    pub fn get_cards_in_and_outside_play() -> &'static [TreacheryCard] {
        Self::items()
    }

    /// `Get(id)`
    pub fn get(id: i32) -> Option<TreacheryCard> {
        Self::items().iter().copied().cs_single_or_default_where(|i| i.id() == id)
    }

    /// `GetId(value)`
    pub fn get_id(value: Option<TreacheryCard>) -> i32 {
        let value = value.and_then(crate::cs::CsNull::opt);
        value.and_then(crate::cs::CsNull::opt).map_or(-1, |c| c.id())
    }
}

/// `TreacheryCardManager.TreacheryCardFetcher`
#[derive(Clone, Copy, Debug)]
pub struct TreacheryCardFetcher;

impl IFetcher<TreacheryCard> for TreacheryCardFetcher {
    fn find(&self, id: i32) -> Option<TreacheryCard> {
        if id < -1 {
            return None;
        }
        TreacheryCardManager::items().iter().copied().cs_single_or_default_where(|t| t.id() == id)
    }

    fn get_id(&self, obj: Option<TreacheryCard>) -> i32 {
        let obj = obj.and_then(crate::cs::CsNull::opt);
        obj.and_then(crate::cs::CsNull::opt).map_or(-1, |c| c.id())
    }
}
