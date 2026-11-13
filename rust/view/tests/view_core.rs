//! The view fields that are not panels, field by field, on the savegame fixtures of
//! `scripts/fixtures` and on states of the sample bot games (for the expansions the fixtures
//! don't use). Each test checks the field against the engine and against who may see it.

mod common;

use common::{first_bot_state, load, load_text, viewer_of, with_rules, OBSERVER};
use treachery_engine::events::GameEventType;
use treachery_engine::heroes::hero::Hero;
use treachery_engine::identity::IIdentifiable;
use treachery_engine::prelude::*;
use treachery_view::replica::ViewReplica;
use treachery_view::schema::*;

const FIXTURES: &[&str] = &[
    "alliance", "ally", "battle", "battle-special", "bid", "blow", "blow-two-decks", "bene_gesserit", "facedancer", "karma-prescience",
    "mc-charity", "mc-clairvoyance", "mc-concluded", "mc-discard", "mc-ended", "mc-handswap", "mc-harvester", "mc-prediction",
    "mc-prescience", "mc-thumper", "mc-traitors", "mc-treachery", "mc-voice", "move", "nexus-prescience", "prescience-seen",
    "retreat", "revival", "setup", "ship-fremen", "tanks-face-down", "worm", "worm-ride-ssw",
];

fn factions(r: &ViewReplica) -> Vec<Faction> {
    r.game.players.iter().map(|p| p.faction).collect()
}

fn player(r: &ViewReplica, f: Faction) -> PlayerIndex {
    r.game.get_player(Some(f)).unwrap()
}

/// Every seated player's view and the observer's, with the viewer's faction.
fn all_views(r: &mut ViewReplica) -> Vec<(Option<Faction>, GameView)> {
    let mut out: Vec<(Option<Faction>, GameView)> = factions(r).into_iter().map(|f| (Some(f), r.view(viewer_of(r, f)))).collect();
    out.push((None, r.view(OBSERVER)));
    out
}

fn name<T: ToString>(v: T) -> String {
    v.to_string()
}

// ---- Card piles ----

/// Deck counts, the discard pile's top, the latest spice cards and Richese's cache count are the
/// same for everyone; the resource deck's top only under prescience during ship and move; Richese's
/// cache only for Richese; the full discard piles only with notekeeping.
#[test]
fn piles() {
    let mut saw_latest = false;
    for fixture in FIXTURES {
        let mut r = load(fixture, 0);
        for (f, view) in all_views(&mut r) {
            let g = &r.game;
            let p = &view.piles;
            assert_eq!(p.treachery_deck_count, g.treachery_deck.as_ref().map_or(0, |d| d.items.len() as i32), "{fixture}");
            let discard = g.treachery_discard_pile.as_ref().map(|d| d.items.clone()).unwrap_or_default();
            assert_eq!(p.treachery_discard_count, discard.len() as i32);
            assert_eq!(p.treachery_discard_top, discard.first().map(|c| c.id()));
            assert_eq!(p.resource_deck_count, g.resource_card_deck.as_ref().map_or(0, |d| d.items.len() as i32));
            assert_eq!(p.latest_spice_card_a, g.latest_spice_card_a.map(|c| c.skin_id()));
            assert_eq!(p.latest_spice_card_b, g.latest_spice_card_b.map(|c| c.skin_id()));
            saw_latest |= p.latest_spice_card_a.is_some();
            assert_eq!(p.sand_trout.is_some(), g.sand_trout_occured() && g.sand_trout.is_some());
            assert_eq!(p.richese_cache_count, g.richese_cache.len() as i32);
            assert_eq!(p.richese_cache.is_some(), f == Some(Faction::Richese));
            // No fixture enables notekeeping; see notekeeping_shows_discard_piles_and_notes.
            assert!(p.treachery_discard.is_none() && p.resource_discard_a.is_none() && view.notes.is_none());
            let prescience = f.is_some_and(|f| g.current_main_phase == MainPhase::ShipmentAndMove && g.has_resource_deck_prescience(Some(player(&r, f))));
            assert_eq!(p.resource_deck_top.is_some(), prescience, "{fixture} {f:?}");
        }
    }
    assert!(saw_latest);
}

