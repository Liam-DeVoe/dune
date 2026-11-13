use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct MulliganPerformed {
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for MulliganPerformed {
    fn default() -> Self {
        MulliganPerformed {
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl MulliganPerformed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !self.passed && !Self::may_mulligan(game, self.player()) {
            return Some(express!("You can't take a mulligan"));
        }

        None
    }

    pub fn may_mulligan(game: &Game, p: PlayerIndex) -> bool {
        let faction = game.player(p).faction;
        game.player(p).traitors.iter().filter(|l| l.is(faction)).count() >= 2
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        if !self.passed {
            let me = self.player();
            // TraitorDeck is dereferenced before Player.Traitors is read.
            game.traitor_deck.as_ref().required("traitor deck")?;
            let traitors = game.player(me).traitors.clone();
            game.traitor_deck.as_mut().required("traitor deck")?.items.extend(traitors);
            game.player_mut(me).traitors.clear();
            game.traitor_deck.as_mut().required("traitor deck")?.shuffle();
            game.stone(Milestone::Shuffled);
            game.deal_harkonnen_traitor_cards();
            game.enter(Phase::HarkonnenMulligan, true);
        } else {
            self.deal_non_harkonnen_traitor_cards(game);
            game.enter_select_traitors();
        }

        self.log(game);
        Ok(())
    }

    pub fn deal_non_harkonnen_traitor_cards(&self, game: &mut Game) {
        for _i in 1..=4 {
            // foreach over a lazy Where: the predicate is checked per player, before its body.
            for p in 0..game.players.len() {
                let f = game.players[p].faction;
                if f != Faction::Harkonnen && f != Faction::Tleilaxu {
                    let traitor = game.traitor_deck.as_mut().expect("traitor deck").draw();
                    game.players[p].traitors.push(traitor);
                }
            }
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), if self.passed { " pass" } else { " take" }, " a mulligan")
    }

}

impl GameEventMethods for MulliganPerformed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::MulliganPerformed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::MulliganPerformed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        MulliganPerformed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        MulliganPerformed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        MulliganPerformed::get_message(self, game)
    }

}

impl PassableGameEventMethods for MulliganPerformed {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
