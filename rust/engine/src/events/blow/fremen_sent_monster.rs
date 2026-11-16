use crate::log::blow::WormSentLog;
use crate::game::MonsterAppearance;
use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug)]
pub struct FremenSentMonster {
    pub territory: Option<Territory>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for FremenSentMonster {
    fn default() -> Self {
        FremenSentMonster {
            territory: Territory::from_id(0),
            base: Default::default(),
        }
    }
}

impl FremenSentMonster {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        if self.territory.is_none() {
            return Some(express!("No territory selected"));
        }
        if self.territory.expect("territory").is_protected_from_worm() {
            return Some(express!("Selected territory is protected"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game) -> impl Iterator<Item = Territory> {
        g.board.map.territories(false).filter(move |t| !t.is_protected_from_worm())
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        // A null Territory (only without validation) is stored in WormSentLog and the
        // MonsterAppearance; PerformMonster then throws on `m.Territory.Locations`.
        let msg = self.get_message(game);
        let territory = self.territory.required("territory")?;
        let msg = msg.with_log(WormSentLog::new(self.initiator().required("faction")?, territory));
        self.log_objects(game, vec![Part::from(msg)]);
        let monster = MonsterAppearance::new(territory, false);
        game.blow.monsters.push(monster.clone());
        game.perform_monster(&monster);
        let condition = game.turn.current_phase == Phase::FremenSendingMonsterA;
        game.enter_either(condition, Phase::BlowA, Phase::BlowB);
        game.draw_resource_card();
        game.let_factions_discard_surplus_cards();
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " send ", Concept::Monster, " to ", self.territory)
    }

}

impl GameEventMethods for FremenSentMonster {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::FremenSentMonster
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::FremenSentMonster(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        FremenSentMonster::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        FremenSentMonster::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        FremenSentMonster::get_message(self, game)
    }

}
