use crate::prelude::*;

// C# enum `AsideKind` is generated into enumerations.rs (tools/gen_enumerations.py).

#[derive(Clone, Debug, PartialEq, Canon)]
pub struct AsideLog {
    pub kind: AsideKind,
}

impl Default for AsideLog {
    fn default() -> Self {
        AsideLog {
            kind: Default::default(),
        }
    }
}

impl AsideLog {
    pub fn new(kind: AsideKind) -> Self {
        AsideLog { kind }
    }

    pub fn aside() -> AsideLog {
        AsideLog::new(AsideKind::Aside)
    }

    pub fn answer() -> AsideLog {
        AsideLog::new(AsideKind::Answer)
    }

    pub fn after() -> AsideLog {
        AsideLog::new(AsideKind::After)
    }

}
