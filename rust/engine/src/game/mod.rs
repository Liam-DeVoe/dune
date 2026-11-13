//! The `Game` struct and its rules. Each submodule holds the `impl Game` block for one phase or
//! topic, plus the state types it uses; see docs/engine.md.

mod alliances;
mod applicable_events;
mod battalion;
pub use battalion::*;
mod battle;
pub use battle::*;
mod battle_outcome;
pub use battle_outcome::*;
mod bidding;
pub(crate) use bidding::*;
mod blow;
pub use blow::*;
mod charity;
mod collection;
pub use collection::*;
mod contemplate;
mod counters;
pub use counters::*;
mod deals;
pub use deals::*;
mod info;
pub use info::*;
mod leaders;
pub use leaders::*;
mod moment;
pub use moment::*;
mod movement;
pub use movement::*;
mod other;
pub use other::*;
mod participation;
pub use participation::*;
mod player;
pub use player::*;
mod player_sequence;
pub use player_sequence::*;
mod revival;
pub use revival::*;
mod rules;
mod settings;
pub use settings::*;
mod setup;
mod state;
pub use state::*;
mod statistics;
pub use statistics::*;
mod status;
pub use status::*;
mod storm;

use crate::prelude::*;

pub struct Game {
    // ---- Core state ----

    pub seed: i32,
    pub random: Option<SharedRandom>,
    pub recent_milestones: Vec<Milestone>,
    pub version: i32,
    pub map: Map,
    pub ruleset: Ruleset,
    pub rules: Vec<Rule>,
    pub rules_for_bots: Vec<Rule>,
    /// `EstablishPlayers` can assign a null `Settings.InitialRules`, hence the `Option`.
    pub all_rules: Option<Vec<Rule>>,
    pub history: Vec<GameEvent>,
    pub moments: Vec<Moment>,
    pub recently_undone_event_count: i32,
    pub recently_undone_events: Vec<GameEvent>,
    /// The game as it was before the last `undo`, which `restore_recently_undone` returns to.
    pub previous_game_state: Option<Box<Game>>,
    pub current_turn: i32,
    pub current_main_phase: MainPhase,
    pub current_moment: MainPhaseMoment,
    pub current_phase: Phase,
    pub has_acted_or_passed: Vec<Faction>,
    pub report_list: Vec<Report>,
    pub pre_game_report: Report,
    pub treachery_deck: Option<Deck<TreacheryCard>>,
    pub treachery_discard_pile: Option<Deck<TreacheryCard>>,
    pub removed_treachery_cards: Vec<TreacheryCard>,
    pub resource_card_deck: Option<Deck<ResourceCard>>,
    pub resource_card_discard_pile_a: Option<Deck<ResourceCard>>,
    pub resource_card_discard_pile_b: Option<Deck<ResourceCard>>,
    pub sector_in_storm: i32,
    pub next_storm_moves: i32,
    pub shield_wall_destroyed: bool,
    pub resources_on_planet: NetDictionary<Location, i32>,
    pub timers: IndexMap<Faction, Timer<MainPhase>>,
    pub recently_discarded: NetDictionary<TreacheryCard, Faction>,
    pub phase_before_discarding: Phase,
    pub factions_that_must_discard: Vec<Faction>,
    pub recently_paid: Vec<Payment>,
    pub stored_recently_paid: Vec<Payment>,
    pub skill_deck: Option<Deck<LeaderSkill>>,
    pub richese_cache: Vec<TreacheryCard>,
    pub economics_status: ChoamEconomicsStatus,
    pub terror_on_planet: NetDictionary<TerrorType, Territory>,
    pub unassigned_ambassadors: Option<Deck<Ambassador>>,
    pub atomics_aftermath: Option<Territory>,
    pub ambassadors_set_aside: Vec<Ambassador>,
    pub fremen_discovery_tokens: Option<Deck<DiscoveryToken>>,
    pub guild_discovery_tokens: Option<Deck<DiscoveryToken>>,
    pub discoveries_on_planet: NetDictionary<Location, Discovery>,
    pub ambassadors_on_planet: NetDictionary<Territory, Ambassador>,
    pub nexus_card_deck: Option<Deck<Faction>>,
    pub nexus_discard_pile: Vec<Faction>,
    pub homeworld_occupation: NetDictionary<Homeworld, Faction>,
    pub phase_before_discarding_traitor: Phase,
    pub faction_that_must_discard_traitor: Faction,
    pub number_of_traitors_to_discard: i32,
    pub factions_in_play: Shared<Vec<Faction>>,
    pub unplaced_terror_tokens: Vec<TerrorType>,
    pub traitor_deck: Option<Deck<Hero>>,
    pub ecaz_loyal_leader: Option<Leader>,
    pub automation_rules: Vec<Rc<AutomationConfigured>>,

    // ---- Alliances ----

    pub current_card_trade_offer: Option<Rc<CardTraded>>,
    pub phase_before_card_trade: Option<Phase>,
    pub last_turn_card_was_traded: i32,
    pub current_alliance_offers: Vec<Rc<AllianceOffered>>,
    pub nexus_has_occured: bool,
    pub permitted_use_of_ally_spice: IndexMap<Faction, i32>,
    /// `Option` because `EmperorBidSupport` assigns its `Amounts`, which JSON can leave null.
    pub permitted_use_of_emperor_spice: Option<IndexMap<Faction, i32>>,
    pub permitted_use_of_ally_karma: IndexMap<Faction, TreacheryCard>,
    pub atreides_shares_prescience: bool,
    pub fremen_will_protect_from_monster: bool,
    pub fremen_allows_three_free_revivals: bool,
    pub fremen_shares_prescience: bool,
    pub fremen_refunds_battle_dial: bool,
    pub emperor_will_pay_for_extra_revival: i32,
    pub guild_allows_shipping_discount: bool,
    pub bene_gesserit_allows_use_of_voice: bool,
    pub ixian_allows_replacing_cards: bool,
    pub tleilaxu_allows_revival_discount: bool,
    pub richese_allows_use_of_no_field: bool,
    pub moritani_allows_keeping_cards: bool,
    pub ecaz_shares_ambassadors: bool,

    // ---- Battle ----

    pub battle_sequence: Option<PlayerSequence>,
    pub battle_about_to_start: Option<Rc<BattleInitiated>>,
    pub current_battle: Option<Rc<BattleInitiated>>,
    pub aggressor_plan: Option<Rc<Battle>>,
    pub previous_aggressor_plan: Option<Rc<Battle>>,
    pub aggressor_traitor_action: Option<Rc<TreacheryCalled>>,
    pub defender_plan: Option<Rc<Battle>>,
    pub previous_defender_plan: Option<Rc<Battle>>,
    pub defender_traitor_action: Option<Rc<TreacheryCalled>>,
    pub battle_outcome: Option<BattleOutcome>,
    pub battle_winner: Option<Faction>,
    pub battle_loser: Option<Faction>,
    pub nr_of_battles_fought: i32,
    pub current_ecaz_or_ally_fighter: Option<Faction>,
    pub current_ecaz_battle_contribution: i32,
    pub current_voice: Option<Rc<Voice>>,
    pub current_prescience: Option<Rc<Prescience>>,
    /// The values are the live plan events, shared with `History` and the plan fields.
    pub prescience_seen_plans: IndexMap<PrescienceAspect, Rc<Battle>>,
    pub current_thought: Option<Rc<Thought>>,
    pub chosen_hms_advantage: Option<StrongholdAdvantage>,
    pub current_portable_antidote_used: Option<Rc<PortableAntidoteUsed>>,
    pub poison_tooth_cancelled: bool,
    pub current_rock_was_melted: Option<Rc<RockWasMelted>>,
    pub traitors_decipherer_can_look_at: Vec<Hero>,
    pub decipherer_may_replace_traitor: bool,
    pub loser_may_try_to_assassinate: bool,
    pub battle_winner_may_choose_to_discard: bool,
    pub secret_ally_allows_keeping_cards_after_losing_battle: bool,
    pub cards_to_be_discarded_by_loser_after_battle: Vec<TreacheryCard>,
    pub current_diplomacy: Option<Rc<Diplomacy>>,
    pub audited_cards: Vec<TreacheryCard>,
    pub harkonnen_victim: Option<Leader>,
    pub ixian_special_force_losses_to_take: i32,
    pub battle_triggered_bureaucracy: Option<TriggeredBureaucracy>,
    pub card_used_by_diplomat: Option<TreacheryCard>,
    pub auditor_survived_battle: bool,
    pub harkonnen_do_not_have_to_return_used_captured_leader: bool,
    pub battle_was_concluded_by_winner: bool,
    pub assassinated: Vec<Leader>,
    pub captured_leaders: IndexMap<Leader, Faction>,

    // ---- Bidding ----

