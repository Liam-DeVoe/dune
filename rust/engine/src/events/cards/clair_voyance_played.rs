use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct ClairVoyancePlayed {
    pub question: ClairvoyanceQuestion,
    pub target: Faction,
    pub question_parameter1: Option<String>,
    pub question_parameter2: Option<String>,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for ClairVoyancePlayed {
    fn default() -> Self {
        ClairVoyancePlayed {
            question: Default::default(),
            target: Default::default(),
            question_parameter1: Default::default(),
            question_parameter2: Default::default(),
            base: Default::default(),
        }
    }
}

impl ClairVoyancePlayed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn is_about(&self, game: &Game, type_: TreacheryCardType) -> bool {
        match self.parameter1(game) {
            ExpressionElement::Enum(e) => e.get::<TreacheryCardType>() == Some(type_),
            _ => false,
        }
    }

    /// The boxed value as a message element: an enum, an `IHero`, a `Territory`, a float, or null.
    pub fn parameter1(&self, game: &Game) -> ExpressionElement {
        let Some(p1) = self.question_parameter1.as_deref() else {
            return ExpressionElement::Null;
        };

        match self.question {
            ClairvoyanceQuestion::CardTypeInBattle
            | ClairvoyanceQuestion::CardTypeAsDefenseInBattle
            | ClairvoyanceQuestion::CardTypeAsWeaponInBattle
            | ClairvoyanceQuestion::HasCardTypeInHand => {
                TreacheryCardType(dotnet_compat::parse::enum_parse(Some(p1), TreacheryCardType::INFO.declared)).into()
            }
            ClairvoyanceQuestion::LeaderAsFacedancer
            | ClairvoyanceQuestion::LeaderAsTraitor
            | ClairvoyanceQuestion::LeaderInBattle => {
                LeaderManager::hero_lookup().find(dotnet_compat::parse::int_parse(Some(p1))).into()
            }
            ClairvoyanceQuestion::Prediction => Faction(dotnet_compat::parse::enum_parse(Some(p1), Faction::INFO.declared)).into(),
            ClairvoyanceQuestion::WillAttackX => game.map.territory_lookup().find(dotnet_compat::parse::int_parse(Some(p1))).into(),
            ClairvoyanceQuestion::DialOfMoreThanXInBattle => dotnet_compat::parse::float_parse_invariant(Some(p1)).into(),
            _ => ExpressionElement::Null,
        }
    }

    pub fn set_parameter1(&mut self, value: ExpressionElement) {
        if let ExpressionElement::Null = value {
            self.question_parameter1 = None;
        } else {
            self.question_parameter1 = Some(match self.question {
                ClairvoyanceQuestion::DialOfMoreThanXInBattle => match value {
                    // ((float)value).ToString(CultureInfo.InvariantCulture)
                    ExpressionElement::Float(f) => dotnet_compat::fmt::f32_to_string(f),
                    other => throw!(
                        InvalidCastException,
                        "Unable to cast object of type '{}' to type 'System.Single'.",
                        ClairVoyancePlayed::boxed_type_name(&other)
                    ),
                },
                _ => value.to_cs_string(),
            });
        }
    }

    /// The runtime type name of a boxed value, for the InvalidCastException of an unboxing cast.
    fn boxed_type_name(e: &ExpressionElement) -> String {
        match e {
            ExpressionElement::String(_) => "System.String".to_string(),
            ExpressionElement::Bool(_) => "System.Boolean".to_string(),
            ExpressionElement::Int(_) => "System.Int32".to_string(),
            ExpressionElement::Long(_) => "System.Int64".to_string(),
            ExpressionElement::Float(_) => "System.Single".to_string(),
            ExpressionElement::Double(_) => "System.Double".to_string(),
            ExpressionElement::Enum(v) => format!("Treachery.Shared.{}", v.ty.name),
            other => other.to_cs_string(),
        }
    }

    pub fn parameter2(&self) -> ExpressionElement {
        match self.question {
            ClairvoyanceQuestion::Prediction
                if !self.question_parameter2.as_deref().map_or(true, |s| s.is_empty()) =>
            {
                dotnet_compat::parse::int_parse(self.question_parameter2.as_deref()).into()
            }
            _ => ExpressionElement::Null,
        }
    }

    pub fn set_parameter2(&mut self, value: ExpressionElement) {
        if let ExpressionElement::Null = value {
            self.question_parameter2 = None;
        } else {
            self.question_parameter2 = Some(value.to_cs_string());
        }
    }

    pub fn get_question(&self, game: &Game) -> Message {
        if self.question == ClairvoyanceQuestion::None {
            return express!("");
        }

        ClairVoyancePlayed::express(self.question, self.parameter1(game), self.parameter2())
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_targets(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Faction> {
        g.players_other_than(p)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_questions(g: &Game, target: Faction) -> impl Iterator<Item = ClairvoyanceQuestion> {
        use either::Either;
        let all_values: Vec<ClairvoyanceQuestion> = ClairvoyanceQuestion::ALL.iter().copied().collect();
        let target_player = g.get_player(Some(target));
        let Some(target_player) = target_player.filter(|tp| g.is_bot(*tp)) else {
            return Either::Left(all_values.into_iter());
        };

        Either::Right(all_values.into_iter().filter(move |v| ClairVoyancePlayed::applies_to_bot(*v, g, target_player)))
    }

    pub fn applies_to_bot(q: ClairvoyanceQuestion, g: &Game, p: PlayerIndex) -> bool {
        match q {
            ClairvoyanceQuestion::CardTypeInBattle
            | ClairvoyanceQuestion::CardTypeAsDefenseInBattle
            | ClairvoyanceQuestion::CardTypeAsWeaponInBattle
            | ClairvoyanceQuestion::LeaderInBattle
            | ClairvoyanceQuestion::DialOfMoreThanXInBattle => {
                g.current_battle.as_ref().map(|b| b.is_aggressor_or_defender(g, p)) == Some(true)
            }
            _ => true,
        }
    }

    pub fn express(q: ClairvoyanceQuestion, parameter1: ExpressionElement, parameter2: ExpressionElement) -> Message {
        let p1: ExpressionElement = match parameter1 {
            ExpressionElement::Null => "...".into(),
            v => v,
        };
        let p2: ExpressionElement = match parameter2 {
            ExpressionElement::Null => "...".into(),
            v => v,
        };

        match q {
            ClairvoyanceQuestion::None => express!("Any question"),
            ClairvoyanceQuestion::CardTypeInBattle => express!("In this battle, is one of your cards a ", p1),
            ClairvoyanceQuestion::CardTypeAsDefenseInBattle => express!("In this battle, will you use ", p1, " as defense?"),
            ClairvoyanceQuestion::CardTypeAsWeaponInBattle => express!("In this battle, will you use ", p1, " as weapon?"),
            ClairvoyanceQuestion::HasCardTypeInHand => express!("Do you own a ", p1, "?"),
            ClairvoyanceQuestion::LeaderInBattle => express!("In this battle, is your leader ", p1, "?"),
            ClairvoyanceQuestion::DialOfMoreThanXInBattle => express!("In this battle, is your dial higher than ", p1, "?"),
            ClairvoyanceQuestion::LeaderAsFacedancer => express!("Is ", p1, " one of your face dancers?"),
            ClairvoyanceQuestion::LeaderAsTraitor => express!("Is ", p1, " one of your traitors?"),
            ClairvoyanceQuestion::Prediction => express!("Did you predict a ", p1, " win in turn ", p2, "?"),
            ClairvoyanceQuestion::WillAttackX => express!("Will you ship or move to ", p1, " this turn?"),
            _ => express!("unknown question"),
        }
    }

    pub fn is_in_scope_of(as_weapon: bool, card_type: TreacheryCardType, in_scope_of_question: TreacheryCardType) -> bool {
        if card_type == in_scope_of_question {
            return true;
        }

        match in_scope_of_question {
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
        let card = self.card(game);
        if let Some(card) = card {
            // The question names the card, so it's not logged as a separate discard.
            game.discard_treachery_card_boolean(Some(card), false);
            let m = self.asked(game, card.into()).with_log(AsideLog::aside());
            self.log_objects(game, vec![m.into()]);
            game.stone(Milestone::Clairvoyance);
        }

        if self.target != Faction::None {
            game.latest_clairvoyance = Some(self.clone());
            game.latest_clairvoyance_qn_a = None;
            game.latest_clairvoyance_battle = game.current_battle.clone();
            game.phase_paused_by_clairvoyance = game.current_phase;
            game.enter(Phase::Clairvoyance, true);
        }
    }

    pub fn card(&self, game: &Game) -> Option<TreacheryCard> {
        let me = self.player();
        let shrine: Location = game.map.shrine().location();
        let mut result: Option<TreacheryCard> = None;
        if game.player(me).occupies(game, Some(shrine)) {
            result = Karma::valid_karma_cards(game, me).into_iter().find(|c| c.type_() == TreacheryCardType::Useless);
        }

        if result.is_none() {
            result = game.player(me).treachery_cards.iter().copied().find(|c| c.type_() == TreacheryCardType::Clairvoyance);
        }

        if result.is_none() && game.player(me).occupies(game, Some(shrine)) {
            result = Karma::valid_karma_cards(game, me).into_iter().next();
        }

        result
    }

    pub fn get_message(&self, game: &Game) -> Message {
        self.asked(game, TreacheryCardType::Clairvoyance.into())
    }

    pub fn asked(&self, game: &Game, card: ExpressionElement) -> Message {
        if self.target == Faction::None {
            return express!(self.initiator(), " perform ", card);
        }

        if self.question == ClairvoyanceQuestion::None {
            return express!("By ", card, ", ", self.initiator(), " ask ", self.target, " a question");
        }

        express!("By ", card, ", ", self.initiator(), " ask ", self.target, ": \"", self.get_question(game), "\"")
    }

    pub fn get_short_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " perform ", TreacheryCardType::Clairvoyance)
    }
}

impl GameEventMethods for ClairVoyancePlayed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::ClairVoyancePlayed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::ClairVoyancePlayed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ClairVoyancePlayed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        ClairVoyancePlayed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ClairVoyancePlayed::get_message(self, game)
    }

    fn get_short_message(&self, game: &Game) -> Message {
        ClairVoyancePlayed::get_short_message(self, game)
    }

}
