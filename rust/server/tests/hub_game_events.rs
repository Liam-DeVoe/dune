//! Unit tests for package H3: `hub/game_events.rs` (GameHub_GameEvents.cs) and `hub/chat.rs`
//! (GameHub_Chat.cs). Each test states the C# behaviour it pins down. The equivalence harness
//! (server-equiv) compares the same paths against the C# hub; these run without .NET.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use treachery_engine::cs::Shared;
use treachery_engine::date_time_offset::{DateTimeOffset, TICKS_PER_MINUTE};
use treachery_engine::enumerations::{ErrorType, Phase};
use treachery_engine::events::GameEvent;
use treachery_engine::game::Game;
use treachery_engine::participation::Participation;
use treachery_server::clients::{Callback, ConnectionSink};
use treachery_server::clock::{Clock, ManualClock};
use treachery_server::config::Config;
use treachery_server::db::user::User;
use treachery_server::db::{date_time, Db};
use treachery_server::hub::{GameHub, HubError, ManagedGameRef, SequentialIds, Services};
use treachery_server::logged_in_user::LoggedInUser;
use treachery_server::mail::RecordingMailer;
use treachery_server::model::chat_message::{GameChatMessage, GlobalChatMessage};
use treachery_server::model::managed_game::ManagedGame;
use treachery_server::timers::{RecordingTimers, TimerAction};

const GAME: &str = "game-1";

type Received = Rc<RefCell<BTreeMap<u64, Vec<String>>>>;

struct Sink {
    connection: u64,
    received: Received,
}

/// A short name for each callback, enough to check who got what.
fn describe(callback: &Callback) -> String {
    match callback {
        Callback::HandleGameEvent { evt, new_event_number } => format!("GameEvent {:?} {new_event_number}", evt.event_type()),
        Callback::HandleChatMessage(m) => format!("Chat {}", m.body.clone().unwrap_or_default()),
        Callback::HandleGlobalChatMessage(m) => format!("GlobalChat {}", m.body.clone().unwrap_or_default()),
        Callback::HandleSetTimer(v) => format!("SetTimer {v}"),
        Callback::HandlePhaseControl(info) => format!("PhaseControl waiting for {:?}", info.waiting_for),
        Callback::HandleAssignSeats(seats) => {
            format!("AssignSeats {}", seats.iter().map(|(u, s)| format!("{u}:{s}")).collect::<Vec<_>>().join(","))
        }
        _ => "other".to_string(),
    }
}

impl ConnectionSink for Sink {
    fn send(&self, _game_id: Option<&str>, _user_id: Option<i32>, callback: &Callback) {
        self.received.borrow_mut().entry(self.connection).or_default().push(describe(callback));
    }
}

struct Fixture {
    hub: GameHub,
    clock: Rc<ManualClock>,
    mailer: Rc<RecordingMailer>,
    timers: Rc<RecordingTimers>,
    received: Received,
}

fn start() -> DateTimeOffset {
    date_time::parse("2026-11-08 12:00:00+02:00").unwrap()
}

/// An `EstablishPlayers` for two players in the savegame format, with the given rules,
/// number of players and async-play settings.
fn establish_players(rules: &str, players: i32, async_play: bool, interval: i32) -> GameEvent {
    let json = format!(
        r#"{{"$type":"EstablishPlayers","Settings":{{"NumberOfPlayers":{players},"MaximumTurns":3,"InitialRules":[{rules}],"AllowedFactionsInPlay":[20,10],"AutoOpenEmptySeats":true,"AsyncPlay":{async_play},"AsyncPlayMessageIntervalMinutes":{interval}}},"Seed":12345,"MaximumNumberOfPlayers":{players},"MaximumTurns":3,"ApplicableRules":[],"_players":"","_factionsInPlay":"","Initiator":0,"Time":"0001-01-01T00:00:00+00:00"}}"#
    );
    treachery_engine::json::deserialize(&json).unwrap()
}

fn event(json: &str) -> GameEvent {
    treachery_engine::json::deserialize(json).unwrap()
}

impl Fixture {
    async fn new() -> Fixture {
        let clock = Rc::new(ManualClock::new(start()));
        let mailer = Rc::new(RecordingMailer::default());
        let timers = Rc::new(RecordingTimers::default());
        let services = Services {
            db: Db::open_in_memory().await.unwrap(),
            // SendMail sends only with a user name configured.
            config: Config::from_pairs([("GameEndEmailUsername", "mailer")]),
            clock: clock.clone(),
            timers: timers.clone(),
            mailer: mailer.clone(),
            ids: Rc::new(SequentialIds::default()),
        };
        Fixture { hub: GameHub::new(services), clock, mailer, timers, received: Rc::default() }
    }

