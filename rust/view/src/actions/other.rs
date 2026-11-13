//! The forms of the events outside bidding, ship and move, battle, revival and setup, and every
//! event drawn as a simple confirm form or a yes/no form.
//!
//! `AllianceOffered`, `AlliancePassed`, `AllianceBroken` and `StormDialled` are built in
//! `actions.rs`, which had them first. `NexusPlayed` is in `other_nexus.rs`.
//!
//! As in `revival.rs`, a form adjusts its inputs: a required image choice or select falls back
//! to its first value, a number picker clamps into its range, a widget that is not drawn resets
//! its field. [`normalize`] does that to a draft, so the validation next to the options is that
//! of the inputs the form shows, and the options carry the adjusted draft back to the client.

use treachery_engine::events::GameEventType;
use treachery_engine::messages::expression::ExpressionElement;
use treachery_engine::json;
use treachery_engine::enum_type::GameEnum;
use treachery_engine::prelude::*;
use treachery_engine::express;

use crate::build::{event, validate, ResolvedViewer};
use crate::describe::Describer;
use crate::expr::message;
use crate::schema::*;

use super::other_nexus;
use super::widgets::{picker, required, faction_options, hero_options, card_ids, draft_json, describe};

/// The event types drawn as a simple confirm form (`ActionOptions::Simple`).
pub const SIMPLE: [GameEventType; 19] = [
    GameEventType::MetheorPlayed,
    GameEventType::CharityClaimed,
    GameEventType::GuildDelay,
    GameEventType::BattleRevision,
    GameEventType::KarmaPrescience,
    GameEventType::KarmaEcazDial,
    GameEventType::AmalPlayed,
    GameEventType::RecruitsPlayed,
    GameEventType::HarvesterPlayed,
    GameEventType::PoisonToothCancelled,
    GameEventType::ThumperPlayed,
    GameEventType::ExtortionPrevented,
    GameEventType::ChoamExtraMove,
    GameEventType::RicheseRevealedNoField,
    GameEventType::ResidualPlayed,
    GameEventType::FlightDiscoveryUsed,
    GameEventType::DiscardedSearchedAnnounced,
    GameEventType::PortableAntidoteUsed,
    GameEventType::SwitchedSkilledLeader,
];

/// The event types drawn as a yes/no form (`ActionOptions::YesNo`).
pub const YES_NO: [GameEventType; 8] = [
    GameEventType::NexusVoted,
    GameEventType::AllianceByTerror,
    GameEventType::AllianceByAmbassador,
    GameEventType::BattleClaimed,
    GameEventType::DivideResourcesAccepted,
    GameEventType::CardGiven,
    GameEventType::Bureaucracy,
    GameEventType::Captured,
];

/// The event types with a form of their own in this module.
pub const FORMS: [GameEventType; 42] = [
    GameEventType::AllyPermission,
    GameEventType::AutomationConfigured,
    GameEventType::BeneGesseritPrediction,
    GameEventType::CardTraded,
    GameEventType::ChoamDiscarded,
    GameEventType::ChoamEconomics,
    GameEventType::ChoamKarmaPrevention,
    GameEventType::DealOffered,
    GameEventType::Discarded,
    GameEventType::DiscardedSearched,
    GameEventType::DiscardedTaken,
    GameEventType::DistransUsed,
    GameEventType::DivideResources,
    GameEventType::Donated,
    GameEventType::EmperorBidSupport,
    GameEventType::EmperorDiscarded,
    GameEventType::FactionSelected,
    GameEventType::FactionTradeOffered,
    GameEventType::IxianRemovedCardFromAuction,
    GameEventType::IxianSelectedStartingCard,
    GameEventType::IxianSwappedCardOnBid,
    GameEventType::Karma,
    GameEventType::KarmaChoamDiscard,
    GameEventType::KarmaHandSwap,
    GameEventType::KarmaHandSwapInitiated,
    GameEventType::KarmaMonster,
    GameEventType::KarmaRicheseBuy,
    GameEventType::MulliganPerformed,
    GameEventType::NexusCardDrawn,
    GameEventType::NexusPlayed,
    GameEventType::ReplacedCardWon,
    GameEventType::ResourcesAudited,
    GameEventType::ResourcesTransferred,
    GameEventType::SkillAssigned,
    GameEventType::StormSpellPlayed,
    GameEventType::TraitorDiscarded,
    GameEventType::TraitorsSelected,
    GameEventType::RicheseAnnouncesAuction,
    GameEventType::RicheseAnnouncesBlackMarket,
    GameEventType::RicheseGaveCard,
    GameEventType::RicheseKeepsUnsoldCard,
    GameEventType::RicheseSpecifiesAuction,
];

fn handles(t: GameEventType) -> bool {
    SIMPLE.contains(&t) || YES_NO.contains(&t) || FORMS.contains(&t)
}

// ---------------------------------------------------------------------------------------------
// Widget rules and helpers shared with other_nexus.rs.

/// `Skin.Format(m, args)`: `string.Format` over the described arguments; `m` itself when that
/// fails.
pub(super) fn format(d: &dyn Describer, m: &str, args: &[ExpressionElement]) -> String {
    let described: Vec<String> = args.iter().map(|a| d.describe_in(a, None)).collect();
    let refs: Vec<Option<&str>> = described.iter().map(|s| Some(s.as_str())).collect();
    dotnet_compat::fmt::string_format(m, &refs).unwrap_or_else(|_| m.to_string())
}

pub(super) fn enum_option<T: GameEnum + Into<ExpressionElement> + Copy + ToString>(d: &dyn Describer, v: T) -> EnumOption {
    EnumOption { name: v.to_string(), value: v.value(), label: describe(d, v) }
}

fn labelled<T: GameEnum + Copy + ToString>(v: T, label: &str) -> EnumOption {
    EnumOption { name: v.to_string(), value: v.value(), label: label.to_string() }
}

