use crate::prelude::*;

/// The game log: one report per phase, plus the report of messages logged before the game started.
pub struct Log {
    pub report_list: Vec<Report>,
    pub pre_game_report: Report,
}

impl Log {
    pub fn new() -> Self {
        Log {
            report_list: Vec::new(),
            pre_game_report: Report::new(None, 0),
        }
    }
}

impl Game {
    pub fn reports(&self) -> &[Report] {
        &self.log.report_list
    }

    /// The live report; use [`Game::current_report_mut`] to change it.
    pub fn current_report(&self) -> &Report {
        if self.log.report_list.len() > 0 {
            &self.log.report_list[self.log.report_list.len() - 1]
        } else {
            &self.log.pre_game_report
        }
    }

    /// `CurrentReport`, for mutation.
    pub fn current_report_mut(&mut self) -> &mut Report {
        if self.log.report_list.len() > 0 {
            let last = self.log.report_list.len() - 1;
            &mut self.log.report_list[last]
        } else {
            &mut self.log.pre_game_report
        }
    }

    /// Returns the live report, not a copy.
    pub fn start_report(&mut self, about: MainPhase) -> &mut Report {
        // Avoid leaving an empty duplicate behind if a report for the same phase is started twice in a row
        let current_turn = self.turn.current_turn;
        let has_reports = self.log.report_list.len() > 0;
        let current = self.current_report();
        if has_reports && current.about == Some(about) && current.turn == current_turn && !current.is_battle() && current.messages.len() == 0 {
            return self.current_report_mut();
        }

        let result = Report::new(Some(about), self.turn.current_turn);
        self.log.report_list.push(result);
        let last = self.log.report_list.len() - 1;
        &mut self.log.report_list[last]
    }

    pub fn log(&mut self, expression: Vec<ExpressionElement>) {
        // A message logged on its own keeps what it carries, such as a route.
        if expression.len() == 1 {
            if let ExpressionElement::Message(_) = &expression[0] {
                let Some(ExpressionElement::Message(message)) = expression.into_iter().next() else { unreachable!() };
                self.current_report_mut().express_message(*message);
                return;
            }
        }
        self.current_report_mut().express(expression);
    }

    pub fn log_if(&mut self, condition: bool, expression: Vec<ExpressionElement>) {
        if condition {
            self.current_report_mut().express(expression);
        }
    }

    pub fn log_to(&mut self, faction: Faction, expression: Vec<ExpressionElement>) {
        self.current_report_mut().express_to(faction, expression);
    }
}