    pub bid_sequence: Option<PlayerSequence>,
    pub cards_on_auction: Option<Deck<TreacheryCard>>,
    pub current_auction_type: AuctionType,
    pub card_number: i32,
    pub current_bid: Option<GameEvent>,
    pub bids: NetDictionary<Faction, GameEvent>,
    pub card_just_won: Option<TreacheryCard>,
    pub winning_bid: Option<GameEvent>,
    pub bidding_round_was_started: bool,
    pub regular_bidding_is_done: bool,
    pub card_that_must_be_kept_or_given_to_ally: Option<TreacheryCard>,
    pub number_of_cards_on_auction: i32,
    pub bidding_triggered_bureaucracy: Option<TriggeredBureaucracy>,
    pub ixian_swapped_card_on_bid: bool,
    pub faction_that_may_replace_bought_card: Faction,
    pub replacing_bought_card_using_nexus: bool,
    pub karma_hand_swap_number_of_cards: i32,
    pub karma_hand_swap_target: Faction,
    pub karma_hand_swap_paused_phase: Phase,
    pub black_market_auction_type: AuctionType,
    pub card_sold_on_black_market: Option<TreacheryCard>,
    pub richese_auction_should_still_happen: bool,
    pub richese_bidding_just_finished: bool,
    pub richese_occupier_specified_card: bool,
    pub latest_bid_by_atreides_or_atreides_ally_was_passed: bool,

    // ---- Blow ----

    pub latest_spice_card_a: Option<ResourceCard>,
    pub latest_spice_card_b: Option<ResourceCard>,
    pub ignored_monsters: Vec<ResourceCard>,
    pub monsters: Vec<MonsterAppearence>,
    pub number_of_monsters: i32,
    pub nexus_votes: Vec<Rc<NexusVoted>>,
    pub thumper_called_shai_hulud: bool,
    pub ignored_sandtrout: Option<ResourceCard>,
    pub sand_trout: Option<ResourceCard>,
    pub sand_trout_doubles_resources: bool,
    pub factions_that_may_draw_nexus_card: Vec<Faction>,
    pub factions_that_drew_nexus_card: Vec<Faction>,

    // ---- Charity ----

    pub resource_tech_token_income: bool,

    // ---- Collection ----

    pub resources_collected_by_fremen: i32,
    pub resources_collected_by_harkonnen_from_desert_or_homeworld: i32,
    pub pending_discoveries: Vec<DiscoveryToken>,
    pub collected_resources_to_be_divided: Vec<ResourcesToBeDivided>,
    pub current_division_proposal: Option<Rc<DivideResources>>,
    pub owner_of_flight_discovery: Option<Faction>,
    pub just_revealed_discovery_strongholds: Vec<DiscoveredLocation>,

    // ---- Mentat ----

    pub extortion_to_be_returned: bool,
    pub stronghold_ownership: NetDictionary<Location, Faction>,
    pub moritani_has_performed_terror: bool,
    pub winners: Vec<PlayerIndex>,
    pub win_method: WinMethod,

    // ---- Deals ----

    pub deal_offers: Vec<Rc<DealOffered>>,
    pub deals: Vec<Deal>,
    pub deal_responses: IndexMap<RcKey<DealOffered>, IndexMap<Faction, bool>>,

    // ---- Leaders ----

    pub leader_state: IndexMap<Hero, LeaderState>,

    // ---- Shipment and movement ----

    pub shipment_and_move_sequence: Option<PlayerSequence>,
    pub last_shipment_or_movement: Option<GameEvent>,
    pub intrusions: Vec<Intrusion>,
    pub ships_tech_token_income: bool,
    pub recent_moves: Vec<GameEvent>,
    pub current_no_field_value: i32,
    pub latest_revealed_no_field_value: i32,
    pub shipment_permissions: NetDictionary<Faction, ShipmentPermission>,
    pub factions_with_ornithopters_at_start_of_movement: Vec<Faction>,
    pub beginning_of_shipment_and_move_phase: bool,
    pub current_karma_shipment_prevention: Option<Rc<KarmaShipmentPrevention>>,
    pub chosen_destinations_with_allies: Vec<Territory>,
    pub current_player_may_perform_extra_move: bool,
    pub bene_gesserit_may_accompany: bool,
    pub phase_before_caravan_caused_intrusion: Phase,
    pub phase_before_revival_caused_intrusion: Phase,
    pub current_flight_used: Option<Rc<FlightUsed>>,
    pub current_flight_discovery_used: Option<Rc<FlightDiscoveryUsed>>,
    pub current_ambassador_activated: Option<Rc<AmbassadorActivated>>,
    pub alliance_by_ambassador_offered_to: Faction,
    pub paused_ambassador_phase: Phase,
    pub player_to_set_aside_vidal: Option<PlayerIndex>,
    pub when_to_set_aside_vidal: VidalMoment,
    pub paused_terror_phase: Phase,
    pub alliance_by_terror_was_offered: bool,
    pub alliance_by_terror_offered_to: Faction,
    pub in_guild_cunning_shipment: bool,
    pub current_blocked_territories: Vec<Territory>,
    pub choam_has_extra_move: bool,
    pub moves_logged: i32,

    // ---- Other ----

    pub resource_audited_factions: Vec<Faction>,
    pub phase_paused_by_clairvoyance: Phase,
    pub latest_clairvoyance: Option<Rc<ClairVoyancePlayed>>,
    pub latest_clairvoyance_qn_a: Option<ClairVoyanceQandA>,
    pub latest_clairvoyance_battle: Option<Rc<BattleInitiated>>,
    pub karma_hms_moves_left: i32,
    pub atreides_karma: bool,
    pub ecaz_karma_bonus: i32,
    pub secrets_remain_hidden: Vec<Faction>,
    pub prevented_advantages: Vec<FactionAdvantage>,
    pub phase_before_searching_discarded: Phase,
    pub current_juice: Option<Rc<JuicePlayed>>,
    pub bureaucrat_was_used_this_phase: bool,
    pub phase_before_bureaucrat_was_activated: Phase,
    pub target_of_bureaucracy: Faction,
    pub current_nexus_prescience: Option<Rc<NexusPlayed>>,
    pub current_fremen_secret_ally: Option<Rc<NexusPlayed>>,
    pub current_emperor_cunning: Option<Rc<NexusPlayed>>,
    pub current_guild_cunning: Option<Rc<NexusPlayed>>,
    pub current_guild_secret_ally: Option<Rc<NexusPlayed>>,
    pub current_bene_gesserit_cunning: Option<Rc<NexusPlayed>>,
    pub current_ixian_cunning: Option<Rc<NexusPlayed>>,
    pub was_victim_of_bureaucracy: Faction,
    pub banker_was_used_this_phase: bool,
    pub current_karma_prevention: Option<Rc<ChoamKarmaPrevention>>,
    pub current_planetology: Option<Rc<Planetology>>,
    pub harkonnen_traitor_was_cancelled: bool,
    pub facedancer_was_cancelled: bool,

    // ---- Participation ----

    pub players: Vec<Player>,
    pub legacy_names: IndexMap<PlayerIndex, String>,
    /// Shared with the caller of `new Game` and with every game `Undo`,
    /// `RestoreRecentlyUndone` and `DismissRecentlyUndone` make from this one.
    pub participation: Shared<Participation>,

    // ---- Revival ----

    pub revival_tech_token_income: bool,
    pub total_revivals_this_turn: IndexMap<Faction, i32>,
    pub free_revivals_this_turn: IndexMap<Faction, i32>,
    pub factions_that_revived_special_forces_this_turn: Vec<Faction>,
    pub leader_revivals_this_turn: IndexMap<Faction, i32>,
    pub tleilaxu_started_revival_with_low_threshold: bool,
    pub current_recruits_played: Option<Rc<RecruitsPlayed>>,
    /// Null after `SetIncreasedRevivalLimits` with `"Factions":null` executed without validation.
    pub factions_with_increased_revival_limits: Option<Vec<Faction>>,
    pub current_revival_requests: Vec<Rc<RequestTleilaxuRevival>>,
    pub early_revivals_offers: NetDictionary<Hero, i32>,
    pub current_free_revival_prevention: Option<Rc<ChoamFreeRevivalPrevention>>,
    pub current_karma_revival_prevention: Option<Rc<KarmaRevivalPrevention>>,
    pub ambassadors_placed_this_turn: i32,

    // ---- Setup ----

    /// `EstablishPlayers` can assign null, hence the `Option`.
    pub settings: Option<Shared<GameSettings>>,
    pub current_trade_offers: Vec<Rc<FactionTradeOffered>>,
    pub phase_before_skill_assignment: Phase,
    pub starting_treachery_cards: Option<Deck<TreacheryCard>>,
    pub extra_starting_card_for_harkonnen: Option<TreacheryCard>,

    // ---- Storm ----

    pub hms_moves_left: i32,
    pub has_battle_wheel: Vec<Faction>,
    pub dials: Vec<i32>,
    pub current_testing_station_used: bool,
    pub phase_before_storm_loss: Phase,
    pub storm_losses_to_take: Vec<LossToTake>,

}

