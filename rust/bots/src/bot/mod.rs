mod alliance;
mod battle;
mod bidding;
mod deals;
mod factions;
pub use factions::*;
mod knowledge;
pub use knowledge::*;
mod movement;
pub use movement::*;
mod other;
mod shipment;
pub use shipment::*;

use std::collections::HashMap;

use crate::prelude::*;
use treachery_engine::compat::cs;

/// The bot of one player. One bot per faction lives for the whole game, and its fields carry
/// plans from one decision to the next. The game itself is passed to every method.
pub struct ClassicBot {
    pub player: PlayerIndex,
    pub param: BotParameters,
    pub random: SharedRandom,
    /// The Voice the bot used in the current battle, for its battle plan.
    pub voice_plan: Option<VoicePlan>,
    /// The price asked earlier for reviving a hero, for Tleilaxu revival requests.
    pub price_set_earlier: HashMap<Hero, i32>,
    pub turn_when_revival_was_requested: i32,
    /// The shipment chosen while deciding, with the move planned after it.
    pub decided_shipment_action: ShipmentDecision,
    pub decided_shipment: Option<Rc<Shipment>>,
    pub final_destination: Option<Location>,
}

/// An event type a decision method can return.
pub trait EventKind: Sized {
    const TYPE: GameEventType;
    fn into_event(e: Rc<Self>) -> GameEvent;
}

macro_rules! event_kind {
    ($($t:ident),* $(,)?) => {
        $(impl EventKind for $t {
            const TYPE: GameEventType = GameEventType::$t;
            fn into_event(e: Rc<Self>) -> GameEvent { GameEvent::$t(e) }
        })*
    };
}

event_kind!(
    AcceptOrCancelTleilaxuRevival,
    AllianceBroken,
    AllianceByAmbassador,
    AllianceByTerror,
    AllianceOffered,
    AlliancePassed,
    AllyPermission,
    AmalPlayed,
    AmbassadorActivated,
    AmbassadorPlaced,
    AuditCancelled,
    Audited,
    Battle,
    BattleClaimed,
    BattleConcluded,
    BattleInitiated,
    BeneGesseritAccompanies,
    BeneGesseritBattleAnnouncement,
    BeneGesseritFlip,
    BeneGesseritPrediction,
    Bid,
    BlackMarketBid,
    Bureaucracy,
    Captured,
    Caravan,
    CardGiven,
    CardTraded,
    CharityClaimed,
    ChoamEconomics,
    ChoamRemoveForce,
    ClairVoyanceAnswered,
    ClairVoyancePlayed,
    DealAccepted,
    DealOffered,
    Diplomacy,
    Discarded,
    DiscardedSearched,
    DiscardedSearchedAnnounced,
    DiscardedTaken,
    DiscoveryEntered,
    DiscoveryRevealed,
    DistransUsed,
    DivideResources,
    DivideResourcesAccepted,
    EmperorDiscarded,
    EndPhase,
    ExtortionPrevented,
    FaceDanced,
    FaceDancerReplaced,
    FaceDancerRevealed,
    FactionTradeOffered,
    FlightDiscoveryUsed,
    FlightUsed,
    FremenRidesMonster,
    FremenSentMonster,
    GuildDelay,
    HMSAdvantageChosen,
    HarvesterPlayed,
    IxianRemovedCardFromAuction,
    IxianSelectedStartingCard,
    IxianSwappedCardOnBid,
    JuicePlayed,
    KarmaFreeRevival,
    KarmaHandSwap,
    KarmaHandSwapInitiated,
    KarmaShipmentPrevention,
    LoserConcluded,
    MetheorPlayed,
    Move,
    MulliganPerformed,
    NexusCardDrawn,
    NexusPlayed,
    NexusVoted,
    PerformBeneGesseritPlacement,
    PerformFremenSetup,
    PerformHmsMovement,
    PerformHmsPlacement,
    PerformMoritaniSetup,
    Planetology,
    PortableAntidoteUsed,
    Prescience,
    RaiseDeadPlayed,
    RecruitsPlayed,
    ReplacedCardWon,
    RequestTleilaxuRevival,
    ResidualPlayed,
    ResourcesAudited,
    ResourcesTransferred,
    Retreat,
    Revival,
    RicheseAnnouncesAuction,
    RicheseAnnouncesBlackMarket,
    RicheseKeepsUnsoldCard,
    RicheseRevealedNoField,
    RicheseSpecifiesAuction,
    RockWasMelted,
    SetIncreasedRevivalLimits,
    SetShipmentPermission,
    Shipment,
    SkillAssigned,
    StormDialled,
    StormSpellPlayed,
    SwitchedSkilledLeader,
    TakeLosses,
    TerrorPlanted,
    TerrorRevealed,
    TestingStationUsed,
    Thought,
    ThoughtAnswered,
    ThumperPlayed,
    TraitorDiscarded,
    TraitorsSelected,
    TreacheryCalled,
    Voice
);


/// The five rounds in which the bots are asked for an event, in the order they are asked.
/// `End` is only asked when bots act as host.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Priority {
    Highest,
    High,
    Middle,
    Low,
    End,
}

