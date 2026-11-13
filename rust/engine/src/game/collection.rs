use crate::log::collection::SpiceCollected;
use crate::prelude::*;

/// `params object?[]` element list for `Log(...)`.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

impl Game {
    pub fn start_collection(&mut self) {
        self.collect_resources_from_territories();
        self.collect_resources_from_strongholds_and_homeworlds();
        let condition = !self.collected_resources_to_be_divided.is_empty();
        self.enter_boolean_phase_action(condition, Phase::DividingCollectedResources, Game::end_collection_main_phase);
    }

    pub fn divide_resources_from_collection(&mut self, division_was_agreed: bool) {
        let Some(proposal) = self.current_division_proposal.clone() else { return };

        let to_be_divided = DivideResources::get_resources_to_be_divided(self);

        let gained_by_first_faction =
            DivideResources::gained_by_first_faction(to_be_divided.as_ref(), division_was_agreed, proposal.portion_to_first_player);
        let gained_by_other_faction =
            DivideResources::gained_by_other_faction(to_be_divided.as_ref(), division_was_agreed, proposal.portion_to_first_player);

        let to_be_divided = to_be_divided.nre();
        self.gain_collected_resources(to_be_divided.first_faction, to_be_divided.territory, gained_by_first_faction, None, false);
        self.gain_collected_resources(to_be_divided.other_faction, to_be_divided.territory, gained_by_other_faction, None, false);

        // `Remove(toBeDivided)` removes by reference; `toBeDivided` is the first element.
        self.collected_resources_to_be_divided.remove(0);

        self.current_division_proposal = None;
    }

    // field: the location collected from, when spice was taken from the desert.
    // income: spice paid by a stronghold or an occupied homeworld.
    pub fn gain_collected_resources(&mut self, faction: Faction, from: Territory, amount: i32, field: Option<Location>, income: bool) {
        let field = field.and_then(crate::compat::cs::CsNull::opt);
        let msg = express!(faction, " collect ", Payment::of(amount), " from ", from)
            .with_log(SpiceCollected::new(faction, from, amount, field, income));
        self.log(elems![msg]);
        if let Some(player) = self.get_player(Some(faction)) {
            let r = &mut self.player_mut(player).resources;
            *r += amount;
        }
        if faction == Faction::Fremen {
            self.resources_collected_by_fremen += amount;
        }
        if faction == Faction::Harkonnen && !from.is_stronghold() && !from.is_protected_from_worm() {
            self.resources_collected_by_harkonnen_from_desert_or_homeworld =
                self.resources_collected_by_harkonnen_from_desert_or_homeworld + amount;
        }
    }

    pub fn modify_income_because_of_low_threshold_or_occupation(&mut self, from: PlayerIndex, received_amount: &mut i32) {
        if *received_amount > 1 && self.applicable(Rule::Homeworlds) {
            let homeworld = self.player(from).primary_homeworld();
            let occupier = self.occupier_of(homeworld.world());

            let fp = self.player(from);
            let occupied_or_emperor_or_guild = || occupier.is_some() || fp.is(Faction::Emperor) || fp.is(Faction::Guild);
            let half_is_lost = occupied_or_emperor_or_guild() && fp.has_low_threshold(self, homeworld.world());

            let amount_lost = if half_is_lost { (0.5f32 * *received_amount as f32) as i32 } else { 0 };

            *received_amount -= amount_lost;

            let Some(occupier) = occupier else { return };

            let r = &mut self.player_mut(occupier).resources;
            *r += amount_lost;
            // C# parity: logs "0 ... goes to occupier" whenever there is an occupier (hazards 18.2).
            let from_faction = self.player(from).faction;
            let occupier_faction = self.player(occupier).faction;
            self.log(elems![Payment::of(amount_lost), " received by ", from_faction, " goes to ", occupier_faction, " due to occupation"]);
        }
    }

    pub fn collect_resources_from_strongholds_and_homeworlds(&mut self) {
        if self.applicable(Rule::IncreasedResourceFlow) || self.applicable(Rule::ResourceBonusForStrongholds) {
            for i in 0..self.players.len() {
                let player = self.player_at(i);
                self.collect_resources_from_stronghold(self.map.arrakeen(), player, 2);
                self.collect_resources_from_stronghold(self.map.carthag(), player, 2);
                self.collect_resources_from_stronghold(self.map.tueks_sietch(), player, 1);
                self.collect_resources_from_stronghold(self.map.cistern().location(), player, 2);
            }
        }

        let homeworlds: Vec<Homeworld> = self.map.homeworlds().filter(|hw| hw.resource_amount() != 0).collect();
        for homeworld in homeworlds {
            if let Some(occupier) = self.occupier_of(homeworld.world()) {
                let faction = self.player(occupier).faction;
                self.gain_collected_resources(faction, homeworld.territory(), homeworld.resource_amount(), None, true);
                let r = &mut self.player_mut(occupier).transferable_resources;
                *r += homeworld.resource_amount();
            }
        }
    }

    pub fn collect_resources_from_stronghold(&mut self, stronghold: Location, player: PlayerIndex, amount: i32) {
        let contested = self.applicable(Rule::ContestedStongholdsCountAsOccupied);
        let p = self.player(player);
        if p.controls(self, stronghold, contested)
            && !(p.is(Faction::Ecaz)
                && self.prevented(FactionAdvantage::EcazCollection)
                && p.has_ally()
                && !self.player(p.allied_player(self).nre()).controls(self, stronghold, self.applicable(Rule::ContestedStongholdsCountAsOccupied)))
        {
            let faction = p.faction;
            self.gain_collected_resources(faction, stronghold.territory(), amount, None, true);
        }
    }

