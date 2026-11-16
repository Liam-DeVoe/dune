//! The `Game` struct and its rules. Each submodule holds the `impl Game` block for one phase or
//! topic, plus the state types it uses; see docs/engine.md.

mod alliances;
pub mod amounts;
pub use alliances::Alliances;
mod applicable_events;
mod battalion;
pub use battalion::*;
mod battle;
pub use battle::*;
mod battle_outcome;
pub use battle_outcome::*;
mod bidding;
pub use bidding::BiddingState;
pub(crate) use bidding::*;
mod blow;
pub use blow::*;
mod board;
pub use board::Board;
mod charity;
pub use charity::CharityState;
mod collection;
pub use collection::*;
mod contemplate;
pub use contemplate::ContemplateState;
mod counters;
pub use counters::*;
mod deals;
pub use deals::*;
mod decks;
pub use decks::Decks;
pub mod half_points;
pub use half_points::HalfPoints;
mod history;
pub use history::History;
mod leaders;
pub use leaders::*;
mod log;
pub use log::Log;
mod moment;
pub use moment::*;
mod movement;
pub use movement::*;
mod other;
pub use other::*;
mod player;
pub use player::*;
mod player_sequence;
pub use player_sequence::*;
mod revival;
pub use revival::*;
mod rules;
pub use rules::Rules;
mod seats;
pub use seats::{seat_or_minus_one, Seat};
mod settings;
pub use settings::*;
mod setup;
pub use setup::SetupState;
mod state;
pub use state::*;
mod statistics;
pub use statistics::*;
mod status;
pub use status::*;
mod storm;
pub use storm::StormState;
mod turn;
pub use turn::{Step, Turn, TurnState};

use crate::prelude::*;
use crate::messages::part::Part;

/// A game: its rules, players and board, and the state of each phase. Each field's type is
/// defined in the file of its topic under `game/` (see docs/engine.md).
pub struct Game {
    pub seed: i32,
    pub random: Option<SharedRandom>,
    pub rules: Rules,
    pub players: Vec<Player>,
    /// The seats users play. Every other seat is a bot's (`is_bot`). The server seats users, so
    /// it keeps this up to date. It isn't part of the events: replaying a game's events gives
    /// the same game only with the same human seats.
    pub human_seats: IndexSet<Seat>,
    pub board: Board,
    pub decks: Decks,
    pub leaders: Leaders,
    pub alliances: Alliances,
    pub deals: Deals,
    pub turn: TurnState,
    pub history: History,
    pub log: Log,
    pub counters: Counters,

    // The state of each phase. They are all kept side by side, whatever the current phase.
    pub setup: SetupState,
    pub storm: StormState,
    pub blow: BlowState,
    pub charity: CharityState,
    pub bidding: BiddingState,
    pub revival: RevivalState,
    pub ship_move: ShipMoveState,
    pub battle: BattleState,
    pub collection: CollectionState,
    pub contemplate: ContemplateState,
}

impl Game {
    pub const LATEST_VERSION: i32 = 190;

    pub fn maximum_turns(&self) -> Turn {
        self.rules.settings.as_ref().expect("settings").maximum_turns
    }

    pub fn maximum_players(&self) -> i32 {
        self.rules.settings.as_ref().expect("settings").number_of_players
    }

    /// A game of the latest version that bots play.
    pub fn new() -> Self {
        Self::of_version(Self::LATEST_VERSION)
    }

    /// Only `LATEST_VERSION` games can be played, so savegames and event logs of older versions
    /// fail to load instead of replaying.
    pub fn check_version(version: i32) -> Result<(), String> {
        if version != Self::LATEST_VERSION {
            return Err(format!("Game version {} is not supported. Only version {} is supported.", version, Self::LATEST_VERSION));
        }
        Ok(())
    }