/// Atreides (resource prescience) sees the top of the resource deck during ship and move, and
/// nobody else does.
#[test]
fn resource_deck_top_under_prescience() {
    let mut r = load("ship", 0);
    assert_eq!(r.game.current_main_phase, MainPhase::ShipmentAndMove);
    let top = r.game.resource_card_deck.as_ref().unwrap().top().unwrap().skin_id();
    let mut seen = 0;
    for (f, view) in all_views(&mut r) {
        let expected = f.is_some_and(|f| r.game.has_resource_deck_prescience(Some(player(&r, f))));
        assert_eq!(view.piles.resource_deck_top, expected.then_some(top), "{f:?}");
        seen += expected as i32;
    }
    assert!(seen >= 1, "Atreides is in the ship fixture");
}

/// `Rule.AssistedNotekeeping`: the discard pile shows every card, face down where the viewer
/// doesn't know it (`Game.KnownCards`, which includes the ally's); the spice discard piles are
/// shown whole; the notes list every other player's cards, known ones by id (the viewer's
/// own `Player.KnownCards` only), spice and victory points. Observers get none of it.
#[test]
fn notekeeping_shows_discard_piles_and_notes() {
    let mut r = load_text(&with_rules("mc-concluded", &[Rule::AssistedNotekeeping.0]), 0);
    assert!(r.game.applicable(Rule::AssistedNotekeeping));
    let mut known_somewhere = 0;
    let mut unknown_somewhere = 0;
    for (f, view) in all_views(&mut r) {
        let g = &r.game;
        let Some(f) = f else {
            assert!(view.notes.is_none() && view.piles.treachery_discard.is_none() && view.piles.resource_discard_a.is_none());
            continue;
        };
        let me = player(&r, f);
        let known = g.known_cards(me);
        let discard = g.treachery_discard_pile.as_ref().unwrap().items.clone();
        let expected: Vec<Option<i32>> = discard.iter().map(|c| known.contains(c).then(|| c.id())).collect();
        assert_eq!(view.piles.treachery_discard.as_ref(), Some(&expected));
        let a: Vec<i32> = g.resource_card_discard_pile_a.as_ref().unwrap().items.iter().map(|c| c.skin_id()).collect();
        assert_eq!(view.piles.resource_discard_a.as_ref(), Some(&a));

        let notes = view.notes.as_ref().expect("notes for a player");
        assert_eq!(notes.players.len(), g.players.len() - 1);
        assert!(notes.players.iter().all(|n| n.faction != f.to_string()));
        for n in &notes.players {
            let other = g.get_player(Some(Faction::ALL.iter().copied().find(|x| x.to_string() == n.faction).unwrap())).unwrap();
            let pl = g.player(other);
            assert_eq!((n.resources, n.bribes, n.resources_after_bidding), (pl.resources, pl.bribes, pl.resources_after_bidding));
            assert_eq!(n.victory_points, g.number_of_victory_points(other, true));
            let cards = g.cards_owned_by(other);
            assert_eq!(n.cards.len(), cards.len());
            for (shown, card) in n.cards.iter().zip(&cards) {
                assert_eq!(*shown, g.player(me).known_cards.contains(card).then(|| card.id()));
                if shown.is_some() {
                    known_somewhere += 1;
                } else {
                    unknown_somewhere += 1;
                }
            }
            assert_eq!(n.revealed_traitors.len(), pl.revealed_traitors.len());
        }
    }
    assert!(known_somewhere > 0 && unknown_somewhere > 0, "the fixture has known and unknown cards in hands");
}

// ---- Tanks ----

/// Forces killed, dead leaders in order of death, and the Messiah. A face-down leader is shown
/// to its owner and to Tleilaxu; everyone else gets `null`.
#[test]
fn tanks() {
    let mut face_down = 0;
    for fixture in FIXTURES {
        let mut r = load(fixture, 0);
        for (f, view) in all_views(&mut r) {
            let g = &r.game;
            assert_eq!(view.tanks.len(), g.players.len());
            for (row, pl) in view.tanks.iter().zip(&g.players) {
                assert_eq!(row.faction, pl.faction.to_string());
                assert_eq!((row.forces_killed, row.special_forces_killed), (pl.forces_killed, pl.special_forces_killed));
                let dead: Vec<Leader> = pl.leaders.iter().copied().filter(|l| !g.is_alive(Hero::Leader(*l))).collect();
                assert_eq!(row.leaders.len(), dead.len(), "{fixture}");
                let mut times: Vec<i32> = vec![];
                for shown in &row.leaders {
                    let l = match shown.id {
                        Some(id) => *dead.iter().find(|l| l.id() == id).expect("a dead leader of this player"),
                        None => {
                            assert!(shown.face_down, "{fixture}: only face-down leaders are hidden");
                            continue;
                        }
                    };
                    let s = &g.leader_state[&Hero::Leader(l)];
                    assert_eq!(shown.face_down, s.is_face_down_dead());
                    times.push(s.time_of_death);
                    if shown.face_down {
                        assert!(f == Some(pl.faction) || f == Some(Faction::Tleilaxu), "{fixture}: {f:?} identifies a face-down leader");
                    }
                }
                assert!(times.windows(2).all(|w| w[0] <= w[1]), "in order of death");
                for shown in row.leaders.iter().filter(|l| l.face_down) {
                    face_down += 1;
                    assert_eq!(shown.id.is_some(), f == Some(pl.faction) || f == Some(Faction::Tleilaxu), "{fixture} {f:?}");
                }
                assert_eq!(row.messiah_dead, pl.faction == Faction::Atreides && !g.messiah_is_alive());
            }
        }
    }
    assert!(face_down > 0, "a fixture has a face-down leader in the tanks");
}

