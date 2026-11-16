use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct DealRejected {
    pub bound_faction: Option<Faction>,
    #[serde(rename = "dealType")]
    pub type_: DealType,
    pub deal_parameter1: Option<String>,
    pub deal_parameter2: Option<String>,
    pub text: Option<String>,
    pub end: Option<Phase>,
    pub price: u16,
    pub benefit: u16,
}

impl DealRejected {
    /// The offer this rejects that `by` may still answer.
    pub fn open_offer<'a>(&self, game: &'a Game, by: By) -> Option<&'a DealOffer> {
        // AcceptableDeals(Game, Player).FirstOrDefault(...): lazy, one offer at a time.
        let p = by.player?;
        game.deals.offers.iter().find(|offer| DealAccepted::is_acceptable_deal(game, p, offer) && offer.is_rejected_by(self, by.faction))
    }
}

impl EventRules for DealRejected {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !game.deals.offers.iter().any(|offer| offer.is_rejected_by(self, by.faction)) {
            return Err(Rejection(express!("Offer is not valid (anymore)")));
        }
        if self.open_offer(game, by).is_none() {
            return Err(Rejection(express!("You already answered this offer")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // A deal parameter that isn't a territory id fails the event before anything changes.
        let m = self.try_describe(game, by)?.with_log(AsideLog::aside());
        game.log(parts![m]);

        if let Some(offer) = self.open_offer(game, by).map(|o| o.id) {
            game.record_deal_response(offer, by.required_faction()?, false);
        }
        Ok(())
    }

    fn describe(&self, game: &Game, by: By) -> Message {
        express!(
            by.faction,
            " decline ",
            self.bound_faction,
            " offer for ",
            Payment::of(i32::from(self.price)),
            ": ",
            Deal::describe_deal_contents(
                game,
                self.type_,
                self.text.as_deref(),
                i32::from(self.benefit),
                self.end,
                self.deal_parameter1.as_deref(),
            )
        )
    }

    fn try_describe(&self, game: &Game, by: By) -> GameResult<Message> {
        Deal::try_deal_contents_description(game, self.type_, self.text.as_deref(), i32::from(self.benefit), self.end, self.deal_parameter1.as_deref())?;
        Ok(self.describe(game, by))
    }

    fn describe_short(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " decline a deal by ", self.bound_faction)
    }
}
