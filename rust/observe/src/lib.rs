//! The observable-behavior harness. A frozen reference build records bot and fuzz games as traces
//! of calls and what players could observe of them; the working tree replays the calls and must
//! observe the same. See rust/docs/native-harness.md.

pub mod config;
pub mod memory;
pub mod rng;
pub mod session;
pub mod trace;
pub mod fuzz;
pub mod record;
pub mod watch;
pub mod allow;
pub mod jsondiff;
pub mod replay;
pub mod heavy;
pub mod live;
pub mod report;
pub mod reach;
