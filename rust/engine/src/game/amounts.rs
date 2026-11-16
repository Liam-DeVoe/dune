//! Amounts of spice and forces, which the game holds as `u16`: a player's spice
//! (`Player.resources` and the five other spice fields), spice on the board, forces in a
//! battalion and forces in the tanks. The rules compute with `i32` (event fields, costs,
//! bonuses), so a change to any of them goes through these helpers. A result below zero, or a negative gain, means validation
//! let through something it shouldn't have, so the event fails with a Game Error. Where the
//! caller can't return one, it panics with `expect`. Where a player could cause it, validation
//! refuses the event instead (rewrite-plan.md, section 4, risk 5). Where the rules say to take
//! "as much as possible", the caller takes `min(amount, have)` first, as the host taking spice
//! into the bank does (`Donated`).

use crate::error::{GameError, GameResult};

/// `have - amount`. A Game Error if `have` holds less than `amount`, or if `amount` is negative.
pub fn spend(have: &mut u16, amount: i32, what: &str) -> GameResult {
    match u16::try_from(amount).ok().and_then(|a| have.checked_sub(a)) {
        Some(left) => {
            *have = left;
            Ok(())
        }
        None => Err(GameError::new(format!("Can't take {amount} {what} from {have}"))),
    }
}

/// `have + amount`. A Game Error if `amount` is negative or the result doesn't fit.
pub fn gain(have: &mut u16, amount: i32, what: &str) -> GameResult {
    match u16::try_from(amount).ok().and_then(|a| have.checked_add(a)) {
        Some(sum) => {
            *have = sum;
            Ok(())
        }
        None => Err(GameError::new(format!("Can't add {amount} {what} to {have}"))),
    }
}


/// `have + delta`, for a change that may go either way: [`gain`] or [`spend`].
pub fn change(have: &mut u16, delta: i32, what: &str) -> GameResult {
    if delta >= 0 { gain(have, delta, what) } else { spend(have, delta.checked_neg().unwrap_or(i32::MAX), what) }
}

/// An amount a bot or a form computed with `i32`, as an event field: below zero is 0, and above
/// 65535 is 65535. Validation then decides whether the event is acceptable.
pub fn clamp_u16(n: i32) -> u16 {
    u16::try_from(n.max(0)).unwrap_or(u16::MAX)
}
