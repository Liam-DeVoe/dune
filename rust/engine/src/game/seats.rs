use crate::prelude::*;

/// Finding players by faction and seat, and which seats bots play.
impl Game {
    pub fn is_playing(&self, faction: impl Into<Option<Faction>>) -> bool {
        let faction: Option<Faction> = faction.into();
        self.players.iter().any(|p| p.faction == faction)
    }

    pub fn get_player(&self, f: Option<Faction>) -> Option<PlayerId> {
        self.players.iter().position(|p| p.faction == f).map(|i| self.player_at(i))
    }

    pub fn get_player_in_seat(&self, seat: i32) -> Option<PlayerId> {
        self.players.iter().position(|p| p.seat == seat).map(|i| self.player_at(i))
    }

    pub fn get_faction_in_seat(&self, seat: i32) -> Option<Faction> {
        self.get_player_in_seat(seat).map(|p| self.player(p).faction).flatten()
    }

    pub fn get_ally(&self, f: Option<Faction>) -> Option<Faction> {
        self.get_player(f).map(|p| self.player(p).ally).flatten()
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn players_other_than(&self, p: PlayerId) -> impl Iterator<Item = Faction> {
        self.players.iter().filter(move |x| x.faction != self.player(p).faction).filter_map(move |x| x.faction)
    }

    pub fn get_player_by_seat(&self, seat_nr: i32) -> Option<PlayerId> {
        self.players.iter().position(|p| p.seat == seat_nr).map(|i| self.player_at(i))
    }

    /// Whether a bot plays this player: whether no user sits in its seat (`human_seats`).
    pub fn is_bot(&self, p: PlayerId) -> bool {
        !self.human_seats.contains(&self.player(p).seat)
    }

    pub fn number_of_bots(&self) -> i32 {
        self.player_indices().filter(|p| self.is_bot(*p)).count() as i32
    }
}
