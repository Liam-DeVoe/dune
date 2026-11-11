//! `Utilities.Serialize`/`Deserialize` for the server's types, on top of the engine's
//! byte-exact System.Text.Json writer (`treachery_engine::json`).
//!
//! The engine covers events, `GameSettings` and `GameState`. This adds what the server stores
//! in the database or returns from hub methods: `Dictionary<int, V>` (keys as decimal
//! strings), `HashSet<int>`, `Participation`, and the `Shared/Model` classes in [`crate::model`].

use dotnet_compat::{NetDictionary, NetHashSet};
use treachery_engine::json::{FromJson, JsonError, JsonValue, JsonWriter, ToJson};
use treachery_engine::participation::Participation;

thread_local! {
    static WIRE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `f` with the WebSocket protocol's encoding of `Dictionary<int, V>` (protocol.md,
/// Encoding): an array of `[key, value]` pairs, so a client keeps the server's order
/// (`JSON.parse` would reorder an object's integer keys by value). Only the transport uses it;
/// the database and the equivalence transcripts keep the C# object form.
pub fn wire<R>(f: impl FnOnce() -> R) -> R {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            WIRE.with(|c| c.set(self.0));
        }
    }
    let _reset = Reset(WIRE.with(|c| c.replace(true)));
    f()
}

/// Writes `Dictionary<int, V>` in the dictionary's enumeration order: `{"12":value,...}` as
/// `Utilities.Serialize` does, or `[[12,value],...]` inside [`wire`].
pub fn write_int_dict<V: ToJson>(w: &mut JsonWriter, d: &NetDictionary<i32, V>) {
    if WIRE.with(|c| c.get()) {
        w.array_value();
        for (k, v) in d.iter() {
            w.array_value();
            w.int(*k as i64);
            v.write_json(w);
            w.end_array();
        }
        w.end_array();
        return;
    }
    w.object_value();
    for (k, v) in d.iter() {
        w.key(&k.to_string());
        v.write_json(w);
    }
    w.end_object();
}

/// Reads `Dictionary<int, V>`. .NET parses the key with `int.Parse` semantics
/// (System.Text.Json number-key handling); package P checks the corner cases against .NET.
pub fn read_int_dict<V: FromJson>(v: &JsonValue) -> Result<NetDictionary<i32, V>, JsonError> {
    let JsonValue::Object(map) = v else {
        return Err(JsonError::new("JsonException", "expected an object"));
    };
    let mut d = NetDictionary::new();
    for (k, v) in map {
        let key: i32 = k.parse().map_err(|_| JsonError::new("JsonException", format!("invalid key {k}")))?;
        d.add(key, V::from_json(v)?);
    }
    Ok(d)
}

/// Reads a `Dictionary<int, V>` sent by a client: the wire's `[[key, value],...]`, or the C#
/// object form.
pub fn read_wire_int_dict<V: FromJson>(v: &JsonValue) -> Result<NetDictionary<i32, V>, JsonError> {
    let JsonValue::Array(pairs) = v else {
        return read_int_dict(v);
    };
    let mut d = NetDictionary::new();
    for pair in pairs {
        let JsonValue::Array(kv) = pair else {
            return Err(JsonError::new("JsonException", "expected a [key, value] pair"));
        };
        let [k, v] = kv.as_slice() else {
            return Err(JsonError::new("JsonException", "expected a [key, value] pair"));
        };
        d.add(i32::from_json(k)?, V::from_json(v)?);
    }
    Ok(d)
}

/// Writes `HashSet<int>` as an array in enumeration order.
pub fn write_int_set(w: &mut JsonWriter, s: &NetHashSet<i32>) {
    w.array_value();
    for i in s.iter() {
        i.write_json(w);
    }
    w.end_array();
}

pub fn read_int_set(v: &JsonValue) -> Result<NetHashSet<i32>, JsonError> {
    let JsonValue::Array(items) = v else {
        return Err(JsonError::new("JsonException", "expected an array"));
    };
    let mut s = NetHashSet::new();
    for item in items {
        s.add(i32::from_json(item)?);
    }
    Ok(s)
}

/// `Utilities.Serialize(participation)`, the `PersistedGames.GameParticipation` column.
pub fn serialize_participation(p: &Participation) -> String {
    let mut w = JsonWriter::new();
    write_participation(&mut w, p);
    w.finish()
}

pub fn write_participation(w: &mut JsonWriter, p: &Participation) {
    w.object_value();
    w.key("PlayerNames");
    write_int_dict(w, &p.player_names);
    w.key("SeatedPlayers");
    write_int_dict(w, &p.seated_players);
    w.key("Observers");
    write_int_set(w, &p.observers);
    w.key("Hosts");
    write_int_set(w, &p.hosts);
    w.key("AvailableSeats");
    write_int_set(w, &p.available_seats);
    w.key("Kicked");
    write_int_set(w, &p.kicked);
    w.member("BotsSpeed", &p.bots_speed);
    w.end_object();
}

/// `Utilities.Deserialize<Participation>(text)`: missing members keep their initializers.
pub fn deserialize_participation(text: &str) -> Result<Participation, JsonError> {
    let v: JsonValue = serde_json::from_str(text).map_err(|e| JsonError::new("JsonException", e.to_string()))?;
    let JsonValue::Object(map) = &v else {
        return Err(JsonError::new("JsonException", "expected an object"));
    };
    let mut p = Participation::new();
    for (k, v) in map {
        match k.as_str() {
            "PlayerNames" => p.player_names = read_int_dict(v)?,
            "SeatedPlayers" => p.seated_players = read_int_dict(v)?,
            "Observers" => p.observers = read_int_set(v)?,
            "Hosts" => p.hosts = read_int_set(v)?,
            "AvailableSeats" => p.available_seats = read_int_set(v)?,
            "Kicked" => p.kicked = read_int_set(v)?,
            "BotsSpeed" => p.bots_speed = i32::from_json(v)?,
            _ => {}
        }
    }
    Ok(p)
}

/// Serializes any [`ToJson`] value: `Utilities.Serialize(value)`.
pub fn serialize<T: ToJson + ?Sized>(value: &T) -> String {
    treachery_engine::json::serialize(value)
}
