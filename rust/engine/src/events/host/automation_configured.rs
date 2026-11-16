use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct AutomationConfigured {
    /// The rule to delete. A created rule gets its id in [`AutomationRuleState`].
    pub automation_rule_id: i32,
    pub action: ItemAction,
    pub rule_type: AutomationRuleType,
    pub bidding_above_amount: u16,
    pub bidding_winning_faction: Option<Faction>,
}

impl Default for AutomationConfigured {
    fn default() -> Self {
        AutomationConfigured {
            automation_rule_id: -1,
            action: Default::default(),
            rule_type: Default::default(),
            bidding_above_amount: Default::default(),
            bidding_winning_faction: Default::default(),
        }
    }
}

impl AutomationConfigured {
    pub fn get_available_rule_types(g: &Game, p: PlayerId) -> Vec<AutomationRuleType> {
        let atreides_is_playing = g.is_playing(Faction::Atreides);
        AutomationRuleType::ALL
            .iter()
            .copied()
            .filter(|x| {
                *x != AutomationRuleType::Unknown
                    && (*x != AutomationRuleType::BiddingPassWhenAtreidesOrAtreidesAllyPassed || atreides_is_playing)
                    && (*x != AutomationRuleType::CharityAutoClaim || !g.player(p).is(Faction::BeneGesserit))
                    && (*x != AutomationRuleType::ShipmentGuildAutoDelay
                        || (g.player(p).is(Faction::Guild) && g.applicable(Rule::GuildDetermineShipment)))
                    && (*x != AutomationRuleType::BiddingPassWhenHighestBidByFaction
                        || !Self::get_valid_bidding_factions(g, p).is_empty())
            })
            .collect()
    }

    pub fn get_valid_bidding_factions(g: &Game, p: PlayerId) -> Vec<Faction> {
        let faction = g.player(p).faction;
        g.players.iter().filter(|x| x.faction != faction).filter_map(|x| x.faction).collect()
    }

}

impl AutomationConfigured {
    /// The rule this event creates, with id `id`, for the submitter `by`.
    pub fn rule(&self, id: i32, by: By) -> AutomationRuleState {
        AutomationRuleState {
            id,
            initiator: by.faction,
            player: by.player,
            rule_type: self.rule_type,
            bidding_above_amount: self.bidding_above_amount,
            bidding_winning_faction: self.bidding_winning_faction,
        }
    }
}

/// An automation rule a player created (`AutomationConfigured`), with the id the game gave it.
#[derive(Clone, Debug, PartialEq)]
pub struct AutomationRuleState {
    pub id: i32,
    pub initiator: Option<Faction>,
    /// The submitter's player when the rule was created.
    pub player: Option<PlayerId>,
    pub rule_type: AutomationRuleType,
    pub bidding_above_amount: u16,
    pub bidding_winning_faction: Option<Faction>,
}

impl AutomationRuleState {
    pub fn get_description(&self, game: &Game) -> Option<Message> {
        match self.rule_type {
            AutomationRuleType::CharityAutoClaim => {
                return Some(express!("Auto claim charity when possible"));
            }

            AutomationRuleType::BiddingPassAboveAmount => {
                return Some(express!("Auto pass if bid is equal to or higher than ", Payment::of(i32::from(self.bidding_above_amount))));
            }

            AutomationRuleType::BiddingPassWhenHighestBidByFaction => {
                return Some(express!("Auto pass if highest bid by ", self.bidding_winning_faction));
            }

            AutomationRuleType::BiddingPassWhenAtreidesOrAtreidesAllyPassed => {
                let atreides = game.get_player(Some(Faction::Atreides));
                let has_ally = atreides.map_or(false, |g| game.player(g).has_ally());
                let ally: Option<Faction> = atreides.map(|g| game.player(g).ally).flatten();
                return Some(express!(
                    "Auto pass if most recent bid by ",
                    Faction::Atreides,
                    express_part_if!(has_ally; ally),
                    " was passed"
                ));
            }

            AutomationRuleType::RevivalAutoClaimFreeRevival => {
                let me = self.player.expect("the rule has a player");
                if game.player(me).has_special_forces(game) {
                    return Some(express!("Auto claim free revival, prioritizing ", game.player(me).special_force()));
                }

                return Some(express!("Auto claim free revival"));
            }

            AutomationRuleType::ShipmentGuildAutoDelay => {
                return Some(express!("Auto delay shipment until last"));
            }

            _ => {}
        }

        None
    }
}

impl EventRules for AutomationConfigured {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        match self.action {
            ItemAction::Create => {
                let id = if game.rules.automation_rules.is_empty() {
                    0
                } else {
                    // Non-empty, so Max cannot throw.
                    game.rules.automation_rules.iter().map(|x| x.id).max().required("max")? + 1
                };
                game.rules.automation_rules.push(self.rule(id, by));
            }

            ItemAction::Delete => {
                // C# parity: an unknown id fails the event (C#'s First throws),
                // and ownership is not checked.
                let id = self.automation_rule_id;
                let Some(i) = game.rules.automation_rules.iter().position(|x| x.id == id) else {
                    game_error!("There is no automation rule {id}");
                };
                game.rules.automation_rules.remove(i);
            }

            ItemAction::Update => {
                // C# parity: Update does nothing.
            }

        }
        Ok(())
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        match self.action {
            ItemAction::Create => {
                // The description reads the initiator's player, which a seatless host doesn't have.
                let description = game.get_player(by.faction).and_then(|_| self.rule(self.automation_rule_id, by).get_description(game));
                match description {
                    Some(d) => express!(by.faction, " add an automation rule: ", d),
                    None => express!(by.faction, " add an automation rule"),
                }
            }
            ItemAction::Delete => express!(by.faction, " remove an automation rule"),
            _ => express!(by.faction, " change an automation rule"),
        }
    }
}
