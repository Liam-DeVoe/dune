//! Unit tests for package H4: `hub/other.rs` (GameHub_Other.cs) and the `hub/mod.rs` helpers
//! (`AreValid`, `SendMail`). Each test states the C# behaviour it pins down. The equivalence
//! harness compares the same paths against the C# hub; these tests run without .NET.

use std::cell::RefCell;
use std::rc::Rc;

use dotnet_compat::NetDictionary;
use sqlx::Row;
use treachery_engine::cs::Shared;
use treachery_engine::date_time_offset::{DateTimeOffset, TICKS_PER_DAY, TICKS_PER_MINUTE};
use treachery_engine::enumerations::{ErrorType, Faction, Ruleset, SubscriptionType};
use treachery_engine::game::Game;
use treachery_engine::game_state::GameState;
use treachery_engine::participation::Participation;
use treachery_server::clock::{Clock, ManualClock};
use treachery_server::config::Config;
use treachery_server::db::persisted_game::PersistedGame;
use treachery_server::db::persisted_scheduled_game::PersistedScheduledGame;
use treachery_server::db::persisted_user_token::PersistedUserToken;
use treachery_server::db::user::User;
use treachery_server::db::{date_time, Db};
use treachery_server::hub::{GameHub, ManagedGameRef, SequentialIds, Services};
use treachery_server::logged_in_user::LoggedInUser;
use treachery_server::mail::{MailAddress, MailMessage, MailPriority, RecordingMailer};
use treachery_server::model::managed_game::ManagedGame;
use treachery_server::model::result::CallResult;
use treachery_server::model::scheduled_game::ScheduledGame;
use treachery_server::timers::RecordingTimers;

const ADMIN: &str = "admin-user";

/// One `EstablishPlayers`, as a C# savegame.
const SAVEGAME: &str = r#"{"Version":188,"_events":[{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":6,"MaximumTurns":10,"InitialRules":[10000,10001,998,40,85],"AllowedFactionsInPlay":[20,30,40,50,10,60],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1,"MaximumNumberOfPlayers":6,"MaximumTurns":10,"ApplicableRules":[],"_players":"","_factionsInPlay":"","Initiator":0,"Time":"2026-09-19T20:36:09.766684-04:00"}]}"#;

struct Fixture {
    hub: GameHub,
    clock: Rc<ManualClock>,
    mailer: Rc<RecordingMailer>,
}

fn t(text: &str) -> DateTimeOffset {
    date_time::parse(text).unwrap()
}

fn start() -> DateTimeOffset {
    t("2026-11-08 12:00:00-05:00")
}

async fn fixture_with(config: Config) -> Fixture {
    let clock = Rc::new(ManualClock::new(start()));
    let mailer = Rc::new(RecordingMailer::default());
    let services = Services {
        db: Db::open_in_memory().await.unwrap(),
        config,
        clock: clock.clone(),
        timers: Rc::new(RecordingTimers::default()),
        mailer: mailer.clone(),
        ids: Rc::new(SequentialIds::default()),
    };
    Fixture { hub: GameHub::new(services), clock, mailer }
}

async fn fixture() -> Fixture {
    fixture_with(Config::from_pairs([("GameAdminUsername", ADMIN)])).await
}

impl Fixture {
    fn advance_minutes(&self, minutes: i64) {
        let now = self.clock.now();
        self.clock.set(DateTimeOffset::new(now.ticks + minutes * TICKS_PER_MINUTE, now.offset_minutes));
    }

    fn user(id: i32, name: &str) -> User {
        User { id, name: name.to_string(), player_name: format!("{name} player"), email: format!("{name}@example.com"), ..User::default() }
    }

    /// Logs a user in with `token`, as `LoginAndCreateToken` leaves the caches.
    fn login(&mut self, id: i32, name: &str, token: &str) -> treachery_server::logged_in_user::UserRef {
        let user = Rc::new(RefCell::new(Self::user(id, name)));
        self.hub.users_by_id.insert(id, user.clone());
        let now = self.hub.now();
        self.hub.users_by_user_token.insert(token.to_string(), LoggedInUser::new(user.clone(), None, now));
        user
    }

    fn add_game(&mut self, game_id: &str, creator: i32, game: Game) -> ManagedGameRef {
        let managed = Rc::new(RefCell::new(ManagedGame {
            creation_date: start(),
            last_activity: start(),
            last_persisted: DateTimeOffset::default(),
            creator_user_id: creator,
            game_id: game_id.to_string(),
            game,
            name: Some(format!("{game_id} name")),
            hashed_password: None,
            observers_require_password: false,
            statistics_sent: false,
            last_async_play_message_sent: DateTimeOffset::default(),
            seated_players_ref: None,
            bots: Vec::new(),
        }));
        self.hub.running_games_by_game_id.insert(game_id.to_string(), managed.clone());
        managed
    }

