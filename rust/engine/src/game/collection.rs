use crate::log::collection::SpiceCollected;
use crate::prelude::*;
use crate::messages::part::Part;

/// The spice collection phase (`PhaseState::Collection`): its step, the spice collected, and the
/// shared collections still to divide. It is dropped when the phase ends.
pub struct CollectionState {
    pub step: CollectionStep,
    pub resources_collected_by_fremen: i32,
    pub resources_collected_by_harkonnen_from_desert_or_homeworld: i32,
    pub collected_resources_to_be_divided: Vec<ResourcesToBeDivided>,
    pub current_division_proposal: Option<DivisionProposal>,
}

impl CollectionState {
    pub fn new(step: CollectionStep) -> Self {
        CollectionState {
            step,
            resources_collected_by_fremen: 0,
            resources_collected_by_harkonnen_from_desert_or_homeworld: 0,
            collected_resources_to_be_divided: Vec::new(),
            current_division_proposal: None,
        }
    }
}

/// Where the collection phase is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionStep {
    /// Before spice is collected, while the host may still wait for players.
    Beginning,
    /// The first faction of a shared collection proposes how to divide it.
    Dividing,
    /// The other faction answers the proposal.
    Accepting,
    Report,
}

impl CollectionStep {
    pub fn phase(self) -> Phase {
        match self {
            CollectionStep::Beginning => Phase::BeginningOfCollection,
            CollectionStep::Dividing => Phase::DividingCollectedResources,
            CollectionStep::Accepting => Phase::AcceptingResourceDivision,
            CollectionStep::Report => Phase::CollectionReport,
        }
    }

    pub fn of(phase: Phase) -> Option<CollectionStep> {
        match phase {
            Phase::BeginningOfCollection => Some(CollectionStep::Beginning),
            Phase::DividingCollectedResources => Some(CollectionStep::Dividing),
            Phase::AcceptingResourceDivision => Some(CollectionStep::Accepting),
            Phase::CollectionReport => Some(CollectionStep::Report),
            _ => None,
        }
    }
}

/// `params object?[]` element list for `Log(...)`.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(Part::from($e)),*] };
}

/// The division of shared spice that the first faction proposed (`DivideResources`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DivisionProposal {
    pub portion_to_first_player: u16,
}

impl Game {
    /// The collection phase's state, while the game is in it (an interrupt may suspend it).
    pub fn collection(&self) -> Option<&CollectionState> {
        match &self.phase {
            PhaseState::Collection(c) => Some(c),
            _ => None,
        }
    }

    pub fn collection_mut(&mut self) -> Option<&mut CollectionState> {
        match &mut self.phase {
            PhaseState::Collection(c) => Some(c),
            _ => None,
        }
    }

    /// Starts the collection phase (`enter_spice_collection_phase` in end_phase.rs does the rest).
    pub fn begin_collection(&mut self) {
        self.enter_state(PhaseState::Collection(CollectionState::new(CollectionStep::Beginning)));
    }

    /// Moves the collection phase to `step`, dropping any interrupt, as entering a phase did.
    pub fn enter_collection(&mut self, step: CollectionStep) {
        match self.collection_mut() {
            Some(c) => {
                c.step = step;
                self.interrupts.clear();
                self.remove_ended_deals(step.phase());
            }
            None => self.enter_state(PhaseState::Collection(CollectionState::new(step))),
        }
    }

    /// The next shared collection to divide, or the end of the phase.
    pub fn divide_next_collection_or_end(&mut self) {
        if self.collection().is_some_and(|c| !c.collected_resources_to_be_divided.is_empty()) {
            self.enter_collection(CollectionStep::Dividing);
        } else {
            self.end_collection_main_phase();
        }
    }

    pub fn start_collection(&mut self) {
        self.collect_resources_from_territories();
        self.collect_resources_from_strongholds_and_homeworlds();
        self.divide_next_collection_or_end();
    }

    pub fn divide_resources_from_collection(&mut self, division_was_agreed: bool) {
        let Some(proposal) = self.collection().and_then(|c| c.current_division_proposal) else { return };

        let to_be_divided = DivideResources::get_resources_to_be_divided(self);

        let gained_by_first_faction =
            DivideResources::gained_by_first_faction(to_be_divided.as_ref(), division_was_agreed, i32::from(proposal.portion_to_first_player));
        let gained_by_other_faction =
            DivideResources::gained_by_other_faction(to_be_divided.as_ref(), division_was_agreed, i32::from(proposal.portion_to_first_player));

        let to_be_divided = to_be_divided.expect("to be divided");
        self.gain_collected_resources(to_be_divided.first_faction, to_be_divided.territory, gained_by_first_faction, None, false);
        self.gain_collected_resources(to_be_divided.other_faction, to_be_divided.territory, gained_by_other_faction, None, false);

        // `Remove(toBeDivided)` removes by reference; `toBeDivided` is the first element.
        let c = self.collection_mut().expect("in the collection phase");
        c.collected_resources_to_be_divided.remove(0);
        c.current_division_proposal = None;
    }

