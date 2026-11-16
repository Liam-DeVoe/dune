use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Voice {
    pub must: bool,
    pub type_: Option<TreacheryCardType>,
}

/// How a player gets to use Voice: openly, as the Bene Gesserit or their ally, or as a secret
/// ally through the Bene Gesserit Nexus card.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VoiceRight {
    Open,
    SecretAlly,
}

impl Voice {
    pub fn may_not(&self) -> bool {
        !self.must
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

    pub fn may_use_voice(g: &Game, p: PlayerId) -> bool {
        Self::may_use_voice_opt(g, Some(p))
    }

    /// [`may_use_voice`](Self::may_use_voice) for a player that may be missing (a bot asks about
    /// its opponent in a battle it may not fight). A missing player panics where the answer
    /// depends on it, after the checks that don't.
    ///
    /// Voice comes before Prescience (rulebook FAQ), so it is closed once declined. A secret ally
    /// may also not use it after Prescience; an open one keeps Prescience waiting instead (see
    /// [`voice_pending`](Self::voice_pending)).
    pub fn may_use_voice_opt(g: &Game, p: Option<PlayerId>) -> bool {
        if g.battle.current_voice.is_some() || g.battle.voice_declined {
            return false;
        }

        match Self::voice_right(g, p) {
            Some(VoiceRight::Open) => true,
            Some(VoiceRight::SecretAlly) => g.battle.current_prescience.is_none(),
            None => false,
        }
    }

    /// Whether `p` could use Voice in the current battle if nobody had used or declined it yet,
    /// and on what grounds. The same panics as [`may_use_voice_opt`](Self::may_use_voice_opt).
    pub fn voice_right(g: &Game, p: Option<PlayerId>) -> Option<VoiceRight> {
        let battle = g.battle.current_battle.as_ref()?;
        let p = p.expect("player");
        if !battle.is_involved(g, p) {
            return None;
        }

        if g.player(p).nexus == Some(Faction::BeneGesserit) && NexusPlayed::can_use_secret_ally(g, p) {
            return battle.is_aggressor_or_defender(g, p).then_some(VoiceRight::SecretAlly);
        }

        // IsOccupiedByFactionOrTheirAlly(World.BeneGesserit, OpponentOf(p)): the Player overload,
        // which dereferences a null opponent only when there is an occupier.
        let occupied = || match battle.opponent_of(g, p) {
            Some(opponent) => g.is_occupied_by_player_or_their_ally(World::BeneGesserit, opponent),
            None => {
                if g.occupier_of(World::BeneGesserit).is_some() {
                    panic!("A required value is missing")
                }
                false
            }
        };

        if !g.prevented(FactionAdvantage::BeneGesseritUsingVoice) && !occupied() {
            if g.player(p).faction == Some(Faction::BeneGesserit) {
                return Some(VoiceRight::Open);
            }
            if g.player(p).ally == Some(Faction::BeneGesserit) && g.alliances.bene_gesserit_allows_use_of_voice && battle.is_aggressor_or_defender(g, p) {
                return Some(VoiceRight::Open);
            }
        }

        None
    }

    /// Whether Voice may still come from a player everyone knows could use it: nobody has used or
    /// declined it, and the Bene Gesserit (or their ally, when allowed) could. Prescience waits
    /// while this holds. A secret ally doesn't count, as waiting for them would reveal their
    /// Nexus card.
    pub fn voice_pending(g: &Game) -> bool {
        g.battle.current_battle.is_some()
            && g.battle.current_voice.is_none()
            && !g.battle.voice_declined
            && g.player_indices().any(|q| Self::voice_right(g, Some(q)) == Some(VoiceRight::Open))
    }

    /// Whether someone would use Prescience now if Voice weren't pending: then the player who
    /// may use Voice is asked to use or decline it.
    pub fn prescience_waits_for_voice(g: &Game) -> bool {
        Self::voice_pending(g) && g.player_indices().any(|q| Prescience::prescience_right(g, q))
    }

    pub fn is_voiced_by(g: &Game, as_weapon: bool, must: bool, card_type: TreacheryCardType, voiced_type: Option<TreacheryCardType>) -> bool {
        // A Voice without a type names no card.
        let Some(voiced_type) = voiced_type else { return false };
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
}

impl EventRules for Voice {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !Self::may_use_voice_opt(game, by.player) {
            return Err(Rejection(express!("You cannot use Voice")));
        }
        if !self.type_.is_some_and(|t| Self::valid_types(game).contains(&t)) {
            return Err(Rejection(express!("Invalid use of Voice")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        game.battle.current_voice = Some(VoiceUsed { submitter: by, must: self.must, type_: self.type_ });

        if !game.is_playing(Faction::BeneGesserit) {
            game.play_nexus_card(by.player(), parts!["use Voice"]);
        }

        if let Some(battle) = game.battle.current_battle.clone() {
            let opponent = battle.opponent_of(game, by.player());

            if let Some(opponent) = opponent {
                let f = game.player(opponent).faction;
                game.revoke_plan_if_needed(f.required("faction")?);
            }
        }

        let m = self.describe(game, by);
        game.log_event(m);
        game.stone(Milestone::Voice);
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.must {
            return express!(by.faction, " use Voice to force the use of ", self.type_);
        }
        express!(by.faction, " use Voice to deny the use of ", self.type_)
    }
}
