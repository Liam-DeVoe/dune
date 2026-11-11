//! Port of `Server/Data/User.cs` and the hub's queries on `db.Users`.

use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use treachery_engine::date_time_offset::DateTimeOffset;

use super::{date_time, get_date_time, Db};
use crate::dotnet_string;

/// The SQL EF Core 10 generates for the hub's LINQ, captured from EF's log by the H1 probe
/// (`server/dotnet-probe`, mode `ef`) and compared in `tests/auth_dotnet.rs`. EF's named parameters are numbered
/// (`?1`, `?2`) in order of first appearance, since sqlx binds only those; `?1`, used
/// twice, is `?1` both times.
pub mod sql {
    pub const USERS_ANY_WITH_NAME: &str = r#"SELECT EXISTS (
    SELECT 1
    FROM "Users" AS "u"
    WHERE "u"."Name" = ?1)"#;
    pub const USERS_FIRST_BY_NAME_AND_PASSWORD: &str = r#"SELECT "u"."Id", "u"."Email", "u"."HashedPassword", "u"."LastLogin", "u"."Name", "u"."PasswordResetToken", "u"."PasswordResetTokenCreated", "u"."PlayerName"
FROM "Users" AS "u"
WHERE "u"."Name" = ?1 AND "u"."HashedPassword" = ?2
LIMIT 1"#;
    pub const USERS_FIRST_FOR_LOGIN: &str = r#"SELECT "u"."Id", "u"."Email", "u"."HashedPassword", "u"."LastLogin", "u"."Name", "u"."PasswordResetToken", "u"."PasswordResetTokenCreated", "u"."PlayerName"
FROM "Users" AS "u"
WHERE lower(trim("u"."Name")) = ?1 AND "u"."HashedPassword" = ?2
LIMIT 1"#;
    pub const USERS_WHERE_EMAIL_OR_NAME: &str = r#"SELECT "u"."Id", "u"."Email", "u"."HashedPassword", "u"."LastLogin", "u"."Name", "u"."PasswordResetToken", "u"."PasswordResetTokenCreated", "u"."PlayerName"
FROM "Users" AS "u"
WHERE lower(trim("u"."Email")) = ?1 OR lower(trim("u"."Name")) = ?1"#;
    pub const USERS_FIRST_BY_NAME: &str = r#"SELECT "u"."Id", "u"."Email", "u"."HashedPassword", "u"."LastLogin", "u"."Name", "u"."PasswordResetToken", "u"."PasswordResetTokenCreated", "u"."PlayerName"
FROM "Users" AS "u"
WHERE lower(trim("u"."Name")) = ?1
LIMIT 1"#;
    /// `context.Users.FindAsync(userId)` on a fresh context (GameHub_GameEvents.cs:271).
    pub const USERS_FIND: &str = r#"SELECT "u"."Id", "u"."Email", "u"."HashedPassword", "u"."LastLogin", "u"."Name", "u"."PasswordResetToken", "u"."PasswordResetTokenCreated", "u"."PlayerName"
FROM "Users" AS "u"
WHERE "u"."Id" = ?1
LIMIT 1"#;
}

/// One `Users` row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub hashed_password: String,
    pub password_reset_token: Option<String>,
    pub password_reset_token_created: DateTimeOffset,
    pub player_name: String,
    pub last_login: DateTimeOffset,
}

