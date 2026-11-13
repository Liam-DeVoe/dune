use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct Karma {
    pub prevented: FactionAdvantage,
    #[canon(name = "_cardId")]
    pub card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
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
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.card_id = TreacheryCardManager::get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if !Karma::valid_faction_advantages(game, self.player()).contains(&self.prevented) {
            return Some(express!("You cannot prevent ", self.prevented, " at this moment"));
        }

        None
    }

    pub fn valid_faction_advantages(g: &Game, p: PlayerIndex) -> Vec<FactionAdvantage> {
        let mut result = vec![FactionAdvantage::None];
        let faction = g.player(p).faction;

        if faction != Faction::Atreides && g.is_playing(Faction::Atreides) {
            if Karma::in_(g, MainPhase::Bidding) {
                result.push(FactionAdvantage::AtreidesBiddingPrescience);
            }
            if Karma::in_(g, MainPhase::Resurrection) || Karma::in_(g, MainPhase::ShipmentAndMove) {
                result.push(FactionAdvantage::AtreidesSpiceBlowPrescience);
            }

            if Karma::in_(g, MainPhase::Battle) {
                if g.applicable(Rule::AtreidesMessiah) {
                    result.push(FactionAdvantage::AtreidesUseMessiah);
                }
                result.push(FactionAdvantage::AtreidesBattlePlanPrescience);
            }
        }

        if faction != Faction::Harkonnen && g.is_playing(Faction::Harkonnen) {
            if Karma::in_(g, MainPhase::Bidding) {
                result.push(FactionAdvantage::HarkonnenFreeCard);
            }

            if Karma::in_(g, MainPhase::Battle) {
                if g.applicable(Rule::HarkonnenCapturesOrKillsLeaders) {
                    result.push(FactionAdvantage::HarkonnenCaptureLeader);
                }

                result.push(FactionAdvantage::HarkonnenCallTraitorForAlly);
            }
        }

        if faction != Faction::Fremen && g.is_playing(Faction::Fremen) {
            if Karma::in_(g, MainPhase::Storm) {
                if g.applicable(Rule::FremenDeterminesStorm) {
                    result.push(FactionAdvantage::FremenStormPrescience);
                }
                if g.applicable(Rule::FremenStormLosses) {
                    result.push(FactionAdvantage::FremenProtectedFromStorm);
                }
            }

            if Karma::in_(g, MainPhase::Blow) {
                result.push(FactionAdvantage::FremenControlsMonster);
                result.push(FactionAdvantage::FremenProtectedFromMonster);
            }

            if Karma::in_(g, MainPhase::ShipmentAndMove) {
                result.push(FactionAdvantage::FremenExtraMove);
            }

            if Karma::in_(g, MainPhase::Battle) {
                if g.applicable(Rule::FremenSpecialForces) {
                    result.push(FactionAdvantage::FremenSpecialForceBonus);
                }
                if g.applicable(Rule::AdvancedCombat) {
                    result.push(FactionAdvantage::FremenNotPayingForBattles);
                }
            }
        }

        if faction != Faction::Emperor && g.is_playing(Faction::Emperor) {
            result.push(FactionAdvantage::EmperorGiveSpiceToAlly);

            if Karma::in_(g, MainPhase::Bidding) {
                result.push(FactionAdvantage::EmperorReceiveBid);
            }

            if Karma::in_(g, MainPhase::Resurrection) {
                result.push(FactionAdvantage::EmperorLetAllyReviveExtraForces);
            }

            if Karma::in_(g, MainPhase::Battle) && g.applicable(Rule::EmperorSpecialForces) {
                result.push(FactionAdvantage::EmperorSpecialForceBonus);
            }
        }

        if faction != Faction::Guild && g.is_playing(Faction::Guild) {
            if Karma::in_(g, MainPhase::ShipmentAndMove) {
                let guild_determine_shipment_may_still_be_prevented =
                    !g.has_acted_or_passed.contains(&Faction::Guild) && g.current_phase != Phase::GuildShip;
                if g.applicable(Rule::GuildDetermineShipment) && guild_determine_shipment_may_still_be_prevented {
                    result.push(FactionAdvantage::GuildDetermineMoveMoment);
                }
                result.push(FactionAdvantage::GuildSpecialShipments);
                result.push(FactionAdvantage::GuildShipmentsDiscount);
                result.push(FactionAdvantage::GuildShipmentsDiscountAlly);
                result.push(FactionAdvantage::GuildReceiveShipment);
            }
        }

        if faction != Faction::BeneGesserit && g.is_playing(Faction::BeneGesserit) {
            if Karma::in_(g, MainPhase::Charity) {
                if g.applicable(Rule::BeneGesseritAutoCharity) {
                    result.push(FactionAdvantage::BeneGesseritCharity);
                }
            }

            if g.applicable(Rule::BeneGesseritWorthlessAsKarma) {
                result.push(FactionAdvantage::BeneGesseritWorthlessAsKarma);
            }
            if g.applicable(Rule::BeneGesseritAdvisors) {
                result.push(FactionAdvantage::BeneGesseritAnnouncesBattle);
            }

            if Karma::in_(g, MainPhase::ShipmentAndMove) {
                if g.applicable(Rule::BeneGesseritAdvisors) {
                    result.push(FactionAdvantage::BeneGesseritIntrusion);
                }
                result.push(FactionAdvantage::BeneGesseritAccompanies);
            }

            if Karma::in_(g, MainPhase::Battle) {
                result.push(FactionAdvantage::BeneGesseritUsingVoice);
            }
        }

        if faction != Faction::Ixian && g.is_playing(Faction::Ixian) {
            if Karma::in_(g, MainPhase::Storm) {
                result.push(FactionAdvantage::IxianMovingHms);
            }

            if Karma::in_(g, MainPhase::Bidding) {
                result.push(FactionAdvantage::IxianSelectingCardsOnAuction);
                result.push(FactionAdvantage::IxianAllyDiscardingCard);
                if g.applicable(Rule::IxianSwappingCardOnBid) {
                    result.push(FactionAdvantage::IxianSwappingCard);
                }
            }

            if Karma::in_(g, MainPhase::ShipmentAndMove) {
                result.push(FactionAdvantage::IxianCyborgExtraMove);
            }

            if Karma::in_(g, MainPhase::Battle) {
                result.push(FactionAdvantage::IxianSpecialForceBonus);
                result.push(FactionAdvantage::IxianReplacingSpecialForces);
            }
        }

        if faction != Faction::Tleilaxu && g.is_playing(Faction::Tleilaxu) {
            if Karma::in_(g, MainPhase::Bidding) || Karma::in_(g, MainPhase::Resurrection) {
                result.push(FactionAdvantage::TleilaxuIncreasingRevivalLimits);
            }

            if Karma::in_(g, MainPhase::Resurrection) {
                result.push(FactionAdvantage::TleilaxuRevivalDiscount);
                result.push(FactionAdvantage::TleilaxuRevivalDiscountAlly);
                result.push(FactionAdvantage::TleilaxuReceiveRevive);
                result.push(FactionAdvantage::TleilaxuEarlyLeaderRevive);
                if g.applicable(Rule::TleilaxuGholas) {
                    result.push(FactionAdvantage::TleilaxuReviveGhola);
                }
            }

            if Karma::in_(g, MainPhase::Contemplate) {
                result.push(FactionAdvantage::TleilaxuReplacingFaceDancer);
            }
        }

        if faction != Faction::Choam && g.is_playing(Faction::Choam) {
            if Karma::in_(g, MainPhase::Charity) {
                result.push(FactionAdvantage::ChoamControllingCharity);
            }

            result.push(FactionAdvantage::ChoamDiscarding);

            if Karma::in_(g, MainPhase::Resurrection) {
                result.push(FactionAdvantage::ChoamRevival);
            }

            if Karma::in_(g, MainPhase::Battle) {
                result.push(FactionAdvantage::ChoamReceiveForcePayment);
                if g.applicable(Rule::ChoamAuditor) {
                    result.push(FactionAdvantage::ChoamAudit);
                }
            }

            if Karma::in_(g, MainPhase::Contemplate) {
                result.push(FactionAdvantage::ChoamEconomics);
            }
        }

        if faction != Faction::Richese && g.is_playing(Faction::Richese) {
            if Karma::in_(g, MainPhase::Bidding) {
                result.push(FactionAdvantage::RicheseAuction);
                if g.applicable(Rule::RicheseBlackMarket) {
                    result.push(FactionAdvantage::RicheseBlackMarket);
                }
            }

            if Karma::in_(g, MainPhase::ShipmentAndMove) {
                result.push(FactionAdvantage::RicheseNofield);
            }
        }

        if faction != Faction::Ecaz && g.is_playing(Faction::Ecaz) {
            if Karma::in_(g, MainPhase::Resurrection) {
                result.push(FactionAdvantage::EcazAmbassadors);
            }
            if Karma::in_(g, MainPhase::Battle) {
                result.push(FactionAdvantage::EcazOccupation);
            }
            if Karma::in_(g, MainPhase::Collection) {
                result.push(FactionAdvantage::EcazCollection);
            }
        }

        if faction != Faction::Moritani && g.is_playing(Faction::Moritani) {
            result.push(FactionAdvantage::MoritaniEnemyOfEnemy);
            if Karma::in_(g, MainPhase::ShipmentAndMove) {
                result.push(FactionAdvantage::MoritaniGainingVidal);
            }
            if Karma::in_(g, MainPhase::Contemplate) {
                result.push(FactionAdvantage::MoritaniPlantingTerror);
            }
        }

        result
    }

    pub fn in_(g: &Game, phase: MainPhase) -> bool {
        g.current_main_phase == phase
    }

    /// Lazy like the C# `Where`: callers stop at the first match (lazy-queries.md).
    pub fn valid_karma_cards(g: &Game, p: PlayerIndex) -> impl Iterator<Item = TreacheryCard> {
        let pl = g.player(p);
        pl.treachery_cards
            .iter()
            .copied()
            .filter(move |c| {
                c.type_() == TreacheryCardType::Karma
                    || (c.type_() == TreacheryCardType::Useless && pl.is(Faction::BeneGesserit) && g.applicable(Rule::BeneGesseritWorthlessAsKarma))
                    || (c.type_() == TreacheryCardType::Clairvoyance && pl.occupies(g, Some(g.map.shrine().location())))
            })
    }

    pub fn can_karma(g: &Game, p: PlayerIndex) -> bool {
        // Any() stops at the first match.
        Karma::valid_karma_cards(g, p).next().is_some()
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        game.discard_treachery_card_boolean(self.card(), false);
        let effect: Vec<ExpressionElement> = if self.prevented == FactionAdvantage::None {
            vec![]
        } else {
            vec!["prevent ".into(), self.prevented.into()]
        };
        let m = Game::karma_played(self.initiator(), self.card(), effect);
        game.log(vec![m.into()]);
        game.stone(Milestone::Karma);

        if self.prevented != FactionAdvantage::None {
            game.prevent(self.initiator(), self.prevented, false);
        }

        self.revoke_battle_plans_if_needed(game);

        if self.prevented == FactionAdvantage::BeneGesseritUsingVoice {
            game.current_voice = None;
        }

        if self.prevented == FactionAdvantage::AtreidesBattlePlanPrescience {
            game.current_prescience = None;
        }

        let me = self.player();
        if game.player(me).is(Faction::Ixian)
            && game.current_phase == Phase::IxianSwappingCard
            && game.player(me).treachery_cards.len() == 0
        {
            game.continue_after_ixian_card_swap_decision();
        }

        if self.prevented == FactionAdvantage::FremenRidesMonster
            && matches!(game.current_phase, Phase::FremenRidingMonsterA | Phase::FremenRidingMonsterB)
        {
            game.determine_next_shipment_and_move_sub_phase();
        }
    }

    pub fn revoke_battle_plans_if_needed(&self, game: &mut Game) {
        if game.current_main_phase == MainPhase::Battle {
            if self.prevented == FactionAdvantage::FremenNotPayingForBattles
                || self.prevented == FactionAdvantage::FremenSpecialForceBonus
            {
                game.revoke_plan_if_needed(Faction::Fremen);
            }

            if self.prevented == FactionAdvantage::EmperorSpecialForceBonus {
                game.revoke_plan_if_needed(Faction::Emperor);
            }

            if self.prevented == FactionAdvantage::IxianSpecialForceBonus {
                game.revoke_plan_if_needed(Faction::Ixian);
            }

            if self.prevented == FactionAdvantage::AtreidesUseMessiah {
                game.revoke_plan_if_needed(Faction::Atreides);
            }
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        // C# parity: the doubled space in "play  a" (hazards 11.x) comes from the empty ExpressIf part.
        let card = self.card();
        express!(
            self.initiator(),
            " play ",
            express_part_if!(card.nre().type_() != TreacheryCardType::Karma; card, " as "),
            " a ",
            TreacheryCardType::Karma,
            " card"
        )
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

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) {
        Karma::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Karma::get_message(self, game)
    }

}