    /// A game that bots play. Only `LATEST_VERSION` is supported (see [`Game::check_version`]);
    /// other versions panic.
    pub fn of_version(version: i32) -> Self {
        if let Err(e) = Self::check_version(version) {
            panic!("{e}");
        }

        let mut game = Game {
            seed: -1,
            random: None,
            rules: Rules::new(version),
            players: Vec::new(),
            human_seats: IndexSet::new(),
            board: Board::new(),
            decks: Decks::new(),
            leaders: Leaders::new(),
            alliances: Alliances::new(),
            deals: Deals::new(),
            turn: TurnState::new(),
            history: History::new(),
            log: Log::new(),
            counters: Counters::default(),
            setup: SetupState::new(),
            storm: StormState::new(),
            blow: BlowState::new(),
            charity: CharityState::new(),
            bidding: BiddingState::new(),
            revival: RevivalState::new(),
            ship_move: ShipMoveState::new(),
            battle: BattleState::new(),
            collection: CollectionState::new(),
            contemplate: ContemplateState::new(),
        };

        game.initialize_leader_state();
        game.enter_phase_awaiting_players();
        game
    }

    pub fn perform_pre_event_tasks(&mut self, entry: &HistoryEntry) {
        self.update_timers(entry);
        let e = &entry.event;

        // e is AllyPermission or PlayerReplaced or NexusPlayed { IsBetrayal: true, Faction: Faction.Richese }
        let skip = match e {
            Event::AllyPermission(_) | Event::PlayerReplaced(_) => true,
            Event::NexusPlayed(np) => np.is_betrayal(self) && np.faction == Some(Faction::Richese),
            _ => false,
        };
        if skip {
            return;
        }

        self.clear_recent_payments();

        if !matches!(e, Event::DealOffered(_) | Event::DealAccepted(_) | Event::DealRejected(_)) {
            self.decks.recently_discarded.clear();
        }
    }

    pub fn perform_post_event_tasks(&mut self, entry: HistoryEntry, just_entered_start_of_phase: bool) {
        if !just_entered_start_of_phase
            && !matches!(
                entry.event,
                Event::AllyPermission(_) | Event::DealOffered(_) | Event::DealAccepted(_) | Event::DealRejected(_)
            )
        {
            self.main_phase_middle();
        }

        self.history.entries.push(entry);
        self.history.moments.push(Moment::new(self.turn.current_turn, self.turn.current_main_phase));
    }

    pub fn known_cards(&self, p: PlayerId) -> Vec<TreacheryCard> {
        let player = self.player(p);
        let mut result: Vec<TreacheryCard> = player.treachery_cards.clone();
        result.extend(player.known_cards.iter().copied());

        if player.has_ally() {
            let ally = self.player(player.allied_player(self).expect("allied player"));
            result.extend(ally.treachery_cards.iter().copied());
            result.extend(ally.known_cards.iter().copied());
        }

        result.into_iter().distinct()
    }

    pub fn register_known(&mut self, c: TreacheryCard) {
        for i in 0..self.players.len() {
            Game::register_known_by_player(self, self.player_at(i), Some(c));
        }
    }

    pub fn register_known_by_player(game: &mut Game, p: PlayerId, c: Option<TreacheryCard>) {
        if let Some(c) = c {
            game.player_mut(p).known_cards.insert(c);
        }
    }

    pub fn register_known_by_faction(&mut self, f: Faction, c: TreacheryCard) {
        let p = self.get_player(Some(f));
        if let Some(p) = p {
            Game::register_known_by_player(self, p, Some(c));
        }
    }

    pub fn unregister_known(&mut self, c: TreacheryCard) {
        for i in 0..self.players.len() {
            Game::unregister_known_by_player(self, self.player_at(i), c);
        }
    }

    pub fn unregister_known_by_player(game: &mut Game, p: PlayerId, c: TreacheryCard) {
        game.player_mut(p).known_cards.shift_remove(&c);
    }

    pub fn unregister_all_known_by_player(game: &mut Game, p: PlayerId, cards: &[TreacheryCard]) {
        for c in cards {
            Game::unregister_known_by_player(game, p, *c);
        }
    }