impl Priority {
    pub const ALL: [Priority; 5] = [Priority::Highest, Priority::High, Priority::Middle, Priority::Low, Priority::End];
}

/// What a decision method returns: an event, or maybe an event.
pub trait IntoDecision {
    type Event: EventKind;
    fn into_option(self) -> Option<Rc<Self::Event>>;
}

impl<T: EventKind> IntoDecision for Rc<T> {
    type Event = T;
    fn into_option(self) -> Option<Rc<T>> {
        Some(self)
    }
}

impl<T: EventKind> IntoDecision for Option<Rc<T>> {
    type Event = T;
    fn into_option(self) -> Option<Rc<T>> {
        self
    }
}

/// An event a decision method came up with.
#[derive(Clone, Debug)]
pub enum Decision {
    Valid(GameEvent),
    /// The game rejects it, so the bot tries its next decision method.
    Invalid(GameEvent),
}

impl Decision {
    pub fn valid(self) -> Option<GameEvent> {
        match self {
            Decision::Valid(e) => Some(e),
            Decision::Invalid(_) => None,
        }
    }
}

/// Asks each decision method in turn. The first one that comes up with a valid event ends the
/// round. A method whose event the game rejects is passed over like one that came up with nothing.
macro_rules! first_decision {
    ($bot:ident, $game:ident, $allowed:ident; $($method:ident),* $(,)?) => {{
        $(
            if let Some(Decision::Valid(event)) = $bot.try_decide($game, $allowed, |b, g| b.$method(g)) {
                return Some(event);
            }
        )*
        None
    }};
}

impl ClassicBot {
    /// A bot for `player`, drawing from `random`.
    pub fn new(player: PlayerIndex, param: BotParameters, random: SharedRandom) -> Self {
        ClassicBot {
            player,
            param,
            random,
            voice_plan: None,
            price_set_earlier: HashMap::new(),
            turn_when_revival_was_requested: -1,
            decided_shipment_action: ShipmentDecision::None,
            decided_shipment: None,
            final_destination: None,
        }
    }

    /// Points the bot at another player, for when seats change.
    pub fn set_player(&mut self, player: PlayerIndex) {
        self.player = player;
    }

    pub fn faction(&self, game: &Game) -> Faction {
        game.player(self.player).faction
    }

    pub fn ally(&self, game: &Game) -> Faction {
        game.player(self.player).ally
    }

    pub fn allied_player(&self, game: &Game) -> Option<PlayerIndex> {
        game.player(self.player).allied_player(game)
    }

    pub fn resources(&self, game: &Game) -> i32 {
        game.player(self.player).resources
    }

    /// The event the bot wants to submit in this round, if any. `allowed` lists the event types
    /// the player may submit now. Deciding changes the bot's own plans, never the game.
    pub fn decide(&mut self, game: &Game, priority: Priority, allowed: &[GameEventType]) -> Option<GameEvent> {
        match priority {
            Priority::Highest => first_decision!(self, game, allowed;
                determine_nexus_played,
                determine_deal_cancelled,
                determine_accept_or_cancel_tleilaxu_revival,
                determine_thought_answered,
                determine_thought,
            ),
            Priority::High => first_decision!(self, game, allowed; determine_voice),
            Priority::Middle => first_decision!(self, game, allowed;
                determine_metheor_played,
                determine_amal_played,
                determine_recruits_played,
                determine_set_increased_revival_limits,
                determine_set_shipment_permission,
                determine_deal_accepted,
                determine_request_tleilaxu_revival,
                determine_distrans_used,
                determine_juice_played,
                determine_portable_antidote_used,
                determine_diplomacy,
                determine_switched_skilled_leader,
                determine_retreat,
                determine_hms_advantage_chosen,
                determine_planetology,
                determine_prescience,
                determine_card_given,
                determine_karma_shipment_prevention,
            ),
            Priority::Low => first_decision!(self, game, allowed;
                determine_deal_offered,
                determine_faction_trade_offered,
                determine_skill_assigned,
                determine_storm_dialled,
                determine_clair_voyance_answered,
                determine_traitors_selected,
                determine_storm_spell_played,
                determine_testing_station_used,
                determine_thumper_played,
                determine_harvester_played,
                determine_alliance_broken,
                determine_alliance_offered,
                determine_alliance_passed,
                determine_alliance_permissions,
                determine_charity_claimed,
                determine_black_market_bid,
                determine_bid,
                determine_revival,
                determine_delay,
                determine_raise_dead_played,
                determine_shipment,
                determine_caravan,
                determine_move,
                determine_battle_initiated,
                determine_clairvoyance,
                determine_battle,
                determine_treachery_called,
                determine_battle_concluded,
                determine_mulligan_performed,
                determine_replaced_card_won,
                determine_audited,
                determine_audit_cancelled,
                determine_card_traded,
                determine_rock_was_melted,
                determine_residual_played,
                determine_flight_used,
                determine_flight_discovery_used,
                determine_discarded_searched_announced,
                determine_discarded_searched,
                determine_discarded_taken,
                determine_bureaucracy,
                determine_nexus_card_drawn,
                determine_extortion_prevented,
                determine_discarded,
                determine_traitor_discarded,
                determine_alliance_by_terror,
                determine_alliance_by_ambassador,
                determine_loser_concluded,
                determine_divide_resources,
                determine_divide_resources_accepted,
                determine_battle_claimed,
                determine_nexus_voted,
                determine_discovery_revealed,
                determine_discovery_entered,
                determine_resources_transferred,
                // CHOAM
                determine_choam_economics,
                determine_choam_remove_force,
                determine_resources_audited,
                // Harkonnen
                determine_captured,
                determine_karma_hand_swap_initiated,
                determine_karma_hand_swap,
                // Bene Gesserit
                determine_bene_gesserit_accompanies,
                determine_bene_gesserit_prediction,
                determine_bene_gesserit_flip,
                determine_bene_gesserit_battle_announcement,
                determine_bene_gesserit_placement,
                // Fremen
                determine_fremen_rides_monster,
                determine_fremen_sent_monster,
                determine_perform_fremen_setup,
                determine_take_losses,
                // Ixian
                determine_ixian_removed_card_from_auction,
                determine_ixian_selected_starting_card,
                determine_ixian_swapped_card_on_bid,
                determine_perform_hms_placement,
                determine_perform_hms_movement,
                // Emperor
                determine_karma_free_revival,
                determine_emperor_discarded,
                // Moritani
                determine_terror_planted,
                determine_terror_revealed,
                determine_perform_moritani_setup,
                // Ecaz
                determine_ambassador_placed,
                determine_ambassador_activated,
                // Tleilaxu
                determine_face_dancer_revealed,
                determine_face_danced,
                determine_face_dancer_replaced,
                // Richese
                determine_richese_announces_black_market,
                determine_richese_announces_auction,
                determine_richese_specifies_auction,
                determine_richese_keeps_unsold_card,
                determine_richese_revealed_no_field,
            ),
            Priority::End => first_decision!(self, game, allowed; determine_end_phase),
        }
    }

