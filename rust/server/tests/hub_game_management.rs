//! Unit tests for package H2: `hub/game_management.rs` (GameHub_GameManagement.cs) and
//! `model/user_connections.rs`. Each test states the C# behaviour it pins down. The scenarios in
//! rust/server-equiv (lobby, seating, settings, undo, load_game, lobby_live_seats) compare the
//! same paths against the C# hub; these tests run without .NET.

use std::cell::RefCell;
use std::rc::Rc;

use dotnet_compat::NetDictionary;
use treachery_engine::date_time_offset::{DateTimeOffset, TICKS_PER_MINUTE};
use treachery_engine::enumerations::{ErrorType, Faction, GameListScope, Rule, SubscriptionType};
use treachery_engine::game_settings::GameSettings;
use treachery_server::clients::{Callback, ConnectionSink};
use treachery_server::clock::{Clock, ManualClock};
use treachery_server::config::Config;
use treachery_server::db::{date_time, Db};
use treachery_server::hub::{CallContext, GameHub, ManagedGameRef, SequentialIds, Services};
use treachery_server::mail::RecordingMailer;
use treachery_server::model::result::CallResult;
use treachery_server::model::user_connections::UserConnections;
use treachery_server::timers::RecordingTimers;

type Log = Rc<RefCell<Vec<(u64, String)>>>;

struct Sink {
    connection: u64,
    log: Log,
}

impl ConnectionSink for Sink {
    fn send(&self, _game_id: Option<&str>, _user_id: Option<i32>, callback: &Callback) {
        let name = match callback {
            Callback::HandleSetSkin(s) => format!("SetSkin {s}"),
            Callback::HandleJoinGame { user_id, seat, .. } => format!("JoinGame {user_id} {seat}"),
            Callback::HandleSetOrUnsetHost(id) => format!("SetOrUnsetHost {id}"),
            Callback::HandleRemoveUser { user_id, kick } => format!("RemoveUser {user_id} {kick}"),
            Callback::HandleObserveGame { user_id, .. } => format!("ObserveGame {user_id}"),
            Callback::HandleBotSpeed(s) => format!("BotSpeed {s}"),
            Callback::HandleAssignSeats(_) => "AssignSeats".to_string(),
            Callback::HandleUpdateSettings(_) => "UpdateSettings".to_string(),
            Callback::HandleOpenOrCloseSeats(s) => format!("OpenOrCloseSeats {s:?}"),
            _ => "other".to_string(),
        };
        self.log.borrow_mut().push((self.connection, name));
    }
}

struct Fixture {
    hub: GameHub,
    clock: Rc<ManualClock>,
    log: Log,
}

fn start() -> DateTimeOffset {
    date_time::parse("2026-03-02 10:00:00+02:00").unwrap()
}

fn cx(connection: u64) -> CallContext {
    CallContext { connection_id: Some(connection) }
}

async fn fixture() -> Fixture {
    let clock = Rc::new(ManualClock::new(start()));
    let services = Services {
        db: Db::open_in_memory().await.unwrap(),
        config: Config::from_pairs([("GameAdminUsername", "admin")]),
        clock: clock.clone(),
        timers: Rc::new(RecordingTimers::default()),
        mailer: Rc::new(RecordingMailer::default()),
        ids: Rc::new(SequentialIds::default()),
    };
    let mut f = Fixture { hub: GameHub::new(services), clock, log: Rc::default() };
    for c in 1..=5 {
        f.hub.clients.connect(c, Box::new(Sink { connection: c, log: f.log.clone() }));
    }
    f
}

impl Fixture {
    async fn user(&mut self, name: &str) -> String {
        let r = self.hub.request_create_user(name, "hash", None, &format!("{name} player")).await.unwrap();
        r.contents.unwrap().token
    }

    async fn create_game(&mut self, conn: u64, token: &str, name: Option<&str>, password: Option<&str>) -> String {
        let r = self.hub.request_create_game(&cx(conn), name, token, password, None, None).await.unwrap();
        assert!(r.success, "{:?}", r.error);
        r.contents.unwrap().game_id.unwrap()
    }

