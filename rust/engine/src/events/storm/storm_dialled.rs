use crate::log::storm::{StormDials, StormPlaced};
use crate::game::PlayerSequence;
use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct StormDialled {
    pub amount: u16,
}

impl StormDialled {
    pub fn valid_min_amount(g: &Game) -> i32 {
        if g.turn.current_turn == Turn::FIRST {
            return 0;
        }
        1
    }

    pub fn valid_max_amount(g: &Game) -> i32 {
        if g.turn.current_turn == Turn::FIRST {
            return 20;
        }
        3
    }

    pub fn position_first_storm(&self, game: &mut Game) -> GameResult {
        // The dials are validated, so their sum isn't negative.
        let moves = u32::try_from(game.board.next_storm_moves).expect("storm moves");
        let sector = Sector::first_storm(moves);
        game.board.sector_in_storm = Some(sector);
        let msg = express!("The first storm moves ", sector, " sectors").with_log(StormPlaced::new(sector));
        game.log(parts![msg]);
        game.perform_storm()?;

        if game.applicable(Rule::TechTokens) {
            self.assign_tech_tokens(game);
        }

        if game.use_storm_deck() {
            game.board.next_storm_moves = game.determine_later_storm_with_storm_deck();
        }

        if game.is_playing(Faction::Ixian) || game.applicable(Rule::HmSwithoutIxian) {
            game.enter_storm(StormStep::HmsPlacement);
        } else {
            game.end_storm_phase();
        }
        Ok(())
    }

    pub fn assign_tech_tokens(&self, game: &mut Game) {
        let mut tech_tokens_to_be_dealt: Vec<TechToken> = Vec::new();

        let fremen = game.get_player(Some(Faction::Fremen));
        if let Some(fremen) = fremen {
            game.player_mut(fremen).tech_tokens.push(TechToken::Resources);
            game.log(parts![Faction::Fremen, " receive ", TechToken::Resources]);
        } else {
            tech_tokens_to_be_dealt.push(TechToken::Resources);
        }

        let tleilaxu = game.get_player(Some(Faction::Tleilaxu));
        if let Some(tleilaxu) = tleilaxu {
            game.player_mut(tleilaxu).tech_tokens.push(TechToken::Graveyard);
            game.log(parts![Faction::Tleilaxu, " receive ", TechToken::Graveyard]);
        } else {
            tech_tokens_to_be_dealt.push(TechToken::Graveyard);
        }

        let ixian = game.get_player(Some(Faction::Ixian));
        if let Some(ixian) = ixian {
            game.player_mut(ixian).tech_tokens.push(TechToken::Ships);
            game.log(parts![Faction::Ixian, " receive ", TechToken::Ships]);
        } else {
            tech_tokens_to_be_dealt.push(TechToken::Ships);
        }

        let mut remaining_tech_tokens = Deck::new(tech_tokens_to_be_dealt);
        remaining_tech_tokens.shuffle(&mut game.rng);
        game.stone(Milestone::Shuffled);

        let mut tech_token_sequence = PlayerSequence::starting_with_first_player(game);

        while !remaining_tech_tokens.is_empty() && game.players.iter().any(|p| p.tech_tokens.is_empty()) {
            let current = tech_token_sequence.current_player(game).expect("current player");
            if game.player(current).tech_tokens.is_empty() {
                let token = remaining_tech_tokens.draw();
                game.player_mut(current).tech_tokens.push(token);
                let faction = game.player(current).faction;
                game.log(parts![faction, " receive ", token]);
            }

            tech_token_sequence.next_player(game);
        }
    }
}

impl EventRules for StormDialled {
    fn validate(&self, game: &Game, _by: By) -> Result<(), Rejection> {
        if i32::from(self.amount) < Self::valid_min_amount(game) || i32::from(self.amount) > Self::valid_max_amount(game) {
            return Err(Rejection(express!("Invalid amount")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        // Only reached outside the phase when events are replayed without validation.
        if game.storm().is_none() {
            game_error!("Not in the storm phase");
        }
        game.storm_state().dials.push(i32::from(self.amount));
        game.turn.has_acted_or_passed.push(by.faction.required("faction")?);

        if game.board.has_battle_wheel.iter().all(|f| game.turn.has_acted_or_passed.contains(f)) {
            // Arguments are evaluated left to right; a one-player game throws at Dials[1].
            let d0 = game.storm_state().dials[0];
            let f0 = game.turn.has_acted_or_passed[0];
            let d1 = game.storm_state().dials[1];
            let f1 = game.turn.has_acted_or_passed[1];
            let msg = express!("Storm dial: ", d0, f0, " + ", d1, f1, " = ", d0 + d1)
                .with_log(StormDials::new(f0, d0, f1, d1));
            game.log(parts![msg]);
            game.board.next_storm_moves = game.storm_state().dials.iter().copied().sum::<i32>();

            if game.turn.current_turn == Turn::FIRST {
                self.position_first_storm(game)?;
            } else {
                game.reveal_storm();
            }
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " dial for the storm")
    }
}
