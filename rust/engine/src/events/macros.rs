//! The `events!` macro, which declares every event type once.

/// What an `Event` variant holds: the event itself, or an `Rc` for an `@legacy` event.
macro_rules! event_payload {
    (legacy $name:ident) => { ::std::rc::Rc<$name> };
    ($name:ident) => { $name };
}
pub(crate) use event_payload;

/// Wraps a decoded event in its variant.
macro_rules! wrap_event {
    (legacy $name:ident, $e:expr) => { Event::$name(::std::rc::Rc::new($e)) };
    ($name:ident, $e:expr) => { Event::$name($e) };
}
pub(crate) use wrap_event;

/// `==` on one variant: by identity for an `@legacy` event (it is shared), by value otherwise.
macro_rules! event_eq {
    (legacy $a:ident, $b:ident) => { ::std::rc::Rc::ptr_eq($a, $b) };
    ($a:ident, $b:ident) => { $a == $b };
}
pub(crate) use event_eq;

/// The event struct inside a variant.
macro_rules! event_inner {
    (legacy $e:ident) => { &**$e };
    ($e:ident) => { $e };
}
pub(crate) use event_inner;

/// A copy of the event that shares nothing with the original.
macro_rules! event_copy {
    (legacy $name:ident, $e:ident) => { Event::$name(::std::rc::Rc::new((**$e).clone())) };
    ($name:ident, $e:ident) => { Event::$name($e.clone()) };
}
pub(crate) use event_copy;

/// The base of an `@legacy` event, `None` for a converted one.
macro_rules! event_legacy_base {
    (legacy $e:ident) => { Some(&$e.base) };
    ($e:ident) => { { let _ = $e; None } };
}
pub(crate) use event_legacy_base;

/// Declares `Event` (one variant per event type), `EventType`, the `From` impls and the dispatch.
///
/// Each item is an event struct of the same name. `@legacy X` is an event not yet converted to
/// plain data: its variant holds an `Rc<X>`, and it implements `GameEventMethods` instead of
/// `EventRules` (engine-conventions.md, "Events").
macro_rules! events {
    ($( $(@$legacy:ident)? $name:ident ),* $(,)?) => {
        /// Any event.
        #[derive(Clone)]
        pub enum Event {
            $( $name($crate::events::macros::event_payload!($($legacy)? $name)), )*
        }

        $(
            impl From<$crate::events::macros::event_payload!($($legacy)? $name)> for Event {
                fn from(e: $crate::events::macros::event_payload!($($legacy)? $name)) -> Self {
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

        impl PartialEq for Event {
            fn eq(&self, other: &Self) -> bool {
                match (self, other) {
                    $( (Event::$name(a), Event::$name(b)) => $crate::events::macros::event_eq!($($legacy)? a, b), )*
                    _ => false,
                }
            }
        }

        impl Event {
            pub fn event_type(&self) -> EventType {
                match self {
                    $( Event::$name(_) => EventType::$name, )*
                }
            }

            /// The event's rules, whether it is converted or `@legacy`.
            pub(crate) fn rules(&self) -> &dyn $crate::events::dispatch::EventImpl {
                match self {
                    $( Event::$name(e) => e, )*
                }
            }

            /// A copy that shares nothing with this event: `clone` shares an `@legacy` event,
            /// whose base `Game::play` writes.
            pub fn detached(&self) -> Event {
                match self {
                    $( Event::$name(e) => $crate::events::macros::event_copy!($($legacy)? $name, e), )*
                }
            }

            /// The base of an `@legacy` event: its initiator and player. `None` for a converted
            /// event, whose submitter is in its `HistoryEntry`.
            pub fn legacy_base(&self) -> Option<&$crate::events::GameEventBase> {
                match self {
                    $( Event::$name(e) => $crate::events::macros::event_legacy_base!($($legacy)? e), )*
                }
            }

            /// The event's JSON members (without `$type`, `Initiator` and `Time`).
            pub(crate) fn json_members(&self) -> &dyn $crate::json::JsonMembers {
                match self {
                    $( Event::$name(e) => $crate::events::macros::event_inner!($($legacy)? e), )*
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
                    $( EventType::$name => $crate::events::macros::wrap_event!($($legacy)? $name, $crate::json::read_object_with::<$name>(members, other)?), )*
                })
            }
        }
    };
}
pub(crate) use events;
