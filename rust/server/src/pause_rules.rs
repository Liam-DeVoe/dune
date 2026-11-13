//! What each pause between phases waits for before the server continues it
//! (`server/pauses.rs`).
//!
//! Waiting for a player reveals that they have an option, so a player is only waited for when
//! everyone can see they have one. Abilities that depend on a secret card anyone might hold get a
//! countdown instead, which any player can hold. Countdown conditions only use what is fixed when
//! the game starts: settings, the factions playing and the turn number.
//!
//! Only reads game state, except that it calls `get_applicable_events` for `CardTraded` and the
//! revival actions, which can advance the battle sequence.

use treachery_engine::events::DiscoveryEntered;
use treachery_engine::events::TerrorPlanted;
use treachery_engine::events::TestingStationUsed;
use treachery_engine::events::{GameEvent, GameEventType};
use treachery_engine::prelude::*;

#[derive(Clone, Debug)]
pub struct PauseRequirements {
    /// Players who may act here, each of whom must act or pass.
    pub wait_for: Vec<PlayerIndex>,
    /// Whether the pause runs a countdown that any player can hold.
    pub countdown: bool,
}

pub const COUNTDOWN_MILLISECONDS: i32 = 5000;

fn nothing() -> PauseRequirements {
    PauseRequirements { wait_for: vec![], countdown: false }
}

fn wait(players: Vec<PlayerIndex>) -> PauseRequirements {
    PauseRequirements { wait_for: players, countdown: false }
}

fn with(players: Vec<PlayerIndex>, countdown: bool) -> PauseRequirements {
    PauseRequirements { wait_for: players, countdown }
}

impl PauseRequirements {
    fn with_countdown(mut self, countdown: bool) -> Self {
        self.countdown = self.countdown || countdown;
        self
    }
}

/// None when the current phase is not a pause this controls; the host keeps sole control.
pub fn determine(g: &mut Game) -> Option<PauseRequirements> {
    let amal = amal_in_play(g);
    let nexus = g.applicable(Rule::NexusCards);

    Some(match g.current_phase {
        Phase::SelectingFactions => wait(players(g, |g, p| g.player(p).faction == Faction::None)),
        Phase::TradingFactions => wait(g.player_indices().collect()),

        Phase::BeginningOfStorm => with(players(g, |g, p| DiscoveryEntered::valid_targets(g, p).next().is_some()), amal),
        Phase::MetheorAndStormSpell => {
            let turn = g.current_turn;
            with(players(g, |g, p| TestingStationUsed::can_be_played(g, p)), turn > 1)
        }
        Phase::StormReport => end_of_phase(g, vec![]),

        Phase::Thumper => {
            let countdown = amal || thumper_in_play(g) || g.is_playing(Faction::Fremen) || secret_ally(g, nexus, Faction::Fremen);
            with(vec![], countdown)
        }
        Phase::HarvesterA | Phase::HarvesterB => with(vec![], true),
        Phase::BlowReport => {
            let others = fremen_sandworm(g);
            end_of_phase(g, others).with_countdown(amal)
        }

        Phase::BeginningOfCharity => {
            let countdown = amal || g.is_playing(Faction::Choam) || (g.is_playing(Faction::BeneGesserit) && g.applicable(Rule::BeneGesseritAutoCharity));
            with(vec![], countdown)
        }
        Phase::ClaimingCharity => with(vec![], true),
        Phase::CharityReport => end_of_phase(g, vec![]),

        Phase::BeginningOfBidding => {
            let countdown = amal || g.is_playing(Faction::Richese) || g.is_playing(Faction::Ixian);
            end_of_phase(g, vec![]).with_countdown(countdown)
        }
        Phase::ReplacingCardJustWon => wait(present(&[g.get_player(Some(g.faction_that_may_replace_bought_card))])),
        Phase::WaitingForNextBiddingRound => wait(present(&[atreides(g)])),
        Phase::BiddingReport => {
            let others = present(&[tleilaxu_limits(g), emperor_discard(g)]);
            end_of_phase(g, others)
        }

        Phase::BeginningOfResurrection => {
            let countdown = amal || g.applicable(Rule::Expansion3TreacheryCards) || secret_ally(g, nexus, Faction::Fremen);
            with(present(&[tleilaxu_limits(g).or_else(|| tleilaxu_karma(g))]), countdown)
        }
        Phase::Resurrection => wait(revivers(g)),
        Phase::ResurrectionReport => {
            let others = present(&[ecaz(g)]);
            end_of_phase(g, others)
        }

        Phase::BeginningOfShipAndMove => {
            let countdown = amal || g.applicable(Rule::RicheseTreacheryCards) || g.is_playing(Faction::Guild);
            end_of_phase(g, vec![]).with_countdown(countdown)
        }
        Phase::ShipmentAndMoveConcluded => {
            let end = end_of_phase(g, vec![]);
            let no_field = players(g, |g, p| g.player(p).no_field_is_active());
            merge(end, no_field)
        }

        Phase::BeginningOfBattle => with(vec![], amal),
        Phase::CancellingTraitor => with(vec![], true),
        Phase::BattleReport if g.current_moment != MainPhaseMoment::End => nothing(),
        Phase::BattleReport => end_of_phase(g, vec![]),

        Phase::BeginningOfCollection => with(present(&[moritani_removing_tokens(g)]), amal || g.is_playing(Faction::Ecaz)),
        Phase::CollectionReport => {
            let others = present(&[moritani_removing_tokens(g)]);
            let countdown = secret_ally(g, nexus, Faction::Choam);
            end_of_phase(g, others).with_countdown(countdown)
        }

        Phase::Extortion => with(vec![], true),
        Phase::Contemplate => with(present(&[choam_in_mentat(g), moritani_planting(g)]), secret_ally(g, nexus, Faction::Harkonnen)),
        Phase::TurnConcluded => end_of_phase(g, vec![]),

        _ => return None,
    })
}

