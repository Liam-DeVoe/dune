use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct ObjectCounter<T> {
    #[canon(name = "_counters")]
    pub counters: NetDictionary<T, i32>,
}

impl<T: Clone + Eq + std::hash::Hash + 'static> Default for ObjectCounter<T> {
    fn default() -> Self {
        ObjectCounter {
            counters: Default::default(),
        }
    }
}

impl<T: Clone + Eq + std::hash::Hash + 'static> ObjectCounter<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn count(&mut self, obj: T) {
        if !self.counters.try_add(obj.clone(), 1) {
            let c = self.counters.get_mut(&obj).nre();
            *c += 1;
        }
    }

    pub fn count2(&mut self, obj: T) {
        if !self.counters.try_add(obj.clone(), 2) {
            let c = self.counters.get_mut(&obj).nre();
            *c += 2;
        }
    }

    pub fn count_n(&mut self, obj: T, n: i32) {
        if !self.counters.try_add(obj.clone(), n) {
            let c = self.counters.get_mut(&obj).nre();
            *c += n;
        }
    }

    pub fn set_to_n(&mut self, obj: T, n: i32) {
        self.counters.insert(obj, n);
    }

    pub fn count_of(&self, obj: T) -> i32 {
        self.counters.get_value_or(&obj, 0)
    }

    pub fn counted(&self) -> Vec<T> {
        self.counters.keys().cloned().collect()
    }

    pub fn get_highest(&self, amount_of_items: i32) -> Vec<T> {
        // C# returns a lazy query over the live dictionary; every caller enumerates it right away.
        dotnet_compat::linq::order_by_descending(self.counters.iter(), |c| *c.1)
            .into_iter()
            .take(if amount_of_items < 0 { 0 } else { amount_of_items as usize })
            .map(|c| c.0.clone())
            .collect()
    }

    pub fn highest(&self) -> Option<T> {
        if self.counters.len() == 0 {
            return None;
        }

        let best_value = dotnet_compat::linq::max(self.counters.values().copied());
        let best = self.counters.iter().find(|c| *c.1 == best_value);
        best.map(|c| c.0.clone())
    }

}

#[derive(Clone, Debug, Canon)]
pub struct Timer<T> {
    #[canon(name = "_times")]
    pub times: NetDictionary<T, TimeSpan>,
}

impl<T: Clone + Eq + std::hash::Hash + 'static> Default for Timer<T> {
    fn default() -> Self {
        Timer {
            times: Default::default(),
        }
    }
}

impl<T: Clone + Eq + std::hash::Hash + 'static> Timer<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn time_spent(&self, timed_item: T) -> TimeSpan {
        match self.times.get(&timed_item) {
            Some(ts) => *ts,
            None => TimeSpan::default(),
        }
    }

    pub fn add(&mut self, timed_item: T, ts: TimeSpan) {
        if !self.times.try_add(timed_item.clone(), ts) {
            // _times[timedItem] += ts (TimeSpan + TimeSpan throws on overflow)
            let current = self.times.get_mut(&timed_item).nre();
            *current = current.add(ts);
        }
    }

}
