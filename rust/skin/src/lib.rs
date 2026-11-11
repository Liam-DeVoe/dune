//! Port of `Shared/Model/Skin.cs` and `Shared/DefaultSkin.cs`: how messages, game objects and
//! enums are described as text, plus the skin's art references and HTML popups.
//!
//! The server renders validation errors and mails with it, and the view layer log and status
//! text. Every output is byte-identical to C#'s under the invariant culture; see
//! `rust/docs/skin.md` for the differential tests.
//!
//! ```
//! use treachery_engine::{express, enumerations::{Concept, Faction}};
//! let skin = treachery_skin::default_skin();
//! let m = express!(Faction::Atreides, " pays ", 3, " ", Concept::Resource);
//! assert_eq!(skin.message_to_string(&m), "Atreides pays 3 Spice");
//! ```

pub mod default_skin;
pub mod faction_manual;
pub mod skin;
pub mod skin_json;
pub mod text;

pub use skin::{Skin, SkinDictionary, SkinKey};

/// `DefaultSkin.Default`: one shared instance per process, like the C# static.
pub fn default_skin() -> &'static Skin {
    static DEFAULT: std::sync::OnceLock<Skin> = std::sync::OnceLock::new();
    DEFAULT.get_or_init(default_skin::build)
}
