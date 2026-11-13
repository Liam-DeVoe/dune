use crate::prelude::*;
use crate::support::{cast_hero, unbox_enum, unbox_float, unbox_int};
use std::cmp::Reverse;
use treachery_engine::events::PlacementEventMethods;

/// Total forces in `locations` that belong to a faction `counted` accepts.
fn forces_in(game: &Game, locations: &[Location], counted: impl Fn(Faction) -> bool) -> i32 {
    locations
        .iter()
        .map(|l| game.battalions_in(*l).iter().filter(|b| counted(b.faction)).map(|b| b.total_amount_of_forces()).sum::<i32>())
        .sum()
}

fn yes_no(value: bool) -> ClairVoyanceAnswer {
    if value { ClairVoyanceAnswer::Yes } else { ClairVoyanceAnswer::No }
}

/// Whether a question about `covered_by` card types covers a card of type `type_to_check`.
fn covers(type_to_check: TreacheryCardType, covered_by: TreacheryCardType) -> bool {
    ClairVoyanceAnswered::is_questioned_by(true, type_to_check, covered_by) || ClairVoyanceAnswered::is_questioned_by(false, type_to_check, covered_by)
}

/// Forces the storm would kill in total when it moves `moves` sectors.
#[derive(Clone, Copy)]
struct StormKills {
    moves: i32,
    mine: i32,
    enemy: i32,
}

impl ClassicBot {
    pub fn determine_resources_transferred(&self, game: &Game) -> Option<Rc<ResourcesTransferred>> {
        let amount = ResourcesTransferred::max_amount(game, self.player);
        if self.resources(game) - amount > 15 && game.player(self.player).allied_resources(game) < self.resources(game) {
            let mut e = ResourcesTransferred::new(game, self.faction(game));
            e.resources = amount;
            return Some(Rc::new(e));
        }

        None
    }

    pub fn determine_discovery_entered(&self, game: &Game) -> Rc<DiscoveryEntered> {
        let to = DiscoveryEntered::valid_targets(game, self.player).next();
        // Fails right away when there is no target.
        let me = game.player(self.player);
        let mut battalions_to_move: IndexMap<Location, Battalion> = IndexMap::new();
        for l in DiscoveryEntered::valid_sources(game, self.player, to.expect("destination")) {
            battalions_to_move.insert_new(l, me.battalion_in_location(l).expect("battalion in location"));
        }
        let mut e = DiscoveryEntered::new(game, self.faction(game));
        e.set_force_locations(game, &battalions_to_move);
        e.set_to(game, to.map(|d| d.location()));
        Rc::new(e)
    }

    pub fn determine_discovery_revealed(&self, game: &Game) -> Rc<DiscoveryRevealed> {
        let mut e = DiscoveryRevealed::new(game, self.faction(game));
        let location = DiscoveryRevealed::get_locations(game, self.player).next().expect("an element");
        e.set_location(game, Some(location));
        Rc::new(e)
    }

    pub fn determine_testing_station_used(&self, game: &Game) -> Option<Rc<TestingStationUsed>> {
        const N: i32 = Map::NUMBER_OF_SECTORS;
        let faction = self.faction(game);
        let ally = self.ally(game);
        let me = game.player(self.player);
        let storm_end = (game.sector_in_storm + game.next_storm_moves) % N;

        let holds_weakly = |p: &Player, l: Location| p.occupies(game, Some(l)) && p.any_forces_in(Some(l)) < 6;
        // What the storm passing over `sector` gains me: enemy losses and weakly held strongholds of mine
        // (worth 3 forces each) minus my own losses and enemy strongholds.
        let score = |sector: i32| {
            let locations: Vec<Location> =
                game.map.locations(false).filter(|l| l.sector(&game.map) == sector && !l.is_protected_from_storm()).collect();
            // Mine and my ally's (C# had `&&`, which no faction matches, so it always counted 0).
            let my_forces_killed = forces_in(game, &locations, |f| f == faction || (ally != Faction::None && f == ally));
            let my_strongholds = locations
                .iter()
                .filter(|l| {
                    l.is_stronghold()
                        && (holds_weakly(me, **l) || (me.has_ally() && holds_weakly(game.player(self.allied_player(game).expect("allied player")), **l)))
                })
                .count() as i32;
            let enemy_forces_killed = forces_in(game, &locations, |f| f != faction && f != ally);
            let enemy_strongholds = locations.iter().filter(|l| l.is_stronghold() && self.occupied_by_opponent(game, **l)).count() as i32;
            enemy_forces_killed + my_strongholds * 3 - my_forces_killed - enemy_strongholds * 3
        };

        let score_if_minus1 = score(if storm_end == 0 { N } else { storm_end - 1 });
        let score_if_passed = score_if_minus1 + score(storm_end);
        let score_if_plus1 = score_if_passed + score(if storm_end == N { 0 } else { storm_end + 1 });

        let value_added = if score_if_minus1 > score_if_passed && score_if_minus1 > score_if_plus1 {
            -1
        } else if score_if_plus1 > score_if_passed && score_if_plus1 > score_if_minus1 {
            1
        } else {
            return None;
        };
        let mut e = TestingStationUsed::new(game, faction);
        e.value_added = value_added;
        Some(Rc::new(e))
    }

    pub fn determine_divide_resources(&self, game: &Game) -> Rc<DivideResources> {
        let tbd = DivideResources::get_resources_to_be_divided(game);
        let spice_i_want = std::cmp::max(0, tbd.expect("resources to be divided").amount - self.resources(game));
        let mut e = DivideResources::new(game, self.faction(game));
        e.portion_to_first_player = spice_i_want;
        Rc::new(e)
    }

    pub fn determine_card_given(&self, game: &Game) -> Rc<CardGiven> {
        let mut e = CardGiven::new(game, self.faction(game));
        let card = game.card_that_must_be_kept_or_given_to_ally.expect("card that must be kept or given to ally");
        let ally = self.ally(game);
        e.passed = !(self.card_quality(game, card, Some(self.player)) <= 2
            && (ally == Faction::BeneGesserit || ally == Faction::Choam || self.card_quality(game, card, self.allied_player(game)) > 0));
        Rc::new(e)
    }

    pub fn determine_divide_resources_accepted(&self, game: &Game) -> Rc<DivideResourcesAccepted> {
        let tbd = DivideResources::get_resources_to_be_divided(game);

        let portion = game.current_division_proposal.as_ref().expect("current division proposal").portion_to_first_player;
        let i_get_when_agreed = DivideResources::gained_by_other_faction(tbd.as_ref(), true, portion);
        let i_get_when_denied = DivideResources::gained_by_other_faction(tbd.as_ref(), false, portion);

        let accepted = i_get_when_agreed >= i_get_when_denied || self.resources(game) >= game.player(self.player).allied_resources(game);

        let mut e = DivideResourcesAccepted::new(game, self.faction(game));
        e.passed = !accepted;
        Rc::new(e)
    }

    pub fn determine_discarded(&self, game: &Game) -> Rc<Discarded> {
        let Some(worst_card) = game.player(self.player).treachery_cards.iter().copied().min_by_key(|c| self.card_quality(game, *c, None)) else {
            panic!("Sequence contains no elements")
        };
        let mut e = Discarded::new(game, self.faction(game));
        e.set_card(Some(worst_card));
        Rc::new(e)
    }

