use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Karma {
    pub prevented: Option<FactionAdvantage>,
    pub card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for Karma {
    fn default() -> Self {
        Karma {
            prevented: Default::default(),
            card_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl Karma {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::get(self.card_id)
    }

    pub fn set_card(&mut self, value: Option<TreacheryCard>) {
        self.card_id = TreacheryCardManager::get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !Karma::valid_faction_advantages(game, self.player()).contains(&self.prevented) {
            return Some(express!("You cannot prevent ", self.prevented, " at this moment"));
        }

        None
    }

    pub fn valid_faction_advantages(g: &Game, p: PlayerIndex) -> Vec<Option<FactionAdvantage>> {
        let mut result = vec![None];
        let faction = g.player(p).faction;

        if faction != Faction::Atreides && g.is_playing(Faction::Atreides) {
            if Karma::in_(g, MainPhase::Bidding) {
                result.push(Some(FactionAdvantage::AtreidesBiddingPrescience));
            }
            if Karma::in_(g, MainPhase::Resurrection) || Karma::in_(g, MainPhase::ShipmentAndMove) {
                result.push(Some(FactionAdvantage::AtreidesSpiceBlowPrescience));
            }

            if Karma::in_(g, MainPhase::Battle) {
                if g.applicable(Rule::AtreidesMessiah) {
                    result.push(Some(FactionAdvantage::AtreidesUseMessiah));
                }
                result.push(Some(FactionAdvantage::AtreidesBattlePlanPrescience));
            }
        }

        if faction != Faction::Harkonnen && g.is_playing(Faction::Harkonnen) {
            if Karma::in_(g, MainPhase::Bidding) {
                result.push(Some(FactionAdvantage::HarkonnenFreeCard));
            }

            if Karma::in_(g, MainPhase::Battle) {
                if g.applicable(Rule::HarkonnenCapturesOrKillsLeaders) {
                    result.push(Some(FactionAdvantage::HarkonnenCaptureLeader));
                }

                result.push(Some(FactionAdvantage::HarkonnenCallTraitorForAlly));
            }
        }

        if faction != Faction::Fremen && g.is_playing(Faction::Fremen) {
            if Karma::in_(g, MainPhase::Storm) {
                if g.applicable(Rule::FremenDeterminesStorm) {
                    result.push(Some(FactionAdvantage::FremenStormPrescience));
                }
                if g.applicable(Rule::FremenStormLosses) {
                    result.push(Some(FactionAdvantage::FremenProtectedFromStorm));
                }
            }

            if Karma::in_(g, MainPhase::Blow) {
                result.push(Some(FactionAdvantage::FremenControlsMonster));
                result.push(Some(FactionAdvantage::FremenProtectedFromMonster));
            }

            if Karma::in_(g, MainPhase::ShipmentAndMove) {
                result.push(Some(FactionAdvantage::FremenExtraMove));
            }

            if Karma::in_(g, MainPhase::Battle) {
                if g.applicable(Rule::FremenSpecialForces) {
                    result.push(Some(FactionAdvantage::FremenSpecialForceBonus));
                }
                if g.applicable(Rule::AdvancedCombat) {
                    result.push(Some(FactionAdvantage::FremenNotPayingForBattles));
                }
            }
        }

        if faction != Faction::Emperor && g.is_playing(Faction::Emperor) {
            result.push(Some(FactionAdvantage::EmperorGiveSpiceToAlly));

            if Karma::in_(g, MainPhase::Bidding) {
                result.push(Some(FactionAdvantage::EmperorReceiveBid));
            }

            if Karma::in_(g, MainPhase::Resurrection) {
                result.push(Some(FactionAdvantage::EmperorLetAllyReviveExtraForces));
            }

            if Karma::in_(g, MainPhase::Battle) && g.applicable(Rule::EmperorSpecialForces) {
                result.push(Some(FactionAdvantage::EmperorSpecialForceBonus));
            }
        }

        if faction != Faction::Guild && g.is_playing(Faction::Guild) {
            if Karma::in_(g, MainPhase::ShipmentAndMove) {
                let guild_determine_shipment_may_still_be_prevented =
                    !g.turn.has_acted_or_passed.contains(&Faction::Guild) && g.turn.current_phase != Phase::GuildShip;
                if g.applicable(Rule::GuildDetermineShipment) && guild_determine_shipment_may_still_be_prevented {
                    result.push(Some(FactionAdvantage::GuildDetermineMoveMoment));
                }
                result.push(Some(FactionAdvantage::GuildSpecialShipments));
                result.push(Some(FactionAdvantage::GuildShipmentsDiscount));
                result.push(Some(FactionAdvantage::GuildShipmentsDiscountAlly));
                result.push(Some(FactionAdvantage::GuildReceiveShipment));
            }
        }

        if faction != Faction::BeneGesserit && g.is_playing(Faction::BeneGesserit) {
            if Karma::in_(g, MainPhase::Charity) {
                if g.applicable(Rule::BeneGesseritAutoCharity) {
                    result.push(Some(FactionAdvantage::BeneGesseritCharity));
                }
            }

            if g.applicable(Rule::BeneGesseritWorthlessAsKarma) {
                result.push(Some(FactionAdvantage::BeneGesseritWorthlessAsKarma));
            }
            if g.applicable(Rule::BeneGesseritAdvisors) {
                result.push(Some(FactionAdvantage::BeneGesseritAnnouncesBattle));
            }

            if Karma::in_(g, MainPhase::ShipmentAndMove) {
                if g.applicable(Rule::BeneGesseritAdvisors) {
                    result.push(Some(FactionAdvantage::BeneGesseritIntrusion));
                }
                result.push(Some(FactionAdvantage::BeneGesseritAccompanies));
            }

            if Karma::in_(g, MainPhase::Battle) {
                result.push(Some(FactionAdvantage::BeneGesseritUsingVoice));
            }
        }

        if faction != Faction::Ixian && g.is_playing(Faction::Ixian) {
            if Karma::in_(g, MainPhase::Storm) {
                result.push(Some(FactionAdvantage::IxianMovingHms));
            }

            if Karma::in_(g, MainPhase::Bidding) {
                result.push(Some(FactionAdvantage::IxianSelectingCardsOnAuction));
                result.push(Some(FactionAdvantage::IxianAllyDiscardingCard));
                if g.applicable(Rule::IxianSwappingCardOnBid) {
                    result.push(Some(FactionAdvantage::IxianSwappingCard));
                }
            }

            if Karma::in_(g, MainPhase::ShipmentAndMove) {
                result.push(Some(FactionAdvantage::IxianCyborgExtraMove));
            }

            if Karma::in_(g, MainPhase::Battle) {
                result.push(Some(FactionAdvantage::IxianSpecialForceBonus));
                result.push(Some(FactionAdvantage::IxianReplacingSpecialForces));
            }
        }

        if faction != Faction::Tleilaxu && g.is_playing(Faction::Tleilaxu) {
            if Karma::in_(g, MainPhase::Bidding) || Karma::in_(g, MainPhase::Resurrection) {
                result.push(Some(FactionAdvantage::TleilaxuIncreasingRevivalLimits));
            }

            if Karma::in_(g, MainPhase::Resurrection) {
                result.push(Some(FactionAdvantage::TleilaxuRevivalDiscount));
                result.push(Some(FactionAdvantage::TleilaxuRevivalDiscountAlly));
                result.push(Some(FactionAdvantage::TleilaxuReceiveRevive));
                result.push(Some(FactionAdvantage::TleilaxuEarlyLeaderRevive));
                if g.applicable(Rule::TleilaxuGholas) {
                    result.push(Some(FactionAdvantage::TleilaxuReviveGhola));
                }
            }

            if Karma::in_(g, MainPhase::Contemplate) {
                result.push(Some(FactionAdvantage::TleilaxuReplacingFaceDancer));
            }
        }

        if faction != Faction::Choam && g.is_playing(Faction::Choam) {
            if Karma::in_(g, MainPhase::Charity) {
                result.push(Some(FactionAdvantage::ChoamControllingCharity));
            }

            result.push(Some(FactionAdvantage::ChoamDiscarding));

            if Karma::in_(g, MainPhase::Resurrection) {
                result.push(Some(FactionAdvantage::ChoamRevival));
            }

            if Karma::in_(g, MainPhase::Battle) {
                result.push(Some(FactionAdvantage::ChoamReceiveForcePayment));
                if g.applicable(Rule::ChoamAuditor) {
                    result.push(Some(FactionAdvantage::ChoamAudit));
                }
            }

            if Karma::in_(g, MainPhase::Contemplate) {
                result.push(Some(FactionAdvantage::ChoamEconomics));
            }
        }

        if faction != Faction::Richese && g.is_playing(Faction::Richese) {
            if Karma::in_(g, MainPhase::Bidding) {
                result.push(Some(FactionAdvantage::RicheseAuction));
                if g.applicable(Rule::RicheseBlackMarket) {
                    result.push(Some(FactionAdvantage::RicheseBlackMarket));
                }
            }

            if Karma::in_(g, MainPhase::ShipmentAndMove) {
                result.push(Some(FactionAdvantage::RicheseNofield));
            }
        }

        if faction != Faction::Ecaz && g.is_playing(Faction::Ecaz) {
            if Karma::in_(g, MainPhase::Resurrection) {
                result.push(Some(FactionAdvantage::EcazAmbassadors));
            }
            if Karma::in_(g, MainPhase::Battle) {
                result.push(Some(FactionAdvantage::EcazOccupation));
            }
            if Karma::in_(g, MainPhase::Collection) {
                result.push(Some(FactionAdvantage::EcazCollection));
            }
        }

        if faction != Faction::Moritani && g.is_playing(Faction::Moritani) {
            result.push(Some(FactionAdvantage::MoritaniEnemyOfEnemy));
            if Karma::in_(g, MainPhase::ShipmentAndMove) {
                result.push(Some(FactionAdvantage::MoritaniGainingVidal));
            }
            if Karma::in_(g, MainPhase::Contemplate) {
                result.push(Some(FactionAdvantage::MoritaniPlantingTerror));
            }
        }

        result
    }

    pub fn in_(g: &Game, phase: MainPhase) -> bool {
        g.turn.current_main_phase == phase
    }

    /// Lazy: callers stop at the first match.
    pub fn valid_karma_cards(g: &Game, p: PlayerIndex) -> impl Iterator<Item = TreacheryCard> {
        let pl = g.player(p);
        pl.treachery_cards
            .iter()
            .copied()
            .filter(move |c| {
                c.type_() == TreacheryCardType::Karma
                    || (c.type_() == TreacheryCardType::Useless && pl.is(Faction::BeneGesserit) && g.applicable(Rule::BeneGesseritWorthlessAsKarma))
                    || (c.type_() == TreacheryCardType::Clairvoyance && pl.occupies(g, Some(g.board.map.shrine().location())))
            })
    }

    pub fn can_karma(g: &Game, p: PlayerIndex) -> bool {
        // Any() stops at the first match.
        Karma::valid_karma_cards(g, p).next().is_some()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.discard_card(self.card(), false)?;
        let effect: Vec<ExpressionElement> = if self.prevented == None {
            vec![]
        } else {
            vec!["prevent ".into(), self.prevented.into()]
        };
        let m = Game::karma_played(self.initiator(), self.card(), effect);
        game.log(vec![m.into()]);
        game.stone(Milestone::Karma);

        if let Some(prevented) = self.prevented {
            game.prevent(self.initiator(), prevented, false);
        }

        self.revoke_battle_plans_if_needed(game);

        if self.prevented == Some(FactionAdvantage::BeneGesseritUsingVoice) {
            game.battle.current_voice = None;
        }

        if self.prevented == Some(FactionAdvantage::AtreidesBattlePlanPrescience) {
            game.battle.current_prescience = None;
        }

        let me = self.player();
        if game.player(me).is(Faction::Ixian)
            && game.turn.current_phase == Phase::IxianSwappingCard
            && game.player(me).treachery_cards.len() == 0
        {
            game.continue_after_ixian_card_swap_decision();
        }

        if self.prevented == Some(FactionAdvantage::FremenRidesMonster)
            && matches!(game.turn.current_phase, Phase::FremenRidingMonsterA | Phase::FremenRidingMonsterB)
        {
            game.determine_next_shipment_and_move_sub_phase()?;
        }
        Ok(())
    }

    pub fn revoke_battle_plans_if_needed(&self, game: &mut Game) {
        if game.turn.current_main_phase == MainPhase::Battle {
            if self.prevented == Some(FactionAdvantage::FremenNotPayingForBattles)
                || self.prevented == Some(FactionAdvantage::FremenSpecialForceBonus)
            {
                game.revoke_plan_if_needed(Faction::Fremen);
            }

            if self.prevented == Some(FactionAdvantage::EmperorSpecialForceBonus) {
                game.revoke_plan_if_needed(Faction::Emperor);
            }

            if self.prevented == Some(FactionAdvantage::IxianSpecialForceBonus) {
                game.revoke_plan_if_needed(Faction::Ixian);
            }

            if self.prevented == Some(FactionAdvantage::AtreidesUseMessiah) {
                game.revoke_plan_if_needed(Faction::Atreides);
            }
        }
    }

    pub fn try_get_message(&self, _game: &Game) -> GameResult<Message> {
        let card = self.card().required("card")?;
        Ok(express!(
            self.initiator(),
            " play ",
            express_part_if!(card.type_() != TreacheryCardType::Karma; card, " as "),
            "a ",
            TreacheryCardType::Karma,
            " card"
        ))
    }

    pub fn get_message(&self, game: &Game) -> Message {
        self.try_get_message(game).expect("the message of an applicable Karma event")
    }
}

impl GameEventMethods for Karma {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::Karma
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Karma(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Karma::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        Karma::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Karma::get_message(self, game)
    }

    fn try_get_message(&self, game: &Game) -> GameResult<Message> {
        Karma::try_get_message(self, game)
    }

}
