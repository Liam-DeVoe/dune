//! Port of `GameHub_PhaseControl.cs`: continues the pauses between phases once every player who
//! may act there has acted or passed and any countdown has run out (crate::phase_control). The
//! host's Continue stays as an override. Pauses are not persisted: after a server restart, an
//! undo or a loaded game, the current pause waits for the host or for the next event.

use std::collections::BTreeSet;
use std::rc::Rc;
use std::time::Duration;

use treachery_engine::enumerations::{ErrorType, Faction, Phase};
use treachery_engine::events::end_phase::EndPhase;
use treachery_engine::events::GameEvent;

use super::{GameHub, HubResult, ManagedGameRef};
use crate::clients::Callback;
use crate::model::phase_control_info::PhaseControlInfo;
use crate::model::result::VoidResult;
use crate::phase_control;
use crate::timers::TimerAction;

/// C#: `private class Pause` (GameHub_PhaseControl.cs)
#[derive(Clone, Debug, Default)]
pub struct Pause {
    pub id: i32,
    pub phase: Phase,
    pub turn: i32,
    /// Set after an undo or a loaded game, until the next event.
    pub suspended: bool,
    /// Seats of the players who clicked Done.
    pub done: BTreeSet<i32>,
    pub holding: BTreeSet<i32>,
    pub was_held: bool,
    pub countdown_ends: Option<treachery_engine::date_time_offset::DateTimeOffset>,
    pub countdown_expired: bool,
    /// Whether the bots had a turn in this pause and did nothing.
    pub bots_passed: bool,
    pub last_sent: Option<PhaseControlInfo>,
}

/// A change to the current pause, made before it is evaluated.
pub type PauseChange = Box<dyn FnOnce(&mut Pause)>;

impl GameHub {
    /// C#: `public async Task<VoidResult> RequestPhaseDone(string userToken, string gameId)`
    pub async fn request_phase_done(&mut self, user_token: &str, game_id: &str) -> HubResult<VoidResult> {
        self.change_pause(user_token, game_id, |pause, seat| {
            pause.done.insert(seat);
            pause.holding.remove(&seat);
        })
        .await
    }

    /// C#: `public async Task<VoidResult> RequestPhaseHold(string userToken, string gameId)`
    pub async fn request_phase_hold(&mut self, user_token: &str, game_id: &str) -> HubResult<VoidResult> {
        self.change_pause(user_token, game_id, |pause, seat| {
            if pause.countdown_ends.is_none() || pause.countdown_expired {
                return;
            }
            pause.holding.insert(seat);
            pause.was_held = true;
        })
        .await
    }

    /// C#: `public async Task<VoidResult> RequestPhaseRelease(string userToken, string gameId)`
    pub async fn request_phase_release(&mut self, user_token: &str, game_id: &str) -> HubResult<VoidResult> {
        self.change_pause(user_token, game_id, |pause, seat| {
            pause.holding.remove(&seat);
        })
        .await
    }

