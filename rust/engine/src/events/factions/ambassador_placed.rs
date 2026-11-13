use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct AmbassadorPlaced {
    pub ambassador: Ambassador,
    #[canon(name = "_strongholdId")]
    pub stronghold_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for AmbassadorPlaced {
    fn default() -> Self {
        AmbassadorPlaced {
            ambassador: Default::default(),
            stronghold_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl AmbassadorPlaced {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn stronghold(&self, game: &Game) -> Option<Territory> {
        game.map.territory_lookup().find(self.stronghold_id)
    }

    pub fn set_stronghold(&mut self, game: &Game, value: Option<Territory>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.stronghold_id = game.map.territory_lookup().get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        let stronghold = self.stronghold(game);
        if !AmbassadorPlaced::valid_strongholds(game, self.player()).into_iter().any(|t| Some(t) == stronghold) {
            return Some(express!("Invalid stronghold"));
        }
        if !AmbassadorPlaced::valid_ambassadors(game, self.player()).contains(&self.ambassador) {
            return Some(express!("Ambassador not available"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_strongholds(g: &Game, _p: PlayerIndex) -> impl Iterator<Item = Territory> {
        g.map
            .territories(false)
            .filter(move |t| {
                t.is_visible(&g.map) && t.is_stronghold() && !g.is_in_storm_territory(*t) && g.ambassador_in(*t) == Ambassador::None
            })
    }

    pub fn valid_ambassadors(game: &Game, p: PlayerIndex) -> Vec<Ambassador> {
        game.player(p).ambassadors.clone()
    }

    pub fn is_applicable(g: &Game, p: PlayerIndex) -> bool {
        g.player(p).resources > g.ambassadors_placed_this_turn
            && g.current_phase == Phase::ResurrectionReport
            && !g.prevented(FactionAdvantage::EcazAmbassadors)
            && !AmbassadorPlaced::valid_ambassadors(g, p).is_empty()
            && !AmbassadorPlaced::valid_strongholds(g, p).next().is_none()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let Some(stronghold) = self.stronghold(game) else {
            game_error!("Stronghold is null");
        };

        game.ambassadors_placed_this_turn += 1;
        let me = self.player();
        let placed = game.ambassadors_placed_this_turn;
        let p = game.player_mut(me);
        p.resources -= placed;
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
        if game.ambassadors_on_planet.contains_key(&stronghold) {
            game_error!("There already is an ambassador in {}", ExpressionElement::from(stronghold).to_cs_string());
        }
        game.ambassadors_on_planet.add_with(stronghold, self.ambassador, |k| ExpressionElement::from(*k).to_cs_string());
        // List.Remove: the first equal element
        let ambassadors = &mut game.player_mut(me).ambassadors;
        if let Some(pos) = ambassadors.iter().position(|a| *a == self.ambassador) {
            ambassadors.remove(pos);
        }
        game.stone(Milestone::AmbassadorPlaced);
        Ok(())
    }

    pub fn get_message(&self, game: &Game) -> Message {
        express!(self.initiator(), " station the ", self.ambassador, " ambassador in ", self.stronghold(game))
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
