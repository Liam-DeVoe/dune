//! The pauses between phases: the server continues one once every player who may act in it has
//! acted or said they are done, and its countdown (if any) has run out or every player holding
//! it has released it. `crate::pause_rules` decides what each pause waits for. The host's
//! Continue still ends a pause at once.
//!
//! Pauses live in memory only: after a restart, an undo or a load, the current pause waits for
//! the host or for the next event.

use treachery_protocol::UserId;
use std::collections::BTreeSet;
use std::time::Duration;

use treachery_engine::enumerations::Phase;
use treachery_engine::prelude::{PlayerId, Seat};
use treachery_engine::events::EndPhase;
use treachery_engine::events::{By, Event, HistoryEntry};

use super::{Answer, CallError, Server};
use crate::protocol::{ErrorCode, Notification};
use crate::timers::Timer;
use treachery_protocol::{PauseWait, PhaseControlInfo};
use crate::pause_rules;

#[derive(Clone, Debug, Default)]
pub struct Pause {
    /// Numbers the pauses of a game, so a countdown ends only its own pause.
    pub id: u32,
    /// `None` before the first pause.
    pub phase: Option<Phase>,
    pub turn: i32,
    /// After an undo or a load, until the next event: the host continues.
    pub suspended: bool,
    /// Seats of the players who said they are done.
    pub done: BTreeSet<Seat>,
    /// Seats of the players holding the countdown.
    pub holding: BTreeSet<Seat>,
    pub was_held: bool,
    pub countdown_ends: Option<crate::time::Time>,
    pub countdown_expired: bool,
    /// The bots had a turn in this pause and did nothing.
    pub bots_passed: bool,
    /// What the players were last told.
    pub last_sent: Option<PhaseControlInfo>,
}

/// A change to the current pause, made before it is evaluated.
pub type PauseChange = Box<dyn FnOnce(&mut Pause)>;

impl Server {
    pub(super) async fn phase_done(&mut self, user: UserId, game: &str) -> Answer<()> {
        self.change_pause(user, game, |pause, seat| {
            pause.done.insert(seat);
            pause.holding.remove(&seat);
        })
        .await
    }

    /// Holds the running countdown; does nothing when none is running.
    pub(super) async fn phase_hold(&mut self, user: UserId, game: &str) -> Answer<()> {
        self.change_pause(user, game, |pause, seat| {
            if pause.countdown_ends.is_some() && !pause.countdown_expired {
                pause.holding.insert(seat);
                pause.was_held = true;
            }
        })
        .await
    }

    pub(super) async fn phase_release(&mut self, user: UserId, game: &str) -> Answer<()> {
        self.change_pause(user, game, |pause, seat| {
            pause.holding.remove(&seat);
        })
        .await
    }

    /// The host's Continue: ends the pause the host saw, at `event_number` events. If the game
    /// has moved on since, it does nothing.
    pub(super) async fn phase_continue(&mut self, user: UserId, game: &str, event_number: i32) -> Answer<()> {
        let game = self.host_of(user, game)?;
        let event = {
            let running = &self.games[&game];
            let g = &running.game;
            let faction = running.participation.player_of(g, user).map_or(None, |p| g.player(p).faction);
            HistoryEntry::new(faction, EndPhase::default())
        };
        let Ok(expected) = usize::try_from(event_number) else { return Ok(()) };
        self.execute(&game, event, true, Some(expected), false).await?;
        Ok(())
    }

    async fn change_pause(&mut self, user: UserId, game: &str, change: impl FnOnce(&mut Pause, Seat) + 'static) -> Answer<()> {
        let game = self.player_of(user, game)?;
        let running = &self.games[&game];
        let g = &running.game;
        let Some(seat) = running.participation.player_of(g, user).map(|p| g.player(p).seated()) else {
            return Err(CallError::Refused(ErrorCode::InvalidGameEvent, "Only players with a faction can do this".into()));
        };
        self.update_pause(&game, None, Some(Box::new(move |pause: &mut Pause| change(pause, seat)))).await?;
        Ok(())
    }

    /// After an undo or a load: the current pause waits for the host.
    pub(super) fn suspend_pause(&mut self, game: &str) {
        let Some(g) = self.games.get_mut(game) else { return };
        g.pause = new_pause(&mut g.pause_count, &g.game, Some(&g.pause), true);
        let info = send(&mut g.pause, PhaseControlInfo::default());
        if let Some(info) = info {
            self.connections.notify(game, Notification::PhaseControl(info));
        }
    }

    pub(super) async fn countdown_ended(&mut self, game: &str, pause: u32) -> Result<(), CallError> {
        let change: PauseChange = Box::new(move |p: &mut Pause| {
            if p.id == pause {
                p.countdown_expired = true;
            }
        });
        self.update_pause(game, None, Some(change)).await
    }

