use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct PlayerSequence {
    pub players: Vec<PlayerId>,
    /// Elements are `Option` because a null `CurrentPlayer` can be added (see `players_in_order`).
    pub played: Vec<Option<PlayerId>>,
    pub skip_players_that_cant_bid_on_cards: bool,
    pub round: i32,
    pub fully_circled: i32,
    /// `Option` because the constructor can be handed a null `toStartWith`
    /// (`richesePlayer` in `Game.StartBidSequenceAndAuctionType`).
    pub first_player: Option<PlayerId>,
}

impl PlayerSequence {
    pub fn new(game: &Game, skip_players_that_cant_bid_on_cards: bool, direction: i32, to_start_with: Option<PlayerId>, begin_next_to_that_player: bool) -> Self {
        // Players = [.. _game.Players.OrderBy(p => direction * p.Seat)]  (stable)
        let mut players: Vec<PlayerId> = game.player_indices().collect();
        players.sort_by_cached_key(|p| direction * game.player(*p).seat);

        let mut result = PlayerSequence {
            players,
            played: Vec::new(),
            skip_players_that_cant_bid_on_cards,
            round: 1,
            fully_circled: 0,
            first_player: to_start_with,
        };

        if begin_next_to_that_player && result.number_of_players_that_may_get_turn(game) > 1 {
            result.first_player = Some(result.player_after(game, result.first_player, false));
        }

        result.change_first_player_if_they_cant_bid(game);

        result
    }

    pub fn starting_with_first_player_options(game: &Game, skip_players_that_cant_bid_on_cards: bool, direction: i32) -> Self {
        let first = PlayerSequence::determine_first_player(game);
        PlayerSequence::new(game, skip_players_that_cant_bid_on_cards, direction, Some(first), false)
    }

    pub fn starting_with_first_player(game: &Game) -> Self {
        let first = PlayerSequence::determine_first_player(game);
        PlayerSequence::new(game, false, 1, Some(first), false)
    }

    pub fn change_first_player_if_they_cant_bid(&mut self, game: &Game) {
        if self.number_of_players_that_may_get_turn(game) > 1 {
            while !self.may_get_turn_opt(game, self.first_player) || self.juice_forces_last_opt(game, self.first_player) {
                self.first_player = Some(self.player_after(game, self.first_player, false));
            }
        }
    }

    pub fn number_of_players_that_may_get_turn(&self, game: &Game) -> i32 {
        self.players
            .iter()
            .filter(|p| !self.skip_players_that_cant_bid_on_cards || game.player(**p).has_room_for_cards(game))
            .count() as i32
    }

    pub fn may_get_turn(&self, game: &Game, p: PlayerId) -> bool {
        !self.skip_players_that_cant_bid_on_cards || game.player(p).has_room_for_cards(game)
    }

    /// `may_get_turn` for a player that may be missing: panics for a missing player when it matters.
    fn may_get_turn_opt(&self, game: &Game, p: Option<PlayerId>) -> bool {
        !self.skip_players_that_cant_bid_on_cards || game.player(p.expect("player")).has_room_for_cards(game)
    }

    /// `JuiceForcesLast(p)` where `p` may be null.
    fn juice_forces_last_opt(&self, game: &Game, p: Option<PlayerId>) -> bool {
        match &game.turn.current_juice {
            Some(juice) if juice.type_ == Some(JuiceType::GoLast) => juice.player_opt() == p,
            _ => false,
        }
    }

    pub fn check_current_player(&mut self, game: &Game) {
        if self.round == 1 && self.played.is_empty() {
            self.change_first_player_if_they_cant_bid(game);
        }
    }

    /// `current_player` may be null; it then matches no player.
    pub fn player_after(&self, game: &Game, current_player: Option<PlayerId>, include_players_that_cant_bid: bool) -> PlayerId {
        let mut current_player_found = false;

        let mut first_found: Option<PlayerId> = None;
        for &p in &self.players {
            if current_player_found {
                if include_players_that_cant_bid || self.may_get_turn(game, p) {
                    return p;
                }
            } else if Some(p) == current_player {
                current_player_found = true;
            }

            if first_found.is_none() && (include_players_that_cant_bid || self.may_get_turn(game, p)) {
                first_found = Some(p);
            }
        }

        match first_found {
            Some(p) => p,
            None => {
                let faction = game.player(current_player.expect("current player")).faction;
                panic!("Could not find a player after {}", name_or_absent(faction))
            }
        }
    }

    pub fn current_faction(&self, game: &Game) -> Faction {
        game.player(self.current_player(game).expect("current player")).faction.expect("faction")
    }

