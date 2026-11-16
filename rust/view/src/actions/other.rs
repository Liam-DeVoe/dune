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

use treachery_engine::events::EventType;
use treachery_engine::messages::part::Part as P;
use treachery_engine::enum_type::GameEnum;
use treachery_engine::prelude::*;
use treachery_engine::express;

use crate::build::{event, validate, ResolvedViewer, Seating};
use crate::describe::Describer;
use crate::expr::message;
use crate::schema::*;

use super::other_nexus;
use super::widgets::{picker, required, required_opt, faction_options, hero_options, card_ids, draft_json, describe, render};

/// The event types drawn as a simple confirm form (`ActionOptions::Simple`).
pub const SIMPLE: [EventType; 20] = [
    EventType::MetheorPlayed,
    EventType::CharityClaimed,
    EventType::GuildDelay,
    EventType::BattleRevision,
    EventType::KarmaPrescience,
    EventType::KarmaEcazDial,
    EventType::AmalPlayed,
    EventType::RecruitsPlayed,
    EventType::HarvesterPlayed,
    EventType::PoisonToothCancelled,
    EventType::ThumperPlayed,
    EventType::ExtortionPrevented,
    EventType::ChoamExtraMove,
    EventType::RicheseRevealedNoField,
    EventType::ResidualPlayed,
    EventType::FlightDiscoveryUsed,
    EventType::DiscardedSearchedAnnounced,
    EventType::PortableAntidoteUsed,
    EventType::SwitchedSkilledLeader,
    EventType::VoiceDeclined,
];

/// The event types drawn as a yes/no form (`ActionOptions::YesNo`).
pub const YES_NO: [EventType; 8] = [
    EventType::NexusVoted,
    EventType::AllianceByTerror,
    EventType::AllianceByAmbassador,
    EventType::BattleClaimed,
    EventType::DivideResourcesAccepted,
    EventType::CardGiven,
    EventType::Bureaucracy,
    EventType::Captured,
];

/// The event types with a form of their own in this module.
pub const FORMS: [EventType; 42] = [
    EventType::AllyPermission,
    EventType::AutomationConfigured,
    EventType::BeneGesseritPrediction,
    EventType::CardTraded,
    EventType::ChoamDiscarded,
    EventType::ChoamEconomics,
    EventType::ChoamKarmaPrevention,
    EventType::DealOffered,
    EventType::Discarded,
    EventType::DiscardedSearched,
    EventType::DiscardedTaken,
    EventType::DistransUsed,
    EventType::DivideResources,
    EventType::Donated,
    EventType::EmperorBidSupport,
    EventType::EmperorDiscarded,
    EventType::FactionSelected,
    EventType::FactionTradeOffered,
    EventType::IxianRemovedCardFromAuction,
    EventType::IxianSelectedStartingCard,
    EventType::IxianSwappedCardOnBid,
    EventType::Karma,
    EventType::KarmaChoamDiscard,
    EventType::KarmaHandSwap,
    EventType::KarmaHandSwapInitiated,
    EventType::KarmaMonster,
    EventType::KarmaRicheseBuy,
    EventType::MulliganPerformed,
    EventType::NexusCardDrawn,
    EventType::NexusPlayed,
    EventType::ReplacedCardWon,
    EventType::ResourcesAudited,
    EventType::ResourcesTransferred,
    EventType::SkillAssigned,
    EventType::StormSpellPlayed,
    EventType::TraitorDiscarded,
    EventType::TraitorsSelected,
    EventType::RicheseAnnouncesAuction,
    EventType::RicheseAnnouncesBlackMarket,
    EventType::RicheseGaveCard,
    EventType::RicheseKeepsUnsoldCard,
    EventType::RicheseSpecifiesAuction,
];

fn handles(t: EventType) -> bool {
    SIMPLE.contains(&t) || YES_NO.contains(&t) || FORMS.contains(&t)
}

// ---------------------------------------------------------------------------------------------
// Widget rules and helpers shared with other_nexus.rs.

pub(super) fn enum_option<T: GameEnum + Into<P> + Copy + ToString>(d: &dyn Describer, v: T) -> EnumOption {
    EnumOption { name: v.to_string(), value: v.value(), label: describe(d, v) }
}