impl Game {
    pub const LATEST_VERSION: i32 = 188;

    pub fn maximum_turns(&self) -> i32 {
        self.settings.as_ref().expect("settings").borrow().maximum_turns
    }

    pub fn maximum_players(&self) -> i32 {
        self.settings.as_ref().expect("settings").borrow().number_of_players
    }

    pub fn reports(&self) -> &[Report] {
        &self.report_list
    }

    /// The live report; use [`Game::current_report_mut`] to change it.
    pub fn current_report(&self) -> &Report {
        if self.report_list.len() > 0 {
            &self.report_list[self.report_list.len() - 1]
        } else {
            &self.pre_game_report
        }
    }

    /// `CurrentReport`, for mutation.
    pub fn current_report_mut(&mut self) -> &mut Report {
        if self.report_list.len() > 0 {
            let last = self.report_list.len() - 1;
            &mut self.report_list[last]
        } else {
            &mut self.pre_game_report
        }
    }

    pub fn new() -> Self {
        Self::new_int32_participation(Self::LATEST_VERSION, &Shared::new(Participation::new()))
    }

    /// Only `LATEST_VERSION` games can be played, so savegames and event logs of older versions
    /// fail to load instead of replaying.
    pub fn check_version(version: i32) -> Result<(), String> {
        if version != Self::LATEST_VERSION {
            return Err(format!("Game version {} is not supported. Only version {} is supported.", version, Self::LATEST_VERSION));
        }
        Ok(())
    }

    /// The game keeps the caller's `Participation` object: changes made
    /// through either are seen by both.
    ///
    /// Only `LATEST_VERSION` is supported (see [`Game::check_version`]); other versions panic.
    pub fn new_int32_participation(version: i32, participation: &Shared<Participation>) -> Self {
        if let Err(e) = Self::check_version(version) {
            panic!("{e}");
        }

        let mut game = Game {
            seed: -1,
            random: None,
            recent_milestones: Vec::new(),
            version: version,
            map: Map::new(),
            ruleset: Default::default(),
            rules: Vec::new(),
            rules_for_bots: Vec::new(),
            all_rules: Some(Vec::new()),
            history: Vec::new(),
            moments: Vec::new(),
            recently_undone_event_count: Default::default(),
            recently_undone_events: Vec::new(),
            previous_game_state: None,
            current_turn: Default::default(),
            current_main_phase: MainPhase::Started,
            current_moment: MainPhaseMoment::None,
            current_phase: Phase::None,
            has_acted_or_passed: Vec::new(),
            report_list: Vec::new(),
            pre_game_report: Report::new(MainPhase::None, 0),
            treachery_deck: None,
            treachery_discard_pile: None,
            removed_treachery_cards: Vec::new(),
            resource_card_deck: None,
            resource_card_discard_pile_a: None,
            resource_card_discard_pile_b: None,
            sector_in_storm: -1,
            next_storm_moves: -1,
            shield_wall_destroyed: Default::default(),
            resources_on_planet: Default::default(),
            timers: Default::default(),
            recently_discarded: Default::default(),
            phase_before_discarding: Default::default(),
            factions_that_must_discard: Vec::new(),
            recently_paid: Vec::new(),
            stored_recently_paid: Vec::new(),
            skill_deck: None,
            richese_cache: Vec::new(),
            economics_status: ChoamEconomicsStatus::None,
            terror_on_planet: Default::default(),
            unassigned_ambassadors: None,
            atomics_aftermath: None,
            ambassadors_set_aside: Vec::new(),
            fremen_discovery_tokens: None,
            guild_discovery_tokens: None,
            discoveries_on_planet: Default::default(),
            ambassadors_on_planet: Default::default(),
            nexus_card_deck: None,
            nexus_discard_pile: Vec::new(),
            homeworld_occupation: Default::default(),
            phase_before_discarding_traitor: Default::default(),
            faction_that_must_discard_traitor: Default::default(),
            number_of_traitors_to_discard: Default::default(),
            factions_in_play: Default::default(),
            unplaced_terror_tokens: Vec::new(),
            traitor_deck: None,
            ecaz_loyal_leader: None,
            automation_rules: Vec::new(),
            current_card_trade_offer: None,
            phase_before_card_trade: None,
            last_turn_card_was_traded: -1,
            current_alliance_offers: Vec::new(),
            nexus_has_occured: Default::default(),
            permitted_use_of_ally_spice: Default::default(),
            permitted_use_of_emperor_spice: Some(IndexMap::new()),
            permitted_use_of_ally_karma: Default::default(),
            atreides_shares_prescience: Default::default(),
            fremen_will_protect_from_monster: Default::default(),
            fremen_allows_three_free_revivals: Default::default(),
            fremen_shares_prescience: Default::default(),
            fremen_refunds_battle_dial: Default::default(),
            emperor_will_pay_for_extra_revival: Default::default(),
            guild_allows_shipping_discount: Default::default(),
            bene_gesserit_allows_use_of_voice: Default::default(),
            ixian_allows_replacing_cards: Default::default(),
            tleilaxu_allows_revival_discount: Default::default(),
            richese_allows_use_of_no_field: Default::default(),
            moritani_allows_keeping_cards: Default::default(),
            ecaz_shares_ambassadors: Default::default(),
            battle_sequence: None,
            battle_about_to_start: None,
            current_battle: None,
            aggressor_plan: None,
            previous_aggressor_plan: None,
            aggressor_traitor_action: None,
            defender_plan: None,
            previous_defender_plan: None,
            defender_traitor_action: None,
            battle_outcome: None,
            battle_winner: None,
            battle_loser: None,
            nr_of_battles_fought: Default::default(),
            current_ecaz_or_ally_fighter: None,
            current_ecaz_battle_contribution: Default::default(),
            current_voice: None,
            current_prescience: None,
            prescience_seen_plans: IndexMap::new(),
            current_thought: None,
            chosen_hms_advantage: None,
            current_portable_antidote_used: None,
            poison_tooth_cancelled: Default::default(),
            current_rock_was_melted: None,
            traitors_decipherer_can_look_at: Vec::new(),
            decipherer_may_replace_traitor: Default::default(),
            loser_may_try_to_assassinate: Default::default(),
            battle_winner_may_choose_to_discard: true,
            secret_ally_allows_keeping_cards_after_losing_battle: Default::default(),
            cards_to_be_discarded_by_loser_after_battle: Vec::new(),
            current_diplomacy: None,
            audited_cards: Vec::new(),
            harkonnen_victim: None,
            ixian_special_force_losses_to_take: Default::default(),
            battle_triggered_bureaucracy: None,
            card_used_by_diplomat: None,
            auditor_survived_battle: Default::default(),
            harkonnen_do_not_have_to_return_used_captured_leader: Default::default(),
            battle_was_concluded_by_winner: Default::default(),
            assassinated: Vec::new(),
            captured_leaders: Default::default(),
            bid_sequence: None,
            cards_on_auction: None,
            current_auction_type: Default::default(),
            card_number: Default::default(),
            current_bid: None,
            bids: Default::default(),
            card_just_won: None,
            winning_bid: None,
            bidding_round_was_started: Default::default(),
            regular_bidding_is_done: Default::default(),
            card_that_must_be_kept_or_given_to_ally: None,
            number_of_cards_on_auction: Default::default(),
            bidding_triggered_bureaucracy: None,
            ixian_swapped_card_on_bid: Default::default(),
            faction_that_may_replace_bought_card: Default::default(),
            replacing_bought_card_using_nexus: Default::default(),
            karma_hand_swap_number_of_cards: Default::default(),
            karma_hand_swap_target: Default::default(),
            karma_hand_swap_paused_phase: Default::default(),
            black_market_auction_type: Default::default(),
            card_sold_on_black_market: None,
            richese_auction_should_still_happen: Default::default(),
            richese_bidding_just_finished: Default::default(),
            richese_occupier_specified_card: Default::default(),
            latest_bid_by_atreides_or_atreides_ally_was_passed: Default::default(),
            latest_spice_card_a: None,
            latest_spice_card_b: None,
            ignored_monsters: Vec::new(),
            monsters: Vec::new(),
            number_of_monsters: Default::default(),
            nexus_votes: Vec::new(),
            thumper_called_shai_hulud: Default::default(),
            ignored_sandtrout: None,
            sand_trout: None,
            sand_trout_doubles_resources: Default::default(),
            factions_that_may_draw_nexus_card: Vec::new(),
            factions_that_drew_nexus_card: Vec::new(),
            resource_tech_token_income: Default::default(),
            resources_collected_by_fremen: Default::default(),
            resources_collected_by_harkonnen_from_desert_or_homeworld: Default::default(),
            pending_discoveries: Vec::new(),
            collected_resources_to_be_divided: Vec::new(),
            current_division_proposal: None,
            owner_of_flight_discovery: None,
            just_revealed_discovery_strongholds: Vec::new(),
            extortion_to_be_returned: Default::default(),
            stronghold_ownership: Default::default(),
            moritani_has_performed_terror: Default::default(),
            winners: Vec::new(),
            win_method: Default::default(),
            deal_offers: Vec::new(),
            deals: Vec::new(),
            deal_responses: Default::default(),
            leader_state: Default::default(),
            shipment_and_move_sequence: None,
            last_shipment_or_movement: None,
            intrusions: Vec::new(),
            ships_tech_token_income: Default::default(),
            recent_moves: Vec::new(),
            current_no_field_value: -1,
            latest_revealed_no_field_value: -1,
            shipment_permissions: Default::default(),
            factions_with_ornithopters_at_start_of_movement: Vec::new(),
            beginning_of_shipment_and_move_phase: Default::default(),
            current_karma_shipment_prevention: None,
            chosen_destinations_with_allies: Vec::new(),
            current_player_may_perform_extra_move: Default::default(),
            bene_gesserit_may_accompany: Default::default(),
            phase_before_caravan_caused_intrusion: Default::default(),
            phase_before_revival_caused_intrusion: Default::default(),
            current_flight_used: None,
            current_flight_discovery_used: None,
            current_ambassador_activated: None,
            alliance_by_ambassador_offered_to: Default::default(),
            paused_ambassador_phase: Default::default(),
            player_to_set_aside_vidal: None,
            when_to_set_aside_vidal: Default::default(),
            paused_terror_phase: Default::default(),
            alliance_by_terror_was_offered: Default::default(),
            alliance_by_terror_offered_to: Default::default(),
            in_guild_cunning_shipment: Default::default(),
            current_blocked_territories: Vec::new(),
            choam_has_extra_move: Default::default(),
            moves_logged: Default::default(),
            resource_audited_factions: Vec::new(),
            phase_paused_by_clairvoyance: Default::default(),
            latest_clairvoyance: None,
            latest_clairvoyance_qn_a: None,
            latest_clairvoyance_battle: None,
            karma_hms_moves_left: 2,
            atreides_karma: Default::default(),
            ecaz_karma_bonus: Default::default(),
            secrets_remain_hidden: Vec::new(),
            prevented_advantages: Vec::new(),
            phase_before_searching_discarded: Default::default(),
            current_juice: None,
            bureaucrat_was_used_this_phase: Default::default(),
            phase_before_bureaucrat_was_activated: Default::default(),
            target_of_bureaucracy: Default::default(),
            current_nexus_prescience: None,
            current_fremen_secret_ally: None,
            current_emperor_cunning: None,
            current_guild_cunning: None,
            current_guild_secret_ally: None,
            current_bene_gesserit_cunning: None,
            current_ixian_cunning: None,
            was_victim_of_bureaucracy: Default::default(),
            banker_was_used_this_phase: Default::default(),
            current_karma_prevention: None,
            current_planetology: None,
            harkonnen_traitor_was_cancelled: Default::default(),
            facedancer_was_cancelled: Default::default(),
            players: Vec::new(),
            legacy_names: Default::default(),
            participation: participation.clone(),
            revival_tech_token_income: Default::default(),
            total_revivals_this_turn: Default::default(),
            free_revivals_this_turn: Default::default(),
            factions_that_revived_special_forces_this_turn: Vec::new(),
            leader_revivals_this_turn: Default::default(),
            tleilaxu_started_revival_with_low_threshold: Default::default(),
            current_recruits_played: None,
            factions_with_increased_revival_limits: Some(Vec::new()),
            current_revival_requests: Vec::new(),
            early_revivals_offers: Default::default(),
            current_free_revival_prevention: None,
            current_karma_revival_prevention: None,
            ambassadors_placed_this_turn: Default::default(),
            settings: Some(Default::default()),
            current_trade_offers: Vec::new(),
            phase_before_skill_assignment: Default::default(),
            starting_treachery_cards: None,
            extra_starting_card_for_harkonnen: None,
            hms_moves_left: Default::default(),
            has_battle_wheel: Vec::new(),
            dials: Vec::new(),
            current_testing_station_used: Default::default(),
            phase_before_storm_loss: Default::default(),
            storm_losses_to_take: Vec::new(),
        };

        game.initialize_leader_state();
        game.enter_phase_awaiting_players();
        game
    }