    fn game(&self, game_id: &str) -> ManagedGameRef {
        self.hub.running_games_by_game_id[game_id].clone()
    }

    fn seats(&self, game_id: &str) -> Vec<(i32, i32)> {
        self.game(game_id).borrow().game.participation.borrow().seated_players.iter().map(|(k, v)| (*k, *v)).collect()
    }

    fn take_log(&self) -> Vec<(u64, String)> {
        std::mem::take(&mut *self.log.borrow_mut())
    }

    fn advance_seconds(&self, seconds: i64) {
        let now = self.clock.now();
        self.clock.set(DateTimeOffset::new(now.ticks + seconds * TICKS_PER_MINUTE / 60, now.offset_minutes));
    }
}

fn error_of<T>(r: &CallResult<T>) -> ErrorType {
    assert!(!r.success);
    r.error
}

#[test]
fn user_connections_keep_the_last_connection_per_game() {
    let mut c = UserConnections::default();
    assert_eq!(c.try_get_connection_id("g"), None);
    c.set_connection_id("g", 3, start());
    c.set_connection_id("h", 4, start());
    c.set_connection_id("g", 5, start());
    assert_eq!(c.try_get_connection_id("g"), Some(5));
    // The indexer set keeps the key's position.
    assert_eq!(c.entries().map(|(g, _)| g.as_str()).collect::<Vec<_>>(), ["g", "h"]);
    c.remove_connection_id("g");
    c.remove_connection_id("g");
    assert_eq!(c.try_get_connection_id("g"), None);
    assert_eq!(c.try_get_connection_id("h"), Some(4));
}

#[tokio::test]
async fn create_game_names_seats_and_limits() {
    let mut f = fixture().await;
    assert_eq!(error_of(&f.hub.request_create_game(&cx(1), None, "nope", None, None, None).await.unwrap()), ErrorType::UserNotFound);

    let alice = f.user("alice").await;
    let r = f.hub.request_create_game(&cx(1), Some(""), &alice, None, None, Some("skin")).await.unwrap();
    let info = r.contents.unwrap();
    // An empty name becomes "<PlayerName>'s Game"; the creator is seated (seat -1) and host.
    assert_eq!(info.game_name.as_deref(), Some("alice player's Game"));
    let game_id = info.game_id.unwrap();
    assert_eq!(f.seats(&game_id), [(1, -1)]);
    assert!(f.game(&game_id).borrow().game.is_host(Some(1)));
    // The skin goes to the group, which the creator's connection has just joined.
    assert_eq!(f.take_log(), [(1, "SetSkin skin".to_string())]);
    // GameInitInfo carries the game's own Participation object.
    assert!(info.participation.unwrap().ptr_eq(&f.game(&game_id).borrow().game.participation));
    // Persisted right away (LastPersisted starts at MinValue).
    assert_eq!(f.hub.services.db.persisted_games_all().await.unwrap().len(), 1);

    for _ in 1..10 {
        f.create_game(1, &alice, Some("x"), None).await;
    }
    assert_eq!(error_of(&f.hub.request_create_game(&cx(1), None, &alice, None, None, None).await.unwrap()), ErrorType::TooManyGames);
}

