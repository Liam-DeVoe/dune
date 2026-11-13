use crate::prelude::*;

/// Holds the static members of `PassableGameEvent`. Its fields are inlined into every derived event struct;
/// its instance members are the default methods of [`PassableGameEventMethods`].
pub struct PassableGameEvent;

/// The instance members of the C# class `PassableGameEvent`. Every event deriving from it implements this trait
/// (generated in its module); overridden virtual members are delegated to the event's inherent method.
pub trait PassableGameEventMethods: GameEventMethods {
    /// The inherited field `Passed` (`self.passed` on the event struct).
    fn field_passed(&self) -> &bool;

}