    pub fn determine_end_phase(&self, game: &Game) -> Rc<EndPhase> {
        Rc::new(EndPhase::new(game, self.faction(game)))
    }

    /// Runs `decide` if the player may submit its event type, and checks the event it comes up
    /// with. `None` if the type isn't allowed, or `decide` came up with nothing or panicked.
    /// Random draws and changes to the bot made before a panic stay.
    pub fn try_decide<R: IntoDecision>(
        &mut self,
        game: &Game,
        allowed: &[GameEventType],
        decide: impl FnOnce(&mut ClassicBot, &Game) -> R,
    ) -> Option<Decision> {
        if !allowed.contains(&<R::Event as EventKind>::TYPE) {
            return None;
        }

        // The bots aren't written to avoid every state they can't handle (a card the game doesn't
        // have, for example), so a decision that panics counts as no decision.
        let event = guarded(|| decide(self, game)).ok()?.into_option()?;
        let event = <R::Event as EventKind>::into_event(event);

        let error = event.validate(game);
        // Building the event's message can throw, which ends the bot's round with that exception.
        let _ = event.get_message(game);
        Some(if error.is_none() { Decision::Valid(event) } else { Decision::Invalid(event) })
    }

    /// The sum of `amount` dice with `sides` sides each.
    pub fn d(&self, amount: i32, sides: i32) -> i32 {
        if amount == 0 || sides == 0 {
            return 0;
        }
        // A negative `sides` throws after the draw is counted.
        (0..amount).map(|_| self.random.next(sides) + 1).sum()
    }
}

/// Runs a bot decision, turning a panic into its message.
pub(crate) fn guarded<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|p| cs::panic_message(p.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    impl ClassicBot {
        /// A Thought without a card, which the game rejects.
        fn invalid_thought(&self, game: &Game) -> Rc<Thought> {
            Rc::new(Thought::new(game, Faction::Atreides))
        }

        fn end_phase(&self, game: &Game) -> Rc<EndPhase> {
            Rc::new(EndPhase::new(game, Faction::Atreides))
        }

        fn invalid_then_valid(&mut self, game: &Game, allowed: &[GameEventType]) -> Option<GameEvent> {
            first_decision!(self, game, allowed; invalid_thought, end_phase)
        }
    }

    /// A decision the game rejects no longer ends the round: the next method gets its turn.
    #[test]
    fn an_invalid_decision_falls_through_to_the_next_method() {
        let game = Game::new();
        let mut bot = ClassicBot::new(game.player_at(0), BotParameters::for_faction(Faction::Atreides), SharedRandom::from_seed(1));
        let allowed = [GameEventType::Thought, GameEventType::EndPhase];
        assert!(matches!(bot.try_decide(&game, &allowed, |b, g| b.invalid_thought(g)), Some(Decision::Invalid(_))));
        assert!(matches!(bot.invalid_then_valid(&game, &allowed), Some(GameEvent::EndPhase(_))));
    }
}