    /// The lists hold copies of the players' battalions. No caller mutates them.
    pub fn forces(&self, include_homeworlds: bool) -> IndexMap<Location, Vec<Battalion>> {
        let mut result: IndexMap<Location, Vec<Battalion>> = IndexMap::new();

        for l in self.board.map.locations(include_homeworlds) {
            result.insert_new(l, Vec::new());
        }

        for p in &self.players {
            if include_homeworlds {
                // C# parity: Map.Locations(true) already holds every homeworld, so this
                // Add throws ArgumentException for the first homeworld (no caller passes true).
                for w in &p.home_worlds {
                    result.insert_new(w.location(), Vec::new());
                }
            }

            let forces = if include_homeworlds { p.forces_in_locations.iter().map(|(k, v)| (*k, v.clone())).collect() } else { p.forces_on_planet() };

            for (key, value) in forces.iter() {
                match result.get_mut(key) {
                    Some(list) => list.push(value.clone()),
                    None => panic!("{key:?} is not a location of the map"),
                }
            }
        }

        result
    }

    /// In map order (by location id), so the board lists forces the same way every time.
    pub fn forces_on_planet_excluding_empty_locations(&self, include_homeworlds: bool) -> BTreeMap<Location, Vec<Battalion>> {
        let mut result: BTreeMap<Location, Vec<Battalion>> = BTreeMap::new();

        for p in &self.players {
            let forces = if include_homeworlds { p.forces_in_locations.iter().map(|(k, v)| (*k, v.clone())).collect() } else { p.forces_on_planet() };

            for (key, value) in forces.iter() {
                if !result.contains_key(key) {
                    result.insert_new(*key, Vec::new());
                }

                match result.get_mut(key) {
                    Some(list) => list.push(value.clone()),
                    None => panic!("{key:?} is not a location of the map"),
                }
            }
        }

        result
    }

    pub fn any_forces_in(&self, t: Territory) -> bool {
        for p in &self.players {
            for l in t.locations() {
                if p.forces_in_locations.contains_key(l) {
                    return true;
                }
            }
        }

        false
    }

    pub fn battalions_in(&self, l: Location) -> Vec<Battalion> {
        let mut result = Vec::new();
        for p in &self.players {
            let mut battalion: Option<Battalion> = None;
            if p.battalion_in(l, &mut battalion) {
                if let Some(battalion) = battalion {
                    result.push(battalion);
                }
            }
        }

        result
    }

    pub fn occupying_forces_on_planet(&self) -> IndexMap<Location, Vec<Battalion>> {
        let mut result: IndexMap<Location, Vec<Battalion>> = IndexMap::new();

        for p in &self.players {
            for (key, value) in p.forces_in_locations.iter().filter(|kvp| p.occupies(self, Some(*kvp.0))) {
                if !result.contains_key(key) {
                    result.insert_new(*key, Vec::new());
                }

                match result.get_mut(key) {
                    Some(list) => list.push(value.clone()),
                    None => panic!("{key:?} is not a location of the map"),
                }
            }
        }

        result
    }

    pub fn flip_bene_gesserit_advisors_when_alone(&mut self) -> GameResult {
        if self.turn.current_main_phase >= MainPhase::Battle && self.applicable(Rule::DisableNovaFlipping) {
            return Ok(());
        }

        let bg = self.get_player(Some(Faction::BeneGesserit));
        if let Some(bg) = bg {
            // A lazy Where enumerated by foreach: the filter is evaluated for each
            // territory right before its body runs, after earlier flips.
            for t in self.board.map.territories(true) {
                let matches = self.player(bg).special_forces_in_territory(Some(t)) > 0
                    && (t == self.board.map.polar_sink().territory()
                        || !self.players.iter().any(|p| p.faction != Some(Faction::BeneGesserit) && p.any_forces_in_territory(Some(t)) > 0));
                if !matches {
                    continue;
                }

                Player::flip_forces_in_territory(self, bg, t, false)?;
                self.log(vec![express!(Faction::BeneGesserit, " are alone and flip to ", FactionForce::BeneGesserit, " in ", t)
                    .with_log(BeneGesseritFlipped::new(t))
                    .into()]);
            }
        }
        Ok(())
    }

