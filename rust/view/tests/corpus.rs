//! Views over the bot-game corpus (`rust/traces/sample/bots-*.jsonl.gz` and `tests/data`): after every accepted
//! event of every game, the view of every player and of an observer.
//!
//! - One replica serves all viewers of a game (client-plan.md, The view replica). Views built in
//!   one order must equal views built in another order on a second replica that follows the same
//!   events, at every step. A difference would mean that building a view changes what a later
//!   view shows, and the fallback (one replica per viewer) would be needed.
//! - Every view keeps the visibility rules of protocol.md, checked against the engine's state
//!   independently of the builder.
//!
//! Set `VIEW_CORPUS_DIR` to run over another directory of bot traces (generate them with the
//! oracle through `rust/tools/heavy.sh`, then delete them).

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;

use treachery_engine::cs::Shared;
use treachery_engine::i_identifiable::IIdentifiable;
use treachery_engine::leader_manager::LeaderManager;
use treachery_engine::participation::Participation;
use treachery_engine::prelude::*;
use treachery_view::build::ResolvedViewer;
use treachery_view::replica::ViewReplica;
use treachery_view::schema::*;

fn corpus() -> Vec<common::Trace> {
    match std::env::var("VIEW_CORPUS_DIR") {
        Ok(dir) => {
            let files = common::bot_traces_in(&PathBuf::from(&dir));
            assert!(!files.is_empty(), "no bot traces in {dir}");
            files.iter().map(|f| common::read_trace(f)).collect()
        }
        Err(_) => common::sample_bot_traces().iter().map(|f| common::read_trace(f)).collect(),
    }
}

/// Every seated player (the first one as host) and an observer.
fn viewers(game: &Game) -> Vec<ResolvedViewer> {
    let mut out: Vec<ResolvedViewer> = game
        .player_indices()
        .map(|p| ResolvedViewer { player: Some(p), faction: Some(game.player(p).faction), is_host: p.index == 0, is_observer: false })
        .collect();
    out.push(ResolvedViewer { player: None, faction: None, is_host: false, is_observer: true });
    out
}

