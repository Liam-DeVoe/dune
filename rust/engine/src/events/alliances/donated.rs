use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
pub struct Donated {
    pub target: Faction,
    pub resources: i32,
    pub from_bank: bool,
    #[canon(name = "_cardId")]
    pub card_id: i32,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for Donated {
    fn default() -> Self {
        Donated {
            target: Default::default(),
            resources: Default::default(),
            from_bank: false,
            card_id: -1,
            base: Default::default(),
        }
    }
}

impl Donated {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn card(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::lookup().find(self.card_id)
    }

    pub fn set_card(&mut self, value: Option<TreacheryCard>) {
        self.card_id = TreacheryCardManager::lookup().get_id(value);
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.from_bank {
            return None;
        }

        // MayDonate(Game, Player) only dereferences a null Player inside HasBidToPay, where C#
        // threw NullReferenceException.
        let may_donate = match self.player_opt() {
            Some(p) => Donated::may_donate(game, p),
            None => {
                if game.current_phase == Phase::Bidding && game.current_bid.is_some() {
                    return Some(express!("You are not playing"));
                }
                true
            }
        };
        if !may_donate {
            return Some(express!("You currently have an outstanding bid"));
        }
        if self.card().is_none() && self.resources <= 0 {
            return Some(express!("Invalid amount"));
        }
        if game.player(self.player()).resources < self.resources {
            return Some(express!("You can't give that much"));
        }
        if self.initiator() != Faction::Emperor && self.target == game.player(self.player()).ally {
            return Some(express!("You can't bribe your ally"));
        }
        if self.initiator() != Faction::Emperor && game.applicable(Rule::DisableResourceTransfers) {
            return Some(express!(Concept::Resource, " transfers are disabled by house rule"));
        }
        if !self.from_bank
            && (game.economics_status == ChoamEconomicsStatus::Double || game.economics_status == ChoamEconomicsStatus::DoubleFlipped)
        {
            // C# parity: the message is built but never returned, so this check has no effect.
            let _ = express!("No bribes can be made during Double Inflation");
        }

        let target_player = game.get_player(Some(self.target));
        if self.card().is_some() && !game.player(target_player.expect("target player")).has_room_for_cards(game) {
            return Some(express!("Target faction's hand is full"));
        }

        None
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_targets(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Faction> {
        // `p` is dereferenced inside the predicate, per element, as in C#.
        g.players
            .iter()
            .filter(move |x| x.faction != g.player(p).faction && (g.player(p).is(Faction::Emperor) || x.faction != g.player(p).ally))
            .map(move |x| x.faction)
    }

    pub fn may_donate(g: &Game, p: PlayerIndex) -> bool {
        if g.has_bid_to_pay(p) {
            return false;
        }

        true
    }

    pub fn min_amount(g: &Game, p: PlayerIndex) -> i32 {
        if !g.applicable(Rule::CardsCanBeTraded) {
            return g.player(p).resources.min(1);
        }
        0
    }

    pub fn max_amount(game: &Game, p: PlayerIndex) -> i32 {
        game.player(p).resources
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        // `GetPlayer(Target)!` does not throw; a null target throws where it is dereferenced.
        let target = self.get_player(game, Some(self.target));

        if !self.from_bank {
            match target {
                Some(target) => game.exchange_resources_in_bribe(self.player(), target, self.resources),
                None => {
                    // ExchangeResourcesInBribe(Player, null, Resources): the payer pays first.
                    let me = self.player();
                    let from = game.player_mut(me);
                    from.resources -= self.resources;
                    let _ = game.bribes_during_mentat();
                    game_error!("A required value is missing");
                }
            }
            let target = target.required("target")?;

            if let Some(card) = self.card() {
                let me = self.player();
                {
                    let cards = &mut game.player_mut(me).treachery_cards;
                    if let Some(i) = cards.iter().position(|c| *c == card) {
                        cards.remove(i);
                    }
                }
                Game::register_known_player_treachery_card(game, me, Some(card));
                game.player_mut(target).treachery_cards.push(card);

                let initiator = self.initiator();
                for i in 0..game.players.len() {
                    let p = game.player_at(i);
                    if !game.player(p).is(initiator) && p != target {
                        let my_cards = game.player(me).treachery_cards.clone();
                        Game::unregister_known_player_i_enumerable_treachery_card(game, p, &my_cards);
                        let target_cards = game.player(target).treachery_cards.clone();
                        Game::unregister_known_player_i_enumerable_treachery_card(game, p, &target_cards);
                    }
                }
            }

            let m = self.get_message(game).with_log(AsideLog::aside());
            self.log_objects(game, vec![m.into()]);
            game.stone(Milestone::Bribe);
        } else if self.resources < 0 {
            let abs = self.resources.abs();
            let resources_to_take = abs.min(game.player(target.required("target")?).resources);
            let target = target.required("target")?;
            self.log_objects(
                game,
                vec![
                    "Host puts ".into(),
                    Payment::of(resources_to_take).into(),
                    " from ".into(),
                    self.target.into(),
                    " into the ".into(),
                    Concept::Resource.into(),
                    " Bank".into(),
                ],
            );
            let t = game.player_mut(target);
            t.resources -= resources_to_take;
        } else {
            self.log_objects(
                game,
                vec![
                    "Host gives ".into(),
                    self.target.into(),
                    Payment::of(self.resources).into(),
                    " from the ".into(),
                    Concept::Resource.into(),
                    " Bank".into(),
                ],
            );
            let t = game.player_mut(target.required("target")?);
            t.resources += self.resources;
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        express!(
            self.initiator(),
            " give ",
            Payment::of(self.resources),
            express_part_if!(self.card().is_some(); " and a card"),
            " to ",
            self.target
        )
    }

}

impl GameEventMethods for Donated {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::Donated
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::Donated(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        Donated::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        Donated::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        Donated::get_message(self, game)
    }

}
