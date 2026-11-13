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

    /// A C# static field: decide per site whether it is a constant table (a fn or a `static`) or process-wide state (hazards.md section 6.2).
    pub fn aside() -> AsideLog {
        AsideLog::new(AsideKind::Aside)
    }

    /// A C# static field: decide per site whether it is a constant table (a fn or a `static`) or process-wide state (hazards.md section 6.2).
    pub fn answer() -> AsideLog {
        AsideLog::new(AsideKind::Answer)
    }

    /// A C# static field: decide per site whether it is a constant table (a fn or a `static`) or process-wide state (hazards.md section 6.2).
    pub fn after() -> AsideLog {
        AsideLog::new(AsideKind::After)
    }

}
