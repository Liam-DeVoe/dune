use crate::prelude::*;
use crate::compat::lazy::LazyQuery;

#[derive(Clone, Debug, Canon)]
pub struct FactionTradeOffered {
    pub target: Faction,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for FactionTradeOffered {
    fn default() -> Self {
        FactionTradeOffered {
            target: Default::default(),
            base: Default::default(),
        }
    }
}

impl FactionTradeOffered {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        // `p.Faction` is read inside the lazy predicate, so a null Player throws only once the union has an element.
        if !Self::valid_targets(game, self.player_opt().unwrap_or(PlayerIndex::NULL)).cs_contains(&self.target) {
            return Some(express!("Invalid target"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_targets(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Faction> {
        let factions: Vec<Faction> = g.factions_in_play.borrow().clone();
        // `p.Faction` is read inside the predicate, per element, as in C#.
        crate::compat::lazy::union(factions.into_iter(), g.players.iter().map(|p| p.faction)).filter(move |f| *f != g.player(p).faction)
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        if !self.is_playing(game, self.target) {
            self.log_objects(game, vec![self.initiator().into(), " switch to ".into(), self.target.into()]);

            game.player_mut(self.player()).faction = self.target;
        } else {
            let initiator = self.initiator();
            let target_faction = self.target;
            let r#match = game
                .current_trade_offers
                .iter()
                .cs_single_or_default_where(|m| m.initiator() == target_faction && m.target == initiator)
                .cloned();
            if let Some(r#match) = r#match {
                self.log_objects(
                    game,
                    vec![self.initiator().into(), " and ".into(), r#match.initiator().into(), " traded factions".into()],
                );
                let target = self.get_player(game, Some(self.target));
                // (target.Faction, Player.Faction) = (Player.Faction, target.Faction);
                let me = self.player();
                let my_faction = game.player(me).faction;
                let target = target.required("target")?;
                let target_faction = game.player(target).faction;
                game.player_mut(target).faction = my_faction;
                game.player_mut(me).faction = target_faction;
                self.remove_invalid_trade_offers(game);
            } else {
                let message = self.get_message(game);
                self.log_objects(game, vec![message.into()]);
                if !game.current_trade_offers.iter().any(|o| o.initiator() == initiator && o.target == target_faction) {
                    game.current_trade_offers.push(self.clone());
                }
            }
        }
        Ok(())
    }

    pub fn remove_invalid_trade_offers(&self, game: &mut Game) {
        let initiator = self.initiator();
        let target = self.target;
        let invalid_offers: Vec<Rc<FactionTradeOffered>> = game
            .current_trade_offers
            .iter()
            .filter(|x| x.initiator() == initiator || x.initiator() == target)
            .cloned()
            .collect();
        for invalid_offer in invalid_offers {
            // List.Remove: the first element that is the same object.
            if let Some(i) = game.current_trade_offers.iter().position(|o| Rc::ptr_eq(o, &invalid_offer)) {
                game.current_trade_offers.remove(i);
            }
        }
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(self.initiator(), " offer to trade factions with ", self.target)
    }

}

impl GameEventMethods for FactionTradeOffered {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::FactionTradeOffered
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::FactionTradeOffered(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        FactionTradeOffered::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        FactionTradeOffered::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        FactionTradeOffered::get_message(self, game)
    }

}
