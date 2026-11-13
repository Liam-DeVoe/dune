use crate::prelude::*;

/// Holds the static members of `PassableGameEvent`. Its fields are inlined into every derived event struct;
/// its instance members are the default methods of [`PassableGameEventMethods`].
pub struct PassableGameEvent;

/// The instance members of passable events. Every passable event implements this trait
/// (generated in its module); members an event overrides are delegated to its inherent method.
pub trait PassableGameEventMethods: GameEventMethods {
    /// The inherited field `Passed` (`self.passed` on the event struct).
    fn field_passed(&self) -> &bool;

}
