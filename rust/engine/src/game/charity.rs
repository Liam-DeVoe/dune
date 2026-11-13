use crate::log::charity::CharityClaim;
use crate::prelude::*;

/// `params object?[]` element list for `Log(...)`.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

impl Game {
    pub fn charity_is_cancelled(&self) -> bool {
        matches!(self.economics_status, ChoamEconomicsStatus::Cancel | ChoamEconomicsStatus::CancelFlipped)
    }

    pub fn receive_tech_income(&mut self, token: TechToken) {
        let tech_token_owner = self.player_indices().find(|p| self.player(*p).tech_tokens.contains(&token));
        if let Some(owner) = tech_token_owner {
            let amount = self.player(owner).tech_tokens.len() as i32;
            let r = &mut self.player_mut(owner).resources;
            *r += amount;
            let faction = self.player(owner).faction;
            self.log(elems![faction, " receive ", Payment::of(amount), " from ", token]);
        }
    }

    pub fn current_charity_multiplier(&self) -> i32 {
        if matches!(self.economics_status, ChoamEconomicsStatus::Double | ChoamEconomicsStatus::DoubleFlipped) {
            return 2;
        }
        if matches!(self.economics_status, ChoamEconomicsStatus::Cancel | ChoamEconomicsStatus::CancelFlipped) {
            return 0;
        }
        1
    }

    pub fn claim_charity(&mut self, player: PlayerIndex) {
        let faction = self.player(player).faction;
        self.has_acted_or_passed.push(faction);

        let basic_amount = (2i32 - self.player(player).resources) * self.current_charity_multiplier();
        self.give_charity(player, basic_amount);

        if !self.player(player).is(Faction::BeneGesserit) {
            self.resource_tech_token_income = true;
        }

        self.stone(Milestone::CharityClaimed);
    }

    pub fn give_charity(&mut self, to: PlayerIndex, basic_amount: i32) {
        let mut homeworld_bonus = 0;
        if self.gets_extra_charity_and_free_revival_due_to_low_threshold(to) {
            homeworld_bonus = 1;
        }

        let choam = self.get_player(Some(Faction::Choam));

        let to_faction = self.player(to).faction;
        let r = &mut self.player_mut(to).resources;
        *r += basic_amount + homeworld_bonus;
        match choam {
            Some(choam) if !self.prevented(FactionAdvantage::ChoamControllingCharity) => {
                if self.player(choam).resources >= basic_amount {
                    let r = &mut self.player_mut(choam).resources;
                    *r -= basic_amount;
                    self.log(elems![
                        to_faction,
                        " get ",
                        Payment::of(basic_amount),
                        " from ",
                        Faction::Choam,
                        express_part_if!(homeworld_bonus > 0; " and ", Payment::of(homeworld_bonus), " from the bank")
                    ]);
                } else {
                    // C# parity: the claimant was already paid above, even though Choam cannot pay.
                    self.log(elems![
                        to_faction,
                        " are unable to claim ",
                        Payment::of(basic_amount),
                        " from ",
                        Faction::Choam,
                        express_part_if!(homeworld_bonus > 0; " but get ", Payment::of(homeworld_bonus), " from the bank")
                    ]);
                }
            }
            _ => {
                let amount = basic_amount + homeworld_bonus;
                let msg = express!(to_faction, " claim ", Payment::of(amount)).with_log(CharityClaim::new(to_faction, amount));
                self.log(elems![msg]);
            }
        }
    }
}