    pub fn initialize_leader_state(&mut self) {
        for l in LeaderManager::leaders() {
            self.leader_state.insert_new(Hero::Leader(*l), LeaderState { death_counter: 0, current_territory: None, skill: LeaderSkill::None, in_front_of_shield: false, ..Default::default() });
        }

        self.leader_state.insert_new(LeaderManager::messiah(), LeaderState { death_counter: 0, current_territory: None, ..Default::default() });
    }

    pub fn perform_pre_event_tasks(&mut self, e: &GameEvent) {
        self.update_timers(e);

        // e is AllyPermission or PlayerReplaced or NexusPlayed { IsBetrayal: true, Faction: Faction.Richese }
        let skip = match e {
            GameEvent::AllyPermission(_) | GameEvent::PlayerReplaced(_) => true,
            GameEvent::NexusPlayed(np) => np.is_betrayal(self) && np.faction == Faction::Richese,
            _ => false,
        };
        if skip {
            return;
        }

        self.clear_recent_payments();

        if !matches!(e, GameEvent::DealOffered(_) | GameEvent::DealAccepted(_) | GameEvent::DealRejected(_)) {
            self.recently_discarded.clear();
        }
    }

    pub fn perform_post_event_tasks(&mut self, e: &GameEvent, just_entered_start_of_phase: bool) {
        if !just_entered_start_of_phase
            && !matches!(
                e,
                GameEvent::AllyPermission(_) | GameEvent::DealOffered(_) | GameEvent::DealAccepted(_) | GameEvent::DealRejected(_)
            )
        {
            self.main_phase_middle();
        }

        self.history.push(e.clone());
        self.moments.push(Moment::new(self.current_turn, self.current_main_phase));
    }

    /// Goes back to the state after the first `until_event_nr` events: a new game replays
    /// them, and the events after them become `recently_undone_events`. The current game
    /// is kept as `previous_game_state`, so `restore_recently_undone` can return to it. A
    /// negative `until_event_nr` throws and leaves this game unchanged.
    pub fn undo(&mut self, until_event_nr: i32) {
        let history = self.history.clone();
        let history_count = history.len() as i32;
        let max_event_nr = std::cmp::min(until_event_nr, history_count);
        let mut result = self.replay(&history[..max_event_nr.max(0) as usize]);

        result.recently_undone_event_count = history_count - max_event_nr;
        for i in max_event_nr..history_count {
            // A negative event number panics here.
            result.recently_undone_events.push(history[i as usize].clone());
        }

        let previous = std::mem::replace(self, result);
        self.previous_game_state = Some(Box::new(previous));
    }

    /// Returns to the game as it was before the last `undo`. Without one (a game loaded
    /// with undone events), replays the history followed by the undone events.
    pub fn restore_recently_undone(&mut self) {
        if let Some(previous) = self.previous_game_state.take() {
            *self = *previous;
            return;
        }

        if self.recently_undone_event_count == 0 || self.recently_undone_events.is_empty() {
            return;
        }

        let events: Vec<GameEvent> = self.history.iter().chain(self.recently_undone_events.iter()).cloned().collect();
        *self = self.replay(&events);
    }

    /// Forgets the undone events and the game before the undo: the history is replayed
    /// into a new game.
    pub fn dismiss_recently_undone(&mut self) {
        let history = self.history.clone();
        *self = self.replay(&history);
    }

    /// A new game of the same version and participation that has executed `events`.
    fn replay(&self, events: &[GameEvent]) -> Game {
        let mut result = Game::new_int32_participation(self.version, &self.participation);
        for e in events {
            e.initialize_game(&result);
            e.execute(&mut result, false, false);
        }
        result
    }

    pub fn update_timers(&mut self, e: &GameEvent) {
        if e.time() != DateTimeOffset::default() && self.history.len() > 0 && self.in_timed_phase() {
            let previous_event = match e.event_type() {
                GameEventType::Battle => self.find_most_recent_event(vec![GameEventType::BattleInitiated]),
                GameEventType::Move => self.find_most_recent_event(vec![GameEventType::Shipment, GameEventType::EndPhase]),
                GameEventType::Shipment => self.find_most_recent_event(vec![GameEventType::Move, GameEventType::EndPhase]),
                _ => self.find_most_recent_event(vec![]),
            };

            let Some(previous_event) = previous_event else { return };

            // DateTimeOffset.Subtract compares UTC instants.
            let elapsed_time = TimeSpan::from_ticks(e.time().utc_ticks() - previous_event.time().utc_ticks());
            if !(elapsed_time.total_hours() < 1.0) {
                return;
            }

            let initiator = e.initiator();
            if self.timers.get(&initiator).is_none() {
                let timer = Timer::<MainPhase>::new();
                self.timers.insert_new(initiator, timer);
            }

            let phase = self.current_main_phase;
            self.timers.get_mut(&initiator).expect("a timer for this faction").add(phase, elapsed_time);
        }
    }

