//! Focused tests for alliances, deals, discards and the game status. The comments say which
//! behavior each test pins down.

use std::rc::Rc;
use treachery_engine::error::catch_panic;
use treachery_engine::prelude::*;

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    for (seat, f) in factions.iter().enumerate() {
        let mut p = Player::new(*f);
        p.seat = seat as i32;
        g.players.push(p);
    }
    g
}

fn idx(g: &Game, f: Faction) -> PlayerIndex {
    g.get_player(Some(f)).unwrap()
}

fn exception_text<R>(f: impl FnOnce() -> R) -> String {
    match catch_panic(f) {
        Ok(_) => "no exception".to_string(),
        Err(e) => e,
    }
}

fn ally(g: &mut Game, a: Faction, b: Faction) {
    let pa = idx(g, a);
    let pb = idx(g, b);
    g.player_mut(pa).ally = b;
    g.player_mut(pb).ally = a;
}

// ---------------------------------------------------------------- Game_Alliances

/// Breaking an alliance clears every permission of both factions through SetPermissions.
#[test]
fn break_alliance_resets_permissions() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    ally(&mut g, Faction::Atreides, Faction::Emperor);
    g.atreides_shares_prescience = true;
    g.emperor_will_pay_for_extra_revival = 3;
    g.permitted_use_of_ally_spice.insert(Faction::Atreides, 4);
    g.permitted_use_of_ally_spice.insert(Faction::Emperor, 5);

    g.break_alliance(Faction::Atreides);

    assert!(!g.atreides_shares_prescience);
    assert_eq!(g.emperor_will_pay_for_extra_revival, 0);
    assert!(!g.permitted_use_of_ally_spice.contains_key(&Faction::Atreides));
    assert!(!g.permitted_use_of_ally_spice.contains_key(&Faction::Emperor));
    assert_eq!(g.player(idx(&g, Faction::Atreides)).ally, Faction::None);
    assert_eq!(g.player(idx(&g, Faction::Emperor)).ally, Faction::None);
}

/// Without an allied player, BreakAlliance returns before touching anything.
#[test]
fn break_alliance_without_ally_is_a_no_op() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    g.atreides_shares_prescience = true;
    g.break_alliance(Faction::Atreides);
    assert!(g.atreides_shares_prescience);
}

#[test]
fn decrease_permitted_use_of_ally_spice_clamps_at_zero() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    g.decrease_permitted_use_of_ally_spice(Faction::Atreides, 3); // no entry: nothing added
    assert!(!g.permitted_use_of_ally_spice.contains_key(&Faction::Atreides));
    g.permitted_use_of_ally_spice.insert(Faction::Atreides, 2);
    g.decrease_permitted_use_of_ally_spice(Faction::Atreides, 3);
    assert_eq!(*g.permitted_use_of_ally_spice.get(&Faction::Atreides).unwrap(), 0);
}

/// `Players.SingleOrDefault(p => p.Ally == f)`: two players claiming the same ally throws.
#[test]
fn permitted_use_of_ally_karma_single_or_default_throws() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor, Faction::BeneGesserit]);
    let r = idx(&g, Faction::Emperor);
    let b = idx(&g, Faction::BeneGesserit);
    g.player_mut(r).ally = Faction::Atreides;
    g.player_mut(b).ally = Faction::Atreides;
    assert_eq!(
        exception_text(|| g.get_permitted_use_of_ally_karma(Faction::Atreides)),
        "more than one player allied to Atreides"
    );
}

#[test]
fn resources_your_ally_can_pay_is_capped_by_ally_resources() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    ally(&mut g, Faction::Atreides, Faction::Emperor);
    let atreides = idx(&g, Faction::Atreides);
    let emperor = idx(&g, Faction::Emperor);
    assert_eq!(g.resources_your_ally_can_pay(atreides), 0);
    g.permitted_use_of_ally_spice.insert(Faction::Atreides, 10);
    g.player_mut(emperor).resources = 4;
    assert_eq!(g.resources_your_ally_can_pay(atreides), 4);
    assert_eq!(g.get_permitted_use_of_ally_resources(Faction::Atreides), 4);
}

// ---------------------------------------------------------------- Game_Deals / deal events

fn offer(g: &Game, by: Faction, to: Vec<Faction>, price: i32) -> Rc<DealOffered> {
    let mut o = DealOffered::new(g, by);
    o.to = Some(to);
    o.price = price;
    Rc::new(o)
}

