//! How a view names things: the skin's `Describe`.
//!
//! The text of every log line and prompt is rendered on the server, with the game's skin
//! (`rust/skin`). A custom skin is a `treachery_skin::Skin` loaded with `Skin::load`, which the
//! server puts in the replica.

use treachery_engine::messages::part::Part as P;
use treachery_engine::game::Game;
use treachery_skin::Skin;

pub trait Describer {
    /// `Skin.Describe(object)`: the skin's description of a value.
    fn describe(&self, el: &P) -> String;

    /// The game text is rendered for: a leader's popover shows the skill it has in it
    /// (`Game::skill`).
    fn game(&self) -> Option<&Game> {
        None
    }

    /// The skin itself, for texts that are more than `Describe` (the faction sheets). `None` for
    /// a describer without one.
    fn skin(&self) -> Option<&Skin> {
        None
    }
}

impl Describer for Skin {
    fn describe(&self, el: &P) -> String {
        Skin::describe(self, el)
    }
    fn skin(&self) -> Option<&Skin> {
        Some(self)
    }
}

impl<T: Describer + ?Sized> Describer for &T {
    fn describe(&self, el: &P) -> String {
        (**self).describe(el)
    }
    fn game(&self) -> Option<&Game> {
        (**self).game()
    }
    fn skin(&self) -> Option<&Skin> {
        (**self).skin()
    }
}

/// `DefaultSkin.Default`, the skin of every game that has no custom one.
pub fn default_describer() -> Box<dyn Describer> {
    Box::new(treachery_skin::default_skin())
}

/// A skin together with the game it renders for, which is what all text in the game screen
/// uses.
pub struct InGame<'a> {
    pub skin: &'a dyn Describer,
    pub game: &'a Game,
}

impl<'a> InGame<'a> {
    pub fn new(skin: &'a dyn Describer, game: &'a Game) -> Self {
        InGame { skin, game }
    }
}

impl Describer for InGame<'_> {
    fn describe(&self, el: &P) -> String {
        self.skin.describe(el)
    }
    fn game(&self) -> Option<&Game> {
        Some(self.game)
    }
    fn skin(&self) -> Option<&Skin> {
        self.skin.skin()
    }
}
