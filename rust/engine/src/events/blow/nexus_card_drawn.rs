use crate::prelude::*;

/// `params object?[]` element list for `Log(...)`.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

#[derive(Clone, Debug, Canon)]
pub struct NexusCardDrawn {
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for NexusCardDrawn {
    fn default() -> Self {
        NexusCardDrawn {
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl NexusCardDrawn {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !self.passed && !Self::may_draw(game, self.player()) {
            return Some(express!("You're not allowed to draw a Nexus Card"));
        }
        None
    }

    pub fn may_draw(g: &Game, p: PlayerIndex) -> bool {
        !g.factions_that_drew_nexus_card.contains(&g.player(p).faction) || g.player(p).faction == g.player(p).nexus
    }

    pub fn applicable(g: &Game, p: PlayerIndex) -> bool {
        g.factions_that_may_draw_nexus_card.contains(&g.player(p).faction)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        if !self.passed {
            self.deal_nexus_card(game);
            game.factions_that_drew_nexus_card.push(self.initiator());
        }

        if self.passed {
            let initiator = self.initiator();
            if let Some(i) = game.factions_that_may_draw_nexus_card.iter().position(|f| *f == initiator) {
                game.factions_that_may_draw_nexus_card.remove(i);
            }
        }

        if game.factions_that_may_draw_nexus_card.is_empty() {
            game.end_blow_phase();
        }
    }

    pub fn deal_nexus_card(&self, game: &mut Game) {
        let me = self.player();
        game.discard_nexus_card(me);

        if game.nexus_card_deck.as_ref().nre().is_empty() {
            let pile = game.nexus_discard_pile.clone();
            game.nexus_card_deck.as_mut().nre().items.extend(pile);
            game.nexus_discard_pile.clear();
            game.nexus_card_deck.as_mut().nre().shuffle();
            game.stone(Milestone::Shuffled);
            self.log_objects(game, elems!["The Nexus Card discard pile was shuffled into a new Nexus Card deck"]);
        }

        let faction = game.player(self.player()).faction;
        self.log_objects(game, elems![faction, " draw a Nexus Card"]);
        let card = game.nexus_card_deck.as_mut().nre().draw();
        game.player_mut(self.player()).nexus = card;
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), express_part_if!(self.passed; " don't"), " draw a Nexus card")
    }

}

impl GameEventMethods for NexusCardDrawn {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::NexusCardDrawn
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::NexusCardDrawn(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        NexusCardDrawn::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        NexusCardDrawn::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        NexusCardDrawn::get_message(self, game)
    }

}

impl PassableGameEventMethods for NexusCardDrawn {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
