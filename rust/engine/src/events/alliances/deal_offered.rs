use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug, PartialEq)]
pub struct DealOffered {
    pub to: Option<Vec<Faction>>,
    pub type_: DealType,
    pub deal_parameter1: Option<String>,
    pub deal_parameter2: Option<String>,
    pub text: Option<String>,
    pub end_phase: Option<Phase>,
    pub price: u16,
    pub benefit: u16,
    pub cancel: bool,
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
        }
    }
}

impl DealOffered {
    pub fn get_standard_deal_types(g: &Game, p: PlayerId) -> Vec<DealType> {
        let mut result = vec![DealType::None, DealType::ForfeitBattle, DealType::TellDiscardedTraitors];

        match g.player(p).faction {
            Some(Faction::Atreides) => {
                if g.has_bidding_prescience(p) {
                    result.push(DealType::ShareBiddingPrescience);
                }
                if g.has_resource_deck_prescience(Some(p)) {
                    result.push(DealType::ShareResourceDeckPrescience);
                }
            }

            Some(Faction::Fremen) => {
                if g.has_storm_prescience(Some(p)) {
                    result.push(DealType::ShareStormPrescience);
                }
            }

            _ => {}
        }

        result
    }
}

impl EventRules for DealOffered {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        if self.cancel {
            // The first offer with the same terms, whoever made it.
            if let Some(i) = game.deals.offers.iter().position(|o| o.same(self)) {
                let same_offer = game.deals.offers.remove(i);
                game.deals.responses.shift_remove(&same_offer.id);
            }
        } else {
            // The entry of this event goes into the history right after it is applied.
            let id = DealId(game.history.entries.len());
            game.deals.offers.push(DealOffer {
                id,
                initiator: by.faction,
                player: by.player,
                to: self.to.clone(),
                type_: self.type_,
                deal_parameter1: self.deal_parameter1.clone(),
                deal_parameter2: self.deal_parameter2.clone(),
                text: self.text.clone(),
                end_phase: self.end_phase,
                price: self.price,
                benefit: self.benefit,
            });
        }
        Ok(())
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        if !self.cancel {
            let to = self.to.as_ref().expect("destination");
            express!(
                by.faction,
                " offer ",
                express_part_if!(to.len() != 0; Part::list(to.iter().copied()), " "),
                "for ",
                Payment::of(i32::from(self.price)),
                ": ",
                Deal::describe_deal_contents(
                    game,
                    self.type_,
                    self.text.as_deref(),
                    i32::from(self.benefit),
                    self.end_phase,
                    self.deal_parameter1.as_deref(),
                )
            )
        } else {
            express!(by.faction, " withdraw a deal offer")
        }
    }

    fn describe_short(&self, _game: &Game, by: By) -> Message {
        if !self.cancel {
            return express!(by.faction, " offer a deal");
        }
        express!(by.faction, " withdraw a deal offer")
    }
}