// ---- The ring of players ----

#[test]
fn ring() {
    let mut skilled = 0;
    for fixture in FIXTURES {
        let mut r = load(fixture, 0);
        let view = r.view(OBSERVER);
        let g = &r.game;
        // SectorGeometry.PlayerTokenPosition places seats by Game.MaximumPlayers.
        assert_eq!(view.maximum_players, g.maximum_players());
        for (pp, pl) in view.players.iter().zip(&g.players) {
            let p = player(&r, pl.faction);
            assert_eq!(pp.has_special_forces, pl.has_special_forces(g));
            assert_eq!(pp.has_room_for_cards, pl.has_room_for_cards(g));
            assert_eq!(pp.cards_owned, g.cards_owned_by(p).len() as i32);
            let worlds: Vec<String> = pl.home_worlds.iter().map(|h| name(h.world())).collect();
            assert_eq!(pp.homeworlds.iter().map(|h| h.world.clone()).collect::<Vec<_>>(), worlds);
            for (hv, h) in pp.homeworlds.iter().zip(&pl.home_worlds) {
                assert_eq!(hv.location, h.location().id());
                assert_eq!((hv.high_threshold, hv.low_threshold), (pl.has_high_threshold(g, h.world()), pl.has_low_threshold(g, h.world())));
                assert_eq!(hv.occupier.is_some(), g.is_occupied_world(h.world()));
            }
            let expected: Vec<(i32, String)> = g
                .get_skilled_leaders(p)
                .filter(|l| g.is_in_front_of_shield(Some(Hero::Leader(*l))))
                .map(|l| (l.id(), name(g.skill(Some(Hero::Leader(l))))))
                .collect();
            assert_eq!(pp.skilled_leaders.iter().map(|s| (s.id, s.skill.clone())).collect::<Vec<_>>(), expected);
            skilled += expected.len();
            assert_eq!(pp.loyal_leader.is_some(), pl.faction == Faction::Ecaz && g.ecaz_loyal_leader.is_some());
        }
    }
    assert!(skilled > 0, "the retreat fixture has skilled leaders in front of shields");
}

/// Ecaz's loyal leader and Richese's latest revealed No-Field, from bot games with those factions.
#[test]
fn ring_ecaz_and_richese() {
    let mut r = first_bot_state("a Ecaz loyal leader", |g| g.ecaz_loyal_leader.is_some() && g.is_playing(Faction::Ecaz));
    let view = r.view(OBSERVER);
    let ecaz = view.players.iter().find(|p| p.faction == "Ecaz").unwrap();
    assert_eq!(ecaz.loyal_leader, r.game.ecaz_loyal_leader.map(|l| l.id()));
    assert!(view.players.iter().filter(|p| p.faction != "Ecaz").all(|p| p.loyal_leader.is_none()));

    let mut r = first_bot_state("a revealed No-Field", |g| g.latest_revealed_no_field_value >= 0);
    let view = r.view(OBSERVER);
    let richese = view.players.iter().find(|p| p.faction == "Richese").unwrap();
    assert_eq!(richese.latest_revealed_no_field, Some(r.game.latest_revealed_no_field_value));
}

/// `Game.HomeworldOccupation`-free thresholds: a homeworld at low threshold shows as such.
#[test]
fn ring_homeworld_thresholds() {
    let mut r = first_bot_state("a homeworld at low threshold", |g| {
        g.applicable(Rule::Homeworlds) && g.players.iter().any(|p| p.home_worlds.iter().any(|h| p.has_low_threshold(g, h.world())))
    });
    let view = r.view(OBSERVER);
    assert!(view.players.iter().any(|p| p.homeworlds.iter().any(|h| h.low_threshold && !h.high_threshold)));
}