    async fn persisted_games(&self) -> Vec<PersistedGame> {
        let mut games = self.hub.services.db.persisted_games_all().await.unwrap();
        games.sort_by(|a, b| a.game_id.cmp(&b.game_id));
        games
    }
}

fn empty_game() -> Game {
    Game::new_int32_participation(Game::LATEST_VERSION, &Shared::new(Participation::new()))
}

fn loaded_game() -> Game {
    let mut game = None;
    let message = Game::try_load(&GameState::load(SAVEGAME), &Shared::new(Participation::new()), false, true, &mut game);
    assert!(message.is_none());
    game.unwrap()
}

fn error_of<T>(r: &CallResult<T>) -> ErrorType {
    assert!(!r.success);
    r.error
}

// ---------------------------------------------------------------------------------------------
// AreValid and SendMail (GameHub.cs)

#[tokio::test]
async fn are_valid_checks_token_then_game() {
    let mut f = fixture().await;
    f.login(1, "alice", "tok");
    f.add_game("g1", 1, empty_game());

    let e = |r: Result<_, CallResult<()>>| r.err().unwrap().error;
    assert_eq!(e(f.hub.are_valid::<()>(None, Some("g1"))), ErrorType::UserTokenNotFound);
    assert_eq!(e(f.hub.are_valid::<()>(Some("nope"), Some("g1"))), ErrorType::UserNotFound);
    // A null game id is checked only after the token.
    assert_eq!(e(f.hub.are_valid::<()>(Some("nope"), None)), ErrorType::UserNotFound);
    assert_eq!(e(f.hub.are_valid::<()>(Some("tok"), None)), ErrorType::GameIdNotFound);
    assert_eq!(e(f.hub.are_valid::<()>(Some("tok"), Some("g2"))), ErrorType::GameNotFound);

    let (user, game) = f.hub.are_valid::<()>(Some("tok"), Some("g1")).ok().unwrap();
    assert_eq!(user.borrow().id, 1);
    assert!(Rc::ptr_eq(&game, &f.hub.running_games_by_game_id["g1"]));
    // The user is the LoggedInUser's own object.
    assert!(Rc::ptr_eq(&user, &f.hub.users_by_user_token["tok"].user));

    // The error results carry no details and no contents.
    let r = f.hub.are_valid::<()>(Some("tok"), Some("g2")).err().unwrap();
    assert_eq!((r.success, r.error_details.as_str(), r.contents), (false, "", None));
}

fn mail() -> MailMessage {
    MailMessage {
        from: MailAddress::parse("from@example.com").unwrap(),
        to: vec![MailAddress::parse("to@example.com").unwrap()],
        subject: "s".into(),
        body: "b".into(),
        is_body_html: false,
        priority: MailPriority::Normal,
        attachments: vec![],
    }
}

#[tokio::test]
async fn send_mail_needs_a_username() {
    let f = fixture().await;
    f.hub.send_mail(mail());
    assert!(f.mailer.sent.borrow().is_empty());

    let f = fixture_with(Config::from_pairs([("GameEndEmailUsername", "")])).await;
    f.hub.send_mail(mail());
    assert!(f.mailer.sent.borrow().is_empty());

    let f = fixture_with(Config::from_pairs([("GameEndEmailUsername", "u")])).await;
    f.hub.send_mail(mail());
    assert_eq!(*f.mailer.sent.borrow(), vec![mail()]);
}

#[tokio::test]
async fn send_mail_settings() {
    use treachery_server::mail::{Mailer, SmtpSettings};
    #[derive(Default)]
    struct Capture(RefCell<Vec<SmtpSettings>>);
    impl Mailer for Capture {
        fn send(&self, settings: &SmtpSettings, _mail: MailMessage) {
            self.0.borrow_mut().push(settings.clone());
        }
    }
    let mut f = fixture_with(Config::from_pairs([("GameEndEmailUsername", "u"), ("GameEndEmailPassword", "p")])).await;
    let capture = Rc::new(Capture::default());
    f.hub.services.mailer = capture.clone();
    f.hub.send_mail(mail());
    f.hub.services.config.set("SmtpHost", "mail.example.org");
    f.hub.send_mail(mail());
    let sent = capture.0.borrow();
    assert_eq!((sent[0].host.as_str(), sent[0].port, sent[0].username.as_str(), sent[0].password.as_str()), ("smtp.strato.com", 25, "u", "p"));
    assert_eq!(sent[1].host, "mail.example.org");
}

// ---------------------------------------------------------------------------------------------
// Connect and restore

