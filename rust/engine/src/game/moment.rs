use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Moment {
    pub turn: i32,
    pub phase: MainPhase,
}

impl Moment {
    pub fn new(turn: i32, phase: MainPhase) -> Self {
        Moment { turn, phase }
    }

}