/// Recipients exclude the initiator and the initiator's ally, and respect `To`.
#[test]
fn deal_recipients_in_seat_order() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor, Faction::BeneGesserit, Faction::Fremen]);
    ally(&mut g, Faction::Atreides, Faction::Emperor);
    let open = offer(&g, Faction::Atreides, vec![], 0);
    assert_eq!(g.deal_recipients(&open).collect::<Vec<_>>(), vec![Faction::BeneGesserit, Faction::Fremen]);
    let targeted = offer(&g, Faction::Atreides, vec![Faction::Fremen], 0);
    assert_eq!(g.deal_recipients(&targeted).collect::<Vec<_>>(), vec![Faction::Fremen]);
}

/// DealResponses is keyed by the offer object, not its contents.
#[test]
fn deal_responses_are_keyed_by_identity() {
    let g0 = game_with(&[Faction::Atreides, Faction::Emperor]);
    let a = offer(&g0, Faction::Atreides, vec![], 1);
    let b = offer(&g0, Faction::Atreides, vec![], 1);
    let mut g = g0;
    assert!(a.same(&b));
    g.record_deal_response(&a, Faction::Emperor, false);
    assert_eq!(g.deal_response(&a, Faction::Emperor), Some(false));
    assert_eq!(g.deal_response(&b, Faction::Emperor), None);
    g.record_deal_response(&a, Faction::Emperor, true); // indexer set: overwrites
    assert!(g.has_responded_to_deal(&a, Faction::Emperor));
    assert_eq!(g.deal_response(&a, Faction::Emperor), Some(true));
}

/// `Same` compares `To` with SequenceEqual. Under C# 14 that binds to the span
/// overload, so a null array counts as empty.
#[test]
fn deal_same_with_null_to_matches_empty_to() {
    let g = game_with(&[Faction::Atreides]);
    let a = offer(&g, Faction::Atreides, vec![], 1);
    let mut b = DealOffered::new(&g, Faction::Atreides);
    b.price = 1;
    b.to = None;
    assert!(a.same(&b));
}

/// Cancelling removes the first offer with the same contents, and its responses.
#[test]
fn deal_cancellation_removes_matching_offer() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    let a = offer(&g, Faction::Atreides, vec![], 2);
    a.execute_concrete_event(&mut g).unwrap();
    g.record_deal_response(&a, Faction::Emperor, false);
    assert_eq!(g.deal_offers.len(), 1);
    let c = a.cancellation().unwrap();
    assert!(c.cancel);
    c.execute_concrete_event(&mut g).unwrap();
    assert!(g.deal_offers.is_empty());
    assert!(g.deal_responses.is_empty());
}

#[test]
fn deal_contents_description_uses_text_when_not_blank() {
    let g = game_with(&[Faction::Atreides]);
    let m = Deal::deal_contents_description_game_deal_type_string_int32_phase_string(&g, DealType::None, Some("  \t"), 0, Phase::BattlePhase, None);
    assert!(m.to_cs_string().contains("Custom deal"), "{}", m.to_cs_string());
    let m = Deal::deal_contents_description_game_deal_type_string_int32_phase_string(&g, DealType::None, Some("hi"), 0, Phase::BattlePhase, None);
    assert!(m.to_cs_string().starts_with("hi until"), "{}", m.to_cs_string());
}

/// DontShipOrMoveTo parses its parameter with int.Parse, so an empty one is a FormatException.
#[test]
fn deal_parameter_is_parsed_with_int_parse() {
    let g = game_with(&[Faction::Atreides]);
    let text = exception_text(|| Deal::get_parameter1_game_deal_type_string(&g, DealType::DontShipOrMoveTo, ""));
    assert!(text.starts_with("FormatException"), "{text}");
}

// ---------------------------------------------------------------- Donated

/// The Double Inflation check builds a message and never returns it (C# bug kept).
#[test]
fn donated_double_inflation_check_has_no_effect() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    g.economics_status = ChoamEconomicsStatus::Double;
    let atreides = idx(&g, Faction::Atreides);
    g.player_mut(atreides).resources = 5;
    let mut d = Donated::new(&g, Faction::Atreides);
    d.target = Faction::Emperor;
    d.resources = 2;
    assert!(d.validate(&g).is_none());
}

// ---------------------------------------------------------------- Game_Contemplate

#[test]
fn threshold_for_win() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    let atreides = idx(&g, Faction::Atreides);
    assert_eq!(g.threshold_for_win(atreides), 4); // two players
    let mut g3 = game_with(&[Faction::Atreides, Faction::Emperor, Faction::BeneGesserit]);
    let atreides3 = idx(&g3, Faction::Atreides);
    assert_eq!(g3.threshold_for_win(atreides3), 3);
    ally(&mut g3, Faction::Atreides, Faction::Emperor);
    assert_eq!(g3.threshold_for_win(atreides3), 4);
    ally(&mut g, Faction::Atreides, Faction::Emperor);
    assert_eq!(g.threshold_for_win(atreides), 4);
}

// ---------------------------------------------------------------- GameStatus