/// [`enum_option`] for an enum whose C# `None` member is `None` in Rust.
pub(super) fn enum_option_opt<T: GameEnum>(d: &dyn Describer, v: Option<T>) -> EnumOption
where
    Option<T>: Into<P>,
{
    EnumOption { name: name_or_absent(v), value: value_or_absent(v), label: describe(d, v) }
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
pub fn default_draft(game: &Game, s: &Seating, faction: Option<Faction>, t: EventType) -> Option<Event> {
    if !handles(t) {
        return None;
    }
    let p = game.get_player(faction)?;
    let pl = game.player(p);
    Some(match t {
        EventType::AllyPermission => {
            // OnParametersSet while not edited: the permissions in force.
            let mut e = AllyPermission::default();
            current_permissions(game, faction.expect("faction"), &mut e);
            event(e)
        }
        EventType::AutomationConfigured => {
            let mut e = AutomationConfigured::default();
            e.action = ItemAction::Create;
            event(e)
        }
        EventType::CardTraded => {
            let mut e = CardTraded::default();
            e.target = pl.ally;
            event(e)
        }
        EventType::ChoamEconomics => {
            let mut e = ChoamEconomics::default();
            e.status = Some(ChoamEconomicsStatus::Double);
            event(e)
        }
        EventType::DealOffered => {
            let mut e = DealOffered::default();
            // Every target picked: the offer goes to everyone (To is empty).
            e.to = Some(vec![]);
            e.text = Some(String::new());
            event(e)
        }
        EventType::KarmaMonster => {
            // Nothing picked on the map yet.
            let mut e = KarmaMonster::default();
            e.territory = None;
            event(e)
        }
        EventType::EmperorBidSupport => {
            let mut e = EmperorBidSupport::default();
            let mut amounts = IndexMap::new();
            for f in opponents_but_ally(game, p) {
                amounts.insert(f, game.spice_for_bids_emperor_can_pay(f));
            }
            e.amounts = Some(amounts);
            event(e)
        }
        EventType::FactionSelected => {
            // Named and seated as the player.
            let mut e = FactionSelected::default();
            e.initiator_player_name = Some(s.player_name(game, p));
            e.seat = pl.seated().number();
            event(e)
        }
        EventType::NexusPlayed => other_nexus::default_draft(game, faction),
        EventType::RicheseAnnouncesAuction => {
            let mut e = RicheseAnnouncesAuction::default();
            e.first = true;
            event(e)
        }
        EventType::RicheseAnnouncesBlackMarket => {
            let mut e = RicheseAnnouncesBlackMarket::default();
            e.auction_type = Some(AuctionType::BlackMarketNormal);
            e.direction = 1;
            event(e)
        }
        EventType::RicheseSpecifiesAuction => {
            let mut e = RicheseSpecifiesAuction::default();
            e.auction_type = Some(AuctionType::RicheseOnceAround);
            e.direction = 1;
            event(e)
        }
        _ => {
            // A fresh event of the type.
            t.default_event()
        }
    })
}

/// The permissions in force, which the ally permission form starts from.
fn current_permissions(game: &Game, faction: Faction, e: &mut AllyPermission) {
    let ally = game.player(game.get_player(Some(faction)).expect("player")).ally;
    e.permitted_resources = treachery_engine::game::amounts::clamp_u16(game.get_permitted_use_of_ally_resources(ally));
    e.permitted_karma_card = game.get_permitted_use_of_ally_karma(ally);
    match faction {
        Faction::Tleilaxu => e.set_tleilaxu_allows_revival_discount(game.alliances.tleilaxu_allows_revival_discount),
        Faction::Ixian => e.set_ixian_allows_replacing_cards(game.alliances.ixian_allows_replacing_cards),
        Faction::Guild => e.set_guild_allows_shipping_discount(game.alliances.guild_allows_shipping_discount),
        Faction::Emperor => e.emperor_will_pay_for_extra_revival = treachery_engine::game::amounts::clamp_u16(game.alliances.emperor_will_pay_for_extra_revival),
        Faction::Fremen => {
            e.fremen_will_protect_from_monster = game.alliances.fremen_will_protect_from_monster;
            e.fremen_allows_three_free_revivals = game.alliances.fremen_allows_three_free_revivals;
            e.fremen_shares_prescience = game.alliances.fremen_shares_prescience;
            e.fremen_refunds_battle_dial = game.alliances.fremen_refunds_battle_dial;
        }
        Faction::Atreides => e.atreides_shares_prescience = game.alliances.atreides_shares_prescience,
        Faction::BeneGesserit => e.bene_gesserit_allows_use_of_voice = game.alliances.bene_gesserit_allows_use_of_voice,
        Faction::Richese => e.richese_allows_use_of_no_field = game.alliances.richese_allows_use_of_no_field,
        Faction::Moritani => e.moritani_allows_keeping_cards = game.alliances.moritani_allows_keeping_cards,
        Faction::Ecaz => e.ecaz_shares_ambassadors = game.alliances.ecaz_shares_ambassadors,
        _ => {}
    }
}

/// The opponents other than the ally: who Emperor may support in bidding, and who a deal may be
/// offered to.
fn opponents_but_ally(game: &Game, p: PlayerId) -> Vec<Faction> {
    let ally = game.player(p).ally;
    game.players_other_than(p).filter(|f| Some(*f) != ally).collect()
}

/// The phases a deal offer may last until.
fn deal_end_phases(t: DealType) -> Vec<Option<Phase>> {
    match t {
        DealType::ShareStormPrescience => vec![Some(Phase::TurnConcluded)],
        DealType::ShareBiddingPrescience => vec![Some(Phase::Bidding), Some(Phase::BiddingReport)],
        DealType::ShareResourceDeckPrescience => vec![Some(Phase::TurnConcluded)],
        _ => vec![Some(Phase::TurnConcluded), Some(Phase::GameEnded)],
    }
}

// ---------------------------------------------------------------------------------------------
// Adjusting a draft as the form does.

/// The draft as the form holds it after adjusting these inputs.
pub fn normalize(game: &Game, p: PlayerId, v: &ResolvedViewer, s: &Seating, draft: &Event) -> Event {
    let pl = game.player(p);
    let hand: Vec<TreacheryCard> = pl.treachery_cards.clone();
    macro_rules! adjust {
        ($e:expr, |$m:ident| $body:block) => {{
            let mut $m = $e.clone();
            $body
            event($m)
        }};
    }
    match draft {
        Event::AutomationConfigured(e) => adjust!(e, |m| {
            let types = AutomationConfigured::get_available_rule_types(game, p);
            m.rule_type = required(m.rule_type, &types, AutomationRuleType::default());
            m.action = ItemAction::Create;
            match m.rule_type {
                AutomationRuleType::BiddingPassAboveAmount => m.bidding_above_amount = treachery_engine::game::amounts::clamp_u16(picker(i32::from(m.bidding_above_amount), 0, 40)),
                AutomationRuleType::BiddingPassWhenHighestBidByFaction => {
                    let fs = AutomationConfigured::get_valid_bidding_factions(game, p);
                    m.bidding_winning_faction = required_opt(m.bidding_winning_faction, &fs);
                }
                _ => {}
            }
        }),
        Event::AllyPermission(e) => adjust!(e, |m| {
            m.permitted_resources = treachery_engine::game::amounts::clamp_u16(picker(i32::from(m.permitted_resources), 0, i32::from(pl.resources)));
            if !Karma::can_karma(game, p) {
                m.permitted_karma_card = None;
            }
            if pl.faction == Some(Faction::Emperor) {
                m.emperor_will_pay_for_extra_revival = treachery_engine::game::amounts::clamp_u16(picker(i32::from(m.emperor_will_pay_for_extra_revival), 0, 3));
            }
        }),
        Event::BeneGesseritPrediction(e) => adjust!(e, |m| {
            let targets: Vec<Faction> = BeneGesseritPrediction::valid_targets(game, p).collect();
            m.to_win = required_opt(m.to_win, &targets);
            m.turn = Turn::new(picker(m.turn.number().into(), 1, game.maximum_turns().number().into()) as u8);
        }),
        Event::CardTraded(e) => adjust!(e, |m| {
            m.target = pl.ally;
            let cards: Vec<TreacheryCard> = CardTraded::valid_cards(game, p).into_iter().collect();
            m.card = required_opt(m.card, &cards);
            match pl.allied_player(game) {
                // An optional image choice keeps whatever card was asked for.
                Some(a) if game.is_bot(a) => {}
                _ => m.requested_card = None,
            }
        }),
        Event::ChoamDiscarded(e) => adjust!(e, |m| {
            m.card = required_opt(m.card, &ChoamDiscarded::valid_cards(game, p).into_iter().collect::<Vec<_>>());
        }),
        Event::ChoamEconomics(e) => adjust!(e, |m| {
            m.status = required(m.status, &[Some(ChoamEconomicsStatus::Double), Some(ChoamEconomicsStatus::Cancel)], None);
        }),
        Event::ChoamKarmaPrevention(e) => adjust!(e, |m| {
            let targets: Vec<Faction> = game.players_other_than(p).collect();
            m.target = required_opt(m.target, &targets);
        }),
        Event::DealOffered(e) => adjust!(e, |m| {
            let types = DealOffered::get_standard_deal_types(game, p);
            m.type_ = required(m.type_, &types, DealType::None);
            m.end_phase = required(m.end_phase, &deal_end_phases(m.type_), None);
            m.price = treachery_engine::game::amounts::clamp_u16(picker(i32::from(m.price), 0, 50));
            m.benefit = treachery_engine::game::amounts::clamp_u16(picker(i32::from(m.benefit), 0, 50));
        }),
        Event::Discarded(e) => adjust!(e, |m| {
            m.card = required_opt(m.card, &hand);
        }),
        Event::DiscardedSearched(e) => adjust!(e, |m| {
            m.card = required_opt(m.card, &DiscardedSearched::valid_cards(game).into_iter().collect::<Vec<_>>());
        }),
        Event::DiscardedTaken(e) => adjust!(e, |m| {
            m.card = required_opt(m.card, &DiscardedTaken::valid_cards(game, p).into_iter().collect::<Vec<_>>());
        }),
        Event::DistransUsed(e) => adjust!(e, |m| {
            let targets: Vec<Faction> = DistransUsed::valid_targets(game, p).collect();
            m.target = required_opt(m.target, &targets);
            m.card = required_opt(m.card, &DistransUsed::valid_cards(game, p).into_iter().collect::<Vec<_>>());
        }),
        Event::DivideResources(e) => adjust!(e, |m| {
            let amount = DivideResources::get_resources_to_be_divided(game).map_or(0, |t| t.amount);
            m.portion_to_first_player = treachery_engine::game::amounts::clamp_u16(picker(i32::from(m.portion_to_first_player), 0, amount));
        }),
        Event::Donated(e) => adjust!(e, |m| {
            if !v.is_host {
                m.from_bank = false;
            }
            let (targets, min, max) = donation_range(game, p, m.from_bank);
            m.target = required_opt(m.target, &targets);
            m.resources = picker(m.resources, min, max);
            if !game.applicable(Rule::CardsCanBeTraded) {
                m.card = None;
            }
        }),
        Event::EmperorBidSupport(e) => adjust!(e, |m| {
            if let Some(amounts) = m.amounts.as_mut() {
                for (_, a) in amounts.iter_mut() {
                    *a = picker(*a, 0, i32::from(pl.resources));
                }
            }
        }),
        Event::EmperorDiscarded(e) => adjust!(e, |m| {
            m.card = required_opt(m.card, &hand);
        }),
        Event::FactionSelected(e) => adjust!(e, |m| {
            let fs: Vec<Faction> = FactionSelected::valid_factions(game).collect();
            m.faction = required_opt(m.faction, &fs);
            m.seat = pl.seated().number();
            m.initiator_player_name = Some(s.player_name(game, p));
        }),
        Event::FactionTradeOffered(e) => adjust!(e, |m| {
            let targets: Vec<Faction> = FactionTradeOffered::valid_targets(game, p).collect();
            m.target = required_opt(m.target, &targets);
        }),
        Event::IxianRemovedCardFromAuction(e) => adjust!(e, |m| {
            let cards: Vec<TreacheryCard> = game.cards_on_auction().map(|d| d.items.to_vec()).unwrap_or_default().into_iter().collect();
            m.card = required_opt(m.card, &cards);
        }),
        Event::IxianSelectedStartingCard(e) => adjust!(e, |m| {
            let cards: Vec<TreacheryCard> = game.setup().and_then(|s| s.starting_treachery_cards.as_ref()).map(|d| d.items.to_vec()).unwrap_or_default().into_iter().collect();
            m.card = required_opt(m.card, &cards);
        }),
        Event::IxianSwappedCardOnBid(e) => adjust!(e, |m| {
            m.passed = false;
            m.card = required_opt(m.card, &hand);
        }),
        Event::Karma(e) => adjust!(e, |m| {
            m.card = required_opt(m.card, &Karma::valid_karma_cards(game, p).into_iter().collect::<Vec<_>>());
            m.prevented = required(m.prevented, &Karma::valid_faction_advantages(game, p), None);
        }),
        Event::KarmaChoamDiscard(e) => adjust!(e, |m| {
            let karma: Vec<TreacheryCard> = Karma::valid_karma_cards(game, p).into_iter().collect();
            let before = m.card_to_use;
            // More than one: a required picker. Otherwise `CardToUse = ValidKarmaCards.FirstOrDefault()`.
            m.card_to_use = if karma.len() > 1 { required_opt(m.card_to_use, &karma) } else { karma.first().copied() };
            if m.card_to_use != before && karma.len() > 1 && before.is_some_and(|b| karma.contains(&b)) {
                // ChangeCardToUse clears the selection.
                m.set_cards(&[]);
            }
        }),
        Event::KarmaHandSwapInitiated(e) => adjust!(e, |m| {
            let targets: Vec<Faction> = game.players_other_than(p).collect();
            m.target = required_opt(m.target, &targets);
        }),
        Event::KarmaRicheseBuy(e) => adjust!(e, |m| {
            m.card = required_opt(m.card, &game.decks.richese_cache.iter().copied().into_iter().collect::<Vec<_>>());
        }),
        Event::MulliganPerformed(e) => adjust!(e, |m| {
            m.passed = false;
        }),
        Event::NexusCardDrawn(e) => adjust!(e, |m| {
            m.passed = false;
        }),
        Event::NexusPlayed(_) => other_nexus::normalize(game, p, draft),
        Event::ReplacedCardWon(e) => adjust!(e, |m| {
            m.passed = false;
        }),
        Event::ResourcesAudited(e) => adjust!(e, |m| {
            let targets: Vec<Faction> = ResourcesAudited::valid_factions(game, p).collect();
            m.target = required_opt(m.target, &targets);
        }),
        Event::ResourcesTransferred(e) => adjust!(e, |m| {
            m.resources = treachery_engine::game::amounts::clamp_u16(picker(i32::from(m.resources), 0, ResourcesTransferred::max_amount(game, p)));
        }),
        Event::SkillAssigned(e) => adjust!(e, |m| {
            m.passed = false;
            m.skill = required_opt(m.skill, &SkillAssigned::valid_skills(game, p));
            let leaders: Vec<Option<Leader>> = SkillAssigned::valid_leaders(game, p).into_iter().collect();
            m.leader = required(m.leader, &leaders, None);
        }),
        Event::StormSpellPlayed(e) => adjust!(e, |m| {
            m.move_amount = treachery_engine::game::amounts::clamp_u16(picker(i32::from(m.move_amount), 0, 10));
        }),
        Event::TraitorDiscarded(e) => adjust!(e, |m| {
            let heroes: Vec<Hero> = TraitorDiscarded::valid_traitors(game, p).into_iter().collect();
            m.traitor = required_opt(m.traitor, &heroes);
        }),
        Event::TraitorsSelected(e) => adjust!(e, |m| {
            m.selected_traitor = required_opt(m.selected_traitor, &pl.traitors);
        }),
        Event::RicheseAnnouncesAuction(e) => adjust!(e, |m| {
            m.first = true;
        }),
        Event::RicheseAnnouncesBlackMarket(e) => adjust!(e, |m| {
            m.passed = false;
            m.card = required_opt(m.card, &RicheseAnnouncesBlackMarket::valid_cards(game, p).into_iter().collect::<Vec<_>>());
            let types = [Some(AuctionType::BlackMarketNormal), Some(AuctionType::BlackMarketOnceAround), Some(AuctionType::BlackMarketSilent)];
            m.auction_type = required(m.auction_type, &types, None);
            if m.auction_type == Some(AuctionType::BlackMarketOnceAround) {
                m.direction = required(m.direction, &[1, -1], 0);
            }
        }),
        Event::RicheseGaveCard(e) => adjust!(e, |m| {
            m.card = required_opt(m.card, &RicheseGaveCard::valid_cards(game, p).into_iter().collect::<Vec<_>>());
        }),
        Event::RicheseKeepsUnsoldCard(e) => adjust!(e, |m| {
            m.passed = false;
        }),
        Event::RicheseSpecifiesAuction(e) => adjust!(e, |m| {
            if RicheseSpecifiesAuction::may_specify_card(game, p) {
                m.card = required_opt(m.card, &RicheseSpecifiesAuction::valid_cards(game).into_iter().collect::<Vec<_>>());
            }
            if RicheseSpecifiesAuction::may_specify_type_of_auction(game, p) {
                let types = [Some(AuctionType::RicheseOnceAround), Some(AuctionType::RicheseSilent)];
                m.auction_type = required(m.auction_type, &types, None);
                if m.auction_type == Some(AuctionType::RicheseOnceAround) {
                    m.direction = required(m.direction, &[1, -1], 0);
                }
            }
        }),
        _ => draft.clone(),
    }
}

/// Who may receive a donation, and its minimum and maximum amount.
fn donation_range(game: &Game, p: PlayerId, from_bank: bool) -> (Vec<Faction>, i32, i32) {
    if from_bank {
        (game.players.iter().filter_map(|pl| pl.faction).collect(), -99, 99)
    } else {
        (Donated::valid_targets(game, p).collect(), Donated::min_amount(game, p), Donated::max_amount(game, p))
    }
}

// ---------------------------------------------------------------------------------------------
// Options.

/// The options and validation of a form of this module for `draft`, or `None` for other event
/// types. The draft is first adjusted as the form would ([`normalize`]).
pub fn options_and_validation(game: &Game, v: &ResolvedViewer, s: &Seating, draft: &Event, d: &dyn Describer) -> Option<(Option<ActionOptions>, Option<RichText>)> {
    let t = draft.event_type();
    if !handles(t) {
        return None;
    }
    let p = v.player?;
    let draft = normalize(game, p, v, s, draft);
    let options = options(game, p, v, &draft, d);
    // The draft's `To` already says "everyone" when all are picked; picking none is the
    // client's check.
    Some((options, validate(game, game.player(p).faction, draft, d)))
}

fn options(game: &Game, p: PlayerId, v: &ResolvedViewer, draft: &Event, d: &dyn Describer) -> Option<ActionOptions> {
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
        Event::AllyPermission(_) => ActionOptions::AllyPermission(ally_permission(game, p, d, json.clone())),
        Event::AutomationConfigured(_) => ActionOptions::AutomationConfigured(AutomationConfiguredOptions {
            rule_types: AutomationConfigured::get_available_rule_types(game, p).into_iter().map(|r| enum_option(d, r)).collect(),
            bidding_factions: faction_options(AutomationConfigured::get_valid_bidding_factions(game, p)),
            rules: game
                .rules.automation_rules
                .iter()
                .filter(|r| r.initiator == pl.faction)
                .map(|r| AutomationRule {
                    id: r.id,
                    description: r.get_description(game).map(|m| message(&m, d)).unwrap_or_default(),
                })
                .collect(),
            draft: json.clone(),
        }),
        Event::BeneGesseritPrediction(_) => ActionOptions::BeneGesseritPrediction(BeneGesseritPredictionOptions {
            targets: faction_options(BeneGesseritPrediction::valid_targets(game, p)),
            max_turn: game.maximum_turns(),
            draft: json.clone(),
        }),
        Event::CardTraded(_) => ActionOptions::CardTraded(CardTradedOptions {
            cards: card_ids(CardTraded::valid_cards(game, p)),
            ally_cards: pl.allied_player(game).filter(|a| game.is_bot(*a)).map(|a| card_ids(CardTraded::valid_cards(game, a))),
            draft: json.clone(),
        }),
        Event::ChoamDiscarded(_) => ActionOptions::ChoamDiscarded(cards(card_ids(ChoamDiscarded::valid_cards(game, p)))),
        Event::ChoamEconomics(_) => ActionOptions::ChoamEconomics(ChoamEconomicsOptions {
            statuses: [ChoamEconomicsStatus::Double, ChoamEconomicsStatus::Cancel].into_iter().map(|s| enum_option(d, s)).collect(),
            draft: json.clone(),
        }),
        Event::ChoamKarmaPrevention(_) => ActionOptions::ChoamKarmaPrevention(ChoamKarmaPreventionOptions {
            card: ChoamKarmaPrevention::card_to_use(game, p).map(|c| message(&express!(c), d)),
            targets: faction_options(game.players_other_than(p)),
            draft: json.clone(),
        }),
        Event::DealOffered(_) => ActionOptions::DealOffered(DealOfferedOptions {
            targets: faction_options(opponents_but_ally(game, p)),
            deal_types: DealOffered::get_standard_deal_types(game, p)
                .into_iter()
                .map(|t| DealTypeOption {
                    value: t.value(),
                    label: plain(&message(&Deal::express(t, P::Null), d)),
                    end_phases: deal_end_phases(t).into_iter().map(|ph| enum_option_opt(d, ph)).collect(),
                })
                .collect(),
            draft: json.clone(),
        }),
        Event::Discarded(_) => ActionOptions::Discarded(cards(hand())),
        Event::DiscardedSearched(_) => ActionOptions::DiscardedSearched(cards(card_ids(DiscardedSearched::valid_cards(game)))),
        Event::DiscardedTaken(_) => ActionOptions::DiscardedTaken(cards(card_ids(DiscardedTaken::valid_cards(game, p)))),
        Event::DistransUsed(_) => ActionOptions::DistransUsed(DistransUsedOptions {
            targets: faction_options(DistransUsed::valid_targets(game, p)),
            cards: card_ids(DistransUsed::valid_cards(game, p)),
            draft: json.clone(),
        }),
        Event::DivideResources(e) => {
            let tbd = DivideResources::get_resources_to_be_divided(game)?;
            ActionOptions::DivideResources(DivideResourcesOptions {
                amount: tbd.amount,
                territory: tbd.territory.id(),
                other_faction: tbd.other_faction.to_string(),
                gained_if_passed: DivideResources::gained_by_first_faction(Some(&tbd), false, i32::from(e.portion_to_first_player)),
                other_gains_if_passed: DivideResources::gained_by_other_faction(Some(&tbd), false, i32::from(e.portion_to_first_player)),
                draft: json.clone(),
            })
        }
        Event::Donated(e) => {
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
        Event::EmperorBidSupport(_) => ActionOptions::EmperorBidSupport(EmperorBidSupportOptions {
            targets: faction_options(opponents_but_ally(game, p)),
            max: i32::from(pl.resources),
            draft: json.clone(),
        }),
        Event::EmperorDiscarded(_) => ActionOptions::EmperorDiscarded(cards(hand())),
        Event::FactionSelected(_) => ActionOptions::FactionSelected(factions(FactionSelected::valid_factions(game).collect())),
        Event::FactionTradeOffered(_) => {
            let targets: Vec<Faction> = FactionTradeOffered::valid_targets(game, p).collect();
            ActionOptions::FactionTradeOffered(FactionTradeOfferedOptions {
                in_play: targets.iter().filter(|f| game.players.iter().any(|pl| pl.faction == Some(**f))).map(|f| f.to_string()).collect(),
                targets: faction_options(targets),
                draft: json.clone(),
            })
        }
        Event::IxianRemovedCardFromAuction(_) => {
            ActionOptions::IxianRemovedCardFromAuction(cards(card_ids(game.cards_on_auction().map(|d| d.items.to_vec()).unwrap_or_default())))
        }
        Event::IxianSelectedStartingCard(_) => {
            ActionOptions::IxianSelectedStartingCard(cards(card_ids(game.setup().and_then(|s| s.starting_treachery_cards.as_ref()).map(|d| d.items.to_vec()).unwrap_or_default())))
        }
        Event::IxianSwappedCardOnBid(_) => ActionOptions::IxianSwappedCardOnBid(cards(hand())),
        Event::Karma(_) => ActionOptions::Karma(KarmaOptions {
            cards: card_ids(Karma::valid_karma_cards(game, p)),
            advantages: Karma::valid_faction_advantages(game, p).into_iter().map(|a| enum_option_opt(d, a)).collect(),
            draft: json.clone(),
        }),
        Event::KarmaChoamDiscard(e) => ActionOptions::KarmaChoamDiscard(KarmaChoamDiscardOptions {
            karma_cards: card_ids(Karma::valid_karma_cards(game, p)),
            cards: hand().into_iter().filter(|c| Some(*c) != e.card_to_use.map(|c| c.id())).collect(),
            draft: json.clone(),
        }),
        Event::KarmaHandSwap(_) => ActionOptions::KarmaHandSwap(KarmaHandSwapOptions {
            number: game.karma_hand_swap().map_or(0, |(n, _)| n),
            target: name_or_absent(game.karma_hand_swap().and_then(|(_, t)| t)),
            cards: hand(),
            draft: json.clone(),
        }),
        Event::KarmaHandSwapInitiated(_) => ActionOptions::KarmaHandSwapInitiated(factions(game.players_other_than(p).collect())),
        Event::KarmaMonster(_) => {
            let ts: Vec<Territory> = KarmaMonster::valid_targets(game).collect();
            ActionOptions::KarmaMonster(TerritoryChoiceOptions {
                territories: ts.iter().map(|t| t.id()).collect(),
                locations: ts.iter().flat_map(|t| t.locations().iter().map(|l| l.id())).collect(),
                draft: json.clone(),
            })
        }
        Event::KarmaRicheseBuy(_) => ActionOptions::KarmaRicheseBuy(cards(card_ids(game.decks.richese_cache.iter().copied()))),
        Event::MulliganPerformed(_) => ActionOptions::MulliganPerformed(MayOptions { may: MulliganPerformed::may_mulligan(game, p), draft: json.clone() }),
        Event::NexusCardDrawn(_) => ActionOptions::NexusCardDrawn(MayOptions { may: NexusCardDrawn::may_draw(game, p), draft: json.clone() }),
        Event::NexusPlayed(e) => ActionOptions::NexusPlayed(other_nexus::options(game, p, e, d, json.clone())),
        Event::ReplacedCardWon(_) => ActionOptions::ReplacedCardWon(draft_only()),
        Event::ResourcesAudited(_) => ActionOptions::ResourcesAudited(factions(ResourcesAudited::valid_factions(game, p).collect())),
        Event::ResourcesTransferred(_) => ActionOptions::ResourcesTransferred(ResourcesTransferredOptions {
            max: ResourcesTransferred::max_amount(game, p),
            draft: json.clone(),
        }),
        Event::SkillAssigned(_) => ActionOptions::SkillAssigned(SkillAssignedOptions {
            skills: SkillAssigned::valid_skills(game, p).into_iter().map(|s| enum_option(d, s)).collect(),
            leaders: hero_options(SkillAssigned::valid_leaders(game, p).into_iter().flatten().map(Hero::Leader)),
            draft: json.clone(),
        }),
        Event::StormSpellPlayed(_) => ActionOptions::StormSpellPlayed(draft_only()),
        Event::TraitorDiscarded(_) => ActionOptions::TraitorDiscarded(HeroChoiceOptions {
            heroes: hero_options(TraitorDiscarded::valid_traitors(game, p)),
            draft: json.clone(),
        }),
        Event::TraitorsSelected(_) => ActionOptions::TraitorsSelected(HeroChoiceOptions {
            heroes: hero_options(pl.traitors.iter().copied()),
            draft: json.clone(),
        }),
        Event::RicheseAnnouncesAuction(_) => ActionOptions::RicheseAnnouncesAuction(draft_only()),
        Event::RicheseAnnouncesBlackMarket(_) => ActionOptions::RicheseAnnouncesBlackMarket(RicheseAnnouncesBlackMarketOptions {
            cards: card_ids(RicheseAnnouncesBlackMarket::valid_cards(game, p)),
            auction_types: vec![
                labelled(AuctionType::BlackMarketNormal, "Normal"),
                labelled(AuctionType::BlackMarketOnceAround, "Once Around"),
                labelled(AuctionType::BlackMarketSilent, "Silent"),
            ],
            draft: json.clone(),
        }),
        Event::RicheseGaveCard(_) => ActionOptions::RicheseGaveCard(cards(card_ids(RicheseGaveCard::valid_cards(game, p)))),
        Event::RicheseKeepsUnsoldCard(_) => ActionOptions::RicheseKeepsUnsoldCard(draft_only()),
        Event::RicheseSpecifiesAuction(_) => ActionOptions::RicheseSpecifiesAuction(RicheseSpecifiesAuctionOptions {
            may_specify_card: RicheseSpecifiesAuction::may_specify_card(game, p),
            may_specify_type: RicheseSpecifiesAuction::may_specify_type_of_auction(game, p),
            cards: card_ids(RicheseSpecifiesAuction::valid_cards(game)),
            auction_types: vec![labelled(AuctionType::RicheseOnceAround, "Once Around"), labelled(AuctionType::RicheseSilent, "Silent")],
            draft: json.clone(),
        }),
        _ => return None,
    })
}

