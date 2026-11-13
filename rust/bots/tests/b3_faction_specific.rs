//! The faction-specific decisions (`bot/factions.rs`): every `determine_*` method, plus
//! `pass_ambassador_activated`, `i_want_to_be_fighters_in`, `i_want_to_announce_battle_in`,
//! `best_voice` and `best_prescience`, with the bot's draw count after each call.
//!
//! The games are in `golden/knowledge-*.jsonl.gz`; `golden/b3-*.jsonl.gz` holds the probes and
//! their expected results. The probes run whatever the phase, so they also reach states bots never
//! decide in: decisions outside their phase (and the panics they cause there), ResourcesAudited and
//! ChoamRemoveForce (rare in bot games), and Voice and Prescience against every player.
//!
//! The expected results are snapshots of the current behavior. After a deliberate change, rewrite
//! them with `UPDATE_GOLDENS=1 cargo test --release -p treachery-bots` and review the diff
//! (see `common/mod.rs`).

mod common;

use treachery_bots::bot::EventKind;
use treachery_bots::prelude::*;

use common::{run_probes, Bots};

fn ev<T: EventKind>(e: Rc<T>) -> String {
    treachery_engine::json::serialize(&T::into_event(e))
}

fn opt<T: EventKind>(e: Option<Rc<T>>) -> String {
    e.map_or_else(|| "null".to_string(), ev)
}

fn b(v: bool) -> String {
    if v { "true".into() } else { "false".into() }
}

/// A bool as the snapshots spell it.
fn title_bool(v: bool) -> &'static str {
    if v { "True" } else { "False" }
}

fn voice_plan(p: &VoicePlan) -> String {
    format!(
        "battle:{} voice:{} weapon:{} defense:{} survive:{} zero:{}",
        if p.battle.is_some() { "set" } else { "null" },
        ev(p.voice.clone()),
        p.weapon_to_use.map_or(-1, |c| c.id()),
        p.defense_to_use.map_or(-1, |c| c.id()),
        title_bool(p.player_hero_will_certainly_survive),
        title_bool(p.opponent_hero_will_certainly_be_zero)
    )
}

fn call(game: &Game, bot: &mut ClassicBot, m: &str, a: &[String]) -> String {
    let n = |i: usize| a[i][1..].parse::<i32>().unwrap();
    let player = |i: usize| game.player_at(n(i) as usize);
    let territory = |i: usize| game.map.territory_lookup().find(n(i)).unwrap();
    match m {
        "DetermineSetShipmentPermission" => opt(bot.determine_set_shipment_permission(game)),
        "DetermineEmperorDiscarded" => opt(bot.determine_emperor_discarded(game)),
        "DetermineReplacedCardWon" => ev(bot.determine_replaced_card_won(game)),
        "DetermineIxianRemovedCardFromAuction" => ev(bot.determine_ixian_removed_card_from_auction(game)),
        "DetermineIxianSwappedCardOnBid" => ev(bot.determine_ixian_swapped_card_on_bid(game)),
        "DetermineIxianSelectedStartingCard" => ev(bot.determine_ixian_selected_starting_card(game)),
        "DeterminePerformHmsMovement" => ev(bot.determine_perform_hms_movement(game)),
        "DeterminePerformHmsPlacement" => ev(bot.determine_perform_hms_placement(game)),
        "DetermineTakeLosses" => ev(bot.determine_take_losses(game)),
        "DeterminePerformFremenSetup" => ev(bot.determine_perform_fremen_setup(game)),
        "DetermineFremenSentMonster" => ev(bot.determine_fremen_sent_monster(game)),
        "DetermineFremenRidesMonster" => ev(bot.determine_fremen_rides_monster(game)),
        "DetermineBeneGesseritPlacement" => ev(bot.determine_bene_gesserit_placement(game)),
        "DetermineBeneGesseritFlip" => ev(bot.determine_bene_gesserit_flip(game)),
        "DetermineBeneGesseritBattleAnnouncement" => opt(bot.determine_bene_gesserit_battle_announcement(game)),
        "DetermineBeneGesseritAccompanies" => ev(bot.determine_bene_gesserit_accompanies(game)),
        "DetermineBeneGesseritPrediction" => ev(bot.determine_bene_gesserit_prediction(game)),
        "DetermineVoice" => opt(bot.determine_voice(game)),
        "DeterminePrescience" => opt(bot.determine_prescience(game)),
        "DetermineSetIncreasedRevivalLimits" => opt(bot.determine_set_increased_revival_limits(game)),
        "DetermineFaceDancerRevealed" => ev(bot.determine_face_dancer_revealed(game)),
        "DetermineFaceDanced" => ev(bot.determine_face_danced(game)),
        "DetermineFaceDancerReplaced" => ev(bot.determine_face_dancer_replaced(game)),
        "DetermineRequestTleilaxuRevival" => opt(bot.determine_request_tleilaxu_revival(game)),
        "DetermineAcceptOrCancelTleilaxuRevival" => opt(bot.determine_accept_or_cancel_tleilaxu_revival(game)),
        "DetermineResourcesAudited" => opt(bot.determine_resources_audited(game)),
        "DetermineChoamEconomics" => opt(bot.determine_choam_economics(game)),
        "DetermineChoamRemoveForce" => opt(bot.determine_choam_remove_force(game)),
        "DetermineExtortionPrevented" => opt(bot.determine_extortion_prevented(game)),
        "DeterminePerformMoritaniSetup" => ev(bot.determine_perform_moritani_setup(game)),
        "DetermineTerrorPlanted" => opt(bot.determine_terror_planted(game)),
        "DetermineTerrorRevealed" => ev(bot.determine_terror_revealed(game)),
        "DetermineAmbassadorPlaced" => opt(bot.determine_ambassador_placed(game)),
        "DetermineAmbassadorActivated" => opt(bot.determine_ambassador_activated(game)),
        "PassAmbassadorActivated" => opt(bot.pass_ambassador_activated(game)),
        "DetermineRicheseAnnouncesBlackMarket" => ev(bot.determine_richese_announces_black_market(game)),
        "DetermineRicheseRevealedNoField" => opt(bot.determine_richese_revealed_no_field(game)),
        "DetermineRicheseAnnouncesAuction" => ev(bot.determine_richese_announces_auction(game)),
        "DetermineRicheseSpecifiesAuction" => ev(bot.determine_richese_specifies_auction(game)),
        "DetermineRicheseKeepsUnsoldCard" => ev(bot.determine_richese_keeps_unsold_card(game)),
        "DetermineCaptured" => ev(bot.determine_captured(game)),
        "IWantToBeFightersIn" => b(bot.i_want_to_be_fighters_in(game, territory(0))),
        "IWantToAnnounceBattleIn" => b(bot.i_want_to_announce_battle_in(game, territory(0))),
        "BestVoice" => voice_plan(&bot.best_voice(game, None, player(1), player(2))),
        "BestPrescience" => {
            let strength = a[2][1..].parse::<f32>().unwrap();
            let aspect = PrescienceAspect(n(3));
            bot.best_prescience(game, player(0), player(1), strength, aspect, territory(4)).0.to_string()
        }
        _ => panic!("unknown probe method {m}"),
    }
}

fn run(name: &str) {
    run_probes("b3", name, Bots::Shared, |game, b, m, a| call(game, b, m, a));
}

#[test]
fn b3_00008() {
    run("00008");
}

#[test]
fn b3_77_00001() {
    run("77-00001");
}

#[test]
fn b3_77_00002() {
    run("77-00002");
}

#[test]
fn b3_77_00004() {
    run("77-00004");
}
