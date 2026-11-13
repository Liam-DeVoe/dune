use crate::prelude::*;

/// The fields shared by all events, embedded as `base` in every event struct.
/// The polymorphic `GameEvent` itself is the enum in `events/mod.rs`.
#[derive(Clone, Debug, Canon)]
pub struct GameEventBase {
    /// Interior mutability: written after the event is stored in game state.
    pub initiator: Cell<Faction>,
    /// Interior mutability: the server sets this member after `Execute`, when the event is already in
    /// `History`.
    pub time: Cell<DateTimeOffset>,
    /// Whether the event was initialized against a game (C#'s `Game` field is set). Only
    /// the state dump reads it.
    #[canon(name = "Game", with = "crate::canon::game_link")]
    pub initialized: Cell<bool>,
    /// Interior mutability: written after the event is stored in game state.
    pub player: Cell<Option<PlayerIndex>>,
}

impl Default for GameEventBase {
    fn default() -> Self {
        GameEventBase {
            initiator: Cell::new(Default::default()),
            time: Cell::new(Default::default()),
            initialized: Cell::new(false),
            player: Cell::new(Default::default()),
        }
    }
}

/// The instance members shared by all events. Every event implements this trait
/// (generated in its module); members an event overrides are delegated to its inherent method.
pub trait GameEventMethods {
    /// The `GameEvent` fields of this event.
    fn base(&self) -> &GameEventBase;

    /// `GetType()`
    fn event_type(&self) -> GameEventType;

    /// Upcast to the polymorphic event (shares the object).
    fn into_event(self: Rc<Self>) -> GameEvent;

    /// `Initiator`
    fn initiator(&self) -> Faction {
        self.base().initiator.get()
    }

    /// `Time`
    fn time(&self) -> DateTimeOffset {
        self.base().time.get()
    }

    /// `Player`, dereferenced: NullReferenceException when it was never set (hazards.md 1.8).
    fn player(&self) -> PlayerIndex {
        self.base().player.get().nre()
    }

    /// `Player`, possibly null.
    fn player_opt(&self) -> Option<PlayerIndex> {
        self.base().player.get()
    }

    fn validate(&self, game: &Game) -> Option<Message>;

    fn is_valid(&self, game: &Game) -> bool {
        self.validate(game).is_none()
    }

    fn is_applicable(&self, game: &mut Game, is_host: bool) -> bool {
        // `if (Game == null) throw new ArgumentException(...)`: an event always has its
        // game here (the caller passes it).
        let player = self.player_opt();
        game.get_applicable_events(player, is_host).contains(&self.event_type())
    }

    fn initialize(&self, game: &Game, initiator: Faction) {
        self.base().initialized.set(true);
        self.base().initiator.set(initiator);

        let player = game.get_player(Some(initiator));
        if let Some(player) = player {
            self.base().player.set(Some(player));
        }
    }

    fn initialize_game(&self, game: &Game) {
        self.initialize(game, self.initiator())
    }

    fn execute(self: &Rc<Self>, game: &mut Game, perform_validation: bool, is_host: bool) -> Option<Message> {
        // `if (Game == null) throw new ArgumentException(...)` is outside the try in C#;
        // here the game is always passed in.
        let result = cs::catch(|| {
            let moment_just_before_event = game.current_moment;

            let mut result: Option<Message> = None;

            if perform_validation {
                if !self.is_applicable(game, is_host) {
                    return Some(express!("Event '", self.get_message(game), "' is not applicable"));
                }

                result = self.validate(game);
            }

            if result.is_none() {
                let this: GameEvent = self.clone().into_event();
                game.recent_milestones.clear();
                game.perform_pre_event_tasks(&this);
                self.execute_concrete_event(game);
                let just_entered_start_of_phase =
                    moment_just_before_event != MainPhaseMoment::Start && game.current_moment == MainPhaseMoment::Start;
                game.perform_post_event_tasks(&this, just_entered_start_of_phase);
            }

            result
        });

        match result {
            Ok(r) => r,
            Err(e) => Some(express!("Game Error: ", e.message.clone(), ". Technical description: ", e, ".")),
        }
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game);

    fn get_message(&self, _game: &Game) -> Message {
        express!(self.event_type().name(), " by ", self.initiator())
    }

    fn get_short_message(&self, game: &Game) -> Message {
        self.get_message(game)
    }

    fn by(&self, f: Faction) -> bool {
        self.initiator() == f
    }

    fn by_ally_of(&self, game: &Game, f: Faction) -> bool {
        game.player(self.player()).ally == f
    }

    fn by_player(&self, _game: &Game, p: PlayerIndex) -> bool {
        // Player == p (reference equality; a null Player is never equal)
        self.player_opt() == Some(p)
    }

    fn cs_clone(&self) -> Option<GameEvent>
    where
        Self: Clone + Sized,
    {
        // MemberwiseClone: a shallow copy with the same field values
        Some(Rc::new(self.clone()).into_event())
    }

    fn log(&self, game: &mut Game) {
        let m = self.get_message(game);
        game.current_report_mut().express_event(m);
    }

    fn log_objects(&self, game: &mut Game, expression: Vec<ExpressionElement>) {
        game.log(expression);
    }

    fn log_if(&self, game: &mut Game, condition: bool, expression: Vec<ExpressionElement>) {
        game.log_if(condition, expression);
    }

    fn log_to(&self, game: &mut Game, faction: Faction, expression: Vec<ExpressionElement>) {
        game.log_to(faction, expression);
    }

    fn get_player(&self, game: &Game, f: Option<Faction>) -> Option<PlayerIndex> {
        game.get_player(f)
    }

    fn is_playing(&self, game: &Game, f: Faction) -> bool {
        game.is_playing(f)
    }

}

impl GameEvent {
    /// `((ILocationEvent)e).To` of an event the game holds (`LastShipmentOrMovement`,
    /// `Intrusion.TriggeringEvent`).
    pub fn to_location(&self, game: &Game) -> Option<Location> {
        self.i_location_event_to(game)
    }
}

/// Associated items shared by all events.
impl GameEvent {
    pub const ID_STRING_SEPARATOR: char = ';';

    pub fn id_string_to_objects<T>(ids: Option<&str>, lookup: &dyn IFetcher<T>) -> Vec<T> {
        let mut result = Vec::new();

        if let Some(ids) = ids {
            if !ids.is_empty() {
                for id in ids.split(GameEvent::ID_STRING_SEPARATOR) {
                    let obj = lookup.find(dotnet_compat::parse::convert_to_int32(Some(id)));
                    if let Some(obj) = obj {
                        result.push(obj);
                    }
                }
            }
        }

        result
    }

    pub fn objects_to_id_string<T: Clone>(objs: &[T], lookup: &dyn IFetcher<T>) -> String {
        objs.iter()
            .map(|pj| lookup.get_id(Some(pj.clone())).to_string())
            .collect::<Vec<_>>()
            .join(&GameEvent::ID_STRING_SEPARATOR.to_string())
    }

}