    // field: the location collected from, when spice was taken from the desert.
    // income: spice paid by a stronghold or an occupied homeworld.
    pub fn gain_collected_resources(&mut self, faction: Faction, from: Territory, amount: i32, field: Option<Location>, income: bool) {
        let msg = express!(faction, " collect ", Payment::of(amount), " from ", from)
            .with_log(SpiceCollected::new(faction, from, amount, field, income));
        self.log(elems![msg]);
        if let Some(player) = self.get_player(Some(faction)) {
            crate::game::amounts::gain(&mut self.player_mut(player).resources, amount, "spice").expect("collected spice");
        }
        let harkonnen_from_desert_or_homeworld = faction == Faction::Harkonnen && !from.is_stronghold() && !from.is_protected_from_worm();
        let c = self.collection_mut().expect("in the collection phase");
        if faction == Faction::Fremen {
            c.resources_collected_by_fremen += amount;
        }
        if harkonnen_from_desert_or_homeworld {
            c.resources_collected_by_harkonnen_from_desert_or_homeworld += amount;
        }
    }

    pub fn modify_income_because_of_low_threshold_or_occupation(&mut self, from: PlayerId, received_amount: &mut i32) {
        if *received_amount > 1 && self.applicable(Rule::Homeworlds) {
            let homeworld = self.player(from).primary_homeworld();
            let occupier = self.occupier_of(homeworld.world());

            let fp = self.player(from);
            let occupied_or_emperor_or_guild = || occupier.is_some() || fp.is(Faction::Emperor) || fp.is(Faction::Guild);
            let half_is_lost = occupied_or_emperor_or_guild() && fp.has_low_threshold(self, homeworld.world());

            // Half, rounded towards zero.
            let amount_lost = if half_is_lost { *received_amount / 2 } else { 0 };

            *received_amount -= amount_lost;

            let Some(occupier) = occupier else { return };

            crate::game::amounts::gain(&mut self.player_mut(occupier).resources, amount_lost, "spice").expect("income lost to the occupier");
            // C# parity: logs "0 ... goes to occupier" whenever there is an occupier.
            let from_faction = self.player(from).faction;
            let occupier_faction = self.player(occupier).faction;
            self.log(elems![Payment::of(amount_lost), " received by ", from_faction, " goes to ", occupier_faction, " due to occupation"]);
        }
    }

    pub fn collect_resources_from_strongholds_and_homeworlds(&mut self) {
        if self.applicable(Rule::IncreasedResourceFlow) || self.applicable(Rule::ResourceBonusForStrongholds) {
            for i in 0..self.players.len() {
                let player = self.player_at(i);
                self.collect_resources_from_stronghold(self.board.map.arrakeen(), player, 2);
                self.collect_resources_from_stronghold(self.board.map.carthag(), player, 2);
                self.collect_resources_from_stronghold(self.board.map.tueks_sietch(), player, 1);
                self.collect_resources_from_stronghold(self.board.map.cistern().location(), player, 2);
            }
        }

        let homeworlds: Vec<Homeworld> = self.board.map.homeworlds().filter(|hw| hw.resource_amount() != 0).collect();
        for homeworld in homeworlds {
            if let Some(occupier) = self.occupier_of(homeworld.world()) {
                let faction = self.player(occupier).faction;
                self.gain_collected_resources(faction.expect("faction"), homeworld.territory(), homeworld.resource_amount(), None, true);
                crate::game::amounts::gain(&mut self.player_mut(occupier).transferable_resources, homeworld.resource_amount(), "spice").expect("homeworld income");
            }
        }
    }

    pub fn collect_resources_from_stronghold(&mut self, stronghold: Location, player: PlayerId, amount: i32) {
        let contested = self.applicable(Rule::ContestedStongholdsCountAsOccupied);
        let p = self.player(player);
        if p.controls(self, stronghold, contested)
            && !(p.is(Faction::Ecaz)
                && self.prevented(FactionAdvantage::EcazCollection)
                && p.has_ally()
                && !self.player(p.allied_player(self).expect("allied player")).controls(self, stronghold, self.applicable(Rule::ContestedStongholdsCountAsOccupied)))
        {
            let faction = p.faction;
            self.gain_collected_resources(faction.expect("faction"), stronghold.territory(), amount, None, true);
        }
    }

