use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct SetShipmentPermission {
    pub factions: Option<Vec<Faction>>,
    pub permission: ShipmentPermission,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for SetShipmentPermission {
    fn default() -> Self {
        SetShipmentPermission {
            factions: Default::default(),
            permission: Default::default(),
            base: Default::default(),
        }
    }
}

impl SetShipmentPermission {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        // Enumerable.Any on a null array: ArgumentNullException.
        let Some(factions) = &self.factions else {
            throw!(ArgumentNullException, "Value cannot be null. (Parameter 'source')");
        };
        if factions.is_empty() {
            return Some(express!("Select one or more factions"));
        }
        // ValidTargets is re-evaluated for every faction, as in C#.
        if factions.iter().any(|f| !SetShipmentPermission::valid_targets(game, self.player()).any(|v| v == *f)) {
            return Some(express!("Invalid faction"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_targets(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Faction> {
        // `p.Faction` is read inside the predicate, per element, as in C#.
        g.players.iter().filter(move |x| x.faction != g.player(p).faction).map(|x| x.faction)
    }

    pub fn is_applicable(g: &Game, p: PlayerIndex) -> bool {
        g.current_main_phase == MainPhase::ShipmentAndMove && g.player(p).is(Faction::Guild) && g.player(p).has_high_threshold_0(g)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        self.log(game);

        let factions = self.factions.as_ref().nre();
        for f in factions {
            game.shipment_permissions.remove(f);
        }

        if self.permission != ShipmentPermission::None {
            for f in factions {
                // Dictionary.Add: ArgumentException on a faction listed twice (hazards 1.4).
                game.shipment_permissions.add_with(*f, self.permission, |k| k.to_string());
            }
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        // A Faction[] is one message element (a value-type array is not spread).
        let factions: ExpressionElement = match &self.factions {
            Some(f) => ExpressionElement::array(f.iter().copied()),
            None => ExpressionElement::from(Option::<Faction>::None),
        };
        if self.permission == ShipmentPermission::None {
            return express!(self.initiator(), " → ", factions, ": all permissions revoked");
        }
        let has = |flag: ShipmentPermission| (self.permission.0 & flag.0) == flag.0;
        express!(
            self.initiator(),
            " → ",
            factions,
            ": ",
            "cross shipping: ",
            if has(ShipmentPermission::Cross) { "yes" } else { "no" },
            ", to (own) world: ",
            if has(ShipmentPermission::ToHomeworld) { "yes" } else { "no" },
            ", discount: ",
            if has(ShipmentPermission::GuildRate) { "yes" } else { "no" }
        )
    }

}

impl GameEventMethods for SetShipmentPermission {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::SetShipmentPermission
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::SetShipmentPermission(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        SetShipmentPermission::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        SetShipmentPermission::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        SetShipmentPermission::get_message(self, game)
    }

}
