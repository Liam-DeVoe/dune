//! Checks of API behavior that the golden tests don't exercise (serde, trait impls, panics that model .NET exceptions).

use dotnet_compat::{NetDictionary, NetHashSet, math};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// The .NET exception a panic stands for.
struct Thrown {
    type_name: String,
    message: String,
}

impl std::fmt::Display for Thrown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.type_name, self.message)
    }
}

fn thrown(f: impl FnOnce()) -> Thrown {
    let payload = catch_unwind(AssertUnwindSafe(f)).expect_err("expected a .NET exception");
    let text = dotnet_compat::exception::panic_message(payload.as_ref());
    let (type_name, message) = text.split_once(": ").expect("panic message is \"Type: message\"");
    Thrown { type_name: type_name.to_string(), message: message.to_string() }
}

#[test]
fn dictionary_serde_preserves_slot_order() {
    let mut d = NetDictionary::new();
    for k in [5, 1, 9, 3] {
        d.add(k, k * 10);
    }
    d.remove(&1);
    d.insert(7, 70); // reuses slot 1
    let json = serde_json::to_string(&d).unwrap();
    assert_eq!(json, r#"{"5":50,"7":70,"9":90,"3":30}"#);
    let back: NetDictionary<i32, i32> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, d);
    assert_eq!(back.debug_slot_count(), 4);
    // Duplicate keys: first slot, last value (indexer semantics).
    let dup: NetDictionary<String, i32> = serde_json::from_str(r#"{"a":1,"b":2,"a":3}"#).unwrap();
    assert_eq!(dup.iter().map(|(k, v)| (k.as_str(), *v)).collect::<Vec<_>>(), vec![("a", 3), ("b", 2)]);
}

#[test]
fn hashset_serde_preserves_slot_order() {
    let mut s = NetHashSet::new();
    for x in [4, 2, 8] {
        s.add(x);
    }
    s.remove(&4);
    s.add(6);
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(json, "[6,2,8]");
    let back: NetHashSet<i32> = serde_json::from_str("[1,2,1,3]").unwrap();
    assert_eq!(back.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3]);
}

#[test]
fn clone_keeps_free_list_but_copy_constructor_compacts() {
    let mut d = NetDictionary::new();
    for k in 0..4 {
        d.add(k, ());
    }
    d.remove(&1);
    let mut cloned = d.clone();
    let mut copied = NetDictionary::from_dictionary(&d);
    cloned.add(10, ());
    copied.add(10, ());
    assert_eq!(cloned.keys().copied().collect::<Vec<_>>(), vec![0, 10, 2, 3]);
    assert_eq!(copied.keys().copied().collect::<Vec<_>>(), vec![0, 2, 3, 10]);
}

#[test]
fn dictionary_add_duplicate_throws() {
    let mut d = NetDictionary::new();
    d.add("a".to_string(), 1);
    let e = thrown(|| d.add("a".to_string(), 2));
    assert_eq!(e.type_name, "ArgumentException");
    assert_eq!(e.message, "An item with the same key has already been added. Key: a");
    // Keys without a CsToString impl supply the text themselves.
    let mut d2: NetDictionary<(i32, i32), ()> = NetDictionary::new();
    d2.add_with((1, 2), (), |_| "(1, 2)".into());
    let e = thrown(|| d2.add_with((1, 2), (), |_| "(1, 2)".into()));
    assert_eq!(e.message, "An item with the same key has already been added. Key: (1, 2)");
}

#[test]
fn dictionary_index_missing_throws() {
    let d: NetDictionary<i32, i32> = NetDictionary::new();
    let e = thrown(|| {
        let _ = d[&1];
    });
    assert_eq!(e.to_string(), "KeyNotFoundException: The given key '1' was not present in the dictionary.");
}

#[test]
fn formatting_examples() {
    assert_eq!(math::format_f32(0.5), "0.5");
    assert_eq!(math::format_f32(-0.0), "-0");
    assert_eq!(math::format_f32(1e9), "1E+09");
    assert_eq!(math::format_f32(123456790.0), "123456790");
    assert_eq!(math::format_f64(1e16), "10000000000000000");
    assert_eq!(math::format_f64(1e17), "1E+17");
    assert_eq!(math::format_f64(1e15), "1000000000000000");
    assert_eq!(math::format_f64(0.0001), "0.0001");
    assert_eq!(math::format_f64(0.00001), "1E-05");
}