    pub fn duration(&self) -> TimeSpan {
        if self.history.len() > 0 {
            let last = &self.history[self.history.len() - 1];
            TimeSpan::from_ticks(last.time().utc_ticks() - self.history[0].time().utc_ticks())
        } else {
            TimeSpan::default()
        }
    }

    pub fn time_spent(&self, faction: Faction, phase: MainPhase) -> TimeSpan {
        if let Some(timer) = self.timers.get(&faction) {
            return timer.time_spent(phase);
        }
        TimeSpan::default()
    }

    pub fn started(&self) -> DateTimeOffset {
        if self.history.len() > 0 {
            self.history[0].time()
        } else {
            DateTimeOffset::default()
        }
    }

    pub fn in_timed_phase(&self) -> bool {
        self.current_phase == Phase::Bidding
            || self.current_phase == Phase::GuildShip
            || self.current_phase == Phase::GuildMove
            || self.current_phase == Phase::NonGuildShip
            || self.current_phase == Phase::NonGuildMove
            || self.current_phase == Phase::BattlePhase
    }

    pub fn latest_event(&self, event_type: GameEventType) -> Option<GameEvent> {
        // List<T>.LastOrDefault(pred) walks backwards; the predicate has no side effects.
        self.history.iter().rev().find(|e| e.event_type() == event_type).cloned()
    }

    pub fn latest_event_0(&self) -> Option<GameEvent> {
        if self.history.len() > 0 {
            Some(self.history[self.history.len() - 1].clone())
        } else {
            None
        }
    }

    pub fn event_count(&self) -> i32 {
        self.history.len() as i32
    }

    pub fn find_most_recent_event(&self, types: Vec<GameEventType>) -> Option<GameEvent> {
        if types.len() == 0 {
            return Some(self.history.last().expect("an event in the history").clone());
        }

        let mut i = self.history.len() as i32 - 1;
        while i >= 0 {
            if types.contains(&self.history[i as usize].event_type()) {
                return Some(self.history[i as usize].clone());
            }
            i -= 1;
        }

        None
    }

    /// Returns the live report, not a copy.
    pub fn start_report(&mut self, about: MainPhase) -> &mut Report {
        // Avoid leaving an empty duplicate behind if a report for the same phase is started twice in a row
        let current_turn = self.current_turn;
        let has_reports = self.report_list.len() > 0;
        let current = self.current_report();
        if has_reports && current.about == about && current.turn == current_turn && !current.is_battle() && current.messages.len() == 0 {
            return self.current_report_mut();
        }

        let result = Report::new(about, self.current_turn);
        self.report_list.push(result);
        let last = self.report_list.len() - 1;
        &mut self.report_list[last]
    }

    pub fn main_phase_start(&mut self, phase: MainPhase, start_report: bool) {
        self.current_main_phase = phase;
        self.current_moment = MainPhaseMoment::Start;
        if start_report {
            self.start_report(phase);
        }
        self.current_karma_prevention = None;
        self.current_juice = None;
        self.bureaucrat_was_used_this_phase = false;
        self.banker_was_used_this_phase = false;
    }

    pub fn main_phase_middle(&mut self) {
        if self.current_moment == MainPhaseMoment::Start {
            self.current_moment = MainPhaseMoment::Middle;
        }
    }

    pub fn main_phase_end(&mut self) {
        self.current_moment = MainPhaseMoment::End;

        let mut exceptions_to_allowing: Vec<FactionAdvantage> = Vec::new();

        if self.current_main_phase == MainPhase::Bidding && self.prevented(FactionAdvantage::TleilaxuIncreasingRevivalLimits) {
            exceptions_to_allowing.push(FactionAdvantage::TleilaxuIncreasingRevivalLimits);
        }

        if self.current_main_phase == MainPhase::Resurrection && self.prevented(FactionAdvantage::AtreidesSpiceBlowPrescience) {
            exceptions_to_allowing.push(FactionAdvantage::AtreidesSpiceBlowPrescience);
        }

        self.was_victim_of_bureaucracy = Faction::None;

        self.allow_all_prevented_faction_advantages(&exceptions_to_allowing);
    }

    pub fn enter(&mut self, phase: Phase, cleanup_deals: bool) {
        self.current_phase = phase;

        if cleanup_deals {
            self.remove_ended_deals(phase);
        }
    }

    pub fn enter_boolean_phase_phase(&mut self, condition: bool, phase_if_true: Phase, phase_if_false: Phase) {
        self.enter(if condition { phase_if_true } else { phase_if_false }, true);
    }

    pub fn enter_boolean_phase_action(&mut self, condition: bool, phase_if_true: Phase, method_otherwise: fn(&mut Game)) {
        if condition {
            self.enter(phase_if_true, true);
        } else {
            method_otherwise(self);
        }
    }

    /// The game is passed to the actions.
    pub fn enter_boolean_action_action(game: &mut Game, condition: bool, method_if_true: fn(&mut Game), method_otherwise: fn(&mut Game)) {
        if condition {
            method_if_true(game);
        } else {
            method_otherwise(game);
        }
    }

    pub fn enter_boolean_phase_boolean_action_action(&mut self, condition1: bool, phase_if1_true: Phase, condition2: bool, method_if2_true: fn(&mut Game), method_otherwise: fn(&mut Game)) {
        if condition1 {
            self.enter(phase_if1_true, true);
        } else {
            Game::enter_boolean_action_action(self, condition2, method_if2_true, method_otherwise);
        }
    }

    pub fn enter_boolean_phase_boolean_phase_phase(&mut self, condition1: bool, phase_if1_true: Phase, condition2: bool, phase_if2_true: Phase, phase_otherwise: Phase) {
        if condition1 {
            self.enter(phase_if1_true, true);
        } else {
            self.enter_boolean_phase_phase(condition2, phase_if2_true, phase_otherwise);
        }
    }

    pub fn enter_boolean_action_boolean_phase_phase(&mut self, condition1: bool, method_if1_true: fn(&mut Game), condition2: bool, phase_if2_true: Phase, phase_otherwise: Phase) {
        if condition1 {
            method_if1_true(self);
        } else {
            self.enter_boolean_phase_phase(condition2, phase_if2_true, phase_otherwise);
        }
    }

    pub fn enter_boolean_phase_boolean_phase_boolean_action_action(&mut self, condition1: bool, phase_if1_true: Phase, condition2: bool, phase_if2_true: Phase, condition3: bool, method_if3_true: fn(&mut Game), method_otherwise: fn(&mut Game)) {
        if condition1 {
            self.enter(phase_if1_true, true);
        } else if condition2 {
            self.enter(phase_if2_true, true);
        } else {
            Game::enter_boolean_action_action(self, condition3, method_if3_true, method_otherwise);
        }
    }

    pub fn enter_boolean_phase_boolean_phase_boolean_phase_action(&mut self, condition1: bool, phase_if1_true: Phase, condition2: bool, phase_if2_true: Phase, condition3: bool, phase_if3_true: Phase, method_otherwise: fn(&mut Game)) {
        if condition1 {
            self.enter(phase_if1_true, true);
        } else if condition2 {
            self.enter(phase_if2_true, true);
        } else {
            self.enter_boolean_phase_action(condition3, phase_if3_true, method_otherwise);
        }
    }

