use crate::prelude::*;
use crate::compat::lazy::LazyQuery;
use crate::compat::random_ext::RandomExt;

#[derive(Clone, Debug, Canon)]
pub struct TerrorRevealed {
    pub alliance_offered: bool,
    pub type_: TerrorType,
    pub forces_in_sneak_attack: i32,
    #[canon(name = "_sneakAttackTo")]
    pub sneak_attack_to: i32,
    pub robbery_takes_card: bool,
    #[canon(name = "_cardToGiveInSabotageId")]
    pub card_to_give_in_sabotage_id: i32,
    // ---- inherited from PassableGameEvent ----
    pub passed: bool,
    /// The fields of `GameEvent` (Initiator, Time, Game, Player).
    #[canon(flatten)]
    pub base: GameEventBase,
}

impl Default for TerrorRevealed {
    fn default() -> Self {
        TerrorRevealed {
            alliance_offered: Default::default(),
            type_: Default::default(),
            forces_in_sneak_attack: Default::default(),
            sneak_attack_to: Default::default(),
            robbery_takes_card: Default::default(),
            card_to_give_in_sabotage_id: Default::default(),
            passed: Default::default(),
            base: Default::default(),
        }
    }
}

macro_rules! elems {
    ($($e:expr),* $(,)?) => { vec![$(ExpressionElement::from($e)),*] };
}

impl TerrorRevealed {
    pub fn new(game: &Game, initiator: Faction) -> Self {
        let e = Self::default();
        e.initialize(game, initiator);
        e
    }

    pub fn sneak_attack_to(&self, game: &Game) -> Option<Location> {
        game.map.location_lookup().find(self.sneak_attack_to)
    }

    pub fn set_sneak_attack_to(&mut self, game: &Game, value: Option<Location>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.sneak_attack_to = game.map.location_lookup().get_id(value);
    }

    pub fn card_to_give_in_sabotage(&self) -> Option<TreacheryCard> {
        TreacheryCardManager::get(self.card_to_give_in_sabotage_id)
    }

    pub fn set_card_to_give_in_sabotage(&mut self, value: Option<TreacheryCard>) {
        let value = value.and_then(crate::compat::cs::CsNull::opt);
        self.card_to_give_in_sabotage_id = TreacheryCardManager::get_id(value);
    }

    pub fn to(&self, game: &Game) -> Option<Location> {
        self.sneak_attack_to(game)
    }

    pub fn total_amount_of_forces_added_to_location(&self, _game: &Game) -> i32 {
        self.forces_in_sneak_attack
    }

    pub fn forces_added_to_location(&self, _game: &Game) -> i32 {
        self.forces_in_sneak_attack
    }

    pub fn special_forces_added_to_location(&self, _game: &Game) -> i32 {
        0
    }

    pub fn validate(&self, game: &Game) -> Option<Message> {
        if self.passed && !TerrorRevealed::may_pass(game) {
            return Some(express!("You must reveal a terror token"));
        }
        if self.alliance_offered && !TerrorRevealed::may_offer_alliance(game) {
            return Some(express!("You can't offer an alliance to this faction"));
        }

        if self.passed || self.alliance_offered {
            return None;
        }
        if !TerrorRevealed::get_types(game).cs_contains(&self.type_) {
            return Some(express!("You cannot use this token"));
        }
        if self.initiator() != Faction::Moritani {
            return Some(express!("Your faction can't reveal terror tokens"));
        }

        if self.type_ == TerrorType::SneakAttack && self.sneak_attack_to(game).is_none() && self.forces_in_sneak_attack > 0 {
            return Some(express!("You cannot send forces there"));
        }
        if self.type_ == TerrorType::SneakAttack
            && self.sneak_attack_to(game).is_some()
            && !TerrorRevealed::valid_sneak_attack_targets(game, self.player()).cs_contains(&self.sneak_attack_to(game).nre())
        {
            return Some(express!("Invalid location of sneak attack"));
        }
        if self.type_ == TerrorType::SneakAttack
            && self.forces_in_sneak_attack > TerrorRevealed::max_amount_of_forces_in_sneak_attack(game, self.player())
        {
            return Some(express!("Too many forces selected"));
        }

        None
    }