    pub fn collect_resources_from_territories(&mut self) {
        let processing_station = self.board.map.processing_station().location();
        let find_thief = |g: &Game| g.player_indices().find(|p| g.player(*p).occupies(g, Some(processing_station)));

        // `.ToList()`: a snapshot of the pairs, in dictionary order.
        let snapshot: Vec<(Location, i32)> = self.board.resources_on_planet.iter().filter(|(_, v)| **v > 0).map(|(k, v)| (*k, i32::from(*v))).collect();
        for (key, value) in snapshot {
            let mut thief = find_thief(self);

            let players_to_collect: Vec<PlayerId> =
                self.player_indices().filter(|y| self.player(*y).occupies(self, Some(key))).collect();
            let mut total_collected_amount = 0i32;
            let mut spice_left = value;

            for &p in &players_to_collect {
                let collection_rate = self.resource_collection_rate(p);
                let pl = self.player(p);
                let forces_collecting_default_amount_of_spice =
                    if pl.faction != Some(Faction::Ixian) { pl.occupying_forces_in_location(key) } else { pl.forces_in(Some(key)) };
                let forces_collecting3_spice = if pl.is(Faction::Ixian) { pl.special_forces_in(Some(key)) } else { 0 };
                let maximum_spice_that_can_be_collected = (forces_collecting_default_amount_of_spice * collection_rate) + (forces_collecting3_spice * 3);
                let mut collected_amount_by_this_player = spice_left.min(maximum_spice_that_can_be_collected);
                self.change_resources_on_planet(key, -collected_amount_by_this_player).expect("at most the spice there");
                spice_left -= collected_amount_by_this_player;

                if let Some(t) = thief {
                    if collected_amount_by_this_player > 0 && p != t {
                        collected_amount_by_this_player -= 1;
                        crate::game::amounts::gain(&mut self.player_mut(t).resources, 1, "spice").expect("stolen spice");
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
                    self.gain_collected_resources(p_faction.expect("faction"), key.territory(), collected_amount_by_this_player, Some(key), false);
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
                    (Some(Faction::Ecaz), self.player(other).faction)
                } else {
                    (self.player(players_to_collect[0]).faction, self.player(players_to_collect[1]).faction)
                };

                // `new ResourcesToBeDivided()` with every field assigned before it is stored.
                let to_be_divided = ResourcesToBeDivided {
                    amount: total_collected_amount,
                    first_faction: first_faction.expect("faction"),
                    other_faction: other_faction.expect("faction"),
                    territory: key.territory(),
                };
                self.collection_mut().expect("in the collection phase").collected_resources_to_be_divided.push(to_be_divided);
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
        let (by_fremen, by_harkonnen) = {
            let c = self.collection().expect("in the collection phase");
            (c.resources_collected_by_fremen, c.resources_collected_by_harkonnen_from_desert_or_homeworld)
        };
        if let Some(fremen) = self.get_player(Some(Faction::Fremen)) {
            let mut received_amount_by_fremen_after_taking_into_account_occupation = by_fremen;
            self.modify_income_because_of_low_threshold_or_occupation(fremen, &mut received_amount_by_fremen_after_taking_into_account_occupation);

            let lost = by_fremen - received_amount_by_fremen_after_taking_into_account_occupation;
            crate::game::amounts::spend(&mut self.player_mut(fremen).resources, lost, "spice").expect("the Fremen's income lost to the occupier");
        }

        let harkonnen = self.get_player(Some(Faction::Harkonnen));
        if by_harkonnen != 0 {
            if let Some(harkonnen) = harkonnen.filter(|b| self.player(*b).has_any_high_threshold(self)) {
                let r = &mut self.player_mut(harkonnen).resources;
                *r += 2;
                self.log(elems![Faction::Harkonnen, " get ", Payment::of(2), " extra as they collected from desert or homeworld"]);
            }
        }

        self.main_phase_end();
        self.enter_collection(CollectionStep::Report);
    }

    pub fn resource_collection_rate(&self, p: PlayerId) -> i32 {
        let pl = self.player(p);
        if pl.controls(self, self.board.map.arrakeen(), self.applicable(Rule::ContestedStongholdsCountAsOccupied))
            || pl.controls(self, self.board.map.carthag(), self.applicable(Rule::ContestedStongholdsCountAsOccupied))
        {
            3
        } else {
            2
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResourcesToBeDivided {
    pub amount: i32,
    pub first_faction: Faction,
    pub other_faction: Faction,
    pub territory: Territory,
}