    /// Can be null only when `PlayersInOrder` contains a null (see `players_in_order`).
    pub fn current_player(&self, game: &Game) -> Option<PlayerId> {
        self.determine_current_player(game)
    }

    pub fn determine_current_player(&self, game: &Game) -> Option<PlayerId> {
        if !self.played.is_empty() {
            let mut found = false;
            for p in self.players_in_order(game) {
                if found {
                    return p;
                } else {
                    found = p == self.played[self.played.len() - 1];
                }
            }
        }

        match self.players_in_order(game).first().copied().flatten() {
            Some(p) => Some(p),
            None => panic!("Could not determine the current player."),
        }
    }

    pub fn next_player(&mut self, game: &Game) {
        let current_player = self.current_player(game);

        self.played.push(current_player);

        if self.number_of_players_that_may_get_turn(game) == 0 {
            self.played.clear();
        }

        if self.current_player(game) == self.first_player {
            self.fully_circled += 1;
        }
    }

    pub fn has_passed_richese(&self, game: &Game) -> bool {
        if self.fully_circled > 0 {
            return true;
        }

        let current = self.current_player(game);

        for p in self.players_in_order(game) {
            if p == current {
                return false;
            } else if game.player(p.expect("player")).faction == Some(Faction::Richese) {
                return true;
            }
        }

        false
    }

    pub fn next_round(&mut self, game: &Game) {
        self.first_player = Some(self.player_after(game, self.first_player, false));

        self.played.clear();

        self.round += 1;
    }

    /// Elements are `Option` because a null `FirstPlayer` can be added to the list.
    pub fn players_in_order(&self, game: &Game) -> Vec<Option<PlayerId>> {
        let mut result: Vec<Option<PlayerId>> = Vec::new();
        let player_with_juice = match &game.turn.current_juice {
            Some(j) if j.type_ == Some(JuiceType::GoFirst) || j.type_ == Some(JuiceType::GoLast) => j.player_opt(),
            _ => None,
        };

        if player_with_juice.is_none() || self.number_of_players_that_may_get_turn(game) > 1 {
            let mut to_add = self.first_player;
            while result.is_empty() || to_add != result[0] {
                if to_add != player_with_juice {
                    result.push(to_add);
                }

                to_add = Some(self.player_after(game, to_add, true));
            }
        }

        if player_with_juice.is_some() {
            let juice = game.turn.current_juice.as_ref().expect("current juice");
            if juice.type_ == Some(JuiceType::GoFirst) {
                result.insert(0, player_with_juice);
            } else if juice.type_ == Some(JuiceType::GoLast) {
                result.push(player_with_juice);
            }
        }

        result
    }

    /// Lazy: PlayersInOrder is computed at the
    /// call, CurrentPlayer (which can throw) per element, and only if the caller enumerates.
    pub fn get_players_in_sequence(&self, game: &Game) -> impl Iterator<Item = SequenceElement> {
        self.players_in_order(game)
            .into_iter()
            .map(move |p| SequenceElement { player: p, has_turn: self.current_player(game) == p })
    }

    pub fn determine_first_player(g: &Game) -> PlayerId {
        let start_looking_in_sector = (((g.board.sector_in_storm as f32 * g.maximum_players() as f32 / Map::NUMBER_OF_SECTORS as f32) as f64).ceil() as i32) % g.maximum_players();

        let mut result: Option<PlayerId> = None;

        let mut position = (g.maximum_players() + start_looking_in_sector) % g.maximum_players();
        let mut i = 0;
        while i < g.maximum_players() {
            result = g.players.iter().position(|p| p.seat == position).map(|i| g.player_at(i));
            if let Some(r) = result {
                return r;
            }
            position = PlayerSequence::mod_(g.maximum_players() + position + 1, g.maximum_players());
            i += 1;
        }

        match result {
            Some(r) => r,
            None => panic!("Could not determine the first player."),
        }
    }

    pub fn mod_(x: i32, m: i32) -> i32 {
        x.rem_euclid(m)
    }

    pub fn is_after(g: &Game, a: PlayerId, b: PlayerId) -> bool {
        let first_player = PlayerSequence::determine_first_player(g);

        let mut i = 0;
        while i < g.maximum_players() {
            let position_to_check = (g.player(first_player).seat + i) % g.maximum_players();

            if position_to_check == g.player(b).seat {
                return true;
            }
            if position_to_check == g.player(a).seat {
                return false;
            }
            i += 1;
        }

        false
    }
}

#[derive(Clone, Debug)]
pub struct SequenceElement {
    /// Can be null (see `PlayerSequence::players_in_order`).
    pub player: Option<PlayerId>,
    pub has_turn: bool,
}
