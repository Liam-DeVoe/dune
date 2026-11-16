//! Everything an engine module usually needs: `use crate::prelude::*;`.

pub use std::cell::{Cell, RefCell};
pub use std::rc::Rc;

pub use std::collections::BTreeMap;
pub use indexmap::{IndexMap, IndexSet};

pub use crate::iter::{ContainsOpt, IterExt, MapExt, MapOpt};
pub use crate::shared::Shared;
pub use crate::compat::date_time_offset::{DateTimeOffset, TimeSpan};
pub use crate::cards::deck::Deck;
pub use crate::enumerations::*;
pub use crate::enum_type::{name_or_absent, value_or_absent};
pub use crate::identity::Fetcher;
pub use crate::heroes::hero::Hero;
pub use crate::identity::Identifiable;
pub use crate::heroes::leader_manager::LeaderManager;
pub use crate::random::{RandomPick, SharedRandom};
pub use crate::map::Map;
pub use crate::messages::message::{Audience, BattleFact, BattleFactKind, BattlePlans, LogDetail, Message, MessagePart, PlanSummary, Rejection, Route};
pub use crate::messages::payment::Payment;
pub use crate::messages::report::Report;
pub use crate::cards::treachery_card_manager::TreacheryCardManager;
pub use crate::{express, express_part, express_part_if, express_to, game_error, parts};
pub use crate::error::{GameError, GameResult, Required};

pub use crate::map::attached_location::AttachedLocation;
pub use crate::map::discovery::DiscoveredLocation;
pub use crate::map::hidden_mobile_stronghold::HiddenMobileStronghold;
pub use crate::map::homeworld::Homeworld;
pub use crate::heroes::leader::Leader;
pub use crate::map::location::Location;
pub use crate::cards::resource_card::ResourceCard;
pub use crate::map::territory::Territory;
pub use crate::map::sector::Sector;
pub use crate::game::half_points::{half_rounded_toward_zero, half_rounded_up, HalfPoints};
pub use crate::cards::treachery_card::TreacheryCard;

pub use crate::events::*;
pub use crate::game::*;
pub use crate::log::*;
pub use crate::map::discovery::Discovery;