    pub fn determine_traitor_discarded(&self, game: &Game) -> Rc<TraitorDiscarded> {
        let worst_traitor = self
            .determine_worst_traitor(game)
            .or_else(|| game.player(self.player).traitors.iter().copied().lowest_or_default(&self.random, |t| t.value()));
        let mut e = TraitorDiscarded::new(game, self.faction(game));
        e.set_traitor(worst_traitor);
        Rc::new(e)
    }

    pub fn determine_worst_traitor(&self, game: &Game) -> Option<Hero> {
        let me = game.player(self.player);
        let traitors = || me.traitors.iter().copied();
        let faction = self.faction(game);

        traitors()
            .filter(|t| me.revealed_traitors.contains(t))
            .highest_or_default(&self.random, |t| t.value())
            .or_else(|| {
                if self.ally(game) == Faction::Tleilaxu || !game.is_playing(Faction::Tleilaxu) {
                    traitors().filter(|t| t.faction() == faction).lowest_or_default(&self.random, |t| t.value())
                } else {
                    None
                }
            })
            .or_else(|| traitors().filter(|t| t.as_leader().is_some() && !game.is_alive(*t)).lowest_or_default(&self.random, |t| t.value()))
            .or_else(|| traitors().filter(|t| t.as_leader().is_some()).lowest_or_default(&self.random, |t| t.value()))
    }

    pub fn determine_nexus_played(&mut self, game: &Game) -> Option<Rc<NexusPlayed>> {
        let mut result = NexusPlayed::new(game, self.faction(game));
        result.faction = game.player(self.player).nexus;

        if NexusPlayed::can_use_cunning(game, self.player) {
            self.determine_nexus_played_cunning(game, result)
        } else if NexusPlayed::can_use_secret_ally(game, self.player) {
            self.determine_nexus_played_secret_ally(game, result)
        } else {
            self.determine_nexus_played_betrayal(game, result)
        }
    }

    /// Plays the nexus card held (`result.faction`) to betray that faction.
    pub fn determine_nexus_played_betrayal(&mut self, game: &Game, mut result: NexusPlayed) -> Option<Rc<NexusPlayed>> {
        if self.ally(game) == result.faction {
            return None;
        }

        let play = match game.player(self.player).nexus {
            Faction::Atreides => {
                self.is_winning_or_is_opponent_in_battle(game, Faction::Atreides) && !game.prevented(FactionAdvantage::AtreidesBattlePlanPrescience)
            }

            Faction::Harkonnen | Faction::Tleilaxu => game.current_battle.as_ref().expect("current battle").is_involved(game, self.player),

            Faction::Fremen => {
                (game.current_main_phase == MainPhase::Blow && FremenRidesMonster::is_applicable(game))
                    || (game.current_main_phase == MainPhase::ShipmentAndMove
                        && self
                            .almost_winning_opponents_i_wish_to_attack(game, 0, true)
                            .into_iter()
                            .any(|p| game.player(p).faction == Faction::Fremen))
            }

            Faction::Emperor => {
                let ally_bids_high = game.current_main_phase == MainPhase::Bidding
                    && game.current_bid.as_ref().is_some_and(|bid| {
                        game.player(bid.i_bid_player().expect("i bid player")).ally == Faction::Emperor
                            && bid.i_bid_total_amount() > 5
                            && !game.prevented(FactionAdvantage::EmperorReceiveBid)
                    });
                ally_bids_high
                    || (game.current_main_phase == MainPhase::Battle
                        && self.is_winning_or_is_opponent_in_battle(game, Faction::Emperor)
                        && game
                            .player(game.get_player(Some(Faction::Emperor)).expect("player"))
                            .special_forces_in_territory(game.current_battle.as_ref().expect("current battle").territory(game))
                            >= 3)
            }

            Faction::Guild => game.recently_paid_total_amount() > 5,

            Faction::BeneGesserit => {
                self.is_winning_or_is_opponent_in_battle(game, Faction::BeneGesserit) && !game.prevented(FactionAdvantage::BeneGesseritUsingVoice)
            }

            Faction::Ixian => true,

            Faction::Choam => {
                self.is_winning_or_is_opponent_in_battle(game, Faction::Choam)
                    && Self::known_opponent_cards(game, self.player, game.get_player(Some(Faction::Choam)).expect("Choam"))
                        .iter()
                        .any(|c| c.is_weapon() || c.is_defense())
            }

            Faction::Richese => {
                let richese = game.get_player(Some(Faction::Richese)).expect("Richese");
                if game.player(richese).has(game.card_just_won) {
                    self.card_quality(game, game.card_just_won.expect("card just won"), Some(richese)) > 2
                } else {
                    game.recently_paid_total_amount() > 4
                }
            }

            Faction::Ecaz => {
                game.current_phase == Phase::ResurrectionReport
                    && match self.determine_territory_to_remove_ecaz_from(game) {
                        Some(t) => {
                            result.set_ecaz_territory(game, Some(t));
                            true
                        }
                        None => false,
                    }
            }

            Faction::Moritani => {
                let faction = self.faction(game);
                let my_shipment_turn = (faction == Faction::Guild && game.current_phase == Phase::GuildShip && !game.guild_may_delay())
                    || (faction != Faction::Guild
                        && game.current_phase == Phase::NonGuildShip
                        && game.shipment_and_move_sequence.as_ref().expect("shipment and move sequence").current_player(game) == Some(self.player));
                my_shipment_turn && {
                    let territory = self.determine_shipment(game).to(game).expect("destination").territory();
                    let has_terror = game.terror_in(territory).next().is_some();
                    if has_terror {
                        result.set_moritani_territory(game, Some(territory));
                    }
                    has_terror
                }
            }

            _ => false,
        };

        play.then(|| Rc::new(result))
    }

    /// The stronghold to remove Ecaz forces from with the Ecaz nexus card.
    pub fn determine_territory_to_remove_ecaz_from(&self, game: &Game) -> Option<Territory> {
        let candidates: Vec<Territory> =
            NexusPlayed::valid_ecaz_territories(game).filter(|t| t.is_stronghold() || game.is_special_stronghold(*t)).collect();

        if candidates.is_empty() {
            return None;
        }

        let ecaz = game.get_player(Some(Faction::Ecaz)).expect("Ecaz");
        let forces = |t: &Territory| game.player(ecaz).any_forces_in_territory(Some(*t));

        let mut result = None;
        if candidates.len() >= 2 {
            result = candidates.iter().copied().highest_or_default(&self.random, forces);
        }

        result.or_else(|| candidates.iter().copied().filter(|t| forces(t) > 7).highest_or_default(&self.random, forces))
    }

    pub fn is_winning_or_is_opponent_in_battle(&self, game: &Game, faction: Faction) -> bool {
        let Some(battle) = game.current_battle.as_ref() else { return false };
        faction != self.ally(game)
            && ((battle.is_involved_faction(game, faction) && self.is_winning_faction(game, faction))
                || battle.opponent_of_faction(game, faction) == Some(self.player))
    }

