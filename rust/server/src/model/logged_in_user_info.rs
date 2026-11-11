//! Port of `Model/LoggedInUserInfo.cs`.

use treachery_engine::date_time_offset::DateTimeOffset;
use treachery_engine::enumerations::UserStatus;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoggedInUserInfo {
    pub id: i32,
    /// C# `string Name` without an initializer: null unless set.
    pub name: Option<String>,
    pub status: UserStatus,
    pub last_seen: Option<DateTimeOffset>,
}

super::to_json_members!(LoggedInUserInfo {
    "Id" => id,
    "Name" => name,
    "Status" => status,
    "LastSeen" => last_seen,
});
