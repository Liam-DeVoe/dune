//! Port of `Model/AvailableSeatInfo.cs`.

use treachery_engine::enumerations::Faction;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AvailableSeatInfo {
    pub seat: i32,
    pub faction: Faction,
    pub is_bot: bool,
}

super::to_json_members!(AvailableSeatInfo {
    "Seat" => seat,
    "Faction" => faction,
    "IsBot" => is_bot,
});
