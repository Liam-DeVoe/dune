//! The `events!` macro, which declares every event type once.

/// Declares `Event` (one variant per event type), `EventType`, the `From` impls, the dispatch
/// and how events are written (wire.rs).
///
/// Each item is an event struct of the same name, which implements `EventRules`
/// (engine-conventions.md, "Events"). `Name: pass` marks an event that can be passed: its struct
/// has a `passed` field, and it is written with its fields in `play`.
macro_rules! events {
    (@ser $s:ident, $name:ident, $e:ident, pass) => {
        ::serde::Serialize::serialize(&$crate::events::wire::Passable { type_: stringify!($name), play: if $e.passed { None } else { Some($e) } }, $s)
    };
    (@ser $s:ident, $name:ident, $e:ident) => {
        ::serde::Serialize::serialize(&$crate::events::wire::Tagged { type_: stringify!($name), event: $e }, $s)
    };
    (@de $map:ident, $name:ident, pass) => {
        $crate::events::wire::play::<$name>($map).map(|play| Event::$name(play.unwrap_or_else(|| $name { passed: true, ..Default::default() })))
    };
    (@de $map:ident, $name:ident) => {
        $crate::events::wire::fields::<$name>($map).map(Event::$name)
    };
    (@ts $name:ident, pass) => {
        format!("  | {{ type: \"{0}\"; play: {0} | null }}", stringify!($name))
    };
    (@ts $name:ident) => {
        format!("  | ({{ type: \"{0}\" }} & {0})", stringify!($name))
    };
    ($( $name:ident $(: $pass:ident)? ),* $(,)?) => {
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

            /// The type's name, which is also its `type` in JSON.
            pub fn name(self) -> &'static str {
                match self {
                    $( EventType::$name => stringify!($name), )*
                }
            }

            /// Inverse of [`Self::name`] (case-sensitive).
            pub fn from_name(s: &str) -> Option<EventType> {
                match s {
                    $( stringify!($name) => Some(EventType::$name), )*
                    _ => None,
                }
            }

            /// A new event of this type, with its fields' defaults.
            pub fn default_event(self) -> Event {
                match self {
                    $( EventType::$name => Event::$name(<$name>::default()), )*
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

            /// The event as the wire format carries it: a pass without its other fields. Playing
            /// an event plays this (`Game::play`), so a game played from its savegame is the game
            /// that was played.
            pub fn normalized(self) -> Event {
                match self {
                    $( Event::$name(e) => $crate::events::macros::events!(@normalize e, $name $(, $pass)?), )*
                }
            }

            /// The TypeScript declaration of `Event`: a union of the event types, tagged with
            /// `type`. Each event's own type is declared by ts-rs under its name.
            pub fn typescript_union() -> String {
                let variants: Vec<String> = vec![ $( $crate::events::macros::events!(@ts $name $(, $pass)?), )* ];
                format!("export type Event =\n{};\n", variants.join("\n"))
            }

            /// The TypeScript declarations of every event struct (ts-rs `decl`), by name.
            pub fn typescript_structs(cfg: &::ts_rs::Config) -> Vec<(&'static str, String)> {
                vec![ $( (stringify!($name), <$name as ::ts_rs::TS>::decl(cfg)), )* ]
            }
        }

        impl ::serde::Serialize for Event {
            fn serialize<S: ::serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                match self {
                    $( Event::$name(e) => $crate::events::macros::events!(@ser s, $name, e $(, $pass)?), )*
                }
            }
        }

        impl<'de> ::serde::Deserialize<'de> for Event {
            fn deserialize<D: ::serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let (t, map) = $crate::events::wire::split_type(d)?;
                let event = match t {
                    $( EventType::$name => $crate::events::macros::events!(@de map, $name $(, $pass)?), )*
                };
                event.map_err(<D::Error as ::serde::de::Error>::custom)
            }
        }
    };
    (@normalize $e:ident, $name:ident, pass) => {
        if $e.passed { Event::$name($name { passed: true, ..Default::default() }) } else { Event::$name($e) }
    };
    (@normalize $e:ident, $name:ident) => {
        Event::$name($e)
    };
}
pub(crate) use events;
