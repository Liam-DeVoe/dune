use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Moment {
    pub turn: Turn,
    pub phase: MainPhase,
}

impl Moment {
    pub fn new(turn: Turn, phase: MainPhase) -> Self {
        Moment { turn, phase }
    }

}
