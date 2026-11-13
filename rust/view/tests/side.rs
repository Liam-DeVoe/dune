//! The side column's own view fields: `deals`, `tracker`, `botsSpeed`, `mentat`, the leaders'
//! `skill`, the undo list's `listed`/`turn`/`phase` and the undo preview. Each test checks the
//! field against the engine, and that no viewer gets what they may not see.

mod common;

use std::rc::Rc;

use common::{fixture, load, load_text, truncated, viewer_of, OBSERVER};
use treachery_engine::events::DealAccepted;
use treachery_engine::heroes::hero::Hero;
use treachery_engine::identity::IIdentifiable;
use treachery_engine::prelude::*;
use treachery_engine::cards::treachery_card_manager::TreacheryCardManager;
use treachery_view::replica::ViewReplica;
use treachery_view::schema::*;

fn name<T: ToString>(v: T) -> String {
    v.to_string()
}

fn factions(r: &ViewReplica) -> Vec<Faction> {
    r.game.players.iter().map(|p| p.faction).collect()
}

/// Every seated player's view and the observer's, with the viewer's faction.
fn all_views(r: &mut ViewReplica) -> Vec<(Option<Faction>, GameView)> {
    let mut out: Vec<(Option<Faction>, GameView)> = factions(r).into_iter().map(|f| (Some(f), r.view(viewer_of(r, f)))).collect();
    out.push((None, r.view(OBSERVER)));
    out
}

/// Calls `each` on every state of a fixture, from the first event on, with everyone seated.
fn each_state(name: &str, mut each: impl FnMut(&mut ViewReplica, usize)) {
    let v: serde_json::Value = serde_json::from_str(&fixture(name)).unwrap();
    let events = v["_events"].as_array().unwrap();
    let mut r = load_text(&truncated(name, 1), 0);
    each(&mut r, 1);
    for (i, e) in events.iter().enumerate().skip(1) {
        r.apply(&e.to_string()).unwrap_or_else(|m| panic!("{name} event {i}: {m}"));
        each(&mut r, i + 1);
    }
}

fn parse_event(v: &serde_json::Value) -> GameEvent {
    treachery_engine::json::deserialize(&v.to_string()).unwrap_or_else(|e| panic!("{v}: {e:?}"))
}

/// The offer an id names: the `DealOffered` at that place in the history.
fn offer_at(game: &Game, id: i32) -> Rc<treachery_engine::events::DealOffered> {
    match &game.history[id as usize] {
        GameEvent::DealOffered(o) => o.clone(),
        e => panic!("deal id {id} is a {}", e.event_type().name()),
    }
}

// ---- Deals (the panel, the menu's deal count, the toasts) ----

