use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct ClairVoyanceAnswered {
    pub answer: ClairVoyanceAnswer,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for ClairVoyanceAnswered {
    fn default() -> Self {
        ClairVoyanceAnswered {
            answer: Default::default(),
            base: Default::default(),
        }
    }
}

impl ClairVoyanceAnswered {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn is_no(&self) -> bool {
        self.answer == ClairVoyanceAnswer::No
    }

    pub fn is_yes(&self) -> bool {
        self.answer == ClairVoyanceAnswer::Yes
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
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

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        // `Game.LatestClairvoyance!` does not throw; the NRE comes at the first member access below.
        let latest = game.latest_clairvoyance.clone();
        game.latest_clairvoyance_qn_a = Some(ClairVoyanceQandA::new(latest.clone(), self));
        let m = self.get_message(game).with_log(AsideLog::answer());
        self.log_objects(game, vec![m.into()]);
        let latest = latest.nre();

        if latest.question == ClairvoyanceQuestion::WillAttackX && self.is_no() {
            let mut deal = Deal::default();
            deal.type_ = DealType::DontShipOrMoveTo;
            deal.bound_faction = self.initiator();
            deal.consuming_faction = latest.initiator();
            deal.deal_parameter1 = latest.question_parameter1.clone();
            deal.end = Phase::ShipmentAndMoveConcluded;
            game.start_deal(&deal);
        } else if latest.question == ClairvoyanceQuestion::LeaderAsTraitor {
            if let ExpressionElement::Hero(hero) = latest.parameter1(game) {
                if self.is_yes() {
                    game.player_mut(self.player()).told_traitors.add(hero);
                } else if self.is_no() {
                    game.player_mut(self.player()).told_non_traitors.add(hero);
                }
            }
        } else if latest.question == ClairvoyanceQuestion::LeaderAsFacedancer {
            if let ExpressionElement::Hero(hero) = latest.parameter1(game) {
                if self.is_yes() {
                    game.player_mut(self.player()).told_face_dancers.add(hero);
                } else if self.is_no() {
                    game.player_mut(self.player()).told_non_face_dancers.add(hero);
                }
            }
        }

        let paused = game.phase_paused_by_clairvoyance;
        game.enter(paused, false);
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " answer: ", self.answer)
    }
}

impl GameEventMethods for ClairVoyanceAnswered {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::ClairVoyanceAnswered
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::ClairVoyanceAnswered(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ClairVoyanceAnswered::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        ClairVoyanceAnswered::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ClairVoyanceAnswered::get_message(self, game)
    }

}