    /// Brings the game's pause up to date after `executed` (or a change), tells the players if
    /// what it waits for changed, and continues it when it waits for nothing.
    pub(super) async fn update_pause(&mut self, game: &str, executed: Option<(&Event, By)>, change: Option<PauseChange>) -> Result<(), CallError> {
        let now = self.now();
        let Some(g) = self.games.get_mut(game) else { return Ok(()) };
        let mut counter = g.pause_count;
        let bots_speed = g.participation.bots_speed;
        let state = &mut g.game;
        let mut pause = std::mem::take(&mut g.pause);
        if pause.id == 0 {
            pause = new_pause(&mut counter, state, None, true);
        }
        if let Some((executed, by)) = executed {
            if matches!(executed, Event::EndPhase(_)) || pause.phase != Some(state.turn.current_phase) || pause.turn != i32::from(state.turn.current_turn) {
                pause = new_pause(&mut counter, state, Some(&pause), false);
            } else {
                pause.suspended = false;
                pause.bots_passed = false;
                if let Some(p) = by.player {
                    let seat = state.player(p).seated();
                    pause.holding.remove(&seat);
                    if pause_rules::finishes_pause(state, executed) {
                        pause.done.insert(seat);
                    }
                }
            }
        }
        if let Some(change) = change {
            change(&mut pause);
        }

        let requirements = if pause.suspended { None } else { pause_rules::determine(state) };
        let mut continue_at = None;
        let mut countdown_started = None;
        let info = match requirements {
            None => send(&mut pause, PhaseControlInfo::default()),
            Some(requirements) => {
                let any_bot = state.player_indices().any(|p| state.is_bot(p));
                let bots_done = pause.bots_passed || bots_speed <= 0 || !any_bot;
                let waited: Vec<PlayerId> = requirements
                    .players()
                    .into_iter()
                    .filter(|&p| !pause.done.contains(&state.player(p).seated()) && !(state.is_bot(p) && bots_done))
                    .collect();
                let waiting_for: Vec<Seat> = waited.iter().map(|&p| state.player(p).seated()).collect();
                let reasons: Vec<PauseWait> =
                    waited.iter().map(|&p| PauseWait { seat: state.player(p).seated(), reasons: requirements.reasons_of(p) }).collect();
                if requirements.countdown && pause.countdown_ends.is_none() {
                    pause.countdown_ends = Some(now + chrono::Duration::milliseconds(i64::from(pause_rules::COUNTDOWN_MILLISECONDS)));
                    countdown_started = Some(pause.id);
                }
                let countdown_done = !requirements.countdown || (pause.holding.is_empty() && (pause.countdown_expired || pause.was_held));
                if waiting_for.is_empty() && countdown_done && bots_done {
                    continue_at = Some(state.history.entries.len());
                    None
                } else {
                    let running = requirements.countdown && !pause.countdown_expired && !pause.was_held;
                    let info = PhaseControlInfo {
                        active: true,
                        waiting_for,
                        countdown: requirements.countdown,
                        countdown_ends: if running { pause.countdown_ends } else { None },
                        holding: pause.holding.iter().copied().collect(),
                        reasons,
                    };
                    send(&mut pause, info)
                }
            }
        };
        g.pause = pause;
        g.pause_count = counter;

        if let Some(info) = info {
            self.connections.notify(game, Notification::PhaseControl(info));
        }
        if let Some(pause) = countdown_started {
            self.services.timers.schedule(
                Duration::from_millis(pause_rules::COUNTDOWN_MILLISECONDS as u64),
                Timer::PauseCountdown { game: game.to_string(), pause },
            );
        }
        if let Some(events) = continue_at {
            let event = HistoryEntry::new(None, EndPhase::default());
            match Box::pin(self.execute(game, event, true, Some(events), false)).await {
                Err(CallError::Refused(_, reason)) => println!("game {game}: the pause could not be ended: {reason}"),
                r => r?,
            }
        }
        Ok(())
    }
}

fn new_pause(counter: &mut u32, g: &treachery_engine::game::Game, previous: Option<&Pause>, suspended: bool) -> Pause {
    *counter += 1;
    Pause {
        id: *counter,
        phase: Some(g.turn.current_phase),
        turn: g.turn.current_turn.into(),
        suspended,
        last_sent: previous.and_then(|p| p.last_sent.clone()),
        ..Pause::default()
    }
}

/// The info to send if it differs from what the players last got. Nothing is sent before the
/// first pause that is under control.
fn send(pause: &mut Pause, info: PhaseControlInfo) -> Option<PhaseControlInfo> {
    let unchanged = match &pause.last_sent {
        None => !info.active,
        Some(last) => *last == info,
    };
    if unchanged {
        return None;
    }
    pause.last_sent = Some(info.clone());
    Some(info)
}