/// Over every state of four fixtures with many offers: observers get no deals; a player gets
/// exactly `AcceptableDeals` (offers addressed to them) and `CancellableDeals` (their own),
/// each recipient's answer only on their own offers, and `CurrentDeals`, which everyone gets
/// alike. The Accept/Decline/Withdraw events are the engine's, for the viewer, and the Accept
/// button is disabled exactly when `Acceptance(...).Validate()` fails.
#[test]
fn deals() {
    let mut seen = [0usize; 5];
    for fx in ["blow", "ally", "alliance-allied", "ship-guild"] {
        each_state(fx, |r, step| {
            let at = format!("{fx} at {step}");
            let mut current: Option<Vec<CurrentDeal>> = None;
            for (f, view) in all_views(r) {
                let g = &r.game;
                let Some(f) = f else {
                    assert!(view.deals.is_none(), "{at}: deals for an observer");
                    continue;
                };
                let p = g.get_player(Some(f)).unwrap();
                let d = view.deals.as_ref().unwrap_or_else(|| panic!("{at}: no deals for {f}"));

                let expected: Vec<i32> = DealAccepted::acceptable_deals(g, Some(p)).map(|o| g.history.iter().position(|e| matches!(e, GameEvent::DealOffered(x) if Rc::ptr_eq(x, &o))).unwrap() as i32).collect();
                assert_eq!(d.acceptable.iter().map(|o| o.id).collect::<Vec<_>>(), expected, "{at} {f}");
                for o in &d.acceptable {
                    let offer = offer_at(g, o.id);
                    // Never someone else's offer to a third party.
                    assert!(g.deal_recipients(&offer).any(|x| x == f), "{at} {f}: an offer not addressed to the viewer");
                    assert_eq!(o.initiator, name(offer.initiator()));
                    assert_eq!(o.price, offer.price);
                    assert!(o.answers.is_empty() && o.withdraw.is_none(), "{at} {f}: answers to someone else's offer");
                    let accept = parse_event(&o.accept);
                    assert!(matches!(accept, GameEvent::DealAccepted(_)), "{at}");
                    assert_eq!(accept.initiator(), f);
                    let acceptance = GameEvent::from(offer.acceptance(g, f).unwrap());
                    assert_eq!(o.accept_error.is_some(), acceptance.validate(g).is_some(), "{at} {f}: Accept enabled");
                    let decline = parse_event(&o.decline);
                    assert!(matches!(decline, GameEvent::DealRejected(_)) && decline.initiator() == f, "{at}");
                    seen[0] += 1;
                    seen[1] += o.accept_error.is_some() as usize;
                }

                let own: Vec<_> = DealAccepted::cancellable_deals(g, Some(p)).collect();
                assert_eq!(d.cancellable.len(), own.len(), "{at} {f}");
                for (o, offer) in d.cancellable.iter().zip(&own) {
                    assert!(Rc::ptr_eq(&offer_at(g, o.id), offer), "{at} {f}");
                    assert_eq!(o.initiator, name(f), "{at}: someone else's offer as one's own");
                    let withdraw = parse_event(o.withdraw.as_ref().expect("Withdraw"));
                    assert!(matches!(withdraw, GameEvent::DealOffered(ref w) if w.cancel), "{at}");
                    assert_eq!(withdraw.initiator(), f);
                    let answers: Vec<DealAnswer> = g.deal_recipients(offer).map(|x| DealAnswer { faction: name(x), accepted: g.deal_response(offer, x) }).collect();
                    assert_eq!(o.answers, answers, "{at} {f}");
                    seen[2] += 1;
                    seen[3] += o.answers.iter().any(|a| a.accepted.is_some()) as usize;
                }

                let deals_now: Vec<(String, String)> = DealAccepted::current_deals(g).iter().map(|x| (name(x.bound_faction), name(x.consuming_faction))).collect();
                assert_eq!(d.current.iter().map(|x| (x.bound.clone(), x.consuming.clone())).collect::<Vec<_>>(), deals_now, "{at}");
                seen[4] += d.current.len();
                // Current deals are the same for every player.
                match &current {
                    None => current = Some(d.current.clone()),
                    Some(c) => assert_eq!(c, &d.current, "{at}"),
                }
            }
        });
    }
    let [acceptable, refused, own, answered, current] = seen;
    assert!(acceptable > 0 && refused > 0 && own > 0 && answered > 0 && current > 0, "{seen:?}");
}

// ---- Atreides's card tracker ----

