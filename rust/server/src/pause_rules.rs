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

use treachery_protocol::PauseReason as R;

#[derive(Clone, Debug)]
pub struct PauseRequirements {
    /// Players who may act here, each of whom must act or pass, with what they may do. A player
    /// with several options appears once per option.
    pub wait_for: Vec<(PlayerId, R)>,
    /// Whether the pause runs a countdown that any player can hold.
    pub countdown: bool,
}

pub const COUNTDOWN_MILLISECONDS: i32 = 5000;

fn nothing() -> PauseRequirements {
    PauseRequirements { wait_for: vec![], countdown: false }
}

fn wait(players: Vec<(PlayerId, R)>) -> PauseRequirements {
    PauseRequirements { wait_for: players, countdown: false }
}

fn with(players: Vec<(PlayerId, R)>, countdown: bool) -> PauseRequirements {
    PauseRequirements { wait_for: players, countdown }
}

impl PauseRequirements {
    fn with_countdown(mut self, countdown: bool) -> Self {
        self.countdown = self.countdown || countdown;
        self
    }

    /// The players waited for, each once, in order.
    pub fn players(&self) -> Vec<PlayerId> {
        let mut result = vec![];
        for &(p, _) in &self.wait_for {
            if !result.contains(&p) {
                result.push(p);
            }
        }
        result
    }

    /// What `p` may do here.
    pub fn reasons_of(&self, p: PlayerId) -> Vec<R> {
        self.wait_for.iter().filter(|(q, _)| *q == p).map(|&(_, r)| r).collect()
    }
}

