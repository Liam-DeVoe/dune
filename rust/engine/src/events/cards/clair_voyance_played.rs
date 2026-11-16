use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug)]
pub struct ClairVoyancePlayed {
    pub question: Option<ClairvoyanceQuestion>,
    pub target: Option<Faction>,
    pub question_parameter1: Option<String>,
    pub question_parameter2: Option<String>,
    /// The fields of `Event` (Initiator, Time, Game, Player).
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
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn is_about(&self, game: &Game, type_: TreacheryCardType) -> bool {
        match self.parameter1(game) {
            p => p.get::<TreacheryCardType>() == Some(type_),
        }
    }

    /// The boxed value as a message element: an enum, an `IHero`, a `Territory`, a strength, or null.
    /// Panics like .NET's parse when the parameter doesn't fit the question.
    pub fn parameter1(&self, game: &Game) -> Part {
        self.parse_parameter1(game).unwrap_or_else(|e| panic!("{}: {}", e.type_name, e.message))
    }

    /// [`parameter1`](Self::parameter1), failing the event when the parameter doesn't fit the question.
    pub fn try_parameter1(&self, game: &Game) -> GameResult<Part> {
        self.parse_parameter1(game).map_err(|e| GameError::new(e.message))
    }

    fn parse_parameter1(&self, game: &Game) -> Result<Part, dotnet_compat::exception::CsException> {
        use dotnet_compat::parse::{try_enum_parse, try_halves_parse_invariant, try_int_parse};
        let Some(p1) = self.question_parameter1.as_deref() else {
            return Ok(Part::Null);
        };

        Ok(match self.question {
            Some(ClairvoyanceQuestion::CardTypeInBattle)
            | Some(ClairvoyanceQuestion::CardTypeAsDefenseInBattle)
            | Some(ClairvoyanceQuestion::CardTypeAsWeaponInBattle)
            | Some(ClairvoyanceQuestion::HasCardTypeInHand) => {
                let n = try_enum_parse(Some(p1), TreacheryCardType::INFO.declared)?;
                let Ok(card_type) = crate::enum_type::from_value_or_absent::<TreacheryCardType>(n) else {
                    return Err(dotnet_compat::exception::CsException { type_name: "ArgumentException", message: format!("{n} is not a valid TreacheryCardType") });
                };
                card_type.into()
            }
            Some(ClairvoyanceQuestion::LeaderAsFacedancer)
            | Some(ClairvoyanceQuestion::LeaderAsTraitor)
            | Some(ClairvoyanceQuestion::LeaderInBattle) => {
                LeaderManager::hero_lookup().find(try_int_parse(Some(p1))?).into()
            }
            Some(ClairvoyanceQuestion::Prediction) => {
                let n = try_enum_parse(Some(p1), Faction::INFO.declared)?;
                let Ok(faction) = crate::enum_type::from_value_or_absent::<Faction>(n) else {
                    return Err(dotnet_compat::exception::CsException { type_name: "ArgumentException", message: format!("{n} is not a valid Faction") });
                };
                faction.into()
            }
            Some(ClairvoyanceQuestion::WillAttackX) => game.board.map.territory_lookup().find(try_int_parse(Some(p1))?).into(),
            Some(ClairvoyanceQuestion::DialOfMoreThanXInBattle) => HalfPoints::from_halves(try_halves_parse_invariant(Some(p1))?).into(),
            _ => Part::Null,
        })
    }

    pub fn set_parameter1(&mut self, value: Part) {
        if let Part::Null = value {
            self.question_parameter1 = None;
        } else {
            self.question_parameter1 = Some(match self.question {
                Some(ClairvoyanceQuestion::DialOfMoreThanXInBattle) => match value {
                    // ((float)value).ToString(CultureInfo.InvariantCulture)
                    Part::Strength(h) => h.to_string(),
                    other => panic!("Unable to cast object of type '{}' to type 'System.Single'.",
                        ClairVoyancePlayed::boxed_type_name(&other)),
                },
                _ => match value {
                    Part::Number(i) => i.to_string(),
                    other => panic!("Unable to cast object of type '{}' to type 'System.String'.",
                        ClairVoyancePlayed::boxed_type_name(&other)),
                },
            });
        }
    }