fn ally_permission(game: &Game, p: PlayerId, d: &dyn Describer, draft: serde_json::Value) -> AllyPermissionOptions {
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
            resources: i32::from(al.resources),
        }
    });
    let mut current = AllyPermission::default();
    current_permissions(game, pl.faction.expect("faction"), &mut current);
    let _ = d;
    AllyPermissionOptions {
        ally: name_or_absent(ally),
        bot_claims,
        max_resources: i32::from(pl.resources),
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
    text(render(d, &parts!["Use ", t, "?"]))
}

fn use_card(d: &dyn Describer, verb: &str, t: TreacheryCardType) -> String {
    render(d, &parts![format!("{verb} "), t])
}

fn simple_options(game: &Game, p: PlayerId, t: EventType, d: &dyn Describer) -> Option<ActionOptions> {
    use TreacheryCardType as C;
    let pl = game.player(p);
    let m = |e: Message| message(&e, d);
    Some(match t {
        EventType::MetheorPlayed => simple(describe_card(d, C::Metheor), use_card(d, "Use", C::Metheor), false),
        EventType::CharityClaimed => simple(text("Claim Charity?"), "Claim", false),
        EventType::GuildDelay => ActionOptions::Simple {
            title: Some(text("Delay your turn?")),
            confirm_label: "Delay".into(),
            alert: true,
            allow_auto_confirm: true,
        },
        EventType::BattleRevision => simple(text("Change your battle plan?"), "Change Plan", false),
        EventType::VoiceDeclined => simple(text("Prescience waits until you use Voice or decline it. Decline Voice for this battle?"), "Don't use Voice", false),
        EventType::KarmaPrescience => simple(text(render(d, &parts!["Use ", C::Karma, " to see the entire enemy battle plan"])), use_card(d, "Use", C::Karma), false),
        EventType::KarmaEcazDial => {
            simple(text(render(d, &parts!["Use ", C::Karma, " to add the difference between the leader discs to your dial"])), use_card(d, "Use", C::Karma), false)
        }
        EventType::AmalPlayed => simple(describe_card(d, C::Amal), use_card(d, "Use", C::Amal), false),
        EventType::RecruitsPlayed => simple(describe_card(d, C::Recruits), use_card(d, "Play", C::Recruits), false),
        EventType::HarvesterPlayed => simple(describe_card(d, C::Harvester), use_card(d, "Use", C::Harvester), false),
        EventType::PoisonToothCancelled => simple(text("Cancel Poison Tooth?"), "Cancel", false),
        EventType::ThumperPlayed => simple(describe_card(d, C::Thumper), use_card(d, "Use", C::Thumper), false),
        EventType::ExtortionPrevented => simple(
            m(express!("Pay ", Payment::of(3), " to prevent ", Faction::Moritani, " regaining their ", TerrorType::Extortion, " token?")),
            "Yes",
            false,
        ),
        EventType::ChoamExtraMove => {
            let title = match ChoamExtraMove::card_to_use(game, p) {
                Some(c) => express!("Use ", c, " to be able to move your forces one extra territory?"),
                None => express!("Use ", "Nexus Cunning", " to be able to move your forces one extra territory?"),
            };
            simple(m(title), "Ok", false)
        }
        EventType::RicheseRevealedNoField => simple(
            m(express!(
                "Reveal your currently active No-Field of ",
                game.ship_move.current_no_field_value,
                ", placing ",
                game.ship_move.current_no_field_value.min(pl.forces_in_reserve()),
                " ",
                FactionForce::Richese,
                "?"
            )),
            "Reveal",
            false,
        ),
        EventType::ResidualPlayed => {
            simple(text(render(d, &parts!["Use ", C::Residual, " to kill a random available opponent leader?"])), use_card(d, "Use", C::Residual), false)
        }
        EventType::FlightDiscoveryUsed => simple(m(express!("Use your ", DiscoveryToken::Flight, " Discovery Token to gain extra movement speed?")), "Yes", false),
        EventType::DiscardedSearchedAnnounced => simple(
            m(express!("Use ", C::SearchDiscarded, " to pay ", Payment::of(2), " and search and take a card from the treachery discard pile?")),
            use_card(d, "Use", C::SearchDiscarded),
            false,
        ),
        EventType::PortableAntidoteUsed => simple(text(render(d, &parts!["Use your ", C::PortableAntidote, "?"])), use_card(d, "Use", C::PortableAntidote), false),
        EventType::SwitchedSkilledLeader => {
            let leader = SwitchedSkilledLeader::switchable_leader(game, p);
            let in_front = game.is_in_front_of_shield(leader.map(Hero::Leader));
            let side = if in_front { " behind" } else { " in front of" };
            let label = render(d, &parts!["Put ", if in_front { "behind" } else { "in front of" }, " shield"]);
            simple(m(express!("Place ", leader, side, " your player shield?")), label, false)
        }
        _ => return None,
    })
}

