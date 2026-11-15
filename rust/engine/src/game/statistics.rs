use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct GameStatistics {
    pub date: DateTimeOffset,
    pub time: f32,
    pub turn: i32,
    pub ruleset: String,
    pub players: Vec<GameStatisticsPlayerInfo>,
    pub winners: Vec<String>,
    pub method: String,
}

impl Default for GameStatistics {
    fn default() -> Self {
        GameStatistics {
            date: Default::default(),
            time: Default::default(),
            turn: Default::default(),
            ruleset: Default::default(),
            players: Default::default(),
            winners: Default::default(),
            method: Default::default(),
        }
    }
}

impl GameStatistics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn round_to_halves(value: f64) -> f32 {
        // Math.Round(x, MidpointRounding.AwayFromZero) is f64::round.
        ((value * 2.0).round() as f32) / 2.0
    }

    pub fn win_method_name(m: WinMethod) -> String {
        match m {
            WinMethod::Strongholds => "STRONGHOLDS",
            WinMethod::Prediction => "PREDICTION",
            WinMethod::Timeout => "TIMEOUT",
            WinMethod::Forfeit => "FORFEIT",
            WinMethod::FremenSpecial => "FREMEN_DEFAULT",
            WinMethod::GuildSpecial => "GUILD_DEFAULT",
            _ => "None",
        }
        .to_string()
    }

    pub fn to_string_faction(f: Faction) -> String {
        match f {
            Faction::Fremen => "FREMEN",
            Faction::Atreides => "ATREIDES",
            Faction::Harkonnen => "HARKONNEN",
            Faction::Emperor => "EMPEROR",
            Faction::Guild => "GUILD",
            Faction::BeneGesserit => "BG",
            Faction::Ixian => "IX",
            Faction::Tleilaxu => "TLEILAXU",
            Faction::Richese => "RICHESE",
            Faction::Choam => "CHOAM",
            _ => "None",
        }
        .to_string()
    }

    pub fn to_string_ruleset(rules: Ruleset) -> String {
        match rules {
            Ruleset::AdvancedGame | Ruleset::ExpansionAdvancedGame => "GF9 advanced",
            Ruleset::BasicGame | Ruleset::ExpansionBasicGame => "GF9 basic",
            _ => "None",
        }
        .to_string()
    }

}

#[derive(Clone, Debug)]
pub struct GameStatisticsPlayerInfo {
    pub faction: String,
    pub name: String,
    pub id: String,
}

impl Default for GameStatisticsPlayerInfo {
    fn default() -> Self {
        GameStatisticsPlayerInfo {
            faction: Default::default(),
            name: Default::default(),
            id: Default::default(),
        }
    }
}

impl GameStatisticsPlayerInfo {
    pub fn new() -> Self {
        Self::default()
    }

}