    /// C#: `public async Task<VoidResult> RequestPhaseContinue(string userToken, string gameId, int eventNumber)`
    ///
    /// The host's Continue. It only ends the pause the host was looking at: if the game has moved
    /// on from `event_number` events, it does nothing.
    pub async fn request_phase_continue(&mut self, user_token: &str, game_id: &str, event_number: i32) -> HubResult<VoidResult> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(v) => v,
            Err(error) => return Ok(error),
        };

        let user_id = user.borrow().id;
        if !game.borrow().game.is_host(Some(user_id)) {
            return Ok(Self::error(ErrorType::NoHost, ""));
        }

        let e = {
            let managed = game.borrow();
            let g = &managed.game;
            let faction = g.get_player_by_user_id(Some(user_id)).map_or(Faction::None, |p| g.player(p).faction);
            GameEvent::EndPhase(Rc::new(EndPhase::new(g, faction)))
        };
        self.validate_and_execute(e, &game, true, Some(event_number)).await
    }

    async fn change_pause(&mut self, user_token: &str, game_id: &str, change: impl FnOnce(&mut Pause, i32) + 'static) -> HubResult<VoidResult> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(v) => v,
            Err(error) => return Ok(error),
        };

        let user_id = user.borrow().id;
        let seat = {
            let managed = game.borrow();
            let g = &managed.game;
            g.get_player_by_user_id(Some(user_id)).map(|p| g.player(p).seat)
        };
        let Some(seat) = seat else {
            return Ok(Self::error(ErrorType::InvalidGameEvent, "Only players can do this"));
        };

        self.update_pause(&game, None, Some(Box::new(move |pause: &mut Pause| change(pause, seat))), false).await?;
        Ok(Self::success())
    }

    /// C#: `private async Task UpdatePause(ManagedGame game, GameEvent? executed, Action<Pause>? change, bool suspend)`
    ///
    /// Brings the game's pause up to date, tells the players if what it waits for changed, and
    /// continues it when it waits for nothing.
    pub async fn update_pause(&mut self, game: &ManagedGameRef, executed: Option<&GameEvent>, change: Option<PauseChange>, suspend: bool) -> HubResult<()> {
        let (continue_at, info, schedule_countdown) = self.evaluate_pause(game, executed, change, suspend);
        let game_id = game.borrow().game_id.clone();

        if let Some(info) = info {
            self.clients.group(&game_id, Callback::HandlePhaseControl(info));
        }

        if let Some(pause_id) = schedule_countdown {
            self.services.timers.schedule(
                Duration::from_millis(phase_control::COUNTDOWN_MILLISECONDS as u64),
                TimerAction::PhaseCountdown { game_id: game_id.clone(), pause_id },
            );
        }

        if let Some(event_count) = continue_at {
            let e = GameEvent::EndPhase(Rc::new(EndPhase::new(&game.borrow().game, Faction::None)));
            Box::pin(self.validate_and_execute(e, game, true, Some(event_count))).await?;
        }

        Ok(())
    }

    /// The countdown of pause `pause_id` ran out (the C# `Expire` closure).
    pub async fn expire_pause_countdown(&mut self, game_id: &str, pause_id: i32) -> HubResult<()> {
        let Some(game) = self.running_games_by_game_id.get(game_id).cloned() else {
            return Ok(());
        };

        let change: PauseChange = Box::new(move |pause: &mut Pause| {
            if pause.id == pause_id {
                pause.countdown_expired = true;
            }
        });
        self.update_pause(&game, None, Some(change), false).await
    }

    /// C#: `private static (int?, PhaseControlInfo?, int?) EvaluatePause(...)`
    ///
    /// Returns the number of events the game has when it should be continued, the info to send if
    /// it changed, and the id of a pause whose countdown just started.
    fn evaluate_pause(
        &mut self,
        game: &ManagedGameRef,
        executed: Option<&GameEvent>,
        change: Option<PauseChange>,
        suspend: bool,
    ) -> (Option<i32>, Option<PhaseControlInfo>, Option<i32>) {
        let mut managed = game.borrow_mut();
        let game_id = managed.game_id.clone();
        let g = &mut managed.game;

        let mut pause = match self.pauses_by_game_id.get(&game_id) {
            Some(p) => p.clone(),
            None => self.new_pause(g, None, true),
        };

        if suspend {
            pause = self.new_pause(g, Some(&pause), true);
        } else if let Some(executed) = executed {
            if matches!(executed, GameEvent::EndPhase(_)) || pause.phase != g.current_phase || pause.turn != g.current_turn {
                pause = self.new_pause(g, Some(&pause), false);
            } else {
                pause.suspended = false;
                pause.bots_passed = false;
                if let Some(p) = executed.player_opt() {
                    let seat = g.player(p).seat;
                    pause.holding.remove(&seat);
                    if phase_control::finishes_pause(g, executed) {
                        pause.done.insert(seat);
                    }
                }
            }
        }

        if let Some(change) = change {
            change(&mut pause);
        }

        let requirements = if pause.suspended { None } else { phase_control::determine(g) };
        let Some(requirements) = requirements else {
            let info = send(&mut pause, PhaseControlInfo::default());
            self.pauses_by_game_id.insert(game_id, pause);
            return (None, info, None);
        };

        let any_bot = g.player_indices().any(|p| g.is_bot(p));
        let bots_done = pause.bots_passed || g.participation.borrow().bots_speed == 0 || !any_bot;
        let waiting_for: Vec<i32> = requirements
            .wait_for
            .iter()
            .filter(|&&p| !pause.done.contains(&g.player(p).seat) && !(g.is_bot(p) && bots_done))
            .map(|&p| g.player(p).seat)
            .collect();

        let mut countdown_started = None;
        if requirements.countdown && pause.countdown_ends.is_none() {
            let mut ends = self.now();
            ends.ticks += phase_control::COUNTDOWN_MILLISECONDS as i64 * 10_000;
            pause.countdown_ends = Some(ends);
            countdown_started = Some(pause.id);
        }

        let countdown_done = !requirements.countdown || (pause.holding.is_empty() && (pause.countdown_expired || pause.was_held));

        if waiting_for.is_empty() && countdown_done && bots_done {
            let count = g.history.len() as i32;
            self.pauses_by_game_id.insert(game_id, pause);
            return (Some(count), None, countdown_started);
        }

        let running = requirements.countdown && !pause.countdown_expired && !pause.was_held;
        let info = PhaseControlInfo {
            active: true,
            waiting_for,
            countdown: requirements.countdown,
            countdown_ends: if running { pause.countdown_ends } else { None },
            holding: pause.holding.iter().copied().collect(),
        };

        let info = send(&mut pause, info);
        self.pauses_by_game_id.insert(game_id, pause);
        (None, info, countdown_started)
    }

    /// C#: `private static PhaseControlInfo CurrentPhaseControl(string gameId)`
    pub fn current_phase_control(&self, game_id: &str) -> PhaseControlInfo {
        self.pauses_by_game_id.get(game_id).and_then(|p| p.last_sent.clone()).unwrap_or_default()
    }

    fn new_pause(&mut self, g: &treachery_engine::game::Game, previous: Option<&Pause>, suspended: bool) -> Pause {
        self.last_pause_id += 1;
        Pause {
            id: self.last_pause_id,
            phase: g.current_phase,
            turn: g.current_turn,
            suspended,
            last_sent: previous.and_then(|p| p.last_sent.clone()),
            ..Pause::default()
        }
    }
}

/// C#: `private static PhaseControlInfo? Send(Pause pause, PhaseControlInfo info)`: the info if it
/// differs from what the players last got. Nothing is sent before a first active pause.
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