#[tokio::test]
async fn join_game_checks_and_seat_takeover() {
    let mut f = fixture().await;
    let alice = f.user("alice").await;
    let bob = f.user("bobby").await;
    let carol = f.user("carol").await;
    let g = f.create_game(1, &alice, None, Some("pw")).await;
    f.take_log();

    assert_eq!(error_of(&f.hub.request_join_game(&cx(2), &bob, &g, Some("wrong"), -1).await.unwrap()), ErrorType::IncorrectGamePassword);
    assert_eq!(error_of(&f.hub.request_join_game(&cx(2), &bob, &g, None, -1).await.unwrap()), ErrorType::IncorrectGamePassword);
    assert!(f.hub.request_join_game(&cx(2), &bob, &g, Some("pw"), -1).await.unwrap().success);
    // Bob's connection joins the group after the callback.
    assert_eq!(f.take_log(), [(1, "JoinGame 2 -1".to_string())]);

    // A player joining again skips the password check and is added to the group again.
    assert!(f.hub.request_join_game(&cx(3), &bob, &g, Some("wrong"), 7).await.unwrap().success);
    assert_eq!(f.hub.connection_info_by_user_id[&2].try_get_connection_id(&g), Some(3));

    // Kicked users cannot come back; seat checks come first.
    f.hub.request_kick(&alice, &g, 2).await.unwrap();
    assert_eq!(error_of(&f.hub.request_join_game(&cx(2), &bob, &g, Some("pw"), -1).await.unwrap()), ErrorType::SeatNotAvailableKicked);

    // An open seat that someone holds: they are removed (not kicked) and the joiner takes it.
    f.hub.request_assign_seats(&alice, &g, NetDictionary::from_iter([(1, 0)])).await.unwrap();
    f.hub.request_open_or_close_seat(&alice, &g, 0).await.unwrap();
    f.take_log();
    assert!(f.hub.request_join_game(&cx(4), &carol, &g, Some("pw"), 0).await.unwrap().success);
    assert_eq!(f.seats(&g), [(3, 0)]);
    let log: Vec<String> = f.take_log().into_iter().filter(|(c, _)| *c == 1).map(|(_, s)| s).collect();
    // Carol is the only seated player, so she becomes host.
    assert_eq!(log, ["RemoveUser 1 false", "JoinGame 3 0", "SetOrUnsetHost 3"]);
}

#[tokio::test]
async fn leave_promotes_seated_players_and_removes_one_connection() {
    let mut f = fixture().await;
    let alice = f.user("alice").await;
    let bob = f.user("bobby").await;
    let carol = f.user("carol").await;
    let g = f.create_game(1, &alice, None, None).await;
    f.hub.request_join_game(&cx(2), &bob, &g, None, -1).await.unwrap();
    f.hub.request_observe_game(&cx(3), &carol, &g, None).await.unwrap();
    // Alice's second connection.
    f.hub.request_reconnect_game(&cx(4), &alice, &g).await.unwrap();
    f.take_log();

    let r = f.hub.request_leave_game(&alice, &g).await.unwrap();
    assert!(r.success);
    // Only seated players are promoted; the observer is not.
    let game = f.game(&g);
    assert!(game.borrow().game.is_host(Some(2)));
    assert!(!game.borrow().game.is_host(Some(3)));
    // Only connection 4 (recorded last) left the group; connection 1 stays.
    let members: Vec<u64> = f.hub.clients.group_members(&g).into_iter().map(|(c, _)| c).collect();
    assert_eq!(members, [1, 2, 3]);
    // The emptied UserConnections entry stays.
    assert!(f.hub.connection_info_by_user_id.contains_key(&1));
}

#[tokio::test]
async fn host_and_creator_checks() {
    let mut f = fixture().await;
    let alice = f.user("alice").await;
    let bob = f.user("bobby").await;
    let g = f.create_game(1, &alice, None, None).await;
    f.hub.request_join_game(&cx(2), &bob, &g, None, -1).await.unwrap();

    assert_eq!(error_of(&f.hub.request_set_or_unset_host(&alice, &g, 1).await.unwrap()), ErrorType::CannotRemoveLastHost);
    assert_eq!(error_of(&f.hub.request_set_or_unset_host(&bob, &g, 2).await.unwrap()), ErrorType::NoHost);
    assert!(f.hub.request_set_or_unset_host(&alice, &g, 2).await.unwrap().success);
    // Bob is host now, but only the creator may change settings or close the game.
    assert_eq!(error_of(&f.hub.request_update_settings(&bob, &g, GameSettings::new()).await.unwrap()), ErrorType::NoCreator);
    assert_eq!(error_of(&f.hub.request_close_game(&bob, &g).await.unwrap()), ErrorType::NoCreator);
    assert_eq!(error_of(&f.hub.request_undo(&bob, "nope", 0).await.unwrap()), ErrorType::GameNotFound);
}