/// Only Atreides gets the tracker, from `Phase.SelectingTraitors` on. Its contents are the
/// engine's; the resource deck is a sorted set, so its order is not given away.
#[test]
fn tracker() {
    let mut shown = 0;
    for fx in ["blow", "ally", "mc-discard"] {
        each_state(fx, |r, step| {
            if step % 5 != 0 {
                return;
            }
            let at = format!("{fx} at {step}");
            for (f, view) in all_views(r) {
                let g = &r.game;
                let expected = f == Some(Faction::Atreides) && g.current_phase >= Phase::SelectingTraitors;
                assert_eq!(view.tracker.is_some(), expected, "{at} {f:?}");
                let Some(t) = &view.tracker else { continue };
                shown += 1;
                let mut cards: Vec<i32> = TreacheryCardManager::get_cards_in_play(g).map(|c| c.id()).collect();
                if g.is_playing(Faction::Richese) {
                    cards.extend(TreacheryCardManager::get_richese_cards().iter().map(|c| c.id()).filter(|id| !cards.contains(id)).collect::<Vec<_>>());
                }
                assert_eq!(t.cards_in_play, cards, "{at}");
                let mut deck: Vec<i32> = g.resource_card_deck.as_ref().map(|d| d.items.iter().map(|c| c.skin_id()).collect()).unwrap_or_default();
                deck.sort();
                assert_eq!(t.resource_cards, deck, "{at}");
                let pile = |d: &Option<Deck<ResourceCard>>| d.as_ref().map(|d| d.items.iter().map(|c| c.skin_id()).collect::<Vec<_>>()).unwrap_or_default();
                assert_eq!(t.resource_discard_a, pile(&g.resource_card_discard_pile_a));
                assert_eq!(t.resource_discard_b, pile(&g.resource_card_discard_pile_b));
                assert_eq!(t.traitors_in_play.len(), g.traitors_in_play().len());
                assert_eq!(t.players.len(), g.players.len());
                for (tp, pl) in t.players.iter().zip(&g.players) {
                    assert_eq!(tp.faction, name(pl.faction));
                    assert_eq!(tp.maximum_number_of_cards, pl.maximum_number_of_cards(g));
                    assert_eq!(tp.number_of_traitors, pl.number_of_traitors());
                    assert_eq!(tp.number_of_face_dancers, pl.number_of_face_dancers());
                }
            }
        });
    }
    assert!(shown > 0);
}

// ---- Bot speed ----

/// `Participation.BotsSpeed`, the same for everyone, following the participation.
#[test]
fn bots_speed() {
    let mut r = load("bid", 0);
    let initial = r.game.participation.borrow().bots_speed;
    for speed in [initial, 0, 2, 4] {
        r.game.participation.borrow_mut().bots_speed = speed;
        for (f, view) in all_views(&mut r) {
            assert_eq!(view.bots_speed, speed, "{f:?}");
        }
    }
}

// ---- The menu's "List of Cards" ----

/// The cards in play, from the engine, the same for every viewer; Richese's cards only when
/// Richese plays.
#[test]
fn mentat() {
    let mut saw_richese = false;
    for fx in ["bid", "blow", "ally", "nexus-moritani", "retreat", "worm"] {
        let mut r = load(fx, 0);
        let views = all_views(&mut r);
        let g = &r.game;
        let expected = MentatView {
            treachery_cards: TreacheryCardManager::get_cards_in_play(g).map(|c| c.id()).collect(),
            richese_cards: if g.is_playing(Faction::Richese) { TreacheryCardManager::get_richese_cards().iter().map(|c| c.id()).collect() } else { vec![] },
            resource_cards: Map::get_resource_cards_in_play(g).map(|c| c.skin_id()).collect(),
        };
        saw_richese |= !expected.richese_cards.is_empty();
        assert!(!expected.treachery_cards.is_empty() && !expected.resource_cards.is_empty());
        for (f, view) in views {
            assert_eq!(view.mentat, expected, "{fx} {f:?}");
        }
    }
    let _ = saw_richese;
}

// ---- Leader skills (popovers) ----

/// `OwnLeader.skill` and the traitors' and face dancers' `skill` are `Game.Skill` of the leader.
/// A skill is public once assigned (`SkillAssigned` logs it to everyone), so traitors may show
/// the skill of another faction's leader.
#[test]
fn leader_skills() {
    let mut own = 0;
    let mut traitors = 0;
    each_state("retreat", |r, step| {
        if step % 10 != 0 {
            return;
        }
        for (f, view) in all_views(r) {
            let Some(me) = &view.me else {
                assert!(f.is_none());
                continue;
            };
            let g = &r.game;
            for l in &me.leaders {
                let leader = treachery_engine::heroes::leader_manager::LeaderManager::leaders().iter().copied().find(|x| x.id() == l.id).unwrap();
                let skill = g.skill(Some(Hero::Leader(leader)));
                assert_eq!(l.skill, (skill != LeaderSkill::None).then(|| name(skill)), "{f:?} at {step}");
                own += l.skill.is_some() as usize;
            }
            for c in me.traitors.iter().chain(&me.face_dancers) {
                let HeroRef::Leader { id } = c.hero else {
                    assert!(c.skill.is_none());
                    continue;
                };
                let leader = treachery_engine::heroes::leader_manager::LeaderManager::leaders().iter().copied().find(|x| x.id() == id).unwrap();
                let skill = g.skill(Some(Hero::Leader(leader)));
                assert_eq!(c.skill, (skill != LeaderSkill::None).then(|| name(skill)), "{f:?} at {step}");
                traitors += c.skill.is_some() as usize;
            }
        }
    });
    assert!(own > 0, "retreat has skilled leaders");
    let _ = traitors;
}

