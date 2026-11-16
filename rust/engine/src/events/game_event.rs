use crate::prelude::*;
use crate::messages::part::Part;

/// The submitter of an `@legacy` event, embedded as `base` in its struct. `Game::submit` fills it
/// from the history entry before validating (`Event::prepare`). Converted events have no base:
/// they get the submitter as a [`By`] argument.
#[derive(Clone, Debug)]
pub struct GameEventBase {
    /// Interior mutability: written when the event is submitted, after it may be stored.
    pub initiator: Cell<Option<Faction>>,
    /// A copy of the entry's time (`Game::play`, `History::stamp_last`), because copies of the
    /// event take it along: a deal's cancellation carries the time of the offer.
    pub time: Cell<DateTimeOffset>,
    /// Interior mutability: written when the event is submitted, after it may be stored.
    pub player: Cell<Option<PlayerId>>,
}

impl Default for GameEventBase {
    fn default() -> Self {
        GameEventBase {
            initiator: Cell::new(Default::default()),
            time: Cell::new(Default::default()),
            player: Cell::new(Default::default()),
        }
    }
}

/// The members of an `@legacy` event. Every legacy event implements this trait (in its module);
/// members an event overrides are delegated to its inherent method. Converted events implement
/// [`EventRules`] instead.
pub trait GameEventMethods {
    /// The `Event` fields of this event.
    fn base(&self) -> &GameEventBase;

    fn event_type(&self) -> EventType;

    /// Upcast to the polymorphic event (shares the object).
    fn into_event(self: Rc<Self>) -> Event;

    fn initiator(&self) -> Option<Faction> {
        self.base().initiator.get()
    }

    /// `Player`, dereferenced: NullReferenceException when it was never set.
    fn player(&self) -> PlayerId {
        self.base().player.get().expect("the event has a player")
    }

    /// `Player`, possibly null.
    fn player_opt(&self) -> Option<PlayerId> {
        self.base().player.get()
    }

    fn validate(&self, game: &Game) -> Option<Message>;

    /// The validation of [`try_execute`](GameEventMethods::try_execute): the refusal, or why the
    /// event can't even be validated (a submitted event can lack values its validation needs).
    /// Events with such values override this; their `validate` reports the failure as a refusal.
    fn try_validate(&self, game: &Game) -> GameResult<Option<Message>> {
        Ok(self.validate(game))
    }

    fn is_valid(&self, game: &Game) -> bool {
        self.validate(game).is_none()
    }

    fn initialize(&self, game: &Game, initiator: Option<Faction>) {
        self.base().initiator.set(initiator);

        let player = game.get_player(initiator);
        if let Some(player) = player {
            self.base().player.set(Some(player));
        }
    }

    fn initialize_game(&self, game: &Game) {
        self.initialize(game, self.initiator())
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult;

    fn get_message(&self, _game: &Game) -> Message {
        express!(self.event_type().name(), " by ", self.initiator())
    }

    /// The message, or why it can't be built: some events can be submitted without values their
    /// message needs (a faction that isn't playing, no card). The refusal of an event that isn't
    /// applicable quotes its message through this.
    fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        Ok(self.get_message(game))
    }

    fn get_short_message(&self, game: &Game) -> Message {
        self.get_message(game)
    }

    fn by(&self, f: impl Into<Option<Faction>>) -> bool {
        let f: Option<Faction> = f.into();
        self.initiator() == f
    }

    fn by_ally_of(&self, game: &Game, f: Faction) -> bool {
        game.player(self.player()).ally == Some(f)
    }

    fn by_player(&self, _game: &Game, p: PlayerId) -> bool {
        // Player == p (reference equality; a null Player is never equal)
        self.player_opt() == Some(p)
    }

    fn log(&self, game: &mut Game) {
        let m = self.get_message(game);
        game.current_report_mut().express_event(m);
    }

    fn log_objects(&self, game: &mut Game, expression: Vec<Part>) {
        game.log(expression);
    }

    fn log_if(&self, game: &mut Game, condition: bool, expression: Vec<Part>) {
        game.log_if(condition, expression);
    }

    fn log_to(&self, game: &mut Game, faction: Faction, expression: Vec<Part>) {
        game.log_to(faction, expression);
    }

    fn get_player(&self, game: &Game, f: Option<Faction>) -> Option<PlayerId> {
        game.get_player(f)
    }

    fn is_playing(&self, game: &Game, f: impl Into<Option<Faction>>) -> bool {
        let f: Option<Faction> = f.into();
        game.is_playing(f)
    }

}

impl Event {
    /// `((LocationEvent)e).To` of an event the game holds (`LastShipmentOrMovement`,
    /// `Intrusion.TriggeringEvent`).
    pub fn to_location(&self, game: &Game) -> Option<Location> {
        self.location_event_to(game)
    }
}

/// Associated items shared by all events.
impl Event {
    pub const ID_STRING_SEPARATOR: char = ';';

    pub fn id_string_to_objects<T>(ids: Option<&str>, lookup: &dyn Fetcher<T>) -> Vec<T> {
        let mut result = Vec::new();

        if let Some(ids) = ids {
            if !ids.is_empty() {
                for id in ids.split(Event::ID_STRING_SEPARATOR) {
                    let obj = lookup.find(dotnet_compat::parse::convert_to_int32(Some(id)));
                    if let Some(obj) = obj {
                        result.push(obj);
                    }
                }
            }
        }

        result
    }

    pub fn objects_to_id_string<T: Clone>(objs: &[T], lookup: &dyn Fetcher<T>) -> String {
        objs.iter()
            .map(|pj| lookup.get_id(Some(pj.clone())).to_string())
            .collect::<Vec<_>>()
            .join(&Event::ID_STRING_SEPARATOR.to_string())
    }

}