    fn advance_minutes(&self, minutes: i64) {
        let now = self.clock.now();
        self.clock.set(DateTimeOffset::new(now.ticks + minutes * TICKS_PER_MINUTE, now.offset_minutes));
    }

    fn connect(&mut self, connection: u64) {
        self.hub.clients.connect(connection, Box::new(Sink { connection, received: self.received.clone() }));
    }

    /// A user in the database and logged in with token `T<id>`.
    async fn user(&mut self, name: &str, email: &str) -> i32 {
        let user = User { name: name.to_string(), player_name: format!("{name} player"), email: email.to_string(), ..User::default() };
        let id = self.hub.services.db.users_insert(&user).await.unwrap();
        let user = Rc::new(RefCell::new(User { id, ..user }));
        self.hub.users_by_id.insert(id, user.clone());
        let now = self.hub.now();
        self.hub.users_by_user_token.insert(format!("T{id}"), LoggedInUser::new(user, None, now));
        id
    }

    /// A new game with `seated` users seated (the first one host), each with a connection in
    /// the group (connection = user id), as `RequestCreateGame` and `RequestJoinGame` leave it.
    fn game(&mut self, seated: &[i32]) -> ManagedGameRef {
        let mut game = Game::new_int32_participation(Game::LATEST_VERSION, &Shared::new(Participation::new()));
        for (seat, user_id) in seated.iter().enumerate() {
            game.add_player(*user_id, &format!("user {user_id}"), seat as i32);
            self.connect(*user_id as u64);
            self.hub.clients.add_to_group(*user_id as u64, GAME, *user_id);
        }
        game.set_or_unset_host(seated[0]);
        let managed = Rc::new(RefCell::new(ManagedGame {
            creation_date: start(),
            last_activity: start(),
            last_persisted: start(),
            creator_user_id: seated[0],
            game_id: GAME.to_string(),
            game,
            name: Some("The game".to_string()),
            hashed_password: None,
            observers_require_password: false,
            statistics_sent: false,
            last_async_play_message_sent: DateTimeOffset::default(),
            seated_players_ref: None,
            bots: Vec::new(),
        }));
        self.hub.running_games_by_game_id.insert(GAME.to_string(), managed.clone());
        managed
    }

    fn received(&self, connection: u64) -> Vec<String> {
        self.received.borrow().get(&connection).cloned().unwrap_or_default()
    }

    fn clear_received(&self) {
        self.received.borrow_mut().clear();
    }
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()
}

#[test]
fn chat_goes_to_the_game_group_whatever_its_target() {
    rt().block_on(async {
        let mut f = Fixture::new().await;
        let a = f.user("alice", "alice@example.com").await;
        let b = f.user("bob", "bob@example.com").await;
        f.game(&[a, b]);
        f.connect(99); // not in the group

        let message = GameChatMessage { target_user_id: b, source_user_id: 12345, body: Some("hi".to_string()) };

        // AreValid: unknown token, then unknown game.
        let r = f.hub.send_chat_message("nope", GAME, message.clone()).await.unwrap();
        assert_eq!(r.error, ErrorType::UserNotFound);
        let r = f.hub.send_chat_message(&format!("T{a}"), "other", message.clone()).await.unwrap();
        assert_eq!(r.error, ErrorType::GameNotFound);
        assert!(f.received.borrow().is_empty());

        // The source user id is not checked, and every group member gets it.
        let r = f.hub.send_chat_message(&format!("T{a}"), GAME, message).await.unwrap();
        assert!(r.success);
        assert_eq!(f.received(a as u64), vec!["Chat hi"]);
        assert_eq!(f.received(b as u64), vec!["Chat hi"]);
        assert!(f.received(99).is_empty());
    });
}

#[test]
fn global_chat_goes_to_every_connection() {
    rt().block_on(async {
        let mut f = Fixture::new().await;
        let a = f.user("alice", "alice@example.com").await;
        f.connect(1);
        f.connect(2);

        let message = GlobalChatMessage { source_user_id: 7, body: Some("hello".to_string()) };
        let r = f.hub.send_global_chat_message("nope", message.clone()).await.unwrap();
        assert_eq!(r.error, ErrorType::UserNotFound);
        assert!(f.received.borrow().is_empty());

        let r = f.hub.send_global_chat_message(&format!("T{a}"), message).await.unwrap();
        assert!(r.success);
        assert_eq!(f.received(1), vec!["GlobalChat hello"]);
        assert_eq!(f.received(2), vec!["GlobalChat hello"]);
    });
}

#[test]
fn set_timer_is_host_only() {
    rt().block_on(async {
        let mut f = Fixture::new().await;
        let a = f.user("alice", "alice@example.com").await;
        let b = f.user("bob", "bob@example.com").await;
        f.game(&[a, b]);

        let r = f.hub.set_timer(&format!("T{b}"), GAME, 30).await.unwrap();
        assert_eq!(r.error, ErrorType::NoHost);
        assert!(f.received.borrow().is_empty());

        let r = f.hub.set_timer(&format!("T{a}"), GAME, 30).await.unwrap();
        assert!(r.success);
        assert_eq!(f.received(b as u64), vec!["SetTimer 30"]);
    });
}