    pub fn known_cards(&self, p: PlayerIndex) -> Vec<TreacheryCard> {
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
            Game::register_known_player_treachery_card(self, self.player_at(i), Some(c));
        }
    }

    pub fn register_known_player_treachery_card(game: &mut Game, p: PlayerIndex, c: Option<TreacheryCard>) {
        if let Some(c) = c {
            game.player_mut(p).known_cards.insert(c);
        }
    }

    pub fn register_known_faction_treachery_card(&mut self, f: Faction, c: TreacheryCard) {
        let p = self.get_player(Some(f));
        if let Some(p) = p {
            Game::register_known_player_treachery_card(self, p, Some(c));
        }
    }

    pub fn unregister_known(&mut self, c: TreacheryCard) {
        for i in 0..self.players.len() {
            Game::unregister_known_player_treachery_card(self, self.player_at(i), c);
        }
    }

    pub fn unregister_known_player_treachery_card(game: &mut Game, p: PlayerIndex, c: TreacheryCard) {
        game.player_mut(p).known_cards.remove(&c);
    }

    pub fn unregister_known_player_i_enumerable_treachery_card(game: &mut Game, p: PlayerIndex, cards: &[TreacheryCard]) {
        for c in cards {
            Game::unregister_known_player_treachery_card(game, p, *c);
        }
    }

    /// The lists hold copies of the players' battalions. No caller mutates them.
    pub fn forces(&self, include_homeworlds: bool) -> IndexMap<Location, Vec<Battalion>> {
        let mut result: IndexMap<Location, Vec<Battalion>> = IndexMap::new();

        for l in self.map.locations(include_homeworlds) {
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

    pub fn forces_on_planet_excluding_empty_locations(&self, include_homeworlds: bool) -> IndexMap<Location, Vec<Battalion>> {
        let mut result: IndexMap<Location, Vec<Battalion>> = IndexMap::new();

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

    pub fn resources_in(&self, l: Location) -> i32 {
        self.resources_on_planet.get(&l).copied().unwrap_or(0)
    }

    pub fn change_resources_on_planet(&mut self, location: Location, amount: i32) {
        if self.resources_on_planet.contains_key(&location) {
            let current = self.resources_on_planet[&location];
            if (current + amount) == 0 {
                self.resources_on_planet.remove(&location);
            } else {
                self.resources_on_planet.insert(location, current + amount);
            }
        } else if amount != 0 {
            self.resources_on_planet.insert_new(location, amount);
        }
    }

    pub fn remove_resources(&mut self, location: Location) -> i32 {
        let mut result = 0;
        if self.resources_on_planet.contains_key(&location) {
            result = self.resources_on_planet[&location];
            self.resources_on_planet.remove(&location);
        }

        result
    }

    pub fn remove_resources_territory(&mut self, territory: Territory) -> i32 {
        let mut result: i32 = 0;
        for l in territory.locations() {
            result += self.remove_resources(*l);
        }

        result
    }

    pub fn flip_bene_gesserit_advisors_when_alone(&mut self) {
        if self.current_main_phase >= MainPhase::Battle && self.applicable(Rule::DisableNovaFlipping) {
            return;
        }

        let bg = self.get_player(Some(Faction::BeneGesserit));
        if let Some(bg) = bg {
            // A lazy Where enumerated by foreach: the filter is evaluated for each
            // territory right before its body runs, after earlier flips.
            for t in self.map.territories(true) {
                let matches = self.player(bg).special_forces_in_territory(Some(t)) > 0
                    && (t == self.map.polar_sink().territory()
                        || !self.players.iter().any(|p| p.faction != Faction::BeneGesserit && p.any_forces_in_territory(Some(t)) > 0));
                if !matches {
                    continue;
                }

                Player::flip_forces_territory_boolean(self, bg, t, false);
                self.log(vec![express!(Faction::BeneGesserit, " are alone and flip to ", FactionForce::BeneGesserit, " in ", t)
                    .with_log(BeneGesseritFlipped::new(t))
                    .into()]);
            }
        }
    }

    pub fn allow_all_prevented_faction_advantages(&mut self, exceptions: &[FactionAdvantage]) {
        for adv in get_values_except_default(FactionAdvantage::None) {
            if !exceptions.contains(&adv) {
                self.allow(adv);
            }
        }
    }

    pub fn determine_occupation_at_start_or_end_of_turn(&mut self) {
        let current_occupier_of_ecaz_homeworld = self.occupier_of(World::Ecaz);
        let mut updated_occupation: NetDictionary<Homeworld, Faction> = NetDictionary::new();

        for hw in self.map.homeworlds() {
            for i in 0..self.players.len() {
                let player = self.player(self.player_at(i));
                let faction = player.faction;
                if player.controls(self, hw.location(), false) && !player.is_native_homeworld(hw) {
                    updated_occupation.insert_new(hw, faction);

                    if !self.occupies(faction, hw.world()) {
                        self.log(vec![faction.into(), " now occupy ".into(), hw.location().into()]);
                    }
                }
            }
        }

        let previous: Vec<(Homeworld, Faction)> = self.homeworld_occupation.iter().map(|(k, v)| (*k, *v)).collect();
        for (key, value) in previous {
            // ICollection<KeyValuePair>.Contains: the key is present with an equal value
            if updated_occupation.get(&key) != Some(&value) {
                self.log(vec![value.into(), " no longer occupies ".into(), key.location().into()]);
            }
        }

        self.homeworld_occupation = updated_occupation;

        self.check_if_shipment_permissions_should_be_revoked();
        self.check_if_occupier_takes_vidal(current_occupier_of_ecaz_homeworld);
        self.let_factions_discard_surplus_cards();
    }

    pub fn check_if_shipment_permissions_should_be_revoked(&mut self) {
        if !self.has_high_threshold(Faction::Guild) && self.shipment_permissions.len() > 0 {
            self.shipment_permissions.clear();
            self.log(vec!["Only ".into(), Faction::Guild.into(), " can ship cross/from planet now".into()]);
        }
    }

    pub fn let_factions_discard_surplus_cards(&mut self) {
        let factions: Vec<Faction> = self.players.iter().filter(|p| p.hand_size_exceeded(self)).map(|p| p.faction).collect();
        self.factions_that_must_discard.extend(factions);
        if self.factions_that_must_discard.len() > 0 {
            self.phase_before_discarding = self.current_phase;
            self.enter(Phase::Discarding, true);
        }
    }

    pub fn check_if_occupier_takes_vidal(&mut self, previous_occupier_of_ecaz_homeworld: Option<PlayerIndex>) {
        if self.vidal().is_none() {
            return;
        }

        let occupier_of_ecaz_homeworld = self.occupier_of(World::Ecaz);
        if let Some(occupier) = occupier_of_ecaz_homeworld {
            let vidal = self.vidal().expect("vidal");
            if !self.player(occupier).leaders.contains(&vidal) {
                self.take_vidal(occupier, VidalMoment::WhileEcazWorldIsOccupied);
            }
        } else if let Some(previous) = previous_occupier_of_ecaz_homeworld {
            let vidal = self.vidal();
            if vidal.map_or(false, |v| self.player(previous).leaders.contains(&v)) {
                let vidal = self.vidal().expect("vidal");
                // List.Remove: the first equal element
                let leaders = &mut self.player_mut(previous).leaders;
                if let Some(pos) = leaders.iter().position(|l| *l == vidal) {
                    leaders.remove(pos);
                }
                let faction = self.player(previous).faction;
                let vidal = self.vidal();
                self.log(vec![faction.into(), " lose ".into(), vidal.into()]);
            }
        }
    }

    pub fn set_aside_vidal(&mut self) {
        if self.vidal().is_none() || self.player_to_set_aside_vidal.is_none() {
            return;
        }

        let vidal = self.vidal().expect("vidal");
        let p = self.player_to_set_aside_vidal.expect("player to set aside vidal");
        if self.is_alive(Hero::Leader(vidal)) && self.player(p).leaders.contains(&vidal) {
            let leaders = &mut self.player_mut(p).leaders;
            if let Some(pos) = leaders.iter().position(|l| *l == vidal) {
                leaders.remove(pos);
            }
            let vidal = self.vidal();
            self.log(vec![vidal.into(), " is set aside".into()]);
        }

        self.player_to_set_aside_vidal = None;
        self.when_to_set_aside_vidal = VidalMoment::None;
    }

    /// Discards the card a player plays as Karama and logs the play as one aside:
    /// "X play [Karama]: <effect>". A card played as Karama that isn't one says
    /// so: "X play [Baliset] as a Karama card: <effect>".
    pub fn play_karma(&mut self, player: PlayerIndex, card: Option<TreacheryCard>, effect: Vec<ExpressionElement>) {
        self.discard_player_treachery_card_boolean(Some(player), card, false);
        let faction = self.player(player).faction;
        let m = Game::karma_played(faction, card, effect);
        self.log(vec![m.into()]);
    }

    pub fn karma_played(initiator: Faction, card: Option<TreacheryCard>, effect: Vec<ExpressionElement>) -> Message {
        let card_or_karma: ExpressionElement = match card {
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

    /// Discards the first card of `card_type` in the player's hand (for Bene Gesserit, a Useless
    /// card can stand in for Karma). Fails if they have none.
    pub fn discard(&mut self, player: PlayerIndex, card_type: TreacheryCardType) -> GameResult {
        let bene_gesserit_karma = card_type == TreacheryCardType::Karma && self.player(player).is(Faction::BeneGesserit);
        let card = self
            .player(player)
            .treachery_cards
            .iter()
            .copied()
            .find(|x| if bene_gesserit_karma { x.type_() == TreacheryCardType::Karma || x.type_() == TreacheryCardType::Useless } else { x.type_() == card_type })
            .ok_or_else(|| GameError::new(format!("{} have no {} card to discard", self.player(player).faction, card_type)))?;

        self.discard_player_treachery_card_boolean(Some(player), Some(card), true);
        Ok(())
    }

    /// `log` is false when the caller logs the card's play itself, such as PlayKarma.
    /// Discards `card` from the hand that holds it. Fails if more than one hand holds it.
    pub fn discard_treachery_card_boolean(&mut self, card: Option<TreacheryCard>, log: bool) -> GameResult {
        let Some(c) = card else { return Ok(()) };

        let mut holders = self.player_indices().filter(|p| self.player(*p).has(Some(c)));
        let player = holders.next();
        if holders.next().is_some() {
            game_error!("Card {} is in more than one hand", c.id());
        }
        self.discard_player_treachery_card_boolean(player, Some(c), log);
        self.register_known(c);
        Ok(())
    }

    pub fn discard_player_treachery_card_boolean(&mut self, player: Option<PlayerIndex>, card: Option<TreacheryCard>, log: bool) {
        if let (Some(player), Some(card)) = (player, card) {
            let faction = self.player(player).faction;
            if log {
                let mut fact = BattleFact::new(BattleFactKind::Discard, faction);
                fact.card = Some(card);
                self.log(vec![express!(faction, " discard ", card).with_fact(fact).into()]);
            }
            // List.Remove: the first equal element
            let cards = &mut self.player_mut(player).treachery_cards;
            if let Some(pos) = cards.iter().position(|c| *c == card) {
                cards.remove(pos);
            }
            self.treachery_discard_pile.as_mut().expect("treachery discard pile").put_on_top(card);
            self.register_known(card);
            self.recently_discarded.insert_new(card, faction);
            self.stone(Milestone::Discard);

            if !matches!(
                card.type_(),
                TreacheryCardType::Poison | TreacheryCardType::ProjectileAndPoison | TreacheryCardType::PoisonTooth
            ) {
                return;
            }

            let ecaz = self.get_player(Some(Faction::Ecaz));
            let Some(ecaz) = ecaz else { return };
            if !self.player(ecaz).has_high_threshold_0(self) {
                return;
            }
            self.log(vec![
                Faction::Ecaz.into(),
                " get ".into(),
                Payment::of(3).into(),
                " from the discarded ".into(),
                card.into(),
            ]);
            let p = self.player_mut(ecaz);
            p.resources += 3;
        }
    }

    /// Replays a savegame's events into a new game, or says why it can't.
    pub fn load(state: &GameState, participation: &Shared<Participation>, perform_validation: bool, is_host: bool) -> Result<Game, Message> {
        Game::check_version(state.version).map_err(|e| express!(e))?;
        let mut game = Game::new_int32_participation(state.version, participation);
        let events = state.events.as_ref().ok_or_else(|| express!("The savegame has no events"))?;
        for (nr, e) in events.iter().enumerate() {
            e.initialize_game(&game);
            if let Some(message) = e.execute(&mut game, perform_validation, is_host) {
                return Err(express!(e.event_type().name(), "(", nr as i32, "):", message));
            }
        }
        Ok(game)
    }

    pub fn log(&mut self, expression: Vec<ExpressionElement>) {
        // A message logged on its own keeps what it carries, such as a route.
        if expression.len() == 1 {
            if let ExpressionElement::Message(_) = &expression[0] {
                let Some(ExpressionElement::Message(message)) = expression.into_iter().next() else { unreachable!() };
                self.current_report_mut().express_message(*message);
                return;
            }
        }
        self.current_report_mut().express(expression);
    }

    pub fn log_if(&mut self, condition: bool, expression: Vec<ExpressionElement>) {
        if condition {
            self.current_report_mut().express(expression);
        }
    }

    pub fn log_to(&mut self, faction: Faction, expression: Vec<ExpressionElement>) {
        self.current_report_mut().express_to(faction, expression);
    }

    pub fn stone(&mut self, m: Milestone) {
        self.recent_milestones.push(m);
    }

    pub fn set_recent_payment(&mut self, amount: i32, by: Faction, to: Faction, reason: &GameEvent) {
        if amount > 0 {
            self.recently_paid.push(Payment::new_full(amount, by, to, Some(reason.clone())));
        }
    }

    pub fn set_recent_payment_int32_faction_game_event(&mut self, amount: i32, by: Faction, reason: &GameEvent) {
        self.set_recent_payment(amount, by, Faction::None, reason);
    }

    pub fn clear_recent_payments(&mut self) {
        // StoredRecentlyPaid = RecentlyPaid (a reference swap); RecentlyPaid = []
        self.stored_recently_paid = std::mem::take(&mut self.recently_paid);
    }

    pub fn has_recent_payment_for(&self, t: GameEventType) -> bool {
        self.recently_paid.iter().any(|p| p.reason.as_ref().map_or(false, |r| r.event_type() == t))
    }

    pub fn has_quite_recent_payment_to(&self, f: Faction) -> bool {
        self.recently_paid.iter().any(|p| p.to == f) || self.stored_recently_paid.iter().any(|p| p.to == f)
    }

    pub fn quite_recent_payment_to(&self, f: Faction) -> Option<Payment> {
        let res = self
            .recently_paid
            .iter()
            .find(|p| p.to == f)
            .or_else(|| self.stored_recently_paid.iter().find(|p| p.to == f));
        res.cloned()
    }

    pub fn recently_paid_total_amount(&self) -> i32 {
        self.recently_paid.iter().map(|p| p.amount).sum::<i32>()
    }

    pub fn occupies(&self, f: Faction, w: World) -> bool {
        if f != Faction::None {
            let hw_occupation = self.homeworld_occupation.keys().find(|hw| hw.world() == w).copied();
            if let Some(hw) = hw_occupation {
                return self.homeworld_occupation.get(&hw) == Some(&f);
            }
        }

        false
    }

    pub fn occupier_of(&self, w: World) -> Option<PlayerIndex> {
        let hw_occupation = self.homeworld_occupation.keys().find(|hw| hw.world() == w).copied();
        match hw_occupation {
            Some(hw) => self.get_player(self.homeworld_occupation.get(&hw).copied()),
            None => None,
        }
    }

    pub fn is_in_storm(&self, l: Location) -> bool {
        l.sector(&self.map) == self.sector_in_storm
    }

    pub fn is_in_storm_territory(&self, t: Territory) -> bool {
        t.locations().iter().any(|l| self.is_in_storm(*l))
    }

    pub fn is_occupied(&self, l: Location) -> bool {
        self.players.iter().any(|p| p.occupies(self, Some(l)))
    }

    pub fn is_occupied_territory(&self, t: Territory) -> bool {
        self.players.iter().any(|p| p.occupies_territory(self, Some(t)))
    }

    pub fn is_occupied_world(&self, world: World) -> bool {
        self.occupier_of(world).is_some()
    }

    pub fn is_occupied_by_faction_or_their_ally(&self, world: World, p: PlayerIndex) -> bool {
        let occupier = self.occupier_of(world);
        match occupier {
            Some(o) => o == p || self.player(o).ally == self.player(p).faction,
            None => false,
        }
    }

    pub fn is_occupied_by_faction_or_their_ally_world_faction(&self, world: World, f: Faction) -> bool {
        let occupier = self.occupier_of(world);
        match occupier {
            Some(o) => self.player(o).is(f) || self.player(o).ally == f,
            None => false,
        }
    }

    pub fn contains_conflicting_ally(&self, initiator: PlayerIndex, to: Location) -> bool {
        let ini = self.player(initiator);
        if !ini.has_ally() || self.map.polar_sink() == to || ini.faction == Faction::Ecaz || ini.ally == Faction::Ecaz {
            return false;
        }

        let ally = ini.allied_player(self);

        if ini.ally == Faction::BeneGesserit && self.applicable(Rule::AdvisorsDontConflictWithAlly) {
            return self.player(ally.expect("ally")).forces_in_territory(Some(to.territory())) > 0;
        }

        self.player(ally.expect("ally")).any_forces_in_territory(Some(to.territory())) > 0
    }

    pub fn owner_of(&self, hero: Hero) -> Option<PlayerIndex> {
        // Enumerable.Contains<IHero> over a List<Leader>: Equals on each leader
        self.players.iter().position(|p| p.leaders.iter().any(|l| Hero::Leader(*l) == hero)).map(|i| self.player_at(i))
    }

    pub fn owner_of_treachery_card(&self, card: TreacheryCard) -> Option<PlayerIndex> {
        self.players.iter().position(|p| p.has(Some(card))).map(|i| self.player_at(i))
    }

    pub fn owner_of_treachery_card_type(&self, card_type: TreacheryCardType) -> Option<PlayerIndex> {
        self.players.iter().position(|p| p.treachery_cards.iter().any(|c| c.type_() == card_type)).map(|i| self.player_at(i))
    }

    pub fn owner_of_location(&self, stronghold: Location) -> Option<PlayerIndex> {
        match self.stronghold_ownership.get(&stronghold) {
            Some(value) => self.get_player(Some(*value)),
            None => None,
        }
    }

    pub fn nr_of_occupants_excluding_faction(&self, l: Location, to_exclude: Faction) -> i32 {
        let factions: Vec<Faction> = self.battalions_in(l).iter().filter(|b| b.can_occupy()).map(|b| b.faction).collect();
        self.count_different_factions(&factions, to_exclude)
    }

    pub fn nr_of_occupants_excluding_faction_territory_faction(&self, t: Territory, to_exclude: Faction) -> i32 {
        let factions: Vec<Faction> = t
            .locations()
            .iter()
            .flat_map(|l| self.battalions_in(*l).into_iter().filter(|b| b.can_occupy()).map(|b| b.faction))
            .collect();
        self.count_different_factions(&factions, to_exclude)
    }

    pub fn count_different_factions(&self, factions: &[Faction], to_exclude: Faction) -> i32 {
        let mut result = 0;
        let ecaz_ally = self.get_ally(Some(Faction::Ecaz));
        let mut ecaz_or_ecaz_ally_counted = false;

        for faction in factions.iter().copied().distinct() {
            if faction == Faction::Ecaz || Some(faction) == ecaz_ally {
                if !ecaz_or_ecaz_ally_counted && to_exclude != Faction::Ecaz && Some(to_exclude) != ecaz_ally {
                    result += 1;
                }

                ecaz_or_ecaz_ally_counted = true;
            } else if faction != to_exclude {
                result += 1;
            }
        }

        result
    }

    pub fn has_ornithopters(&self, p: PlayerIndex) -> bool {
        (if self.applicable(Rule::MovementBonusRequiresOccupationBeforeMovement) {
            self.factions_with_ornithopters_at_start_of_movement.contains(&self.player(p).faction)
        } else {
            self.occupies_arrakeen_or_carthag(p)
        }) || (self.current_flight_used.as_ref().map_or(false, |f| f.move_three_territories)
            && self.current_flight_used.as_ref().expect("current flight used").player_opt() == Some(p))
            || (self.current_flight_discovery_used.is_some() && self.current_flight_discovery_used.as_ref().expect("current flight discovery used").player_opt() == Some(p))
    }

    pub fn determine_maximum_move_distance(&self, p: PlayerIndex, moved: &[Battalion]) -> i32 {
        let player = self.player(p);
        let choam_extra_move_bonus = if player.faction == Faction::Choam && self.choam_has_extra_move { 1 } else { 0 };

        let planetology_bonus = if self.current_planetology.as_ref().map_or(false, |pl| pl.add_one_to_movement)
            && self.current_planetology.as_ref().expect("current planetology").initiator() == player.faction
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
    pub fn locations_with_any_forces_not_in_storm(&self, p: PlayerIndex) -> impl Iterator<Item = Location> {
        // `p` is dereferenced inside the predicate, per element, as in C#.
        self.map
            .locations(true)
            .filter(move |l| l.sector(&self.map) != self.sector_in_storm && self.player(p).any_forces_in(Some(*l)) > 0)
    }

    pub fn is_not_full(&self, p: PlayerIndex, l: Location) -> bool {
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
                && player.ally == Faction::Ecaz
                && self.player(player.allied_player(self).expect("allied player")).occupying_forces_in(Some(l.territory())) > 0)
            || self.nr_of_occupants_excluding_faction(l, player.faction) < 2
    }

    pub fn everyone_acted_or_passed(&self) -> bool {
        self.has_acted_or_passed.len() == self.players.len()
    }

    pub fn assisted_note_keeping_enabled(&self, p: Option<PlayerIndex>) -> bool {
        match p {
            Some(p) => {
                self.applicable(Rule::AssistedNotekeeping)
                    || (self.player(p).is(Faction::Atreides) && self.applicable(Rule::AssistedNotekeepingForAtreides))
            }
            None => false,
        }
    }

    pub fn has_storm_prescience(&self, p: Option<PlayerIndex>) -> bool {
        let Some(p) = p else { return false };
        let player = self.player(p);
        self.applicable(Rule::FremenDeterminesStorm)
            && !self.prevented(FactionAdvantage::FremenStormPrescience)
            && (player.faction == Faction::Fremen
                || (player.ally == Faction::Fremen && self.fremen_shares_prescience)
                || self.has_deal(player.faction, DealType::ShareStormPrescience))
    }

    pub fn next_storm_will_pass_over(&self, l: Location, storm_moves: i32) -> bool {
        let mut i = 1;
        while i <= storm_moves {
            if (self.sector_in_storm + i) % Map::NUMBER_OF_SECTORS == l.sector(&self.map) {
                return true;
            }
            i += 1;
        }

        false
    }

    pub fn distance_from_storm(&self, l: Location) -> i32 {
        for i in 1..=Map::NUMBER_OF_SECTORS {
            if (self.sector_in_storm + i) % Map::NUMBER_OF_SECTORS == l.sector(&self.map) {
                return i;
            }
        }

        -1
    }

    pub fn has_high_threshold(&self, f: Faction) -> bool {
        let player = self.get_player(Some(f));
        player.map_or(false, |p| self.player(p).has_high_threshold_0(self))
    }

    pub fn has_high_threshold_faction_world(&self, f: Faction, w: World) -> bool {
        let player = self.get_player(Some(f));
        player.map_or(false, |p| self.player(p).has_high_threshold(self, w))
    }

    pub fn has_low_threshold(&self, f: Faction) -> bool {
        let player = self.get_player(Some(f));
        player.map_or(false, |p| self.player(p).has_low_threshold_0(self))
    }

    pub fn has_resource_deck_prescience(&self, p: Option<PlayerIndex>) -> bool {
        let Some(p) = p else { return false };
        let player = self.player(p);
        !self.prevented(FactionAdvantage::AtreidesSpiceBlowPrescience)
            && !self.has_low_threshold(Faction::Atreides)
            && (player.faction == Faction::Atreides
                || (player.ally == Faction::Atreides && self.atreides_shares_prescience)
                || self.has_deal(player.faction, DealType::ShareResourceDeckPrescience))
    }

    pub fn has_bidding_prescience(&self, p: PlayerIndex) -> bool {
        let is_publicly_known =
            self.current_auction_type == AuctionType::RicheseOnceAround || self.current_auction_type == AuctionType::RicheseSilent;
        let player = self.player(p);

        is_publicly_known
            || self.occupies(player.faction, World::Atreides)
            || self.occupies(player.ally, World::Atreides)
            || (!self.prevented(FactionAdvantage::AtreidesBiddingPrescience)
                && (player.faction == Faction::Atreides
                    || (player.ally == Faction::Atreides && self.atreides_shares_prescience)
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
                    factions_in_play.clone().any(|f| f == l.faction())
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

    /// A `DateTime` (`DateTimeOffset.DateTime`: the clock time, offset dropped),
    /// returned as a `DateTimeOffset` with offset 0. `DateTime.MinValue` is tick 0.
    pub fn first_action(&self) -> DateTimeOffset {
        match self.history.first() {
            Some(e) => DateTimeOffset::new(e.time().ticks, 0),
            None => DateTimeOffset::default(),
        }
    }

    pub fn last_action(&self) -> DateTimeOffset {
        match self.history.last() {
            Some(e) => DateTimeOffset::new(e.time().ticks, 0),
            None => DateTimeOffset::default(),
        }
    }
}

impl Game {
    /// The player at position `index` of `Players`.
    pub fn player_at(&self, index: usize) -> PlayerIndex {
        PlayerIndex { index }
    }

    /// Every player of `Players`, in order.
    pub fn player_indices(&self) -> impl Iterator<Item = PlayerIndex> + Clone + 'static {
        (0..self.players.len()).map(|index| PlayerIndex { index })
    }

    /// The player `p` refers to.
    pub fn player(&self, p: PlayerIndex) -> &Player {
        &self.players[p.index]
    }

    /// `p`, for mutation.
    pub fn player_mut(&mut self, p: PlayerIndex) -> &mut Player {
        &mut self.players[p.index]
    }

    /// The index of a player object of this game (`this` inside a `Player` method).
    pub fn index_of(&self, p: &Player) -> Option<PlayerIndex> {
        self.players.iter().position(|x| std::ptr::eq(x, p)).map(|index| PlayerIndex { index })
    }
}

/// What `Map` needs to know about the game (see map.rs).
impl crate::map::MapGame for Game {
    fn sector_in_storm(&self) -> i32 {
        self.sector_in_storm
    }

    fn history_count(&self) -> i32 {
        self.history.len() as i32
    }

    fn applicable(&self, rule: Rule) -> bool {
        Game::applicable(self, rule)
    }

    fn forces_battalion_factions(&self) -> Vec<(Location, Vec<Faction>)> {
        // game.Forces(): every location (in Map.Locations order), with the factions of its battalions
        self.forces(false).iter().map(|(l, bs)| (*l, bs.iter().map(|b| b.faction).collect())).collect()
    }

    fn player_ally(&self, f: Faction) -> Option<Faction> {
        self.get_player(Some(f)).map(|p| self.player(p).ally)
    }

    fn nr_of_occupants_excluding_faction(&self, l: Location, f: Faction) -> i32 {
        Game::nr_of_occupants_excluding_faction(self, l, f)
    }

    fn current_blocked_territories(&self) -> Vec<Territory> {
        self.current_blocked_territories.clone()
    }
}
