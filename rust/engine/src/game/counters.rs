use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct ObjectCounter<T> {
    #[canon(name = "_counters")]
    pub counters: IndexMap<T, i32>,
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
        *self.counters.entry(obj).or_insert(0) += 1;
    }

    pub fn count2(&mut self, obj: T) {
        *self.counters.entry(obj).or_insert(0) += 2;
    }

    pub fn count_n(&mut self, obj: T, n: i32) {
        *self.counters.entry(obj).or_insert(0) += n;
    }

    pub fn set_to_n(&mut self, obj: T, n: i32) {
        self.counters.insert(obj, n);
    }

    pub fn count_of(&self, obj: T) -> i32 {
        self.counters.get(&obj).copied().unwrap_or(0)
    }

    pub fn counted(&self) -> Vec<T> {
        self.counters.keys().cloned().collect()
    }

    pub fn get_highest(&self, amount_of_items: i32) -> Vec<T> {
        // Most counted first; a stable sort keeps equal counts in counting order.
        let mut counters: Vec<(&T, &i32)> = self.counters.iter().collect();
        counters.sort_by(|a, b| b.1.cmp(a.1));
        counters
            .into_iter()
            .take(if amount_of_items < 0 { 0 } else { amount_of_items as usize })
            .map(|c| c.0.clone())
            .collect()
    }

    pub fn highest(&self) -> Option<T> {
        if self.counters.len() == 0 {
            return None;
        }

        let best_value = self.counters.values().copied().max().expect("a counter");
        let best = self.counters.iter().find(|c| *c.1 == best_value);
        best.map(|c| c.0.clone())
    }

}

#[derive(Clone, Debug, Canon)]
pub struct Timer<T> {
    #[canon(name = "_times")]
    pub times: IndexMap<T, TimeSpan>,
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
        if let Some(current) = self.times.get_mut(&timed_item) {
            *current = current.add(ts);
        } else {
            self.times.insert(timed_item, ts);
        }
    }

}