// ---- The faction panel ----

/// A card glows when an applicable action plays it, and opens every such action. Karma plays
/// several actions.
#[test]
fn hand_cards_name_the_actions_that_play_them() {
    let mut r = load("karma-prescience-base", 0);
    let atreides = r.view(viewer_of(&r, Faction::Atreides));
    let me = atreides.me.as_ref().unwrap();
    let card = me.hand.iter().find(|c| c.id == 23).expect("Atreides holds Karma (23)");
    assert_eq!(card.playable_with, vec!["Karma".to_string(), "KarmaPrescience".to_string()]);

    let mut r = load("mc-harvester", 0);
    let emperor = r.view(viewer_of(&r, Faction::Emperor));
    let harvester = emperor.me.as_ref().unwrap().hand.iter().find(|c| c.id == 43).unwrap();
    assert_eq!(harvester.playable_with, vec!["HarvesterPlayed".to_string()]);

    // Everywhere: only applicable actions, each playing a card of the right type.
    let mut playable = 0;
    for fixture in FIXTURES {
        let mut r = load(fixture, 0);
        for (f, view) in all_views(&mut r) {
            let Some(me) = &view.me else { continue };
            let p = player(&r, f.unwrap());
            for c in &me.hand {
                let card = r.game.player(p).treachery_cards.iter().copied().find(|x| x.id() == c.id).unwrap();
                for a in &c.playable_with {
                    playable += 1;
                    assert!(view.actions.iter().any(|x| &x.event_type == a), "{fixture}: {a} is applicable");
                    let t = GameEventType::ALL.iter().copied().find(|t| t.name() == a).unwrap();
                    assert!(treachery_view::shield::cards_for(&r.game, p, t).contains(&card));
                    if a.starts_with("Karma") {
                        assert!(Karma::valid_karma_cards(&r.game, p).any(|k| k == card));
                    }
                }
            }
        }
    }
    assert!(playable > 10);
}

/// `PlayerPanel.StillPickingTraitor`: the dealt traitor candidates stay hidden until the player
/// has picked; Harkonnen and Tleilaxu keep all theirs and see them.
#[test]
fn traitors_hidden_while_picking() {
    let mut r = load("mc-traitors", 0);
    assert_eq!(r.game.current_phase, Phase::SelectingTraitors);
    let mut hidden = 0;
    for f in factions(&r) {
        let view = r.view(viewer_of(&r, f));
        let pl = r.game.player(player(&r, f));
        let picking = f != Faction::Harkonnen && f != Faction::Tleilaxu && !r.game.has_acted_or_passed.contains(&f);
        let me = view.me.unwrap();
        if picking {
            hidden += 1;
            assert!(me.traitors.is_empty() && !pl.traitors.is_empty());
        } else {
            assert_eq!(me.traitors.len(), pl.traitors.len());
        }
    }
    assert!(hidden > 0);
}

/// The Messiah line (Atreides, `Rule.AtreidesMessiah`), Moritani's unplaced terror, and the card just won
/// when the viewer knows it.
#[test]
fn faction_panel_extras() {
    let mut r = first_bot_state("Atreides with the Messiah rule", |g| g.applicable(Rule::AtreidesMessiah) && g.is_playing(Faction::Atreides) && g.current_turn >= 2);
    for (f, view) in all_views(&mut r) {
        let Some(f) = f else { continue };
        let me = view.me.unwrap();
        let pl = r.game.player(player(&r, f));
        if f == Faction::Atreides {
            assert_eq!(me.messiah, Some(MessiahState { forces_killed_in_battle: pl.total_forces_killed_in_battle, available: pl.messiah_available(&r.game) }));
        } else {
            assert!(me.messiah.is_none());
        }
    }

    let mut r = first_bot_state("Moritani holding terror tokens", |g| g.is_playing(Faction::Moritani) && !g.unplaced_terror_tokens.is_empty() && g.current_turn >= 1);
    for (f, view) in all_views(&mut r) {
        let Some(f) = f else { continue };
        let me = view.me.unwrap();
        if f == Faction::Moritani {
            assert_eq!(me.unplaced_terror, Some(r.game.unplaced_terror_tokens.iter().map(|t| name(*t)).collect()));
        } else {
            assert!(me.unplaced_terror.is_none());
        }
    }

    let mut r = first_bot_state("a card just won", |g| g.card_just_won.is_some());
    let card = r.game.card_just_won.unwrap();
    let mut knowers = 0;
    for (f, view) in all_views(&mut r) {
        let Some(f) = f else { continue };
        let knows = r.game.known_cards(player(&r, f)).contains(&card);
        assert_eq!(view.me.unwrap().card_just_won, knows.then(|| card.id()));
        knowers += knows as i32;
    }
    assert!(knowers >= 1);
}

