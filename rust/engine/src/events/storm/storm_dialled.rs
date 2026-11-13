use crate::log::storm::{StormDials, StormPlaced};
use crate::game::PlayerSequence;
use crate::prelude::*;

/// `params object?[]` element list for `Log(...)`.
macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

#[derive(Clone, Debug, Canon)]
pub struct StormDialled {
    pub amount: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
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
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.amount < Self::valid_min_amount(game) || self.amount > Self::valid_max_amount(game) {
            return Some(express!("Invalid amount"));
        }

        None
    }

    pub fn valid_min_amount(g: &Game) -> i32 {
        if g.current_turn == 1 {
            return 0;
        }
        1
    }

    pub fn valid_max_amount(g: &Game) -> i32 {
        if g.current_turn == 1 {
            return 20;
        }
        3
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        game.dials.push(self.amount);
        game.has_acted_or_passed.push(self.initiator());

        if game.has_battle_wheel.iter().all(|f| game.has_acted_or_passed.contains(f)) {
            // Arguments are evaluated left to right; a one-player game throws at Dials[1] (hazards 1.4).
            let d0 = game.dials[0];
            let f0 = game.has_acted_or_passed[0];
            let d1 = game.dials[1];
            let f1 = game.has_acted_or_passed[1];
            let msg = express!("Storm dial: ", d0, f0, " + ", d1, f1, " = ", d0 + d1)
                .with_log(StormDials::new(f0, d0, f1, d1));
            self.log_objects(game, elems![msg]);
            game.next_storm_moves = game.dials.iter().copied().sum::<i32>();

            if game.current_turn == 1 {
                self.position_first_storm(game);
            } else {
                game.reveal_storm();
            }
        }
        Ok(())
    }

    pub fn position_first_storm(&self, game: &mut Game) {
        game.sector_in_storm = game.next_storm_moves % Map::NUMBER_OF_SECTORS;
        let msg = express!("The first storm moves ", game.sector_in_storm, " sectors").with_log(StormPlaced::new(game.sector_in_storm));
        self.log_objects(game, elems![msg]);
        game.perform_storm();

        if game.applicable(Rule::TechTokens) {
            self.assign_tech_tokens(game);
        }

        if game.use_storm_deck() {
            game.next_storm_moves = game.determine_later_storm_with_storm_deck();
        }

        let condition = self.is_playing(game, Faction::Ixian) || game.applicable(Rule::HmSwithoutIxian);
        game.enter_boolean_phase_action(condition, Phase::HmsPlacement, Game::end_storm_phase);
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

        let mut tech_token_sequence = PlayerSequence::new_game(game);

        while !remaining_tech_tokens.is_empty() && game.players.iter().any(|p| p.tech_tokens.is_empty()) {
            let current = tech_token_sequence.current_player(game).expect("current player");
            if game.player(current).tech_tokens.is_empty() {
                let token = remaining_tech_tokens.draw();
                // CurrentPlayer is re-read in C#; nothing in between changes the sequence.
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

    fn event_type(&self) -> GameEventType {
        GameEventType::StormDialled
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::StormDialled(self)
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
