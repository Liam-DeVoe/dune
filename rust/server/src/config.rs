//! The part of ASP.NET Core's `IConfiguration` the C# server reads.
//!
//! Sources, later ones overriding earlier ones, as `Host.CreateDefaultBuilder` sets them up:
//! `appsettings.json`, `appsettings.{Environment}.json`, then environment variables
//! (`ConnectionStrings__TreacheryDatabase` is the key `ConnectionStrings:TreacheryDatabase`).
//! The environment is `ASPNETCORE_ENVIRONMENT`, else `DOTNET_ENVIRONMENT`, else `Production`.
//! User secrets (development only in C#) are not read. Keys are case-insensitive, as in .NET.
//!
//! Keys used by the server: `ConnectionStrings:TreacheryDatabase`, `GameEndEmailUsername`,
//! `GameEndEmailPassword`, `GameEndEmailFrom`, `GameEndEmailTo`, `GameAdminUsername`, and
//! `urls` (`ASPNETCORE_URLS`) for the listen addresses (`http://` only: TLS belongs to the
//! reverse proxy). Rust only: `SmtpHost` (default `smtp.strato.com`, the C# value), `WebRoot`
//! (the web client build, default `web/dist`), `SharedAssetsRoot` (the C# client's `wwwroot`),
//! `DevScene` (`true` turns on `POST /dev/scene`), `DevSceneFixtures` (default
//! `scripts/fixtures`) and `DevSceneClock` (a fixed time for the server's clock, read only with
//! `DevScene` on; `dev.rs`). The defaults of the paths are this repository's directories.

use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct Config {
    /// Flattened keys (`Section:Key`), lowercased, to their values.
    values: BTreeMap<String, String>,
}

impl Config {
    /// An empty configuration with the given entries (tests, the equivalence harness).
    pub fn from_pairs<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        let mut c = Config::default();
        for (k, v) in pairs {
            c.set(k, v);
        }
        c
    }

    /// Reads the sources listed in the module docs from `dir`.
    pub fn load(dir: &Path) -> Self {
        let mut c = Config::default();
        let environment = std::env::var("ASPNETCORE_ENVIRONMENT")
            .or_else(|_| std::env::var("DOTNET_ENVIRONMENT"))
            .unwrap_or_else(|_| "Production".to_string());
        for file in ["appsettings.json".to_string(), format!("appsettings.{environment}.json")] {
            if let Ok(text) = std::fs::read_to_string(dir.join(file)) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                    c.add_json("", &v);
                }
            }
        }
        for (k, v) in std::env::vars() {
            let k = k.strip_prefix("ASPNETCORE_").unwrap_or(&k).replace("__", ":");
            c.set(&k, &v);
        }
        c
    }

    pub fn load_from_current_dir() -> Self {
        Self::load(&std::env::current_dir().unwrap_or_default())
    }

    fn add_json(&mut self, prefix: &str, v: &serde_json::Value) {
        match v {
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}:{k}") };
                    self.add_json(&key, v);
                }
            }
            serde_json::Value::Array(items) => {
                for (i, v) in items.iter().enumerate() {
                    self.add_json(&format!("{prefix}:{i}"), v);
                }
            }
            serde_json::Value::Null => self.set(prefix, ""),
            serde_json::Value::String(s) => self.set(prefix, s),
            other => self.set(prefix, &other.to_string()),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.values.insert(key.to_ascii_lowercase(), value.to_string());
    }

    /// `Configuration[key]`: `None` when the key is absent (C# null).
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(&key.to_ascii_lowercase()).map(String::as_str)
    }

    /// `Configuration.GetConnectionString(name)`.
    pub fn connection_string(&self, name: &str) -> Option<&str> {
        self.get(&format!("ConnectionStrings:{name}"))
    }

    /// The SQLite file named by `Data Source=...` in `ConnectionStrings:TreacheryDatabase`.
    pub fn database_path(&self) -> Option<String> {
        let cs = self.connection_string("TreacheryDatabase")?;
        cs.split(';').find_map(|part| {
            let (k, v) = part.split_once('=')?;
            let k = k.trim().to_ascii_lowercase();
            (k == "data source" || k == "datasource" || k == "filename").then(|| v.trim().to_string())
        })
    }

    /// The listen addresses (`urls`), default `http://localhost:5000` as in ASP.NET Core.
    pub fn urls(&self) -> Vec<String> {
        self.get("urls")
            .unwrap_or("http://localhost:5000")
            .split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}
