use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug)]
pub struct KarmaMonster {
    pub territory_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for KarmaMonster {
    fn default() -> Self {
        KarmaMonster {
            territory_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl KarmaMonster {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn territory(&self, game: &Game) -> Option<Territory> {
        game.board.map.territory_lookup().find(self.territory_id)
    }

    pub fn set_territory(&mut self, game: &Game, value: Option<Territory>) {
        self.territory_id = game.board.map.territory_lookup().get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let Some(territory) = self.territory(game) else {
            return Some(express!("No territory selected"));
        };
        if territory.is_protected_from_worm() {
            return Some(express!("Selected territory is protected"));
        }
        if !KarmaMonster::valid_targets(game).any(|t| t == territory) {
            return Some(express!("Invalid territory"));
        }
        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game) -> impl Iterator<Item = Territory> {
        g.board.map.territories(false).filter(move |t| !t.is_protected_from_worm())
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        // Arguments in C# order: Player, the karma card, then the effect (which reads Territory).
        let me = self.player();
        let card = Karma::valid_karma_cards(game, me).next();
        let effect = vec![Part::from("send "), Part::from(Concept::Monster), Part::from(" to "), Part::from(self.territory(game))];
        game.play_karma(me, card, effect);
        game.player_mut(me).special_karma_power_used = true;
        game.stone(Milestone::Karma);
        game.blow.number_of_monsters += 1;
        let territory = self.territory(game);
        game.let_monster_appear(territory, false);

        if game.turn.current_phase == Phase::BlowReport {
            game.enter_alliance_phase(false);
        }
        Ok(())
    }

    pub fn get_message(&self, game: &Game) -> Message {
        express!("Using ", TreacheryCardType::Karma, ", ", self.initiator(), " send ", Concept::Monster, " to ", self.territory(game))
    }

}

impl GameEventMethods for KarmaMonster {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::KarmaMonster
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::KarmaMonster(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        KarmaMonster::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        KarmaMonster::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        KarmaMonster::get_message(self, game)
    }

}
