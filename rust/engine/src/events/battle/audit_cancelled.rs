use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct AuditCancelled {
    pub cancelled: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for AuditCancelled {
    fn default() -> Self {
        AuditCancelled {
            cancelled: Default::default(),
            base: Default::default(),
        }
    }
}

macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

impl AuditCancelled {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.cancelled && game.player(self.player()).resources < Self::cost_game(game) {
            return Some(express!("You can't pay enough to prevent the audit"));
        }
        None
    }

    pub fn cost(&self, game: &Game) -> i32 {
        Self::cost_game(game)
    }

    pub fn cost_game(g: &Game) -> i32 {
        Self::get_number_of_cards_that_may_be_audited(g)
    }

    pub fn get_number_of_cards_that_may_be_audited(g: &Game) -> i32 {
        std::cmp::min(if g.auditor_survived_battle { 2 } else { 1 }, Self::get_cards_that_may_be_audited(g).len() as i32)
    }

    pub fn get_cards_that_may_be_audited(g: &Game) -> Vec<TreacheryCard> {
        let Some(auditee) = g.auditee() else { return Vec::new() };
        let recent_battle_plan = g.current_battle.as_ref().nre().plan_of(g, Some(auditee));
        if let Some(plan) = recent_battle_plan {
            let (weapon, defense, hero) = (plan.weapon(), plan.defense(), plan.hero());
            return g
                .player(auditee)
                .treachery_cards
                .iter()
                .copied()
                // c != Hero compares a card with an IHero by reference.
                .filter(|c| Some(*c) != weapon && Some(*c) != defense && Some(Hero::TreacheryCard(*c)) != hero)
                .collect();
        }
        Vec::new()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        if self.cancelled {
            let cost = Payment::of(Self::cost_game(game));
            let n = Self::get_number_of_cards_that_may_be_audited(game);
            game.log(elems![self.initiator(), " pay ", Faction::Choam, cost, " to cancel an audit of ", n, " cards"]);
        } else {
            let n = Self::get_number_of_cards_that_may_be_audited(game);
            game.log(elems![self.initiator(), " don't cancel an audit targeting ", n, " of their cards"]);
        }

        if self.cancelled {
            let me = self.player();
            let cost = self.cost(game);
            game.player_mut(me).resources = game.player(me).resources.wrapping_sub(cost);
            let choam = self.get_player(game, Some(Faction::Choam));
            if let Some(choam) = choam {
                let cost = self.cost(game);
                game.player_mut(choam).resources = game.player(choam).resources.wrapping_add(cost);
            }
        }

        if !self.cancelled {
            game.enter(Phase::Auditing, true);
            // C# passes the live Game.AuditedCards list, which the next audit clears and
            // refills (hazards 4.5), so the message keeps it as a live list.
            let audited = ExpressionElement::live_game_audited_cards(game);
            game.log_to(self.initiator(), elems![Faction::Choam, " see: ", audited]);
        } else {
            let winner_is_none = game.battle_winner == Some(Faction::None);
            let harkonnen_must_decide = game.harkonnen_must_decide_to_capture();
            game.enter_boolean_action_boolean_phase_phase(
                winner_is_none,
                Game::finish_battle,
                harkonnen_must_decide,
                Phase::CaptureDecision,
                Phase::BattleConclusion,
            );
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.cancelled {
            return express!(self.initiator(), " pay to cancel an audit");
        }
        express!(self.initiator(), " don't pay to cancel an audit")
    }
}

impl GameEventMethods for AuditCancelled {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::AuditCancelled
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::AuditCancelled(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        AuditCancelled::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        AuditCancelled::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        AuditCancelled::get_message(self, game)
    }

}
