use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug)]
pub struct KarmaMonster {
    pub territory: Option<Territory>,
    /// The fields of `Event` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for KarmaMonster {
    fn default() -> Self {
        KarmaMonster {
            territory: Territory::from_id(0),
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

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let Some(territory) = self.territory else {
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
        let effect = vec![Part::from("send "), Part::from(Concept::Monster), Part::from(" to "), Part::from(self.territory)];
        game.play_karma(me, card, effect);
        game.player_mut(me).special_karma_power_used = true;
        game.stone(Milestone::Karma);
        game.blow.number_of_monsters += 1;
        let territory = self.territory;
        game.let_monster_appear(territory, false)?;

        if game.turn.current_phase == Phase::BlowReport {
            game.enter_alliance_phase(false);
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!("Using ", TreacheryCardType::Karma, ", ", self.initiator(), " send ", Concept::Monster, " to ", self.territory)
    }

}

impl GameEventMethods for KarmaMonster {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> EventType {
        EventType::KarmaMonster
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::KarmaMonster(self)
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