#[test]
fn status_unknown_phase_waits_for_host() {
    let mut g = game_with(&[Faction::Atreides]);
    g.current_phase = Phase::None;
    let s = GameStatus::determine_status(&mut g, None, false);
    assert!(s.waiting_for_host);
    assert_eq!(s.get_message(g.player_at(0), true).to_cs_string(), "Unknown phase: None");
}

/// Discarding waits for the factions that must discard, skipping factions not in the game.
#[test]
fn status_discarding_waits_for_present_factions() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    g.current_phase = Phase::Discarding;
    g.factions_that_must_discard = vec![Faction::BeneGesserit, Faction::Emperor];
    let s = GameStatus::determine_status(&mut g, None, false);
    let emperor = idx(&g, Faction::Emperor);
    assert_eq!(s.waiting_for_players, vec![Some(emperor)]);
    assert!(s.waiting_for_me(emperor, false));
    assert!(s.waiting_for_others(g.player_at(0), true));
    assert_eq!(s.get_message(emperor, false).to_cs_string(), "Please decide which card to discard.");
}

/// `BattleWinner` is a `Faction?`, so `BattleWinner != Faction.None` holds for null, and the
/// flash shows "null win!" (C# bug kept).
#[test]
fn flash_battle_conclusion_with_null_winner() {
    let g = game_with(&[Faction::Atreides]);
    let mut g = g;
    g.current_phase = Phase::BattleConclusion;
    g.battle_winner = None;
    let flashes = GameStatus::determine_flash(&g, Faction::None, false);
    assert_eq!(flashes.len(), 1);
    g.battle_winner = Some(Faction::None);
    assert!(GameStatus::determine_flash(&g, Faction::None, false).is_empty());
}

#[test]
fn flash_game_ended_lists_winners() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    g.current_phase = Phase::GameEnded;
    g.winners = vec![idx(&g, Faction::Emperor), idx(&g, Faction::Atreides)];
    let flashes = GameStatus::determine_flash(&g, Faction::None, false);
    let texts: Vec<String> = flashes.iter().map(|f| f.message.to_cs_string()).collect();
    assert_eq!(texts.len(), 2);
    assert!(texts[0].starts_with("Emperor") || texts[0].contains("Emperor"), "{texts:?}");
}

/// Ecaz's ambassador phase: `Ally(game, Ecaz)` reads `player!.Ally` even when Ecaz is absent,
/// so the status throws a NullReferenceException.
#[test]
fn status_ambassador_without_ecaz_throws() {
    let mut g = game_with(&[Faction::Atreides]);
    g.current_phase = Phase::AmbassadorTriggeredByGuildShip;
    let text = exception_text(|| GameStatus::determine_status(&mut g, None, false));
    assert_eq!(text, "player");
}

#[test]
fn status_highlights_fremen_setup() {
    let mut g = game_with(&[Faction::Fremen]);
    g.current_phase = Phase::FremenSettingUp;
    let s = GameStatus::determine_status(&mut g, None, false);
    assert!(s.highlighted_territories.is_empty());
    let fremen = idx(&g, Faction::Fremen);
    let s = GameStatus::determine_status(&mut g, Some(fremen), true);
    assert_eq!(s.highlighted_territories.len(), 3);
    assert_eq!(s.highlighted_territories[1], Some(g.map.false_wall_south()));
    assert_eq!(s.waiting_for_players, vec![Some(idx(&g, Faction::Fremen))]);
}

/// A card trade offer must name the offerer's ally. C# accepted any faction as `Target`. The game
/// then waited in TradingCards for the target's answer, and a target without an ally could never
/// answer, because `Validate` dereferences its ally (fuzz seed 1 game 31, rust/docs/escapes.md).
#[test]
fn card_trade_offer_must_name_the_ally() {
    let mut g = game_with(&[Faction::Choam, Faction::Atreides, Faction::Guild]);
    ally(&mut g, Faction::Choam, Faction::Atreides);
    let cards: Vec<TreacheryCard> = TreacheryCardManager::items().iter().copied().take(3).collect();
    for (f, c) in [(Faction::Choam, cards[0]), (Faction::Atreides, cards[1]), (Faction::Guild, cards[2])] {
        let p = idx(&g, f);
        g.player_mut(p).treachery_cards = vec![c];
    }

    let offer = |target: Faction| {
        let mut e = CardTraded::default();
        e.base().initiator.set(Faction::Choam);
        e.base().player.set(g.get_player(Some(Faction::Choam)));
        e.target = target;
        e.set_card(Some(cards[0]));
        e.set_requested_card(None);
        e.validate(&g).map(|m| m.to_cs_string())
    };
    assert_eq!(offer(Faction::Atreides), None);
    assert_eq!(offer(Faction::Guild).as_deref(), Some("You can only trade cards with your ally"));
}
