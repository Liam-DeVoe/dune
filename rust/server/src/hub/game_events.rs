//! Port of `Server/GameHub_GameEvents.cs`. Package H3.
//!
//! The 140 `Request<Event>` methods are [`GameHub::submit_event`]. The bots
//! (`ScheduleBotEvent`, `PerformBotEvent`, `GetOrInitializeBot`, `DetermineBotDelay`) run the
//! `treachery-bots` port on the live game, as C# runs `ClassicBot` on it.

use std::panic::AssertUnwindSafe;
use std::rc::Rc;
use std::time::Duration;

use futures::FutureExt;
use treachery_bots::{BotParameters, ClassicBot};
use treachery_engine::deck::Deck;
use treachery_engine::enumerations::{ErrorType, MainPhase, Phase};
use treachery_engine::events::establish_players::EstablishPlayers;
use treachery_engine::events::GameEvent;
use treachery_engine::player::PlayerIndex;
use treachery_engine::game_status::GameStatus;

use super::{GameHub, HubError, HubResult, ManagedGameRef};
use crate::dotnet_string;
use crate::mail::{MailAddress, MailMessage, MailPriority};
use crate::model::result::VoidResult;
use crate::net_time::{DateTimeOffsetExt, TimeSpanExt};
use crate::render;
use crate::model::managed_game::ManagedGame;
use crate::timers::TimerAction;

/// `Task.Delay(int)` with the hub's timers: -1 waits forever, anything lower throws.
fn delay(ms: i32) -> HubResult<Option<Duration>> {
    if ms < -1 {
        return Err(HubError::Exception {
            type_name: "ArgumentOutOfRangeException",
            message: format!("The value must be greater than or equal to -1. (Parameter 'millisecondsDelay')\nActual value was {ms}."),
        });
    }
    Ok((ms >= 0).then(|| Duration::from_millis(ms as u64)))
}

/// `new NullReferenceException()` thrown by server code.
fn null_reference() -> HubError {
    HubError::Exception { type_name: "NullReferenceException", message: "Object reference not set to an instance of an object.".to_string() }
}

impl GameHub {
    /// Every `Request<Event>(string userToken, string gameId, Event e)` (GameHub_GameEvents.cs:35-173):
    /// `RequestEstablishPlayers` for an `EstablishPlayers`, `ProcessGameEvent` for every other
    /// event. The C# hub has a method for every event type, `AutomationConfigured` included.
    pub async fn submit_event(&mut self, user_token: &str, game_id: &str, e: GameEvent) -> HubResult<VoidResult> {
        match e {
            GameEvent::EstablishPlayers(e) => self.request_establish_players(user_token, game_id, e).await,
            e => self.process_game_event(user_token, game_id, e).await,
        }
    }

    /// C#: `public async Task<VoidResult> RequestEstablishPlayers(string userToken, string gameId, EstablishPlayers e)` (GameHub_GameEvents.cs:8)
    pub async fn request_establish_players(&mut self, user_token: &str, game_id: &str, e: Rc<EstablishPlayers>) -> HubResult<VoidResult> {
        let game = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok((_, game)) => game,
            Err(error) => return Ok(error),
        };

        // C# parity: the result is discarded. An invalid EstablishPlayers still reassigns the
        // seats and returns Success.
        self.process_game_event(user_token, game_id, GameEvent::EstablishPlayers(e)).await?;

        let seated_players = {
            let managed = game.borrow();
            let mut participation = managed.game.participation.borrow_mut();

            let user_ids: Vec<i32> = participation.seated_players.keys().copied().collect();
            let mut participant_index = 0;
            for player in &managed.game.players {
                if participant_index >= user_ids.len() {
                    break;
                }

                let user_id = user_ids[participant_index];
                participant_index += 1;
                participation.seated_players.insert(user_id, player.seat);
            }

            participation.seated_players.clone()
        };

        self.clients.group(game_id, crate::clients::Callback::HandleAssignSeats(seated_players));

