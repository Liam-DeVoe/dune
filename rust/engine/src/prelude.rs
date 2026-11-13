//! Everything an engine module usually needs: `use crate::prelude::*;`.

pub use std::cell::{Cell, RefCell};
pub use std::rc::Rc;

pub use canon_derive::Canon;
pub use dotnet_compat::{NetDictionary, NetHashSet, NetRandom};
pub use indexmap::{IndexMap, IndexSet};

pub use crate::iter::{IterExt, MapExt};
pub use crate::compat::cs::{self, CsIterExt, CsNull, NullRef, OrNull, RcKey, Shared};
pub use crate::compat::date_time_offset::{DateTimeOffset, TimeSpan};
pub use crate::cards::deck::Deck;
pub use crate::enumerations::*;
pub use crate::messages::expression::{Expression, ExpressionElement};
pub use crate::identity::IFetcher;
pub use crate::heroes::hero::Hero;
pub use crate::identity::IIdentifiable;
pub use crate::heroes::leader_manager::LeaderManager;
pub use crate::random::{LoggedRandom, SharedRandom};
pub use crate::map::Map;
pub use crate::messages::message::{BattleFact, BattleFactKind, BattlePlans, Message, MessagePart, PlanSummary, Route};
pub use crate::messages::payment::Payment;
pub use crate::map::point::PointD;
pub use crate::messages::report::Report;
pub use crate::cards::treachery_card_manager::TreacheryCardManager;
pub use crate::{express, express_part, express_part_if, express_to, game_error};
pub use crate::error::{GameError, GameResult, Required};

pub use crate::map::attached_location::AttachedLocation;
pub use crate::map::discovery::DiscoveredLocation;
pub use crate::map::hidden_mobile_stronghold::HiddenMobileStronghold;
pub use crate::map::homeworld::Homeworld;
pub use crate::heroes::leader::Leader;
pub use crate::map::location::Location;
pub use crate::cards::resource_card::ResourceCard;
pub use crate::map::territory::Territory;
pub use crate::cards::treachery_card::TreacheryCard;

pub use crate::events::*;
pub use crate::game::*;
pub use crate::log::*;
pub use crate::map::discovery::Discovery;
