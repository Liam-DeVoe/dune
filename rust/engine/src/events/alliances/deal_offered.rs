use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct DealOffered {
    pub to: Option<Vec<Faction>>,
    pub type_: DealType,
    pub deal_parameter1: Option<String>,
    pub deal_parameter2: Option<String>,
    pub text: Option<String>,
    pub end_phase: Phase,
    pub price: i32,
    pub benefit: i32,
    pub cancel: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for DealOffered {
    fn default() -> Self {
        DealOffered {
            to: Some(Default::default()),
            type_: Default::default(),
            deal_parameter1: Default::default(),
            deal_parameter2: Default::default(),
            text: Default::default(),
            end_phase: Default::default(),
            price: Default::default(),
            benefit: Default::default(),
            cancel: Default::default(),
            base: Default::default(),
        }
    }
}

impl DealOffered {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    pub fn cancellation(&self) -> Option<Rc<DealOffered>> {
        // MemberwiseClone: the same field values, including Initiator and Player.
        let mut result = self.clone();
        result.cancel = true;
        Some(Rc::new(result))
    }

    pub fn acceptance(&self, game: &Game, by: Faction) -> Option<Rc<DealAccepted>> {
        // new DealAccepted(Game, by)
        let mut result = DealAccepted::new(game, by);
        result.bound_faction = self.initiator();
        result.price = self.price;
        result.type_ = self.type_;
        result.deal_parameter1 = self.deal_parameter1.clone();
        result.deal_parameter2 = self.deal_parameter2.clone();
        result.text = self.text.clone();
        result.benefit = self.benefit;
        result.end = self.end_phase;
        Some(Rc::new(result))
    }

    pub fn same(&self, offered_deal: &DealOffered) -> bool {
        offered_deal.price == self.price
            && offered_deal.type_ == self.type_
            && offered_deal.deal_parameter1 == self.deal_parameter1
            && offered_deal.deal_parameter2 == self.deal_parameter2
            && offered_deal.text == self.text
            && offered_deal.benefit == self.benefit
            && offered_deal.end_phase == self.end_phase
            // C# parity: C# 14 binds `Faction[].SequenceEqual(Faction[])` to
            // MemoryExtensions.SequenceEqual on spans, where a null array is an
            // empty span. So null equals [] and nothing throws.
            && offered_deal.to.as_deref().unwrap_or(&[]) == self.to.as_deref().unwrap_or(&[])
    }

    pub fn rejection(&self, game: &Game, by: Faction) -> Option<Rc<DealRejected>> {
        // new DealRejected(Game, by)
        let mut result = DealRejected::new(game, by);
        result.bound_faction = self.initiator();
        result.price = self.price;
        result.type_ = self.type_;
        result.deal_parameter1 = self.deal_parameter1.clone();
        result.deal_parameter2 = self.deal_parameter2.clone();
        result.text = self.text.clone();
        result.benefit = self.benefit;
        result.end = self.end_phase;
        Some(Rc::new(result))
    }

    pub fn is_accepted_by(&self, accepted_deal: &DealAccepted) -> bool {
        self.matches(
            accepted_deal.initiator(),
            accepted_deal.bound_faction,
            accepted_deal.price,
            accepted_deal.type_,
            accepted_deal.deal_parameter1.as_deref(),
            accepted_deal.deal_parameter2.as_deref(),
            accepted_deal.text.as_deref(),
            accepted_deal.benefit,
            accepted_deal.end,
        )
    }

    pub fn is_rejected_by(&self, rejected_deal: &DealRejected) -> bool {
        self.matches(
            rejected_deal.initiator(),
            rejected_deal.bound_faction,
            rejected_deal.price,
            rejected_deal.type_,
            rejected_deal.deal_parameter1.as_deref(),
            rejected_deal.deal_parameter2.as_deref(),
            rejected_deal.text.as_deref(),
            rejected_deal.benefit,
            rejected_deal.end,
        )
    }

    pub fn matches(&self, by: Faction, bound_faction: Faction, price: i32, type_: DealType, deal_parameter1: Option<&str>, deal_parameter2: Option<&str>, text: Option<&str>, benefit: i32, end: Phase) -> bool {
        bound_faction == self.initiator()
            && price == self.price
            && type_ == self.type_
            && deal_parameter1 == self.deal_parameter1.as_deref()
            && deal_parameter2 == self.deal_parameter2.as_deref()
            && text == self.text.as_deref()
            && benefit == self.benefit
            && end == self.end_phase
            && {
                let to = self.to.as_ref().expect("destination");
                to.len() == 0 || to.contains(&by)
            }
    }

    pub fn get_standard_deal_types(g: &Game, p: PlayerIndex) -> Vec<DealType> {
        let mut result = vec![DealType::None, DealType::ForfeitBattle, DealType::TellDiscardedTraitors];

        match g.player(p).faction {
            Faction::Atreides => {
                if g.has_bidding_prescience(p) {
                    result.push(DealType::ShareBiddingPrescience);
                }
                if g.has_resource_deck_prescience(Some(p)) {
                    result.push(DealType::ShareResourceDeckPrescience);
                }
            }

            Faction::Fremen => {
                if g.has_storm_prescience(Some(p)) {
                    result.push(DealType::ShareStormPrescience);
                }
            }

            _ => {}
        }

        result
    }

    pub fn get_deal_description(&self, game: &Game) -> Message {
        Deal::deal_contents_description_game_deal_type_string_int32_phase_string(
            game,
            self.type_,
            self.text.as_deref(),
            self.benefit,
            self.end_phase,
            self.deal_parameter1.as_deref(),
        )
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        if self.cancel {
            let same_offer = game.deal_offers.iter().find(|o| o.same(self)).cloned();
            if let Some(same_offer) = same_offer {
                // List.Remove: reference equality, first occurrence.
                if let Some(i) = game.deal_offers.iter().position(|o| Rc::ptr_eq(o, &same_offer)) {
                    game.deal_offers.remove(i);
                }
                game.deal_responses.shift_remove(&RcKey(same_offer));
            }
        } else {
            game.deal_offers.push(self.clone());
        }
        Ok(())
    }

    pub fn get_message(&self, game: &Game) -> Message {
        if !self.cancel {
            let to = self.to.as_ref().expect("destination");
            express!(
                self.initiator(),
                " offer ",
                express_part_if!(to.len() != 0; ExpressionElement::array(to.iter().copied()), " "),
                "for ",
                Payment::of(self.price),
                ": ",
                Deal::deal_contents_description_game_deal_type_string_int32_phase_string(
                    game,
                    self.type_,
                    self.text.as_deref(),
                    self.benefit,
                    self.end_phase,
                    self.deal_parameter1.as_deref(),
                )
            )
        } else {
            express!(self.initiator(), " withdraw a deal offer")
        }
    }

    pub fn get_short_message(&self, _game: &Game) -> Message {
        if !self.cancel {
            return express!(self.initiator(), " offer a deal");
        }
        express!(self.initiator(), " withdraw a deal offer")
    }

}

impl GameEventMethods for DealOffered {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::DealOffered
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::DealOffered(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        DealOffered::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        DealOffered::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        DealOffered::get_message(self, game)
    }

    fn get_short_message(&self, game: &Game) -> Message {
        DealOffered::get_short_message(self, game)
    }

}