    /// The runtime type name of a boxed value, for the InvalidCastException of an unboxing cast.
    fn boxed_type_name(e: &Part) -> String {
        match e {
            Part::Text(_) => "System.String".to_string(),
            Part::Bool(_) => "System.Boolean".to_string(),
            Part::Number(_) => "System.Int32".to_string(),
            Part::Strength(_) => "System.Single".to_string(),
            other => match other.enum_value() {
                Some((info, _)) => format!("Treachery.Shared.{}", info.name),
                None => format!("{other:?}"),
            },
        }
    }

    /// Panics like .NET's parse when the parameter isn't a number.
    pub fn parameter2(&self) -> Part {
        self.parse_parameter2().unwrap_or_else(|e| panic!("{}: {}", e.type_name, e.message))
    }

    /// [`parameter2`](Self::parameter2), failing the event when the parameter isn't a number.
    pub fn try_parameter2(&self) -> GameResult<Part> {
        self.parse_parameter2().map_err(|e| GameError::new(e.message))
    }

    fn parse_parameter2(&self) -> Result<Part, dotnet_compat::exception::CsException> {
        Ok(match self.question {
            Some(ClairvoyanceQuestion::Prediction)
                if !self.question_parameter2.as_deref().map_or(true, |s| s.is_empty()) =>
            {
                dotnet_compat::parse::try_int_parse(self.question_parameter2.as_deref())?.into()
            }
            _ => Part::Null,
        })
    }

    pub fn get_question(&self, game: &Game) -> Message {
        if self.question == None {
            return express!("");
        }

        ClairVoyancePlayed::express(self.question, self.parameter1(game), self.parameter2())
    }

    pub fn try_get_question(&self, game: &Game) -> GameResult<Message> {
        if self.question == None {
            return Ok(express!(""));
        }

        Ok(ClairVoyancePlayed::express(self.question, self.try_parameter1(game)?, self.try_parameter2()?))
    }

