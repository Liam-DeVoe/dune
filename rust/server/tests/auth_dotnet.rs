//! Package H1's string and query helpers against .NET, from goldens written by the H1 probe
//! (`server/dotnet-probe`). Regenerate them with
//! `dotnet run --project server/dotnet-probe -c Release -- chars > server/tests/golden/dotnet_chars.txt`
//! (and `ef > server/tests/golden/ef_auth.jsonl`).
//!
//! - `dotnet_chars.txt`: `string.ToLower()` (invariant culture, ICU) for every code point it
//!   changes, `char.IsWhiteSpace` for every UTF-16 unit, and a few whole strings.
//! - `ef_auth.jsonl`: the SQL EF Core generates for the authentication queries, and which rows
//!   each matches on non-ASCII names and e-mail addresses.

mod h1_probe;

use serde_json::Value;
use treachery_engine::date_time_offset::DateTimeOffset;
use treachery_server::db::user::{sql, User};
use treachery_server::db::Db;
use treachery_server::dotnet_string;

fn hex_char(s: &str) -> char {
    char::from_u32(u32::from_str_radix(s, 16).unwrap()).unwrap()
}

#[test]
fn to_lower_and_white_space_match_dotnet() {
    let text = std::fs::read_to_string(h1_probe::golden_dir().join("dotnet_chars.txt")).unwrap();
    let mut lower = std::collections::BTreeMap::new();
    let mut white_space = std::collections::BTreeSet::new();
    let mut strings = 0;
    for line in text.lines() {
        let (kind, rest) = line.split_once(' ').unwrap();
        match kind {
            "lower" => {
                let (from, to) = rest.split_once(' ').unwrap();
                let to: String = to.split(',').map(hex_char).collect();
                lower.insert(hex_char(from), to);
            }
            "ws" => {
                white_space.insert(u32::from_str_radix(rest, 16).unwrap());
            }
            "str" => {
                let v: Vec<String> = serde_json::from_str(rest).unwrap();
                assert_eq!(dotnet_string::to_lower(&v[0]), v[1], "ToLower({:?})", v[0]);
                assert_eq!(dotnet_string::trim(&v[0]), v[2], "Trim({:?})", v[0]);
                assert_eq!(dotnet_string::length(&v[0]).to_string(), v[3], "Length({:?})", v[0]);
                strings += 1;
            }
            _ => panic!("unknown line {line}"),
        }
    }
    assert!(lower.len() > 1000 && strings > 5);

    let mut differences = Vec::new();
    for c in (0..=0x10FFFFu32).filter_map(char::from_u32) {
        let want = lower.get(&c).cloned().unwrap_or_else(|| c.to_string());
        let got = dotnet_string::to_lower(&c.to_string());
        // Upper-case letters added in Unicode 16 and 17 (Cyrillic Extended-C, Latin Extended-D,
        // Garay, Beria Erfe): the probe's ICU predates them and leaves them unchanged. The
        // deployed server's ICU decides; see rust/docs/unreachable.md.
        let newer_unicode = matches!(c as u32, 0x1C89 | 0xA7CB..=0xA7DC | 0x10D50..=0x10D65 | 0x16EA0..=0x16EB8);
        if got != want && !(newer_unicode && want == c.to_string()) {
            differences.push(format!("U+{:04X}: .NET {:?}, Rust {:?}", c as u32, want, got));
        }
    }
    assert!(differences.is_empty(), "{} code points differ:\n{}", differences.len(), differences[..differences.len().min(100)].join("\n"));

    for unit in 0..=0xFFFFu32 {
        let rust = char::from_u32(unit).is_some_and(char::is_whitespace);
        assert_eq!(rust, white_space.contains(&unit), "char.IsWhiteSpace(U+{unit:04X})");
    }
}