    pub fn check_if_shipment_permissions_should_be_revoked(&mut self) {
        if !self.has_high_threshold(Faction::Guild) && self.ship_move.shipment_permissions.len() > 0 {
            self.ship_move.shipment_permissions.clear();
            self.log(vec!["Only ".into(), Faction::Guild.into(), " can ship cross/from planet now".into()]);
        }
    }

    pub fn let_factions_discard_surplus_cards(&mut self) {
        let factions: Vec<Faction> = self.players.iter().filter(|p| p.hand_size_exceeded(self)).filter_map(|p| p.faction).collect();
        self.turn.factions_that_must_discard.extend(factions);
        if self.turn.factions_that_must_discard.len() > 0 {
            self.turn.phase_before_discarding = Some(self.turn.current_phase);
            self.enter(Phase::Discarding, true);
        }
    }

    /// Discards the card a player plays as Karama and logs the play as one aside:
    /// "X play [Karama]: <effect>". A card played as Karama that isn't one says
    /// so: "X play [Baliset] as a Karama card: <effect>".
    pub fn play_karma(&mut self, player: PlayerId, card: Option<TreacheryCard>, effect: Vec<Part>) {
        self.discard_card_of(Some(player), card, false);
        let faction = self.player(player).faction;
        let m = Game::karma_played(faction.expect("faction"), card, effect);
        self.log(vec![m.into()]);
    }

    pub fn karma_played(initiator: Faction, card: Option<TreacheryCard>, effect: Vec<Part>) -> Message {
        let card_or_karma: Part = match card {
            Some(c) => c.into(),
            None => TreacheryCardType::Karma.into(),
        };
        let has_effect = effect.len() > 0;
        express!(
            initiator,
            " play ",
            card_or_karma,
            express_part_if!(card.is_some() && card.expect("card").type_() != TreacheryCardType::Karma; " as a ", TreacheryCardType::Karma, " card"),
            express_part_if!(has_effect; ": ", MessagePart::express(effect))
        )
        .with_log(AsideLog::aside())
    }

    /// A new game with no events, of this game's version and with its human seats.
    pub fn new_like(&self) -> Game {
        let mut game = Game::of_version(self.rules.version);
        game.human_seats = self.human_seats.clone();
        game
    }

    /// Replays a savegame's events into a new game with these human seats, or says why it can't.
    pub fn load(state: &GameState, human_seats: &IndexSet<Seat>, perform_validation: bool, is_host: bool) -> Result<Game, Message> {
        Game::check_version(state.version).map_err(|e| express!(e))?;
        let mut game = Game::of_version(state.version);
        game.human_seats = human_seats.clone();
        let events = state.events.as_ref().ok_or_else(|| express!("The savegame has no events"))?;
        for (nr, e) in events.iter().enumerate() {
            if let Err(error) = game.play(e.clone(), perform_validation, is_host) {
                return Err(express!(e.event_type().name(), "(", nr as i32, "):", error.message()));
            }
        }
        Ok(game)
    }

    pub fn is_occupied_territory(&self, t: Territory) -> bool {
        self.players.iter().any(|p| p.occupies_territory(self, Some(t)))
    }

    pub fn is_occupied_world(&self, world: World) -> bool {
        self.occupier_of(world).is_some()
    }

    pub fn is_occupied_by_player_or_their_ally(&self, world: World, p: PlayerId) -> bool {
        let occupier = self.occupier_of(world);
        match occupier {
            Some(o) => o == p || self.player(o).ally == self.player(p).faction,
            None => false,
        }
    }

    pub fn is_occupied_by_faction_or_their_ally(&self, world: World, f: Faction) -> bool {
        let occupier = self.occupier_of(world);
        match occupier {
            Some(o) => self.player(o).is(f) || self.player(o).ally == Some(f),
            None => false,
        }
    }