/// A revival counts as its player being done with the revival pause.
pub fn finishes_pause(g: &Game, e: &GameEvent) -> bool {
    g.current_phase == Phase::Resurrection && matches!(e, GameEvent::Revival(_))
}

fn players(g: &Game, can_act: impl Fn(&Game, PlayerIndex) -> bool) -> Vec<PlayerIndex> {
    g.player_indices().filter(|&p| can_act(g, p)).collect()
}

fn present(players: &[Option<PlayerIndex>]) -> Vec<PlayerIndex> {
    players.iter().flatten().copied().collect()
}

fn merge(mut r: PauseRequirements, others: Vec<PlayerIndex>) -> PauseRequirements {
    for p in others {
        if !r.wait_for.contains(&p) {
            r.wait_for.push(p);
        }
    }
    r
}

/// An end-of-phase pause: CHOAM may discard and CHOAM may trade a card with its ally, plus
/// whoever else may act there.
fn end_of_phase(g: &mut Game, others: Vec<PlayerIndex>) -> PauseRequirements {
    let mut result = vec![];

    if let Some(choam) = g.get_player(Some(Faction::Choam)) {
        if !g.player(choam).treachery_cards.is_empty() && !g.prevented(FactionAdvantage::ChoamDiscarding) {
            result.push(choam);
        }
    }

    // The trade's conditions are all public. Either side may start it.
    let candidates: Vec<PlayerIndex> = g
        .player_indices()
        .filter(|&p| {
            let pl = g.player(p);
            (pl.faction == Faction::Choam && pl.has_ally()) || pl.ally == Faction::Choam
        })
        .collect();
    for p in candidates {
        if !result.contains(&p) && g.get_applicable_events(Some(p), false).contains(&GameEventType::CardTraded) {
            result.push(p);
        }
    }

    merge(PauseRequirements { wait_for: result, countdown: false }, others)
}

/// Players who may revive, ask Tleilaxu for a revival or answer such a request, or (for
/// Tleilaxu) still set revival limits.
fn revivers(g: &mut Game) -> Vec<PlayerIndex> {
    let revival_actions = [GameEventType::Revival, GameEventType::RequestTleilaxuRevival, GameEventType::AcceptOrCancelTleilaxuRevival];
    let limits = tleilaxu_limits(g);

    let mut result = vec![];
    for p in g.player_indices().collect::<Vec<_>>() {
        if Some(p) == limits || g.get_applicable_events(Some(p), false).iter().any(|t| revival_actions.contains(t)) {
            result.push(p);
        }
    }
    result
}