#[tokio::test]
async fn update_settings_accepts_bot_rules() {
    let mut f = fixture().await;
    let alice = f.user("alice").await;
    let g = f.create_game(1, &alice, None, None).await;
    f.take_log();

    // As in C#, any rules are taken, the ones that add bots included.
    let mut settings = GameSettings::new();
    settings.initial_rules = Some(vec![Rule::AdvancedCombat, Rule::FillWithBots, Rule::BotsCannotAlly]);
    settings.number_of_players = 3;
    assert!(f.hub.request_update_settings(&alice, &g, settings).await.unwrap().success);
    assert_eq!(f.game(&g).borrow().game.settings.as_ref().unwrap().borrow().number_of_players, 3);
    assert_eq!(f.take_log(), [(1, "UpdateSettings".to_string())]);
}

#[tokio::test]
async fn close_game_removes_everyone() {
    let mut f = fixture().await;
    let alice = f.user("alice").await;
    let bob = f.user("bobby").await;
    let carol = f.user("carol").await;
    let g = f.create_game(1, &alice, None, None).await;
    f.hub.request_join_game(&cx(2), &bob, &g, None, -1).await.unwrap();
    f.hub.request_observe_game(&cx(3), &carol, &g, None).await.unwrap();
    f.take_log();

    let r = f.hub.request_close_game(&alice, &g).await.unwrap();
    assert!(r.success);
    assert!(f.hub.running_games_by_game_id.is_empty());
    assert!(f.hub.services.db.persisted_games_all().await.unwrap().is_empty());
    assert!(f.hub.clients.group_members(&g).is_empty());
    // In PlayerNames order; each user is removed from the group after their own callback.
    let to_carol: Vec<String> = f.take_log().into_iter().filter(|(c, _)| *c == 3).map(|(_, s)| s).collect();
    assert_eq!(to_carol, ["RemoveUser 1 true", "RemoveUser 2 true", "RemoveUser 3 true"]);
}

#[tokio::test]
async fn cached_lobby_entries_read_the_live_seat_dictionary() {
    let mut f = fixture().await;
    let alice = f.user("alice").await;
    let bob = f.user("bobby").await;
    let carol = f.user("carol").await;
    let g = f.create_game(1, &alice, None, None).await;
    f.hub.update_server_status_if_needed(true);

    // A later join shows in the cached entry's seats but not in its count.
    f.hub.request_join_game(&cx(2), &bob, &g, None, -1).await.unwrap();
    let cached = f.hub.running_games[0].clone();
    assert_eq!(cached.nr_of_players, 1);
    assert_eq!(cached.seated_players.get().len(), 2);
    assert!(cached.you_are_in(2));

    // RequestAssignSeats replaces the dictionary: the cached entry keeps the old contents.
    f.hub.request_assign_seats(&alice, &g, NetDictionary::from_iter([(1, 0), (2, 1)])).await.unwrap();
    f.hub.request_join_game(&cx(3), &carol, &g, None, -1).await.unwrap();
    assert_eq!(cached.seated_players.get().iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(), [(1, -1), (2, -1)]);

    // Not refreshed within 6 s unless forced; a refresh shares the new dictionary.
    f.advance_seconds(5);
    f.hub.update_server_status_if_needed(false);
    assert_eq!(f.hub.running_games[0].nr_of_players, 1);
    f.advance_seconds(2);
    f.hub.update_server_status_if_needed(false);
    let fresh = f.hub.running_games[0].clone();
    assert_eq!(fresh.nr_of_players, 3);
    f.hub.request_kick(&alice, &g, 3).await.unwrap();
    assert!(!fresh.you_are_in(3));
    // Two extractions while the same dictionary is current share it.
    let again = f.hub.extract_game_info(&f.game(&g));
    f.hub.request_assign_seats(&alice, &g, NetDictionary::from_iter([(1, 2)])).await.unwrap();
    assert_eq!(again.seated_players.get().len(), 2);
    assert_eq!(fresh.seated_players.get().len(), 2);
}