    pub fn contains_conflicting_ally(&self, initiator: PlayerId, to: Location) -> bool {
        let ini = self.player(initiator);
        if !ini.has_ally() || self.board.map.polar_sink() == to || ini.faction == Some(Faction::Ecaz) || ini.ally == Some(Faction::Ecaz) {
            return false;
        }

        let ally = ini.allied_player(self);

        if ini.ally == Some(Faction::BeneGesserit) && self.applicable(Rule::AdvisorsDontConflictWithAlly) {
            return self.player(ally.expect("ally")).forces_in_territory(Some(to.territory())) > 0;
        }

        self.player(ally.expect("ally")).any_forces_in_territory(Some(to.territory())) > 0
    }

    pub fn owner_of(&self, hero: Hero) -> Option<PlayerId> {
        // Enumerable.Contains<IHero> over a List<Leader>: Equals on each leader
        self.players.iter().position(|p| p.leaders.iter().any(|l| Hero::Leader(*l) == hero)).map(|i| self.player_at(i))
    }

    pub fn owner_of_treachery_card(&self, card: TreacheryCard) -> Option<PlayerId> {
        self.players.iter().position(|p| p.has(Some(card))).map(|i| self.player_at(i))
    }

    pub fn owner_of_treachery_card_type(&self, card_type: TreacheryCardType) -> Option<PlayerId> {
        self.players.iter().position(|p| p.treachery_cards.iter().any(|c| c.type_() == card_type)).map(|i| self.player_at(i))
    }

    pub fn nr_of_occupants_excluding_faction(&self, l: Location, to_exclude: impl Into<Option<Faction>>) -> i32 {
        let to_exclude: Option<Faction> = to_exclude.into();
        let factions: Vec<Faction> = self.battalions_in(l).iter().filter(|b| b.can_occupy()).map(|b| b.faction).collect();
        self.count_different_factions(&factions, to_exclude)
    }

    pub fn nr_of_occupants_of_territory_excluding(&self, t: Territory, to_exclude: impl Into<Option<Faction>>) -> i32 {
        let to_exclude: Option<Faction> = to_exclude.into();
        let factions: Vec<Faction> = t
            .locations()
            .iter()
            .flat_map(|l| self.battalions_in(*l).into_iter().filter(|b| b.can_occupy()).map(|b| b.faction))
            .collect();
        self.count_different_factions(&factions, to_exclude)
    }

    pub fn count_different_factions(&self, factions: &[Faction], to_exclude: Option<Faction>) -> i32 {
        let mut result = 0;
        let ecaz_ally = self.get_ally(Some(Faction::Ecaz));
        let mut ecaz_or_ecaz_ally_counted = false;

        for faction in factions.iter().copied().distinct() {
            if faction == Faction::Ecaz || Some(faction) == ecaz_ally {
                if !ecaz_or_ecaz_ally_counted && to_exclude != Some(Faction::Ecaz) && to_exclude != ecaz_ally {
                    result += 1;
                }

                ecaz_or_ecaz_ally_counted = true;
            } else if Some(faction) != to_exclude {
                result += 1;
            }
        }

        result
    }

    pub fn has_ornithopters(&self, p: PlayerId) -> bool {
        (if self.applicable(Rule::MovementBonusRequiresOccupationBeforeMovement) {
            self.ship_move.factions_with_ornithopters_at_start_of_movement.contains_opt(self.player(p).faction)
        } else {
            self.occupies_arrakeen_or_carthag(p)
        }) || (self.ship_move.current_flight_used.as_ref().map_or(false, |f| f.move_three_territories)
            && self.ship_move.current_flight_used.as_ref().expect("current flight used").player_opt() == Some(p))
            || (self.ship_move.current_flight_discovery_used.is_some() && self.ship_move.current_flight_discovery_used.as_ref().expect("current flight discovery used").player_opt() == Some(p))
    }

