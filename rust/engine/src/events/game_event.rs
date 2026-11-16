use crate::prelude::*;

impl Event {
    /// `((LocationEvent)e).To` of an event the game holds (`LastShipmentOrMovement`,
    /// `Intrusion.TriggeringEvent`).
    pub fn to_location(&self, game: &Game) -> Option<Location> {
        self.location_event_to(game)
    }
}

/// Associated items shared by all events.
impl Event {
    pub const ID_STRING_SEPARATOR: char = ';';

    pub fn id_string_to_objects<T>(ids: Option<&str>, lookup: &dyn Fetcher<T>) -> Vec<T> {
        let mut result = Vec::new();

        if let Some(ids) = ids {
            if !ids.is_empty() {
                for id in ids.split(Event::ID_STRING_SEPARATOR) {
                    let obj = lookup.find(crate::parse::int_or_panic(id));
                    if let Some(obj) = obj {
                        result.push(obj);
                    }
                }
            }
        }

        result
    }

    pub fn objects_to_id_string<T: Clone>(objs: &[T], lookup: &dyn Fetcher<T>) -> String {
        objs.iter()
            .map(|pj| lookup.get_id(Some(pj.clone())).to_string())
            .collect::<Vec<_>>()
            .join(&Event::ID_STRING_SEPARATOR.to_string())
    }

}
