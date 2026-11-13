//! C# null inside collections of game objects (engine-conventions.md, "Null
//! handles"). Each handle type has a null value that must behave like a C#
//! null reference wherever one can travel: in lists, sets and decks, as a
//! dictionary key, in the state dump, in messages and as a JSON id. The .NET
//! behaviour asserted here was checked with a scratch program against .NET 10;
//! member access on null is covered for every member by
//! `boundary.rs::null_receivers_throw_null_reference`.

use treachery_engine::canon::dump_detached;
use treachery_engine::compat::cs::{self, CsNull, OrNull};
use treachery_engine::messages::expression::ExpressionElement;
use treachery_engine::prelude::*;
use treachery_engine::game::PlayerIndex;

fn err(f: impl FnOnce()) -> String {
    match cs::catch(f) {
        Ok(()) => "ok".into(),
        Err(e) => format!("{}: {}", e.type_name, e.message),
    }
}

const NRE: &str = "NullReferenceException: Object reference not set to an instance of an object.";
const NULL_KEY: &str = "ArgumentNullException: Value cannot be null. (Parameter 'key')";
const NULL_SOURCE: &str = "ArgumentNullException: Value cannot be null. (Parameter 'source')";

#[test]
fn null_equals_only_null() {
    let card = TreacheryCardManager::items()[0];
    assert_eq!(TreacheryCard::NULL, TreacheryCard::NULL);
    assert_ne!(TreacheryCard::NULL, card);
    assert_ne!(Location::NULL, Location::from_id(0).unwrap());
    assert_ne!(Territory::NULL, Territory::from_id(0).unwrap());
    assert_ne!(Leader::NULL, LeaderManager::leaders()[0]);
    assert_eq!(PlayerIndex::NULL.opt(), None);
    // Every null hero is the same null, however it was built.
    assert_eq!(Hero::Leader(Leader::NULL), Hero::NULL);
    assert_eq!(Hero::from(Leader::NULL), Hero::NULL);
    assert_eq!(Hero::from(TreacheryCard::NULL), Hero::NULL);
    assert!(Hero::NULL.as_leader().is_none() && Hero::NULL.as_treachery_card().is_none() && !Hero::NULL.is_messiah());
    let mut set = NetHashSet::new();
    set.add(Hero::Leader(Leader::NULL));
    assert!(set.contains(&Hero::NULL));
}

#[test]
fn null_in_lists_like_csharp() {
    let a = TreacheryCardManager::items()[0];
    // new List { a, null }: IndexOf(null) is 1, Remove(null) removes it.
    let mut hand = vec![a, None.or_null()];
    assert_eq!(hand.iter().position(|c| *c == TreacheryCard::NULL), Some(1));
    assert!(cs::list_remove(&mut hand, None));
    assert_eq!(hand, vec![a]);
    assert!(!cs::list_remove(&mut hand, None));
    // HashSet<T>.Add(null) works, Contains(null) is true.
    let mut set = NetHashSet::new();
    assert!(set.add(TreacheryCard::NULL));
    assert!(set.contains(&TreacheryCard::NULL));
    // LINQ over a list with null: the selector's member access throws.
    assert_eq!(err(|| { let _: i32 = hand.iter().chain([TreacheryCard::NULL].iter()).map(|c| c.id()).sum(); }), NRE);
}

#[test]
fn null_dictionary_keys_throw_argument_null() {
    cs::install_null_key_check();
    let mut d: NetDictionary<Location, i32> = NetDictionary::new();
    let l = Location::NULL;
    assert_eq!(err(|| d.clone().add(l, 1)), NULL_KEY);
    assert_eq!(err(|| { d.get(&l); }), NULL_KEY);
    assert_eq!(err(|| { d.contains_key(&l); }), NULL_KEY);
    assert_eq!(err(|| { d.clone().remove(&l); }), NULL_KEY);
    assert_eq!(err(|| { d.clone().try_add(l, 1); }), NULL_KEY);
    assert_eq!(err(|| d.clone().insert(l, 1)), NULL_KEY);
    assert_eq!(err(|| { d.get_value_or(&l, 0); }), NULL_KEY);
    d.add(Location::from_id(3).unwrap(), 1);
    let mut h: NetDictionary<Hero, i32> = NetDictionary::new();
    assert_eq!(err(|| { h.get(&Hero::Leader(Leader::NULL)); }), NULL_KEY);
    h.add(Hero::Messiah, 1);
    assert_eq!(err(|| { NetDictionary::<Territory, i32>::new().index_with(&Territory::NULL, |_| String::new()); }), NULL_KEY);
}

#[test]
fn null_dumps_as_null() {
    let a = TreacheryCardManager::items()[3];
    let hand = vec![TreacheryCard::NULL, a];
    assert_eq!(String::from_utf8(dump_detached(&hand, None)).unwrap(), r#"[null,{"$ref":"TreacheryCard","id":3}]"#);
    assert_eq!(String::from_utf8(dump_detached(&vec![Location::NULL], None)).unwrap(), "[null]");
    assert_eq!(String::from_utf8(dump_detached(&vec![Territory::NULL], None)).unwrap(), "[null]");
    assert_eq!(String::from_utf8(dump_detached(&vec![Hero::NULL, Hero::Leader(Leader::NULL)], None)).unwrap(), "[null,null]");
    assert_eq!(String::from_utf8(dump_detached(&vec![ResourceCard::NULL], None)).unwrap(), "[null]");
    assert_eq!(String::from_utf8(dump_detached(&vec![PlayerIndex::NULL], None)).unwrap(), "[null]");
}

#[test]
fn null_in_messages_and_json_ids() {
    assert!(matches!(ExpressionElement::from(TreacheryCard::NULL), ExpressionElement::Null));
    assert!(matches!(ExpressionElement::from(Leader::NULL), ExpressionElement::Null));
    assert!(matches!(ExpressionElement::from(Location::NULL), ExpressionElement::Null));
    assert!(matches!(ExpressionElement::from(Territory::NULL), ExpressionElement::Null));
    assert!(matches!(ExpressionElement::from(PlayerIndex::NULL), ExpressionElement::Null));
    // A list with a null element prints like List<TreacheryCard> and keeps the null.
    let e = ExpressionElement::from(vec![TreacheryCard::NULL]);
    match e {
        ExpressionElement::List { items, .. } => assert!(matches!(items[0], ExpressionElement::Null)),
        _ => panic!("not a list"),
    }
    // IFetcher.GetId(null) is -1.
    assert_eq!(TreacheryCardManager::get_id(Some(TreacheryCard::NULL)), -1);
    assert_eq!(LeaderManager::leader_lookup().get_id(Some(Leader::NULL)), -1);
    assert_eq!(LeaderManager::hero_lookup().get_id(Some(Hero::NULL)), -1);
    let map = Map::new();
    assert_eq!(map.location_lookup().get_id(Some(Location::NULL)), -1);
    assert_eq!(map.territory_lookup().get_id(Some(Territory::NULL)), -1);
}

#[test]
fn null_linq_sources_throw_argument_null() {
    assert_eq!(err(|| { cs::linq_source::<Vec<i32>>(None); }), NULL_SOURCE);
    // EmperorBidSupport.GetMessage: Amounts.Sum(...) on a null Amounts.
    let e = EmperorBidSupport { amounts: None, ..Default::default() };
    let g = Game::new();
    assert_eq!(err(|| { e.get_message(&g); }), NULL_SOURCE);
}

#[test]
fn null_player_throws_on_access() {
    let g = Game::new();
    assert_eq!(err(|| { g.player(PlayerIndex::NULL); }), NRE);
}