#[test]
fn establish_players_executes_reassigns_seats_and_sets_the_time_after_execute() {
    rt().block_on(async {
        let mut f = Fixture::new().await;
        let a = f.user("alice", "alice@example.com").await;
        let b = f.user("bob", "bob@example.com").await;
        let game = f.game(&[a, b]);
        f.advance_minutes(3);
        let now = f.hub.now();

        let r = f.hub.submit_event(&format!("T{a}"), GAME, establish_players("", 2, false, 0)).await.unwrap();
        assert!(r.success, "{}", r.error_details);

        let g = &game.borrow().game;
        assert_eq!(g.history.len(), 1);
        assert_eq!(g.history[0].base().time.get(), now);
        assert_ne!(g.current_phase, Phase::AwaitingPlayers);

        // HandleGameEvent; the faction trade pause, which waits for both players; then
        // HandleAssignSeats with the players' seats in seated order.
        let seats: Vec<String> = g.players.iter().map(|p| p.seat.to_string()).collect();
        let expected_seats = format!("AssignSeats {a}:{},{b}:{}", seats[0], seats[1]);
        let expected_pause = format!("PhaseControl waiting for [{}, {}]", seats[0], seats[1]);
        assert_eq!(f.received(b as u64), vec!["GameEvent EstablishPlayers 1".to_string(), expected_pause, expected_seats]);
        assert_eq!(game.borrow().last_activity, now);
    });
}

#[test]
fn establish_players_with_an_empty_seat_gets_a_bot_that_takes_turns() {
    rt().block_on(async {
        let mut f = Fixture::new().await;
        let a = f.user("alice", "alice@example.com").await;
        let game = f.game(&[a]);

        // FillWithBots (998), and two players for one user: since version 170 the empty seat
        // becomes a bot. C# accepts this, and so does the port now.
        let r = f.hub.submit_event(&format!("T{a}"), GAME, establish_players("998", 2, false, 0)).await.unwrap();
        assert!(r.success, "{}", r.error_details);
        assert_eq!(game.borrow().game.number_of_bots(), 1);

        // ValidateAndExecute schedules the bots' turn after DetermineBotDelay.
        let scheduled: Vec<_> = f.timers.scheduled.borrow_mut().drain(..).collect();
        let (delay, action) = scheduled.into_iter().find(|(_, a)| matches!(a, TimerAction::PerformBotEvent { .. })).expect("a bot turn");
        let expected = {
            let g = &game.borrow().game;
            GameHub::determine_bot_delay(g.current_main_phase, g.latest_event_0().as_ref(), 1)
        };
        assert_eq!(delay, Duration::from_millis(expected as u64));

        // The turn runs on the game it was scheduled for and creates the bot of that seat.
        f.hub.on_timer(action).await;
        assert!(f.hub.bot_timer_games.is_empty());
        assert_eq!(game.borrow().bots.len(), 1);
    });
}

