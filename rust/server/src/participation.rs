//! A game's participation (who plays which seat, observers, hosts, open seats, kicked users,
//! bot speed) as JSON: on the wire with integer-keyed maps as `[key, value]` pairs, and in the
//! database with them as objects (`{"7": 0}`), the form older databases hold.

use serde_json::{json, Map, Value};
use treachery_engine::game::Participation;

fn ids<'a>(set: impl IntoIterator<Item = &'a i32>) -> Value {
    Value::Array(set.into_iter().map(|i| json!(i)).collect())
}

/// The wire form (protocol.md, Payload objects).
pub fn to_wire(p: &Participation) -> Value {
    json!({
        "PlayerNames": p.player_names.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>(),
        "SeatedPlayers": p.seated_players.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>(),
        "Observers": ids(p.observers.iter()),
        "Hosts": ids(p.hosts.iter()),
        "AvailableSeats": ids(p.available_seats.iter()),
        "Kicked": ids(p.kicked.iter()),
        "BotsSpeed": p.bots_speed,
    })
}

/// The stored form.
pub fn to_json(p: &Participation) -> String {
    let object = |pairs: Vec<(String, Value)>| Value::Object(pairs.into_iter().collect::<Map<_, _>>());
    json!({
        "PlayerNames": object(p.player_names.iter().map(|(k, v)| (k.to_string(), json!(v))).collect()),
        "SeatedPlayers": object(p.seated_players.iter().map(|(k, v)| (k.to_string(), json!(v))).collect()),
        "Observers": ids(p.observers.iter()),
        "Hosts": ids(p.hosts.iter()),
        "AvailableSeats": ids(p.available_seats.iter()),
        "Kicked": ids(p.kicked.iter()),
        "BotsSpeed": p.bots_speed,
    })
    .to_string()
}

/// Reads the stored form. Missing members keep their defaults.
pub fn from_json(text: &str) -> Result<Participation, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let Value::Object(map) = v else { return Err("participation: not an object".into()) };
    let int = |v: &Value| v.as_i64().and_then(|n| i32::try_from(n).ok()).ok_or_else(|| format!("not an int: {v}"));
    let pairs = |v: &Value| -> Result<Vec<(i32, Value)>, String> {
        match v {
            Value::Object(m) => m.iter().map(|(k, v)| Ok((k.parse().map_err(|_| format!("bad key {k}"))?, v.clone()))).collect(),
            _ => Err(format!("not an object: {v}")),
        }
    };
    let set = |v: &Value| -> Result<Vec<i32>, String> { v.as_array().ok_or_else(|| format!("not an array: {v}"))?.iter().map(int).collect() };
    let mut p = Participation::new();
    for (k, v) in &map {
        match k.as_str() {
            "PlayerNames" => {
                for (id, name) in pairs(v)? {
                    p.player_names.insert(id, name.as_str().unwrap_or_default().to_string());
                }
            }
            "SeatedPlayers" => {
                for (id, seat) in pairs(v)? {
                    p.seated_players.insert(id, int(&seat)?);
                }
            }
            "Observers" => set(v)?.into_iter().for_each(|i| {
                p.observers.insert(i);
            }),
            "Hosts" => set(v)?.into_iter().for_each(|i| {
                p.hosts.insert(i);
            }),
            "AvailableSeats" => set(v)?.into_iter().for_each(|i| {
                p.available_seats.insert(i);
            }),
            "Kicked" => set(v)?.into_iter().for_each(|i| {
                p.kicked.insert(i);
            }),
            "BotsSpeed" => p.bots_speed = int(v)?,
            _ => {}
        }
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_keeps_order() {
        let text = r#"{"PlayerNames":{"7":"Alice","3":"Bob"},"SeatedPlayers":{"7":0,"3":-1},"Observers":[9],"Hosts":[7],"AvailableSeats":[2],"Kicked":[],"BotsSpeed":2}"#;
        let p = from_json(text).unwrap();
        assert_eq!(to_json(&p), text);
        let wire = to_wire(&p);
        assert_eq!(wire["PlayerNames"], json!([[7, "Alice"], [3, "Bob"]]));
        assert_eq!(wire["SeatedPlayers"], json!([[7, 0], [3, -1]]));
        assert_eq!(wire["BotsSpeed"], 2);
        assert_eq!(from_json("{}").unwrap().bots_speed, Participation::new().bots_speed);
        assert!(from_json("[]").is_err());
    }
}
