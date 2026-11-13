use crate::prelude::*;
use dotnet_compat::linq::sum_i32;
use treachery_engine::events::PlacementEventMethods;
use crate::support::{cast_hero, unbox_enum, unbox_float, unbox_int};

/// `x == y` for two possibly null event references: reference equality.
fn same_battle(a: &Option<Rc<BattleInitiated>>, b: &Option<Rc<BattleInitiated>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

impl ClassicBot {
    pub fn determine_resources_transferred(&self, game: &Game) -> Option<Rc<ResourcesTransferred>> {
        let amount = ResourcesTransferred::max_amount(game, self.player);
        if self.resources(game).wrapping_sub(amount) > 15 && game.player(self.player).allied_resources(game) < self.resources(game) {
            let mut e = ResourcesTransferred::new(game, self.faction(game));
            e.resources = amount;
            return Some(Rc::new(e));
        }

        None
    }

    pub fn determine_discovery_entered(&self, game: &Game) -> Rc<DiscoveryEntered> {
        let to = DiscoveryEntered::valid_targets(game, self.player).next();
        // ValidSources(Player, null) throws at once: ds.AttachedToLocation.
        let me = game.player(self.player);
        let mut battalions_to_move: NetDictionary<Location, Battalion> = NetDictionary::new();
        for l in DiscoveryEntered::valid_sources(game, self.player, to.nre()) {
            battalions_to_move.add(l, me.battalion_in_location(l).nre());
        }
        let mut e = DiscoveryEntered::new(game, self.faction(game));
        e.set_force_locations(game, &battalions_to_move);
        e.set_to(game, to.map(|d| d.location()));
        Rc::new(e)
    }

    pub fn determine_discovery_revealed(&self, game: &Game) -> Rc<DiscoveryRevealed> {
        let mut e = DiscoveryRevealed::new(game, self.faction(game));
        let location = DiscoveryRevealed::get_locations(game, self.player).cs_first();
        e.set_location(game, location.opt());
        Rc::new(e)
    }

    pub fn determine_testing_station_used(&self, game: &Game) -> Option<Rc<TestingStationUsed>> {
        const N: i32 = Map::NUMBER_OF_SECTORS;
        let faction = self.faction(game);
        let ally = self.ally(game);
        let me = game.player(self.player);
        let current_storm_end = (game.sector_in_storm.wrapping_add(game.next_storm_moves)) % N;
        let locations_in_sector: Vec<Location> =
            game.map.locations(false).filter(|l| l.sector(&game.map) == current_storm_end && !l.is_protected_from_storm()).collect();
        let locations_in_sector_minus1: Vec<Location> = game
            .map
            .locations(false)
            .filter(|l| !l.is_protected_from_storm() && l.sector(&game.map) == if current_storm_end == 0 { N } else { current_storm_end - 1 })
            .collect();
        let locations_in_sector_plus1: Vec<Location> = game
            .map
            .locations(false)
            .filter(|l| !l.is_protected_from_storm() && l.sector(&game.map) == if current_storm_end == N { 0 } else { current_storm_end + 1 })
            .collect();

        // C# parity: `b.Faction == Faction && b.Faction == Ally` is always false (no faction is its own ally).
        let my_killed = |locs: &[Location]| {
            sum_i32(locs.iter().map(|l| {
                sum_i32(game.battalions_in(*l).iter().filter(|b| b.faction == faction && b.faction == ally).map(|b| b.total_amount_of_forces()))
            }))
        };
        let my_forces_killed_in_sector = my_killed(&locations_in_sector);
        let my_forces_killed_in_sector_minus1 = my_killed(&locations_in_sector_minus1);
        let my_forces_killed_in_sector_plus1 = my_killed(&locations_in_sector_plus1);

        let my_strongholds = |locs: &[Location]| {
            locs.iter()
                .filter(|l| {
                    l.is_stronghold()
                        && ((me.occupies(game, Some(**l)) && me.any_forces_in(Some(**l)) < 6)
                            || (me.has_ally() && {
                                let a = game.player(self.allied_player(game).nre());
                                a.occupies(game, Some(**l)) && a.any_forces_in(Some(**l)) < 6
                            }))
                })
                .count() as i32
        };
        let my_strongholds_in_sector = my_strongholds(&locations_in_sector);
        let my_strongholds_in_sector_minus1 = my_strongholds(&locations_in_sector_minus1);
        let my_strongholds_in_sector_plus1 = my_strongholds(&locations_in_sector_plus1);

        let enemy_killed = |locs: &[Location]| {
            sum_i32(locs.iter().map(|l| {
                sum_i32(game.battalions_in(*l).iter().filter(|b| b.faction != faction && b.faction != ally).map(|b| b.total_amount_of_forces()))
            }))
        };
        let enemy_forces_killed_in_sector = enemy_killed(&locations_in_sector);
        let enemy_forces_killed_in_sector_minus1 = enemy_killed(&locations_in_sector_minus1);
        let enemy_forces_killed_in_sector_plus1 = enemy_killed(&locations_in_sector_plus1);

        let enemy_strongholds =
            |locs: &[Location]| locs.iter().filter(|l| l.is_stronghold() && self.occupied_by_opponent_location(game, **l)).count() as i32;
        let enemy_strongholds_in_sector = enemy_strongholds(&locations_in_sector);
        let enemy_strongholds_in_sector_minus1 = enemy_strongholds(&locations_in_sector_minus1);
        let enemy_strongholds_in_sector_plus1 = enemy_strongholds(&locations_in_sector_plus1);

        let score_if_minus1 = enemy_forces_killed_in_sector_minus1 + my_strongholds_in_sector_minus1 * 3
            - my_forces_killed_in_sector_minus1
            - enemy_strongholds_in_sector_minus1 * 3;
        let score_if_passed = score_if_minus1 + enemy_forces_killed_in_sector + my_strongholds_in_sector * 3
            - my_forces_killed_in_sector
            - enemy_strongholds_in_sector * 3;
        let score_if_plus1 = score_if_passed + enemy_forces_killed_in_sector_plus1 + my_strongholds_in_sector_plus1 * 3
            - my_forces_killed_in_sector_plus1
            - enemy_strongholds_in_sector_plus1 * 3;

        if score_if_minus1 > score_if_passed && score_if_minus1 > score_if_plus1 {
            let mut e = TestingStationUsed::new(game, faction);
            e.value_added = -1;
            return Some(Rc::new(e));
        }
        if score_if_plus1 > score_if_passed && score_if_plus1 > score_if_minus1 {
            let mut e = TestingStationUsed::new(game, faction);
            e.value_added = 1;
            return Some(Rc::new(e));
        }

        None
    }

    pub fn determine_divide_resources(&self, game: &Game) -> Rc<DivideResources> {
        let tbd = DivideResources::get_resources_to_be_divided(game);
        let spice_i_want = std::cmp::max(0, tbd.nre().amount.wrapping_sub(self.resources(game)));
        let mut e = DivideResources::new(game, self.faction(game));
        e.portion_to_first_player = spice_i_want;
        Rc::new(e)
    }

    pub fn determine_card_given(&self, game: &Game) -> Rc<CardGiven> {
        let mut e = CardGiven::new(game, self.faction(game));
        let card = game.card_that_must_be_kept_or_given_to_ally.or_null();
        let ally = self.ally(game);
        e.passed = !(self.card_quality(game, card, Some(self.player)) <= 2
            && (ally == Faction::BeneGesserit || ally == Faction::Choam || self.card_quality(game, card, self.allied_player(game)) > 0));
        Rc::new(e)
    }

    pub fn determine_divide_resources_accepted(&self, game: &Game) -> Rc<DivideResourcesAccepted> {
        let tbd = DivideResources::get_resources_to_be_divided(game);

        let portion = game.current_division_proposal.as_ref().nre().portion_to_first_player;
        let i_get_when_agreed = DivideResources::gained_by_other_faction(tbd.as_ref(), true, portion);
        let portion = game.current_division_proposal.as_ref().nre().portion_to_first_player;
        let i_get_when_denied = DivideResources::gained_by_other_faction(tbd.as_ref(), false, portion);

        let accepted = i_get_when_agreed >= i_get_when_denied || self.resources(game) >= game.player(self.player).allied_resources(game);

        let mut e = DivideResourcesAccepted::new(game, self.faction(game));
        e.passed = !accepted;
        Rc::new(e)
    }

    pub fn determine_discarded(&self, game: &Game) -> Rc<Discarded> {
        // OrderBy(...).First(): keys in element order, stable sort, first.
        let mut keyed: Vec<(i32, TreacheryCard)> =
            game.player(self.player).treachery_cards.iter().map(|c| (self.card_quality(game, *c, None), *c)).collect();
        keyed.sort_by(|a, b| a.0.cmp(&b.0));
        let worst_card = keyed.into_iter().map(|(_, c)| c).cs_first();
        let mut e = Discarded::new(game, self.faction(game));
        e.set_card(worst_card.opt());
        Rc::new(e)
    }

    pub fn determine_traitor_discarded(&self, game: &Game) -> Rc<TraitorDiscarded> {
        let mut worst_traitor = self.determine_worst_traitor(game);
        if worst_traitor.is_none() {
            worst_traitor = game.player(self.player).traitors.iter().copied().lowest_or_default(&self.random, |t| t.value());
        }
        let mut e = TraitorDiscarded::new(game, self.faction(game));
        e.set_traitor(worst_traitor);
        Rc::new(e)
    }

    pub fn determine_worst_traitor(&self, game: &Game) -> Option<Hero> {
        let me = game.player(self.player);
        let mut result =
            me.traitors.iter().copied().filter(|t| me.revealed_traitors.contains(t)).highest_or_default(&self.random, |t| t.value());

        if result.is_none() && (self.ally(game) == Faction::Tleilaxu || !game.is_playing(Faction::Tleilaxu)) {
            let faction = self.faction(game);
            result = me.traitors.iter().copied().filter(|t| t.faction() == faction).lowest_or_default(&self.random, |t| t.value());
        }

        if result.is_none() {
            result = me
                .traitors
                .iter()
                .copied()
                .filter(|t| t.as_leader().is_some() && !game.is_alive(*t))
                .lowest_or_default(&self.random, |t| t.value());
        }
        if result.is_none() {
            result = me.traitors.iter().copied().filter(|t| t.as_leader().is_some()).lowest_or_default(&self.random, |t| t.value());
        }

        result
    }

    pub fn determine_nexus_played(&mut self, game: &Game) -> Option<Rc<NexusPlayed>> {
        let mut result = NexusPlayed::new(game, self.faction(game));
        result.faction = game.player(self.player).nexus;

        if NexusPlayed::can_use_cunning(game, self.player) {
            return self.determine_nexus_played_cunning(game, result);
        }

        if NexusPlayed::can_use_secret_ally(game, self.player) {
            self.determine_nexus_played_secret_ally(game, result)
        } else {
            self.determine_nexus_played_betrayal(game, result)
        }
    }

    pub fn i_wish_to_attack(&self, game: &Game, max_number_of_contested_strongholds: i32, f: Faction) -> bool {
        self.almost_winning_opponents_i_wish_to_attack(game, max_number_of_contested_strongholds, true)
            .into_iter()
            .map(|p| game.player(p).faction)
            .any(|x| x == f)
    }

    pub fn determine_nexus_played_betrayal(&mut self, game: &Game, mut result: NexusPlayed) -> Option<Rc<NexusPlayed>> {
        if self.ally(game) == result.faction {
            return None;
        }

        let nexus = game.player(self.player).nexus;
        match nexus {
            Faction::Atreides => {
                if self.is_winning_or_is_opponent_in_battle(game, Faction::Atreides)
                    && !game.prevented(FactionAdvantage::AtreidesBattlePlanPrescience)
                {
                    return Some(Rc::new(result));
                }
            }

            Faction::Harkonnen => {
                if game.current_battle.as_ref().nre().is_involved(game, self.player) {
                    return Some(Rc::new(result));
                }
            }

            Faction::Fremen => {
                if (game.current_main_phase == MainPhase::Blow && FremenRidesMonster::is_applicable(game))
                    || (game.current_main_phase == MainPhase::ShipmentAndMove && self.i_wish_to_attack(game, 0, Faction::Fremen))
                {
                    return Some(Rc::new(result));
                }
            }

            Faction::Emperor => {
                let bid_case = game.current_main_phase == MainPhase::Bidding
                    && game.current_bid.is_some()
                    && {
                        let bid = game.current_bid.as_ref().nre();
                        game.player(bid.i_bid_player().nre()).ally == Faction::Emperor
                            && bid.i_bid_total_amount() > 5
                            && !game.prevented(FactionAdvantage::EmperorReceiveBid)
                    };
                if bid_case
                    || (game.current_main_phase == MainPhase::Battle
                        && self.is_winning_or_is_opponent_in_battle(game, Faction::Emperor)
                        && game
                            .player(game.get_player(Some(Faction::Emperor)).nre())
                            .special_forces_in_territory(game.current_battle.as_ref().nre().territory(game))
                            >= 3)
                {
                    return Some(Rc::new(result));
                }
            }

            Faction::Guild => {
                if game.recently_paid_total_amount() > 5 {
                    return Some(Rc::new(result));
                }
            }

            Faction::BeneGesserit => {
                if self.is_winning_or_is_opponent_in_battle(game, Faction::BeneGesserit)
                    && !game.prevented(FactionAdvantage::BeneGesseritUsingVoice)
                {
                    return Some(Rc::new(result));
                }
            }

            Faction::Ixian => return Some(Rc::new(result)),

            Faction::Tleilaxu => {
                if game.current_battle.as_ref().nre().is_involved(game, self.player) {
                    return Some(Rc::new(result));
                }
            }

            Faction::Choam => {
                if self.is_winning_or_is_opponent_in_battle(game, Faction::Choam)
                    && Self::known_opponent_cards(game, self.player, game.get_player(Some(Faction::Choam)).or_null())
                        .iter()
                        .any(|c| c.is_weapon() || c.is_defense())
                {
                    return Some(Rc::new(result));
                }
            }

            Faction::Richese => {
                let richese = game.get_player(Some(Faction::Richese)).or_null();
                if game.player(richese).has(game.card_just_won) {
                    if self.card_quality(game, game.card_just_won.or_null(), Some(richese)) > 2 {
                        return Some(Rc::new(result));
                    }
                } else if game.recently_paid_total_amount() > 4 {
                    return Some(Rc::new(result));
                }
            }

            Faction::Ecaz => {
                if game.current_phase == Phase::ResurrectionReport {
                    let territory_to_remove_ecaz_from = self.determine_territory_to_remove_ecaz_from(game);
                    if territory_to_remove_ecaz_from.is_some() {
                        result.set_ecaz_territory(game, territory_to_remove_ecaz_from);
                        return Some(Rc::new(result));
                    }
                }
            }

            Faction::Moritani => {
                let faction = self.faction(game);
                if (faction == Faction::Guild && game.current_phase == Phase::GuildShip && !game.guild_may_delay())
                    || (faction != Faction::Guild
                        && game.current_phase == Phase::NonGuildShip
                        && game.shipment_and_move_sequence.as_ref().nre().current_player(game) == Some(self.player))
                {
                    let shipment = self.determine_shipment(game);
                    let territory = shipment.to(game).nre().territory();
                    if game.terror_in(territory).next().is_some() {
                        result.set_moritani_territory(game, Some(territory));
                        return Some(Rc::new(result));
                    }
                }
            }

            _ => {}
        }

        None
    }

    pub fn determine_territory_to_remove_ecaz_from(&self, game: &Game) -> Option<Territory> {
        let mut result: Option<Territory> = None;
        let territories_with_ecaz_and_ecaz_ally: Vec<Territory> =
            NexusPlayed::valid_ecaz_territories(game).filter(|t| t.is_stronghold() || game.is_special_stronghold(*t)).collect();

        if territories_with_ecaz_and_ecaz_ally.is_empty() {
            return result;
        }

        let ecaz = game.get_player(Some(Faction::Ecaz)).or_null();
        let forces = |t: &Territory| game.player(ecaz).any_forces_in_territory(Some(*t));

        if territories_with_ecaz_and_ecaz_ally.len() >= 2 {
            result = territories_with_ecaz_and_ecaz_ally.iter().copied().highest_or_default(&self.random, forces);
        }

        if result.is_none() {
            result = territories_with_ecaz_and_ecaz_ally.iter().copied().filter(|t| forces(t) > 7).highest_or_default(&self.random, forces);
        }

        result
    }

    pub fn is_winning_or_is_opponent_in_battle(&self, game: &Game, faction: Faction) -> bool {
        let Some(battle) = game.current_battle.as_ref() else { return false };
        faction != self.ally(game)
            && ((battle.is_involved_faction(game, faction) && self.is_winning_faction(game, faction))
                || battle.opponent_of_faction(game, faction) == Some(self.player))
    }

    pub fn determine_nexus_played_cunning(&mut self, game: &Game, mut result: NexusPlayed) -> Option<Rc<NexusPlayed>> {
        let nexus = game.player(self.player).nexus;
        match nexus {
            Faction::Atreides => {
                if let Some(prescience) = game.current_prescience.as_ref() {
                    let battle = game.current_battle.as_ref().nre();
                    let opponent = battle.opponent_of_faction(game, self.faction(game));

                    if !(Voice::may_use_voice(game, opponent.or_null())
                        && game.current_voice.is_none()
                        && battle.plan_of(game, opponent).is_none())
                    {
                        if battle.is_aggressor_or_defender(game, self.player) {
                            let territory = battle.territory(game);
                            let max_dial = Self::max_dial(game, self.player, territory.or_null(), opponent, false);
                            result.atreides_prescience_aspect =
                                self.best_prescience(game, self.player, opponent.or_null(), max_dial, prescience.aspect, territory.or_null());

                            if result.atreides_prescience_aspect != PrescienceAspect::None {
                                return Some(Rc::new(result));
                            }
                        }
                    }
                }
            }

            Faction::Harkonnen => {
                if game.current_phase == Phase::ShipmentAndMoveConcluded && self.determine_worst_traitor(game).is_some() {
                    return Some(Rc::new(result));
                }
            }

            Faction::Fremen => {
                if !game.monsters.is_empty() && self.determine_moved_battalion(game, true).is_some() {
                    return Some(Rc::new(result));
                }
            }

            Faction::Emperor => {
                let t = game.current_battle.as_ref().nre().territory(game);
                if game.player(self.player).forces_in_territory(t) >= 4 {
                    return Some(Rc::new(result));
                }
            }

            Faction::Guild => {
                let shipment = self.determine_shipment(game);
                if !shipment.passed
                    && matches!(
                        self.decided_shipment_action,
                        ShipmentDecision::PreventNormalWin | ShipmentDecision::PreventFremenWin | ShipmentDecision::AttackWeakStronghold
                    )
                {
                    return Some(Rc::new(result));
                }
            }

            Faction::BeneGesserit => {
                let flip = self.determine_bene_gesserit_battle_announcement(game);
                if flip.is_some() {
                    return Some(Rc::new(result));
                }
            }

            Faction::Ixian => {
                let t = game.current_battle.as_ref().nre().territory(game);
                if game.player(self.player).forces_in_territory(t) >= 3 {
                    return Some(Rc::new(result));
                }
            }

            Faction::Tleilaxu => {
                if game.current_main_phase == MainPhase::Battle && !game.player(self.player).revealed_face_dancers.is_empty() {
                    return Some(Rc::new(result));
                }
            }

            Faction::Ecaz => {
                if game.current_phase == Phase::BattlePhase
                    && game.current_battle.is_some()
                    && game.current_battle.as_ref().nre().is_aggressor_or_defender(game, self.player)
                    && !game.vidal().is_some_and(|v| game.player(self.player).leaders.contains(&v))
                {
                    return Some(Rc::new(result));
                }
            }

            _ => {}
        }

        None
    }

    pub fn determine_nexus_played_secret_ally(&mut self, game: &Game, mut result: NexusPlayed) -> Option<Rc<NexusPlayed>> {
        let nexus = game.player(self.player).nexus;
        match nexus {
            Faction::Atreides => {
                let battle = game.current_battle.as_ref().nre();
                let opponent = battle.opponent_of_faction(game, self.faction(game));
                if !(Voice::may_use_voice(game, opponent.or_null()) && game.current_voice.is_none() && battle.plan_of(game, opponent).is_none()) {
                    if battle.is_aggressor_or_defender(game, self.player) {
                        let territory = battle.territory(game);
                        let max_dial = Self::max_dial(game, self.player, territory.or_null(), opponent, false);
                        result.atreides_prescience_aspect =
                            self.best_prescience(game, self.player, opponent.or_null(), max_dial, PrescienceAspect::None, territory.or_null());
                        if result.atreides_prescience_aspect != PrescienceAspect::None {
                            return Some(Rc::new(result));
                        }
                    }
                }
            }

            Faction::Harkonnen => {
                if self.determine_worst_traitor(game).is_some() {
                    return Some(Rc::new(result));
                }
            }

            Faction::Fremen => {
                if game.current_main_phase == MainPhase::Resurrection && game.player(self.player).forces_killed >= 3 {
                    return Some(Rc::new(result));
                }
            }

            Faction::Guild => {
                let shipment = self.determine_shipment(game);
                if !shipment.passed && Shipment::determine_cost(game, self.player, &shipment) > 5 {
                    return Some(Rc::new(result));
                }
            }

            Faction::BeneGesserit => {
                if let Some(battle) = game.current_battle.as_ref() {
                    if battle.is_involved(game, self.player) {
                        return Some(Rc::new(result));
                    }
                }
            }

            Faction::Ixian => {
                if self.card_quality(game, game.card_just_won.or_null(), Some(self.player)) < 2 {
                    return Some(Rc::new(result));
                }
            }

            Faction::Tleilaxu => {
                if game.has_acted_or_passed.contains(&self.faction(game)) {
                    let mut amount_of_special_forces = 0;
                    let mut amount_of_forces = 0;
                    let resources = self.resources(game);

                    while amount_of_special_forces + 1 < NexusPlayed::valid_tleilaxu_max_amount(game, self.player, true)
                        && NexusPlayed::determine_tleilaxu_cost_int32_int32(0, amount_of_special_forces + 1) < resources
                        && amount_of_special_forces + 1 + amount_of_forces <= 5
                    {
                        amount_of_special_forces += 1;
                    }

                    while amount_of_forces + 1 < NexusPlayed::valid_tleilaxu_max_amount(game, self.player, false)
                        && NexusPlayed::determine_tleilaxu_cost_int32_int32(0, amount_of_forces + 1) < resources
                        && amount_of_special_forces + amount_of_forces + 1 <= 5
                    {
                        amount_of_forces += 1;
                    }

                    let hero = NexusPlayed::valid_tleilaxu_heroes(game, self.player).into_iter().highest_or_default(&self.random, |l| l.value());
                    if (hero.is_some() && amount_of_forces + amount_of_special_forces > 2) || amount_of_forces + amount_of_special_forces >= 5 {
                        result.tleilaxu_forces = amount_of_forces;
                        result.tleilaxu_special_forces = amount_of_special_forces;
                        result.set_tleilaxu_hero(hero);
                        result.tleilaxu_assign_skill = Revival::may_assign_skill(game, self.player, hero);

                        result.tleilaxu_number_of_special_forces_in_location =
                            Revival::number_of_special_forces_that_may_be_placed_on_planet(game, self.player, amount_of_special_forces);
                        if result.tleilaxu_number_of_special_forces_in_location > 0 {
                            let locations: Vec<Location> = Revival::valid_revived_force_locations(game, self.player).collect();
                            let location = self.best_revival_location(game, &locations);
                            result.set_tleilaxu_location(game, location);
                            if location.is_none() {
                                result.tleilaxu_number_of_special_forces_in_location = 0;
                            }
                        }

                        return Some(Rc::new(result));
                    }
                }
            }

            Faction::Choam => {
                if game.current_main_phase == MainPhase::Collection {
                    if self.faction(game) != Faction::BeneGesserit {
                        let card = NexusPlayed::valid_choam_cards(game, self.player).next().and_then(CsNull::opt);
                        result.set_choam_card(card);
                        return Some(Rc::new(result));
                    }
                } else if game.current_main_phase == MainPhase::Battle {
                    let battle = game.current_battle.as_ref().nre();
                    let auditee = battle.opponent_of(game, self.player);
                    let recent_battle_plan = battle.plan_of(game, auditee);
                    let me = game.player(self.player);
                    // Lazy Where: the plan is read only for cards that reach the lambda.
                    let any = game.player(auditee.nre()).treachery_cards.iter().copied().any(|c| {
                        let plan = recent_battle_plan.as_ref().nre();
                        let auditable = Some(c) != plan.weapon() && Some(c) != plan.defense() && Some(Hero::TreacheryCard(c)) != plan.hero();
                        auditable && !me.known_cards.contains(&c)
                    });
                    if any {
                        return Some(Rc::new(result));
                    }
                }
            }

            Faction::Ecaz => {
                if game.current_phase == Phase::BattlePhase
                    && game.current_battle.is_some()
                    && game.current_battle.as_ref().nre().is_aggressor_or_defender(game, self.player)
                {
                    let opp = game.current_battle.as_ref().nre().opponent_of(game, self.player).nre();
                    let opp = game.player(opp);
                    if opp.faction != Faction::Tleilaxu && opp.unrevealed_traitors().next().is_some() {
                        result.ecaz_faction = opp.faction;
                        return Some(Rc::new(result));
                    }
                }
            }

            _ => {}
        }

        None
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

        if self.last_turn(game) || forces_killed.wrapping_add(special_forces_that_can_be_revived) >= 6 {
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
        let mut to_return: Vec<TreacheryCard> = Vec::new();
        let mut keyed: Vec<(i32, TreacheryCard)> =
            game.player(self.player).treachery_cards.iter().map(|c| (self.card_quality(game, *c, Some(self.player)), *c)).collect();
        keyed.sort_by(|a, b| a.0.cmp(&b.0));
        for (_, c) in keyed {
            if to_return.len() as i32 == game.karma_hand_swap_number_of_cards {
                break;
            }

            to_return.push(c);
        }

        let mut e = KarmaHandSwap::new(game, self.faction(game));
        e.set_returned_cards(&to_return);
        Rc::new(e)
    }

    pub fn determine_karma_shipment_prevention(&self, game: &Game) -> Option<Rc<KarmaShipmentPrevention>> {
        if game.current_phase != Phase::NonGuildShip {
            return None;
        }

        let valid_targets: Vec<Faction> = KarmaShipmentPrevention::get_valid_targets(game, self.player).collect();

        let candidates: Vec<PlayerIndex> = self
            .opponents_to_ship_and_move(game)
            .into_iter()
            .filter(|p| valid_targets.contains(&game.player(*p).faction))
            .filter(|p| {
                let pl = game.player(*p);
                // `p.Resources + p.AlliedPlayer?.Resources > 2`: false when there is no ally.
                self.is_winning_opponent(game, *p)
                    && pl.forces_in_reserve().wrapping_add(pl.special_forces_in_reserve()) > 2
                    && pl.allied_player(game).is_some_and(|a| pl.resources.wrapping_add(game.player(a).resources) > 2)
            })
            .collect();
        // OrderByDescending is stable: sort with the comparison reversed.
        let mut keyed: Vec<(i32, PlayerIndex)> = candidates
            .into_iter()
            .map(|p| {
                let pl = game.player(p);
                let k = std::cmp::min(
                    pl.forces_in_reserve().wrapping_add(pl.special_forces_in_reserve()),
                    pl.resources.wrapping_add(if pl.allied_player(game).is_some() { pl.allied_resources(game) } else { 0 }),
                );
                (k, p)
            })
            .collect();
        keyed.sort_by(|a, b| b.0.cmp(&a.0));
        let winning_opponent_that_can_ship_most = keyed.into_iter().map(|(_, p)| p).next();

        if let Some(w) = winning_opponent_that_can_ship_most {
            if game.shipment_and_move_sequence.as_ref().nre().current_player(game) == Some(w) {
                let mut e = KarmaShipmentPrevention::new(game, self.faction(game));
                e.target = game.player(w).faction;
                return Some(Rc::new(e));
            }
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

        let good_cards = |o: PlayerIndex| {
            Self::known_cards_held_by_other_player(game, me, o).iter().filter(|c| self.card_quality(game, **c, Some(me)) >= 3).count() as i32
        };
        let bad_cards = |o: PlayerIndex| {
            Self::known_cards_held_by_other_player(game, me, o).iter().filter(|c| self.card_quality(game, **c, Some(me)) < 3).count() as i32
        };

        let mut best_opponent_to_swap_with = self.opponents(game).into_iter().highest_or_default(&self.random, |o| good_cards(*o));
        // `"..." + player`: Player has no ToString override, so this can't throw.
        Self::log_info("opponent with most known good cards: ", vec![]);

        if let Some(b) = best_opponent_to_swap_with {
            if good_cards(b) >= 2 {
                //Swap with an opponent that 2 or more good cards that I know of
                let n = good_cards(b);
                Self::log_info(&format!("swapping, because number of good cards = {n}"), vec![]);
                let mut e = KarmaHandSwapInitiated::new(game, self.faction(game));
                e.target = game.player(b).faction;
                return Some(Rc::new(e));
            }
        }

        best_opponent_to_swap_with = self.opponents(game).into_iter().find(|o| game.player(*o).treachery_cards.len() == 4);
        Self::log_info("opponent with 4 cards: ", vec![]);

        if let Some(b) = best_opponent_to_swap_with {
            if bad_cards(b) <= 2 {
                let n = bad_cards(b);
                Self::log_info(&format!("swapping, because number of known bad cards = {n}"), vec![]);
                //Swap with an opponent that has 4 cards and 2 or less useless cards that I know of
                let mut e = KarmaHandSwapInitiated::new(game, self.faction(game));
                e.target = game.player(b).faction;
                return Some(Rc::new(e));
            }
        }

        None
    }

    pub fn determine_harvester_played(&self, game: &Game) -> Option<Rc<HarvesterPlayed>> {
        if game.current_turn > 1
            && ((game.current_phase == Phase::HarvesterA
                && self.resources_in(game, game.latest_spice_card_a.nre().location().or_null()) > 6)
                || (game.current_phase == Phase::HarvesterB
                    && self.resources_in(game, game.latest_spice_card_b.nre().location().or_null()) > 6))
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
        let im_in_battle = game.current_phase == Phase::BattlePhase
            && game.current_battle.is_some()
            && game.current_battle.as_ref().nre().is_aggressor_or_defender(game, self.player);
        let mut wait_for_prescience_or_voice = false;
        if im_in_battle {
            let battle = game.current_battle.as_ref().nre();
            let opponent = battle.opponent_of(game, self.player);
            wait_for_prescience_or_voice = battle.plan_of(game, opponent).is_none()
                && (Prescience::may_use_prescience(game, opponent.or_null()) || Voice::may_use_voice(game, opponent.or_null()));
        }

        if im_in_battle && !wait_for_prescience_or_voice && !same_battle(&game.latest_clairvoyance_battle, &game.current_battle) {
            let battle = game.current_battle.as_ref().nre();
            let opponent = battle.opponent_of(game, self.player).or_null();

            if Self::nr_of_unknown_opponent_cards(game, self.player, opponent) > 0 {
                // `"..." + opponent`: Player has no ToString override.
                Self::log_info("Start using Clairvoyance against ", vec![]);

                let territory = battle.territory(game);
                let my_weapons: Vec<TreacheryCard> =
                    Battle::valid_weapons(game, self.player, None, None, territory, false).into_iter().map(OrNull::or_null).collect();
                // `Game.KnownCards(Player)` runs per element in the lambda; it is pure, so once is enough,
                // but only when an element reaches it.
                let mut known: Option<Vec<TreacheryCard>> = None;
                let enemy_defenses: Vec<TreacheryCard> = Battle::valid_defenses(game, opponent, None, territory, false)
                    .into_iter()
                    .map(OrNull::or_null)
                    .filter(|w| known.get_or_insert_with(|| game.known_cards(self.player)).contains(w))
                    .collect();

                if !self.my_prescience(game).is_some_and(|p| p.aspect == PrescienceAspect::Defense)
                    && my_weapons.iter().all(|w| w.type_() != TreacheryCardType::ProjectileAndPoison)
                {
                    if my_weapons.iter().any(|w| w.is_poison_weapon())
                        && !self.opponent_may_not_use(game, TreacheryCardType::PoisonDefense, false)
                        && !enemy_defenses.iter().any(|w| w.is_poison_defense())
                    {
                        return Some(self.use_clairvoyance_in_battle(
                            game,
                            game.player(opponent).faction,
                            ClairvoyanceQuestion::CardTypeAsDefenseInBattle,
                            TreacheryCardType::PoisonDefense,
                        ));
                    }
                    if my_weapons.iter().any(|w| w.is_projectile_weapon())
                        && !self.opponent_may_not_use(game, TreacheryCardType::ProjectileDefense, false)
                        && !enemy_defenses.iter().any(|w| w.is_projectile_defense())
                    {
                        return Some(self.use_clairvoyance_in_battle(
                            game,
                            game.player(opponent).faction,
                            ClairvoyanceQuestion::CardTypeAsDefenseInBattle,
                            TreacheryCardType::ProjectileDefense,
                        ));
                    }
                }

                let my_defenses: Vec<TreacheryCard> =
                    Battle::valid_defenses(game, self.player, None, territory, false).into_iter().map(OrNull::or_null).collect();

                if !self.my_prescience(game).is_some_and(|p| p.aspect == PrescienceAspect::Weapon)
                    && my_defenses.iter().all(|w| w.type_() != TreacheryCardType::ShieldAndAntidote)
                    && my_defenses.iter().any(|w| w.is_poison_defense())
                    && my_defenses.iter().any(|w| w.is_projectile_defense())
                    && !self.opponent_may_not_use(game, TreacheryCardType::Poison, true)
                    && !self.opponent_may_not_use(game, TreacheryCardType::Projectile, true)
                {
                    return Some(self.use_clairvoyance_in_battle(
                        game,
                        game.player(opponent).faction,
                        ClairvoyanceQuestion::CardTypeAsWeaponInBattle,
                        TreacheryCardType::Poison,
                    ));
                }
            }
        }

        if game.current_phase == Phase::Bidding && !game.player(self.player).has_room_for_cards(game) {
            let me = game.player(self.player);
            // SafeOrKnownTraitorLeaders is a property, evaluated per element; it is pure.
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
                    (pl.traitors.len() as i32).wrapping_sub(pl.revealed_traitors.len() as i32)
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

    pub fn opponent_may_not_use(&self, game: &Game, type_: TreacheryCardType, as_weapon: bool) -> bool {
        let voice = self.my_voice(game);
        voice.is_some_and(|v| v.may_not() && Voice::is_voiced_by(game, as_weapon, false, type_, v.type_))
    }

    pub fn use_clairvoyance_in_battle(&self, game: &Game, opponent: Faction, question: ClairvoyanceQuestion, card_type: TreacheryCardType) -> Rc<ClairVoyancePlayed> {
        let mut e = ClairVoyancePlayed::new(game, self.faction(game));
        e.target = opponent;
        e.question = question;
        e.question_parameter1 = Some(card_type.to_string());
        Rc::new(e)
    }

    pub fn determine_clair_voyance_answered(&self, game: &Game) -> Rc<ClairVoyanceAnswered> {
        // Outside the try: a null LatestClairvoyance escapes to Do.
        let _ = game.latest_clairvoyance.as_ref().nre().question;
        Self::log_info("DetermineClairVoyanceAnswered() {0}", vec![]);

        let mut answer = ClairVoyanceAnswer::Unknown;

        let p1 = || game.latest_clairvoyance.as_ref().nre().parameter1(game);
        let my_plan = || -> Rc<Battle> {
            let plan = game.current_battle.as_ref().nre().plan_of(game, Some(self.player));
            let plan = match plan {
                Some(p) => p,
                None => self.determine_battle_plan(game, true),
            };
            let msg = plan.get_battle_plan_message(game).to_cs_string();
            Self::log_info(&format!("My plan will be: {msg}"), vec![]);
            plan
        };
        let in_battle = || game.current_battle.as_ref().is_some_and(|b| b.is_aggressor_or_defender(game, self.player));

        let attempt = cs::catch(|| {
            let me = game.player(self.player);
            match game.latest_clairvoyance.as_ref().nre().question {
                ClairvoyanceQuestion::CardTypeAsDefenseInBattle => {
                    if in_battle() {
                        let plan = my_plan();
                        answer = self.answer(
                            game,
                            plan.defense().is_some()
                                && ClairVoyanceAnswered::is_questioned_by(false, plan.defense().nre().type_(), unbox_enum::<TreacheryCardType>(&p1())),
                        );
                    }
                }

                ClairvoyanceQuestion::CardTypeAsWeaponInBattle => {
                    if in_battle() {
                        let plan = my_plan();
                        answer = self.answer(
                            game,
                            plan.weapon().is_some()
                                && ClairVoyanceAnswered::is_questioned_by(true, plan.weapon().nre().type_(), unbox_enum::<TreacheryCardType>(&p1())),
                        );
                    }
                }

                ClairvoyanceQuestion::CardTypeInBattle => {
                    if in_battle() {
                        let plan = my_plan();
                        answer = self.answer(
                            game,
                            (plan.defense().is_some()
                                && ClairVoyanceAnswered::is_questioned_by(false, plan.defense().nre().type_(), unbox_enum::<TreacheryCardType>(&p1())))
                                || (plan.weapon().is_some()
                                    && ClairVoyanceAnswered::is_questioned_by(true, plan.weapon().nre().type_(), unbox_enum::<TreacheryCardType>(&p1())))
                                || (plan.hero().is_some_and(|h| h.as_treachery_card().is_some())
                                    && unbox_enum::<TreacheryCardType>(&p1()) == TreacheryCardType::Mercenary),
                        );
                    }
                }

                ClairvoyanceQuestion::DialOfMoreThanXInBattle => {
                    if in_battle() {
                        let plan = my_plan();
                        let opponent = game.current_battle.as_ref().nre().opponent_of(game, self.player).nre();
                        let dial = plan.dial(game, game.player(opponent).faction);
                        answer = self.answer(game, dial > unbox_float(&p1()));
                    }
                }

                ClairvoyanceQuestion::LeaderInBattle => {
                    if in_battle() {
                        let plan = my_plan();
                        answer = self.answer(game, plan.hero() == cast_hero(&p1()));
                    }
                }

                ClairvoyanceQuestion::HasCardTypeInHand => {
                    answer = self.answer(game, me.treachery_cards.iter().any(|c| self.covers(game, c.type_(), p1())));
                }

                ClairvoyanceQuestion::LeaderAsFacedancer => {
                    answer = self.answer(game, me.face_dancers.iter().any(|f| f.is_face_dancer(cast_hero(&p1()).or_null())));
                }

                ClairvoyanceQuestion::LeaderAsTraitor => {
                    answer = self.answer(game, me.traitors.iter().any(|f| f.is_traitor(cast_hero(&p1()).or_null())));
                }

                ClairvoyanceQuestion::Prediction => {
                    let lc = game.latest_clairvoyance.as_ref().nre();
                    answer = self.answer(
                        game,
                        me.predicted_faction == unbox_enum::<Faction>(&lc.parameter1(game)) && me.predicted_turn == unbox_int(&lc.parameter2()),
                    );
                }

                ClairvoyanceQuestion::WillAttackX => {
                    answer = ClairVoyanceAnswer::No;
                }

                _ => {}
            }
        });
        if let Err(e) = attempt {
            Self::log_info(&cs::exception_to_string(&e), vec![]);
        }

        let mut e = ClairVoyanceAnswered::new(game, self.faction(game));
        e.answer = answer;
        Rc::new(e)
    }

    pub fn covers(&self, _game: &Game, type_to_check: TreacheryCardType, covered_by_type: ExpressionElement) -> bool {
        ClairVoyanceAnswered::is_questioned_by(true, type_to_check, unbox_enum::<TreacheryCardType>(&covered_by_type))
            || ClairVoyanceAnswered::is_questioned_by(false, type_to_check, unbox_enum::<TreacheryCardType>(&covered_by_type))
    }

    pub fn answer(&self, _game: &Game, value: bool) -> ClairVoyanceAnswer {
        if value { ClairVoyanceAnswer::Yes } else { ClairVoyanceAnswer::No }
    }

    pub fn determine_raise_dead_played(&self, game: &Game) -> Option<Rc<RaiseDeadPlayed>> {
        let me = game.player(self.player);
        if (game.current_main_phase == MainPhase::Bidding && !me.has_room_for_cards(game)) || game.current_main_phase == MainPhase::ShipmentAndMove {
            let nr_of_living_leaders = me.leaders.iter().filter(|l| game.is_alive(Hero::Leader(**l))).count() as i32;
            let special_forces_that_can_be_revived = std::cmp::min(5, Revival::valid_max_revivals(game, self.player, true, false));

            if (me.forces_killed.wrapping_add(special_forces_that_can_be_revived) >= 7 && nr_of_living_leaders > 1)
                || (me.forces_killed.wrapping_add(special_forces_that_can_be_revived) >= 17 && nr_of_living_leaders > 0)
            {
                let forces = std::cmp::max(0, std::cmp::min(5, me.forces_killed) - special_forces_that_can_be_revived);

                let mut target_of_forces: Option<Location> = None;
                let mut special_forces_to_planet =
                    Revival::number_of_special_forces_that_may_be_placed_on_planet(game, self.player, special_forces_that_can_be_revived);
                if special_forces_to_planet > 0 {
                    let locations: Vec<Location> = Revival::valid_revived_force_locations(game, self.player).collect();
                    target_of_forces = self.best_revival_location(game, &locations);
                    if target_of_forces.is_none() {
                        special_forces_to_planet = 0;
                    }
                }

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

            let mut safe: Option<Vec<Hero>> = None;
            let mut leader_to_revive = RaiseDeadPlayed::valid_heroes(game, self.player)
                .into_iter()
                .filter(|l| {
                    safe.get_or_insert_with(|| self.safe_or_known_traitor_leaders(game)).contains(l)
                        && l.faction() != ally
                        && l.value() >= minimum_value
                })
                .highest_or_default(&self.random, |l| l.value().wrapping_add(self.hero_revival_penalty(game, *l)));
            if leader_to_revive.is_none() {
                leader_to_revive = RaiseDeadPlayed::valid_heroes(game, self.player)
                    .into_iter()
                    .filter(|l| l.faction() != ally && l.value() >= minimum_value)
                    .highest_or_default(&self.random, |l| l.value().wrapping_add(self.hero_revival_penalty(game, *l)));
            }

            let leader_to_revive = leader_to_revive?;

            let assign_skill = Revival::may_assign_skill(game, self.player, Some(leader_to_revive));
            let mut e = RaiseDeadPlayed::new(game, self.faction(game));
            e.set_hero(Some(leader_to_revive));
            e.amount_of_forces = 0;
            e.amount_of_special_forces = 0;
            e.assign_skill = assign_skill;
            return Some(Rc::new(e));
        }

        None
    }

    pub fn determine_amal_played(&self, game: &Game) -> Option<Rc<AmalPlayed>> {
        let opponents_resources = || sum_i32(self.opponents(game).into_iter().map(|p| game.player(p).resources));
        if self.faction(game) == Faction::Guild {
            let ally_resources = if self.ally(game) != Faction::None { game.player(self.player).allied_resources(game) } else { 0 };

            if game.current_phase == Phase::Resurrection && opponents_resources() > 2 * (self.resources(game) + ally_resources) {
                return Some(Rc::new(AmalPlayed::new(game, self.faction(game))));
            }
        } else {
            let ally_resources = if self.ally(game) != Faction::None { game.player(self.player).allied_resources(game) } else { 0 };

            if game.current_turn > 1
                && game.current_main_phase == MainPhase::Bidding
                && opponents_resources() > 10
                && opponents_resources() > (self.opponents(game).len() as i32 + 1) * (self.resources(game) + ally_resources)
            {
                return Some(Rc::new(AmalPlayed::new(game, self.faction(game))));
            }
        }

        None
    }

    pub fn determine_recruits_played(&self, game: &Game) -> Option<Rc<RecruitsPlayed>> {
        let me = game.player(self.player);
        if (me.forces_killed + me.special_forces_killed >= 6 && self.resources(game) >= 14)
            || (me.forces_killed + me.special_forces_killed >= 4 && game.free_revivals(self.player, false) >= 2)
        {
            return Some(Rc::new(RecruitsPlayed::new(game, self.faction(game))));
        }

        None
    }

    pub fn determine_metheor_played(&self, game: &Game) -> Option<Rc<MetheorPlayed>> {
        let faction = self.faction(game);
        let ally = self.ally(game);
        let others = |l: Location| {
            sum_i32(game.battalions_in(l).iter().filter(|b| b.faction != faction && b.faction != ally).map(|b| b.total_amount_of_forces()))
        };
        let mine = |l: Location| {
            sum_i32(game.battalions_in(l).iter().filter(|b| b.faction == faction || b.faction == ally).map(|b| b.total_amount_of_forces()))
        };
        let other_forces_in_arrakeen = others(game.map.arrakeen());
        let mine_and_allied_forces_in_arrakeen = mine(game.map.arrakeen());

        let other_forces_in_carthag = others(game.map.carthag());
        let mine_and_allied_forces_in_carthag = mine(game.map.carthag());

        if other_forces_in_arrakeen + other_forces_in_carthag > 2 * (mine_and_allied_forces_in_arrakeen + mine_and_allied_forces_in_carthag) {
            return Some(Rc::new(MetheorPlayed::new(game, faction)));
        }
        None
    }

    pub fn determine_storm_dialled(&self, game: &Game) -> Rc<StormDialled> {
        let min = StormDialled::valid_min_amount(game);
        let max = StormDialled::valid_max_amount(game);
        let mut e = StormDialled::new(game, self.faction(game));
        e.amount = min.wrapping_add(self.d(game, 1, 1i32.wrapping_add(max).wrapping_sub(min))).wrapping_sub(1);
        Rc::new(e)
    }

    pub fn determine_traitors_selected(&self, game: &Game) -> Rc<TraitorsSelected> {
        let me = game.player(self.player);
        let faction = self.faction(game);
        let mut traitor = me.traitors.iter().copied().filter(|l| l.faction() != faction).highest_or_default(&self.random, |l| l.value());
        if traitor.is_none() {
            traitor = me.traitors.iter().copied().highest_or_default(&self.random, |l| {
                l.value() - if l.faction() == Faction::Atreides && game.applicable(Rule::AtreidesMessiah) { 2 } else { 0 }
            });
        }
        let mut e = TraitorsSelected::new(game, faction);
        e.set_selected_traitor(traitor);
        Rc::new(e)
    }

    pub fn determine_faction_trade_offered(&self, game: &Game) -> Option<Rc<FactionTradeOffered>> {
        let faction = self.faction(game);
        let m = game.current_trade_offers.iter().cs_single_or_default_where(|matching_offer| matching_offer.target == faction);
        if let Some(m) = m {
            let mut e = FactionTradeOffered::new(game, faction);
            e.target = m.initiator();
            return Some(Rc::new(e));
        }

        None
    }

    pub fn determine_thumper_played(&self, game: &Game) -> Rc<ThumperPlayed> {
        Rc::new(ThumperPlayed::new(game, self.faction(game)))
    }

    pub fn determine_storm_spell_played(&self, game: &Game) -> Option<Rc<StormSpellPlayed>> {
        let faction = self.faction(game);
        let ally = self.ally(game);
        // Dictionary<int, int> with keys 0..10 added in order and never removed: index = key.
        let mut my_kills: Vec<i32> = vec![0];
        let mut enemy_kills: Vec<i32> = vec![0];

        for moves in 1..=10usize {
            let moves_to_check = moves as i32;
            let affected_locations: Vec<Location> = game
                .map
                .locations(false)
                .filter(|l| l.sector(&game.map) == (game.sector_in_storm.wrapping_add(moves_to_check)) % Map::NUMBER_OF_SECTORS && !game.is_protected_from_storm(*l))
                .collect();
            let my_and_ally_forces = sum_i32(affected_locations.iter().map(|l| {
                sum_i32(game.battalions_in(*l).iter().filter(|bat| bat.faction == faction || bat.faction == ally).map(|bat| bat.total_amount_of_forces()))
            }));
            let enemy_forces = sum_i32(affected_locations.iter().map(|l| {
                sum_i32(game.battalions_in(*l).iter().filter(|bat| bat.faction != faction && bat.faction != ally).map(|bat| bat.total_amount_of_forces()))
            }));
            my_kills.push(my_kills[moves - 1] + my_and_ally_forces);
            enemy_kills.push(enemy_kills[moves - 1] + enemy_forces);
        }

        // KeyValuePair is a value type, but the dictionary is never empty.
        let most_effective_move = (0..my_kills.len())
            .map(|k| (k, my_kills[k]))
            .highest_or_default(&self.random, |x| enemy_kills[x.0] - x.1)
            .nre();
        let key = most_effective_move.0;
        Self::log_info(
            "StormSpellPlayed() - Most effective number of moves: {0} sectors with {1} allied and {2} enemy kills.",
            vec![(key as i32).into(), my_kills[key].into(), enemy_kills[key].into()],
        );

        if enemy_kills[key] - my_kills[key] >= 10 - (game.current_turn + if game.player(self.player).has_room_for_cards(game) { 0 } else { 4 }) {
            let mut storm_spell = StormSpellPlayed::new(game, faction);
            storm_spell.move_amount = key as i32;
            return Some(Rc::new(storm_spell));
        }

        None
    }

    pub fn determine_distrans_used(&self, game: &Game) -> Option<Rc<DistransUsed>> {
        if game.current_phase != Phase::WaitingForNextBiddingRound {
            return None;
        }

        let worst_card = DistransUsed::valid_cards(game, self.player).lowest_or_default(&self.random, |c| self.card_quality(game, *c, Some(self.player)));

        let worst_card = worst_card?;
        if self.card_quality(game, worst_card, Some(self.player)) > 1 {
            return None;
        }

        let ally = self.ally(game);
        let target = DistransUsed::valid_targets(game, self.player)
            .filter(|f| *f != ally && (!game.applicable(Rule::BeneGesseritWorthlessAsKarma) || *f != Faction::BeneGesserit))
            .map(|f| game.get_player(Some(f)).or_null())
            .highest_or_default(&self.random, |p| game.number_of_victory_points(*p, true));

        let target = target?;
        let mut e = DistransUsed::new(game, self.faction(game));
        e.set_card(Some(worst_card));
        e.target = game.player(target).faction;
        Some(Rc::new(e))
    }

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

        let mut safe: Option<Vec<Hero>> = None;
        let mut leader_to_revive = Revival::valid_revival_heroes(game, self.player)
            .filter(|l| {
                safe.get_or_insert_with(|| self.safe_or_known_traitor_leaders(game)).contains(l)
                    && Revival::get_price_of_hero_revival(game, self.player, Some(*l)) <= max_to_spend_on_hero_revival
                    && l.faction() != ally
                    && l.value() >= minimum_value
            })
            .highest_or_default(&self.random, |l| l.value().wrapping_add(self.hero_revival_penalty(game, *l)));
        if leader_to_revive.is_none() {
            leader_to_revive = Revival::valid_revival_heroes(game, self.player)
                .filter(|l| {
                    Revival::get_price_of_hero_revival(game, self.player, Some(*l)) <= max_to_spend_on_hero_revival
                        && l.faction() != ally
                        && l.value() >= minimum_value
                })
                .highest_or_default(&self.random, |l| l.value().wrapping_add(self.hero_revival_penalty(game, *l)));
        }

        let use_secret_ally =
            Revival::may_use_emperor_secret_ally(game, self.player) && me.forces_killed - game.free_revivals(self.player, false) >= 3;

        let mut forces_revived_by_emperor = 0;
        let mut special_forces_revived_by_emperor = 0;
        Self::determine_optimal_use_of_emperor_revivals(
            game,
            self.player,
            &mut forces_revived_by_emperor,
            &mut special_forces_revived_by_emperor,
            use_secret_ally,
        );

        let mut special_forces_to_revive = 0;
        while
        //check limit of special forces
        special_forces_revived_by_emperor + special_forces_to_revive + 1 <= Revival::valid_max_revivals(game, self.player, true, use_secret_ally)
            //check if there are enough forces killed
            && special_forces_revived_by_emperor + special_forces_to_revive + 1 <= me.special_forces_killed
            //check if I have enough spice
            && Revival::determine_cost(
                game,
                self.player,
                leader_to_revive,
                0,
                special_forces_to_revive + 1,
                forces_revived_by_emperor,
                special_forces_revived_by_emperor,
                use_secret_ally,
            )
            .total_cost_for_player()
                <= resources
        {
            special_forces_to_revive += 1;
        }

        let mut normal_forces_to_revive = 0;

        while
        //check limit of total amount of forces
        special_forces_to_revive + normal_forces_to_revive + 1 <= Revival::valid_max_revivals(game, self.player, false, use_secret_ally)
            //check if there are enough forces killed
            && forces_revived_by_emperor + normal_forces_to_revive + 1 <= me.forces_killed
            //check if I have enough spice
            && Revival::determine_cost(
                game,
                self.player,
                leader_to_revive,
                normal_forces_to_revive + 1,
                special_forces_to_revive,
                forces_revived_by_emperor,
                special_forces_revived_by_emperor,
                use_secret_ally,
            )
            .total_cost_for_player()
                <= resources
        {
            normal_forces_to_revive += 1;
        }

        let assign_skill = leader_to_revive.is_some() && Revival::may_assign_skill(game, self.player, leader_to_revive);

        let mut target_of_forces: Option<Location> = None;
        let mut forces_to_planet = Revival::number_of_forces_that_may_be_placed_on_planet(
            game,
            self.player,
            use_secret_ally,
            normal_forces_to_revive + forces_revived_by_emperor,
        );
        let mut special_forces_to_planet =
            Revival::number_of_special_forces_that_may_be_placed_on_planet(game, self.player, special_forces_to_revive + special_forces_revived_by_emperor);

        if forces_to_planet > 0 {
            let locations: Vec<Location> = Revival::valid_revived_force_locations(game, self.player).collect();
            target_of_forces = self.best_revival_location(game, &locations);
            if target_of_forces.is_none() {
                forces_to_planet = 0;
            }
        } else if special_forces_to_planet > 0 {
            let locations: Vec<Location> = Revival::valid_revived_force_locations(game, self.player).collect();
            target_of_forces = self.best_revival_location(game, &locations);
            if target_of_forces.is_none() {
                special_forces_to_planet = 0;
            }
        }

        if leader_to_revive.is_some() || special_forces_to_revive + normal_forces_to_revive > 0 {
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
            return Some(Rc::new(e));
        }
        None
    }

    pub fn best_revival_location(&self, game: &Game, valid_locations: &[Location]) -> Option<Location> {
        let me = game.player(self.player);
        let few_or_vacant = |l: Location| self.vacant(game, l) || (me.any_forces_in(Some(l)) > 0 && me.any_forces_in(Some(l)) < 4);
        let mut result = valid_locations
            .iter()
            .copied()
            .find(|l| *l == game.map.carthag() && few_or_vacant(*l))
            .or_else(|| valid_locations.iter().copied().find(|l| *l == game.map.arrakeen() && few_or_vacant(*l)))
            .or_else(|| valid_locations.iter().copied().find(|l| self.resources_in(game, *l) >= 4))
            .or_else(|| valid_locations.iter().copied().find(|l| l.is_stronghold() && few_or_vacant(*l)));

        if result.is_none() && me.forces_in_reserve() + me.special_forces_in_reserve() > 0 {
            result = valid_locations.iter().copied().find(|l| *l == game.map.polar_sink());
        }

        result.and_then(CsNull::opt)
    }

    pub fn determine_optimal_use_of_emperor_revivals(g: &Game, p: PlayerIndex, forces: &mut i32, special_forces: &mut i32, use_cunning: bool) {
        *forces = 0;
        *special_forces = 0;

        let pl = g.player(p);
        if pl.ally != Faction::Emperor {
            return;
        }

        let emperor = g.get_player(Some(Faction::Emperor)).or_null();

        let potential_maximum_by_emperor = if pl.ally == Faction::Emperor
            && !g.prevented(FactionAdvantage::EmperorLetAllyReviveExtraForces)
        {
            Revival::valid_max_revivals_by_emperor(g, p)
        } else {
            0
        };

        let mut max_special_forces = Revival::valid_max_revivals(g, p, true, use_cunning);

        let mut free_revivals = g.free_revivals(p, use_cunning);

        if max_special_forces > 0 && free_revivals > 0 {
            max_special_forces -= 1;
            free_revivals -= 1;
        }

        while *special_forces + 1 <= pl.special_forces_killed
            && *special_forces + 1 <= max_special_forces
            && *special_forces + 1 <= potential_maximum_by_emperor
            && Revival::determine_cost_of_forces_for_emperor(g, emperor, pl.faction, 0, *special_forces + 1) <= g.player(emperor).resources
        {
            *special_forces += 1;
        }

        let mut max_forces = Revival::valid_max_revivals(g, p, false, use_cunning);

        max_forces = std::cmp::max(max_forces - free_revivals, 0);

        while *forces + 1 <= pl.forces_killed
            && *forces + 1 <= max_forces
            && *special_forces + *forces + 1 <= potential_maximum_by_emperor
            && Revival::determine_cost_of_forces_for_emperor(g, emperor, pl.faction, *forces + 1, *special_forces) <= g.player(emperor).resources
        {
            *forces += 1;
        }
    }

    pub fn determine_discarded_searched_announced(&self, game: &Game) -> Option<Rc<DiscardedSearchedAnnounced>> {
        if game.current_main_phase == MainPhase::Contemplate {
            let card_to_search = DiscardedSearched::valid_cards(game)
                .into_iter()
                .highest_or_default(&self.random, |c| self.card_quality(game, *c, Some(self.player)));
            if let Some(c) = card_to_search {
                if self.card_quality(game, c, Some(self.player)) >= 4 {
                    return Some(Rc::new(DiscardedSearchedAnnounced::new(game, self.faction(game))));
                }
            }
        }

        None
    }

    pub fn determine_discarded_searched(&self, game: &Game) -> Rc<DiscardedSearched> {
        let card_to_search = DiscardedSearched::valid_cards(game)
            .into_iter()
            .highest_or_default(&self.random, |c| self.card_quality(game, *c, Some(self.player)));
        let mut e = DiscardedSearched::new(game, self.faction(game));
        e.set_card(card_to_search);
        Rc::new(e)
    }

    pub fn determine_discarded_taken(&self, game: &Game) -> Option<Rc<DiscardedTaken>> {
        let card_to_take =
            DiscardedTaken::valid_cards(game, self.player).highest_or_default(&self.random, |c| self.card_quality(game, *c, Some(self.player)));
        if let Some(c) = card_to_take {
            if self.card_quality(game, c, Some(self.player)) >= 4 {
                let mut e = DiscardedTaken::new(game, self.faction(game));
                e.set_card(Some(c));
                return Some(Rc::new(e));
            }
        }

        None
    }

    pub fn determine_juice_played(&self, game: &Game) -> Option<Rc<JuicePlayed>> {
        if game.current_main_phase == MainPhase::ShipmentAndMove
            && game.current_moment == MainPhaseMoment::Start
            && self.faction(game) != Faction::Guild
            && game.shipment_and_move_sequence.as_ref().nre().get_players_in_sequence(game).last().and_then(|e| e.player) != Some(self.player)
        {
            let mut e = JuicePlayed::new(game, self.faction(game));
            e.type_ = JuiceType::GoLast;
            return Some(Rc::new(e));
        }

        if game.current_main_phase == MainPhase::Battle
            && game.current_moment == MainPhaseMoment::Start
            && !Battle::battles_to_be_fought(game, self.player, false).is_empty()
        {
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
        let opponent_plan = game.current_battle.as_ref().nre().plan_of_opponent(game, Some(self.player));
        let opponent_plan = opponent_plan.nre();
        if let Some(w) = opponent_plan.weapon() {
            if w.countered_by(opponent_plan.defense(), None) {
                let mut e = Diplomacy::new(game, self.faction(game));
                e.set_card(Diplomacy::valid_cards(game, self.player).into_iter().cs_first().opt());
                return Some(Rc::new(e));
            }
        }

        None
    }

    pub fn determine_skill_assigned(&self, game: &Game) -> Rc<SkillAssigned> {
        let me = game.player(self.player);
        let mut skill = LeaderSkill::None;
        let skills: Vec<LeaderSkill> = SkillAssigned::valid_skills(game, self.player);
        // FirstOrDefault(s => s == x) on a value type: default (None) when absent.
        let first = |x: LeaderSkill| skills.iter().copied().find(|s| *s == x).unwrap_or_default();

        if me.treachery_cards.iter().any(|c| c.is_projectile_weapon()) {
            skill = first(LeaderSkill::Swordmaster);
        }
        if skill == LeaderSkill::None && me.treachery_cards.iter().any(|c| c.is_poison_weapon()) {
            skill = first(LeaderSkill::MasterOfAssassins);
        }
        if skill == LeaderSkill::None && me.treachery_cards.iter().any(|c| c.is_projectile_defense()) {
            skill = first(LeaderSkill::Adept);
        }
        if skill == LeaderSkill::None && me.treachery_cards.iter().any(|c| c.is_poison_defense()) {
            skill = first(LeaderSkill::KillerMedic);
        }
        if skill == LeaderSkill::None && me.treachery_cards.iter().any(|c| c.is_useless()) {
            skill = first(LeaderSkill::Warmaster);
        }

        if skill == LeaderSkill::None {
            skill = first(LeaderSkill::Graduate);
        }
        if skill == LeaderSkill::None && !me.is(Faction::Atreides) {
            skill = first(LeaderSkill::Thinker);
        }

        if skill == LeaderSkill::None && me.is(Faction::Fremen) {
            skill = first(LeaderSkill::Sandmaster);
        }
        if skill == LeaderSkill::None && !me.is(Faction::Fremen) {
            skill = first(LeaderSkill::Smuggler);
        }

        let is_economic_faction = me.is(Faction::Emperor) || me.is(Faction::Guild) || me.is(Faction::Choam) || me.is(Faction::Tleilaxu);
        if skill == LeaderSkill::None && !is_economic_faction {
            skill = first(LeaderSkill::Bureaucrat);
        }
        if skill == LeaderSkill::None && !is_economic_faction {
            skill = first(LeaderSkill::Banker);
        }

        if skill == LeaderSkill::None && !me.is(Faction::Tleilaxu) {
            skill = first(LeaderSkill::Decipherer);
        }

        if skill == LeaderSkill::None {
            skill = first(LeaderSkill::Swordmaster);
        }
        if skill == LeaderSkill::None {
            skill = first(LeaderSkill::MasterOfAssassins);
        }
        if skill == LeaderSkill::None {
            skill = first(LeaderSkill::Adept);
        }
        if skill == LeaderSkill::None {
            skill = first(LeaderSkill::KillerMedic);
        }
        if skill == LeaderSkill::None {
            skill = first(LeaderSkill::Warmaster);
        }

        if skill == LeaderSkill::None {
            skill = skills.iter().copied().find(|s| *s != LeaderSkill::Planetologist).unwrap_or_default();
        }

        if skill == LeaderSkill::None {
            skill = skills.iter().copied().cs_first();
        }

        let leader = self.random_item_from(game, &SkillAssigned::valid_leaders(game, self.player));
        let mut e = SkillAssigned::new(game, self.faction(game));
        e.passed = false;
        e.set_leader(leader);
        e.skill = skill;
        Rc::new(e)
    }

    pub fn random_item_from<T: Clone>(&self, game: &Game, items: &[T]) -> T {
        cs::index_array(items, self.d(game, 1, items.len() as i32) - 1).clone()
    }

    pub fn determine_planetology(&self, game: &Game) -> Rc<Planetology> {
        let mut e = Planetology::new(game, self.faction(game));
        e.add_one_to_movement = true;
        Rc::new(e)
    }
}
