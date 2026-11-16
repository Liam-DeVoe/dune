//! The 18 sectors of the board, and the storm's movement over them.

/// One of the 18 sectors of the board, numbered 0 to 17 counterclockwise, as the storm moves.
/// Homeworlds have no sector, and neither has an attached location (the Hidden Mobile Stronghold,
/// a discovered stronghold) before it is placed, so `Location::sector` returns `Option<Sector>`.
/// The storm has no sector before the first storm (`Board::sector_in_storm` is `None`).
///
/// Views and the log write the number, and -1 for none ([`Sector::number_or_none`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sector(u8);

impl Sector {
    pub const COUNT: u8 = 18;

    /// The sector numbered `number`. Panics outside 0 to 17: the static data and the storm's
    /// movement only make valid sectors.
    #[track_caller]
    pub const fn new(number: u8) -> Sector {
        assert!(number < Self::COUNT, "a sector is numbered 0 to 17");
        Sector(number)
    }

    pub fn number(self) -> u8 {
        self.0
    }

    /// Where the first storm lands: `moves` sectors from sector 0.
    pub fn first_storm(moves: u32) -> Sector {
        Sector((moves % Self::COUNT as u32) as u8)
    }

    /// The sector `moves` sectors further on, counterclockwise. The only place the storm's
    /// arithmetic modulo 18 is done.
    pub fn moved(self, moves: u32) -> Sector {
        Sector(((self.0 as u32 + moves % Self::COUNT as u32) % Self::COUNT as u32) as u8)
    }

    /// Whether a location in `sector` is in a storm over `storm`. A location without a sector
    /// never is, and nothing is before the first storm.
    pub fn in_storm(sector: Option<Sector>, storm: Option<Sector>) -> bool {
        sector.is_some() && sector == storm
    }

    /// The number of `sector`, or -1 for none, as views and the log write it.
    pub fn number_or_none(sector: Option<Sector>) -> i32 {
        sector.map_or(-1, |s| s.0 as i32)
    }
}

impl From<Sector> for i32 {
    fn from(s: Sector) -> i32 {
        s.0 as i32
    }
}

impl std::fmt::Display for Sector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
