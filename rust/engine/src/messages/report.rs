use crate::enumerations::{Faction, MainPhase};
use crate::messages::part::Part;
use crate::messages::message::Message;
use crate::map::territory::Territory;

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub about: Option<MainPhase>,
    /// `None` for the report before the game starts.
    /// The value of `Game.CurrentTurn` when this report was started (0 for the setup report).
    pub turn: crate::game::Turn,
    /// Only set on a report that covers a single battle (started by BattleInitiated).
    pub aggressor: Option<Faction>,
    pub defender: Option<Faction>,
    pub territory: Option<Territory>,
    /// Set once the battle outcome is resolved. `battle_winner` stays
    /// `None` when nobody won.
    pub battle_decided: bool,
    pub battle_winner: Option<Faction>,
    /// Only appended to and enumerated.
    pub messages: Vec<Message>,
}

impl Report {
    pub fn new(about: Option<MainPhase>, turn: crate::game::Turn) -> Self {
        Report {
            about,
            turn,
            aggressor: None,
            defender: None,
            territory: None,
            battle_decided: false,
            battle_winner: None,
            messages: Vec::new(),
        }
    }

    pub fn is_battle(&self) -> bool {
        self.aggressor != None
    }

    pub fn express(&mut self, list: Vec<Part>) {
        self.add(Message::express(list));
    }

    pub fn express_to(&mut self, to: Faction, list: Vec<Part>) {
        self.add(Message::express_to(Some(to), list));
    }

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
