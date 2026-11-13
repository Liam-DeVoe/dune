use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct LoserConcluded {
    pub karma_forced_kept_card_decision: i32,
    #[canon(name = "_forcedKeptOrDiscardedCardIds")]
    pub forced_kept_or_discarded_card_ids: Option<String>,
    pub assassinate: bool,
    #[canon(name = "_keptCardId")]
    pub kept_card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for LoserConcluded {
    fn default() -> Self {
        LoserConcluded {
            karma_forced_kept_card_decision: Default::default(),
            forced_kept_or_discarded_card_ids: Default::default(),
            assassinate: Default::default(),
            kept_card_id: Default::default(),
            base: Default::default(),
        }
    }
}

impl LoserConcluded {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub const KARMA_NO: i32 = 0;

    pub const KARMA_KEEP: i32 = 1;

    pub const KARMA_DISCARD: i32 = 2;

    pub fn forced_kept_or_discarded_cards(&self) -> Vec<TreacheryCard> {
        GameEvent::id_string_to_objects(self.forced_kept_or_discarded_card_ids.as_deref(), &TreacheryCardManager::lookup())
    }

    pub fn set_forced_kept_or_discarded_cards(&mut self, value: &[TreacheryCard]) {
        self.forced_kept_or_discarded_card_ids = Some(GameEvent::objects_to_id_string(value, &TreacheryCardManager::lookup()));
    }

    pub fn kept_card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::get(self.kept_card_id)
    }

    pub fn set_kept_card(&mut self, value: Option<TreacheryCard>) {
        self.kept_card_id = TreacheryCardManager::get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.assassinate && !LoserConcluded::can_assassinate(game, self.player()) {
            return Some(express!("You can't assassinate"));
        }
        if let Some(kept_card) = self.kept_card() {
            if !LoserConcluded::cards_loser_may_keep(game).contains(&kept_card) {
                return Some(express!("You can't keep ", kept_card));
            }
        }
        let karma_used = self.karma_forced_kept_card_decision == LoserConcluded::KARMA_KEEP
            || self.karma_forced_kept_card_decision == LoserConcluded::KARMA_DISCARD;
        if karma_used && !LoserConcluded::may_use_karma_to_force_kept_cards(game, self.player()) {
            return Some(express!("You can't use ", TreacheryCardType::Karma, " to force keeping or discarding cards"));
        }
        // C# parity: KeptCard is a stray extra argument in the next two messages (hazards 18.2).
        if !(self.karma_forced_kept_card_decision == LoserConcluded::KARMA_NO || karma_used) {
            return Some(express!("Invalid decision", self.kept_card()));
        }
        if !karma_used && !self.forced_kept_or_discarded_cards().is_empty() {
            return Some(express!("You need to decide to either force keeping or discarding cards", self.kept_card()));
        }

        None
    }

    pub fn can_assassinate(g: &Game, p: PlayerIndex) -> bool {
        g.loser_may_try_to_assassinate && LoserConcluded::target_of_assassination(g, p).is_some()
    }

    pub fn target_of_assassination(g: &Game, p: PlayerIndex) -> Option<Leader> {
        let pl = g.player(p);
        pl.traitors
            .iter()
            .copied()
            .find(|l| {
                l.as_leader().is_some()
                    && Some(*l) != g.winner_hero()
                    && (Some(l.faction()) == g.battle_winner || (g.battle_winner == Some(Faction::Tleilaxu) && g.is_ghola(*l)))
                    && !pl.revealed_traitors.contains(l)
            })
            .and_then(|l| l.as_leader())
    }

    /// `winner` can be null; it is only dereferenced when `target` is alive.
    pub fn assassination_reward(game: &Game, winner: Option<PlayerIndex>, target: Leader) -> Option<i32> {
        if !game.is_alive(Hero::Leader(target)) || !game.player(winner.expect("winner")).leaders.contains(&target) {
            return None;
        }
        if target.hero_type() == HeroType::VariableValue {
            Some(3)
        } else {
            Some(target.cost_to_revive())
        }
    }

    pub fn cards_loser_may_keep(g: &Game) -> Vec<TreacheryCard> {
        g.cards_to_be_discarded_by_loser_after_battle.clone()
    }

    pub fn is_applicable(g: &Game, p: Option<PlayerIndex>) -> bool {
        let Some(p) = p else { return false };
        Some(g.player(p).faction) == g.battle_loser
            && (!g.cards_to_be_discarded_by_loser_after_battle.is_empty() || g.loser_may_try_to_assassinate)
    }

    pub fn may_use_karma_to_force_kept_cards(game: &Game, player: PlayerIndex) -> bool {
        let pl = game.player(player);
        pl.faction == Faction::Moritani
            && !game.karma_prevented(pl.faction)
            && !pl.special_karma_power_used
            && pl.has_karma(game)
            && game.current_phase == Phase::BattleConclusion
            && game.battle_loser == Some(pl.faction)
            && game.applicable(Rule::AdvancedKarama)
    }

    pub fn cards_that_may_be_kept_or_discarded(game: &Game) -> Vec<TreacheryCard> {
        let (Some(current_battle), Some(battle_winner)) = (&game.current_battle, game.battle_winner) else {
            return Vec::new();
        };

        let mut result: Vec<TreacheryCard> = Vec::new();
        let winner_plan = current_battle.plan_of_faction(game, battle_winner).expect("plan of faction");

        if let Some(weapon) = winner_plan.weapon() {
            result.push(weapon);
        }
        if let Some(defense) = winner_plan.defense() {
            result.push(defense);
        }
        if let Some(cheap_hero) = winner_plan.hero().and_then(|h| h.as_treachery_card()) {
            result.push(cheap_hero);
        }

        result
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        if let Some(kept_card) = self.kept_card() {
            if game.secret_ally_allows_keeping_cards_after_losing_battle {
                game.secret_ally_allows_keeping_cards_after_losing_battle = false;
                game.play_nexus_card(self.player(), vec!["keep ".into(), kept_card.into()]);
            } else {
                self.log_objects(game, vec![self.initiator().into(), " keep ".into(), kept_card.into()]);
            }
        }

        // The lazy Where reads a list that Discard doesn't change.
        let kept_card = self.kept_card();
        let to_discard: Vec<TreacheryCard> =
            game.cards_to_be_discarded_by_loser_after_battle.iter().copied().filter(|c| Some(*c) != kept_card).collect();
        for c in to_discard {
            game.discard_treachery_card_boolean(Some(c), true)?;
        }

        game.cards_to_be_discarded_by_loser_after_battle.clear();

        self.log(game);

        let winner = self.get_player(game, game.battle_winner);

        if self.karma_forced_kept_card_decision == LoserConcluded::KARMA_DISCARD
            || self.karma_forced_kept_card_decision == LoserConcluded::KARMA_KEEP
        {
            game.battle_winner_may_choose_to_discard = false;
            game.discard(self.player(), TreacheryCardType::Karma)?;
            game.stone(Milestone::Karma);

            for c in self.forced_kept_or_discarded_cards() {
                if self.karma_forced_kept_card_decision == LoserConcluded::KARMA_DISCARD {
                    let winner_faction = game.player(winner.required("winner")?).faction;
                    self.log_objects(
                        game,
                        vec![
                            "Using ".into(),
                            TreacheryCardType::Karma.into(),
                            ", ".into(),
                            self.initiator().into(),
                            " force ".into(),
                            winner_faction.into(),
                            " to discard ".into(),
                            c.into(),
                        ],
                    );
                    if game.player(winner.required("winner")?).has(Some(c)) {
                        game.discard_treachery_card_boolean(Some(c), true)?;
                    }
                } else if self.karma_forced_kept_card_decision == LoserConcluded::KARMA_KEEP {
                    let winner_faction = game.player(winner.required("winner")?).faction;
                    self.log_objects(
                        game,
                        vec![
                            "Using ".into(),
                            TreacheryCardType::Karma.into(),
                            ", ".into(),
                            self.initiator().into(),
                            " force ".into(),
                            winner_faction.into(),
                            " to keep ".into(),
                            c.into(),
                        ],
                    );
                    let pile = game.treachery_discard_pile.as_mut().required("treachery discard pile")?;
                    if pile.items.contains(&c) {
                        if let Some(pos) = pile.items.iter().position(|x| *x == c) {
                            pile.items.remove(pos);
                        }
                        game.player_mut(winner.required("winner")?).treachery_cards.push(c);
                    }
                }
            }
        }

        if self.assassinate {
            game.stone(Milestone::Assassination);

            let assassinated = LoserConcluded::target_of_assassination(game, self.player());

            let assassinated = assassinated.required("a target of the assassination")?;
            game.assassinated.push(assassinated);
            game.player_mut(self.player()).revealed_traitors.insert(Hero::from(assassinated));

            let reward = LoserConcluded::assassination_reward(game, winner, assassinated);
            match reward {
                None => {
                    self.log_objects(
                        game,
                        vec![
                            self.initiator().into(),
                            " reveal ".into(),
                            assassinated.into(),
                            " as their assassination target, but don't collect spice as this leader is in the tanks.".into(),
                        ],
                    );
                }
                Some(reward) => {
                    self.log_objects(
                        game,
                        vec![
                            self.initiator().into(),
                            " get ".into(),
                            Payment::of(reward).into(),
                            " by ASSASSINATING ".into(),
                            assassinated.into(),
                            "!".into(),
                        ],
                    );
                    let pl = game.player_mut(self.player());
                    pl.resources += reward;
                    game.kill_hero(Hero::Leader(assassinated));
                }
            }
        }

        game.loser_may_try_to_assassinate = false;

        if game.battle_was_concluded_by_winner {
            let condition1 = !self.is_playing(game, Faction::Tleilaxu) || game.battle_winner == Some(Faction::Tleilaxu);
            if condition1 {
                Game::finish_battle(game)?;
            } else {
                game.enter(Phase::RevealingFacedancer, true);
            }
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " conclude the battle")
    }

}

impl GameEventMethods for LoserConcluded {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::LoserConcluded
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::LoserConcluded(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        LoserConcluded::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        LoserConcluded::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        LoserConcluded::get_message(self, game)
    }

}
