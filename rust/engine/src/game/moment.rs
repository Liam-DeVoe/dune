use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct Moment {
    pub turn: i32,
    pub phase: MainPhase,
}

impl Default for Moment {
    fn default() -> Self {
        Moment {
            turn: Default::default(),
            phase: Default::default(),
        }
    }
}

impl Moment {
    pub fn new(turn: i32, phase: MainPhase) -> Self {
        Moment { turn, phase }
    }

}