    pub fn determine_maximum_move_distance(&self, p: PlayerId, moved: &[Battalion]) -> i32 {
        let player = self.player(p);
        let choam_extra_move_bonus = if player.faction == Some(Faction::Choam) && self.ship_move.choam_has_extra_move { 1 } else { 0 };

        let planetology_bonus = if self.turn.current_planetology.as_ref().map_or(false, |pl| pl.add_one_to_movement)
            && self.turn.current_planetology.as_ref().expect("current planetology").initiator() == player.faction
        {
            1
        } else {
            0
        };

        let mut result = 1 + planetology_bonus;

        if self.has_ornithopters(p) {
            result = 3;
        } else if player.is(Faction::Fremen) && !self.prevented(FactionAdvantage::FremenExtraMove) {
            result = 2 + planetology_bonus;
        } else if player.is(Faction::Ixian)
            && !self.prevented(FactionAdvantage::IxianCyborgExtraMove)
            && moved.iter().any(|b| b.amount_of_special_forces > 0)
        {
            result = 2 + planetology_bonus;
        }

        result + choam_extra_move_bonus
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn locations_with_any_forces_not_in_storm(&self, p: PlayerId) -> impl Iterator<Item = Location> {
        // `p` is dereferenced inside the predicate, per element.
        self.board.map
            .locations(true)
            .filter(move |l| !Sector::in_storm(l.sector(&self.board.map), self.board.sector_in_storm) && self.player(p).any_forces_in(Some(*l)) > 0)
    }

    pub fn is_not_full(&self, p: PlayerId, l: Location) -> bool {
        // C# dereferences `p` only after the first clause, so a null player passes for
        // locations outside strongholds and homeworlds (boundary.rs lazy cases).
        if !(l.territory().is_stronghold() || l.territory().is_homeworld()) {
            return true;
        }
        let player = self.player(p);
        (player.is(Faction::BeneGesserit) && player.special_forces_in(Some(l)) > 0)
            //Looks at occupying forces
            || (player.is(Faction::Ecaz)
                && player.has_ally()
                && self.player(player.allied_player(self).expect("allied player")).occupying_forces_in(Some(l.territory())) > 0)
            //Looks at occupying forces
            || (player.has_ally()
                && player.ally == Some(Faction::Ecaz)
                && self.player(player.allied_player(self).expect("allied player")).occupying_forces_in(Some(l.territory())) > 0)
            || self.nr_of_occupants_excluding_faction(l, player.faction) < 2
    }

    pub fn assisted_note_keeping_enabled(&self, p: Option<PlayerId>) -> bool {
        match p {
            Some(p) => {
                self.applicable(Rule::AssistedNotekeeping)
                    || (self.player(p).is(Faction::Atreides) && self.applicable(Rule::AssistedNotekeepingForAtreides))
            }
            None => false,
        }
    }

    pub fn has_storm_prescience(&self, p: Option<PlayerId>) -> bool {
        let Some(p) = p else { return false };
        let player = self.player(p);
        self.applicable(Rule::FremenDeterminesStorm)
            && !self.prevented(FactionAdvantage::FremenStormPrescience)
            && (player.faction == Some(Faction::Fremen)
                || (player.ally == Some(Faction::Fremen) && self.alliances.fremen_shares_prescience)
                || self.has_deal(player.faction, DealType::ShareStormPrescience))
    }

    pub fn has_high_threshold(&self, f: Faction) -> bool {
        let player = self.get_player(Some(f));
        player.map_or(false, |p| self.player(p).has_any_high_threshold(self))
    }

    pub fn has_high_threshold_in(&self, f: Faction, w: World) -> bool {
        let player = self.get_player(Some(f));
        player.map_or(false, |p| self.player(p).has_high_threshold(self, w))
    }

    pub fn has_low_threshold(&self, f: Faction) -> bool {
        let player = self.get_player(Some(f));
        player.map_or(false, |p| self.player(p).has_any_low_threshold(self))
    }

    pub fn has_resource_deck_prescience(&self, p: Option<PlayerId>) -> bool {
        let Some(p) = p else { return false };
        let player = self.player(p);
        !self.prevented(FactionAdvantage::AtreidesSpiceBlowPrescience)
            && !self.has_low_threshold(Faction::Atreides)
            && (player.faction == Some(Faction::Atreides)
                || (player.ally == Some(Faction::Atreides) && self.alliances.atreides_shares_prescience)
                || self.has_deal(player.faction, DealType::ShareResourceDeckPrescience))
    }

    pub fn has_bidding_prescience(&self, p: PlayerId) -> bool {
        let is_publicly_known =
            self.bidding.current_auction_type == Some(AuctionType::RicheseOnceAround) || self.bidding.current_auction_type == Some(AuctionType::RicheseSilent);
        let player = self.player(p);

        is_publicly_known
            || self.occupies(player.faction, World::Atreides)
            || self.occupies(player.ally, World::Atreides)
            || (!self.prevented(FactionAdvantage::AtreidesBiddingPrescience)
                && (player.faction == Some(Faction::Atreides)
                    || (player.ally == Some(Faction::Atreides) && self.alliances.atreides_shares_prescience)
                    || self.has_deal(player.faction, DealType::ShareBiddingPrescience)))
    }

    pub fn traitors_in_play(&self) -> Vec<Hero> {
        let mut result: Vec<Hero> = Vec::new();

        // `var factionsInPlay = Players.Select(p => p.Faction)` is lazy: each Contains below
        // runs the Select again, up to the first match.
        let factions_in_play = self.players.iter().map(|p| p.faction);
        result.extend(
            LeaderManager::leaders()
                .iter()
                .filter(|l| {
                    factions_in_play.clone().any(|f| f == Some(l.faction()))
                        && l.hero_type() != HeroType::Vidal
                        && (l.hero_type() != HeroType::Auditor || self.applicable(Rule::ChoamAuditor))
                })
                .map(|l| Hero::Leader(*l)),
        );

        if self.applicable(Rule::CheapHeroTraitor) {
            let card = TreacheryCardManager::get_cards_in_play(self)
                .into_iter()
                .find(|c| c.type_() == TreacheryCardType::Mercenary)
                .expect("a mercenary card in play");
            result.push(Hero::TreacheryCard(card));
        }

        result
    }
}

impl Game {
    /// The player at position `index` of `Players`.
    pub fn player_at(&self, index: usize) -> PlayerId {
        PlayerId::from_index(index)
    }