impl User {
    /// The non-key columns in EF's property order, the order of its INSERT and UPDATE lists.
    pub const COLUMNS: &'static [&'static str] =
        &["Email", "HashedPassword", "LastLogin", "Name", "PasswordResetToken", "PasswordResetTokenCreated", "PlayerName"];

    /// The columns whose values differ between `original` and `self`, as EF's change tracker
    /// sees them. Its `DateTimeOffset` comparer is exact: the same instant at another offset
    /// counts as a change (checked with the H1 probe, `ef` mode).
    pub fn changed_columns(&self, original: &User) -> Vec<&'static str> {
        let exact = |a: &DateTimeOffset, b: &DateTimeOffset| a.ticks == b.ticks && a.offset_minutes == b.offset_minutes;
        let mut columns = Vec::new();
        if self.email != original.email {
            columns.push("Email");
        }
        if self.hashed_password != original.hashed_password {
            columns.push("HashedPassword");
        }
        if !exact(&self.last_login, &original.last_login) {
            columns.push("LastLogin");
        }
        if self.name != original.name {
            columns.push("Name");
        }
        if self.password_reset_token != original.password_reset_token {
            columns.push("PasswordResetToken");
        }
        if !exact(&self.password_reset_token_created, &original.password_reset_token_created) {
            columns.push("PasswordResetTokenCreated");
        }
        if self.player_name != original.player_name {
            columns.push("PlayerName");
        }
        columns
    }

    fn bind_column<'q>(
        &self,
        query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
        column: &str,
    ) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
        match column {
            "Email" => query.bind(self.email.clone()),
            "HashedPassword" => query.bind(self.hashed_password.clone()),
            "LastLogin" => query.bind(date_time::format(self.last_login)),
            "Name" => query.bind(self.name.clone()),
            "PasswordResetToken" => query.bind(self.password_reset_token.clone()),
            "PasswordResetTokenCreated" => query.bind(date_time::format(self.password_reset_token_created)),
            "PlayerName" => query.bind(self.player_name.clone()),
            _ => unreachable!("not a Users column: {column}"),
        }
    }

    pub fn from_row(row: &SqliteRow) -> Result<User, sqlx::Error> {
        Ok(User {
            id: row.try_get("Id")?,
            name: row.try_get("Name")?,
            email: row.try_get("Email")?,
            hashed_password: row.try_get("HashedPassword")?,
            password_reset_token: row.try_get("PasswordResetToken")?,
            password_reset_token_created: get_date_time(row, "PasswordResetTokenCreated")?,
            player_name: row.try_get("PlayerName")?,
            last_login: get_date_time(row, "LastLogin")?,
        })
    }
}

impl Db {
    /// `context.Users.AsNoTracking()` (GameHub_Other.cs:332) and the `Users.Select` of
    /// `GetAdminInfo` (GameHub_Other.cs:504): every row, in SQLite's order (no `ORDER BY`).
    pub async fn users_all(&self) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query(r#"SELECT * FROM "Users""#).fetch_all(&self.pool).await?.iter().map(User::from_row).collect()
    }

    /// `db.Add(user); await db.SaveChangesAsync();` (GameHub_Authentication.cs:39-42). Returns the
    /// new `Id`.
    pub async fn users_insert(&self, user: &User) -> Result<i32, sqlx::Error> {
        let result = sqlx::query(
            r#"INSERT INTO "Users" ("Email", "HashedPassword", "LastLogin", "Name", "PasswordResetToken", "PasswordResetTokenCreated", "PlayerName") VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&user.email)
        .bind(&user.hashed_password)
        .bind(date_time::format(user.last_login))
        .bind(&user.name)
        .bind(&user.password_reset_token)
        .bind(date_time::format(user.password_reset_token_created))
        .bind(&user.player_name)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid() as i32)
    }

    /// `db.Users.Any(x => Equals(x.Name, cleanedUsername))` (GameHub_Authentication.cs:34).
    pub async fn users_any_with_name(&self, name: &str) -> Result<bool, sqlx::Error> {
        let row = sqlx::query(sql::USERS_ANY_WITH_NAME).bind(name).fetch_one(&self.pool).await?;
        Ok(row.try_get::<i64, _>(0)? != 0)
    }

    /// `db.Users.FirstOrDefaultAsync(x => Equals(x.Name, cleanedUsername) && x.HashedPassword == hashedPassword)`
    /// (GameHub_Authentication.cs:44).
    pub async fn users_first_by_name_and_password(&self, name: &str, hashed_password: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query(sql::USERS_FIRST_BY_NAME_AND_PASSWORD).bind(name).bind(hashed_password).fetch_optional(&self.pool).await?.as_ref().map(User::from_row).transpose()
    }

    /// `db.Users.FirstOrDefaultAsync(x => x.Name.Trim().ToLower().Equals(userName.Trim().ToLower()) && x.HashedPassword == hashedPassword)`
    /// (GameHub_Authentication.cs:99). EF evaluates the right-hand side in .NET (a parameter) and
    /// the left in SQLite (`lower(trim("Name"))`); see the plan, "Hazards". `user_name` is the
    /// raw argument: this applies the .NET `Trim().ToLower()`.
    pub async fn users_first_for_login(&self, user_name: &str, hashed_password: &str) -> Result<Option<User>, sqlx::Error> {
        let to_lower = dotnet_string::to_lower(dotnet_string::trim(user_name));
        sqlx::query(sql::USERS_FIRST_FOR_LOGIN).bind(to_lower).bind(hashed_password).fetch_optional(&self.pool).await?.as_ref().map(User::from_row).transpose()
    }