/// How often each rule was exercised with something to hide or show, so the test shows that
/// the corpus reached it.
#[derive(Default)]
struct Seen(BTreeMap<&'static str, usize>);

impl Seen {
    fn add(&mut self, what: &'static str) {
        *self.0.entry(what).or_default() += 1;
    }
}

/// protocol.md, Visibility: what this view may hold, derived from the engine state directly.
fn check_visibility(game: &Game, v: &ResolvedViewer, view: &GameView, seen: &mut Seen, at: &str) {
    let me = v.player.filter(|_| !v.is_observer);
    let my_faction = me.map(|p| game.player(p).faction);

    // The shield is the viewer's own.
    match (&view.me, me) {
        (None, None) => {}
        (Some(shield), Some(p)) => {
            assert_eq!(shield.faction, game.player(p).faction.to_string(), "{at}");
            if !shield.hand.iter().all(|c| c.playable_with.is_empty()) {
                seen.add("playable card");
            }
        }
        _ => panic!("{at}: shield of the wrong viewer"),
    }

    // Tanks: a face-down leader is identified only for its owner and Tleilaxu.
    for row in &view.tanks {
        for l in &row.leaders {
            if l.face_down {
                seen.add("face-down leader");
                if let Some(id) = l.id {
                    let leader = LeaderManager::leaders().iter().find(|x| x.id() == id).unwrap();
                    assert!(my_faction == Some(leader.faction()) || my_faction == Some(Faction::Tleilaxu), "{at}: face-down leader identified");
                    seen.add("face-down leader identified");
                }
            }
        }
    }

    // Board tokens.
    for t in &view.board.terror {
        seen.add("terror token");
        assert!(t.token.is_none() || my_faction == Some(Faction::Moritani), "{at}: terror identified");
    }
    for d in &view.board.discoveries {
        seen.add("discovery token");
        if d.token.is_some() {
            let allowed = (my_faction == Some(Faction::Fremen) && d.token_type == "Fremen") || (my_faction == Some(Faction::Guild) && d.token_type == "Guild");
            assert!(allowed, "{at}: discovery identified");
            seen.add("discovery identified");
        }
    }
    assert!(view.board.no_field_value.is_none() || my_faction == Some(Faction::Richese), "{at}: No-Field value");
    if view.board.no_field_value.is_some_and(|n| n >= 0) {
        seen.add("No-Field value");
    }
    assert_eq!(view.board.storm_prescience_moves.is_some(), game.has_storm_prescience(me), "{at}");

    // Piles.
    assert!(view.piles.richese_cache.is_none() || my_faction == Some(Faction::Richese), "{at}: Richese's cache");
    let notekeeping = game.assisted_note_keeping_enabled(me);
    assert_eq!(view.piles.treachery_discard.is_some(), notekeeping, "{at}");
    assert_eq!(view.notes.is_some(), notekeeping, "{at}");
    if let Some(top) = view.piles.resource_deck_top {
        assert!(game.current_main_phase == MainPhase::ShipmentAndMove && game.has_resource_deck_prescience(me), "{at}: deck top");
        assert_eq!(Some(top), game.resource_card_deck.as_ref().and_then(|d| d.top()).map(|c| c.skin_id()), "{at}");
        seen.add("resource deck top");
    }

    // Bids: nothing but "ready" in silent auctions.
    if let Some(b) = &view.bidding {
        if b.auction_type == "BlackMarketSilent" || b.auction_type == "RicheseSilent" {
            assert!(b.bids.iter().all(|x| x.bid == BidShown::Ready), "{at}: silent bid shown");
            if !b.bids.is_empty() {
                seen.add("silent bids");
            }
        }
        if b.bids.iter().any(|x| matches!(x.bid, BidShown::Bid { .. })) {
            seen.add("open bid");
        }
        // The lot's card: only under bidding prescience or in Richese's auctions, where it is
        // public. Observers have no prescience.
        if let Some(card) = b.panel.as_ref().and_then(|p| p.card) {
            let richese = matches!(game.current_auction_type, AuctionType::RicheseOnceAround | AuctionType::RicheseSilent);
            assert!(richese || me.is_some_and(|p| game.has_bidding_prescience(p)), "{at}: card on auction shown");
            assert_eq!(Some(card), game.cards_on_auction.as_ref().and_then(|d| d.top()).map(|c| c.id()), "{at}");
            seen.add("card on auction");
        }
        if my_faction.is_none() && b.panel.is_some() {
            seen.add("observer bidding panel");
        }
    }

    // Battle plans: full for the plan's owner, once both are in, and under Atreides Karama;
    // partial only for the prescience user's side.
    if let Some(battle) = &view.battle {
        let both = game.aggressor_plan.is_some() && game.defender_plan.is_some();
        for (plan, faction_plan) in [(&battle.aggressor_plan, &game.aggressor_plan), (&battle.defender_plan, &game.defender_plan)] {
            let Some(plan) = plan else { continue };
            let actual = faction_plan.as_ref().expect("a shown plan exists");
            assert_eq!(plan.faction, actual.initiator().to_string(), "{at}");
            if plan.full {
                let own = my_faction == Some(actual.initiator());
                assert!(own || both || game.atreides_karma, "{at}: full plan shown");
                seen.add(if own && !both { "own plan" } else { "full plan" });
            } else {
                let users: Vec<Faction> =
                    game.current_prescience.iter().map(|p| p.initiator()).chain(game.current_nexus_prescience.iter().map(|n| n.initiator())).collect();
                let ally = me.map(|p| game.player(p).ally);
                assert!(users.iter().any(|u| my_faction == Some(*u) || ally == Some(*u)), "{at}: partial plan for a bystander");
                seen.add("partial plan");
            }
        }
    }

    // Private log lines.
    for r in &view.log.latest {
        for l in &r.lines {
            assert!(!l.private || my_faction.is_some(), "{at}: private line for an observer");
        }
    }
}

/// Fields that only some states show; counted to see the corpus reach them.
fn count_fields(view: &GameView, seen: &mut Seen) {
    if !view.status.flash.is_empty() {
        seen.add("flash");
    }
    if !view.status.to_bid.is_empty() {
        seen.add("to bid");
    }
    if !view.board.strongholds.is_empty() {
        seen.add("stronghold owner");
    }
    if !view.board.ambassadors.is_empty() {
        seen.add("ambassador on planet");
    }
    if !view.board.attached.is_empty() {
        seen.add("attached location");
    }
    if !view.board.leaders.is_empty() {
        seen.add("leader on planet");
    }
    if !view.board.monsters.is_empty() {
        seen.add("monster");
    }
    if !view.board.recent_moves.is_empty() {
        seen.add("recent move");
    }
    if view.board.forces.iter().any(|f| f.threatened) {
        seen.add("threatened forces");
    }
    for p in &view.players {
        if p.homeworlds.iter().any(|h| h.occupier.is_some()) {
            seen.add("occupied homeworld");
        }
        if p.homeworlds.iter().any(|h| h.low_threshold) {
            seen.add("low threshold");
        }
        if !p.skilled_leaders.is_empty() {
            seen.add("skilled leader");
        }
        if p.loyal_leader.is_some() {
            seen.add("loyal leader");
        }
        if p.latest_revealed_no_field.is_some() {
            seen.add("revealed No-Field");
        }
    }
    if let Some(me) = &view.me {
        if me.messiah.is_some() {
            seen.add("messiah state");
        }
        if me.unplaced_terror.as_ref().is_some_and(|t| !t.is_empty()) {
            seen.add("unplaced terror");
        }
        if me.card_just_won.is_some() {
            seen.add("card just won");
        }
    }
    if view.piles.sand_trout.is_some() {
        seen.add("sand trout");
    }
    if view.piles.richese_cache.as_ref().is_some_and(|c| !c.is_empty()) {
        seen.add("Richese's cache");
    }
    if view.tanks.iter().any(|t| t.messiah_dead) {
        seen.add("messiah dead");
    }
}

#[test]
fn views_do_not_depend_on_build_order_over_the_bot_corpus() {
    let mut seen = Seen::default();
    let mut steps = 0;
    for trace in corpus() {
        let participation = Shared::new(Participation::default());
        let mut forward = ViewReplica::new(trace.version, &participation);
        let mut shuffled = ViewReplica::new(trace.version, &participation);
        for (n, event) in trace.events.iter().enumerate() {
            forward.apply(event).unwrap_or_else(|e| panic!("{} event {n}: {e}", trace.name));
            shuffled.apply(event).unwrap();
            // Player handles belong to one game, so each replica resolves its own viewers.
            let vs = viewers(&forward.game);
            let vs_shuffled = viewers(&shuffled.game);
            let a: Vec<GameView> = vs.iter().map(|v| forward.view_as(v)).collect();
            // Another order on the second replica: reversed, then rotated by the step number.
            let mut order: Vec<usize> = (0..vs.len()).rev().collect();
            order.rotate_left(n % vs.len());
            let mut b: Vec<Option<GameView>> = vec![None; vs.len()];
            for i in order {
                b[i] = Some(shuffled.view_as(&vs_shuffled[i]));
            }
            for (i, (x, y)) in a.iter().zip(&b).enumerate() {
                assert!(Some(x) == y.as_ref(), "{} event {n}: the view of viewer {i} depends on the build order", trace.name);
            }
            for (v, view) in vs.iter().zip(&a) {
                check_visibility(&forward.game, v, view, &mut seen, &format!("{} event {n} viewer {:?}", trace.name, v.faction));
                count_fields(view, &mut seen);
            }
            steps += 1;
        }
    }
    println!("{steps} steps; seen: {:#?}", seen.0);
    for what in ["flash", "to bid", "terror token", "discovery token", "discovery identified", "silent bids", "partial plan", "full plan", "own plan", "playable card", "leader on planet", "recent move", "skilled leader"] {
        assert!(seen.0.get(what).copied().unwrap_or(0) > 0, "the corpus never showed {what}");
    }
}