    /// Every player of `Players`, in order.
    pub fn player_indices(&self) -> impl Iterator<Item = PlayerId> + Clone + 'static {
        (0..self.players.len()).map(|index| PlayerId::from_index(index))
    }

    /// The player `p` refers to.
    pub fn player(&self, p: PlayerId) -> &Player {
        &self.players[p.index()]
    }

    /// `p`, for mutation.
    pub fn player_mut(&mut self, p: PlayerId) -> &mut Player {
        &mut self.players[p.index()]
    }

    /// The index of a player object of this game (`this` inside a `Player` method).
    pub fn index_of(&self, p: &Player) -> Option<PlayerId> {
        self.players.iter().position(|x| std::ptr::eq(x, p)).map(|index| PlayerId::from_index(index))
    }
}

/// What `Map` needs to know about the game (see map.rs).
impl crate::map::MapGame for Game {
    fn sector_in_storm(&self) -> Option<Sector> {
        self.board.sector_in_storm
    }

    fn applicable(&self, rule: Rule) -> bool {
        Game::applicable(self, rule)
    }

    fn forces_battalion_factions(&self) -> Vec<(Location, Vec<Faction>)> {
        // game.Forces(): every location (in Map.Locations order), with the factions of its battalions
        self.forces(false).iter().map(|(l, bs)| (*l, bs.iter().map(|b| b.faction).collect())).collect()
    }

    fn player_ally(&self, f: Faction) -> Option<Faction> {
        self.get_player(Some(f)).map(|p| self.player(p).ally).flatten()
    }

    fn nr_of_occupants_excluding_faction(&self, l: Location, f: Option<Faction>) -> i32 {
        Game::nr_of_occupants_excluding_faction(self, l, f)
    }

    fn current_blocked_territories(&self) -> Vec<Territory> {
        self.ship_move.current_blocked_territories.clone()
    }
}