        game.borrow_mut().last_activity = self.now();
        self.persist_game_if_needed(&game).await?;
        Ok(Self::success())
    }

    /// C#: `public async Task<VoidResult> SetTimer(string userToken, string gameId, int value)` (GameHub_GameEvents.cs:175)
    pub async fn set_timer(&mut self, user_token: &str, game_id: &str, value: i32) -> HubResult<VoidResult> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(v) => v,
            Err(error) => return Ok(error),
        };

        let user_id = user.borrow().id;
        if !game.borrow().game.is_host(Some(user_id)) {
            return Ok(Self::error(ErrorType::NoHost, ""));
        }

        self.clients.group(game_id, crate::clients::Callback::HandleSetTimer(value));
        Ok(Self::success())
    }

    /// C#: `private async Task<VoidResult> ProcessGameEvent<TEvent>(string userToken, string gameId, TEvent e)` (GameHub_GameEvents.cs:187)
    pub async fn process_game_event(&mut self, user_token: &str, game_id: &str, e: GameEvent) -> HubResult<VoidResult> {
        let (user, game) = match self.are_valid(Some(user_token), Some(game_id)) {
            Ok(v) => v,
            Err(error) => return Ok(error),
        };

        let user_id = user.borrow().id;
        let is_host = game.borrow().game.is_host(Some(user_id));
        self.validate_and_execute(e, &game, is_host, None).await
    }

    /// C#: `private async Task<VoidResult> ValidateAndExecute<TEvent>(TEvent e, ManagedGame game, bool isHost)` (GameHub_GameEvents.cs:195)
    ///
    /// `game.ProcessEventAsync` only serializes events per game, which calls running one at a
    /// time already do (model/managed_game.rs).
    ///
    /// With `expected_event_count`, the event is only executed while the game has that many
    /// events. Used to continue a pause exactly once.
    pub async fn validate_and_execute(&mut self, e: GameEvent, game: &ManagedGameRef, is_host: bool, expected_event_count: Option<i32>) -> HubResult<VoidResult> {
        if let Some(expected) = expected_event_count {
            if game.borrow().game.history.len() as i32 != expected {
                return Ok(Self::success());
            }
        }

        let validation_result = {
            let mut managed = game.borrow_mut();
            let g = &mut managed.game;
            e.initialize_game(g);
            e.execute(g, true, is_host)
        };
        // After Execute: a valid event is in History already and gets its Time here.
        e.base().time.set(self.now());

        if let Some(validation_result) = validation_result {
            let text = render::message_to_string_with_default_describer(&validation_result);
            GameHub::log(&format!("Invalid bot decision: {text}"));
            return Ok(Self::error(ErrorType::InvalidGameEvent, &text));
        }

        let send_end_of_game_mail = {
            let managed = game.borrow();
            let g = &managed.game;
            g.current_main_phase == MainPhase::Ended
                && !managed.statistics_sent
                && (g.number_of_bots() as f32) < 0.5f32 * g.number_of_seated_players() as f32
        };
        if send_end_of_game_mail {
            self.send_end_of_game_mail(game).await?;
            game.borrow_mut().statistics_sent = true;
        }

        game.borrow_mut().last_activity = self.now();

        let (game_id, new_event_number) = {
            let managed = game.borrow();
            (managed.game_id.clone(), managed.game.history.len() as i32)
        };
        self.clients.group(&game_id, crate::clients::Callback::HandleGameEvent { evt: e.clone(), new_event_number });
        self.send_async_play_messages_if_applicable(&game_id).await?;
        self.persist_game_if_needed(game).await?;
        self.schedule_bot_event(game, false).await?;

        self.update_pause(game, Some(&e), None, false).await?;
        Ok(Self::success())
    }

    /// C#: `private async Task SendAsyncPlayMessagesIfApplicable(string gameId)` (GameHub_GameEvents.cs:226)
    ///
    /// Calls `GameStatus.DetermineStatus` on the live game, which changes game state; the call is
    /// exactly where C# has it. `new MailAddress(mail)` throws for an invalid address after the
    /// event was executed and broadcast; the error escapes, so the caller skips
    /// `PersistGameIfNeeded` and `ScheduleBotEvent`, and `LastAsyncPlayMessageSent` stays.
    pub async fn send_async_play_messages_if_applicable(&mut self, game_id: &str) -> HubResult<()> {
        let Some(game) = self.running_games_by_game_id.get(game_id).cloned() else {
            return Ok(());
        };

        let (async_play, interval) = {
            let managed = game.borrow();
            let Some(settings) = &managed.game.settings else { return Err(null_reference()) };
            let settings = settings.borrow();
            (settings.async_play, settings.async_play_message_interval_minutes)
        };

        if !async_play {
            return Ok(());
        }

        let now = self.now();
        let last_sent = game.borrow().last_async_play_message_sent;
        // `(int)double` saturates in .NET 9 and later, like `as`.
        let elapsed_minutes = now.subtract(last_sent).total_minutes() as i32;
        if elapsed_minutes < interval {
            let delay_in_seconds = 1i32.wrapping_add(60i32.wrapping_mul(interval.wrapping_sub(elapsed_minutes)));
            let delay_ms = delay_in_seconds.wrapping_mul(1000);
            // `Task.Delay(int)`: -1 waits forever, anything lower throws.
            if delay_ms < -1 {
                return Err(HubError::Exception {
                    type_name: "ArgumentOutOfRangeException",
                    message: format!("The value must be greater than or equal to -1. (Parameter 'millisecondsDelay')\nActual value was {delay_ms}."),
                });
            }
            if delay_ms >= 0 {
                self.services.timers.schedule(Duration::from_millis(delay_ms as u64), TimerAction::SendAsyncPlayMessages { game_id: game_id.to_string() });
            }
            return Ok(());
        }

        let (history, turn_info) = {
            let managed = game.borrow();
            let g = &managed.game;
            let what_happened: Vec<&GameEvent> =
                g.history.iter().filter(|e| e.base().time.get() > last_sent && !matches!(e, GameEvent::AllyPermission(_))).collect();

            if what_happened.is_empty() {
                return Ok(());
            }

            let mut history = "The game was just started. Have fun!".to_string();
            if what_happened.len() > 1 {
                history = "The following happened:".to_string();
                history += "<ul>";
                for evt in what_happened.iter().filter(|e| !matches!(e, GameEvent::EstablishPlayers(_))) {
                    history += &format!("<li>{}</li>", render::message_to_string(&evt.get_short_message(g)));
                }
                history += "</ul>";
            }

            let turn_info =
                if g.current_turn == 0 { "Setting up a new game".to_string() } else { render::format_turn_and_phase(g.current_turn, g.current_phase) };

            (history, turn_info)
        };

        // `Environment.NewLine` is "\n" on Linux, where the C# server runs.
        let nl = "\n\n";
        let user_ids: Vec<i32> = game.borrow().game.participation.borrow().seated_players.keys().copied().collect();
        for user_id in user_ids {
            let user = self.services.db.users_find(user_id).await?;
            let Some(mail) = user.map(|u| u.email) else { continue };
            if dotnet_string::is_white_space(&mail) {
                continue;
            }

            let (status_info, waiting_for) = {
                let mut managed = game.borrow_mut();
                let g = &mut managed.game;
                let Some(player) = g.get_player_by_user_id(Some(user_id)) else { continue };

                let status = GameStatus::determine_status(g, Some(player), true);

                let status_info = render::message_to_string(&status.get_message(player, g.is_host(Some(user_id))));

                let waiting_for = if status.waiting_for_host {
                    format!("Players are waiting for the host to continue the game {nl}")
                } else if !status.waiting_for_players.is_empty() {
                    let mut names = Vec::new();
                    for p in &status.waiting_for_players {
                        let Some(p) = *p else { return Err(null_reference()) };
                        names.push(g.player(p).name(g));
                    }
                    format!("Waiting for: {}{nl}", names.join(", "))
                } else {
                    String::new()
                };

                (status_info, waiting_for)
            };

            let user_token = self.users_by_user_token.iter().find(|(_, u)| u.id() == user_id).map(|(token, _)| token.clone());

            let link = match user_token {
                None => "Join game at: <a href=\"https://treachery.online/\">https://treachery.online/</a>".to_string(),
                Some(token) => {
                    format!("Jump to game: <a href=\"https://treachery.online/{token}/{}\">https://treachery.online/</a>", game.borrow().game_id)
                }
            };

            let async_message = format!("<p>{turn_info}</p><p>{status_info}</p><p>{waiting_for}</p><p>{link}</p><p>{history}</p>");

            let mail_message = MailMessage {
                from: MailAddress::parse("noreply@treachery.online").expect("a valid address"),
                to: vec![MailAddress::parse(&mail)?],
                subject: format!("Update for {}", game.borrow().name.as_deref().unwrap_or("")),
                body: async_message,
                is_body_html: true,
                priority: MailPriority::Normal,
                attachments: Vec::new(),
            };

            self.send_mail(mail_message);
        }

        game.borrow_mut().last_async_play_message_sent = now;
        Ok(())
    }

    /// C#: `private void ScheduleBotEvent(ManagedGame managedGame, bool immediate = false)` (GameHub_GameEvents.cs:325)
    ///
    /// `immediate` is `_ = PerformBotEvent(managedGame)`: it runs now, and whatever it throws
    /// stays in the task nobody awaits. A delayed call is a [`TimerAction::PerformBotEvent`]
    /// that holds on to this `ManagedGame`. `Task.Delay` throws for a delay below -1 (a negative
    /// bot speed), which escapes to the caller.
    pub async fn schedule_bot_event(&mut self, managed_game: &ManagedGameRef, immediate: bool) -> HubResult<()> {
        if immediate {
            let outcome = AssertUnwindSafe(self.perform_bot_event(managed_game)).catch_unwind().await;
            match outcome {
                Ok(Ok(())) => {}
                Ok(Err(e)) => GameHub::log(&format!("PerformBotEvent: {e}")),
                Err(payload) => {
                    // A C# exception is swallowed with the task; a Rust bug is not hidden.
                    if dotnet_compat::exception::as_cs_exception(&*payload).is_none() {
                        std::panic::resume_unwind(payload);
                    }
                }
            }
        } else {
            let ms = {
                let managed = managed_game.borrow();
                let g = &managed.game;
                let speed = g.participation.borrow().bots_speed;
                Self::determine_bot_delay(g.current_main_phase, g.latest_event_0().as_ref(), speed)
            };
            if let Some(d) = delay(ms)? {
                self.next_bot_timer += 1;
                let id = self.next_bot_timer;
                self.bot_timer_games.insert(id, managed_game.clone());
                let game_id = managed_game.borrow().game_id.clone();
                self.services.timers.schedule(d, TimerAction::PerformBotEvent { game_id, id });
            }
        }
        Ok(())
    }

    /// C#: `private async Task PerformBotEvent(ManagedGame managedGame)` (GameHub_GameEvents.cs:341)
    ///
    /// The bots decide on the live game, as in C#: `GetApplicableEvents` and some decisions
    /// change it (bots-plan.md, "Bots are not pure"). The first event any bot decides on, by
    /// level and then in the shuffled bot order, is executed as the bot built it.
    pub async fn perform_bot_event(&mut self, managed_game: &ManagedGameRef) -> HubResult<()> {
        let (bots_act_as_hosts, run) = {
            let managed = managed_game.borrow();
            let g = &managed.game;
            let participation = g.participation.borrow();
            let bots_act_as_hosts = participation.hosts.is_empty() && g.number_of_observers() > 0;
            (bots_act_as_hosts, participation.bots_speed > 0 && g.current_phase > Phase::AwaitingPlayers)
        };
        let game_id = managed_game.borrow().game_id.clone();
        // Rust only: dev scenes have no bots (`GameHub::scene_games`).
        if !run || self.scene_games.contains(&game_id) {
            return Ok(());
        }

        // `Deck<Player>.Randomize(game.Players.Where(p => p.IsBot))`, with the hook's random.
        let random = self.services.ids.new_random(&format!("BotOrder {game_id}"));
        let (bots, events_per_bot) = {
            let mut managed = managed_game.borrow_mut();
            let g = &mut managed.game;
            let mut bots: Vec<PlayerIndex> = g.player_indices().filter(|&p| g.is_bot(p)).collect();
            Deck::shuffle_list(&mut bots, &random);
            // `bots.ToDictionary(x => x.Seat, ...)`: seats are distinct, so this is one list per bot.
            let events: Vec<_> = bots.iter().map(|&p| g.get_applicable_events(Some(p), bots_act_as_hosts)).collect();
            (bots, events)
        };

        // Highest, High, Middle, Low, and End only when the bots act as hosts.
        let levels = if bots_act_as_hosts { 5 } else { 4 };
        for level in 0..levels {
            for (i, &bot) in bots.iter().enumerate() {
                let evt = {
                    let mut managed = managed_game.borrow_mut();
                    let managed = &mut *managed;
                    let index = Self::get_or_initialize_bot(managed, bot, self.services.ids.as_ref());
                    let classic_bot = &mut managed.bots[index].1;
                    let g = &mut managed.game;
                    let events = &events_per_bot[i];
                    match level {
                        0 => classic_bot.determine_highest_priority_in_phase_action(g, events),
                        1 => classic_bot.determine_high_priority_in_phase_action(g, events),
                        2 => classic_bot.determine_middle_priority_in_phase_action(g, events),
                        3 => classic_bot.determine_low_priority_in_phase_action(g, events),
                        _ => classic_bot.determine_end_phase_action(g, events),
                    }
                };
                let Some(evt) = evt else { continue };
                // C# parity: the result is discarded; an invalid decision just ends this turn.
                Box::pin(self.validate_and_execute(evt, managed_game, level == 4, None)).await?;
                return Ok(());
            }
        }

        if !bots.is_empty() {
            self.update_pause(managed_game, None, Some(Box::new(|pause| pause.bots_passed = true)), false).await?;
        }
        Ok(())
    }

    /// C#: `private static IBot GetOrInitializeBot(ManagedGame game, Player player)` (GameHub_GameEvents.cs:402)
    ///
    /// The bot of the player's current faction, rebound to `player`, or a new one. Returns its
    /// index in `ManagedGame::bots`.
    fn get_or_initialize_bot(game: &mut ManagedGame, player: PlayerIndex, ids: &dyn super::IdSource) -> usize {
        let faction = game.game.player(player).faction;
        if let Some(index) = game.bots.iter().position(|(f, _)| *f == faction) {
            game.bots[index].1.set_game_and_player(&game.game, player);
            return index;
        }

        let random = ids.new_random(&format!("Bot {} {faction}", game.game_id));
        let bot = ClassicBot::new(&game.game, player, BotParameters::get_default_parameters(faction), Some(random));
        game.bots.push((faction, bot));
        game.bots.len() - 1
    }

    /// C#: `private static int DetermineBotDelay(MainPhase phase, GameEvent? e, int speed)` (GameHub_GameEvents.cs:423)
    pub fn determine_bot_delay(phase: MainPhase, e: Option<&GameEvent>, speed: i32) -> i32 {
        if speed == 0 {
            return 0;
        }

        if matches!(e, Some(GameEvent::AllyPermission(_) | GameEvent::DealOffered(_) | GameEvent::SetShipmentPermission(_))) {
            return 800 / speed;
        }

        if matches!(e, Some(GameEvent::Bid(_))) {
            return 1600 / speed;
        }

        if phase == MainPhase::Resurrection || phase == MainPhase::Charity {
            return 2400 / speed;
        }

        if phase == MainPhase::ShipmentAndMove || phase == MainPhase::Battle {
            return 9600 / speed;
        }

        4800 / speed
    }

    /// Runs a [`TimerAction`] that came due. Exceptions are swallowed, as in a C#
    /// `Task.ContinueWith` that nobody awaits.
    pub async fn on_timer(&mut self, action: TimerAction) {
        match action {
            TimerAction::SendAsyncPlayMessages { game_id } => {
                if let Err(e) = self.send_async_play_messages_if_applicable(&game_id).await {
                    GameHub::log(&format!("Timer: {e}"));
                }
            }
            TimerAction::PerformBotEvent { id, .. } => {
                let Some(game) = self.bot_timer_games.remove(&id) else { return };
                if let Err(e) = self.perform_bot_event(&game).await {
                    GameHub::log(&format!("Timer: {e}"));
                }
            }
            TimerAction::PhaseCountdown { game_id, pause_id } => {
                if let Err(e) = self.expire_pause_countdown(&game_id, pause_id).await {
                    GameHub::log(&format!("Timer: {e}"));
                }
            }
        }
    }
}