// ---- The undo list (RequestUndoList) ----

/// One entry per event, numbered by its place in the history, with its short message, whether
/// the undo panel lists it, and the turn and phase from `moments[n - 1]`.
#[test]
fn undo_list() {
    for fx in ["blow", "retreat", "mc-ended"] {
        let r = load(fx, 0);
        let list = r.undo_list();
        let g = &r.game;
        assert_eq!(list.len(), g.history.len(), "{fx}");
        let mut unlisted = 0;
        for (i, (entry, e)) in list.iter().zip(&g.history).enumerate() {
            assert_eq!(entry.event_number, i as i32);
            let listed = !matches!(e, GameEvent::AllyPermission(_) | GameEvent::BattleRevision(_) | GameEvent::AutomationConfigured(_));
            assert_eq!(entry.listed, listed, "{fx} {i}");
            unlisted += !listed as usize;
            if i == 0 {
                assert_eq!((entry.turn, entry.phase.as_str()), (0, "None"));
            } else {
                let m = &g.moments[i - 1];
                assert_eq!((entry.turn, entry.phase.clone()), (m.turn, name(m.phase)), "{fx} {i}");
            }
            assert!(!entry.text.is_empty(), "{fx} {i}");
        }
        // Turns never go back, and the last entries are in the current turn.
        assert!(list.windows(2).all(|w| w[0].turn <= w[1].turn), "{fx}");
        assert!(list.last().unwrap().turn <= g.current_turn);
        if fx == "blow" {
            assert!(unlisted > 0, "blow has ally permissions");
        }
    }
}

// ---- The undo preview (RequestUndoPreview) ----

fn without_actions(mut v: GameView) -> GameView {
    v.actions.clear();
    if let Some(me) = &mut v.me {
        for c in &mut me.hand {
            c.playable_with.clear();
        }
    }
    v
}

/// The preview at gap n is the view each viewer had after events 0..n (a replica loaded from the
/// savegame cut at n), without actions or card plays, for any viewer; gaps are clamped to
/// 1..=History.Count; the replica itself is not changed.
#[test]
fn undo_preview() {
    for fx in ["bid", "ally", "mc-treachery"] {
        let mut r = load(fx, 0);
        let count = r.game.history.len() as i32;
        let live = all_views(&mut r);
        let viewers: Vec<_> = factions(&r).into_iter().map(|f| viewer_of(&r, f)).chain([OBSERVER]).collect();
        for gap in [1, count / 3, count / 2, count - 1] {
            let mut cut = load_text(&truncated(fx, gap as i64), 0);
            for v in &viewers {
                let preview = r.preview(*v, gap).unwrap();
                assert_eq!(preview.event_number, gap, "{fx}");
                assert!(preview.actions.is_empty() && preview.me.as_ref().is_none_or(|m| m.hand.iter().all(|c| c.playable_with.is_empty())));
                assert_eq!(preview, without_actions(cut.view(*v)), "{fx} gap {gap} user {}", v.user_id);
            }
        }
        // Clamped: 0 is gap 1, past now is now.
        let host = viewers[0];
        assert_eq!(r.preview(host, 0).unwrap().event_number, 1);
        assert_eq!(r.preview(host, count + 10).unwrap(), without_actions(r.view(host)));
        // The live views are unchanged.
        assert_eq!(all_views(&mut r), live, "{fx}");
    }
}
