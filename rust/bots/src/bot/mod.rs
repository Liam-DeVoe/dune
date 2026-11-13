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

use crate::prelude::*;
use treachery_engine::compat::cs;

/// The fields of every partial file, in file order. One bot per faction lives for the whole game (hazards.md 16.8).
pub struct ClassicBot {
    // Not stored: every method takes the game as a parameter. SetGameAndPlayer only rebinds `player`.
    pub player: PlayerIndex,
    pub param: BotParameters,
    pub random: SharedRandom,
    pub voice_plan: Option<VoicePlan>,
    pub price_set_earlier: NetDictionary<Hero, i32>,
    pub turn_when_revival_was_requested: i32,
    pub decided_shipment_action: ShipmentDecision,
    pub decided_shipment: Option<Rc<Shipment>>,
    pub final_destination: Option<Location>,
}

/// The event type `T` of `do_`: the declared return type of a `Determine*` method.
/// Implemented for every event type the dispatch uses.
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

impl ClassicBot {
    /// A bot for `player`. Without `random`, it uses an unseeded generator.
    pub fn new(_game: &Game, player: PlayerIndex, param: BotParameters, random: Option<SharedRandom>) -> Self {
        ClassicBot {
            player,
            param,
            // C# `new LoggedRandom()` is unseeded (only the server omits the random, phase 3).
            random: random.unwrap_or_else(|| SharedRandom::from_seed(unseeded_seed())),
            voice_plan: None,
            // C#: `= []` (Bot_FactionSpecific.cs:781)
            price_set_earlier: NetDictionary::new(),
            // C#: `= -1` (Bot_FactionSpecific.cs:782)
            turn_when_revival_was_requested: -1,
            decided_shipment_action: ShipmentDecision::None,
            decided_shipment: None,
            final_destination: None,
        }
    }

    pub const LOG_BOT_INFO: bool = false;

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

    pub fn set_game_and_player(&mut self, _game: &Game, player: PlayerIndex) {
        self.player = player;
    }