#[test]
fn bot_delay_follows_the_event_and_phase() {
    use treachery_engine::enumerations::MainPhase;
    assert_eq!(GameHub::determine_bot_delay(MainPhase::Battle, None, 0), 0);
    assert_eq!(GameHub::determine_bot_delay(MainPhase::Battle, None, 2), 4800);
    assert_eq!(GameHub::determine_bot_delay(MainPhase::Charity, None, 1), 2400);
    assert_eq!(GameHub::determine_bot_delay(MainPhase::Bidding, None, 4), 1200);
    let bid = event(r#"{"$type":"Bid","Initiator":20,"Amount":1}"#);
    assert_eq!(GameHub::determine_bot_delay(MainPhase::Bidding, Some(&bid), 2), 800);
    assert_eq!(GameHub::determine_bot_delay(MainPhase::Bidding, Some(&bid), -1), -1600);
}

#[test]
fn invalid_establish_players_still_reassigns_seats_and_succeeds() {
    rt().block_on(async {
        let mut f = Fixture::new().await;
        let a = f.user("alice", "alice@example.com").await;
        let b = f.user("bob", "bob@example.com").await;
        f.game(&[a, b]);
        let r = f.hub.submit_event(&format!("T{a}"), GAME, establish_players("", 2, false, 0)).await.unwrap();
        assert!(r.success);
        f.clear_received();

        // C# parity: RequestEstablishPlayers ignores ProcessGameEvent's result.
        let r = f.hub.submit_event(&format!("T{a}"), GAME, establish_players("", 2, false, 0)).await.unwrap();
        assert!(r.success);
        let received = f.received(a as u64);
        assert_eq!(received.len(), 1);
        assert!(received[0].starts_with("AssignSeats"));
    });
}

#[test]
fn an_invalid_event_returns_the_validation_text_with_the_describer_star() {
    rt().block_on(async {
        let mut f = Fixture::new().await;
        let a = f.user("alice", "alice@example.com").await;
        let b = f.user("bob", "bob@example.com").await;
        let game = f.game(&[a, b]);

        // Not applicable while awaiting players.
        let r = f.hub.submit_event(&format!("T{b}"), GAME, event(r#"{"$type":"EndPhase","Initiator":0}"#)).await.unwrap();
        assert_eq!(r.error, ErrorType::InvalidGameEvent);
        assert_eq!(r.error_details, "Event 'Phase ended by None' is not applicable*");
        assert!(game.borrow().game.history.is_empty());
        assert!(f.received.borrow().is_empty());

        let r = f.hub.submit_event("nope", GAME, event(r#"{"$type":"EndPhase","Initiator":0}"#)).await.unwrap();
        assert_eq!(r.error, ErrorType::UserNotFound);
    });
}

#[test]
fn async_play_mails_every_seated_user_with_an_address() {
    rt().block_on(async {
        let mut f = Fixture::new().await;
        let a = f.user("alice", "alice@example.com").await;
        let b = f.user("bob", "   ").await; // white space only: skipped
        let game = f.game(&[a, b]);

        let r = f.hub.submit_event(&format!("T{a}"), GAME, establish_players("", 2, true, 0)).await.unwrap();
        assert!(r.success, "{}", r.error_details);

        let sent = f.mailer.sent.borrow();
        assert_eq!(sent.len(), 1);
        let mail = &sent[0];
        assert_eq!(mail.to[0].address(), "alice@example.com");
        assert_eq!(mail.from.address(), "noreply@treachery.online");
        assert_eq!(mail.subject, "Update for The game");
        assert!(mail.is_body_html);
        assert!(mail.body.starts_with("<p>Setting up a new game</p><p>"), "{}", mail.body);
        assert!(mail.body.contains(&format!("Jump to game: <a href=\"https://treachery.online/T{a}/{GAME}\">")), "{}", mail.body);
        assert!(mail.body.ends_with("<p>The game was just started. Have fun!</p>"), "{}", mail.body);
        assert_eq!(game.borrow().last_async_play_message_sent, f.hub.now());
    });
}

#[test]
fn async_play_waits_for_the_interval_with_a_timer() {
    rt().block_on(async {
        let mut f = Fixture::new().await;
        let a = f.user("alice", "alice@example.com").await;
        let b = f.user("bob", "bob@example.com").await;
        let game = f.game(&[a, b]);
        game.borrow_mut().last_async_play_message_sent = f.hub.now();
        f.advance_minutes(3);

        let r = f.hub.submit_event(&format!("T{a}"), GAME, establish_players("", 2, true, 10)).await.unwrap();
        assert!(r.success, "{}", r.error_details);
        assert!(f.mailer.sent.borrow().is_empty());

        // 1 + 60 * (10 - 3) seconds. Then ScheduleBotEvent's turn (the game has no bots, so it
        // does nothing when it runs).
        let scheduled: Vec<_> = f.timers.scheduled.borrow_mut().drain(..).collect();
        assert_eq!(scheduled[0], (Duration::from_secs(421), TimerAction::SendAsyncPlayMessages { game_id: GAME.to_string() }));
        assert!(matches!(scheduled[1..], [(_, TimerAction::PerformBotEvent { .. })]));

        f.advance_minutes(8);
        f.hub.on_timer(scheduled[0].1.clone()).await;
        assert_eq!(f.mailer.sent.borrow().len(), 2);
        assert!(f.timers.scheduled.borrow().is_empty());
    });
}

#[test]
fn an_invalid_address_throws_after_the_event_was_executed_and_broadcast() {
    rt().block_on(async {
        let mut f = Fixture::new().await;
        let a = f.user("alice", "not an address").await;
        let b = f.user("bob", "bob@example.com").await;
        let game = f.game(&[a, b]);
        let persisted_before = game.borrow().last_persisted;
        f.advance_minutes(10);

        let r = f.hub.submit_event(&format!("T{a}"), GAME, establish_players("", 2, true, 0)).await;
        assert!(matches!(r, Err(HubError::Exception { type_name: "FormatException", .. })), "{:?}", r.err());

        // Executed and broadcast, but not persisted, and the seats were not reassigned.
        assert_eq!(game.borrow().game.history.len(), 1);
        assert_eq!(f.received(b as u64), vec!["GameEvent EstablishPlayers 1"]);
        assert_eq!(game.borrow().last_persisted, persisted_before);
        assert_eq!(game.borrow().last_async_play_message_sent, DateTimeOffset::default());
    });
}
