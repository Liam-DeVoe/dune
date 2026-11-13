//! Events: validating and executing them, and what follows an event (views, seats, mail,
//! saving, the bots' next turn, the pause between phases).

use std::time::Duration;

use treachery_engine::enumerations::{ErrorType, MainPhase};
use treachery_engine::events::GameEvent;
use treachery_engine::game_status::GameStatus;

use super::{done, CallError, Reply, Server};
use crate::connections::Notification;
use crate::mail::{is_valid_address, Mail};
use crate::time::{self, Time};
use crate::timers::Timer;
use crate::render;

impl Server {
    pub(super) async fn submit_event(&mut self, token: &str, game: &str, event: GameEvent) -> Reply {
        let (user, game) = self.user_in_game(token, game)?;
        let as_host = self.games[&game].game.is_host(Some(user));
        self.execute(&game, event, as_host, None, false).await?;
        done()
    }

    /// Validates and executes an event. With `expected_events`, only while the game has that
    /// many events: a continue that comes late does nothing.
    pub(super) async fn execute(&mut self, game: &str, event: GameEvent, as_host: bool, expected_events: Option<usize>, by_bot: bool) -> Result<(), CallError> {
        let now = self.now();
        let Some(g) = self.games.get_mut(game) else { return Ok(()) };
        if expected_events.is_some_and(|n| n != g.game.history.len()) {
            return Ok(());
        }
        event.initialize_game(&mut g.game);
        let refusal = event.execute(&mut g.game, true, as_host);
        if let Some(message) = refusal {
            let text = render::message_to_string(&message);
            if by_bot {
                println!("game {game}: a bot's event was refused: {text}");
            }
            return Err(CallError::Refused(ErrorType::InvalidGameEvent, text));
        }
        event.base().time.set(time::to_event_time(now));
        g.last_activity = now;

        // Starting the game seats the users, in the order they joined, on the players' seats.
        let seats = matches!(event, GameEvent::EstablishPlayers(_)).then(|| {
            let mut participation = g.game.participation.borrow_mut();
            let users: Vec<i32> = participation.seated_players.keys().copied().collect();
            for (user, player) in users.iter().zip(&g.game.players) {
                participation.seated_players.insert(*user, player.seat);
            }
            participation.seated_players.iter().map(|(u, s)| (*u, *s)).collect::<Vec<_>>()
        });

        let game_over = g.game.current_main_phase == MainPhase::Ended
            && !g.statistics_sent
            && (g.game.number_of_bots() as f32) < 0.5 * g.game.number_of_seated_players() as f32;
        if game_over {
            g.statistics_sent = true;
            self.send_statistics_mail(game);
        }

        self.connections.game_changed(game);
        if let Some(seats) = seats {
            self.connections.notify(game, Notification::AssignSeats(seats));
        }
        if let Err(e) = self.send_async_mail(game).await {
            println!("game {game}: async-play mail: {e:?}");
        }
        self.changed(game);
        self.schedule_bots(game);
        Box::pin(self.update_pause(game, Some(&event), None)).await
    }

