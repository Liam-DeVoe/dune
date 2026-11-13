use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct Voice {
    pub must: bool,
    pub type_: TreacheryCardType,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for Voice {
    fn default() -> Self {
        Voice {
            must: Default::default(),
            type_: Default::default(),
            base: Default::default(),
        }
    }
}

macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

impl Voice {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn may_not(&self) -> bool {
        !self.must
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !Self::may_use_voice_opt(game, self.player_opt()) {
            return Some(express!("You cannot use Voice"));
        }
        if !Self::valid_types(game).contains(&self.type_) {
            return Some(express!("Invalid use of Voice"));
        }

        None
    }

    pub fn valid_types(g: &Game) -> Vec<TreacheryCardType> {
        let mut result = vec![
            TreacheryCardType::Mercenary,
            TreacheryCardType::Laser,
            TreacheryCardType::Poison,
            TreacheryCardType::Projectile,
            TreacheryCardType::Shield,
            TreacheryCardType::Antidote,
            TreacheryCardType::Useless,
        ];

        if g.applicable(Rule::ExpansionTreacheryCardsExceptPBandSSandAmal) {
            result.push(TreacheryCardType::ArtilleryStrike);
            result.push(TreacheryCardType::Chemistry);
            result.push(TreacheryCardType::PoisonTooth);
            result.push(TreacheryCardType::WeirdingWay);
        }

        if g.applicable(Rule::ExpansionTreacheryCardsPBandSs) {
            result.push(TreacheryCardType::ProjectileAndPoison);
            result.push(TreacheryCardType::ShieldAndAntidote);
        }

        if g.is_playing(Faction::Richese) {
            result.push(TreacheryCardType::Rockmelter);
            result.push(TreacheryCardType::MirrorWeapon);
            result.push(TreacheryCardType::PortableAntidote);
        }

        if !g.applicable(Rule::BeneGesseritVoiceMustNameSpecialCards) {
            result.push(TreacheryCardType::PoisonDefense);
            result.push(TreacheryCardType::ProjectileDefense);
        }

        result
    }

    pub fn may_use_voice(g: &Game, p: PlayerIndex) -> bool {
        Self::may_use_voice_opt(g, Some(p))
    }

    /// `MayUseVoice` with a possibly null `p` (`Validate` passes `Player`, hazards 1.8):
    /// a null player throws NullReferenceException at the first dereference, `IsInvolved(p)`.
    fn may_use_voice_opt(g: &Game, p: Option<PlayerIndex>) -> bool {
        let p = p.and_then(crate::compat::cs::CsNull::opt);
        if g.current_voice.is_none() {
            if let Some(battle) = g.current_battle.as_ref() {
                let p = p.nre();
                if battle.is_involved(g, p) {
                    if g.player(p).nexus == Faction::BeneGesserit && NexusPlayed::can_use_secret_ally(g, p) {
                        return battle.is_aggressor_or_defender(g, p);
                    }

                    // IsOccupiedByFactionOrTheirAlly(World.BeneGesserit, OpponentOf(p)): the Player overload,
                    // which dereferences a null opponent only when there is an occupier.
                    let occupied = || match battle.opponent_of(g, p) {
                        Some(opponent) => g.is_occupied_by_faction_or_their_ally(World::BeneGesserit, opponent),
                        None => {
                            if g.occupier_of(World::BeneGesserit).is_some() {
                                panic!("A required value is missing")
                            }
                            false
                        }
                    };

                    if !g.prevented(FactionAdvantage::BeneGesseritUsingVoice) && !occupied() {
                        if g.player(p).faction == Faction::BeneGesserit {
                            return battle.is_involved(g, p);
                        }
                        if g.player(p).ally == Faction::BeneGesserit && g.bene_gesserit_allows_use_of_voice {
                            return battle.is_aggressor_or_defender(g, p);
                        }
                    }
                }
            }
        }

        false
    }

    pub fn is_voiced_by(g: &Game, as_weapon: bool, must: bool, card_type: TreacheryCardType, voiced_type: TreacheryCardType) -> bool {
        if card_type == voiced_type {
            return true;
        }

        if !g.applicable(Rule::BeneGesseritVoiceMustNameSpecialCards) {
            match voiced_type {
                TreacheryCardType::PoisonDefense => {
                    return card_type == TreacheryCardType::Antidote
                        || card_type == TreacheryCardType::PortableAntidote
                        || (!as_weapon && card_type == TreacheryCardType::Chemistry)
                        || card_type == TreacheryCardType::ShieldAndAntidote;
                }
                TreacheryCardType::Poison => {
                    return card_type == TreacheryCardType::PoisonTooth
                        || card_type == TreacheryCardType::ProjectileAndPoison
                        || (!must && as_weapon && card_type == TreacheryCardType::Chemistry);
                }
                TreacheryCardType::Shield => {
                    return card_type == TreacheryCardType::ShieldAndAntidote;
                }
                TreacheryCardType::ProjectileDefense => {
                    return card_type == TreacheryCardType::Shield
                        || card_type == TreacheryCardType::ShieldAndAntidote
                        || (!must && !as_weapon && card_type == TreacheryCardType::WeirdingWay);
                }
                TreacheryCardType::Projectile => {
                    return (as_weapon && card_type == TreacheryCardType::WeirdingWay) || card_type == TreacheryCardType::ProjectileAndPoison;
                }
                _ => {}
            }
        } else {
            match voiced_type {
                TreacheryCardType::PoisonDefense => return card_type == TreacheryCardType::Antidote,
                TreacheryCardType::ProjectileDefense => return card_type == TreacheryCardType::Shield,
                _ => {}
            }
        }

        false
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.current_voice = Some(self.clone());

        if !self.is_playing(game, Faction::BeneGesserit) {
            game.play_nexus_card(self.player(), elems!["Secret Ally", " to use Voice"]);
        }

        if let Some(battle) = game.current_battle.clone() {
            let opponent = battle.opponent_of(game, self.player());

            if let Some(opponent) = opponent {
                let f = game.player(opponent).faction;
                game.revoke_plan_if_needed(f);
            }
        }

        self.log(game);
        game.stone(Milestone::Voice);
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.must {
            return express!(self.initiator(), " use Voice to force the use of ", self.type_);
        }
        express!(self.initiator(), " use Voice to deny the use of ", self.type_)
    }
}

impl GameEventMethods for Voice {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::Voice
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Voice(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Voice::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        Voice::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Voice::get_message(self, game)
    }

}
