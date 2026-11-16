//! Structured records attached to log messages (`with_log`), one module per game phase.

pub mod aside;
pub use aside::*;
pub mod bidding;
pub use bidding::*;
pub mod blow;
pub use blow::*;
pub mod charity;
pub use charity::*;
pub mod collection;
pub use collection::*;
pub mod mentat;
pub use mentat::*;
pub mod revival;
pub use revival::*;
pub mod ship_move;
pub use ship_move::*;
pub mod storm;
pub use storm::*;

/// Declares [`LogRecord`], with one variant per record type.
macro_rules! log_records {
    ($($t:ident),* $(,)?) => {
        /// The record a log line's design draws from.
        #[derive(Clone, Debug, PartialEq)]
        pub enum LogRecord {
            $($t($t),)*
        }

        $(
            impl From<$t> for LogRecord {
                fn from(r: $t) -> Self {
                    LogRecord::$t(r)
                }
            }
        )*
    };
}

log_records! {
    AsideLog,
    BiddingDrawn, TreacheryReshuffled, BiddingSkipped, CardSold, CardWonShown, ExtraCardReceived, ExtraCardShown,
    BiddingPassedByAll,
    BlowDeckLog, BlowCardLog, WormAppearedLog, WormSentLog, WormDevouredForcesLog, WormSurvivedLog, WormDevouredSpiceLog,
    WormIgnoredLog, BlowReshuffleLog, NexusLog,
    CharityClaim,
    SpiceCollected,
    BribesCollected, WinReason, GameEnded, GameWinner,
    RevivalRow,
    ShipLogged, MoveLogged, MovePassed, BeneGesseritReaction, ShipMoveLoss, GuildDelayed,
    StormDials, StormPlaced, StormForecast, StormMoved, StormKilled, StormDestroyedSpice, FamilyAtomicsUsed,
    FamilyAtomicsKilled, BeneGesseritFlipped,
}
