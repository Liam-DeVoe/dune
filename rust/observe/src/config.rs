//! Game configuration drawn from a game's seed, after the C# oracle's `GameConfig.Randomize`
//! (treachery-online/Oracle/GameConfig.cs), at `Game::LATEST_VERSION` only.

use serde_json::{json, Value};
use treachery_engine::events::EstablishPlayers;
use treachery_engine::game::Game;
use treachery_engine::prelude::*;

use crate::rng::Rng;

/// How bots and the host drive the game (trace-format.md, "Host styles").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostStyle {
    /// Every bot sees `GetApplicableEvents(p, true)`, every bot event is submitted as host.
    Tests,
    /// The server with bots acting as hosts: in-phase events as player, end-phase ones as host.
    ServerBotsHost,
    /// The server with a human host: bots never act as host, the recorder plays the host.
    ServerHumanHost,
}

pub struct Config {
    pub ruleset: Ruleset,
    pub rules: Vec<Rule>,
    pub factions: Vec<Faction>,
    pub players: i32,
    pub turns: i32,
    pub engine_seed: i32,
    pub style: HostStyle,
    /// `[userId, seat, name]` of seated humans.
    pub seated: Vec<(i32, i32, String)>,
}

fn toggle(rules: &mut Vec<Rule>, r: Rule) {
    if let Some(i) = rules.iter().position(|x| *x == r) {
        rules.remove(i);
    } else {
        rules.push(r);
    }
}

impl Config {
    pub fn randomize(rng: &mut Rng, fuzz: bool) -> Config {
        use RuleGroup as G;
        let group = Game::get_rule_group;
        let house: Vec<Rule> = Rule::ALL.iter().copied().filter(|r| matches!(group(*r), G::House | G::CoreBasicExceptions | G::CoreAdvancedExceptions)).collect();
        let expansion: Vec<Rule> = Rule::ALL
            .iter()
            .copied()
            .filter(|r| {
                matches!(
                    group(*r),
                    G::CoreAdvanced
                        | G::ExpansionIxAndBtBasic
                        | G::ExpansionIxAndBtAdvanced
                        | G::ExpansionChoamAndRicheseBasic
                        | G::ExpansionChoamAndRicheseAdvanced
                        | G::ExpansionEcazAndMoritaniBasic
                        | G::ExpansionEcazAndMoritaniAdvanced
                )
            })
            .collect();
        let other = [
            Rule::ExpansionTreacheryCardsExceptPBandSSandAmal,
            Rule::ExpansionTreacheryCardsPBandSs,
            Rule::ExpansionTreacheryCardsAmal,
            Rule::RicheseTreacheryCards,
            Rule::BeneGesseritFirstForceInAnyTerritory,
            Rule::BeneGesseritAccompaniesToShipmentLocation,
        ];
        let host_heavy = [Rule::CustomDecks, Rule::CustomInitialForcesAndResources, Rule::PlayersChooseFactions, Rule::HmSwithoutIxian];

        let definitions = Game::ruleset_definition();
        let mut rulesets: Vec<Ruleset> = definitions.iter().map(|(k, _)| *k).collect();
        rulesets.sort_by_key(|r| r.0);
        let ruleset = rulesets[rng.below(rulesets.len())];
        let mut rules: Vec<Rule> = definitions.iter().find(|(k, _)| **k == ruleset).map(|(_, v)| v.clone()).unwrap_or_default();

        for r in expansion.iter().chain(other.iter()) {
            if rng.chance(0.08) {
                toggle(&mut rules, *r);
            }
        }
        for r in &house {
            let p = if host_heavy.contains(r) { 0.04 } else { 0.15 };
            if rng.chance(p) {
                toggle(&mut rules, *r);
            }
        }
        // Fuzz games sometimes get a rule set drawn from every rule, ignoring rulesets.
        if fuzz && rng.chance(0.35) {
            let density = 0.15 + 0.5 * rng.f64();
            for r in Rule::ALL.iter().copied().filter(|r| *r != Rule::None && group(*r) != G::Bots && *r != Rule::FillWithBots) {
                if rng.chance(density) {
                    toggle(&mut rules, r);
                }
            }
        }
        if !rules.contains(&Rule::FillWithBots) {
            rules.push(Rule::FillWithBots);
        }
        if rng.chance(0.2) {
            rules.push(Rule::BotsCannotAlly);
        }
        rng.shuffle(&mut rules);

        let all = EstablishPlayers::available_factions();
        let players = if rng.chance(0.8) { rng.range(2, 9) } else { rng.range(2, all.len() as i32 + 1) };
        let mut factions = all.clone();
        rng.shuffle(&mut factions);
        let allowed = rng.range(players, all.len() as i32 + 1) as usize;
        factions.truncate(allowed);

        let mut turns = if rng.chance(0.05) { rng.range(1, 3) } else { rng.range(3, 11) };
        if fuzz && rng.chance(0.25) {
            turns = rng.range(10, 21);
        }
        let style = match rng.f64() {
            x if x < 0.5 => HostStyle::Tests,
            x if x < 0.75 => HostStyle::ServerBotsHost,
            _ => HostStyle::ServerHumanHost,
        };
        let mut seated = Vec::new();
        if rng.chance(0.3) {
            let count = rng.range(1, players.min(3) + 1);
            let mut seats: Vec<i32> = (0..players).collect();
            rng.shuffle(&mut seats);
            for i in 0..count {
                seated.push((100 + i, seats[i as usize], format!("Human{}", 100 + i)));
            }
        }
        Config { ruleset, rules, factions, players, turns, engine_seed: rng.seed(), style, seated }
    }

    /// The host's `EstablishPlayers`, in savegame JSON (without `Time`).
    pub fn establish_players(&self) -> Value {
        let rules: Vec<i32> = self.rules.iter().map(|r| r.0).collect();
        let factions: Vec<i32> = self.factions.iter().map(|f| f.0).collect();
        let names: Vec<String> = self.factions.iter().map(|f| f.to_string()).collect();
        json!({
            "$type": "EstablishPlayers",
            "Settings": {
                "NumberOfPlayers": self.players,
                "MaximumTurns": self.turns,
                "InitialRules": rules,
                "AllowedFactionsInPlay": factions,
                "AutoOpenEmptySeats": true,
                "AsyncPlay": false,
                "AsyncPlayMessageIntervalMinutes": 0
            },
            "Seed": self.engine_seed,
            "MaximumNumberOfPlayers": self.players,
            "MaximumTurns": self.turns,
            "ApplicableRules": rules,
            "_players": "",
            "_factionsInPlay": names.join(","),
            "Initiator": 0
        })
    }

    pub fn to_json(&self) -> Value {
        json!({
            "ruleset": self.ruleset.to_string(),
            "rules": self.rules.iter().map(|r| r.to_string()).collect::<Vec<_>>(),
            "factions": self.factions.iter().map(|f| f.to_string()).collect::<Vec<_>>(),
            "players": self.players,
            "turns": self.turns,
            "hostStyle": format!("{:?}", self.style),
        })
    }
}
