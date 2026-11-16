use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct DealRejected {
    pub bound_faction: Option<Faction>,
    pub type_: DealType,
    pub deal_parameter1: Option<String>,
    pub deal_parameter2: Option<String>,
    pub text: Option<String>,
    pub end: Option<Phase>,
    pub price: i32,
    pub benefit: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for DealRejected {
    fn default() -> Self {
        DealRejected {
            bound_faction: Default::default(),
            type_: Default::default(),
            deal_parameter1: Default::default(),
            deal_parameter2: Default::default(),
            text: Default::default(),
            end: Default::default(),
            price: Default::default(),
            benefit: Default::default(),
            base: Default::default(),
        }
    }
}

impl DealRejected {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !game.deals.offers.iter().any(|offer| offer.is_rejected_by(self)) {
            return Some(express!("Offer is not valid (anymore)"));
        }
        if self.open_offer(game).is_none() {
            return Some(express!("You already answered this offer"));
        }

        None
    }

    pub fn open_offer(&self, game: &Game) -> Option<Rc<DealOffered>> {
        // AcceptableDeals(Game, Player).FirstOrDefault(...): lazy, one offer at a time.
        let Some(p) = self.player_opt() else { return None };
        game.deals.offers
            .iter()
            .find(|offer| DealAccepted::is_acceptable_deal(game, p, offer) && offer.is_rejected_by(self))
            .cloned()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        // A deal parameter that isn't a territory id fails the event before anything changes.
        let m = self.try_get_message(game)?.with_log(AsideLog::aside());
        self.log_objects(game, vec![m.into()]);

        let offer = self.open_offer(game);
        if let Some(offer) = offer {
            game.record_deal_response(&offer, self.initiator().required("faction")?, false);
        }
        Ok(())
    }

    pub fn get_message(&self, game: &Game) -> Message {
        express!(
            self.initiator(),
            " decline ",
            self.bound_faction,
            " offer for ",
            Payment::of(self.price),
            ": ",
            Deal::describe_deal_contents(
                game,
                self.type_,
                self.text.as_deref(),
                self.benefit,
                self.end,
                self.deal_parameter1.as_deref(),
            )
        )
    }

    pub fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        Deal::try_deal_contents_description(game, self.type_, self.text.as_deref(), self.benefit, self.end, self.deal_parameter1.as_deref())?;
        Ok(self.get_message(game))
    }

    pub fn get_short_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " decline a deal by ", self.bound_faction)
    }

}

impl GameEventMethods for DealRejected {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::DealRejected
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::DealRejected(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        DealRejected::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        DealRejected::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        DealRejected::get_message(self, game)
    }

    fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        DealRejected::try_get_message(self, game)
    }

    fn get_short_message(&self, game: &Game) -> Message {
        DealRejected::get_short_message(self, game)
    }

}
