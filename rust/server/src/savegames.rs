//! Converting savegames of game version 190, in the old JSON format, to version 191 (rewrite-plan.md,
//! "Wire format"). The server converts uploaded savegames when it loads them
//! (`treachery_legacy::load_any`), and stored games once, in the database migration to schema
//! version 3 (`db::migrate_to_v3`).

use treachery_engine::events::Event;
use treachery_engine::game::{Game, GameState};

/// Why a stored game was left as it is.
#[derive(Debug, PartialEq, Eq)]
pub enum NotConverted {
    /// It is already in the engine's format.
    Current,
    /// The old reader refused it, or it is of a version the engine no longer plays. It stays in
    /// the database, and the server leaves it out at startup as before.
    Unreadable(String),
    /// The game in the new format would not be the same game: event `index` reads back as
    /// another event.
    Differs { index: usize },
}

/// The savegame `text` of version 190 in the engine's format. The events of the new text must
/// read back as the old ones, except that a pass carries none of its other fields, as
/// `Game::play` makes it.
pub fn convert(text: &str) -> Result<String, NotConverted> {
    if !treachery_legacy::is_legacy_savegame(text) {
        return Err(NotConverted::Current);
    }
    let state = treachery_legacy::load_savegame(text).map_err(|e| NotConverted::Unreadable(e.message))?;
    Game::check_version(state.version).map_err(NotConverted::Unreadable)?;
    let new = serde_json::to_string(&state).map_err(|e| NotConverted::Unreadable(e.to_string()))?;
    let back = GameState::load(&new).map_err(NotConverted::Unreadable)?;
    for (index, (old, new)) in state.events.iter().zip(&back.events).enumerate() {
        let expected: Event = old.event.clone().normalized();
        if new.event != expected || new.by != old.by || new.at != old.at {
            return Err(NotConverted::Differs { index });
        }
    }
    if back.events.len() != state.events.len() || back.version != state.version {
        return Err(NotConverted::Differs { index: back.events.len().min(state.events.len()) });
    }
    Ok(new)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!("{}/../../scripts/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    /// Every fixture (savegames of version 190) converts, and plays as the same game.
    #[test]
    fn fixtures_convert_to_the_same_game() {
        let dir = format!("{}/../../scripts/fixtures", env!("CARGO_MANIFEST_DIR"));
        let mut n = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            let old = fixture(&name);
            let new = convert(&old).unwrap_or_else(|e| panic!("{name}: {e:?}"));
            assert!(new.starts_with(r#"{"version":191,"events":["#), "{name}");
            let seats = Default::default();
            let a = Game::load(&treachery_legacy::load_savegame(&old).unwrap(), &seats, false, false).unwrap();
            let b = Game::load(&GameState::load(&new).unwrap(), &seats, false, false).unwrap();
            let log = |g: &Game| g.reports().iter().flat_map(|r| r.messages.iter().map(|m| treachery_skin::default_skin().render(m))).collect::<Vec<_>>();
            assert_eq!(log(&a), log(&b), "{name}");
            assert_eq!(convert(&new), Err(NotConverted::Current));
            n += 1;
        }
        assert!(n > 5);
    }

    #[test]
    fn old_versions_stay() {
        assert!(matches!(convert(r#"{"Version":150,"_events":[]}"#), Err(NotConverted::Unreadable(_))));
        assert!(matches!(convert(r#"{"Version":190,"_events":[{"$type":"Nope"}]}"#), Err(NotConverted::Unreadable(_))));
    }
}