#[tokio::test]
async fn filtered_server_status_scopes() {
    let mut f = fixture().await;
    let alice = f.user("alice").await;
    let bob = f.user("bobby").await;
    let g = f.create_game(1, &alice, None, None).await;
    // The first heartbeat restores the caches from the database, and refreshes the status
    // before it sets LastSeenDateTime: the caller is not listed yet.
    let r = f.hub.request_heartbeat(&alice, GameListScope::Active).await.unwrap();
    assert!(r.contents.unwrap().recently_seen_users.is_empty());
    f.hub.update_server_status_if_needed(true);

    let s = f.hub.filtered_server_status(GameListScope::None, 1);
    assert!(s.running_games.is_empty() && s.own_games.is_empty() && s.scheduled_games.is_empty());
    assert_eq!(s.recently_seen_users.len(), 1);
    assert_eq!(f.hub.filtered_server_status(GameListScope::All, 2).running_games.len(), 1);
    assert_eq!(f.hub.filtered_server_status(GameListScope::All, 2).own_games.len(), 0);

    // After 120 minutes without activity a game is only listed (Active) for its players.
    f.advance_seconds(121 * 60);
    f.hub.update_server_status_if_needed(true);
    assert_eq!(f.hub.filtered_server_status(GameListScope::Active, 2).running_games.len(), 0);
    assert_eq!(f.hub.filtered_server_status(GameListScope::Active, 1).running_games.len(), 1);
    f.hub.request_join_game(&cx(2), &bob, &g, None, -1).await.unwrap();
    assert_eq!(f.hub.filtered_server_status(GameListScope::Active, 2).running_games.len(), 1);
    // Users seen more than a minute ago drop out of RecentlySeenUsers.
    assert!(f.hub.recently_seen_users.is_empty());
}

#[tokio::test]
async fn scheduled_games() {
    let mut f = fixture().await;
    let alice = f.user("alice").await;
    let bob = f.user("bobby").await;
    for _ in 0..3 {
        assert!(f.hub.request_schedule_game(&alice, start(), None, 6, 10, vec![Faction::Atreides], false).await.unwrap().success);
    }
    assert_eq!(error_of(&f.hub.request_schedule_game(&alice, start(), None, 6, 10, vec![], false).await.unwrap()), ErrorType::TooManyScheduledGames);
    let id = f.hub.scheduled_games_by_game_id.keys().next().unwrap().clone();

    assert_eq!(error_of(&f.hub.request_subscribe_game(&bob, None, SubscriptionType::MaybeAsPlayer).await.unwrap()), ErrorType::GameNotFound);
    let r = f.hub.request_subscribe_game(&bob, Some(&id), SubscriptionType::MaybeAsPlayer).await.unwrap();
    let info = r.contents.unwrap().scheduled_games.into_iter().find(|s| s.scheduled_game_id.as_deref() == Some(id.as_str())).unwrap();
    assert_eq!(info.subscriber_names.iter().map(|(k, v)| (*k, v.as_str())).collect::<Vec<_>>(), [(1, "alice"), (2, "bobby")]);
    assert_eq!(info.subscribers[&1], SubscriptionType::CertainAsHost);

    assert_eq!(error_of(&f.hub.request_cancel_game(&bob, &id).await.unwrap()), ErrorType::NoHost);
    assert_eq!(error_of(&f.hub.request_cancel_game(&alice, "nope").await.unwrap()), ErrorType::ScheduledGameNotFound);
    let r = f.hub.request_cancel_game(&alice, &id).await.unwrap();
    assert_eq!(r.contents.unwrap().scheduled_games.len(), 2);
}

