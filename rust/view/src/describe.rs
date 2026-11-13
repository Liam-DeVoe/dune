//! How a view names things: the skin's `Describe`.
//!
//! The text of every log line and prompt is rendered on the server, with the game's skin, as the
//! C# client rendered it with `Client.CurrentSkin`. The skin is the shared crate `rust/skin`
//! (coordinator decision, server-plan.md "Coordinator answers" 1); a custom skin is a
//! `treachery_skin::Skin` loaded with `Skin::load`, which the server puts in the replica.

use treachery_engine::messages::expression::ExpressionElement;
use treachery_engine::game::Game;
use treachery_skin::Skin;

pub trait Describer {
    /// `Skin.Describe(object)`. Live game collections inside the value (hazards 4.5) are read
    /// from `game` when one is given, as the C# client enumerated them at render time.
    fn describe_in(&self, el: &ExpressionElement, game: Option<&Game>) -> String;

    /// The game the C# component had in its `Game` parameter: live collections inside messages
    /// are read from it, and a leader's popover shows the skill it has in it (`Game.Skill`).
    fn game(&self) -> Option<&Game> {
        None
    }

    /// `Skin.Describe(object)`, against [`Describer::game`].
    fn describe(&self, el: &ExpressionElement) -> String {
        self.describe_in(el, self.game())
    }

    /// The skin itself, for texts that are more than `Describe` (the faction sheets). `None` for
    /// a describer without one.
    fn skin(&self) -> Option<&Skin> {
        None
    }
}

impl Describer for Skin {
    fn describe_in(&self, el: &ExpressionElement, game: Option<&Game>) -> String {
        Skin::describe_in(self, el, game)
    }
    fn skin(&self) -> Option<&Skin> {
        Some(self)
    }
}

impl<T: Describer + ?Sized> Describer for &T {
    fn describe_in(&self, el: &ExpressionElement, game: Option<&Game>) -> String {
        (**self).describe_in(el, game)
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

/// A skin together with the game it renders for: what an `ExpressionComponent` has when its
/// `Game` parameter is set, which is everywhere in the game screen.
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
    fn describe_in(&self, el: &ExpressionElement, game: Option<&Game>) -> String {
        self.skin.describe_in(el, game.or(Some(self.game)))
    }
    fn game(&self) -> Option<&Game> {
        Some(self.game)
    }
    fn skin(&self) -> Option<&Skin> {
        self.skin.skin()
    }
}