#[tokio::test]
async fn connect_restores_once_and_reports_server_info() {
    let mut f = fixture_with(Config::default()).await;
    assert_eq!(f.hub.last_restored, DateTimeOffset::default());

    let r = f.hub.connect().await.unwrap();
    assert!(r.success);
    let info = r.contents.unwrap();
    assert_eq!(info.admin_name, "admin");
    assert_eq!(info.scheduled_maintenance, DateTimeOffset::default());
    assert_eq!(f.hub.last_restored, start());
    assert_eq!(f.hub.last_persisted_running_games, start());
    assert!(!f.hub.restoring);

    // Restored already: a later connect doesn't restore again.
    f.advance_minutes(10);
    f.hub.connect().await.unwrap();
    assert_eq!(f.hub.last_restored, start());

    let mut f = fixture().await;
    f.hub.maintenance_date = t("2026-12-01 00:00:00+00:00");
    let info = f.hub.connect().await.unwrap().contents.unwrap();
    assert_eq!(info.admin_name, ADMIN);
    assert_eq!(info.scheduled_maintenance, t("2026-12-01 00:00:00+00:00"));
}

#[tokio::test]
async fn restore_loads_users_tokens_games_and_scheduled_games() {
    let mut f = fixture().await;
    let db = f.hub.services.db.clone();
    let alice = db.users_insert(&Fixture::user(0, "alice")).await.unwrap();
    let bob = db.users_insert(&Fixture::user(0, "bob")).await.unwrap();

    let old = DateTimeOffset::new(start().ticks - 90 * TICKS_PER_DAY, start().offset_minutes);
    let almost_old = DateTimeOffset::new(old.ticks + 1, old.offset_minutes);
    db.user_tokens_insert(&PersistedUserToken { token: "fresh".into(), user_id: alice, created: almost_old }).await.unwrap();
    db.user_tokens_insert(&PersistedUserToken { token: "expired".into(), user_id: alice, created: old }).await.unwrap();
    db.user_tokens_insert(&PersistedUserToken { token: "orphan".into(), user_id: 999, created: start() }).await.unwrap();
    db.user_tokens_insert(&PersistedUserToken { token: "kept".into(), user_id: bob, created: start() }).await.unwrap();

    // A token already in memory keeps its LoggedInUser, and so its old User object.
    let stale_bob = f.login(bob, "bob-old", "kept");
    f.hub.users_by_user_token.get_mut("kept").unwrap().status = treachery_engine::enumerations::UserStatus::Away;

    let game = |id: Option<&str>, state: &str| PersistedGame {
        game_id: id.map(str::to_string),
        game_name: id.map(|i| format!("{i} name")),
        creation_date: t("2026-09-19 20:00:00-04:00"),
        creator_user_id: alice,
        game_state: state.to_string(),
        game_participation: r#"{"PlayerNames":{"1":"alice"},"SeatedPlayers":{"1":0},"Observers":[],"Hosts":[1],"AvailableSeats":[],"Kicked":[],"BotsSpeed":0}"#.to_string(),
        hashed_password: Some("pw".into()),
        observers_require_password: true,
        statistics_sent: true,
        last_async_play_message_sent: t("2026-09-20 00:00:00+00:00"),
        last_action: t("2026-09-21 00:00:00+00:00"),
        ..PersistedGame::default()
    };
    db.persisted_games_insert(&game(Some("g1"), SAVEGAME)).await.unwrap();
    db.persisted_games_insert(&game(Some("g1"), SAVEGAME)).await.unwrap(); // same id: counted, not added
    db.persisted_games_insert(&game(None, SAVEGAME)).await.unwrap(); // no id: skipped
    db.persisted_games_insert(&game(Some("bad"), "not json")).await.unwrap(); // throws in Load: logged
    db.persisted_games_insert(&game(Some("v1"), r#"{"Version":1,"_events":[]}"#)).await.unwrap(); // TryLoad message

    let scheduled = PersistedScheduledGame {
        game_id: "s1".into(),
        creator_user_id: alice,
        creator_player_name: "alice player".into(),
        date_time: t("2026-11-10 20:00:00+01:00"),
        number_of_players: 5,
        maximum_turns: 7,
        ruleset: Some(Ruleset(3)),
        async_play: true,
        allowed_factions_in_play: vec![Faction(1), Faction(2)],
        subscribed_users: r#"{"1":1,"2":2}"#.into(),
        ..PersistedScheduledGame::default()
    };
    db.scheduled_games_insert(&scheduled).await.unwrap();

    // A running game not in the database is dropped.
    f.add_game("unsaved", alice, empty_game());

    let (running, scheduled_count) = f.hub.restore_games_and_user_cache().await.unwrap();
    assert_eq!((running, scheduled_count), (2, 1));

    assert_eq!(f.hub.users_by_id.keys().copied().collect::<Vec<_>>(), vec![alice, bob]);
    assert_eq!(f.hub.users_by_user_token.keys().cloned().collect::<Vec<_>>(), vec!["fresh".to_string(), "kept".to_string()]);
    let fresh = &f.hub.users_by_user_token["fresh"];
    assert!(Rc::ptr_eq(&fresh.user, &f.hub.users_by_id[&alice]));
    assert_eq!(fresh.logged_in_date_time, almost_old);
    let kept = &f.hub.users_by_user_token["kept"];
    assert!(Rc::ptr_eq(&kept.user, &stale_bob));
    assert!(!Rc::ptr_eq(&kept.user, &f.hub.users_by_id[&bob]));
    assert_eq!(kept.status, treachery_engine::enumerations::UserStatus::Away);

    assert_eq!(f.hub.running_games_by_game_id.keys().cloned().collect::<Vec<_>>(), vec!["g1".to_string()]);
    let g = f.hub.running_games_by_game_id["g1"].borrow();
    assert_eq!(g.game_id, "g1");
    assert_eq!(g.name.as_deref(), Some("g1 name"));
    assert_eq!(g.hashed_password.as_deref(), Some("pw"));
    assert!(g.observers_require_password && g.statistics_sent);
    assert_eq!(g.last_activity, t("2026-09-21 00:00:00+00:00"));
    assert_eq!(g.last_persisted, DateTimeOffset::default());
    assert_eq!(g.creation_date, t("2026-09-19 20:00:00-04:00"));
    assert_eq!(g.game.history.len(), 1);
    assert_eq!(g.game.participation.borrow().player_names.get(&1).map(String::as_str), Some("alice"));
    drop(g);

    let s = &f.hub.scheduled_games_by_game_id["s1"];
    assert_eq!((s.scheduled_game_id.as_str(), s.number_of_players, s.maximum_turns, s.ruleset, s.async_play), ("s1", 5, 7, Some(Ruleset(3)), true));
    assert_eq!(s.allowed_factions_in_play, vec![Faction(1), Faction(2)]);
    assert_eq!(s.subscribed_users.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(), vec![(1, SubscriptionType(1)), (2, SubscriptionType(2))]);
    assert_eq!(s.date_time, t("2026-11-10 20:00:00+01:00"));

    assert_eq!(f.hub.last_restored, start());
    assert!(!f.hub.restoring);
}

#[tokio::test]
async fn restore_with_bad_subscriptions_throws_and_stays_restoring() {
    let mut f = fixture().await;
    let scheduled = PersistedScheduledGame { game_id: "s1".into(), subscribed_users: "{".into(), ..PersistedScheduledGame::default() };
    f.hub.services.db.scheduled_games_insert(&scheduled).await.unwrap();
    assert!(f.hub.restore_games_and_user_cache().await.is_err());
    assert!(f.hub.restoring);
    assert_eq!(f.hub.last_restored, DateTimeOffset::default());
}

// ---------------------------------------------------------------------------------------------
// Persistence

#[tokio::test]
async fn persist_game_inserts_then_updates_only_on_new_activity() {
    let mut f = fixture().await;
    let game = f.add_game("g1", 7, loaded_game());

    f.hub.persist_game(&game).await.unwrap();
    assert_eq!(game.borrow().last_persisted, start());
    let rows = f.persisted_games().await;
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.game_id.as_deref(), Some("g1"));
    assert_eq!(row.game_name.as_deref(), Some("g1 name"));
    assert_eq!(row.creator_user_id, 7);
    // A new row's LastAction is the time of the last event, not LastActivity.
    assert_eq!(row.last_action, t("2026-09-19 20:36:09.766684-04:00"));
    assert_eq!(row.game_state, GameState::get_state_as_string(&game.borrow().game));
    assert_eq!(row.game_participation, treachery_server::json::serialize_participation(&game.borrow().game.participation.borrow()));

    // LastAction differs from LastActivity, so the next persist updates the row.
    game.borrow_mut().statistics_sent = true;
    f.hub.persist_game(&game).await.unwrap();
    let row = &f.persisted_games().await[0];
    assert_eq!(row.last_action, start());
    assert!(row.statistics_sent);

    // Same LastActivity: nothing is written, but LastPersisted moves.
    game.borrow_mut().statistics_sent = false;
    f.advance_minutes(1);
    f.hub.persist_game(&game).await.unwrap();
    assert!(f.persisted_games().await[0].statistics_sent);
    assert_eq!(game.borrow().last_persisted, f.hub.now());
}

#[tokio::test]
async fn persist_game_without_history_uses_creation_date() {
    let mut f = fixture().await;
    let game = f.add_game("g1", 7, empty_game());
    game.borrow_mut().last_activity = t("2026-11-09 00:00:00+00:00");
    f.hub.persist_game(&game).await.unwrap();
    assert_eq!(f.persisted_games().await[0].last_action, start());
}

#[tokio::test]
async fn persist_game_if_needed_waits_five_minutes() {
    let mut f = fixture().await;
    let game = f.add_game("g1", 7, empty_game());
    game.borrow_mut().last_persisted = start();
    f.advance_minutes(5);
    // LastPersisted + 5 min > now is false at exactly 5 minutes, so it persists.
    f.hub.persist_game_if_needed(&game).await.unwrap();
    assert_eq!(f.persisted_games().await.len(), 1);

    let game2 = f.add_game("g2", 7, empty_game());
    game2.borrow_mut().last_persisted = f.hub.now();
    f.advance_minutes(4);
    f.hub.persist_game_if_needed(&game2).await.unwrap();
    assert_eq!(f.persisted_games().await.len(), 1);
    assert_eq!(game2.borrow().last_persisted, t("2026-11-08 12:05:00-05:00"));
}

#[tokio::test]
async fn persist_running_games_counts() {
    let mut f = fixture().await;
    // Not restored yet: nothing happens.
    f.add_game("g1", 1, empty_game());
    assert_eq!(f.hub.persist_running_games().await.unwrap(), (0, 0, 0, 0));
    assert!(f.persisted_games().await.is_empty());

    f.hub.last_restored = start();
    f.hub.last_persisted_running_games = DateTimeOffset::default();
    f.add_game("g2", 1, loaded_game());
    assert_eq!(f.hub.persist_running_games().await.unwrap(), (2, 0, 0, 0));
    assert_eq!(f.hub.last_persisted_running_games, start());
    let rows = f.persisted_games().await;
    assert_eq!(rows.iter().map(|r| r.game_id.clone().unwrap()).collect::<Vec<_>>(), vec!["g1", "g2"]);
    // GameId is the dictionary key.
    assert_eq!(rows[1].last_action, t("2026-09-19 20:36:09.766684-04:00"));

    // No activity since the last persist: skipped entirely (not even counted as unchanged).
    assert_eq!(f.hub.persist_running_games().await.unwrap(), (0, 0, 0, 0));

    f.advance_minutes(1);
    let now = f.hub.now();
    // g1: LastActivity now differs from its LastAction: updated. g2: equal: unchanged.
    f.hub.running_games_by_game_id["g1"].borrow_mut().last_activity = now;
    f.hub.running_games_by_game_id["g2"].borrow_mut().last_activity = now;
    f.hub.services.db.persisted_games_update_all(&PersistedGame { last_action: now, ..rows[1].clone() }).await.unwrap();
    // A row without a running game is deleted; a row without an id is not.
    f.hub
        .services
        .db
        .persisted_games_insert(&PersistedGame { game_id: Some("gone".into()), ..PersistedGame::default() })
        .await
        .unwrap();
    f.hub.services.db.persisted_games_insert(&PersistedGame { game_id: None, ..PersistedGame::default() }).await.unwrap();
    f.hub.last_persisted_running_games = start();
    assert_eq!(f.hub.persist_running_games().await.unwrap(), (0, 1, 1, 1));
    let rows = f.persisted_games().await;
    assert_eq!(rows.iter().map(|r| r.game_id.clone()).collect::<Vec<_>>(), vec![None, Some("g1".into()), Some("g2".into())]);
    assert_eq!(rows[1].last_action, now);
}

#[tokio::test]
async fn persist_scheduled_games_replaces_the_table() {
    let mut f = fixture().await;
    let mut subscribed = NetDictionary::new();
    subscribed.add(5, SubscriptionType(2));
    subscribed.add(3, SubscriptionType(1));
    let game = ScheduledGame {
        scheduled_game_id: "ignored".into(),
        date_time: t("2026-11-10 20:00:00+01:00"),
        creator_user_id: 3,
        creator_player_name: "c".into(),
        ruleset: None,
        allowed_factions_in_play: vec![Faction(10), Faction(20)],
        subscribed_users: subscribed,
        ..ScheduledGame::default()
    };
    f.hub.scheduled_games_by_game_id.insert("s1".into(), game.clone());
    f.hub.scheduled_games_by_game_id.insert("s2".into(), ScheduledGame::default());

    // Not restored yet.
    assert_eq!(f.hub.persist_scheduled_games().await.unwrap(), 0);
    assert_eq!(f.hub.last_persisted_scheduled_games, DateTimeOffset::default());

    f.hub.last_restored = start();
    f.hub.services.db.scheduled_games_insert(&PersistedScheduledGame { game_id: "old".into(), ..Default::default() }).await.unwrap();
    assert_eq!(f.hub.persist_scheduled_games().await.unwrap(), 2);
    assert_eq!(f.hub.last_persisted_scheduled_games, start());

    let rows = sqlx::query(r#"SELECT "GameId", "AllowedFactionsInPlay", "SubscribedUsers", "Ruleset", "NumberOfPlayers", "MaximumTurns", "DateTime" FROM "ScheduledGames" ORDER BY "GameId""#)
        .fetch_all(&f.hub.services.db.pool)
        .await
        .unwrap();
    let row = |i: usize| {
        let r = &rows[i];
        (
            r.get::<String, _>(0),
            r.get::<String, _>(1),
            r.get::<String, _>(2),
            r.get::<Option<i32>, _>(3),
            r.get::<i32, _>(4),
            r.get::<i32, _>(5),
            r.get::<String, _>(6),
        )
    };
    assert_eq!(rows.len(), 2);
    assert_eq!(
        row(0),
        ("s1".into(), "[10,20]".into(), r#"{"5":2,"3":1}"#.into(), None, 6, 10, "2026-11-10 20:00:00+01:00".into())
    );
    assert_eq!(row(1).1, "[]");
    assert_eq!(row(1).2, "{}");

    // PersistScheduledGamesIfNeeded: every 30 minutes.
    f.hub.scheduled_games_by_game_id.remove("s2");
    f.advance_minutes(29);
    f.hub.persist_scheduled_games_if_needed().await.unwrap();
    assert_eq!(f.hub.services.db.scheduled_games_all().await.unwrap().len(), 2);
    f.advance_minutes(1);
    f.hub.persist_scheduled_games_if_needed().await.unwrap();
    assert_eq!(f.hub.services.db.scheduled_games_all().await.unwrap().len(), 1);
}

// ---------------------------------------------------------------------------------------------
// Cleanup

#[tokio::test]
async fn cleanup_user_tokens_removes_tokens_older_than_90_days() {
    let mut f = fixture().await;
    let db = f.hub.services.db.clone();
    let now = start();
    let days_ago = |d: i64, extra_ticks: i64| DateTimeOffset::new(now.ticks - d * TICKS_PER_DAY + extra_ticks, now.offset_minutes);

    let user = Rc::new(RefCell::new(Fixture::user(1, "alice")));
    f.hub.users_by_user_token.insert("old".into(), LoggedInUser::new(user.clone(), Some(days_ago(90, 0)), now));
    f.hub.users_by_user_token.insert("young".into(), LoggedInUser::new(user.clone(), Some(days_ago(90, 1)), now));
    db.user_tokens_insert(&PersistedUserToken { token: "old".into(), user_id: 1, created: days_ago(90, 0) }).await.unwrap();
    db.user_tokens_insert(&PersistedUserToken { token: "young".into(), user_id: 1, created: days_ago(90, 1) }).await.unwrap();

    f.hub.cleanup_user_tokens_if_needed().await.unwrap();
    assert_eq!(f.hub.users_by_user_token.keys().cloned().collect::<Vec<_>>(), vec!["young".to_string()]);
    assert_eq!(db.user_tokens_all().await.unwrap().into_iter().map(|t| t.token).collect::<Vec<_>>(), vec!["young".to_string()]);
    assert_eq!(f.hub.last_cleaned_up_user_tokens, now);

    // Within 7 hours of the last cleanup nothing happens.
    f.advance_minutes(7 * 60 - 1);
    f.hub.users_by_user_token.insert("old2".into(), LoggedInUser::new(user.clone(), Some(days_ago(200, 0)), now));
    f.hub.cleanup_user_tokens_if_needed().await.unwrap();
    assert!(f.hub.users_by_user_token.contains_key("old2"));
    f.advance_minutes(1);
    f.hub.cleanup_user_tokens_if_needed().await.unwrap();
    assert!(!f.hub.users_by_user_token.contains_key("old2"));
}

#[tokio::test]
async fn cleanup_scheduled_games_drops_games_more_than_4_hours_past() {
    let mut f = fixture().await;
    let now = start();
    let at = |minutes: i64| ScheduledGame { date_time: DateTimeOffset::new(now.ticks + minutes * TICKS_PER_MINUTE, 0), ..ScheduledGame::default() };
    // Offset 0 with clock ticks shifted: compare instants, not clock times.
    let in_utc = |minutes: i64| ScheduledGame { date_time: DateTimeOffset::new(now.utc_ticks() + minutes * TICKS_PER_MINUTE, 0), ..ScheduledGame::default() };
    f.hub.scheduled_games_by_game_id.insert("past".into(), in_utc(-4 * 60 - 1));
    f.hub.scheduled_games_by_game_id.insert("edge".into(), in_utc(-4 * 60));
    f.hub.scheduled_games_by_game_id.insert("future".into(), at(60));

    f.hub.cleanup_scheduled_games_if_needed();
    assert_eq!(f.hub.scheduled_games_by_game_id.keys().cloned().collect::<Vec<_>>(), vec!["edge".to_string(), "future".to_string()]);
    assert_eq!(f.hub.last_cleaned_up_scheduled_games, now);

    f.hub.scheduled_games_by_game_id.insert("past".into(), in_utc(-1000));
    f.advance_minutes(60);
    f.hub.cleanup_scheduled_games_if_needed();
    assert!(f.hub.scheduled_games_by_game_id.contains_key("past"));
}

// ---------------------------------------------------------------------------------------------
// Admin

#[tokio::test]
async fn admin_methods_require_the_configured_admin() {
    let mut f = fixture().await;
    f.login(1, "alice", "user");
    f.login(2, ADMIN, "admin");
    f.add_game("g1", 1, empty_game());

    let m = t("2026-12-01 00:00:00+00:00");
    for token in ["user", "nobody"] {
        assert_eq!(error_of(&f.hub.admin_update_maintenance(token, m).await.unwrap()), ErrorType::InvalidUserNameOrPassword);
        assert_eq!(error_of(&f.hub.admin_persist_state(token).await.unwrap()), ErrorType::InvalidUserNameOrPassword);
        assert_eq!(error_of(&f.hub.admin_restore_state(token).await.unwrap()), ErrorType::InvalidUserNameOrPassword);
        assert_eq!(error_of(&f.hub.admin_close_game(token, "g1").await.unwrap()), ErrorType::InvalidUserNameOrPassword);
        assert_eq!(error_of(&f.hub.admin_download_game(token, "g1").await.unwrap()), ErrorType::InvalidUserNameOrPassword);
        assert_eq!(error_of(&f.hub.admin_cancel_game(token, "s").await.unwrap()), ErrorType::InvalidUserNameOrPassword);
        assert_eq!(error_of(&f.hub.get_admin_info(token).await.unwrap()), ErrorType::InvalidUserNameOrPassword);
    }
    assert_eq!(f.hub.maintenance_date, DateTimeOffset::default());

    let r = f.hub.admin_update_maintenance("admin", m).await.unwrap();
    assert_eq!(r.contents.as_deref(), Some("Maintenance window updated"));
    assert_eq!(f.hub.maintenance_date, m);

    // Without GameAdminUsername nobody is admin.
    let mut f = fixture_with(Config::default()).await;
    f.login(2, ADMIN, "admin");
    assert_eq!(error_of(&f.hub.admin_update_maintenance("admin", m).await.unwrap()), ErrorType::InvalidUserNameOrPassword);
}

#[tokio::test]
async fn admin_persist_and_restore_state_report_counts() {
    let mut f = fixture().await;
    f.login(2, ADMIN, "admin");
    // Not restored yet: persisting does nothing.
    f.add_game("g1", 1, loaded_game());
    let r = f.hub.admin_persist_state("admin").await.unwrap();
    assert_eq!(r.contents.as_deref(), Some("Games: 0 new, 0 updated, 0 unchanged, 0 deleted. Scheduled games: 0."));

    f.hub.last_restored = start();
    f.hub.last_persisted_running_games = DateTimeOffset::default();
    f.hub.scheduled_games_by_game_id.insert("s1".into(), ScheduledGame::default());
    let r = f.hub.admin_persist_state("admin").await.unwrap();
    assert_eq!(r.contents.as_deref(), Some("Games: 1 new, 0 updated, 0 unchanged, 0 deleted. Scheduled games: 1."));

    // Unsaved changes are discarded by a restore.
    f.hub.running_games_by_game_id.clear();
    f.add_game("g2", 1, empty_game());
    let r = f.hub.admin_restore_state("admin").await.unwrap();
    assert_eq!(r.contents.as_deref(), Some("Restored: 1 running games, 1 scheduled games"));
    assert_eq!(f.hub.running_games_by_game_id.keys().cloned().collect::<Vec<_>>(), vec!["g1".to_string()]);
    // The admin's own token survives the restore (not in the database, but kept in memory).
    assert!(f.hub.users_by_user_token.contains_key("admin"));
}

#[tokio::test]
async fn admin_download_cancel_and_close() {
    let mut f = fixture().await;
    f.login(2, ADMIN, "admin");
    f.add_game("g1", 1, loaded_game());

    assert_eq!(error_of(&f.hub.admin_download_game("admin", "nope").await.unwrap()), ErrorType::GameNotFound);
    let r = f.hub.admin_download_game("admin", "g1").await.unwrap();
    assert_eq!(r.contents.unwrap(), GameState::get_state_as_string(&f.hub.running_games_by_game_id["g1"].borrow().game));

    f.hub.scheduled_games_by_game_id.insert("s1".into(), ScheduledGame::default());
    let r = f.hub.admin_cancel_game("admin", "s1").await.unwrap();
    assert_eq!(r.contents.as_deref(), Some("Scheduled game cancelled"));
    assert!(f.hub.scheduled_games_by_game_id.is_empty());
    // Cancelling an unknown game also succeeds.
    assert!(f.hub.admin_cancel_game("admin", "s1").await.unwrap().success);

    // Closing an unknown game succeeds too. A game without users is just removed.
    assert_eq!(f.hub.admin_close_game("admin", "nope").await.unwrap().contents.as_deref(), Some("Game removed"));
    assert!(f.hub.admin_close_game("admin", "g1").await.unwrap().success);
    assert!(f.hub.running_games_by_game_id.is_empty());
}

#[tokio::test]
async fn get_admin_info_lists_database_users_and_cache_sizes() {
    let mut f = fixture().await;
    let db = f.hub.services.db.clone();
    let mut u = Fixture::user(0, "zed");
    u.last_login = t("2026-11-01 10:00:00+01:00");
    let zed = db.users_insert(&u).await.unwrap();
    db.users_insert(&Fixture::user(0, "amy")).await.unwrap();
    f.login(100, ADMIN, "admin"); // only in memory
    f.add_game("g1", 1, empty_game());

    let info = f.hub.get_admin_info("admin").await.unwrap().contents.unwrap();
    let users = info.users.unwrap();
    assert_eq!(users.iter().map(|u| u.username.as_str()).collect::<Vec<_>>(), vec!["zed", "amy"]);
    assert_eq!(users[0].id, zed);
    assert_eq!(users[0].player_name, "zed player");
    assert_eq!(users[0].email, "zed@example.com");
    assert_eq!(users[0].last_login, t("2026-11-01 10:00:00+01:00"));
    assert_eq!((info.users_by_user_token_count, info.connection_info_by_user_id_count, info.games_by_game_id_count), (1, 0, 1));
}

#[tokio::test]
async fn delete_user() {
    let mut f = fixture().await;
    let db = f.hub.services.db.clone();
    let alice = db.users_insert(&Fixture::user(0, "alice")).await.unwrap();
    let bob = db.users_insert(&Fixture::user(0, "bob")).await.unwrap();
    f.login(alice, "alice", "a");
    f.login(bob, "bob", "b");
    f.login(50, ADMIN, "admin");

    assert_eq!(error_of(&f.hub.delete_user("nobody", alice).await.unwrap()), ErrorType::InvalidUserNameOrPassword);
    assert_eq!(error_of(&f.hub.delete_user("b", alice).await.unwrap()), ErrorType::NotAdmin);

    let g = f.add_game("alices", alice, empty_game());
    f.add_game("bobs", bob, empty_game());
    f.hub.last_restored = start();
    f.hub.persist_game(&g).await.unwrap();

    // A user may delete themselves.
    let r = f.hub.delete_user("a", alice).await.unwrap();
    assert_eq!(r.contents.as_deref(), Some("User deleted"));
    assert_eq!(f.hub.running_games_by_game_id.keys().cloned().collect::<Vec<_>>(), vec!["bobs".to_string()]);
    assert!(f.persisted_games().await.is_empty());
    assert_eq!(db.users_all().await.unwrap().into_iter().map(|u| u.id).collect::<Vec<_>>(), vec![bob]);
    // C# parity: the caches still know the deleted user.
    assert!(f.hub.users_by_user_token.contains_key("a"));
    assert!(f.hub.users_by_id.contains_key(&alice));

    // The admin may delete anyone, even a user id that doesn't exist.
    assert!(f.hub.delete_user("admin", bob).await.unwrap().success);
    assert!(f.hub.delete_user("admin", 12345).await.unwrap().success);
    assert!(f.hub.running_games_by_game_id.is_empty());
    assert!(db.users_all().await.unwrap().is_empty());
}

#[tokio::test]
async fn request_nudge_bots_validates_and_succeeds() {
    let mut f = fixture().await;
    f.login(1, "alice", "tok");
    f.add_game("g1", 1, empty_game());
    assert_eq!(error_of(&f.hub.request_nudge_bots("nope", "g1").await.unwrap()), ErrorType::UserNotFound);
    assert_eq!(error_of(&f.hub.request_nudge_bots("tok", "g2").await.unwrap()), ErrorType::GameNotFound);
    let r = f.hub.request_nudge_bots("tok", "g1").await.unwrap();
    assert!(r.success);
    assert!(r.contents.is_some());
}
