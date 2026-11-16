//! The `events!` macro, which declares every event type once.

/// Declares `Event` (one variant per event type), `EventType`, the `From` impls and the dispatch.
///
/// Each item is an event struct of the same name, which implements `EventRules`
/// (engine-conventions.md, "Events").
macro_rules! events {
    ($( $name:ident ),* $(,)?) => {
        /// Any event.
        #[derive(Clone, PartialEq)]
        pub enum Event {
            $( $name($name), )*
        }

        $(
            impl From<$name> for Event {
                fn from(e: $name) -> Self {
                    Event::$name(e)
                }
            }
        )*

        /// The type of an event, as `Event::event_type` gives it and `Game::get_applicable_events`
        /// lists them.
        #[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
        pub enum EventType {
            $( $name, )*
        }

        impl EventType {
            /// Every event type, in name order.
            pub const ALL: &'static [EventType] = &[ $( EventType::$name, )* ];

            pub fn name(self) -> &'static str {
                match self {
                    $( EventType::$name => stringify!($name), )*
                }
            }

            /// The `$type` of the event in JSON: its name.
            pub fn json_discriminator(self) -> Option<&'static str> {
                Some(self.name())
            }

            /// Inverse of [`Self::json_discriminator`] (ordinal, case-sensitive).
            pub fn from_json_discriminator(s: &str) -> Option<EventType> {
                match s {
                    $( stringify!($name) => Some(EventType::$name), )*
                    _ => None,
                }
            }
        }

        impl Event {
            pub fn event_type(&self) -> EventType {
                match self {
                    $( Event::$name(_) => EventType::$name, )*
                }
            }

            /// The event's rules.
            pub(crate) fn rules(&self) -> &dyn $crate::events::EventRules {
                match self {
                    $( Event::$name(e) => e, )*
                }
            }

            /// The event's JSON members (without `$type`, `Initiator` and `Time`).
            pub(crate) fn json_members(&self) -> &dyn $crate::json::JsonMembers {
                match self {
                    $( Event::$name(e) => e, )*
                }
            }

            /// Builds an event of type `t` from its JSON members. `other` reads the members that
            /// aren't the event's own (`Initiator`, `Time`), in document order.
            pub(crate) fn read_json(
                t: EventType,
                members: &[(String, $crate::json::JsonValue)],
                other: &mut dyn FnMut(&str, &$crate::json::JsonValue) -> Result<bool, $crate::json::JsonError>,
            ) -> Result<Event, $crate::json::JsonError> {
                Ok(match t {
                    $( EventType::$name => Event::$name($crate::json::read_object_with::<$name>(members, other)?), )*
                })
            }
        }
    };
}
pub(crate) use events;
