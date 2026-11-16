//! User ids, and the `[userId, seat]` pairs that seat users.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use treachery_engine::game::Seat;

/// A user: the id of their row in the server's `users` table. Written as the number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(pub i64);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A plain `number` in TypeScript. Ids stay far below 2^53, so they fit one.
impl ts_rs::TS for UserId {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name(_: &ts_rs::Config) -> String {
        "number".to_string()
    }

    fn inline(cfg: &ts_rs::Config) -> String {
        Self::name(cfg)
    }
}

/// `#[serde(with = "seated_users")]` writes each user's seat as a pair `[userId, seat]`, with -1
/// for a user who has no seat yet.
pub mod seated_users {
    use super::*;

    pub fn serialize<S: Serializer>(pairs: &[(UserId, Option<Seat>)], s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(pairs.iter().map(|(user, seat)| (user, Seat::number_or_minus_one(*seat))))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<(UserId, Option<Seat>)>, D::Error> {
        Vec::<(UserId, i64)>::deserialize(d)?
            .into_iter()
            .map(|(user, seat)| Ok((user, Seat::from_number_or_minus_one(seat).map_err(serde::de::Error::custom)?)))
            .collect()
    }
}