/// None when the current phase is not a pause this controls; the host keeps sole control.
pub fn determine(g: &mut Game) -> Option<PauseRequirements> {
    let amal = amal_in_play(g);
    let nexus = g.applicable(Rule::NexusCards);

    Some(match g.turn.current_phase {
        Phase::SelectingFactions => wait(tag(players(g, |g, p| g.player(p).faction == None), R::ChooseFaction)),
        Phase::TradingFactions => wait(tag(g.player_indices().collect(), R::TradeFactions)),

        Phase::BeginningOfStorm => with(tag(players(g, |g, p| DiscoveryEntered::valid_targets(g, p).next().is_some()), R::EnterDiscovery), amal),
        Phase::MetheorAndStormSpell => {
            let turn = g.turn.current_turn;
            with(tag(players(g, |g, p| TestingStationUsed::can_be_played(g, p)), R::UseTestingStation), turn > Turn::FIRST)
        }
        Phase::StormReport => end_of_phase(g, vec![]),

        Phase::Thumper => {
            let countdown = amal || thumper_in_play(g) || g.is_playing(Faction::Fremen) || secret_ally(g, nexus, Faction::Fremen);
            with(vec![], countdown)
        }
        Phase::HarvesterA | Phase::HarvesterB => with(vec![], true),
        Phase::BlowReport => {
            let others = tag(fremen_sandworm(g), R::KaramaSandworm);
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
        // The winner, whatever they hold. The engine enters this step on public grounds only, and
        // waiting for just the players who can swap would tell everyone who holds the Ixian Nexus.
        Phase::ReplacingCardJustWon => wait(tag(present(&[auction_winner(g)]), R::ReplaceCardWon)),
        Phase::WaitingForNextBiddingRound => wait(tag(present(&[atreides(g)]), R::LookAtNextCard)),
        Phase::BiddingReport => {
            let mut others = tag(present(&[tleilaxu_limits(g)]), R::SetRevivalLimits);
            others.extend(tag(present(&[emperor_discard(g)]), R::EmperorDiscard));
            end_of_phase(g, others)
        }

        Phase::BeginningOfResurrection => {
            let countdown = amal || g.applicable(Rule::Expansion3TreacheryCards) || secret_ally(g, nexus, Faction::Fremen);
            let mut wait_for = tag(present(&[tleilaxu_limits(g)]), R::SetRevivalLimits);
            wait_for.extend(tag(present(&[tleilaxu_karma(g)]), R::KaramaStopRevival));
            with(wait_for, countdown)
        }
        Phase::Resurrection => wait(revivers(g)),
        Phase::ResurrectionReport => {
            let others = tag(present(&[ecaz(g)]), R::PlaceAmbassador);
            end_of_phase(g, others)
        }

        Phase::BeginningOfShipAndMove => {
            let countdown = amal || g.applicable(Rule::RicheseTreacheryCards) || g.is_playing(Faction::Guild);
            end_of_phase(g, vec![]).with_countdown(countdown)
        }
        Phase::ShipmentAndMoveConcluded => {
            let no_field = tag(players(g, |g, p| g.player(p).no_field_is_active()), R::RevealNoField);
            end_of_phase(g, no_field)
        }

        Phase::BeginningOfBattle => with(vec![], amal),
        Phase::CancellingTraitor => with(vec![], true),
        Phase::BattleReport if g.turn.current_moment != Some(MainPhaseMoment::End) => nothing(),
        Phase::BattleReport => end_of_phase(g, vec![]),

        Phase::BeginningOfCollection => with(tag(present(&[moritani_removing_tokens(g)]), R::RemoveTerror), amal || g.is_playing(Faction::Ecaz)),
        Phase::CollectionReport => {
            let others = tag(present(&[moritani_removing_tokens(g)]), R::RemoveTerror);
            let countdown = secret_ally(g, nexus, Faction::Choam);
            end_of_phase(g, others).with_countdown(countdown)
        }

        Phase::Extortion => with(vec![], true),
        Phase::Contemplate => {
            let mut wait_for = choam_in_mentat(g);
            wait_for.extend(tag(present(&[moritani_planting(g)]), R::PlantTerror));
            with(wait_for, secret_ally(g, nexus, Faction::Harkonnen))
        }
        Phase::TurnConcluded => end_of_phase(g, vec![]),

        _ => return None,
    })
}

/// A revival counts as its player being done with the revival pause.
pub fn finishes_pause(g: &Game, e: &GameEvent) -> bool {
    g.turn.current_phase == Phase::Resurrection && matches!(e, GameEvent::Revival(_))
}

fn players(g: &Game, can_act: impl Fn(&Game, PlayerId) -> bool) -> Vec<PlayerId> {
    g.player_indices().filter(|&p| can_act(g, p)).collect()
}

fn present(players: &[Option<PlayerId>]) -> Vec<PlayerId> {
    players.iter().flatten().copied().collect()
}

fn tag(players: Vec<PlayerId>, reason: R) -> Vec<(PlayerId, R)> {
    players.into_iter().map(|p| (p, reason)).collect()
}

fn merge(mut r: PauseRequirements, others: Vec<(PlayerId, R)>) -> PauseRequirements {
    for w in others {
        if !r.wait_for.contains(&w) {
            r.wait_for.push(w);
        }
    }
    r
}

/// An end-of-phase pause: CHOAM may discard and CHOAM may trade a card with its ally, plus
/// whoever else may act there.
fn end_of_phase(g: &mut Game, others: Vec<(PlayerId, R)>) -> PauseRequirements {
    let mut result = vec![];

    if let Some(choam) = g.get_player(Some(Faction::Choam)) {
        if !g.player(choam).treachery_cards.is_empty() && !g.prevented(FactionAdvantage::ChoamDiscarding) {
            result.push((choam, R::ChoamDiscard));
        }
    }

    // The trade's conditions are all public. Either side may start it.
    let candidates: Vec<PlayerId> = g
        .player_indices()
        .filter(|&p| {
            let pl = g.player(p);
            (pl.faction == Some(Faction::Choam) && pl.has_ally()) || pl.ally == Some(Faction::Choam)
        })
        .collect();
    for p in candidates {
        if g.get_applicable_events(Some(p), false).contains(&GameEventType::CardTraded) {
            result.push((p, R::TradeCard));
        }
    }

    merge(PauseRequirements { wait_for: result, countdown: false }, others)
}

/// Players who may revive, ask Tleilaxu for a revival or answer such a request, or (for
/// Tleilaxu) still set revival limits.
fn revivers(g: &mut Game) -> Vec<(PlayerId, R)> {
    let revival_actions = [GameEventType::Revival, GameEventType::RequestTleilaxuRevival, GameEventType::AcceptOrCancelTleilaxuRevival];
    let limits = tleilaxu_limits(g);

    let mut result = vec![];
    for p in g.player_indices().collect::<Vec<_>>() {
        if Some(p) == limits {
            result.push((p, R::SetRevivalLimits));
        }
        if g.get_applicable_events(Some(p), false).iter().any(|t| revival_actions.contains(t)) {
            result.push((p, R::Revive));
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

fn special_karma(g: &Game, f: Faction) -> Option<PlayerId> {
    let p = g.get_player(Some(f))?;
    (g.applicable(Rule::AdvancedKarama) && !g.player(p).special_karma_power_used && !g.karma_prevented(f)).then_some(p)
}

/// Fremen's once-per-game Karma sandworm. Only during Spice Blow, from turn 2.
fn fremen_sandworm(g: &Game) -> Vec<PlayerId> {
    present(&[if g.turn.current_turn > Turn::FIRST { special_karma(g, Faction::Fremen) } else { None }])
}

/// Tleilaxu's once-per-game Karma, which stops a faction reviving.
fn tleilaxu_karma(g: &Game) -> Option<PlayerId> {
    special_karma(g, Faction::Tleilaxu)
}

/// Tleilaxu may raise revival limits at the end of Bidding or the start of Revival. The limits
/// are reset when Ship & Move starts, so any set limits were set this turn.
fn tleilaxu_limits(g: &Game) -> Option<PlayerId> {
    let p = g.get_player(Some(Faction::Tleilaxu))?;
    (g.players.len() > 1
        && !g.prevented(FactionAdvantage::TleilaxuIncreasingRevivalLimits)
        && g.revival.factions_with_increased_revival_limits.as_ref().expect("factions with increased revival limits").is_empty())
    .then_some(p)
}

/// The Emperor may pay 2 per card to discard cards while Kaitain is at high threshold. The size
/// of the hand is public, so an empty hand is checked; spice is secret, so it is not.
fn emperor_discard(g: &Game) -> Option<PlayerId> {
    let p = g.get_player(Some(Faction::Emperor))?;
    let pl = g.player(p);
    (pl.has_high_threshold(g, World::Emperor) && !pl.treachery_cards.is_empty()).then_some(p)
}

/// Whoever just won the card on auction.
fn auction_winner(g: &Game) -> Option<PlayerId> {
    let winner = g.bidding.winning_bid.as_ref()?.initiator();
    g.get_player(winner)
}

fn atreides(g: &Game) -> Option<PlayerId> {
    if g.prevented(FactionAdvantage::AtreidesBiddingPrescience) {
        None
    } else {
        g.get_player(Some(Faction::Atreides))
    }
}

/// Ecaz places ambassadors at the end of Revival. Its spice is secret, so it is not checked.
fn ecaz(g: &Game) -> Option<PlayerId> {
    if g.prevented(FactionAdvantage::EcazAmbassadors) {
        None
    } else {
        g.get_player(Some(Faction::Ecaz))
    }
}

fn moritani_removing_tokens(g: &Game) -> Option<PlayerId> {
    let p = g.get_player(Some(Faction::Moritani))?;
    TerrorPlanted::may_remove_tokens(g, p).then_some(p)
}

/// Moritani plants a terror token once per turn. Its unplaced tokens are secret, so they are not
/// checked.
fn moritani_planting(g: &Game) -> Option<PlayerId> {
    if !g.contemplate.moritani_has_performed_terror && !g.prevented(FactionAdvantage::MoritaniPlantingTerror) {
        g.get_player(Some(Faction::Moritani))
    } else {
        None
    }
}

/// In the Mentat phase CHOAM may play Inflation or, with cards, Trip to Gamont.
fn choam_in_mentat(g: &Game) -> Vec<(PlayerId, R)> {
    let Some(p) = g.get_player(Some(Faction::Choam)) else { return vec![] };
    let mut result = vec![];
    if g.charity.economics_status == None && !g.prevented(FactionAdvantage::ChoamEconomics) {
        result.push((p, R::Inflation));
    }
    if !g.player(p).treachery_cards.is_empty() {
        result.push((p, R::TripToGamont));
    }
    result
}