    pub fn collect_resources_from_territories(&mut self) {
        let processing_station = self.map.processing_station().location();
        let find_thief = |g: &Game| g.player_indices().find(|p| g.player(*p).occupies(g, Some(processing_station)));

        // `.ToList()`: a snapshot of the pairs, in dictionary order (hazards 3.3).
        let snapshot: Vec<(Location, i32)> = self.resources_on_planet.iter().filter(|(_, v)| **v > 0).map(|(k, v)| (*k, *v)).collect();
        for (key, value) in snapshot {
            let mut thief = find_thief(self);

            let players_to_collect: Vec<PlayerIndex> =
                self.player_indices().filter(|y| self.player(*y).occupies(self, Some(key))).collect();
            let mut total_collected_amount = 0i32;
            let mut spice_left = value;

            for &p in &players_to_collect {
                let collection_rate = self.resource_collection_rate(p);
                let pl = self.player(p);
                let forces_collecting_default_amount_of_spice =
                    if pl.faction != Faction::Ixian { pl.occupying_forces_in_location(key) } else { pl.forces_in(Some(key)) };
                let forces_collecting3_spice = if pl.is(Faction::Ixian) { pl.special_forces_in(Some(key)) } else { 0 };
                let maximum_spice_that_can_be_collected = (forces_collecting_default_amount_of_spice * collection_rate) + (forces_collecting3_spice * 3);
                let mut collected_amount_by_this_player = spice_left.min(maximum_spice_that_can_be_collected);
                self.change_resources_on_planet(key, -collected_amount_by_this_player);
                spice_left -= collected_amount_by_this_player;

                if let Some(t) = thief {
                    if collected_amount_by_this_player > 0 && p != t {
                        collected_amount_by_this_player -= 1;
                        let r = &mut self.player_mut(t).resources;
                        *r += 1;
                        let thief_faction = self.player(t).faction;
                        let p_faction = self.player(p).faction;
                        self.log(elems![
                            thief_faction,
                            " steal ",
                            Payment::of(1),
                            " from the ",
                            Concept::Resource,
                            " collected by ",
                            p_faction,
                            " from ",
                            key.territory()
                        ]);
                        thief = None;
                    }
                }

                if players_to_collect.len() == 1 {
                    let p_faction = self.player(p).faction;
                    self.gain_collected_resources(p_faction, key.territory(), collected_amount_by_this_player, Some(key), false);
                } else {
                    total_collected_amount += collected_amount_by_this_player;
                }

                if spice_left <= 0 {
                    break;
                }
            }

            if players_to_collect.len() > 1 {
                let (first_faction, other_faction) = if players_to_collect.iter().any(|p| self.player(*p).is(Faction::Ecaz)) {
                    let other = players_to_collect.iter().copied().find(|p| !self.player(*p).is(Faction::Ecaz)).expect("a player other than Ecaz");
                    (Faction::Ecaz, self.player(other).faction)
                } else {
                    (self.player(players_to_collect[0]).faction, self.player(players_to_collect[1]).faction)
                };

                // `new ResourcesToBeDivided()` with every field assigned before it is stored.
                let to_be_divided = ResourcesToBeDivided {
                    amount: total_collected_amount,
                    first_faction,
                    other_faction,
                    territory: key.territory(),
                };
                self.collected_resources_to_be_divided.push(to_be_divided);
                self.log(elems![
                    first_faction,
                    " and ",
                    other_faction,
                    " will have to share ",
                    Payment::of(total_collected_amount),
                    " collected from ",
                    key.territory()
                ]);
            }
        }
    }

    pub fn end_collection_main_phase(&mut self) {
        if let Some(fremen) = self.get_player(Some(Faction::Fremen)) {
            let mut received_amount_by_fremen_after_taking_into_account_occupation = self.resources_collected_by_fremen;
            self.modify_income_because_of_low_threshold_or_occupation(fremen, &mut received_amount_by_fremen_after_taking_into_account_occupation);

            let lost = self.resources_collected_by_fremen - received_amount_by_fremen_after_taking_into_account_occupation;
            let r = &mut self.player_mut(fremen).resources;
            *r -= lost;
        }

        let harkonnen = self.get_player(Some(Faction::Harkonnen));
        if self.resources_collected_by_harkonnen_from_desert_or_homeworld != 0 {
            if let Some(harkonnen) = harkonnen.filter(|b| self.player(*b).has_high_threshold_0(self)) {
                let r = &mut self.player_mut(harkonnen).resources;
                *r += 2;
                self.log(elems![Faction::Harkonnen, " get ", Payment::of(2), " extra as they collected from desert or homeworld"]);
            }
        }

        self.main_phase_end();
        self.enter(Phase::CollectionReport, true);
    }

    pub fn resource_collection_rate(&self, p: PlayerIndex) -> i32 {
        let pl = self.player(p);
        if pl.controls(self, self.map.arrakeen(), self.applicable(Rule::ContestedStongholdsCountAsOccupied))
            || pl.controls(self, self.map.carthag(), self.applicable(Rule::ContestedStongholdsCountAsOccupied))
        {
            3
        } else {
            2
        }
    }
}

#[derive(Clone, Debug, Canon)]
pub struct ResourcesToBeDivided {
    pub amount: i32,
    pub first_faction: Faction,
    pub other_faction: Faction,
    pub territory: Territory,
}

// C#: the implicit parameterless constructor leaves `Territory` null until it is
// assigned. Its only caller (`Game.CollectResourcesFromTerritories`) assigns every
// field before the object is stored, so Rust builds it with a struct literal and
// `Territory` stays non-nullable.