    /// The current battle, if it is the battle phase and I am the aggressor or defender.
    fn battle_im_fighting<'g>(&self, game: &'g Game) -> Option<&'g Rc<BattleInitiated>> {
        if game.current_phase != Phase::BattlePhase {
            return None;
        }
        game.current_battle.as_ref().filter(|b| b.is_aggressor_or_defender(game, self.player))
    }

    /// The aspect to ask about with Atreides prescience from a nexus card, or `PrescienceAspect::None`
    /// to wait: until the opponent has used voice, or when I'm not fighting this battle.
    fn nexus_prescience_aspect(&self, game: &Game, earlier: PrescienceAspect) -> PrescienceAspect {
        let battle = game.current_battle.as_ref().expect("current battle");
        let opponent = battle.opponent_of_faction(game, self.faction(game));
        let voice_still_to_come =
            Voice::may_use_voice_opt(game, opponent) && game.current_voice.is_none() && battle.plan_of(game, opponent).is_none();
        if voice_still_to_come || !battle.is_aggressor_or_defender(game, self.player) {
            return PrescienceAspect::None;
        }

        let territory = battle.territory(game);
        let max_dial = Self::max_dial(game, self.player, territory.expect("territory"), opponent, false);
        self.best_prescience(game, self.player, opponent.expect("opponent"), max_dial, earlier, territory.expect("territory"))
    }

    /// Plays the nexus card of my own faction for its cunning effect.
    pub fn determine_nexus_played_cunning(&mut self, game: &Game, mut result: NexusPlayed) -> Option<Rc<NexusPlayed>> {
        let play = match game.player(self.player).nexus {
            Faction::Atreides => game.current_prescience.as_ref().is_some_and(|prescience| {
                result.atreides_prescience_aspect = self.nexus_prescience_aspect(game, prescience.aspect);
                result.atreides_prescience_aspect != PrescienceAspect::None
            }),

            Faction::Harkonnen => game.current_phase == Phase::ShipmentAndMoveConcluded && self.determine_worst_traitor(game).is_some(),

            Faction::Fremen => !game.monsters.is_empty() && self.determine_moved_battalion(game, true).is_some(),

            Faction::Emperor => {
                let t = game.current_battle.as_ref().expect("current battle").territory(game);
                game.player(self.player).forces_in_territory(t) >= 4
            }

            Faction::Guild => {
                let shipment = self.determine_shipment(game);
                !shipment.passed
                    && matches!(
                        self.decided_shipment_action,
                        ShipmentDecision::PreventNormalWin | ShipmentDecision::PreventFremenWin | ShipmentDecision::AttackWeakStronghold
                    )
            }

            Faction::BeneGesserit => self.determine_bene_gesserit_battle_announcement(game).is_some(),

            Faction::Ixian => {
                let t = game.current_battle.as_ref().expect("current battle").territory(game);
                game.player(self.player).forces_in_territory(t) >= 3
            }

            Faction::Tleilaxu => game.current_main_phase == MainPhase::Battle && !game.player(self.player).revealed_face_dancers.is_empty(),

            Faction::Ecaz => {
                self.battle_im_fighting(game).is_some() && !game.vidal().is_some_and(|v| game.player(self.player).leaders.contains(&v))
            }

            _ => false,
        };

        play.then(|| Rc::new(result))
    }

    /// Plays the nexus card of my own faction for its secret ally effect.
    pub fn determine_nexus_played_secret_ally(&mut self, game: &Game, mut result: NexusPlayed) -> Option<Rc<NexusPlayed>> {
        let play = match game.player(self.player).nexus {
            Faction::Atreides => {
                result.atreides_prescience_aspect = self.nexus_prescience_aspect(game, PrescienceAspect::None);
                result.atreides_prescience_aspect != PrescienceAspect::None
            }

            Faction::Harkonnen => self.determine_worst_traitor(game).is_some(),

            Faction::Fremen => game.current_main_phase == MainPhase::Resurrection && game.player(self.player).forces_killed >= 3,

            Faction::Guild => {
                let shipment = self.determine_shipment(game);
                !shipment.passed && Shipment::determine_cost(game, self.player, &shipment) > 5
            }

            Faction::BeneGesserit => game.current_battle.as_ref().is_some_and(|b| b.is_involved(game, self.player)),

            Faction::Ixian => self.card_quality(game, game.card_just_won.expect("card just won"), Some(self.player)) < 2,

            Faction::Tleilaxu => game.has_acted_or_passed.contains(&self.faction(game)) && self.plan_tleilaxu_secret_ally_revival(game, &mut result),

            Faction::Choam => match game.current_main_phase {
                MainPhase::Collection => {
                    self.faction(game) != Faction::BeneGesserit && {
                        let card = NexusPlayed::valid_choam_cards(game, self.player).next();
                        result.set_choam_card(card);
                        true
                    }
                }
                MainPhase::Battle => {
                    let battle = game.current_battle.as_ref().expect("current battle");
                    let auditee = battle.opponent_of(game, self.player);
                    let recent_battle_plan = battle.plan_of(game, auditee);
                    let me = game.player(self.player);
                    // Play it if the opponent holds a card outside their plan that I don't know yet.
                    // The plan is only needed (and fails when missing) if the opponent holds any cards.
                    game.player(auditee.expect("auditee")).treachery_cards.iter().copied().any(|c| {
                        let plan = recent_battle_plan.as_ref().expect("recent battle plan");
                        let auditable = Some(c) != plan.weapon() && Some(c) != plan.defense() && Some(Hero::TreacheryCard(c)) != plan.hero();
                        auditable && !me.known_cards.contains(&c)
                    })
                }
                _ => false,
            },

            Faction::Ecaz => self.battle_im_fighting(game).is_some_and(|battle| {
                let opponent = game.player(battle.opponent_of(game, self.player).expect("opponent of"));
                let play = opponent.faction != Faction::Tleilaxu && opponent.unrevealed_traitors().next().is_some();
                if play {
                    result.ecaz_faction = opponent.faction;
                }
                play
            }),

            _ => false,
        };

        play.then(|| Rc::new(result))
    }

    /// Fills in the free revival of the Tleilaxu secret ally nexus card. Returns whether it is worth playing:
    /// a hero and more than 2 forces, or the full 5 forces.
    fn plan_tleilaxu_secret_ally_revival(&self, game: &Game, result: &mut NexusPlayed) -> bool {
        let resources = self.resources(game);

        let mut special_forces = 0;
        while special_forces + 1 < NexusPlayed::valid_tleilaxu_max_amount(game, self.player, true)
            && NexusPlayed::determine_tleilaxu_cost_int32_int32(0, special_forces + 1) < resources
            && special_forces + 1 <= 5
        {
            special_forces += 1;
        }

        let mut forces = 0;
        while forces + 1 < NexusPlayed::valid_tleilaxu_max_amount(game, self.player, false)
            && NexusPlayed::determine_tleilaxu_cost_int32_int32(0, forces + 1) < resources
            && special_forces + forces + 1 <= 5
        {
            forces += 1;
        }

        let hero = NexusPlayed::valid_tleilaxu_heroes(game, self.player).into_iter().highest_or_default(&self.random, |l| l.value());
        if !((hero.is_some() && forces + special_forces > 2) || forces + special_forces >= 5) {
            return false;
        }

        result.tleilaxu_forces = forces;
        result.tleilaxu_special_forces = special_forces;
        result.set_tleilaxu_hero(hero);
        result.tleilaxu_assign_skill = Revival::may_assign_skill(game, self.player, hero);

        let to_planet = Revival::number_of_special_forces_that_may_be_placed_on_planet(game, self.player, special_forces);
        let (location, to_planet) = self.revival_target(game, to_planet);
        result.tleilaxu_number_of_special_forces_in_location = to_planet;
        result.set_tleilaxu_location(game, location);
        true
    }

    pub fn determine_charity_claimed(&self, game: &Game) -> Option<Rc<CharityClaimed>> {
        if !(game.economics_status == ChoamEconomicsStatus::Cancel || game.economics_status == ChoamEconomicsStatus::CancelFlipped) {
            return Some(Rc::new(CharityClaimed::new(game, self.faction(game))));
        }

        None
    }

    pub fn determine_karma_free_revival(&self, game: &Game) -> Option<Rc<KarmaFreeRevival>> {
        let special_forces_that_can_be_revived = std::cmp::min(3, Revival::valid_max_revivals(game, self.player, true, false));
        let forces_killed = game.player(self.player).forces_killed;

        if self.last_turn(game) || forces_killed + special_forces_that_can_be_revived >= 6 {
            let forces = std::cmp::max(0, std::cmp::min(3, forces_killed) - special_forces_that_can_be_revived);
            let mut e = KarmaFreeRevival::new(game, self.faction(game));
            e.set_hero(None);
            e.amount_of_forces = forces;
            e.amount_of_special_forces = special_forces_that_can_be_revived;
            return Some(Rc::new(e));
        }

        None
    }

    pub fn determine_karma_hand_swap(&self, game: &Game) -> Rc<KarmaHandSwap> {
        let mut cards = game.player(self.player).treachery_cards.clone();
        cards.sort_by_cached_key(|c| self.card_quality(game, *c, Some(self.player)));
        cards.truncate(game.karma_hand_swap_number_of_cards.try_into().unwrap_or(usize::MAX));

        let mut e = KarmaHandSwap::new(game, self.faction(game));
        e.set_returned_cards(&cards);
        Rc::new(e)
    }

    pub fn determine_karma_shipment_prevention(&self, game: &Game) -> Option<Rc<KarmaShipmentPrevention>> {
        if game.current_phase != Phase::NonGuildShip {
            return None;
        }

        let valid_targets: Vec<Faction> = KarmaShipmentPrevention::get_valid_targets(game, self.player).collect();
        let reserves = |pl: &Player| pl.forces_in_reserve() + pl.special_forces_in_reserve();
        // The spice an opponent can ship with: their own and their ally's.
        let resources = |pl: &Player| pl.resources + pl.allied_resources(game);

        let winning_opponent_that_can_ship_most = self
            .opponents_to_ship_and_move(game)
            .into_iter()
            .filter(|p| valid_targets.contains(&game.player(*p).faction))
            .filter(|p| {
                let pl = game.player(*p);
                self.is_winning_opponent(game, *p) && reserves(pl) > 2 && resources(pl) > 2
            })
            .min_by_key(|p| {
                let pl = game.player(*p);
                Reverse(std::cmp::min(reserves(pl), resources(pl)))
            });

        let w = winning_opponent_that_can_ship_most?;
        if game.shipment_and_move_sequence.as_ref().expect("shipment and move sequence").current_player(game) == Some(w) {
            let mut e = KarmaShipmentPrevention::new(game, self.faction(game));
            e.target = game.player(w).faction;
            return Some(Rc::new(e));
        }

        None
    }

    pub fn determine_karma_hand_swap_initiated(&self, game: &Game) -> Option<Rc<KarmaHandSwapInitiated>> {
        if game.current_phase != Phase::BiddingReport {
            return None;
        }

        let me = self.player;
        if game.player(me).treachery_cards.iter().filter(|c| self.card_quality(game, **c, Some(me)) <= 2).count() < 2 {
            return None;
        }

        let known_cards_with = |o: PlayerIndex, good: bool| {
            Self::known_cards_held_by_other_player(game, me, o).iter().filter(|c| (self.card_quality(game, **c, Some(me)) >= 3) == good).count() as i32
        };
        let swap_with = |o: PlayerIndex| {
            let mut e = KarmaHandSwapInitiated::new(game, self.faction(game));
            e.target = game.player(o).faction;
            Some(Rc::new(e))
        };

        // Swap with an opponent holding 2 or more good cards that I know of.
        let most_good_cards = self.opponents(game).into_iter().highest_or_default(&self.random, |o| known_cards_with(*o, true));
        if let Some(o) = most_good_cards.filter(|o| known_cards_with(*o, true) >= 2) {
            return swap_with(o);
        }

        // Or with one holding 4 cards, of which at most 2 are bad as far as I know.
        let full_hand = self.opponents(game).into_iter().find(|o| game.player(*o).treachery_cards.len() == 4);
        if let Some(o) = full_hand.filter(|o| known_cards_with(*o, false) <= 2) {
            return swap_with(o);
        }

        None
    }

    pub fn determine_harvester_played(&self, game: &Game) -> Option<Rc<HarvesterPlayed>> {
        if game.current_turn > 1
            && ((game.current_phase == Phase::HarvesterA && self.resources_in(game, game.latest_spice_card_a.expect("latest spice card a").location().expect("location")) > 6)
                || (game.current_phase == Phase::HarvesterB && self.resources_in(game, game.latest_spice_card_b.expect("latest spice card b").location().expect("location")) > 6))
        {
            return Some(Rc::new(HarvesterPlayed::new(game, self.faction(game))));
        }

        None
    }

    pub fn determine_mulligan_performed(&self, game: &Game) -> Rc<MulliganPerformed> {
        let mut e = MulliganPerformed::new(game, self.faction(game));
        e.passed = !MulliganPerformed::may_mulligan(game, self.player);
        Rc::new(e)
    }

    pub fn determine_delay(&self, game: &Game) -> Option<Rc<GuildDelay>> {
        if !game.prevented(FactionAdvantage::GuildDetermineMoveMoment) {
            return Some(Rc::new(GuildDelay::new(game, self.faction(game))));
        }

        None
    }

    pub fn determine_clairvoyance(&self, game: &Game) -> Option<Rc<ClairVoyancePlayed>> {
        if let Some(battle) = self.battle_im_fighting(game) {
            let opponent = battle.opponent_of(game, self.player);
            let wait_for_prescience_or_voice = battle.plan_of(game, opponent).is_none()
                && (Prescience::may_use_prescience_opt(game, opponent) || Voice::may_use_voice_opt(game, opponent));
            let asked_in_this_battle = game.latest_clairvoyance_battle.as_ref().is_some_and(|b| Rc::ptr_eq(b, battle));

            if !wait_for_prescience_or_voice && !asked_in_this_battle {
                if let Some(e) = self.clairvoyance_about_battle_plan(game, battle) {
                    return Some(e);
                }
            }
        }

        if game.current_phase == Phase::Bidding && !game.player(self.player).has_room_for_cards(game) {
            let me = game.player(self.player);
            // Looked up lazily: it fails when my ally isn't seated.
            let mut safe: Option<Vec<Hero>> = None;
            let best_leader_to_ask_about = me
                .leaders
                .iter()
                .map(|l| Hero::Leader(*l))
                .filter(|l| game.is_alive(*l) && !safe.get_or_insert_with(|| self.safe_or_known_traitor_leaders(game)).contains(l))
                .highest_or_default(&self.random, |l| l.value());
            let best_player_to_ask = self
                .opponents(game)
                .into_iter()
                .filter(|o| !best_leader_to_ask_about.is_some_and(|l| game.player(*o).told_non_traitors.contains(&l)))
                .highest_or_default(&self.random, |p| {
                    let pl = game.player(*p);
                    pl.traitors.len() as i32 - pl.revealed_traitors.len() as i32
                });

            if let (Some(leader), Some(p)) = (best_leader_to_ask_about, best_player_to_ask) {
                let mut e = ClairVoyancePlayed::new(game, self.faction(game));
                e.target = game.player(p).faction;
                e.question = ClairvoyanceQuestion::LeaderAsTraitor;
                e.set_parameter1(ExpressionElement::Int(leader.id()));
                return Some(Rc::new(e));
            }
        }

        None
    }

    /// Asks my opponent in `battle` whether they will defend against my weapons, or use a weapon my
    /// defenses can't both stop.
    fn clairvoyance_about_battle_plan(&self, game: &Game, battle: &BattleInitiated) -> Option<Rc<ClairVoyancePlayed>> {
        let opponent = battle.opponent_of(game, self.player).expect("opponent of");
        if Self::nr_of_unknown_opponent_cards(game, self.player, opponent) <= 0 {
            return None;
        }

        let ask = |question: ClairvoyanceQuestion, card_type: TreacheryCardType| {
            let mut e = ClairVoyancePlayed::new(game, self.faction(game));
            e.target = game.player(opponent).faction;
            e.question = question;
            e.question_parameter1 = Some(card_type.to_string());
            Some(Rc::new(e))
        };

        let territory = battle.territory(game);
        let my_weapons: Vec<TreacheryCard> =
            Battle::valid_weapons(game, self.player, None, None, territory, false).into_iter().flatten().collect();
        // Looked up lazily: it fails when my ally isn't seated.
        let mut known: Option<Vec<TreacheryCard>> = None;
        let enemy_defenses: Vec<TreacheryCard> = Battle::valid_defenses(game, opponent, None, territory, false)
            .into_iter()
            .flatten()
            .filter(|w| known.get_or_insert_with(|| game.known_cards(self.player)).contains(w))
            .collect();

        if !self.my_prescience(game).is_some_and(|p| p.aspect == PrescienceAspect::Defense)
            && my_weapons.iter().all(|w| w.type_() != TreacheryCardType::ProjectileAndPoison)
        {
            if my_weapons.iter().any(|w| w.is_poison_weapon())
                && !self.opponent_may_not_use(game, TreacheryCardType::PoisonDefense, false)
                && !enemy_defenses.iter().any(|w| w.is_poison_defense())
            {
                return ask(ClairvoyanceQuestion::CardTypeAsDefenseInBattle, TreacheryCardType::PoisonDefense);
            }
            if my_weapons.iter().any(|w| w.is_projectile_weapon())
                && !self.opponent_may_not_use(game, TreacheryCardType::ProjectileDefense, false)
                && !enemy_defenses.iter().any(|w| w.is_projectile_defense())
            {
                return ask(ClairvoyanceQuestion::CardTypeAsDefenseInBattle, TreacheryCardType::ProjectileDefense);
            }
        }

        let my_defenses: Vec<TreacheryCard> =
            Battle::valid_defenses(game, self.player, None, territory, false).into_iter().flatten().collect();

        if !self.my_prescience(game).is_some_and(|p| p.aspect == PrescienceAspect::Weapon)
            && my_defenses.iter().all(|w| w.type_() != TreacheryCardType::ShieldAndAntidote)
            && my_defenses.iter().any(|w| w.is_poison_defense())
            && my_defenses.iter().any(|w| w.is_projectile_defense())
            && !self.opponent_may_not_use(game, TreacheryCardType::Poison, true)
            && !self.opponent_may_not_use(game, TreacheryCardType::Projectile, true)
        {
            return ask(ClairvoyanceQuestion::CardTypeAsWeaponInBattle, TreacheryCardType::Poison);
        }

        None
    }

    /// Whether my voice forbids the opponent to use `type_` (as a weapon or as a defense).
    pub fn opponent_may_not_use(&self, game: &Game, type_: TreacheryCardType, as_weapon: bool) -> bool {
        let voice = self.my_voice(game);
        voice.is_some_and(|v| v.may_not() && Voice::is_voiced_by(game, as_weapon, false, type_, v.type_))
    }

    pub fn determine_clair_voyance_answered(&self, game: &Game) -> Rc<ClairVoyanceAnswered> {
        // Not caught below: without a question the whole decision fails.
        let clairvoyance = game.latest_clairvoyance.as_ref().expect("latest clairvoyance");
        let question = clairvoyance.question;

        let p1 = || clairvoyance.parameter1(game);
        let card_type = || unbox_enum::<TreacheryCardType>(&p1());
        let in_battle = || game.current_battle.as_ref().is_some_and(|b| b.is_aggressor_or_defender(game, self.player));
        let my_plan = || match game.current_battle.as_ref().expect("current battle").plan_of(game, Some(self.player)) {
            Some(plan) => plan,
            None => self.determine_battle_plan(game, true),
        };
        let defends_with = |plan: &Battle| plan.defense().is_some() && ClairVoyanceAnswered::is_questioned_by(false, plan.defense().expect("defense").type_(), card_type());
        let attacks_with = |plan: &Battle| plan.weapon().is_some() && ClairVoyanceAnswered::is_questioned_by(true, plan.weapon().expect("weapon").type_(), card_type());

        // A question I fail to work out gets an unknown answer.
        let answer = super::guarded(|| {
            let me = game.player(self.player);
            match question {
                ClairvoyanceQuestion::CardTypeAsDefenseInBattle if in_battle() => yes_no(defends_with(&my_plan())),

                ClairvoyanceQuestion::CardTypeAsWeaponInBattle if in_battle() => yes_no(attacks_with(&my_plan())),

                ClairvoyanceQuestion::CardTypeInBattle if in_battle() => {
                    let plan = my_plan();
                    yes_no(
                        defends_with(&plan)
                            || attacks_with(&plan)
                            || (plan.hero().is_some_and(|h| h.as_treachery_card().is_some()) && card_type() == TreacheryCardType::Mercenary),
                    )
                }

                ClairvoyanceQuestion::DialOfMoreThanXInBattle if in_battle() => {
                    let plan = my_plan();
                    let opponent = game.current_battle.as_ref().expect("current battle").opponent_of(game, self.player).expect("opponent of");
                    yes_no(plan.dial(game, game.player(opponent).faction) > unbox_float(&p1()))
                }

                ClairvoyanceQuestion::LeaderInBattle if in_battle() => yes_no(my_plan().hero() == cast_hero(&p1())),

                ClairvoyanceQuestion::HasCardTypeInHand => yes_no(me.treachery_cards.iter().any(|c| covers(c.type_(), card_type()))),

                ClairvoyanceQuestion::LeaderAsFacedancer => yes_no(me.face_dancers.iter().any(|f| cast_hero(&p1()).is_some_and(|h| f.is_face_dancer(h)))),

                ClairvoyanceQuestion::LeaderAsTraitor => yes_no(me.traitors.iter().any(|f| cast_hero(&p1()).is_some_and(|h| f.is_traitor(h)))),

                ClairvoyanceQuestion::Prediction => yes_no(
                    me.predicted_faction == unbox_enum::<Faction>(&p1()) && me.predicted_turn == unbox_int(&clairvoyance.parameter2()),
                ),

                ClairvoyanceQuestion::WillAttackX => ClairVoyanceAnswer::No,

                _ => ClairVoyanceAnswer::Unknown,
            }
        })
        .unwrap_or(ClairVoyanceAnswer::Unknown);

        let mut e = ClairVoyanceAnswered::new(game, self.faction(game));
        e.answer = answer;
        Rc::new(e)
    }

    /// The most valuable leader among `candidates` that `acceptable` allows, preferring leaders I know
    /// can't betray me.
    fn leader_to_revive<I: IntoIterator<Item = Hero>>(
        &self,
        game: &Game,
        candidates: impl Fn() -> I,
        acceptable: impl Fn(Hero) -> bool,
    ) -> Option<Hero> {
        let score = |l: &Hero| l.value() + self.hero_revival_penalty(game, *l);
        // Looked up lazily: it fails when my ally isn't seated.
        let mut safe: Option<Vec<Hero>> = None;
        candidates()
            .into_iter()
            .filter(|l| safe.get_or_insert_with(|| self.safe_or_known_traitor_leaders(game)).contains(l) && acceptable(*l))
            .highest_or_default(&self.random, score)
            .or_else(|| candidates().into_iter().filter(|l| acceptable(*l)).highest_or_default(&self.random, score))
    }

    pub fn determine_raise_dead_played(&self, game: &Game) -> Option<Rc<RaiseDeadPlayed>> {
        let me = game.player(self.player);
        if !((game.current_main_phase == MainPhase::Bidding && !me.has_room_for_cards(game)) || game.current_main_phase == MainPhase::ShipmentAndMove) {
            return None;
        }

        let nr_of_living_leaders = me.leaders.iter().filter(|l| game.is_alive(Hero::Leader(**l))).count() as i32;
        let special_forces_that_can_be_revived = std::cmp::min(5, Revival::valid_max_revivals(game, self.player, true, false));
        let revivable = me.forces_killed + special_forces_that_can_be_revived;

        if (revivable >= 7 && nr_of_living_leaders > 1) || (revivable >= 17 && nr_of_living_leaders > 0) {
            let forces = std::cmp::max(0, std::cmp::min(5, me.forces_killed) - special_forces_that_can_be_revived);
            let special_forces_to_planet =
                Revival::number_of_special_forces_that_may_be_placed_on_planet(game, self.player, special_forces_that_can_be_revived);
            let (target_of_forces, special_forces_to_planet) = self.revival_target(game, special_forces_to_planet);

            let mut e = RaiseDeadPlayed::new(game, self.faction(game));
            e.set_hero(None);
            e.amount_of_forces = forces;
            e.amount_of_special_forces = special_forces_that_can_be_revived;
            e.assign_skill = false;
            e.set_location(game, target_of_forces);
            e.number_of_special_forces_in_location = special_forces_to_planet;
            return Some(Rc::new(e));
        }

        let minimum_value = if self.faction(game) == Faction::Tleilaxu && nr_of_living_leaders > 2 { 4 } else { 0 };
        let ally = self.ally(game);
        let leader_to_revive = self.leader_to_revive(
            game,
            || RaiseDeadPlayed::valid_heroes(game, self.player),
            |l| l.faction() != ally && l.value() >= minimum_value,
        )?;

        let mut e = RaiseDeadPlayed::new(game, self.faction(game));
        e.set_hero(Some(leader_to_revive));
        e.amount_of_forces = 0;
        e.amount_of_special_forces = 0;
        e.assign_skill = Revival::may_assign_skill(game, self.player, Some(leader_to_revive));
        Some(Rc::new(e))
    }

    pub fn determine_amal_played(&self, game: &Game) -> Option<Rc<AmalPlayed>> {
        let opponents_resources = || self.opponents(game).into_iter().map(|p| game.player(p).resources).sum::<i32>();
        let ally_resources = if self.ally(game) != Faction::None { game.player(self.player).allied_resources(game) } else { 0 };
        let our_resources = self.resources(game) + ally_resources;

        let play = if self.faction(game) == Faction::Guild {
            game.current_phase == Phase::Resurrection && opponents_resources() > 2 * our_resources
        } else {
            game.current_turn > 1
                && game.current_main_phase == MainPhase::Bidding
                && opponents_resources() > 10
                && opponents_resources() > (self.opponents(game).len() as i32 + 1) * our_resources
        };

        play.then(|| Rc::new(AmalPlayed::new(game, self.faction(game))))
    }

    pub fn determine_recruits_played(&self, game: &Game) -> Option<Rc<RecruitsPlayed>> {
        let me = game.player(self.player);
        let killed = me.forces_killed + me.special_forces_killed;
        if (killed >= 6 && self.resources(game) >= 14) || (killed >= 4 && game.free_revivals(self.player, false) >= 2) {
            return Some(Rc::new(RecruitsPlayed::new(game, self.faction(game))));
        }

        None
    }

    pub fn determine_metheor_played(&self, game: &Game) -> Option<Rc<MetheorPlayed>> {
        let faction = self.faction(game);
        let ally = self.ally(game);
        let targets = [game.map.arrakeen(), game.map.carthag()];
        let others = forces_in(game, &targets, |f| f != faction && f != ally);
        let ours = forces_in(game, &targets, |f| f == faction || f == ally);

        (others > 2 * ours).then(|| Rc::new(MetheorPlayed::new(game, faction)))
    }

    pub fn determine_storm_dialled(&self, game: &Game) -> Rc<StormDialled> {
        let min = StormDialled::valid_min_amount(game);
        let max = StormDialled::valid_max_amount(game);
        let mut e = StormDialled::new(game, self.faction(game));
        e.amount = min + self.d(1, 1 + max - min) - 1;
        Rc::new(e)
    }

    pub fn determine_traitors_selected(&self, game: &Game) -> Rc<TraitorsSelected> {
        let me = game.player(self.player);
        let faction = self.faction(game);
        let traitor = me.traitors.iter().copied().filter(|l| l.faction() != faction).highest_or_default(&self.random, |l| l.value()).or_else(|| {
            me.traitors.iter().copied().highest_or_default(&self.random, |l| {
                l.value() - if l.faction() == Faction::Atreides && game.applicable(Rule::AtreidesMessiah) { 2 } else { 0 }
            })
        });
        let mut e = TraitorsSelected::new(game, faction);
        e.set_selected_traitor(traitor);
        Rc::new(e)
    }

    pub fn determine_faction_trade_offered(&self, game: &Game) -> Option<Rc<FactionTradeOffered>> {
        let faction = self.faction(game);
        let mut matching = game.current_trade_offers.iter().filter(|matching_offer| matching_offer.target == faction);
        let m = matching.next()?;
        // C# parity: SingleOrDefault throws when two factions offer me a trade.
        assert!(matching.next().is_none(), "more than one trade offer to {faction}");
        let mut e = FactionTradeOffered::new(game, faction);
        e.target = m.initiator();
        Some(Rc::new(e))
    }

    pub fn determine_thumper_played(&self, game: &Game) -> Rc<ThumperPlayed> {
        Rc::new(ThumperPlayed::new(game, self.faction(game)))
    }

    pub fn determine_storm_spell_played(&self, game: &Game) -> Option<Rc<StormSpellPlayed>> {
        let faction = self.faction(game);
        let ally = self.ally(game);

        let mut options = vec![StormKills { moves: 0, mine: 0, enemy: 0 }];
        let (mut mine, mut enemy) = (0, 0);
        for moves in 1..=10 {
            let sector = (game.sector_in_storm + moves) % Map::NUMBER_OF_SECTORS;
            let affected_locations: Vec<Location> =
                game.map.locations(false).filter(|l| l.sector(&game.map) == sector && !game.is_protected_from_storm(*l)).collect();
            mine += forces_in(game, &affected_locations, |f| f == faction || f == ally);
            enemy += forces_in(game, &affected_locations, |f| f != faction && f != ally);
            options.push(StormKills { moves, mine, enemy });
        }

        let best = options.into_iter().highest_or_default(&self.random, |k| k.enemy - k.mine).expect("there are always 11 options");

        if best.enemy - best.mine >= 10 - (game.current_turn + if game.player(self.player).has_room_for_cards(game) { 0 } else { 4 }) {
            let mut storm_spell = StormSpellPlayed::new(game, faction);
            storm_spell.move_amount = best.moves;
            return Some(Rc::new(storm_spell));
        }

        None
    }

    pub fn determine_distrans_used(&self, game: &Game) -> Option<Rc<DistransUsed>> {
        if game.current_phase != Phase::WaitingForNextBiddingRound {
            return None;
        }

        let worst_card = DistransUsed::valid_cards(game, self.player).lowest_or_default(&self.random, |c| self.card_quality(game, *c, Some(self.player)))?;
        if self.card_quality(game, worst_card, Some(self.player)) > 1 {
            return None;
        }

        let ally = self.ally(game);
        let target = DistransUsed::valid_targets(game, self.player)
            .filter(|f| *f != ally && (!game.applicable(Rule::BeneGesseritWorthlessAsKarma) || *f != Faction::BeneGesserit))
            .map(|f| game.get_player(Some(f)).expect("player"))
            .highest_or_default(&self.random, |p| game.number_of_victory_points(*p, true))?;

        let mut e = DistransUsed::new(game, self.faction(game));
        e.set_card(Some(worst_card));
        e.target = game.player(target).faction;
        Some(Rc::new(e))
    }

    /// How much less I want to revive `h`, on top of its value.
    pub fn hero_revival_penalty(&self, game: &Game, h: Hero) -> i32 {
        if matches!(h.hero_type(), HeroType::Messiah | HeroType::Auditor) {
            return 20;
        }
        if game.player(self.player).known_non_traitors.contains(&h) {
            return 10;
        }
        0
    }

    pub fn determine_revival(&self, game: &Game) -> Option<Rc<Revival>> {
        let faction = self.faction(game);
        if game.current_revival_requests.iter().any(|r| r.by(faction)) {
            return None;
        }

        let me = game.player(self.player);
        let resources = self.resources(game);
        let nr_of_living_leaders = me.leaders.iter().filter(|l| game.is_alive(Hero::Leader(**l))).count() as i32;
        let minimum_value = if faction == Faction::Tleilaxu && nr_of_living_leaders > 2 { 4 } else { 0 };
        let max_to_spend_on_hero_revival = std::cmp::min(resources, 8);
        let ally = self.ally(game);

        let leader_to_revive = self.leader_to_revive(
            game,
            || Revival::valid_revival_heroes(game, self.player),
            |l| {
                Revival::get_price_of_hero_revival(game, self.player, Some(l)) <= max_to_spend_on_hero_revival
                    && l.faction() != ally
                    && l.value() >= minimum_value
            },
        );

        let use_secret_ally = Revival::may_use_emperor_secret_ally(game, self.player) && me.forces_killed - game.free_revivals(self.player, false) >= 3;

        let (forces_revived_by_emperor, special_forces_revived_by_emperor) =
            Self::determine_optimal_use_of_emperor_revivals(game, self.player, use_secret_ally);
        let cost = |forces: i32, special_forces: i32| {
            Revival::determine_cost(
                game,
                self.player,
                leader_to_revive,
                forces,
                special_forces,
                forces_revived_by_emperor,
                special_forces_revived_by_emperor,
                use_secret_ally,
            )
            .total_cost_for_player()
        };

        // Revive as many special forces as allowed, killed and affordable, then normal forces.
        let mut special_forces_to_revive = 0;
        while special_forces_revived_by_emperor + special_forces_to_revive + 1 <= Revival::valid_max_revivals(game, self.player, true, use_secret_ally)
            && special_forces_revived_by_emperor + special_forces_to_revive + 1 <= me.special_forces_killed
            && cost(0, special_forces_to_revive + 1) <= resources
        {
            special_forces_to_revive += 1;
        }

        let mut normal_forces_to_revive = 0;
        while special_forces_to_revive + normal_forces_to_revive + 1 <= Revival::valid_max_revivals(game, self.player, false, use_secret_ally)
            && forces_revived_by_emperor + normal_forces_to_revive + 1 <= me.forces_killed
            && cost(normal_forces_to_revive + 1, special_forces_to_revive) <= resources
        {
            normal_forces_to_revive += 1;
        }

        let assign_skill = leader_to_revive.is_some() && Revival::may_assign_skill(game, self.player, leader_to_revive);

        let forces_to_planet =
            Revival::number_of_forces_that_may_be_placed_on_planet(game, self.player, use_secret_ally, normal_forces_to_revive + forces_revived_by_emperor);
        let special_forces_to_planet =
            Revival::number_of_special_forces_that_may_be_placed_on_planet(game, self.player, special_forces_to_revive + special_forces_revived_by_emperor);

        // The location is picked for normal forces if any may go to the planet, else for special forces.
        // Only the count it was picked for drops to 0 when there is none.
        let (target_of_forces, forces_to_planet, special_forces_to_planet) = if forces_to_planet > 0 {
            let (target, forces_to_planet) = self.revival_target(game, forces_to_planet);
            (target, forces_to_planet, special_forces_to_planet)
        } else {
            let (target, special_forces_to_planet) = self.revival_target(game, special_forces_to_planet);
            (target, forces_to_planet, special_forces_to_planet)
        };

        if leader_to_revive.is_none() && special_forces_to_revive + normal_forces_to_revive <= 0 {
            return None;
        }

        let mut e = Revival::new(game, faction);
        e.set_hero(leader_to_revive);
        e.amount_of_forces = normal_forces_to_revive;
        e.extra_forces_paid_by_emperor = forces_revived_by_emperor;
        e.amount_of_special_forces = special_forces_to_revive;
        e.extra_special_forces_paid_by_emperor = special_forces_revived_by_emperor;
        e.assign_skill = assign_skill;
        e.uses_emperor_secret_ally = use_secret_ally;
        e.set_location(game, target_of_forces);
        e.number_of_forces_in_location = forces_to_planet;
        e.number_of_special_forces_in_location = special_forces_to_planet;
        Some(Rc::new(e))
    }

    /// Where `count` revived forces go on the planet, and how many of them do: all of them, or none
    /// when there is no good location.
    fn revival_target(&self, game: &Game, count: i32) -> (Option<Location>, i32) {
        if count <= 0 {
            return (None, count);
        }
        let locations: Vec<Location> = Revival::valid_revived_force_locations(game, self.player).collect();
        match self.best_revival_location(game, &locations) {
            Some(l) => (Some(l), count),
            None => (None, 0),
        }
    }

    pub fn best_revival_location(&self, game: &Game, valid_locations: &[Location]) -> Option<Location> {
        let me = game.player(self.player);
        let few_or_vacant = |l: Location| self.vacant(game, l) || (me.any_forces_in(Some(l)) > 0 && me.any_forces_in(Some(l)) < 4);
        let first = |pred: &dyn Fn(Location) -> bool| valid_locations.iter().copied().find(|l| pred(*l));
        let mut result = first(&|l| l == game.map.carthag() && few_or_vacant(l))
            .or_else(|| first(&|l| l == game.map.arrakeen() && few_or_vacant(l)))
            .or_else(|| first(&|l| self.resources_in(game, l) >= 4))
            .or_else(|| first(&|l| l.is_stronghold() && few_or_vacant(l)));

        if result.is_none() && me.forces_in_reserve() + me.special_forces_in_reserve() > 0 {
            result = first(&|l| l == game.map.polar_sink());
        }

        result
    }

    /// How many (forces, special forces) the Emperor, as `p`'s ally, should pay to revive for `p`.
    pub fn determine_optimal_use_of_emperor_revivals(g: &Game, p: PlayerIndex, use_cunning: bool) -> (i32, i32) {
        let pl = g.player(p);
        if pl.ally != Faction::Emperor {
            return (0, 0);
        }

        let emperor = g.get_player(Some(Faction::Emperor)).expect("Emperor");
        let potential_maximum_by_emperor =
            if g.prevented(FactionAdvantage::EmperorLetAllyReviveExtraForces) { 0 } else { Revival::valid_max_revivals_by_emperor(g, p) };

        let mut max_special_forces = Revival::valid_max_revivals(g, p, true, use_cunning);
        let mut free_revivals = g.free_revivals(p, use_cunning);

        if max_special_forces > 0 && free_revivals > 0 {
            max_special_forces -= 1;
            free_revivals -= 1;
        }

        let mut special_forces = 0;
        while special_forces + 1 <= pl.special_forces_killed
            && special_forces + 1 <= max_special_forces
            && special_forces + 1 <= potential_maximum_by_emperor
            && Revival::determine_cost_of_forces_for_emperor(g, emperor, pl.faction, 0, special_forces + 1) <= g.player(emperor).resources
        {
            special_forces += 1;
        }

        let max_forces = std::cmp::max(Revival::valid_max_revivals(g, p, false, use_cunning) - free_revivals, 0);

        let mut forces = 0;
        while forces + 1 <= pl.forces_killed
            && forces + 1 <= max_forces
            && special_forces + forces + 1 <= potential_maximum_by_emperor
            && Revival::determine_cost_of_forces_for_emperor(g, emperor, pl.faction, forces + 1, special_forces) <= g.player(emperor).resources
        {
            forces += 1;
        }

        (forces, special_forces)
    }

    pub fn determine_discarded_searched_announced(&self, game: &Game) -> Option<Rc<DiscardedSearchedAnnounced>> {
        if game.current_main_phase != MainPhase::Contemplate {
            return None;
        }

        let card_to_search =
            DiscardedSearched::valid_cards(game).into_iter().highest_or_default(&self.random, |c| self.card_quality(game, *c, Some(self.player)))?;
        (self.card_quality(game, card_to_search, Some(self.player)) >= 4).then(|| Rc::new(DiscardedSearchedAnnounced::new(game, self.faction(game))))
    }

    pub fn determine_discarded_searched(&self, game: &Game) -> Rc<DiscardedSearched> {
        let card_to_search =
            DiscardedSearched::valid_cards(game).into_iter().highest_or_default(&self.random, |c| self.card_quality(game, *c, Some(self.player)));
        let mut e = DiscardedSearched::new(game, self.faction(game));
        e.set_card(card_to_search);
        Rc::new(e)
    }

    pub fn determine_discarded_taken(&self, game: &Game) -> Option<Rc<DiscardedTaken>> {
        let card_to_take =
            DiscardedTaken::valid_cards(game, self.player).highest_or_default(&self.random, |c| self.card_quality(game, *c, Some(self.player)))?;
        if self.card_quality(game, card_to_take, Some(self.player)) < 4 {
            return None;
        }

        let mut e = DiscardedTaken::new(game, self.faction(game));
        e.set_card(Some(card_to_take));
        Some(Rc::new(e))
    }

    pub fn determine_juice_played(&self, game: &Game) -> Option<Rc<JuicePlayed>> {
        let go_last_in_shipment = game.current_main_phase == MainPhase::ShipmentAndMove
            && game.current_moment == MainPhaseMoment::Start
            && self.faction(game) != Faction::Guild
            && game.shipment_and_move_sequence.as_ref().expect("shipment and move sequence").get_players_in_sequence(game).last().and_then(|e| e.player) != Some(self.player);
        let go_last_in_battle = || {
            game.current_main_phase == MainPhase::Battle
                && game.current_moment == MainPhaseMoment::Start
                && !Battle::battles_to_be_fought(game, self.player, false).is_empty()
        };

        if go_last_in_shipment || go_last_in_battle() {
            let mut e = JuicePlayed::new(game, self.faction(game));
            e.type_ = JuiceType::GoLast;
            return Some(Rc::new(e));
        }

        None
    }

    pub fn determine_bureaucracy(&self, game: &Game) -> Rc<Bureaucracy> {
        let mut e = Bureaucracy::new(game, self.faction(game));
        e.passed = game.target_of_bureaucracy == self.ally(game);
        Rc::new(e)
    }

    pub fn determine_diplomacy(&self, game: &Game) -> Option<Rc<Diplomacy>> {
        let opponent_plan = game.current_battle.as_ref().expect("current battle").plan_of_opponent(game, Some(self.player));
        let opponent_plan = opponent_plan.expect("opponent plan");
        if opponent_plan.weapon().is_some_and(|w| w.countered_by(opponent_plan.defense(), None)) {
            let mut e = Diplomacy::new(game, self.faction(game));
            e.set_card(Some(Diplomacy::valid_cards(game, self.player).into_iter().next().expect("an element")));
            return Some(Rc::new(e));
        }

        None
    }

    pub fn determine_skill_assigned(&self, game: &Game) -> Rc<SkillAssigned> {
        let me = game.player(self.player);
        let skills: Vec<LeaderSkill> = SkillAssigned::valid_skills(game, self.player);
        let holds = |is: fn(&TreacheryCard) -> bool| me.treachery_cards.iter().any(is);
        let is_economic_faction = me.is(Faction::Emperor) || me.is(Faction::Guild) || me.is(Faction::Choam) || me.is(Faction::Tleilaxu);

        // Skills in order of preference, each with whether it suits me.
        let preferences = [
            (holds(|c| c.is_projectile_weapon()), LeaderSkill::Swordmaster),
            (holds(|c| c.is_poison_weapon()), LeaderSkill::MasterOfAssassins),
            (holds(|c| c.is_projectile_defense()), LeaderSkill::Adept),
            (holds(|c| c.is_poison_defense()), LeaderSkill::KillerMedic),
            (holds(|c| c.is_useless()), LeaderSkill::Warmaster),
            (true, LeaderSkill::Graduate),
            (!me.is(Faction::Atreides), LeaderSkill::Thinker),
            (me.is(Faction::Fremen), LeaderSkill::Sandmaster),
            (!me.is(Faction::Fremen), LeaderSkill::Smuggler),
            (!is_economic_faction, LeaderSkill::Bureaucrat),
            (!is_economic_faction, LeaderSkill::Banker),
            (!me.is(Faction::Tleilaxu), LeaderSkill::Decipherer),
            (true, LeaderSkill::Swordmaster),
            (true, LeaderSkill::MasterOfAssassins),
            (true, LeaderSkill::Adept),
            (true, LeaderSkill::KillerMedic),
            (true, LeaderSkill::Warmaster),
        ];
        let skill = preferences
            .into_iter()
            .filter(|(suits, _)| *suits)
            .map(|(_, s)| s)
            .find(|s| skills.contains(s))
            .or_else(|| skills.iter().copied().find(|s| *s != LeaderSkill::Planetologist))
            .unwrap_or_else(|| skills.iter().copied().next().expect("an element"));

        let leader = self.random_item_from(&SkillAssigned::valid_leaders(game, self.player));
        let mut e = SkillAssigned::new(game, self.faction(game));
        e.passed = false;
        e.set_leader(leader);
        e.skill = skill;
        Rc::new(e)
    }

    pub fn random_item_from<T: Clone>(&self, items: &[T]) -> T {
        // An empty list rolls 0, which fails here.
        let i = self.d(1, items.len() as i32) - 1;
        match usize::try_from(i).ok().and_then(|i| items.get(i)) {
            Some(item) => item.clone(),
            None => panic!("index out of range"),
        }
    }

    pub fn determine_planetology(&self, game: &Game) -> Rc<Planetology> {
        let mut e = Planetology::new(game, self.faction(game));
        e.add_one_to_movement = true;
        Rc::new(e)
    }
}