fn amal_in_play(g: &Game) -> bool {
    g.applicable(Rule::ExpansionTreacheryCardsAmal)
}

fn thumper_in_play(g: &Game) -> bool {
    g.applicable(Rule::ExpansionTreacheryCardsExceptPBandSSandAmal)
}

/// A Nexus card's "secret ally" mode, usable while that faction is not playing.
fn secret_ally(g: &Game, nexus: bool, f: Faction) -> bool {
    nexus && !g.is_playing(f)
}

fn special_karma(g: &Game, f: Faction) -> Option<PlayerIndex> {
    let p = g.get_player(Some(f))?;
    (g.applicable(Rule::AdvancedKarama) && !g.player(p).special_karma_power_used && !g.karma_prevented(f)).then_some(p)
}

/// Fremen's once-per-game Karma sandworm. Only during Spice Blow, from turn 2.
fn fremen_sandworm(g: &Game) -> Vec<PlayerIndex> {
    present(&[if g.current_turn > 1 { special_karma(g, Faction::Fremen) } else { None }])
}

/// Tleilaxu's once-per-game Karma, which stops a faction reviving.
fn tleilaxu_karma(g: &Game) -> Option<PlayerIndex> {
    special_karma(g, Faction::Tleilaxu)
}

/// Tleilaxu may raise revival limits at the end of Bidding or the start of Revival. The limits
/// are reset when Ship & Move starts, so any set limits were set this turn.
fn tleilaxu_limits(g: &Game) -> Option<PlayerIndex> {
    let p = g.get_player(Some(Faction::Tleilaxu))?;
    (g.players.len() > 1
        && !g.prevented(FactionAdvantage::TleilaxuIncreasingRevivalLimits)
        && g.factions_with_increased_revival_limits.as_ref().nre().is_empty())
    .then_some(p)
}

/// The Emperor may pay 2 to discard a card at high threshold. Spice is secret, so it is not
/// checked.
fn emperor_discard(g: &Game) -> Option<PlayerIndex> {
    let p = g.get_player(Some(Faction::Emperor))?;
    g.player(p).has_high_threshold_0(g).then_some(p)
}

fn atreides(g: &Game) -> Option<PlayerIndex> {
    if g.prevented(FactionAdvantage::AtreidesBiddingPrescience) {
        None
    } else {
        g.get_player(Some(Faction::Atreides))
    }
}

/// Ecaz places ambassadors at the end of Revival. Its spice is secret, so it is not checked.
fn ecaz(g: &Game) -> Option<PlayerIndex> {
    if g.prevented(FactionAdvantage::EcazAmbassadors) {
        None
    } else {
        g.get_player(Some(Faction::Ecaz))
    }
}

fn moritani_removing_tokens(g: &Game) -> Option<PlayerIndex> {
    let p = g.get_player(Some(Faction::Moritani))?;
    TerrorPlanted::may_remove_tokens(g, p).then_some(p)
}

/// Moritani plants a terror token once per turn. Its unplaced tokens are secret, so they are not
/// checked.
fn moritani_planting(g: &Game) -> Option<PlayerIndex> {
    if !g.moritani_has_performed_terror && !g.prevented(FactionAdvantage::MoritaniPlantingTerror) {
        g.get_player(Some(Faction::Moritani))
    } else {
        None
    }
}

/// In the Mentat phase CHOAM may play Inflation or, with cards, Trip to Gamont.
fn choam_in_mentat(g: &Game) -> Option<PlayerIndex> {
    let p = g.get_player(Some(Faction::Choam))?;
    let inflation = g.economics_status == ChoamEconomicsStatus::None && !g.prevented(FactionAdvantage::ChoamEconomics);
    (inflation || !g.player(p).treachery_cards.is_empty()).then_some(p)
}