    pub fn determine_highest_priority_in_phase_action(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent> {
        let mut action: Option<GameEvent> = None;

        if self.do_(game, |b, g| b.determine_nexus_played(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_deal_cancelled(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_accept_or_cancel_tleilaxu_revival(g), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_thought_answered(g)), &mut action, events) ||
            self.do_(game, |b, g| b.determine_thought(g), &mut action, events)
        {
            return action;
        }

        None
    }

    pub fn determine_high_priority_in_phase_action(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent> {
        let mut action: Option<GameEvent> = None;

        if self.do_(game, |b, g| b.determine_voice(g), &mut action, events) {
            return action;
        }

        None
    }

    pub fn determine_middle_priority_in_phase_action(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent> {
        let mut action: Option<GameEvent> = None;

        if self.do_(game, |b, g| b.determine_metheor_played(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_amal_played(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_recruits_played(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_set_increased_revival_limits(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_set_shipment_permission(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_deal_accepted(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_request_tleilaxu_revival(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_distrans_used(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_juice_played(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_portable_antidote_used(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_diplomacy(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_switched_skilled_leader(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_retreat(g), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_hms_advantage_chosen(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_planetology(g)), &mut action, events) ||
            self.do_(game, |b, g| b.determine_prescience(g), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_card_given(g)), &mut action, events) ||
            self.do_(game, |b, g| b.determine_karma_shipment_prevention(g), &mut action, events)
        {
            return action;
        }

        None
    }

    pub fn determine_low_priority_in_phase_action(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent> {
        let mut action: Option<GameEvent> = None;

        if self.do_(game, |b, g| b.determine_deal_offered(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_faction_trade_offered(g), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_skill_assigned(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_storm_dialled(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_clair_voyance_answered(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_traitors_selected(g)), &mut action, events) ||
            self.do_(game, |b, g| b.determine_storm_spell_played(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_testing_station_used(g), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_thumper_played(g)), &mut action, events) ||
            self.do_(game, |b, g| b.determine_harvester_played(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_alliance_broken(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_alliance_offered(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_alliance_passed(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_alliance_permissions(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_charity_claimed(g), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_black_market_bid(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_bid(g)), &mut action, events) ||
            self.do_(game, |b, g| b.determine_revival(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_delay(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_raise_dead_played(g), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_shipment(g)), &mut action, events) ||
            self.do_(game, |b, g| b.determine_caravan(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_move(g), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_battle_initiated(g)), &mut action, events) ||
            self.do_(game, |b, g| b.determine_clairvoyance(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_battle(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_treachery_called(g), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_battle_concluded(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_mulligan_performed(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_replaced_card_won(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_audited(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_audit_cancelled(g)), &mut action, events) ||
            self.do_(game, |b, g| b.determine_card_traded(g), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_rock_was_melted(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_residual_played(g)), &mut action, events) ||
            self.do_(game, |b, g| b.determine_flight_used(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_flight_discovery_used(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_discarded_searched_announced(g), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_discarded_searched(g)), &mut action, events) ||
            self.do_(game, |b, g| b.determine_discarded_taken(g), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_bureaucracy(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_nexus_card_drawn(g)), &mut action, events) ||
            self.do_(game, |b, g| b.determine_extortion_prevented(g), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_discarded(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_traitor_discarded(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_alliance_by_terror(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_alliance_by_ambassador(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_loser_concluded(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_divide_resources(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_divide_resources_accepted(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_battle_claimed(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_nexus_voted(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_discovery_revealed(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_discovery_entered(g)), &mut action, events) ||
            self.do_(game, |b, g| b.determine_resources_transferred(g), &mut action, events) ||

            //Choam
            self.do_(game, |b, g| b.determine_choam_economics(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_choam_remove_force(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_resources_audited(g), &mut action, events) ||

            //Harkonnen
            self.do_(game, |b, g| Some(b.determine_captured(g)), &mut action, events) ||
            self.do_(game, |b, g| b.determine_karma_hand_swap_initiated(g), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_karma_hand_swap(g)), &mut action, events) ||

            //BeneGesserit
            self.do_(game, |b, g| Some(b.determine_bene_gesserit_accompanies(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_bene_gesserit_prediction(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_bene_gesserit_flip(g)), &mut action, events) ||
            self.do_(game, |b, g| b.determine_bene_gesserit_battle_announcement(g), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_bene_gesserit_placement(g)), &mut action, events) ||

            //Fremen
            self.do_(game, |b, g| Some(b.determine_fremen_rides_monster(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_fremen_sent_monster(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_perform_fremen_setup(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_take_losses(g)), &mut action, events) ||

            //Ixian
            self.do_(game, |b, g| Some(b.determine_ixian_removed_card_from_auction(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_ixian_selected_starting_card(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_ixian_swapped_card_on_bid(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_perform_hms_placement(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_perform_hms_movement(g)), &mut action, events) ||

            //Emperor
            self.do_(game, |b, g| b.determine_karma_free_revival(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_emperor_discarded(g), &mut action, events) ||

            //Moritani
            self.do_(game, |b, g| b.determine_terror_planted(g), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_terror_revealed(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_perform_moritani_setup(g)), &mut action, events) ||

            //Ecaz
            self.do_(game, |b, g| b.determine_ambassador_placed(g), &mut action, events) ||
            self.do_(game, |b, g| b.determine_ambassador_activated(g), &mut action, events) ||

            //Tleilaxu
            self.do_(game, |b, g| Some(b.determine_face_dancer_revealed(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_face_danced(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_face_dancer_replaced(g)), &mut action, events) ||

            //Richese
            self.do_(game, |b, g| Some(b.determine_richese_announces_black_market(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_richese_announces_auction(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_richese_specifies_auction(g)), &mut action, events) ||
            self.do_(game, |b, g| Some(b.determine_richese_keeps_unsold_card(g)), &mut action, events) ||
            self.do_(game, |b, g| b.determine_richese_revealed_no_field(g), &mut action, events)
        {
            return action;
        }

        None
    }

    pub fn determine_end_phase_action(&mut self, game: &mut Game, events: &[GameEventType]) -> Option<GameEvent> {
        let mut action: Option<GameEvent> = None;

        if self.do_(game, |b, g| Some(b.determine_end_phase(g)), &mut action, events) {
            return action;
        }

        None
    }

    pub fn determine_end_phase(&self, game: &Game) -> Rc<EndPhase> {
        Rc::new(EndPhase::new(game, self.faction(game)))
    }

    /// `T` is the declared return type of `method` ([`EventKind`]). `method` gets the bot and the
    /// live game, because a decision may change both (bots-plan.md, "Bots are not pure").
    pub fn do_<T: EventKind>(
        &mut self,
        game: &mut Game,
        method: impl FnOnce(&mut ClassicBot, &mut Game) -> Option<Rc<T>>,
        action: &mut Option<GameEvent>,
        allowed_actions: &[GameEventType],
    ) -> bool {
        if action.is_none() && allowed_actions.contains(&T::TYPE) {
            // A decision that panics (it reads a card the game doesn't have, for example) is logged
            // and counts as no decision. Random draws and bot or game state changes made before the
            // panic stay.
            match decide(|| method(self, game)) {
                Ok(result) => *action = result.map(T::into_event),
                Err(e) => Self::log_error(&format!("--error occured -->{e}"), vec![]),
            }

            if let Some(a) = action.clone() {
                // C# `action.Validate()` and `GetMessage()` read the event's own `Game`. That is the live
                // game, except for an event built from an object of another game: `DealOffered.Acceptance`
                // after an undo builds the `DealAccepted` on the offer's (pre-undo) game.
                let own = game.enter_own_game(a.base().game.get());
                let game: &Game = &own;
                let error = a.validate(game);
                if let Some(error) = error {
                    // The interpolation runs outside the try: GetMessage() or ToString() can throw out of Do.
                    let msg = format!(
                        "--invalid decision ({})--> {}: {}",
                        event_type_full_name(&a),
                        a.get_message(game).to_cs_string(),
                        error.to_cs_string()
                    );
                    Self::log_error(&msg, vec![]);
                } else {
                    // LogInfo returns at once, but its argument is evaluated, so GetMessage() runs (and may throw).
                    let msg = format!("--valid decision ({})--> {}", event_type_full_name(&a), a.get_message(game).to_cs_string());
                    Self::log_info(&msg, vec![]);
                    return true;
                }
            }
        }

        false
    }

    /// `LogBotInfo` is false, so this does nothing. Callers must still evaluate every argument,
    /// because an argument can throw (bots-plan.md, "LogInfo and LogError arguments are evaluated").
    pub fn log_info(_msg: &str, _pars: Vec<ExpressionElement>) {
        if !Self::LOG_BOT_INFO {
            return;
        }
        // Unreachable in C# too (LogBotInfo is a const false).
    }

    pub fn log_info_message(_message: Option<&Message>) {
        if !Self::LOG_BOT_INFO {
            return;
        }
        // Unreachable in C# too (LogBotInfo is a const false).
    }

    /// The server sets `Message.DefaultDescriber`, so production runs
    /// `Console.WriteLine(DefaultDescriber.Format(msg, pars))`, and `Skin.Format` returns `msg`
    /// unchanged when `pars` is empty (both callers) and catches any exception otherwise. The
    /// oracle writes to `TextWriter.Null`, which doesn't format either. So `LogError` never throws
    /// (bots-plan.md, Decisions 2), and the text goes nowhere. The arguments are evaluated by the
    /// caller, and those can throw. [`string_format_check`] is what `Console.WriteLine(msg, pars)`
    /// on a formatting console would do; nothing calls it in production.
    pub fn log_error(msg: &str, pars: Vec<ExpressionElement>) {
        let _ = (msg, pars);
    }

    pub fn d(&self, _game: &Game, amount: i32, sides: i32) -> i32 {
        if amount == 0 || sides == 0 {
            return 0;
        }

        let mut result = 0i32;
        for _ in 0..amount {
            // Random.Next throws for a negative `sides`, after LoggedRandom counted the draw.
            result = result.wrapping_add(self.random.next(sides).wrapping_add(1));
        }
        result
    }
}

/// A seed from the clock, for a bot created without a random generator. Not reproducible.
fn unseeded_seed() -> i32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    (nanos as u64 % i32::MAX as u64) as i32
}

/// `action.GetType()` in an interpolated string: the full type name.
fn event_type_full_name(e: &GameEvent) -> String {
    format!("Treachery.Shared.{}", e.event_type().name())
}

/// Runs a bot decision, turning a panic into its message. The bots aren't written to avoid every
/// state they can't handle, so a failed decision is isolated instead of ending the game.
pub(crate) fn decide<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|p| dotnet_compat::exception::panic_message(p.as_ref()))
}

/// The parse that .NET's `string.Format(format, args)` does (`StringBuilder.AppendFormatHelper`),
/// for its exceptions only: the formatted text is never used. Offsets count UTF-16 code units.
/// Probed on .NET 10 (escape-hatch cases in the bot tests).
pub fn string_format_check(format: &str, arg_count: usize) {
    const INDEX_LIMIT: i32 = 1_000_000;
    const WIDTH_LIMIT: i32 = 1_000_000;
    let f: Vec<u16> = format.encode_utf16().collect();
    let len = f.len();

    fn fail(pos: usize, reason: &str) -> ! {
        cs::throw(
            "FormatException",
            format!("Input string was not in a correct format. Failure to parse near offset {pos}. {reason}"),
        )
    }
    const ENDS_PREMATURELY: &str = "Format item ends prematurely.";
    const EXPECTED_DIGIT: &str = "Expected an ASCII digit.";
    const UNEXPECTED_CLOSING: &str = "Unexpected closing brace without a corresponding opening brace.";

    let next = |pos: &mut usize| -> u16 {
        *pos += 1;
        if *pos >= len {
            fail(*pos, ENDS_PREMATURELY);
        }
        f[*pos]
    };
    let is_digit = |c: u16| (b'0' as u16..=b'9' as u16).contains(&c);
    let (open, close, space) = (b'{' as u16, b'}' as u16, b' ' as u16);

    let mut pos = 0usize;
    loop {
        // Literal text up to the next format item; "{{" and "}}" are escapes.
        let mut ch;
        loop {
            if pos >= len {
                return;
            }
            match f[pos..].iter().position(|&c| c == open || c == close) {
                None => return,
                Some(n) => pos += n,
            }
            let brace = f[pos];
            ch = next(&mut pos);
            if brace == ch {
                pos += 1;
                continue;
            }
            if brace != open {
                fail(pos, UNEXPECTED_CLOSING);
            }
            break;
        }

        // Index
        let mut index = ch.wrapping_sub(b'0' as u16) as i32;
        if !(0..10).contains(&index) {
            fail(pos, EXPECTED_DIGIT);
        }
        ch = next(&mut pos);
        if ch != close {
            while is_digit(ch) && index < INDEX_LIMIT {
                index = index * 10 + (ch - b'0' as u16) as i32;
                ch = next(&mut pos);
            }
            while ch == space {
                ch = next(&mut pos);
            }

            // Alignment
            if ch == b',' as u16 {
                loop {
                    ch = next(&mut pos);
                    if ch != space {
                        break;
                    }
                }
                if ch == b'-' as u16 {
                    ch = next(&mut pos);
                }
                let mut width = ch.wrapping_sub(b'0' as u16) as i32;
                if !(0..10).contains(&width) {
                    fail(pos, EXPECTED_DIGIT);
                }
                ch = next(&mut pos);
                while is_digit(ch) && width < WIDTH_LIMIT {
                    width = width * 10 + (ch - b'0' as u16) as i32;
                    ch = next(&mut pos);
                }
                while ch == space {
                    ch = next(&mut pos);
                }
            }

            // Format string
            if ch != close {
                if ch != b':' as u16 {
                    fail(pos, ENDS_PREMATURELY);
                }
                loop {
                    ch = next(&mut pos);
                    if ch == close {
                        break;
                    }
                    if ch == open {
                        fail(pos, ENDS_PREMATURELY);
                    }
                }
            }
        }
        pos += 1;

        if index as usize >= arg_count {
            cs::throw(
                "FormatException",
                "Index (zero based) must be greater than or equal to zero and less than the size of the argument list.".to_string(),
            );
        }
    }
}
