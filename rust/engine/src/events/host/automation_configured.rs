use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct AutomationConfigured {
    /// Interior mutability: written after the event is stored in game state.
    pub automation_rule_id: Cell<i32>,
    pub action: ItemAction,
    pub rule_type: AutomationRuleType,
    pub bidding_above_amount: i32,
    pub bidding_winning_faction: Faction,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for AutomationConfigured {
    fn default() -> Self {
        AutomationConfigured {
            automation_rule_id: Cell::new(-1),
            action: Default::default(),
            rule_type: Default::default(),
            bidding_above_amount: Default::default(),
            bidding_winning_faction: Default::default(),
            base: Default::default(),
        }
    }
}

impl AutomationConfigured {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        match self.action {
            ItemAction::Create => {
                let id = if game.automation_rules.is_empty() {
                    0
                } else {
                    // Non-empty, so Max cannot throw.
                    game.automation_rules.iter().map(|x| x.automation_rule_id.get()).max().nre().wrapping_add(1)
                };
                self.automation_rule_id.set(id);

                game.automation_rules.push(self.clone());
            }

            ItemAction::Delete => {
                // C# parity (hazards 1.4, 18.2): First throws for an unknown id, and ownership is not checked.
                let id = self.automation_rule_id.get();
                let rule = game.automation_rules.iter().cs_first_where(|x| x.automation_rule_id.get() == id).clone();
                if let Some(i) = game.automation_rules.iter().position(|x| Rc::ptr_eq(x, &rule)) {
                    game.automation_rules.remove(i);
                }
            }

            ItemAction::Update => {
                // C# parity (hazards 18.2): Update does nothing.
            }

            _ => {}
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        // C# parity (hazards 18.2): message copied from AllyPermission.
        express!(self.initiator(), " change ally permissions")
    }

    pub fn get_available_rule_types(g: &Game, p: PlayerIndex) -> Vec<AutomationRuleType> {
        let atreides_is_playing = g.is_playing(Faction::Atreides);
        get_values_except_default(AutomationRuleType::Unknown)
            .into_iter()
            .filter(|x| {
                (*x != AutomationRuleType::BiddingPassWhenAtreidesOrAtreidesAllyPassed || atreides_is_playing)
                    && (*x != AutomationRuleType::CharityAutoClaim || !g.player(p).is(Faction::BeneGesserit))
                    && (*x != AutomationRuleType::ShipmentGuildAutoDelay
                        || (g.player(p).is(Faction::Guild) && g.applicable(Rule::GuildDetermineShipment)))
                    && (*x != AutomationRuleType::BiddingPassWhenHighestBidByFaction
                        || !Self::get_valid_bidding_factions(g, p).is_empty())
            })
            .collect()
    }

    pub fn get_valid_bidding_factions(g: &Game, p: PlayerIndex) -> Vec<Faction> {
        let faction = g.player(p).faction;
        g.players.iter().filter(|x| x.faction != faction).map(|x| x.faction).collect()
    }

    pub fn get_description(&self, game: &Game) -> Option<Message> {
        match self.rule_type {
            AutomationRuleType::CharityAutoClaim => {
                return Some(express!("Auto claim charity when possible"));
            }

            AutomationRuleType::BiddingPassAboveAmount => {
                return Some(express!("Auto pass if bid is equal to or higher than ", Payment::of(self.bidding_above_amount)));
            }

            AutomationRuleType::BiddingPassWhenHighestBidByFaction => {
                return Some(express!("Auto pass if highest bid by ", self.bidding_winning_faction));
            }

            AutomationRuleType::BiddingPassWhenAtreidesOrAtreidesAllyPassed => {
                let atreides = game.get_player(Some(Faction::Atreides));
                let has_ally = atreides.map_or(false, |g| game.player(g).has_ally());
                let ally: Option<Faction> = atreides.map(|g| game.player(g).ally);
                return Some(express!(
                    "Auto pass if most recent bid by ",
                    Faction::Atreides,
                    express_part_if!(has_ally; ally),
                    " was passed"
                ));
            }

            AutomationRuleType::RevivalAutoClaimFreeRevival => {
                let me = self.player();
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

impl GameEventMethods for AutomationConfigured {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::AutomationConfigured
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::AutomationConfigured(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        AutomationConfigured::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        AutomationConfigured::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        AutomationConfigured::get_message(self, game)
    }

}
