//! The database schema of the C# server: the EF Core model (`Server/Data`) after its last
//! migration (`Server/Migrations`).
//!
//! [`SCHEMA`] is the DDL that EF's `Database.Migrate()` leaves in a new database, copied from
//! `sqlite_master` of a database the C# server created. tests/golden/ef_schema.json holds that
//! `sqlite_master`, and tests/schema.rs checks that a database made by [`super::Db::open`] has
//! exactly the same one. [`MIGRATIONS`] is the `__EFMigrationsHistory`
//! content of such a database.
//!
//! On startup the Rust server:
//!
//! - creates this schema and history in an empty database, so the C# server can open it later
//!   and finds nothing to migrate;
//! - opens a database whose history ends with the last migration below as it is;
//! - refuses a database with an older or newer history. Migrating an old database is left to the
//!   C# server (run it once), because EF's SQLite migrations rebuild tables in ways not worth
//!   porting for a one-off.

/// The `MigrationId`s in `__EFMigrationsHistory`, oldest first.
pub const MIGRATIONS: &[&str] = &[
    "20240410063427_Initial",
    "20240806204209_PersistedGameAdded",
    "20240807064619_GameNameAdded",
    "20240809073843_CreationDateAdded",
    "20240816150934_UpdatedPersistedGame",
    "20240823053909_LastLoginDateAdded",
    "20240823144537_DateTimeToDateTimeOffset",
    "20250102090521_ScheduledGamesAdded",
    "20250104103235_CreatorPlayerNameColumnInScheduledGame",
    "20250107233339_RemovedRedundantBotsArePaused",
    "20250201222850_NewFieldInPersistedGame",
    "20260729104158_NullabilityUpdates",
    "20260921083521_UserTokensAdded",
];

/// `ProductVersion` written next to each migration: the EF Core version of the C# server
/// (`Microsoft.EntityFrameworkCore.Sqlite` 10.0.12 in Server/Treachery.Server.csproj).
pub const PRODUCT_VERSION: &str = "10.0.12";

/// One statement per entry, in `sqlite_master` order. `sqlite_sequence` is created by SQLite
/// itself with the first AUTOINCREMENT table.
pub const SCHEMA: &[&str] = &[
    // EF writes this one statement with CRLF line ends, and sqlite_master keeps them.
    "CREATE TABLE \"__EFMigrationsLock\" (\r\n    \"Id\" INTEGER NOT NULL CONSTRAINT \"PK___EFMigrationsLock\" PRIMARY KEY,\r\n    \"Timestamp\" TEXT NOT NULL\r\n)",
    r#"CREATE TABLE "__EFMigrationsHistory" (
    "MigrationId" TEXT NOT NULL CONSTRAINT "PK___EFMigrationsHistory" PRIMARY KEY,
    "ProductVersion" TEXT NOT NULL
)"#,
    r#"CREATE TABLE "Users" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_Users" PRIMARY KEY AUTOINCREMENT,
    "Email" TEXT NOT NULL,
    "HashedPassword" TEXT NOT NULL,
    "LastLogin" TEXT NOT NULL,
    "Name" TEXT NOT NULL,
    "PasswordResetToken" TEXT NULL,
    "PasswordResetTokenCreated" TEXT NOT NULL,
    "PlayerName" TEXT NOT NULL
)"#,
    r#"CREATE TABLE "ScheduledGames" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_ScheduledGames" PRIMARY KEY AUTOINCREMENT,
    "AllowedFactionsInPlay" TEXT NOT NULL,
    "AsyncPlay" INTEGER NOT NULL,
    "CreatorPlayerName" TEXT NOT NULL,
    "CreatorUserId" INTEGER NOT NULL,
    "DateTime" TEXT NOT NULL,
    "GameId" TEXT NOT NULL,
    "MaximumTurns" INTEGER NOT NULL,
    "NumberOfPlayers" INTEGER NOT NULL,
    "Ruleset" INTEGER NULL,
    "SubscribedUsers" TEXT NOT NULL
)"#,
    r#"CREATE TABLE "PersistedGames" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_PersistedGames" PRIMARY KEY AUTOINCREMENT,
    "CreationDate" TEXT NOT NULL,
    "CreatorUserId" INTEGER NOT NULL,
    "GameId" TEXT NULL,
    "GameName" TEXT NULL,
    "GameParticipation" TEXT NOT NULL,
    "GameState" TEXT NOT NULL,
    "HashedPassword" TEXT NULL,
    "LastAction" TEXT NOT NULL,
    "LastAsyncPlayMessageSent" TEXT NOT NULL,
    "ObserversRequirePassword" INTEGER NOT NULL,
    "StatisticsSent" INTEGER NOT NULL
)"#,
    r#"CREATE TABLE "ArchivedGames" (
    "Id" INTEGER NOT NULL CONSTRAINT "PK_ArchivedGames" PRIMARY KEY AUTOINCREMENT,
    "CreatorUserId" INTEGER NOT NULL,
    "GameName" TEXT NOT NULL,
    "GameParticipation" TEXT NOT NULL,
    "GameState" TEXT NOT NULL
)"#,
    r#"CREATE INDEX "IX_PersistedGames_GameId" ON "PersistedGames" ("GameId")"#,
    r#"CREATE TABLE "UserTokens" (
    "Token" TEXT NOT NULL CONSTRAINT "PK_UserTokens" PRIMARY KEY,
    "UserId" INTEGER NOT NULL,
    "Created" TEXT NOT NULL
)"#,
];
