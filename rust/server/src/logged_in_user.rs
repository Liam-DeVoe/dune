//! Port of `Server/LoggedInUser.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use treachery_engine::date_time_offset::DateTimeOffset;
use treachery_engine::enumerations::UserStatus;

use crate::db::user::User;

/// A `User` object as C# shares it. `UsersById` and each `LoggedInUser.User` can hold the same
/// object or different ones, and the hub writes a cached object back to the database with
/// `db.Users.Update(user)` (GameHub_Authentication.cs:213), stale columns included. Keeping
/// that identity is part of equivalence; see the plan, "Hazards".
pub type UserRef = Rc<RefCell<User>>;

pub struct LoggedInUser {
    pub user: UserRef,
    /// `= loggedInDateTime ?? DateTimeOffset.Now`.
    pub logged_in_date_time: DateTimeOffset,
    /// Initializer `UserStatus.Online`.
    pub status: UserStatus,
    pub last_seen_date_time: DateTimeOffset,
}

impl LoggedInUser {
    /// `new LoggedInUser(user, loggedInDateTime)` (LoggedInUser.cs:12). `now` is the hub clock,
    /// used when `logged_in_date_time` is `None`.
    pub fn new(user: UserRef, logged_in_date_time: Option<DateTimeOffset>, now: DateTimeOffset) -> Self {
        LoggedInUser {
            user,
            logged_in_date_time: logged_in_date_time.unwrap_or(now),
            status: UserStatus::Online,
            last_seen_date_time: DateTimeOffset::default(),
        }
    }

    pub fn id(&self) -> i32 {
        self.user.borrow().id
    }

    pub fn username(&self) -> String {
        self.user.borrow().name.clone()
    }

    pub fn player_name(&self) -> String {
        self.user.borrow().player_name.clone()
    }

    pub fn email(&self) -> String {
        self.user.borrow().email.clone()
    }
}
