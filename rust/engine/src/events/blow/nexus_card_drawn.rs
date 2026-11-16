use crate::prelude::*;
use crate::messages::part::Part;

/// `params object?[]` element list for `Log(...)`.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(Part::from($e)),*] };
}

#[derive(Clone, Debug)]
pub struct NexusCardDrawn {
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `Event` (Initiator, Time, Game, Player).
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
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !self.passed && !Self::may_draw(game, self.player()) {
            return Some(express!("You're not allowed to draw a Nexus Card"));
        }
        None
    }

    pub fn may_draw(g: &Game, p: PlayerId) -> bool {
        !g.blow.factions_that_drew_nexus_card.contains_opt(g.player(p).faction) || g.player(p).faction == g.player(p).nexus
    }

    pub fn applicable(g: &Game, p: PlayerId) -> bool {
        g.blow.factions_that_may_draw_nexus_card.contains_opt(g.player(p).faction)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        if !self.passed {
            self.deal_nexus_card(game);
            game.blow.factions_that_drew_nexus_card.push(self.initiator().required("faction")?);
        }

        if self.passed {
            let initiator = self.initiator();
            if let Some(i) = game.blow.factions_that_may_draw_nexus_card.iter().position(|f| Some(*f) == initiator) {
                game.blow.factions_that_may_draw_nexus_card.remove(i);
            }
        }

        if game.blow.factions_that_may_draw_nexus_card.is_empty() {
            game.end_blow_phase();
        }
        Ok(())
    }

    pub fn deal_nexus_card(&self, game: &mut Game) {
        let me = self.player();
        game.discard_nexus_card(me);

        if game.decks.nexus_card_deck.as_ref().expect("nexus card deck").is_empty() {
            let pile = game.decks.nexus_discard_pile.clone();
            game.decks.nexus_card_deck.as_mut().expect("nexus card deck").items.extend(pile);
            game.decks.nexus_discard_pile.clear();
            game.decks.nexus_card_deck.as_mut().expect("nexus card deck").shuffle();
            game.stone(Milestone::Shuffled);
            self.log_objects(game, elems!["The Nexus Card discard pile was shuffled into a new Nexus Card deck"]);
        }

        let faction = game.player(self.player()).faction;
        self.log_objects(game, elems![faction, " draw a Nexus Card"]);
        let card = game.decks.nexus_card_deck.as_mut().expect("nexus card deck").draw();
        game.player_mut(self.player()).nexus = Some(card);
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), express_part_if!(self.passed; " don't"), " draw a Nexus card")
    }

}

impl GameEventMethods for NexusCardDrawn {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> EventType {
        EventType::NexusCardDrawn
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::NexusCardDrawn(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        NexusCardDrawn::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
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
