//! .NET behavior the engine still needs: `DateTimeOffset`/`TimeSpan`, whose text format is part
//! of the event JSON, and the random selection helpers, which draw from .NET's `Random` the way
//! C#'s did. The random helpers go when the random number generator is replaced (native-plan.md
//! step 4).

pub mod date_time_offset;
pub mod random_ext;