    pub fn may_pass(g: &Game) -> bool {
        !g.alliance_by_terror_was_offered
    }

    pub fn may_offer_alliance(g: &Game) -> bool {
        let victim = g.get_player(Some(TerrorRevealed::get_victim(g)));
        let moritani = g.get_player(Some(Faction::Moritani));

        // Without a victim or a Moritani player there is nobody to ally with.
        let (Some(victim), Some(moritani)) = (victim, moritani) else { return false };
        !g.alliance_by_terror_was_offered
            && !g.prevented(FactionAdvantage::MoritaniEnemyOfEnemy)
            && !g.player(victim).is(Faction::Ecaz)
            && !g.player(moritani).have_forces_on_each_others_home_worlds(g, victim)
    }

    pub fn get_territory(g: &Game) -> Option<Territory> {
        g.last_terror_trigger().map(|t| t.territory(g))
    }

    pub fn get_victim(g: &Game) -> Faction {
        match g.last_terror_trigger() {
            Some(t) => t.initiator(),
            None => Faction::None,
        }
    }

    /// `Game.TerrorIn(t)` where `t` may be null: no token is ever in a null territory.
    fn terror_in(g: &Game, t: Option<Territory>) -> impl Iterator<Item = TerrorType> {
        use crate::compat::lazy::{Either, empty};
        let t = t.and_then(crate::compat::cs::CsNull::opt);
        match t {
            Some(t) => Either::A(g.terror_in(t)),
            None => Either::B(empty()),
        }
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn get_types(g: &Game) -> impl Iterator<Item = TerrorType> {
        use crate::compat::lazy::{Either, empty};
        if g.last_terror_trigger().is_some() {
            Either::A(TerrorRevealed::terror_in(g, TerrorRevealed::get_territory(g)))
        } else {
            Either::B(empty())
        }
    }

    pub fn max_amount_of_forces_in_sneak_attack(game: &Game, p: PlayerIndex) -> i32 {
        std::cmp::min(5, game.player(p).forces_in_reserve())
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach (docs/lazy-queries.md).
    pub fn valid_sneak_attack_targets(g: &Game, p: PlayerIndex) -> impl Iterator<Item = Location> {
        TerrorRevealed::get_territory(g)
            .nre()
            .locations()
            .iter()
            .copied()
            .filter(move |l| TerrorRevealed::open_despite_ally_and_storm_and_occupancy(g, p, *l))
    }

    pub fn open_despite_ally_and_storm_and_occupancy(g: &Game, p: PlayerIndex, l: Location) -> bool {
        let pl = g.player(p);
        g.is_not_full(p, l)
            && l.sector(&g.map) != g.sector_in_storm
            && (!pl.has_ally()
                || g.player(pl.allied_player(g).nre()).any_forces_in_territory(Some(l.territory())) == 0
                || pl.ally == Faction::Ecaz
                || (pl.ally == Faction::BeneGesserit
                    && g.applicable(Rule::AdvisorsDontConflictWithAlly)
                    && g.player(pl.allied_player(g).nre()).forces_in_territory(Some(l.territory())) == 0))
    }

    pub fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        let territory = TerrorRevealed::get_territory(game);
        let victim = TerrorRevealed::get_victim(game);
        let victim_player = self.get_player(game, Some(victim));

        if self.passed {
            self.log_objects(game, elems![self.initiator(), " don't terrorize ", territory]);
            game.alliance_by_terror_was_offered = false;
            game.dequeue_intrusion(IntrusionType::Terror)?;
            game.determine_next_shipment_and_move_sub_phase()?;
        } else if self.alliance_offered {
            self.log_objects(
                game,
                elems![self.initiator(), " offer an alliance to ", victim, " as an alternative to terror"],
            );
            game.alliance_by_terror_offered_to = victim;
            game.alliance_by_terror_was_offered = true;
            game.paused_terror_phase = game.current_phase;
            game.enter(Phase::AllianceByTerror, true);
        } else {
            game.stone(Milestone::TerrorRevealed);

            game.terror_on_planet.remove(&self.type_);

            // C# parity: Passed is always false here.
            if self.passed || TerrorRevealed::terror_in(game, territory).next().is_none() {
                game.dequeue_intrusion(IntrusionType::Terror)?;
            }

            game.alliance_by_terror_was_offered = false;

            match self.type_ {
                TerrorType::Assassination => {
                    let victim_player = victim_player.required("victim player")?;
                    let random_leader = game
                        .player(victim_player)
                        .leaders
                        .iter()
                        .copied()
                        .filter(|l| game.is_alive(Hero::Leader(*l)))
                        .random_or_default(game.random.as_ref().required("random")?);
                    if let Some(random_leader) = random_leader {
                        let price = if random_leader.hero_type() == HeroType::VariableValue {
                            3
                        } else {
                            random_leader.cost_to_revive()
                        };
                        self.log_objects(
                            game,
                            elems![self.initiator(), " gain ", Payment::of(price), " from assassinating ", random_leader, " in ", territory],
                        );
                        let p = game.player_mut(self.player());
                        p.resources = p.resources.wrapping_add(price);
                        game.kill_hero(Hero::Leader(random_leader));
                    } else {
                        self.log_objects(game, elems![self.initiator(), " fail to assassinate a leader in ", territory]);
                    }
                }

                TerrorType::Atomics => {
                    // A null territory (no LastTerrorTrigger): KillAllForcesIn finds no forces,
                    // then KillAmbassadorIn throws ArgumentNullException (GetValueOrDefault(null)).
                    game.kill_all_forces_in(territory.or_null(), false);
                    game.kill_ambassador_in(territory.or_null());
                    game.atomics_aftermath = territory;

                    while game.player(self.player()).hand_size_exceeded(game) {
                        let card = game.player(self.player()).treachery_cards.iter().copied().random_or_default(game.random.as_ref().required("random")?);
                        game.discard_player_treachery_card_boolean(Some(self.player()), card, true);
                    }

                    if game.player(self.player()).has_ally() {
                        while game.player(game.player(self.player()).allied_player(game).required("allied player")?).hand_size_exceeded(game) {
                            let ally = game.player(self.player()).allied_player(game).required("allied player")?;
                            let ally2 = game.player(self.player()).allied_player(game).required("allied player")?;
                            let card = game.player(ally2).treachery_cards.iter().copied().random_or_default(game.random.as_ref().required("random")?);
                            game.discard_player_treachery_card_boolean(Some(ally), card, true);
                        }
                    }

                    // C# parity: Atomics records the Metheor milestone (hazards 18).
                    game.stone(Milestone::MetheorUsed);
                    self.log_objects(game, elems![self.initiator(), " DETONATE ATOMICS in ", territory]);
                }

                TerrorType::Extortion => {
                    self.log_objects(
                        game,
                        elems![self.initiator(), " will get ", Payment::of(5), " from ", self.type_, " during ", MainPhase::Contemplate],
                    );
                    let p = game.player_mut(self.player());
                    p.extortion = p.extortion.wrapping_add(5);
                }

                TerrorType::Robbery => {
                    if self.robbery_takes_card {
                        let card = game.draw_treachery_card();
                        // C# adds the result even when it is null (both decks empty).
                        game.player_mut(self.player()).treachery_cards.push(card.or_null());
                        self.log_objects(game, elems![self.initiator(), " draw a Treachery Card"]);
                    } else {
                        let victim_player = victim_player.required("victim player")?;
                        // (int)Math.Ceiling(0.5f * resources): the product is a float (hazards 9).
                        let amount_stolen = ((0.5f32 * game.player(victim_player).resources as f32) as f64).ceil() as i32;
                        let p = game.player_mut(self.player());
                        p.resources = p.resources.wrapping_add(amount_stolen);
                        let v = game.player_mut(victim_player);
                        v.resources = v.resources.wrapping_sub(amount_stolen);
                        self.log_objects(
                            game,
                            elems![self.initiator(), " steal ", Payment::of(amount_stolen), " by ", self.type_, " in ", territory],
                        );
                    }
                }

                TerrorType::Sabotage => {
                    let victim_player = victim_player.required("victim player")?;
                    let victim_faction = game.player(victim_player).faction;
                    self.log_objects(game, elems![self.initiator(), " sabotage ", victim_faction]);

                    if !game.player(victim_player).treachery_cards.is_empty() {
                        let card = game
                            .player(victim_player)
                            .treachery_cards
                            .iter()
                            .copied()
                            .random_or_default(game.random.as_ref().required("random")?);
                        game.discard_treachery_card_boolean(card, true)?;
                    } else {
                        self.log_objects(game, elems![victim_faction, " have no treachery cards to discard"]);
                    }

                    if let Some(card) = self.card_to_give_in_sabotage() {
                        // List.Remove: the first equal element
                        let cards = &mut game.player_mut(self.player()).treachery_cards;
                        if let Some(pos) = cards.iter().position(|c| *c == card) {
                            cards.remove(pos);
                        }
                        game.player_mut(victim_player).treachery_cards.push(card);
                        let victim_faction = game.player(victim_player).faction;
                        self.log_objects(game, elems![self.initiator(), " give a treachery card to ", victim_faction]);
                    }
                }

                TerrorType::SneakAttack => {
                    if let Some(sneak_attack_to) = self.sneak_attack_to(game) {
                        let force = game.player(self.player()).force();
                        self.log_objects(
                            game,
                            elems![self.initiator(), " sneak attack ", territory, " with ", self.forces_in_sneak_attack, force],
                        );
                        Player::ship_forces(game, self.player(), sneak_attack_to, self.forces_in_sneak_attack);
                        game.check_intrusion(&self.clone().into_event())?;
                    }
                }

                _ => {}
            }

            game.determine_next_shipment_and_move_sub_phase()?;
        }

        // C# parity: also runs when an alliance was offered (Passed is false then).
        if !self.passed && self.type_ == TerrorType::Robbery && self.robbery_takes_card {
            let me = self.player();
            if game.player(me).treachery_cards.len() as i32 > game.player(me).maximum_number_of_cards(game) {
                game.let_player_discard_treachery_card_of_choice(self.initiator());
            }
        }
        Ok(())
    }

    pub fn get_message(&self, _game: &Game) -> Message {
        if self.alliance_offered {
            return express!(self.initiator(), " offer an alliance instead of terror");
        }
        if self.passed {
            return express!(self.initiator(), " don't terrorize");
        }
        express!(self.initiator(), " reveal a terror token")
    }
}

impl GameEventMethods for TerrorRevealed {
    fn base(&self) -> &GameEventBase {
        &self.base
    }

