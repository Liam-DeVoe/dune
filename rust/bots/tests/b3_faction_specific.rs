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

use treachery_bots::prelude::*;

use common::{run_probes, Bots};

fn b(v: bool) -> String {
    if v { "true".into() } else { "false".into() }
}

/// A bool as the snapshots spell it.
fn title_bool(v: bool) -> &'static str {
    if v { "True" } else { "False" }
}

/// `voice` is the faction that would use the voice.
fn voice_plan(p: &VoicePlan, voice: Option<Faction>) -> String {
    format!(
        "battle:{} voice:{} weapon:{} defense:{} survive:{} zero:{}",
        if p.battle.is_some() { "set" } else { "null" },
        treachery_engine::json::serialize(&HistoryEntry::new(voice, p.voice.clone())),
        p.weapon_to_use.map_or(-1, |c| c.id()),
        p.defense_to_use.map_or(-1, |c| c.id()),
        title_bool(p.player_hero_will_certainly_survive),
        title_bool(p.opponent_hero_will_certainly_be_zero)
    )
}

fn call(game: &Game, bot: &mut ClassicBot, m: &str, a: &[String]) -> String {
    let n = |i: usize| a[i][1..].parse::<i32>().unwrap();
    let player = |i: usize| game.player_at(n(i) as usize);
    let territory = |i: usize| game.board.map.territory_lookup().find(n(i)).unwrap();
    match m {
        "DetermineSetShipmentPermission" => common::decision(bot, game, bot.determine_set_shipment_permission(game)),
        "DetermineEmperorDiscarded" => common::decision(bot, game, bot.determine_emperor_discarded(game)),
        "DetermineReplacedCardWon" => common::decision(bot, game, bot.determine_replaced_card_won(game)),
        "DetermineIxianRemovedCardFromAuction" => common::decision(bot, game, bot.determine_ixian_removed_card_from_auction(game)),
        "DetermineIxianSwappedCardOnBid" => common::decision(bot, game, bot.determine_ixian_swapped_card_on_bid(game)),
        "DetermineIxianSelectedStartingCard" => common::decision(bot, game, bot.determine_ixian_selected_starting_card(game)),
        "DeterminePerformHmsMovement" => common::decision(bot, game, bot.determine_perform_hms_movement(game)),
        "DeterminePerformHmsPlacement" => common::decision(bot, game, bot.determine_perform_hms_placement(game)),
        "DetermineTakeLosses" => common::decision(bot, game, bot.determine_take_losses(game)),
        "DeterminePerformFremenSetup" => common::decision(bot, game, bot.determine_perform_fremen_setup(game)),
        "DetermineFremenSentMonster" => common::decision(bot, game, bot.determine_fremen_sent_monster(game)),
        "DetermineFremenRidesMonster" => common::decision(bot, game, bot.determine_fremen_rides_monster(game)),
        "DetermineBeneGesseritPlacement" => common::decision(bot, game, bot.determine_bene_gesserit_placement(game)),
        "DetermineBeneGesseritFlip" => common::decision(bot, game, bot.determine_bene_gesserit_flip(game)),
        "DetermineBeneGesseritBattleAnnouncement" => {
            let e = bot.determine_bene_gesserit_battle_announcement(game);
            common::decision(bot, game, e)
        }
        "DetermineBeneGesseritAccompanies" => common::decision(bot, game, bot.determine_bene_gesserit_accompanies(game)),
        "DetermineBeneGesseritPrediction" => common::decision(bot, game, bot.determine_bene_gesserit_prediction(game)),
        "DetermineVoice" => {
            let e = bot.determine_voice(game);
            common::decision(bot, game, e)
        }
        "DeterminePrescience" => {
            let e = bot.determine_prescience(game);
            common::decision(bot, game, e)
        }
        "DetermineSetIncreasedRevivalLimits" => common::decision(bot, game, bot.determine_set_increased_revival_limits(game)),
        "DetermineFaceDancerRevealed" => {
            let e = bot.determine_face_dancer_revealed(game);
            common::decision(bot, game, e)
        }
        "DetermineFaceDanced" => {
            let e = bot.determine_face_danced(game);
            common::decision(bot, game, e)
        }
        "DetermineFaceDancerReplaced" => {
            let e = bot.determine_face_dancer_replaced(game);
            common::decision(bot, game, e)
        }
        "DetermineRequestTleilaxuRevival" => {
            let e = bot.determine_request_tleilaxu_revival(game);
            common::decision(bot, game, e)
        }
        "DetermineAcceptOrCancelTleilaxuRevival" => {
            let e = bot.determine_accept_or_cancel_tleilaxu_revival(game);
            common::decision(bot, game, e)
        }
        "DetermineResourcesAudited" => {
            let e = bot.determine_resources_audited(game);
            common::decision(bot, game, e)
        }
        "DetermineChoamEconomics" => {
            let e = bot.determine_choam_economics(game);
            common::decision(bot, game, e)
        }
        "DetermineChoamRemoveForce" => {
            let e = bot.determine_choam_remove_force(game);
            common::decision(bot, game, e)
        }
        "DetermineExtortionPrevented" => {
            let e = bot.determine_extortion_prevented(game);
            common::decision(bot, game, e)
        }
        "DeterminePerformMoritaniSetup" => common::decision(bot, game, bot.determine_perform_moritani_setup(game)),
        "DetermineTerrorPlanted" => {
            let e = bot.determine_terror_planted(game);
            common::decision(bot, game, e)
        }
        "DetermineTerrorRevealed" => {
            let e = bot.determine_terror_revealed(game);
            common::decision(bot, game, e)
        }
        "DetermineAmbassadorPlaced" => {
            let e = bot.determine_ambassador_placed(game);
            common::decision(bot, game, e)
        }
        "DetermineAmbassadorActivated" => {
            let e = bot.determine_ambassador_activated(game);
            common::decision(bot, game, e)
        }
        "PassAmbassadorActivated" => {
            let e = bot.pass_ambassador_activated(game);
            common::decision(bot, game, e)
        }
        "DetermineRicheseAnnouncesBlackMarket" => common::decision(bot, game, bot.determine_richese_announces_black_market(game)),
        "DetermineRicheseRevealedNoField" => common::decision(bot, game, bot.determine_richese_revealed_no_field(game)),
        "DetermineRicheseAnnouncesAuction" => common::decision(bot, game, bot.determine_richese_announces_auction(game)),
        "DetermineRicheseSpecifiesAuction" => common::decision(bot, game, bot.determine_richese_specifies_auction(game)),
        "DetermineRicheseKeepsUnsoldCard" => common::decision(bot, game, bot.determine_richese_keeps_unsold_card(game)),
        "DetermineCaptured" => {
            let e = bot.determine_captured(game);
            common::decision(bot, game, e)
        }
        "IWantToBeFightersIn" => b(bot.i_want_to_be_fighters_in(game, territory(0))),
        "IWantToAnnounceBattleIn" => b(bot.i_want_to_announce_battle_in(game, territory(0))),
        "BestVoice" => voice_plan(&bot.best_voice(game, None, player(1), player(2)), game.player(player(1)).faction),
        "BestPrescience" => {
            let strength = a[2][1..].parse::<f32>().unwrap();
            let aspect = treachery_engine::enum_type::from_value_or_absent::<PrescienceAspect>(n(3)).unwrap();
            treachery_engine::enum_type::value_or_absent(bot.best_prescience(game, player(0), player(1), strength, aspect, territory(4))).to_string()
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