    pub fn validate(&self, _game: &Game) -> Option<Message> {
        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Faction> {
        g.players_other_than(p)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_questions(g: &Game, target: impl Into<Option<Faction>>) -> impl Iterator<Item = Option<ClairvoyanceQuestion>> {
        let target: Option<Faction> = target.into();
        use either::Either;
        // `None` first: "any question".
        let all_values: Vec<Option<ClairvoyanceQuestion>> = std::iter::once(None).chain(ClairvoyanceQuestion::ALL.iter().copied().map(Some)).collect();
        let target_player = g.get_player(target);
        let Some(target_player) = target_player.filter(|tp| g.is_bot(*tp)) else {
            return Either::Left(all_values.into_iter());
        };

        Either::Right(all_values.into_iter().filter(move |v| ClairVoyancePlayed::applies_to_bot(*v, g, target_player)))
    }

    pub fn applies_to_bot(q: Option<ClairvoyanceQuestion>, g: &Game, p: PlayerId) -> bool {
        match q {
            Some(
                ClairvoyanceQuestion::CardTypeInBattle
                | ClairvoyanceQuestion::CardTypeAsDefenseInBattle
                | ClairvoyanceQuestion::CardTypeAsWeaponInBattle
                | ClairvoyanceQuestion::LeaderInBattle
                | ClairvoyanceQuestion::DialOfMoreThanXInBattle,
            ) => {
                g.battle.current_battle.as_ref().map(|b| b.is_aggressor_or_defender(g, p)) == Some(true)
            }
            _ => true,
        }
    }

    pub fn express(q: Option<ClairvoyanceQuestion>, parameter1: Part, parameter2: Part) -> Message {
        let p1: Part = match parameter1 {
            Part::Null => "...".into(),
            v => v,
        };
        let p2: Part = match parameter2 {
            Part::Null => "...".into(),
            v => v,
        };

        match q {
            None => express!("Any question"),
            Some(ClairvoyanceQuestion::CardTypeInBattle) => express!("In this battle, is one of your cards a ", p1),
            Some(ClairvoyanceQuestion::CardTypeAsDefenseInBattle) => express!("In this battle, will you use ", p1, " as defense?"),
            Some(ClairvoyanceQuestion::CardTypeAsWeaponInBattle) => express!("In this battle, will you use ", p1, " as weapon?"),
            Some(ClairvoyanceQuestion::HasCardTypeInHand) => express!("Do you own a ", p1, "?"),
            Some(ClairvoyanceQuestion::LeaderInBattle) => express!("In this battle, is your leader ", p1, "?"),
            Some(ClairvoyanceQuestion::DialOfMoreThanXInBattle) => express!("In this battle, is your dial higher than ", p1, "?"),
            Some(ClairvoyanceQuestion::LeaderAsFacedancer) => express!("Is ", p1, " one of your face dancers?"),
            Some(ClairvoyanceQuestion::LeaderAsTraitor) => express!("Is ", p1, " one of your traitors?"),
            Some(ClairvoyanceQuestion::Prediction) => express!("Did you predict a ", p1, " win in turn ", p2, "?"),
            Some(ClairvoyanceQuestion::WillAttackX) => express!("Will you ship or move to ", p1, " this turn?"),
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

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let card = self.card(game);
        if let Some(card) = card {
            // The question names the card, so it's not logged as a separate discard.
            game.discard_card(Some(card), false)?;
            // A parameter that doesn't fit the question fails the event after the discard.
            let m = self.try_asked(game, card.into())?.with_log(AsideLog::aside());
            self.log_objects(game, vec![m.into()]);
            game.stone(Milestone::Clairvoyance);
        }

        if self.target != None {
            game.turn.latest_clairvoyance = Some(self.clone());
            game.turn.latest_clairvoyance_qn_a = None;
            game.turn.latest_clairvoyance_battle = game.battle.current_battle.clone();
            game.turn.phase_paused_by_clairvoyance = Some(game.turn.current_phase);
            game.enter(Phase::Clairvoyance, true);
        }
        Ok(())
    }

    pub fn card(&self, game: &Game) -> Option<TreacheryCard> {
        let me = self.player();
        let shrine: Location = game.board.map.shrine().location();
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

    pub fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        self.try_asked(game, TreacheryCardType::Clairvoyance.into())
    }

    pub fn asked(&self, game: &Game, card: Part) -> Message {
        if self.target == None {
            return express!(self.initiator(), " perform ", card);
        }

        if self.question == None {
            return express!("By ", card, ", ", self.initiator(), " ask ", self.target, " a question");
        }

        express!("By ", card, ", ", self.initiator(), " ask ", self.target, ": \"", self.get_question(game), "\"")
    }

    pub fn try_asked(&self, game: &Game, card: Part) -> GameResult<Message> {
        if self.target != None && self.question != None {
            self.try_get_question(game)?;
        }
        Ok(self.asked(game, card))
    }

    pub fn get_short_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " perform ", TreacheryCardType::Clairvoyance)
    }
}

impl GameEventMethods for ClairVoyancePlayed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> EventType {
        EventType::ClairVoyancePlayed
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::ClairVoyancePlayed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        ClairVoyancePlayed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        ClairVoyancePlayed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        ClairVoyancePlayed::get_message(self, game)
    }

    fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        ClairVoyancePlayed::try_get_message(self, game)
    }

    fn get_short_message(&self, game: &Game) -> Message {
        ClairVoyancePlayed::get_short_message(self, game)
    }

}