fn yes_no(title: RichText, alert: bool, draft: &Event) -> ActionOptions {
    ActionOptions::YesNo(YesNoOptions { title, alert, draft: draft_json(draft) })
}

fn yes_no_options(game: &Game, p: PlayerId, draft: &Event, d: &dyn Describer) -> Option<ActionOptions> {
    let m = |e: Message| message(&e, d);
    Some(match draft.event_type() {
        EventType::NexusVoted => yes_no(text("Should a Nexus occur?"), true, draft),
        EventType::AllianceByTerror => yes_no(m(express!("Enter an alliance with ", Faction::Moritani, "?")), true, draft),
        EventType::AllianceByAmbassador => yes_no(m(express!("Enter an alliance with ", Faction::Ecaz, "?")), true, draft),
        EventType::BattleClaimed => {
            let territory = game.battle_about_to_start().and_then(|b| b.territory);
            let title = if game.player(p).faction == Some(Faction::Ecaz) {
                express!("Do you wish to fight in ", territory, "? Otherwise, your ally will.")
            } else {
                express!("Should ", Faction::Ecaz, " fight the battle in ", territory, "?")
            };
            yes_no(m(title), true, draft)
        }
        EventType::DivideResourcesAccepted => yes_no(text(render(d, &parts!["Agree with the proposed division of ", Concept::Resource, "?"])), true, draft),
        EventType::CardGiven => yes_no(m(express!("Give your ally the ", game.bidding().and_then(|b| b.card_that_must_be_kept_or_given_to_ally), " you just received?")), true, draft),
        EventType::Bureaucracy => yes_no(m(express!("Use your ", LeaderSkill::Bureaucrat, " skill?")), true, draft),
        EventType::Captured => yes_no(text("Do you wish to capture a random leader from your opponent?"), true, draft),
        _ => return None,
    })
}