#[tokio::test]
async fn bot_speed_goes_to_every_connection() {
    let mut f = fixture().await;
    let alice = f.user("alice").await;
    let g = f.create_game(1, &alice, None, None).await;
    f.take_log();
    f.hub.request_set_bot_speed(&alice, &g, 0).await.unwrap();
    assert_eq!(f.take_log().len(), 5);
    assert_eq!(f.game(&g).borrow().game.participation.borrow().bots_speed, 0);
}

/// One `EstablishPlayers` with `FillWithBots`, as a C# savegame (as in hub_other.rs).
const SAVEGAME: &str = r#"{"Version":188,"_events":[{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":6,"MaximumTurns":10,"InitialRules":[10000,10001,998,40,85],"AllowedFactionsInPlay":[20,30,40,50,10,60],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1,"MaximumNumberOfPlayers":6,"MaximumTurns":10,"ApplicableRules":[],"_players":"","_factionsInPlay":"","Initiator":0,"Time":"2026-09-19T20:36:09.766684-04:00"}]}"#;

#[tokio::test]
async fn end_of_game_mail() {
    use treachery_server::mail::MailPriority;
    let mut f = fixture().await;
    let alice = f.user("alice").await;
    let r = f.hub.request_create_game(&cx(1), Some("Mail"), &alice, None, Some(SAVEGAME), None).await.unwrap();
    let g = r.contents.unwrap().game_id.unwrap();
    let game = f.game(&g);

    // No GameEndEmailFrom/To: nothing is sent.
    f.hub.send_end_of_game_mail(&game).await.unwrap();
    let mailer = Rc::new(RecordingMailer::default());
    f.hub.services.mailer = mailer.clone();
    for (k, v) in [("GameEndEmailUsername", "u"), ("GameEndEmailFrom", "from@example.com"), ("GameEndEmailTo", "to@example.com")] {
        f.hub.services.config.set(k, v);
    }
    f.hub.send_end_of_game_mail(&game).await.unwrap();
    let sent = mailer.sent.borrow();
    let mail = &sent[0];

    let (turn, players) = {
        let mg = game.borrow();
        (mg.game.current_turn, mg.game.players.iter().map(|p| (p.faction, p.seat)).collect::<Vec<_>>())
    };
    assert_eq!(players.len(), 6);
    let info = f.hub.extract_game_info(&game);
    // One seated user (the creator, seat -1) and six bots.
    assert_eq!(mail.subject, format!("Mail (1 Players, 6 Bots, Turn {turn} - {})", info.ruleset));
    assert_eq!(mail.priority, MailPriority::Low);
    assert!(mail.is_body_html);
    assert_eq!(mail.attachments[0].name, "savegame20260302.1000.json");
    assert_eq!(mail.attachments[0].content, treachery_engine::game_state::GameState::get_state_as_string(&game.borrow().game));

    let mut expected = String::from("<h1>Player table in HTML format</h1>\n<table>\n<thead>\n<tr><th>Faction</th><th>Seat</th><th>UserId</th>\n</thead>\n<tbody>\n");
    for (faction, seat) in &players {
        expected += &format!("<tr><td>{}</td><td>{seat}</td><td>-1</td></tr>\n", treachery_server::render::describe_faction(*faction));
    }
    expected += "</tbody>\n</table>\n<h1>Player table in CSV format</h1>\n<p>\nFaction;Seat;UserId\n";
    for (faction, seat) in &players {
        expected += &format!("{};{seat};-1\n", treachery_server::render::describe_faction(*faction));
    }
    expected += "</p>\n";
    assert_eq!(mail.body, expected);
    assert!(mail.body.contains("Atreides") || mail.body.contains("Harkonnen"));
}

#[tokio::test]
async fn end_of_game_mail_bad_address_escapes() {
    let mut f = fixture().await;
    let alice = f.user("alice").await;
    let g = f.create_game(1, &alice, None, None).await;
    f.hub.services.config.set("GameEndEmailFrom", "not an address");
    f.hub.services.config.set("GameEndEmailTo", "to@example.com");
    let game = f.game(&g);
    assert!(f.hub.send_end_of_game_mail(&game).await.is_err());
}
