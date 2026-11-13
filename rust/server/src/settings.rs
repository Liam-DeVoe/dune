//! The server's settings, from environment variables.
//!
//! | Variable | Default | Meaning |
//! |---|---|---|
//! | `TREACHERY_LISTEN` | `127.0.0.1:5000` | addresses to serve HTTP on, comma-separated. TLS is the reverse proxy's job |
//! | `TREACHERY_DATABASE` | `treachery.db` | the SQLite file |
//! | `TREACHERY_WEB_ROOT` | this repository's `web/dist` | the web client build |
//! | `TREACHERY_ASSETS_ROOT` | this repository's `web/assets` | art, fonts, scripts and CSS under `/art`, `/fonts`, `/js`, `/css`, `/docs` |
//! | `TREACHERY_SITE_URL` | `https://treachery.online` | the site's address, for links in mails |
//! | `TREACHERY_ADMIN` | none | the user name with admin rights |
//! | `TREACHERY_SMTP_HOST` | `smtp.strato.com` | the mail server (STARTTLS on port 25) |
//! | `TREACHERY_SMTP_USER`, `TREACHERY_SMTP_PASSWORD` | none | its login. Without a user, no mail is sent |
//! | `TREACHERY_STATS_FROM`, `TREACHERY_STATS_TO` | none | sender and recipient of the statistics mail at the end of each game. Without both, none is sent |
//! | `TREACHERY_DEV_SCENE` | `false` | `true` turns on `POST /dev/scene` (development only, see `dev.rs`) |
//! | `TREACHERY_DEV_FIXTURES` | this repository's `scripts/fixtures` | where the scenes' savegames are |
//! | `TREACHERY_DEV_CLOCK` | none | with the dev scene on, a time (RFC 3339) at which the server's clock stands still |

use std::path::PathBuf;

use super::time::{self, Time};

#[derive(Clone, Debug)]
pub struct Smtp {
    pub host: String,
    pub user: String,
    pub password: String,
}

#[derive(Clone, Debug)]
pub struct DevScene {
    pub fixtures: PathBuf,
    pub clock: Option<Time>,
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub listen: Vec<String>,
    pub database: PathBuf,
    pub web_root: PathBuf,
    pub assets_root: PathBuf,
    pub site_url: String,
    pub admin: Option<String>,
    pub smtp: Option<Smtp>,
    /// Sender and recipient of the end-of-game statistics mail.
    pub stats_mail: Option<(String, String)>,
    pub dev_scene: Option<DevScene>,
}

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

impl Settings {
    /// Reads the environment.
    pub fn from_env() -> Result<Settings, String> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// Settings from name-value pairs, for tests. Missing names take their defaults.
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Result<Settings, String> {
        Self::from_lookup(|k| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.to_string()))
    }

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Settings, String> {
        let get = |k: &str| get(k).filter(|v| !v.trim().is_empty());
        let smtp = get("TREACHERY_SMTP_USER").map(|user| Smtp {
            host: get("TREACHERY_SMTP_HOST").unwrap_or_else(|| "smtp.strato.com".into()),
            user,
            password: get("TREACHERY_SMTP_PASSWORD").unwrap_or_default(),
        });
        let dev_scene = match get("TREACHERY_DEV_SCENE") {
            Some(v) if v.eq_ignore_ascii_case("true") => Some(DevScene {
                fixtures: get("TREACHERY_DEV_FIXTURES").map(PathBuf::from).unwrap_or_else(|| repository().join("scripts/fixtures")),
                clock: match get("TREACHERY_DEV_CLOCK") {
                    Some(t) => Some(time::parse(&t).ok_or_else(|| format!("TREACHERY_DEV_CLOCK: can't read '{t}' as a time"))?),
                    None => None,
                },
            }),
            _ => None,
        };
        Ok(Settings {
            listen: get("TREACHERY_LISTEN")
                .unwrap_or_else(|| "127.0.0.1:5000".into())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            database: get("TREACHERY_DATABASE").unwrap_or_else(|| "treachery.db".into()).into(),
            web_root: get("TREACHERY_WEB_ROOT").map(PathBuf::from).unwrap_or_else(|| repository().join("web/dist")),
            assets_root: get("TREACHERY_ASSETS_ROOT").map(PathBuf::from).unwrap_or_else(|| repository().join("web/assets")),
            site_url: get("TREACHERY_SITE_URL").unwrap_or_else(|| "https://treachery.online".into()).trim_end_matches('/').to_string(),
            admin: get("TREACHERY_ADMIN"),
            smtp,
            stats_mail: get("TREACHERY_STATS_FROM").zip(get("TREACHERY_STATS_TO")),
            dev_scene,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_overrides() {
        let s = Settings::from_pairs(&[]).unwrap();
        assert_eq!(s.listen, ["127.0.0.1:5000"]);
        assert!(s.admin.is_none() && s.smtp.is_none() && s.stats_mail.is_none() && s.dev_scene.is_none());
        let s = Settings::from_pairs(&[
            ("TREACHERY_LISTEN", "0.0.0.0:80, 127.0.0.1:81"),
            ("TREACHERY_ADMIN", "root"),
            ("TREACHERY_SMTP_USER", "mailer"),
            ("TREACHERY_SITE_URL", "https://example.com/"),
            ("TREACHERY_DEV_SCENE", "true"),
            ("TREACHERY_DEV_CLOCK", "2027-01-01T00:00:00Z"),
        ])
        .unwrap();
        assert_eq!(s.listen, ["0.0.0.0:80", "127.0.0.1:81"]);
        assert_eq!(s.admin.as_deref(), Some("root"));
        assert_eq!(s.smtp.unwrap().host, "smtp.strato.com");
        assert_eq!(s.site_url, "https://example.com");
        assert!(s.dev_scene.unwrap().clock.is_some());
        assert!(Settings::from_pairs(&[("TREACHERY_DEV_SCENE", "true"), ("TREACHERY_DEV_CLOCK", "soon")]).is_err());
        // The clock is only read with the dev scene on.
        assert!(Settings::from_pairs(&[("TREACHERY_DEV_CLOCK", "soon")]).is_ok());
    }
}
