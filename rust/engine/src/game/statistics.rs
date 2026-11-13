use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct GameStatistics {
    #[canon(name = "date")]
    pub date: DateTimeOffset,
    #[canon(name = "time")]
    pub time: f32,
    #[canon(name = "turn")]
    pub turn: i32,
    #[canon(name = "ruleset")]
    pub ruleset: String,
    #[canon(name = "players")]
    pub players: Vec<GameStatisticsPlayerInfo>,
    #[canon(name = "winners")]
    pub winners: Vec<String>,
    #[canon(name = "method")]
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

    pub fn get_statistics(game: &Game) -> GameStatistics {
        let result = GameStatistics {
            date: game.last_action(),
            method: GameStatistics::to_string(game.win_method),
            players: game
                .players
                .iter()
                .map(|p| GameStatisticsPlayerInfo { name: p.name(game), faction: GameStatistics::to_string_faction(p.faction), id: String::new() })
                .collect(),
            ruleset: GameStatistics::to_string_ruleset(game.ruleset),
            // DateTime - DateTime: the tick difference (offsets are already dropped).
            time: GameStatistics::round_to_halves(TimeSpan::from_ticks(game.last_action().ticks - game.first_action().ticks).total_hours()),
            turn: game.current_turn,
            winners: game.winners.iter().map(|p| GameStatistics::to_string_faction(game.player(*p).faction)).collect(),
        };

        result
    }

    pub fn round_to_halves(value: f64) -> f32 {
        // Math.Round(x, MidpointRounding.AwayFromZero) is f64::round.
        ((value * 2.0).round() as f32) / 2.0
    }

    pub fn to_string(m: WinMethod) -> String {
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

#[derive(Clone, Debug, Canon)]
pub struct GameStatisticsPlayerInfo {
    #[canon(name = "faction")]
    pub faction: String,
    #[canon(name = "name")]
    pub name: String,
    #[canon(name = "id")]
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