/// A plain `Title`: one text part.
fn text(s: impl Into<String>) -> RichText {
    vec![Part::Text { v: s.into() }]
}

/// `Message.ToString(Skin)`: the rendered parts as one string.
fn plain(rt: &RichText) -> String {
    rt.iter()
        .map(|p| match p {
            Part::Text { v } => v.clone(),
            Part::None => "none".to_string(),
            Part::Num { v } => v.to_string(),
            Part::Bool { label, .. }
            | Part::Faction { label, .. }
            | Part::Enum { label, .. }
            | Part::Location { label, .. }
            | Part::Territory { label, .. }
            | Part::Leader { label, .. }
            | Part::Messiah { label }
            | Part::TreacheryCard { label, .. }
            | Part::ResourceCard { label, .. } => label.clone(),
            Part::Payment { amount, .. } => amount.to_string(),
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Default drafts.

/// The first draft of the form, before the player touches it: its starting inputs (adjusted by
/// [`normalize`] when the options are built). `None` for event types of other modules.
pub fn default_draft(game: &Game, faction: Faction, t: GameEventType) -> Option<GameEvent> {
    if !handles(t) {
        return None;
    }
    let p = game.get_player(Some(faction))?;
    let pl = game.player(p);
    Some(match t {
        GameEventType::AllyPermission => {
            // OnParametersSet while not edited: the permissions in force.
            let mut e = AllyPermission::new(game, faction);
            current_permissions(game, faction, &mut e);
            event(e)
        }
        GameEventType::AutomationConfigured => {
            let mut e = AutomationConfigured::new(game, faction);
            e.action = ItemAction::Create;
            event(e)
        }
        GameEventType::CardTraded => {
            let mut e = CardTraded::new(game, faction);
            e.target = pl.ally;
            event(e)
        }
        GameEventType::ChoamEconomics => {
            let mut e = ChoamEconomics::new(game, faction);
            e.status = ChoamEconomicsStatus::Double;
            event(e)
        }
        GameEventType::DealOffered => {
            let mut e = DealOffered::new(game, faction);
            // Every target picked: the offer goes to everyone (To is empty).
            e.to = Some(vec![]);
            e.text = Some(String::new());
            event(e)
        }
        GameEventType::KarmaMonster => {
            // Nothing picked on the map yet.
            let mut e = KarmaMonster::new(game, faction);
            e.set_territory(game, None);
            event(e)
        }
        GameEventType::EmperorBidSupport => {
            let mut e = EmperorBidSupport::new(game, faction);
            let mut amounts = IndexMap::new();
            for f in opponents_but_ally(game, p) {
                amounts.insert(f, game.spice_for_bids_emperor_can_pay(f));
            }
            e.amounts = Some(amounts);
            event(e)
        }
        GameEventType::FactionSelected => {
            // Named and seated as the player.
            let mut e = FactionSelected::new(game);
            e.initiator_player_name = Some(game.get_player_name_player(p));
            e.seat = pl.seat;
            event(e)
        }
        GameEventType::NexusPlayed => other_nexus::default_draft(game, faction),
        GameEventType::RicheseAnnouncesAuction => {
            let mut e = RicheseAnnouncesAuction::new(game, faction);
            e.first = true;
            event(e)
        }
        GameEventType::RicheseAnnouncesBlackMarket => {
            let mut e = RicheseAnnouncesBlackMarket::new(game, faction);
            e.auction_type = AuctionType::BlackMarketNormal;
            e.direction = 1;
            event(e)
        }
        GameEventType::RicheseSpecifiesAuction => {
            let mut e = RicheseSpecifiesAuction::new(game, faction);
            e.auction_type = AuctionType::RicheseOnceAround;
            e.direction = 1;
            event(e)
        }
        _ => {
            // A fresh event of the type, initialized for the faction.
            let disc = t.json_discriminator().unwrap_or(t.name());
            let e: GameEvent = json::deserialize(&format!(r#"{{"$type":"{disc}"}}"#)).ok()?;
            e.initialize(game, faction);
            e
        }
    })
}

/// The permissions in force, which the ally permission form starts from.
fn current_permissions(game: &Game, faction: Faction, e: &mut AllyPermission) {
    let ally = game.player(game.get_player(Some(faction)).expect("player")).ally;
    e.permitted_resources = game.get_permitted_use_of_ally_resources(ally);
    e.set_permitted_karma_card(game.get_permitted_use_of_ally_karma(ally));
    match faction {
        Faction::Tleilaxu => e.set_tleilaxu_allows_revival_discount(game.tleilaxu_allows_revival_discount),
        Faction::Ixian => e.set_ixian_allows_replacing_cards(game.ixian_allows_replacing_cards),
        Faction::Guild => e.set_guild_allows_shipping_discount(game.guild_allows_shipping_discount),
        Faction::Emperor => e.emperor_will_pay_for_extra_revival = game.emperor_will_pay_for_extra_revival,
        Faction::Fremen => {
            e.fremen_will_protect_from_monster = game.fremen_will_protect_from_monster;
            e.fremen_allows_three_free_revivals = game.fremen_allows_three_free_revivals;
            e.fremen_shares_prescience = game.fremen_shares_prescience;
            e.fremen_refunds_battle_dial = game.fremen_refunds_battle_dial;
        }
        Faction::Atreides => e.atreides_shares_prescience = game.atreides_shares_prescience,
        Faction::BeneGesserit => e.bene_gesserit_allows_use_of_voice = game.bene_gesserit_allows_use_of_voice,
        Faction::Richese => e.richese_allows_use_of_no_field = game.richese_allows_use_of_no_field,
        Faction::Moritani => e.moritani_allows_keeping_cards = game.moritani_allows_keeping_cards,
        Faction::Ecaz => e.ecaz_shares_ambassadors = game.ecaz_shares_ambassadors,
        _ => {}
    }
}

/// The opponents other than the ally: who Emperor may support in bidding, and who a deal may be
/// offered to.
fn opponents_but_ally(game: &Game, p: PlayerIndex) -> Vec<Faction> {
    let ally = game.player(p).ally;
    game.players_other_than(p).filter(|f| *f != ally).collect()
}

/// The phases a deal offer may last until.
fn deal_end_phases(t: DealType) -> Vec<Phase> {
    match t {
        DealType::ShareStormPrescience => vec![Phase::TurnConcluded],
        DealType::ShareBiddingPrescience => vec![Phase::Bidding, Phase::BiddingReport],
        DealType::ShareResourceDeckPrescience => vec![Phase::TurnConcluded],
        _ => vec![Phase::TurnConcluded, Phase::GameEnded],
    }
}

// ---------------------------------------------------------------------------------------------
// Adjusting a draft as the form does.

/// The draft as the form holds it after adjusting these inputs.
pub fn normalize(game: &Game, p: PlayerIndex, v: &ResolvedViewer, draft: &GameEvent) -> GameEvent {
    let pl = game.player(p);
    let hand: Vec<i32> = card_ids(pl.treachery_cards.iter().copied());
    let none_card = TreacheryCardManager::get_id(None);
    macro_rules! adjust {
        ($e:expr, |$m:ident| $body:block) => {{
            let mut $m = (**$e).clone();
            $body
            event($m)
        }};
    }
    match draft {
        GameEvent::AutomationConfigured(e) => adjust!(e, |m| {
            let types = AutomationConfigured::get_available_rule_types(game, p);
            m.rule_type = required(m.rule_type, &types, AutomationRuleType::default());
            m.action = ItemAction::Create;
            match m.rule_type {
                AutomationRuleType::BiddingPassAboveAmount => m.bidding_above_amount = picker(m.bidding_above_amount, 0, 40),
                AutomationRuleType::BiddingPassWhenHighestBidByFaction => {
                    let fs = AutomationConfigured::get_valid_bidding_factions(game, p);
                    m.bidding_winning_faction = required(m.bidding_winning_faction, &fs, Faction::None);
                }
                _ => {}
            }
        }),
        GameEvent::AllyPermission(e) => adjust!(e, |m| {
            m.permitted_resources = picker(m.permitted_resources, 0, pl.resources);
            if !Karma::can_karma(game, p) {
                m.set_permitted_karma_card(None);
            }
            if pl.faction == Faction::Emperor {
                m.emperor_will_pay_for_extra_revival = picker(m.emperor_will_pay_for_extra_revival, 0, 3);
            }
        }),
        GameEvent::BeneGesseritPrediction(e) => adjust!(e, |m| {
            let targets: Vec<Faction> = BeneGesseritPrediction::valid_targets(game, p).collect();
            m.to_win = required(m.to_win, &targets, Faction::None);
            m.turn = picker(m.turn, 1, game.maximum_turns());
        }),
        GameEvent::CardTraded(e) => adjust!(e, |m| {
            m.target = pl.ally;
            let cards = card_ids(CardTraded::valid_cards(game, p));
            m.card_id = required(m.card_id, &cards, none_card);
            match pl.allied_player(game) {
                // An optional image choice keeps whatever card was asked for.
                Some(a) if game.is_bot(a) => {}
                _ => m.requested_card_id = none_card,
            }
        }),
        GameEvent::ChoamDiscarded(e) => adjust!(e, |m| {
            m.card_id = required(m.card_id, &card_ids(ChoamDiscarded::valid_cards(game, p)), none_card);
        }),
        GameEvent::ChoamEconomics(e) => adjust!(e, |m| {
            m.status = required(m.status, &[ChoamEconomicsStatus::Double, ChoamEconomicsStatus::Cancel], ChoamEconomicsStatus::default());
        }),
        GameEvent::ChoamKarmaPrevention(e) => adjust!(e, |m| {
            let targets: Vec<Faction> = game.players_other_than(p).collect();
            m.target = required(m.target, &targets, Faction::None);
        }),
        GameEvent::DealOffered(e) => adjust!(e, |m| {
            let types = DealOffered::get_standard_deal_types(game, p);
            m.type_ = required(m.type_, &types, DealType::None);
            m.end_phase = required(m.end_phase, &deal_end_phases(m.type_), Phase::default());
            m.price = picker(m.price, 0, 50);
            m.benefit = picker(m.benefit, 0, 50);
        }),
        GameEvent::Discarded(e) => adjust!(e, |m| {
            m.card_id = required(m.card_id, &hand, none_card);
        }),
        GameEvent::DiscardedSearched(e) => adjust!(e, |m| {
            m.card_id = required(m.card_id, &card_ids(DiscardedSearched::valid_cards(game)), none_card);
        }),
        GameEvent::DiscardedTaken(e) => adjust!(e, |m| {
            m.card_id = required(m.card_id, &card_ids(DiscardedTaken::valid_cards(game, p)), none_card);
        }),
        GameEvent::DistransUsed(e) => adjust!(e, |m| {
            let targets: Vec<Faction> = DistransUsed::valid_targets(game, p).collect();
            m.target = required(m.target, &targets, Faction::None);
            m.card_id = required(m.card_id, &card_ids(DistransUsed::valid_cards(game, p)), none_card);
        }),
        GameEvent::DivideResources(e) => adjust!(e, |m| {
            let amount = DivideResources::get_resources_to_be_divided(game).map_or(0, |t| t.amount);
            m.portion_to_first_player = picker(m.portion_to_first_player, 0, amount);
        }),
        GameEvent::Donated(e) => adjust!(e, |m| {
            if !v.is_host {
                m.from_bank = false;
            }
            let (targets, min, max) = donation_range(game, p, m.from_bank);
            m.target = required(m.target, &targets, Faction::None);
            m.resources = picker(m.resources, min, max);
            if !game.applicable(Rule::CardsCanBeTraded) {
                m.card_id = none_card;
            }
        }),
        GameEvent::EmperorBidSupport(e) => adjust!(e, |m| {
            if let Some(amounts) = m.amounts.as_mut() {
                for (_, a) in amounts.iter_mut() {
                    *a = picker(*a, 0, pl.resources);
                }
            }
        }),
        GameEvent::EmperorDiscarded(e) => adjust!(e, |m| {
            m.card_id = required(m.card_id, &hand, none_card);
        }),
        GameEvent::FactionSelected(e) => adjust!(e, |m| {
            let fs: Vec<Faction> = FactionSelected::valid_factions(game).collect();
            m.faction = required(m.faction, &fs, Faction::None);
            m.seat = pl.seat;
            m.initiator_player_name = Some(game.get_player_name_player(p));
        }),
        GameEvent::FactionTradeOffered(e) => adjust!(e, |m| {
            let targets: Vec<Faction> = FactionTradeOffered::valid_targets(game, p).collect();
            m.target = required(m.target, &targets, Faction::None);
        }),
        GameEvent::IxianRemovedCardFromAuction(e) => adjust!(e, |m| {
            let cards = card_ids(game.cards_on_auction.as_ref().map(|d| d.items.to_vec()).unwrap_or_default());
            m.card_id = required(m.card_id, &cards, none_card);
        }),
        GameEvent::IxianSelectedStartingCard(e) => adjust!(e, |m| {
            let cards = card_ids(game.starting_treachery_cards.as_ref().map(|d| d.items.to_vec()).unwrap_or_default());
            m.card_id = required(m.card_id, &cards, none_card);
        }),
        GameEvent::IxianSwappedCardOnBid(e) => adjust!(e, |m| {
            m.passed = false;
            m.card_id = required(m.card_id, &hand, none_card);
        }),
        GameEvent::Karma(e) => adjust!(e, |m| {
            m.card_id = required(m.card_id, &card_ids(Karma::valid_karma_cards(game, p)), none_card);
            m.prevented = required(m.prevented, &Karma::valid_faction_advantages(game, p), FactionAdvantage::default());
        }),
        GameEvent::KarmaChoamDiscard(e) => adjust!(e, |m| {
            let karma = card_ids(Karma::valid_karma_cards(game, p));
            let before = m.card_to_use;
            // More than one: a required picker. Otherwise `CardToUse = ValidKarmaCards.FirstOrDefault()`.
            m.card_to_use = if karma.len() > 1 { required(m.card_to_use, &karma, none_card) } else { karma.first().copied().unwrap_or(none_card) };
            if m.card_to_use != before && karma.len() > 1 && karma.contains(&before) {
                // ChangeCardToUse clears the selection.
                m.set_cards(&[]);
            }
        }),
        GameEvent::KarmaHandSwapInitiated(e) => adjust!(e, |m| {
            let targets: Vec<Faction> = game.players_other_than(p).collect();
            m.target = required(m.target, &targets, Faction::None);
        }),
        GameEvent::KarmaRicheseBuy(e) => adjust!(e, |m| {
            m.card_id = required(m.card_id, &card_ids(game.richese_cache.iter().copied()), none_card);
        }),
        GameEvent::MulliganPerformed(e) => adjust!(e, |m| {
            m.passed = false;
        }),
        GameEvent::NexusCardDrawn(e) => adjust!(e, |m| {
            m.passed = false;
        }),
        GameEvent::NexusPlayed(_) => other_nexus::normalize(game, p, draft),
        GameEvent::ReplacedCardWon(e) => adjust!(e, |m| {
            m.passed = false;
        }),
        GameEvent::ResourcesAudited(e) => adjust!(e, |m| {
            let targets: Vec<Faction> = ResourcesAudited::valid_factions(game, p).collect();
            m.target = required(m.target, &targets, Faction::None);
        }),
        GameEvent::ResourcesTransferred(e) => adjust!(e, |m| {
            m.resources = picker(m.resources, 0, ResourcesTransferred::max_amount(game, p));
        }),
        GameEvent::SkillAssigned(e) => adjust!(e, |m| {
            m.passed = false;
            m.skill = required(m.skill, &SkillAssigned::valid_skills(game, p), LeaderSkill::None);
            let leaders: Vec<i32> = SkillAssigned::valid_leaders(game, p).into_iter().map(|l| l.map_or(-1, |l| l.id())).collect();
            m.leader_id = required(m.leader_id, &leaders, -1);
        }),
        GameEvent::StormSpellPlayed(e) => adjust!(e, |m| {
            m.move_amount = picker(m.move_amount, 0, 10);
        }),
        GameEvent::TraitorDiscarded(e) => adjust!(e, |m| {
            let heroes: Vec<i32> = TraitorDiscarded::valid_traitors(game, p).into_iter().map(|h| h.id()).collect();
            m.traitor_id = required(m.traitor_id, &heroes, -1);
        }),
        GameEvent::TraitorsSelected(e) => adjust!(e, |m| {
            let heroes: Vec<i32> = pl.traitors.iter().map(|h| h.id()).collect();
            m.traitor_id = required(m.traitor_id, &heroes, -1);
        }),
        GameEvent::RicheseAnnouncesAuction(e) => adjust!(e, |m| {
            m.first = true;
        }),
        GameEvent::RicheseAnnouncesBlackMarket(e) => adjust!(e, |m| {
            m.passed = false;
            m.card_id = required(m.card_id, &card_ids(RicheseAnnouncesBlackMarket::valid_cards(game, p)), none_card);
            let types = [AuctionType::BlackMarketNormal, AuctionType::BlackMarketOnceAround, AuctionType::BlackMarketSilent];
            m.auction_type = required(m.auction_type, &types, AuctionType::None);
            if m.auction_type == AuctionType::BlackMarketOnceAround {
                m.direction = required(m.direction, &[1, -1], 0);
            }
        }),
        GameEvent::RicheseGaveCard(e) => adjust!(e, |m| {
            m.card_id = required(m.card_id, &card_ids(RicheseGaveCard::valid_cards(game, p)), none_card);
        }),
        GameEvent::RicheseKeepsUnsoldCard(e) => adjust!(e, |m| {
            m.passed = false;
        }),
        GameEvent::RicheseSpecifiesAuction(e) => adjust!(e, |m| {
            if RicheseSpecifiesAuction::may_specify_card(game, p) {
                m.card_id = required(m.card_id, &card_ids(RicheseSpecifiesAuction::valid_cards(game)), none_card);
            }
            if RicheseSpecifiesAuction::may_specify_type_of_auction(game, p) {
                let types = [AuctionType::RicheseOnceAround, AuctionType::RicheseSilent];
                m.auction_type = required(m.auction_type, &types, AuctionType::None);
                if m.auction_type == AuctionType::RicheseOnceAround {
                    m.direction = required(m.direction, &[1, -1], 0);
                }
            }
        }),
        _ => draft.clone(),
    }
}

/// Who may receive a donation, and its minimum and maximum amount.
fn donation_range(game: &Game, p: PlayerIndex, from_bank: bool) -> (Vec<Faction>, i32, i32) {
    if from_bank {
        (game.players.iter().map(|pl| pl.faction).collect(), -99, 99)
    } else {
        (Donated::valid_targets(game, p).collect(), Donated::min_amount(game, p), Donated::max_amount(game, p))
    }
}

// ---------------------------------------------------------------------------------------------
// Options.

/// The options and validation of a form of this module for `draft`, or `None` for other event
/// types. The draft is first adjusted as the form would ([`normalize`]).
pub fn options_and_validation(game: &Game, v: &ResolvedViewer, draft: &GameEvent, d: &dyn Describer) -> Option<(Option<ActionOptions>, Option<RichText>)> {
    let t = draft.event_type();
    if !handles(t) {
        return None;
    }
    let p = v.player?;
    let draft = normalize(game, p, v, draft);
    let options = options(game, p, v, &draft, d);
    // The draft's `To` already says "everyone" when all are picked; picking none is the
    // client's check.
    Some((options, validate(game, draft, d)))
}

fn options(game: &Game, p: PlayerIndex, v: &ResolvedViewer, draft: &GameEvent, d: &dyn Describer) -> Option<ActionOptions> {
    let t = draft.event_type();
    if SIMPLE.contains(&t) {
        return simple_options(game, p, t, d);
    }
    if YES_NO.contains(&t) {
        return yes_no_options(game, p, draft, d);
    }
    let pl = game.player(p);
    let json = draft_json(draft);
    let hand = || card_ids(pl.treachery_cards.iter().copied());
    let cards = |cards: Vec<i32>| CardChoiceOptions { cards, draft: json.clone() };
    let factions = |fs: Vec<Faction>| FactionChoiceOptions { targets: faction_options(fs), draft: json.clone() };
    let draft_only = || DraftOptions { draft: json.clone() };
    Some(match draft {
        GameEvent::AllyPermission(_) => ActionOptions::AllyPermission(ally_permission(game, p, d, json.clone())),
        GameEvent::AutomationConfigured(_) => ActionOptions::AutomationConfigured(AutomationConfiguredOptions {
            rule_types: AutomationConfigured::get_available_rule_types(game, p).into_iter().map(|r| enum_option(d, r)).collect(),
            bidding_factions: faction_options(AutomationConfigured::get_valid_bidding_factions(game, p)),
            rules: game
                .automation_rules
                .iter()
                .filter(|r| r.base.initiator.get() == pl.faction)
                .map(|r| AutomationRule {
                    id: r.automation_rule_id.get(),
                    description: r.get_description(game).map(|m| message(&m, d)).unwrap_or_default(),
                })
                .collect(),
            draft: json.clone(),
        }),
        GameEvent::BeneGesseritPrediction(_) => ActionOptions::BeneGesseritPrediction(BeneGesseritPredictionOptions {
            targets: faction_options(BeneGesseritPrediction::valid_targets(game, p)),
            max_turn: game.maximum_turns(),
            draft: json.clone(),
        }),
        GameEvent::CardTraded(_) => ActionOptions::CardTraded(CardTradedOptions {
            cards: card_ids(CardTraded::valid_cards(game, p)),
            ally_cards: pl.allied_player(game).filter(|a| game.is_bot(*a)).map(|a| card_ids(CardTraded::valid_cards(game, a))),
            draft: json.clone(),
        }),
        GameEvent::ChoamDiscarded(_) => ActionOptions::ChoamDiscarded(cards(card_ids(ChoamDiscarded::valid_cards(game, p)))),
        GameEvent::ChoamEconomics(_) => ActionOptions::ChoamEconomics(ChoamEconomicsOptions {
            statuses: [ChoamEconomicsStatus::Double, ChoamEconomicsStatus::Cancel].into_iter().map(|s| enum_option(d, s)).collect(),
            draft: json.clone(),
        }),
        GameEvent::ChoamKarmaPrevention(_) => ActionOptions::ChoamKarmaPrevention(ChoamKarmaPreventionOptions {
            card: ChoamKarmaPrevention::card_to_use(game, p).map(|c| message(&express!(c), d)),
            targets: faction_options(game.players_other_than(p)),
            draft: json.clone(),
        }),
        GameEvent::DealOffered(_) => ActionOptions::DealOffered(DealOfferedOptions {
            targets: faction_options(opponents_but_ally(game, p)),
            deal_types: DealOffered::get_standard_deal_types(game, p)
                .into_iter()
                .map(|t| DealTypeOption {
                    value: t.value(),
                    label: plain(&message(&Deal::express(t, ExpressionElement::Null), d)),
                    end_phases: deal_end_phases(t).into_iter().map(|ph| enum_option(d, ph)).collect(),
                })
                .collect(),
            draft: json.clone(),
        }),
        GameEvent::Discarded(_) => ActionOptions::Discarded(cards(hand())),
        GameEvent::DiscardedSearched(_) => ActionOptions::DiscardedSearched(cards(card_ids(DiscardedSearched::valid_cards(game)))),
        GameEvent::DiscardedTaken(_) => ActionOptions::DiscardedTaken(cards(card_ids(DiscardedTaken::valid_cards(game, p)))),
        GameEvent::DistransUsed(_) => ActionOptions::DistransUsed(DistransUsedOptions {
            targets: faction_options(DistransUsed::valid_targets(game, p)),
            cards: card_ids(DistransUsed::valid_cards(game, p)),
            draft: json.clone(),
        }),
        GameEvent::DivideResources(e) => {
            let tbd = DivideResources::get_resources_to_be_divided(game)?;
            ActionOptions::DivideResources(DivideResourcesOptions {
                amount: tbd.amount,
                territory: tbd.territory.id(),
                other_faction: tbd.other_faction.to_string(),
                gained_if_passed: DivideResources::gained_by_first_faction(Some(&tbd), false, e.portion_to_first_player),
                other_gains_if_passed: DivideResources::gained_by_other_faction(Some(&tbd), false, e.portion_to_first_player),
                draft: json.clone(),
            })
        }
        GameEvent::Donated(e) => {
            let (targets, min, max) = donation_range(game, p, e.from_bank);
            ActionOptions::Donated(DonatedOptions {
                targets: faction_options(targets),
                min,
                max,
                cards: game.applicable(Rule::CardsCanBeTraded).then(hand),
                is_host: v.is_host,
                draft: json.clone(),
            })
        }
        GameEvent::EmperorBidSupport(_) => ActionOptions::EmperorBidSupport(EmperorBidSupportOptions {
            targets: faction_options(opponents_but_ally(game, p)),
            max: pl.resources,
            draft: json.clone(),
        }),
        GameEvent::EmperorDiscarded(_) => ActionOptions::EmperorDiscarded(cards(hand())),
        GameEvent::FactionSelected(_) => ActionOptions::FactionSelected(factions(FactionSelected::valid_factions(game).collect())),
        GameEvent::FactionTradeOffered(_) => {
            let targets: Vec<Faction> = FactionTradeOffered::valid_targets(game, p).collect();
            ActionOptions::FactionTradeOffered(FactionTradeOfferedOptions {
                in_play: targets.iter().filter(|f| game.players.iter().any(|pl| pl.faction == **f)).map(|f| f.to_string()).collect(),
                targets: faction_options(targets),
                draft: json.clone(),
            })
        }
        GameEvent::IxianRemovedCardFromAuction(_) => {
            ActionOptions::IxianRemovedCardFromAuction(cards(card_ids(game.cards_on_auction.as_ref().map(|d| d.items.to_vec()).unwrap_or_default())))
        }
        GameEvent::IxianSelectedStartingCard(_) => {
            ActionOptions::IxianSelectedStartingCard(cards(card_ids(game.starting_treachery_cards.as_ref().map(|d| d.items.to_vec()).unwrap_or_default())))
        }
        GameEvent::IxianSwappedCardOnBid(_) => ActionOptions::IxianSwappedCardOnBid(cards(hand())),
        GameEvent::Karma(_) => ActionOptions::Karma(KarmaOptions {
            cards: card_ids(Karma::valid_karma_cards(game, p)),
            advantages: Karma::valid_faction_advantages(game, p).into_iter().map(|a| enum_option(d, a)).collect(),
            draft: json.clone(),
        }),
        GameEvent::KarmaChoamDiscard(e) => ActionOptions::KarmaChoamDiscard(KarmaChoamDiscardOptions {
            karma_cards: card_ids(Karma::valid_karma_cards(game, p)),
            cards: hand().into_iter().filter(|c| *c != e.card_to_use).collect(),
            draft: json.clone(),
        }),
        GameEvent::KarmaHandSwap(_) => ActionOptions::KarmaHandSwap(KarmaHandSwapOptions {
            number: game.karma_hand_swap_number_of_cards,
            target: game.karma_hand_swap_target.to_string(),
            cards: hand(),
            draft: json.clone(),
        }),
        GameEvent::KarmaHandSwapInitiated(_) => ActionOptions::KarmaHandSwapInitiated(factions(game.players_other_than(p).collect())),
        GameEvent::KarmaMonster(_) => {
            let ts: Vec<Territory> = KarmaMonster::valid_targets(game).collect();
            ActionOptions::KarmaMonster(TerritoryChoiceOptions {
                territories: ts.iter().map(|t| t.id()).collect(),
                locations: ts.iter().flat_map(|t| t.locations().iter().map(|l| l.id())).collect(),
                draft: json.clone(),
            })
        }
        GameEvent::KarmaRicheseBuy(_) => ActionOptions::KarmaRicheseBuy(cards(card_ids(game.richese_cache.iter().copied()))),
        GameEvent::MulliganPerformed(_) => ActionOptions::MulliganPerformed(MayOptions { may: MulliganPerformed::may_mulligan(game, p), draft: json.clone() }),
        GameEvent::NexusCardDrawn(_) => ActionOptions::NexusCardDrawn(MayOptions { may: NexusCardDrawn::may_draw(game, p), draft: json.clone() }),
        GameEvent::NexusPlayed(e) => ActionOptions::NexusPlayed(other_nexus::options(game, p, e, d, json.clone())),
        GameEvent::ReplacedCardWon(_) => ActionOptions::ReplacedCardWon(draft_only()),
        GameEvent::ResourcesAudited(_) => ActionOptions::ResourcesAudited(factions(ResourcesAudited::valid_factions(game, p).collect())),
        GameEvent::ResourcesTransferred(_) => ActionOptions::ResourcesTransferred(ResourcesTransferredOptions {
            max: ResourcesTransferred::max_amount(game, p),
            draft: json.clone(),
        }),
        GameEvent::SkillAssigned(_) => ActionOptions::SkillAssigned(SkillAssignedOptions {
            skills: SkillAssigned::valid_skills(game, p).into_iter().map(|s| enum_option(d, s)).collect(),
            leaders: hero_options(SkillAssigned::valid_leaders(game, p).into_iter().flatten().map(Hero::Leader)),
            draft: json.clone(),
        }),
        GameEvent::StormSpellPlayed(_) => ActionOptions::StormSpellPlayed(draft_only()),
        GameEvent::TraitorDiscarded(_) => ActionOptions::TraitorDiscarded(HeroChoiceOptions {
            heroes: hero_options(TraitorDiscarded::valid_traitors(game, p)),
            draft: json.clone(),
        }),
        GameEvent::TraitorsSelected(_) => ActionOptions::TraitorsSelected(HeroChoiceOptions {
            heroes: hero_options(pl.traitors.iter().copied()),
            draft: json.clone(),
        }),
        GameEvent::RicheseAnnouncesAuction(_) => ActionOptions::RicheseAnnouncesAuction(draft_only()),
        GameEvent::RicheseAnnouncesBlackMarket(_) => ActionOptions::RicheseAnnouncesBlackMarket(RicheseAnnouncesBlackMarketOptions {
            cards: card_ids(RicheseAnnouncesBlackMarket::valid_cards(game, p)),
            auction_types: vec![
                labelled(AuctionType::BlackMarketNormal, "Normal"),
                labelled(AuctionType::BlackMarketOnceAround, "Once Around"),
                labelled(AuctionType::BlackMarketSilent, "Silent"),
            ],
            draft: json.clone(),
        }),
        GameEvent::RicheseGaveCard(_) => ActionOptions::RicheseGaveCard(cards(card_ids(RicheseGaveCard::valid_cards(game, p)))),
        GameEvent::RicheseKeepsUnsoldCard(_) => ActionOptions::RicheseKeepsUnsoldCard(draft_only()),
        GameEvent::RicheseSpecifiesAuction(_) => ActionOptions::RicheseSpecifiesAuction(RicheseSpecifiesAuctionOptions {
            may_specify_card: RicheseSpecifiesAuction::may_specify_card(game, p),
            may_specify_type: RicheseSpecifiesAuction::may_specify_type_of_auction(game, p),
            cards: card_ids(RicheseSpecifiesAuction::valid_cards(game)),
            auction_types: vec![labelled(AuctionType::RicheseOnceAround, "Once Around"), labelled(AuctionType::RicheseSilent, "Silent")],
            draft: json.clone(),
        }),
        _ => return None,
    })
}

fn ally_permission(game: &Game, p: PlayerIndex, d: &dyn Describer, draft: serde_json::Value) -> AllyPermissionOptions {
    let pl = game.player(p);
    let ally = pl.ally;
    let bot_claims = pl.allied_player(game).filter(|a| game.is_bot(*a)).map(|a| {
        let al = game.player(a);
        let mine = |h: &Hero| h.faction() == pl.faction;
        // KnownNonTraitors.Where(...).Union(Traitors.Where(...).Union(FaceDancers.Where(...))):
        // distinct, in first-seen order.
        let mut safe: Vec<Hero> = vec![];
        for h in al.known_non_traitors.iter().filter(|h| mine(h)).chain(al.traitors.iter().filter(|h| mine(h))).chain(al.face_dancers.iter().filter(|h| mine(h))) {
            if !safe.contains(h) {
                safe.push(*h);
            }
        }
        BotAllyClaims {
            owns_weapon: al.treachery_cards.iter().any(|c| c.is_weapon()),
            owns_defense: al.treachery_cards.iter().any(|c| c.is_defense()),
            safe_leaders: hero_options(safe),
            resources: al.resources,
        }
    });
    let mut current = AllyPermission::new(game, pl.faction);
    current_permissions(game, pl.faction, &mut current);
    let _ = d;
    AllyPermissionOptions {
        ally: ally.to_string(),
        bot_claims,
        max_resources: pl.resources,
        current_resources: game.get_permitted_use_of_ally_resources(ally),
        karma_cards: Karma::can_karma(game, p).then(|| card_ids(Karma::valid_karma_cards(game, p))),
        current_karma_card: game.get_permitted_use_of_ally_karma(ally).map(|c| c.id()),
        current: draft_json(&event(current)),
        fremen_determines_storm: game.applicable(Rule::FremenDeterminesStorm),
        fremen_refunds_dial: game.applicable(Rule::FremenAllyGetsDialedResourcesRefunded),
        draft,
    }
}

// ---------------------------------------------------------------------------------------------
// The simple confirm forms and the yes/no forms.

fn simple(title: RichText, confirm_label: impl Into<String>, alert: bool) -> ActionOptions {
    ActionOptions::Simple { title: Some(title), confirm_label: confirm_label.into(), alert, allow_auto_confirm: false }
}

/// "Use <card type>?", with the skin's name of the card type.
fn describe_card(d: &dyn Describer, t: TreacheryCardType) -> RichText {
    text(format(d, "Use {0}?", &[t.into()]))
}

fn use_card(d: &dyn Describer, verb: &str, t: TreacheryCardType) -> String {
    format(d, &format!("{verb} {{0}}"), &[t.into()])
}

fn simple_options(game: &Game, p: PlayerIndex, t: GameEventType, d: &dyn Describer) -> Option<ActionOptions> {
    use TreacheryCardType as C;
    let pl = game.player(p);
    let m = |e: Message| message(&e, d);
    Some(match t {
        GameEventType::MetheorPlayed => simple(describe_card(d, C::Metheor), use_card(d, "Use", C::Metheor), false),
        GameEventType::CharityClaimed => simple(text("Claim Charity?"), "Claim", false),
        GameEventType::GuildDelay => ActionOptions::Simple {
            title: Some(text("Delay your turn?")),
            confirm_label: "Delay".into(),
            alert: true,
            allow_auto_confirm: true,
        },
        GameEventType::BattleRevision => simple(text("Change your battle plan?"), "Change Plan", false),
        GameEventType::KarmaPrescience => simple(text(format(d, "Use {0} to see the entire enemy battle plan", &[C::Karma.into()])), use_card(d, "Use", C::Karma), false),
        GameEventType::KarmaEcazDial => {
            simple(text(format(d, "Use {0} to add the difference between the leader discs to your dial", &[C::Karma.into()])), use_card(d, "Use", C::Karma), false)
        }
        GameEventType::AmalPlayed => simple(describe_card(d, C::Amal), use_card(d, "Use", C::Amal), false),
        GameEventType::RecruitsPlayed => simple(describe_card(d, C::Recruits), use_card(d, "Play", C::Recruits), false),
        GameEventType::HarvesterPlayed => simple(describe_card(d, C::Harvester), use_card(d, "Use", C::Harvester), false),
        GameEventType::PoisonToothCancelled => simple(text("Cancel Poison Tooth?"), "Cancel", false),
        GameEventType::ThumperPlayed => simple(describe_card(d, C::Thumper), use_card(d, "Use", C::Thumper), false),
        GameEventType::ExtortionPrevented => simple(
            m(express!("Pay ", Payment::of(3), " to prevent ", Faction::Moritani, " regaining their ", TerrorType::Extortion, " token?")),
            "Yes",
            false,
        ),
        GameEventType::ChoamExtraMove => {
            let title = match ChoamExtraMove::card_to_use(game, p) {
                Some(c) => express!("Use ", c, " to be able to move your forces one extra territory?"),
                None => express!("Use ", "Nexus Cunning", " to be able to move your forces one extra territory?"),
            };
            simple(m(title), "Ok", false)
        }
        GameEventType::RicheseRevealedNoField => simple(
            m(express!(
                "Reveal your currently active No-Field of ",
                game.current_no_field_value,
                ", placing ",
                game.current_no_field_value.min(pl.forces_in_reserve()),
                " ",
                FactionForce::Richese,
                "?"
            )),
            "Reveal",
            false,
        ),
        GameEventType::ResidualPlayed => {
            simple(text(format(d, "Use {0} to kill a random available opponent leader?", &[C::Residual.into()])), use_card(d, "Use", C::Residual), false)
        }
        GameEventType::FlightDiscoveryUsed => simple(m(express!("Use your ", DiscoveryToken::Flight, " Discovery Token to gain extra movement speed?")), "Yes", false),
        GameEventType::DiscardedSearchedAnnounced => simple(
            m(express!("Use ", C::SearchDiscarded, " to pay ", Payment::of(2), " and search and take a card from the treachery discard pile?")),
            use_card(d, "Use", C::SearchDiscarded),
            false,
        ),
        GameEventType::PortableAntidoteUsed => simple(text(format(d, "Use your {0}?", &[C::PortableAntidote.into()])), use_card(d, "Use", C::PortableAntidote), false),
        GameEventType::SwitchedSkilledLeader => {
            let leader = SwitchedSkilledLeader::switchable_leader(game, p);
            let in_front = game.is_in_front_of_shield(leader.map(Hero::Leader));
            let side = if in_front { " behind" } else { " in front of" };
            let label = format(d, "Put {0} shield", &[(if in_front { "behind" } else { "in front of" }).into()]);
            simple(m(express!("Place ", leader, side, " your player shield?")), label, false)
        }
        _ => return None,
    })
}

fn yes_no(title: RichText, alert: bool, draft: &GameEvent) -> ActionOptions {
    ActionOptions::YesNo(YesNoOptions { title, alert, draft: draft_json(draft) })
}

fn yes_no_options(game: &Game, p: PlayerIndex, draft: &GameEvent, d: &dyn Describer) -> Option<ActionOptions> {
    let m = |e: Message| message(&e, d);
    Some(match draft.event_type() {
        GameEventType::NexusVoted => yes_no(text("Should a Nexus occur?"), true, draft),
        GameEventType::AllianceByTerror => yes_no(m(express!("Enter an alliance with ", Faction::Moritani, "?")), true, draft),
        GameEventType::AllianceByAmbassador => yes_no(m(express!("Enter an alliance with ", Faction::Ecaz, "?")), true, draft),
        GameEventType::BattleClaimed => {
            let territory = game.battle_about_to_start.as_ref().and_then(|b| b.territory(game));
            let title = if game.player(p).faction == Faction::Ecaz {
                express!("Do you wish to fight in ", territory, "? Otherwise, your ally will.")
            } else {
                express!("Should ", Faction::Ecaz, " fight the battle in ", territory, "?")
            };
            yes_no(m(title), true, draft)
        }
        GameEventType::DivideResourcesAccepted => yes_no(text(format(d, "Agree with the proposed division of {0}?", &[Concept::Resource.into()])), true, draft),
        GameEventType::CardGiven => yes_no(m(express!("Give your ally the ", game.card_that_must_be_kept_or_given_to_ally, " you just received?")), true, draft),
        GameEventType::Bureaucracy => yes_no(m(express!("Use your ", LeaderSkill::Bureaucrat, " skill?")), true, draft),
        GameEventType::Captured => yes_no(text("Do you wish to capture a random leader from your opponent?"), true, draft),
        _ => return None,
    })
}
