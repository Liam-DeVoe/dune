//! How messages, game objects and enums are described as text, plus the skin's art references
//! and HTML popups.
//!
//! The server renders validation errors and mails with it, and the view layer log and status
//! text. Output doesn't depend on the locale. See `rust/docs/skin.md` for the tests.
//!
//! ```
//! use treachery_engine::{express, enumerations::{Concept, Faction}};
//! let skin = treachery_skin::default_skin();
//! let m = express!(Faction::Atreides, " pays ", 3, " ", Concept::Resource);
//! assert_eq!(skin.render(&m), "Atreides pays 3 Spice");
//! ```

pub mod default_skin;
pub mod point;
pub mod faction_manual;
pub mod skin;
pub mod skin_json;
pub mod text;

pub use skin::{Skin, SkinDictionary, SkinKey};

/// The default skin: one shared instance per process.
pub fn default_skin() -> &'static Skin {
    static DEFAULT: std::sync::OnceLock<Skin> = std::sync::OnceLock::new();
    DEFAULT.get_or_init(default_skin::build)
}
