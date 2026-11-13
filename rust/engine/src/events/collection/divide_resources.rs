use crate::prelude::*;

/// `params object?[]` element list for `Log(...)`.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

#[derive(Clone, Debug)]
pub struct DivideResources {
    pub portion_to_first_player: i32,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for DivideResources {
    fn default() -> Self {
        DivideResources {
            portion_to_first_player: Default::default(),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

impl DivideResources {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.passed {
            return None;
        }

        let to_be_divided = Self::get_resources_to_be_divided(game);
        let Some(to_be_divided) = to_be_divided else { return Some(express!("No collections to be divided")) };
        if self.portion_to_first_player > Self::get_resources_to_be_divided(game).expect("resources to be divided").amount {
            return Some(express!("Too much assigned to ", to_be_divided.first_faction));
        }

        None
    }

    pub fn get_resources_to_be_divided(g: &Game) -> Option<ResourcesToBeDivided> {
        g.collected_resources_to_be_divided.first().cloned()
    }

    pub fn is_applicable(g: &Game, p: PlayerIndex) -> bool {
        g.current_phase == Phase::DividingCollectedResources
            && Self::get_resources_to_be_divided(g).expect("resources to be divided").first_faction == g.player(p).faction
    }

    pub fn gained_by_other_faction(tbd: Option<&ResourcesToBeDivided>, agreed: bool, portion_to_first_player: i32) -> i32 {
        // tbd.Amount is read before GainedByFirstFaction runs.
        let amount = tbd.expect("resources to be divided").amount;
        amount - Self::gained_by_first_faction(tbd, agreed, portion_to_first_player)
    }

    pub fn gained_by_first_faction(tbd: Option<&ResourcesToBeDivided>, agreed: bool, portion_to_first_player: i32) -> i32 {
        if agreed {
            portion_to_first_player
        } else {
            // (int)(0.5f * tbd.Amount): float multiply, then truncation toward zero.
            (0.5f32 * tbd.expect("resources to be divided").amount as f32) as i32
        }
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.current_division_proposal = Some(self.clone());

        if self.passed {
            game.divide_resources_from_collection(false);
            let any = !game.collected_resources_to_be_divided.is_empty();
            game.enter_boolean_phase_action(any, Phase::DividingCollectedResources, Game::end_collection_main_phase);
        } else {
            let to_be_divided = Self::get_resources_to_be_divided(game);
            let gained_by_other_faction = Self::gained_by_other_faction(to_be_divided.as_ref(), true, self.portion_to_first_player);
            let other = to_be_divided.required("to be divided")?.other_faction;
            self.log_objects(
                game,
                elems![self.initiator(), " propose that they take ", Payment::of(self.portion_to_first_player), " and ", other, " take ", Payment::of(gained_by_other_faction)],
            );
            game.enter(Phase::AcceptingResourceDivision, true);
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!("Collected resources were divided")
    }

}

impl GameEventMethods for DivideResources {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::DivideResources
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::DivideResources(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        DivideResources::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        DivideResources::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        DivideResources::get_message(self, game)
    }

}

impl PassableGameEventMethods for DivideResources {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}
