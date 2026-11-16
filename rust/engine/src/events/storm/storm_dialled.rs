use crate::log::storm::{StormDials, StormPlaced};
use crate::game::PlayerSequence;
use crate::prelude::*;
use crate::messages::part::Part;

/// `params object?[]` element list for `Log(...)`.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(Part::from($e)),*] };
}

#[derive(Clone, Debug)]
pub struct StormDialled {
    pub amount: u16,
    /// The fields of `Event` (Initiator, Time, Game, Player).
    pub base: GameEventBase,
}

impl Default for StormDialled {
    fn default() -> Self {
        StormDialled {
            amount: Default::default(),
            base: Default::default(),
        }
    }
}

impl StormDialled {
    pub fn new(game: &Game, initiator: impl Into<Option<Faction>>) -> Self {
        let e = Self::default();
        e.initialize(game, initiator.into());
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if i32::from(self.amount) < Self::valid_min_amount(game) || i32::from(self.amount) > Self::valid_max_amount(game) {
            return Some(express!("Invalid amount"));
        }

        None
    }

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

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.storm.dials.push(i32::from(self.amount));
        game.turn.has_acted_or_passed.push(self.initiator().required("faction")?);

        if game.storm.has_battle_wheel.iter().all(|f| game.turn.has_acted_or_passed.contains(f)) {
            // Arguments are evaluated left to right; a one-player game throws at Dials[1].
            let d0 = game.storm.dials[0];
            let f0 = game.turn.has_acted_or_passed[0];
            let d1 = game.storm.dials[1];
            let f1 = game.turn.has_acted_or_passed[1];
            let msg = express!("Storm dial: ", d0, f0, " + ", d1, f1, " = ", d0 + d1)
                .with_log(StormDials::new(f0, d0, f1, d1));
            self.log_objects(game, elems![msg]);
            game.board.next_storm_moves = game.storm.dials.iter().copied().sum::<i32>();

            if game.turn.current_turn == Turn::FIRST {
                self.position_first_storm(game)?;
            } else {
                game.reveal_storm();
            }
        }
        Ok(())
    }

    pub fn position_first_storm(&self, game: &mut Game) -> GameResult {
        // The dials are validated, so their sum isn't negative.
        let moves = u32::try_from(game.board.next_storm_moves).expect("storm moves");
        let sector = Sector::first_storm(moves);
        game.board.sector_in_storm = Some(sector);
        let msg = express!("The first storm moves ", sector, " sectors").with_log(StormPlaced::new(sector));
        self.log_objects(game, elems![msg]);
        game.perform_storm()?;

        if game.applicable(Rule::TechTokens) {
            self.assign_tech_tokens(game);
        }

        if game.use_storm_deck() {
            game.board.next_storm_moves = game.determine_later_storm_with_storm_deck();
        }

        let condition = self.is_playing(game, Faction::Ixian) || game.applicable(Rule::HmSwithoutIxian);
        game.enter_or_else(condition, Phase::HmsPlacement, Game::end_storm_phase);
        Ok(())
    }

    pub fn assign_tech_tokens(&self, game: &mut Game) {
        let mut tech_tokens_to_be_dealt: Vec<TechToken> = Vec::new();

        let fremen = self.get_player(game, Some(Faction::Fremen));
        if let Some(fremen) = fremen {
            game.player_mut(fremen).tech_tokens.push(TechToken::Resources);
            self.log_objects(game, elems![Faction::Fremen, " receive ", TechToken::Resources]);
        } else {
            tech_tokens_to_be_dealt.push(TechToken::Resources);
        }

        let tleilaxu = self.get_player(game, Some(Faction::Tleilaxu));
        if let Some(tleilaxu) = tleilaxu {
            game.player_mut(tleilaxu).tech_tokens.push(TechToken::Graveyard);
            self.log_objects(game, elems![Faction::Tleilaxu, " receive ", TechToken::Graveyard]);
        } else {
            tech_tokens_to_be_dealt.push(TechToken::Graveyard);
        }

        let ixian = self.get_player(game, Some(Faction::Ixian));
        if let Some(ixian) = ixian {
            game.player_mut(ixian).tech_tokens.push(TechToken::Ships);
            self.log_objects(game, elems![Faction::Ixian, " receive ", TechToken::Ships]);
        } else {
            tech_tokens_to_be_dealt.push(TechToken::Ships);
        }

        let mut remaining_tech_tokens = Deck::new(tech_tokens_to_be_dealt, game.random.clone());
        remaining_tech_tokens.shuffle();
        game.stone(Milestone::Shuffled);

        let mut tech_token_sequence = PlayerSequence::starting_with_first_player(game);

        while !remaining_tech_tokens.is_empty() && game.players.iter().any(|p| p.tech_tokens.is_empty()) {
            let current = tech_token_sequence.current_player(game).expect("current player");
            if game.player(current).tech_tokens.is_empty() {
                let token = remaining_tech_tokens.draw();
                game.player_mut(current).tech_tokens.push(token);
                let faction = game.player(current).faction;
                self.log_objects(game, elems![faction, " receive ", token]);
            }

            tech_token_sequence.next_player(game);
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " dial for the storm")
    }

}

impl GameEventMethods for StormDialled {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> EventType {
        EventType::StormDialled
    }

    fn into_event(self: Rc<Self>) -> Event {
        Event::StormDialled(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        StormDialled::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        StormDialled::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        StormDialled::get_message(self, game)
    }

}