// ---- The prompt and the card flash ----

/// `Bid.PlayersToBid` with whose turn it is, in the bidding main phase only.
#[test]
fn prompt_bidders() {
    let mut r = load("bid", 0);
    let view = r.view(OBSERVER);
    let expected: Vec<SequenceEntry> = Bid::players_to_bid(&r.game)
        .iter()
        .filter_map(|se| se.player.map(|p| SequenceEntry { faction: name(r.game.player(p).faction), has_turn: se.has_turn }))
        .collect();
    assert_eq!(view.status.to_bid, expected);
    assert_eq!(view.status.to_bid.iter().filter(|e| e.has_turn).count(), 1);
    assert!(load("alliance", 0).view(OBSERVER).status.to_bid.is_empty());
}

/// `GameStatus.FlashInfo`: the battle winner, the game's winners, a spice blow.
#[test]
fn flash() {
    let mut r = load("mc-concluded", 0);
    let winner = r.game.battle_winner.unwrap();
    let view = r.view(OBSERVER);
    assert_eq!(view.status.flash.len(), 1);
    // Labels come from the skin; the faction is what this test is about.
    let faction_of = |p: &Part| match p {
        Part::Faction { v, .. } => v.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(faction_of(&view.status.flash[0].to_show), name(winner));
    assert_eq!(faction_of(&view.status.flash[0].message[0]), name(winner));
    assert_eq!(view.status.flash[0].message[1], Part::Text { v: " win!".into() });

    let mut r = load("mc-ended", 0);
    let view = r.view(OBSERVER);
    let winners: Vec<String> = r.game.winners.iter().map(|p| name(r.game.player(*p).faction)).collect();
    assert_eq!(view.status.flash.iter().map(|f| faction_of(&f.to_show)).collect::<Vec<_>>(), winners);

    let mut r = load("blow", 0);
    let view = r.view(OBSERVER);
    assert!(view.status.flash.iter().any(|f| matches!(f.to_show, Part::ResourceCard { .. })), "{:?}", view.status.flash);
}

/// `EndPhase` in the prompt: "Continue", with the event's validation.
#[test]
fn prompt_continue() {
    let mut r = load("blow", 0);
    let host = r.game.players.iter().find(|p| p.seat == 0).unwrap().faction;
    let view = r.view(viewer_of(&r, host));
    let end = view.actions.iter().find(|a| a.event_type == "EndPhase").expect("the host may end the blow report");
    assert_eq!(end.options, Some(ActionOptions::Simple { title: None, confirm_label: "Continue".into(), alert: false, allow_auto_confirm: false }));
    assert_eq!(end.validation, None);
}

// ---- Bids ----

#[test]
fn bids_on_the_ring() {
    let mut r = load("bid", 0);
    let view = r.view(OBSERVER);
    let b = view.bidding.expect("bidding phase");
    assert_eq!(b.auction_type, name(r.game.current_auction_type));
    assert_eq!(b.bids.len(), r.game.bids.len());
    for (shown, (f, bid)) in b.bids.iter().zip(r.game.bids.iter()) {
        assert_eq!(shown.faction, name(*f));
        let expected = if bid.i_bid_passed() { BidShown::Pass } else { BidShown::Bid { amount: bid.i_bid_total_amount() } };
        assert_eq!(shown.bid, expected);
    }
    assert!(load("alliance", 0).view(OBSERVER).bidding.is_none());
}

/// Silent auctions show "ready" for every bid, passes included.
#[test]
fn silent_bids_are_ready() {
    let mut r = first_bot_state("a silent auction with bids", |g| {
        (g.current_auction_type == AuctionType::RicheseSilent || g.current_auction_type == AuctionType::BlackMarketSilent)
            && g.current_main_phase == MainPhase::Bidding
            && !g.bids.is_empty()
    });
    for (_, view) in all_views(&mut r) {
        let b = view.bidding.unwrap();
        assert!(!b.bids.is_empty() && b.bids.iter().all(|x| x.bid == BidShown::Ready));
    }
}

// ---- Battle plans ----

/// Both plans in: everyone sees both, whole.
#[test]
fn battle_plans_once_both_are_in() {
    let mut r = load("mc-concluded", 0);
    let battle = r.game.current_battle.clone().unwrap();
    for (f, view) in all_views(&mut r) {
        let g = &r.game;
        let b = view.battle.as_ref().expect("a battle");
        assert_eq!((b.aggressor.as_str(), b.defender.as_str()), (name(battle.aggressor(g)).as_str(), name(battle.defender(g)).as_str()));
        for (shown, plan, opponent) in [(&b.aggressor_plan, &g.aggressor_plan, battle.defender(g)), (&b.defender_plan, &g.defender_plan, battle.aggressor(g))] {
            let shown = shown.as_ref().unwrap_or_else(|| panic!("{f:?} sees both plans"));
            let plan = plan.as_ref().unwrap();
            assert!(shown.full && shown.hero_shown && shown.weapon_shown && shown.defense_shown);
            assert_eq!(shown.faction, name(plan.initiator()));
            assert_eq!(shown.dial, Some(plan.dial(g, opponent) as f64));
            assert_eq!(shown.weapon, plan.weapon().map(|c| c.id()));
            assert_eq!(shown.cost, Some(plan.cost(g)).filter(|c| *c > 0));
            match (plan.hero(), &shown.hero) {
                (Some(Hero::Leader(l)), Some(PlanHero::Leader { id, .. })) => assert_eq!(*id, l.id()),
                (Some(Hero::TreacheryCard(c)), Some(PlanHero::TreacheryCard { id })) => assert_eq!(*id, c.id()),
                (None, None) => {}
                other => panic!("hero {other:?}"),
            }
        }
    }
}

/// One plan in: only its owner sees it; prescience shows the asked aspect of the opponent's
/// plan to the prescience user and their ally, and nothing to anyone else.
#[test]
fn battle_plans_before_both_are_in() {
    for fixture in ["prescience-seen", "karma-prescience", "nexus-prescience", "battle", "mc-voice", "mc-prescience"] {
        let mut r = load(fixture, 0);
        assert!(!(r.game.aggressor_plan.is_some() && r.game.defender_plan.is_some()), "{fixture}");
        let users: Vec<Faction> = r.game.current_prescience.iter().map(|p| p.initiator()).chain(r.game.current_nexus_prescience.iter().map(|n| n.initiator())).collect();
        for (f, view) in all_views(&mut r) {
            let g = &r.game;
            let b = view.battle.as_ref().unwrap();
            let ally = f.map(|f| g.player(player(&r, f)).ally);
            for (shown, plan) in [(&b.aggressor_plan, &g.aggressor_plan), (&b.defender_plan, &g.defender_plan)] {
                let own = plan.as_ref().is_some_and(|p| Some(p.initiator()) == f);
                match shown {
                    None => assert!(!own, "{fixture}: {f:?} sees their own plan"),
                    Some(s) if s.full => assert!(own || g.atreides_karma, "{fixture}: {f:?} sees a hidden plan"),
                    Some(s) => {
                        assert!(users.iter().any(|u| f == Some(*u) || ally == Some(*u)), "{fixture}: {f:?} sees prescience it didn't ask");
                        let aspect = g.current_prescience.as_ref().map(|p| p.aspect).or(g.current_nexus_prescience.as_ref().map(|n| n.atreides_prescience_aspect)).unwrap();
                        let shown_aspects = [
                            (PrescienceAspect::Dial, s.dial.is_some()),
                            (PrescienceAspect::Leader, s.hero_shown),
                            (PrescienceAspect::Weapon, s.weapon_shown),
                            (PrescienceAspect::Defense, s.defense_shown),
                        ];
                        for (a, on) in shown_aspects {
                            assert_eq!(on, a == aspect, "{fixture}: {a:?} for prescience of {aspect:?}");
                        }
                        let p = plan.as_ref().unwrap();
                        if aspect == PrescienceAspect::Weapon {
                            assert_eq!(s.weapon, p.weapon().filter(|w| w.is_weapon()).map(|w| w.id()));
                        }
                    }
                }
            }
        }
    }
}

/// In the battle report and outside the battle phase, the map shows no plans.
#[test]
fn no_plans_outside_the_battle() {
    for fixture in ["alliance", "bid", "move"] {
        let view = load(fixture, 0).view(OBSERVER);
        assert!(view.battle.as_ref().is_none_or(|b| b.aggressor_plan.is_none() && b.defender_plan.is_none()), "{fixture}");
    }
}

// ---- The planet: forces, tokens, leaders, worms, recent moves ----

#[test]
fn board_basics() {
    let mut leaders = 0;
    let mut monsters = 0;
    for fixture in FIXTURES {
        let mut r = load(fixture, 0);
        for (f, view) in all_views(&mut r) {
            let g = &r.game;
            let b = &view.board;
            for fv in &b.forces {
                let p = player(&r, Faction::ALL.iter().copied().find(|x| x.to_string() == fv.faction).unwrap());
                let l = g.map.locations(true).find(|l| l.id() == fv.location).unwrap();
                assert_eq!(fv.threatened, g.threatened_by_ally_presence(p, l.territory()));
            }
            let expected: Vec<(String, i32)> = g
                .leader_state
                .iter()
                .filter(|(_, s)| s.alive() && s.current_territory.is_some())
                .map(|(h, s)| (format!("{h:?}"), s.current_territory.unwrap().id()))
                .collect();
            assert_eq!(b.leaders.len(), expected.len());
            leaders += b.leaders.len();
            assert_eq!(b.monsters.iter().map(|m| m.territory).collect::<Vec<_>>(), g.monsters.iter().map(|m| m.territory.id()).collect::<Vec<_>>());
            monsters += b.monsters.len();
            assert_eq!(b.storm_prescience_moves, g.has_storm_prescience(f.map(|f| player(&r, f))).then_some(g.next_storm_moves));
            assert_eq!(b.no_field_value.is_some(), f == Some(Faction::Richese));
            assert_eq!(b.atomics_aftermath, g.atomics_aftermath.map(|t| t.id()));
            assert_eq!(b.number_of_monsters, g.number_of_monsters);
            for l in g.map.locations(false) {
                assert_eq!(b.protected_from_storm.contains(&l.id()), g.is_protected_from_storm(l));
            }
        }
    }
    assert!(leaders > 0 && monsters > 0);
}

/// During ship and move, an arrow from each origin of each recent move to
/// its destination, or along the path when it crosses the Polar Sink.
#[test]
fn recent_moves() {
    let mut arrows = 0;
    for fixture in ["bene_gesserit", "ship-fremen", "facedancer", "move"] {
        let mut r = load(fixture, 0);
        let view = r.view(OBSERVER);
        let g = &r.game;
        let mut expected_ends: Vec<(String, i32)> = vec![];
        for m in &g.recent_moves {
            let to = m.i_placement_to(g).unwrap();
            if !to.is_homeworld() {
                for _ in m.i_placement_force_locations(g).keys() {
                    expected_ends.push((name(m.i_placement_initiator()), to.id()));
                }
            }
        }
        // Each origin ends in exactly one arrow into the destination.
        let ends: Vec<(String, i32)> = view.board.recent_moves.iter().filter(|a| expected_ends.iter().any(|(f, to)| *f == a.faction && *to == a.to)).map(|a| (a.faction.clone(), a.to)).collect();
        assert_eq!(ends.len(), expected_ends.len(), "{fixture}: {:?}", view.board.recent_moves);
        arrows += view.board.recent_moves.len();
    }
    assert!(arrows > 0);
    assert!(load("alliance", 0).view(OBSERVER).board.recent_moves.is_empty());
}

/// Tokens of the expansions, from bot games: stronghold owners, terror (identity to Moritani only),
/// ambassadors, discoveries (identity to the faction of the token's type only), the Hidden
/// Mobile Stronghold and discovered strongholds with their hosts, No-Field (value to Richese only).
#[test]
fn expansion_tokens() {
    let mut r = first_bot_state("terror on the planet", |g| !g.terror_on_planet.is_empty() && g.is_playing(Faction::Moritani));
    for (f, view) in all_views(&mut r) {
        assert_eq!(view.board.terror.len(), r.game.terror_on_planet.len());
        for (t, (token, territory)) in view.board.terror.iter().zip(r.game.terror_on_planet.iter()) {
            assert_eq!(t.territory, territory.id());
            assert_eq!(t.token, (f == Some(Faction::Moritani)).then(|| name(*token)), "{f:?}");
        }
    }

    let mut r = first_bot_state("a discovery its type's faction may identify", |g| {
        g.discoveries_on_planet.iter().any(|(_, d)| {
            (d.token_type == DiscoveryTokenType::Fremen && g.is_playing(Faction::Fremen)) || (d.token_type == DiscoveryTokenType::Guild && g.is_playing(Faction::Guild))
        })
    });
    let mut identified = 0;
    for (f, view) in all_views(&mut r) {
        assert_eq!(view.board.discoveries.len(), r.game.discoveries_on_planet.len());
        for (d, (l, disc)) in view.board.discoveries.iter().zip(r.game.discoveries_on_planet.iter()) {
            assert_eq!((d.location, d.token_type.clone()), (l.id(), name(disc.token_type)));
            let may = (f == Some(Faction::Fremen) && disc.token_type == DiscoveryTokenType::Fremen)
                || (f == Some(Faction::Guild) && disc.token_type == DiscoveryTokenType::Guild);
            assert_eq!(d.token, may.then(|| name(disc.token)));
            identified += may as i32;
        }
    }
    assert!(identified > 0);

    let mut r = first_bot_state("an ambassador on the planet", |g| !g.ambassadors_on_planet.is_empty());
    let view = r.view(OBSERVER);
    assert_eq!(
        view.board.ambassadors.iter().map(|a| (a.territory, a.ambassador.clone())).collect::<Vec<_>>(),
        r.game.ambassadors_on_planet.iter().map(|(t, a)| (t.id(), name(*a))).collect::<Vec<_>>()
    );

    let mut r = first_bot_state("an owned stronghold", |g| !g.stronghold_ownership.is_empty());
    let view = r.view(OBSERVER);
    assert_eq!(
        view.board.strongholds.iter().map(|s| (s.location, s.owner.clone())).collect::<Vec<_>>(),
        r.game.stronghold_ownership.iter().map(|(l, f)| (l.id(), name(*f))).collect::<Vec<_>>()
    );

    let mut r = first_bot_state("the Hidden Mobile Stronghold on the map", |g| g.map.hidden_mobile_stronghold().attached_to_location(&g.map).is_some());
    let view = r.view(OBSERVER);
    let hms = r.game.map.hidden_mobile_stronghold();
    let shown = view.board.attached.iter().find(|a| a.location == hms.location().id()).expect("the HMS is listed");
    assert_eq!(shown.attached_to, hms.attached_to_location(&r.game.map).unwrap().id());
    assert_eq!(shown.sector, hms.location().sector(&r.game.map));

    let mut r = first_bot_state("a No-Field on the planet", |g| g.current_no_field_value >= 0 && g.is_playing(Faction::Richese));
    for (f, view) in all_views(&mut r) {
        assert_eq!(view.board.no_field_value, (f == Some(Faction::Richese)).then_some(r.game.current_no_field_value), "{f:?}");
    }
}

// ---- Times and the prompt's timer ----

#[test]
fn timing() {
    let mut r = load("bid", 0);
    r.timer = 30;
    let view = r.view(OBSERVER);
    let g = &r.game;
    assert_eq!(view.timing.started, g.history[0].time().to_json_string());
    assert_eq!(view.timing.last_activity, Some(g.history.last().unwrap().time().to_json_string()));
    assert_eq!(view.timing.duration, g.duration().ticks as f64 / 1e7);
    assert_eq!(view.timing.timer, 30);
    assert_eq!(view.timing.time_spent.len(), g.players.len());
    for (t, p) in view.timing.time_spent.iter().zip(&g.players) {
        assert_eq!(t.bidding, g.time_spent(p.faction, MainPhase::Bidding).ticks as f64 / 1e7);
        assert_eq!(t.battle, g.time_spent(p.faction, MainPhase::Battle).ticks as f64 / 1e7);
    }
    assert!(view.timing.time_spent.iter().any(|t| t.bidding > 0.0));
    assert_eq!(view.ruleset, name(g.ruleset));
    assert_eq!(view.rules, g.rules.iter().map(|r| name(*r)).collect::<Vec<_>>());
    assert_eq!(view.economics_status, name(g.economics_status));
}

/// The observer's view holds nothing that only a player may see, in any fixture.
#[test]
fn observer_sees_no_private_fields() {
    for fixture in FIXTURES {
        let view = load(fixture, 0).view(OBSERVER);
        assert!(view.me.is_none() && view.notes.is_none(), "{fixture}");
        assert!(view.piles.richese_cache.is_none() && view.piles.resource_deck_top.is_none() && view.piles.treachery_discard.is_none());
        assert!(view.board.no_field_value.is_none() && view.board.storm_prescience_moves.is_none());
        assert!(view.board.terror.iter().all(|t| t.token.is_none()) && view.board.discoveries.iter().all(|d| d.token.is_none()));
        assert!(view.tanks.iter().all(|t| t.leaders.iter().all(|l| !l.face_down || l.id.is_none())));
        if let Some(b) = &view.battle {
            for p in [&b.aggressor_plan, &b.defender_plan].into_iter().flatten() {
                assert!(p.full, "{fixture}: an observer sees only revealed plans");
            }
        }
    }
}