fn ef_golden() -> Vec<Value> {
    let text = std::fs::read_to_string(h1_probe::golden_dir().join("ef_auth.jsonl")).unwrap();
    text.lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

/// EF's log indents continuation lines; compare with runs of white space collapsed.
/// EF names its parameters (`@ToLower`); sqlx's SQLite driver only binds `?N`, so the Rust SQL
/// numbers them in order of first appearance.
fn normalize_sql(s: &str) -> String {
    let mut names: Vec<String> = Vec::new();
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '@' {
            let mut name = String::new();
            while let Some(&n) = chars.peek().filter(|n| n.is_alphanumeric() || **n == '_') {
                name.push(n);
                chars.next();
            }
            let i = names.iter().position(|x| *x == name).unwrap_or_else(|| {
                names.push(name);
                names.len() - 1
            });
            out.push_str(&format!("?{}", i + 1));
        } else {
            out.push(c);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn sql_matches_ef() {
    let golden = ef_golden();
    let ef_sql = |label: &str| -> String {
        let v = golden.iter().find(|v| v["label"] == label).unwrap_or_else(|| panic!("no {label} in golden"));
        let s = v["sql"].as_str().unwrap();
        // Drop the "Executed DbCommand ... [Parameters=...]" header line.
        normalize_sql(s.split_once('\n').map_or(s, |(_, rest)| rest))
    };
    assert_eq!(normalize_sql(sql::USERS_ANY_WITH_NAME), ef_sql("users_any_with_name"));
    assert_eq!(normalize_sql(sql::USERS_FIRST_BY_NAME_AND_PASSWORD), ef_sql("users_first_by_name_and_password"));
    assert_eq!(normalize_sql(sql::USERS_FIRST_FOR_LOGIN), ef_sql("users_first_for_login"));
    assert_eq!(normalize_sql(sql::USERS_WHERE_EMAIL_OR_NAME), ef_sql("users_where_email_or_name"));
    assert_eq!(normalize_sql(sql::USERS_FIRST_BY_NAME), ef_sql("users_first_by_name"));

    // Tracked updates write the changed columns, in model order; an exact DateTimeOffset
    // comparison (same instant, other offset is a change); no column for an unchanged value.
    let set_clause = |label: &str| {
        let s = ef_sql(label);
        let start = s.find(" SET ").unwrap() + 5;
        let end = s.find(" WHERE").unwrap();
        s[start..end].split(", ").map(|c| c.split(' ').next().unwrap().trim_matches('"').to_string()).collect::<Vec<_>>()
    };
    let base = User { id: 1, name: "abc".into(), hashed_password: "h".into(), last_login: DateTimeOffset::new(10 * 600_000_000, -300), ..User::default() };
    let mut u = base.clone();
    u.last_login = DateTimeOffset::new(u.last_login.ticks + 600_000_000, -300);
    assert_eq!(u.changed_columns(&base), set_clause("tracked_last_login"));
    let mut v = u.clone();
    v.last_login = DateTimeOffset::new(u.last_login.ticks + 300 * 600_000_000, 0);
    assert_eq!(v.last_login, u.last_login, "same instant");
    assert_eq!(v.changed_columns(&u), set_clause("tracked_same_instant_other_offset"));
    let mut w = v.clone();
    w.password_reset_token = Some("T".into());
    w.password_reset_token_created = base.last_login;
    assert_eq!(w.changed_columns(&v), set_clause("tracked_reset"));
    let mut x = w.clone();
    x.hashed_password = "h".into();
    x.password_reset_token = Some(String::new());
    x.password_reset_token_created = DateTimeOffset::default();
    assert_eq!(x.changed_columns(&w), set_clause("tracked_set_password_same_hash"));
    assert_eq!(User::COLUMNS.to_vec(), set_clause("update_all"));
}

#[test]
fn queries_match_ef_on_non_ascii_data() {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let golden = ef_golden();
        let db = Db::open_in_memory().await.unwrap();
        for v in golden.iter().filter(|v| v["label"] == "data") {
            let user = User {
                name: v["name"].as_str().unwrap().into(),
                email: v["email"].as_str().unwrap().into(),
                hashed_password: "h".into(),
                player_name: "p".into(),
                ..User::default()
            };
            assert_eq!(db.users_insert(&user).await.unwrap() as i64, v["id"].as_i64().unwrap());
        }
        // What the probe did between inserting and matching: `Update` of user 2, delete of 3.
        let mut two = User { id: 2, name: "x".into(), email: "e".into(), hashed_password: "h".into(), player_name: "p".into(), ..User::default() };
        assert!(db.users_update_all(&two).await.unwrap());
        two.id = 999;
        assert!(!db.users_update_all(&two).await.unwrap(), "EF throws DbUpdateConcurrencyException");
        sqlx::query(r#"DELETE FROM "Users" WHERE "Id" = 3"#).execute(&db.pool).await.unwrap();

        let ids = |users: Vec<User>| users.into_iter().map(|u| u.id as i64).collect::<Vec<_>>();
        let mut checked = 0;
        for v in golden.iter().filter(|v| v["label"] == "match") {
            let input = v["input"].as_str().unwrap();
            let want = |key: &str| v[key].as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect::<Vec<_>>();
            let login = db.users_first_for_login(input, "h").await.unwrap();
            assert_eq!(login.map(|u| u.id as i64).into_iter().collect::<Vec<_>>(), want("login").into_iter().take(1).collect::<Vec<_>>(), "login {input:?}");
            assert_eq!(ids(db.users_where_email_or_name(input).await.unwrap()), want("reset"), "reset {input:?}");
            let by_name = db.users_first_by_name(input).await.unwrap();
            assert_eq!(by_name.map(|u| u.id as i64), want("login").first().copied(), "set password {input:?}");
            let exact = db.users_any_with_name(input).await.unwrap();
            assert_eq!(exact, !want("exact").is_empty(), "exact {input:?}");
            checked += 1;
        }
        assert!(checked > 10);
    });
}
