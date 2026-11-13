use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Diplomacy {
    pub card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for Diplomacy {
    fn default() -> Self {
        Diplomacy {
            card_id: Default::default(),
            base: Default::default(),
        }
    }
}

macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

impl Diplomacy {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::get(self.card_id)
    }

    pub fn set_card(&mut self, value: Option<TreacheryCard>) {
        self.card_id = TreacheryCardManager::get_id(value);
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn valid_cards(g: &Game, p: PlayerIndex) -> Vec<TreacheryCard> {
        let mut result = Vec::new();
        let plan = g.current_battle.as_ref().expect("current battle").plan_of(g, Some(p)).expect("plan of");
        if let Some(weapon) = plan.weapon() {
            if weapon.is_useless() && plan.defense().is_none() {
                result.push(weapon);
            }
        }
        if let Some(defense) = plan.defense() {
            if defense.is_useless() {
                result.push(defense);
            }
        }
        result
    }

    pub fn can_be_played(g: &Game, p: PlayerIndex) -> bool {
        if g.skilled_as_player_leader_skill(p, LeaderSkill::Diplomat) && g.current_diplomacy.is_none() {
            let plan = g.current_battle.as_ref().expect("current battle").plan_of(g, Some(p));
            let plan_of_opponent = g.current_battle.as_ref().expect("current battle").plan_of_opponent(g, Some(p));
            let Some(plan) = plan else { return false };
            // plan.Defense is not { IsDefense: true }
            if plan.defense().is_some_and(|d| d.is_defense()) {
                return false;
            }
            // planOfOpponent.Defense is { IsDefense: true } (planOfOpponent! may be null)
            if !plan_of_opponent.expect("plan of opponent").defense().is_some_and(|d| d.is_defense()) {
                return false;
            }
            return !Self::valid_cards(g, p).is_empty();
        }

        false
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        // Game.CurrentBattle?.PlanOfOpponent(Player)?.Defense
        let opponent_defense = game
            .current_battle
            .as_ref()
            .and_then(|b| b.plan_of_opponent(game, self.player_opt()))
            .and_then(|plan| plan.defense());
        game.log(elems![self.initiator(), " use Diplomacy to turn ", self.card(), " into a ", opponent_defense]);
        game.current_diplomacy = Some(self.clone());
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " use Diplomacy to turn ", self.card(), " into a copy of the opponent's defense")
    }
}

impl GameEventMethods for Diplomacy {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::Diplomacy
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Diplomacy(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Diplomacy::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        Diplomacy::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Diplomacy::get_message(self, game)
    }

}