    /// With async play on, mails every seated player with an address what happened since the
    /// last mail, at most once per the game's interval.
    pub(super) async fn send_async_mail(&mut self, game: &str) -> Result<(), CallError> {
        let now = self.now();
        let site = self.services.settings.site_url.clone();
        let Some(g) = self.games.get_mut(game) else { return Ok(()) };
        let Some(settings) = g.game.settings.clone() else { return Ok(()) };
        let (async_play, interval) = {
            let s = settings.borrow();
            (s.async_play, s.async_play_message_interval_minutes)
        };
        if !async_play {
            return Ok(());
        }
        if let Some(last) = g.last_async_mail {
            let next = last + chrono::Duration::minutes(i64::from(interval.max(0)));
            if now < next {
                if !g.async_mail_pending {
                    g.async_mail_pending = true;
                    let delay = (next - now).to_std().unwrap_or_default() + Duration::from_secs(1);
                    self.services.timers.schedule(delay, Timer::AsyncMail { game: game.to_string() });
                }
                return Ok(());
            }
        }

        let since = g.last_async_mail;
        let happened: Vec<&GameEvent> = g
            .game
            .history
            .iter()
            .filter(|e| since.is_none_or(|t| time::from_event_time(&e.base().time.get()) > t) && !matches!(e, GameEvent::AllyPermission(_)))
            .collect();
        if happened.is_empty() {
            return Ok(());
        }
        let history = if happened.len() == 1 {
            "The game was just started. Have fun!".to_string()
        } else {
            let items: String = happened
                .iter()
                .filter(|e| !matches!(e, GameEvent::EstablishPlayers(_)))
                .map(|e| format!("<li>{}</li>", render::message_to_string(&e.get_short_message(&g.game))))
                .collect();
            format!("The following happened:<ul>{items}</ul>")
        };
        let turn = if g.game.current_turn == 0 {
            "Setting up a new game".to_string()
        } else {
            render::format_turn_and_phase(g.game.current_turn, g.game.current_phase)
        };

        let seated: Vec<i32> = g.game.participation.borrow().seated_players.keys().copied().collect();
        let mut mails = Vec::new();
        for user in seated {
            let Some(address) = self.users.get(&user).map(|u| u.email.clone()).filter(|a| is_valid_address(a)) else { continue };
            let g = self.games.get_mut(game).expect("present");
            let Some(player) = g.game.get_player_by_user_id(Some(user)) else { continue };
            let status = GameStatus::determine_status(&mut g.game, Some(player), true);
            let status_text = render::message_to_string(&status.get_message(player, g.game.is_host(Some(user))));
            let waiting = if status.waiting_for_host {
                "Players are waiting for the host to continue the game".to_string()
            } else if !status.waiting_for_players.is_empty() {
                let names: Vec<String> = status.waiting_for_players.iter().flatten().map(|p| g.game.player(*p).name(&g.game)).collect();
                format!("Waiting for: {}", names.join(", "))
            } else {
                String::new()
            };
            // The user's newest session, so the link logs them in.
            let token = self.sessions.iter().filter(|(_, s)| s.user == user).max_by_key(|(_, s)| s.created).map(|(t, _)| t.clone());
            let link = match token {
                Some(token) => format!("Jump to game: <a href=\"{site}/{token}/{game}\">{site}/</a>"),
                None => format!("Join game at: <a href=\"{site}/\">{site}/</a>"),
            };
            mails.push(Mail {
                from: "noreply@treachery.online".into(),
                to: address,
                subject: format!("Update for {}", g.name),
                body: format!("<p>{turn}</p><p>{status_text}</p><p>{waiting}</p><p>{link}</p><p>{history}</p>"),
                attachments: vec![],
                low_priority: false,
            });
        }
        for mail in mails {
            self.services.mailer.send(mail);
        }
        if let Some(g) = self.games.get_mut(game) {
            g.last_async_mail = Some(now);
            self.changed(game);
        }
        Ok(())
    }

    /// The statistics mail at the end of a game: the seats, and the savegame attached.
    fn send_statistics_mail(&self, game: &str) {
        let Some((from, to)) = self.services.settings.stats_mail.clone() else { return };
        let g = &self.games[game];
        let info = self.game_info(g);
        let players: Vec<(String, i32, i32)> =
            g.game.players.iter().map(|p| (render::describe_faction(p.faction), p.seat, g.game.user_id_in_seat(p.seat))).collect();
        let mut body = String::from("<h1>Player table in HTML format</h1>\n<table>\n<thead>\n<tr><th>Faction</th><th>Seat</th><th>UserId</th>\n</thead>\n<tbody>\n");
        for (faction, seat, user) in &players {
            body += &format!("<tr><td>{faction}</td><td>{seat}</td><td>{user}</td></tr>\n");
        }
        body += "</tbody>\n</table>\n<h1>Player table in CSV format</h1>\n<p>\nFaction;Seat;UserId\n";
        for (faction, seat, user) in &players {
            body += &format!("{faction};{seat};{user}\n");
        }
        body += "</p>\n";
        let ruleset = treachery_engine::enumerations::Ruleset(info.ruleset);
        self.services.mailer.send(Mail {
            from,
            to,
            subject: format!("{} ({} Players, {} Bots, Turn {} - {ruleset})", info.name, info.nr_of_players, info.nr_of_bots, info.turn),
            body,
            attachments: vec![(format!("savegame{}.json", attachment_time(self.now())), super::storage::savegame(&g.game))],
            low_priority: (info.nr_of_bots as f32) >= 0.5 * info.nr_of_players as f32,
        });
    }
}

/// `yyyyMMdd.HHmm`, UTC.
fn attachment_time(t: Time) -> String {
    t.format("%Y%m%d.%H%M").to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn attachment_names() {
        let t = crate::time::parse("2026-03-02T09:05:59.9Z").unwrap();
        assert_eq!(super::attachment_time(t), "20260302.0905");
    }
}