    /// `db.Users.Where(x => x.Email.Trim().ToLower().Equals(s) || x.Name.Trim().ToLower().Equals(s)).ToList()`
    /// (GameHub_Authentication.cs:117), with `s = usernameOrEmail.Trim().ToLower()` in .NET.
    pub async fn users_where_email_or_name(&self, username_or_email: &str) -> Result<Vec<User>, sqlx::Error> {
        let to_lower = dotnet_string::to_lower(dotnet_string::trim(username_or_email));
        sqlx::query(sql::USERS_WHERE_EMAIL_OR_NAME).bind(to_lower).fetch_all(&self.pool).await?.iter().map(User::from_row).collect()
    }

    /// `db.Users.FirstOrDefaultAsync(x => x.Name.Trim().ToLower().Equals(userName.Trim().ToLower()))`
    /// (GameHub_Authentication.cs:169).
    pub async fn users_first_by_name(&self, user_name: &str) -> Result<Option<User>, sqlx::Error> {
        let to_lower = dotnet_string::to_lower(dotnet_string::trim(user_name));
        sqlx::query(sql::USERS_FIRST_BY_NAME).bind(to_lower).fetch_optional(&self.pool).await?.as_ref().map(User::from_row).transpose()
    }

    /// `await context.Users.FindAsync(userId)` (GameHub_GameEvents.cs:271).
    /// The context is new (`await using var context = GetDbContext()`), so nothing is tracked and
    /// `FindAsync` always queries: EF's `WHERE "u"."Id" = @__p_0 LIMIT 1`.
    pub async fn users_find(&self, id: i32) -> Result<Option<User>, sqlx::Error> {
        sqlx::query(sql::USERS_FIND).bind(id).fetch_optional(&self.pool).await?.as_ref().map(User::from_row).transpose()
    }

    /// `SaveChangesAsync` after changing a tracked `User`: EF writes only the columns whose value
    /// changed (`DateTimeOffset` compared exactly, offset included; see [`User::changed_columns`])
    /// and none at all when nothing changed. Returns `false` when no row has the `Id`, where EF
    /// throws `DbUpdateConcurrencyException`.
    pub async fn users_update_columns(&self, user: &User, columns: &[&str]) -> Result<bool, sqlx::Error> {
        if columns.is_empty() {
            return Ok(true);
        }
        // EF lists the modified columns in the model's (alphabetical) property order.
        let ordered: Vec<&str> = User::COLUMNS.iter().copied().filter(|c| columns.contains(c)).collect();
        assert_eq!(ordered.len(), columns.len(), "unknown column in {columns:?}");
        let set: Vec<String> = ordered.iter().map(|c| format!("\"{c}\" = ?")).collect();
        let text = format!("UPDATE \"Users\" SET {}\nWHERE \"Id\" = ?", set.join(", "));
        let mut query = sqlx::query(&text);
        for c in &ordered {
            query = user.bind_column(query, c);
        }
        Ok(query.bind(user.id).execute(&self.pool).await?.rows_affected() == 1)
    }

    /// `db.Users.Update(user); await db.SaveChangesAsync();` (GameHub_Authentication.cs:213):
    /// every column, from the (possibly stale) cached object. Returns `false` when no row has
    /// the `Id` (EF throws `DbUpdateConcurrencyException`).
    pub async fn users_update_all(&self, user: &User) -> Result<bool, sqlx::Error> {
        self.users_update_columns(user, User::COLUMNS).await
    }

    /// `db.Users.Where(u => u.Id == userId).ExecuteDeleteAsync()` (GameHub_Other.cs:492).
    pub async fn users_delete(&self, id: i32) -> Result<(), sqlx::Error> {
        sqlx::query(r#"DELETE FROM "Users" WHERE "Id" = ?"#).bind(id).execute(&self.pool).await?;
        Ok(())
    }
}
