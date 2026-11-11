//! Port of `Model/ServerInfo.cs`.

use treachery_engine::date_time_offset::DateTimeOffset;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ServerInfo {
    pub scheduled_maintenance: DateTimeOffset,
    pub admin_name: String,
}

super::to_json_members!(ServerInfo {
    "ScheduledMaintenance" => scheduled_maintenance,
    "AdminName" => admin_name,
});
