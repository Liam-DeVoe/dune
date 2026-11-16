use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct AmbassadorPlaced {
    pub ambassador: Option<Ambassador>,
    pub stronghold: Option<Territory>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for AmbassadorPlaced {
    fn default() -> Self {
        AmbassadorPlaced {
            ambassador: Default::default(),
            stronghold: Territory::from_id(0),
            base: Default::default(),
        }
    }
}

impl AmbassadorPlaced {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let stronghold = self.stronghold;
        if !AmbassadorPlaced::valid_strongholds(game, self.player()).into_iter().any(|t| Some(t) == stronghold) {
            return Some(express!("Invalid stronghold"));
        }
        if !AmbassadorPlaced::valid_ambassadors(game, self.player()).iter().any(|a| Some(*a) == self.ambassador) {
            return Some(express!("Ambassador not available"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_strongholds(g: &Game, _p: PlayerId) -> impl Iterator<Item = Territory> {
        g.board.map
            .territories(false)
            .filter(move |t| {
                t.is_visible(&g.board.map) && t.is_stronghold() && !g.is_in_storm_territory(*t) && g.ambassador_in(*t) == None
            })
    }

    pub fn valid_ambassadors(game: &Game, p: PlayerId) -> Vec<Ambassador> {
        game.player(p).ambassadors.clone()
    }

    pub fn is_applicable(g: &Game, p: PlayerId) -> bool {
        i32::from(g.player(p).resources) > g.revival.ambassadors_placed_this_turn
            && g.turn.current_phase == Phase::ResurrectionReport
            && !g.prevented(FactionAdvantage::EcazAmbassadors)
            && !AmbassadorPlaced::valid_ambassadors(g, p).is_empty()
            && !AmbassadorPlaced::valid_strongholds(g, p).next().is_none()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let Some(stronghold) = self.stronghold else {
            game_error!("Stronghold is null");
        };

        game.revival.ambassadors_placed_this_turn += 1;
        let me = self.player();
        let placed = game.revival.ambassadors_placed_this_turn;
        let p = game.player_mut(me);
        crate::game::amounts::spend(&mut p.resources, placed, "spice")?;
        self.log_objects(
            game,
            vec![
                self.initiator().into(),
                " station the ".into(),
                self.ambassador.into(),
                " ambassador in ".into(),
                stronghold.into(),
                " for ".into(),
                Payment::of(placed).into(),
            ],
        );
        if game.board.ambassadors_on_planet.contains_key(&stronghold) {
            game_error!("There already is an ambassador in Treachery.Shared.Territory");
        }
        let ambassador = self.ambassador.required("ambassador")?;
        game.board.ambassadors_on_planet.insert_new(stronghold, ambassador);
        // List.Remove: the first equal element
        let ambassadors = &mut game.player_mut(me).ambassadors;
        if let Some(pos) = ambassadors.iter().position(|a| *a == ambassador) {
            ambassadors.remove(pos);
        }
        game.stone(Milestone::AmbassadorPlaced);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " station the ", self.ambassador, " ambassador in ", self.stronghold)
    }
}

impl GameEventMethods for AmbassadorPlaced {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::AmbassadorPlaced
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::AmbassadorPlaced(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        AmbassadorPlaced::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        AmbassadorPlaced::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        AmbassadorPlaced::get_message(self, game)
    }

}
