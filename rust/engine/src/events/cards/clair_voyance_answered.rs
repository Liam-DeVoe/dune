use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClairVoyanceAnswered {
    pub answer: Option<ClairVoyanceAnswer>,
}

impl ClairVoyanceAnswered {
    pub fn is_no(&self) -> bool {
        self.answer == Some(ClairVoyanceAnswer::No)
    }

    pub fn is_yes(&self) -> bool {
        self.answer == Some(ClairVoyanceAnswer::Yes)
    }

    pub fn is_questioned_by(as_weapon: bool, card_type: TreacheryCardType, asked: TreacheryCardType) -> bool {
        if card_type == asked {
            return true;
        }

        match asked {
            TreacheryCardType::PoisonDefense => {
                return card_type == TreacheryCardType::Antidote
                    || (!as_weapon && card_type == TreacheryCardType::Chemistry)
                    || card_type == TreacheryCardType::ShieldAndAntidote;
            }
            TreacheryCardType::Poison => {
                return card_type == TreacheryCardType::PoisonTooth || card_type == TreacheryCardType::ProjectileAndPoison;
            }
            TreacheryCardType::Shield => return card_type == TreacheryCardType::ShieldAndAntidote,
            TreacheryCardType::ProjectileDefense => {
                return card_type == TreacheryCardType::Shield || card_type == TreacheryCardType::ShieldAndAntidote;
            }
            TreacheryCardType::Projectile => {
                return (as_weapon && card_type == TreacheryCardType::WeirdingWay)
                    || card_type == TreacheryCardType::ProjectileAndPoison;
            }
            _ => {}
        }

        false
    }
}

impl EventRules for ClairVoyanceAnswered {
    fn validate(&self, _game: &Game, _by: By) -> Result<(), Rejection> {
        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // `Game.LatestClairvoyance!` does not throw; the NRE comes at the first member access below.
        let latest = game.turn.latest_clairvoyance.clone();
        game.turn.latest_clairvoyance_qn_a = Some(ClairVoyanceQandA {
            question: latest.clone(),
            answer: ClairvoyanceAnswerGiven { submitter: by, answer: self.answer },
        });
        let m = self.describe(game, by).with_log(AsideLog::answer());
        game.log(vec![m.into()]);
        let latest = latest.required("latest")?;

        if latest.question == Some(ClairvoyanceQuestion::WillAttackX) && self.is_no() {
            let mut deal = Deal::default();
            deal.type_ = DealType::DontShipOrMoveTo;
            deal.bound_faction = by.faction;
            deal.consuming_faction = latest.initiator();
            deal.deal_parameter1 = latest.question_parameter1.clone();
            deal.end = Some(Phase::ShipmentAndMoveConcluded);
            game.start_deal(&deal);
        } else if latest.question == Some(ClairvoyanceQuestion::LeaderAsTraitor) {
            if let Some(hero) = latest.parameter1(game).hero() {
                if self.is_yes() {
                    game.player_mut(by.player()).told_traitors.insert(hero);
                } else if self.is_no() {
                    game.player_mut(by.player()).told_non_traitors.insert(hero);
                }
            }
        } else if latest.question == Some(ClairvoyanceQuestion::LeaderAsFacedancer) {
            if let Some(hero) = latest.parameter1(game).hero() {
                if self.is_yes() {
                    game.player_mut(by.player()).told_face_dancers.insert(hero);
                } else if self.is_no() {
                    game.player_mut(by.player()).told_non_face_dancers.insert(hero);
                }
            }
        }

        let paused = game.turn.phase_paused_by_clairvoyance.required("phase paused by clairvoyance")?;
        game.enter(paused, false);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " answer: ", self.answer)
    }
}
