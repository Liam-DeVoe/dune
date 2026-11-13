use crate::enumerations::{Faction, MainPhase};
use crate::messages::expression::ExpressionElement;
use crate::messages::message::Message;
use crate::map::territory::Territory;

#[derive(Clone, Debug, PartialEq, canon_derive::Canon)]
pub struct Report {
    pub about: MainPhase,
    /// The value of `Game.CurrentTurn` when this report was started (0 for the setup report).
    pub turn: i32,
    /// Only set on a report that covers a single battle (started by BattleInitiated).
    pub aggressor: Faction,
    pub defender: Faction,
    pub territory: Option<Territory>,
    /// Set once the battle outcome is resolved. `battle_winner` stays
    /// `Faction::None` when nobody won.
    pub battle_decided: bool,
    pub battle_winner: Faction,
    /// Only appended to and enumerated.
    pub messages: Vec<Message>,
}

impl Report {
    pub fn new(about: MainPhase, turn: i32) -> Self {
        Report {
            about,
            turn,
            aggressor: Faction::None,
            defender: Faction::None,
            territory: None,
            battle_decided: false,
            battle_winner: Faction::None,
            messages: Vec::new(),
        }
    }

    pub fn is_battle(&self) -> bool {
        self.aggressor != Faction::None
    }

    /// `Express(params object?[] list)`
    pub fn express(&mut self, list: Vec<ExpressionElement>) {
        self.add(Message::express(list));
    }

    /// `ExpressTo(to, params object?[] list)`
    pub fn express_to(&mut self, to: Faction, list: Vec<ExpressionElement>) {
        self.add(Message::express_to(to, list));
    }

    /// `Express(Message m)`
    pub fn express_message(&mut self, m: Message) {
        self.add(m);
    }

    /// `Express(GameEvent e) => Messages.AddLast(e.GetMessage())`. Takes the
    /// message, because `GetMessage` needs the game, which owns this report:
    /// `let m = e.get_message(game); game.current_report_mut().express_message(m)`.
    pub fn express_event(&mut self, message_of_event: Message) {
        self.add(message_of_event);
    }

    fn add(&mut self, m: Message) {
        self.messages.push(m);
    }
}