    fn event_type(&self) -> GameEventType {
        GameEventType::TerrorRevealed
    }

    fn into_event(self: Rc<Self>) -> GameEvent {
        GameEvent::TerrorRevealed(self)
    }

    fn validate(&self, game: &Game) -> Option<Message> {
        TerrorRevealed::validate(self, game)
    }

    fn execute_concrete_event(self: &Rc<Self>, game: &mut Game) -> GameResult {
        TerrorRevealed::execute_concrete_event(self, game)
    }

    fn get_message(&self, game: &Game) -> Message {
        TerrorRevealed::get_message(self, game)
    }

}

impl PassableGameEventMethods for TerrorRevealed {
    fn field_passed(&self) -> &bool {
        &self.passed
    }

}

impl crate::events::location_event::ILocationEvent for TerrorRevealed {
    fn initiator(&self) -> Faction {
        GameEventMethods::initiator(self)
    }

    fn to(&self, game: &Game) -> Option<Location> {
        TerrorRevealed::to(self, game)
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        TerrorRevealed::total_amount_of_forces_added_to_location(self, game)
    }

    fn forces_added_to_location(&self, game: &Game) -> i32 {
        TerrorRevealed::forces_added_to_location(self, game)
    }

    fn special_forces_added_to_location(&self, game: &Game) -> i32 {
        TerrorRevealed::special_forces_added_to_location(self, game)
    }

}
