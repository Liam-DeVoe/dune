use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct TerrorRevealed {
    pub alliance_offered: bool,
    #[serde(rename = "terrorType")]
    pub type_: Option<TerrorType>,
    pub forces_in_sneak_attack: u16,
    pub sneak_attack_to: Option<Location>,
    pub robbery_takes_card: bool,
    pub card_to_give_in_sabotage: Option<TreacheryCard>,
    #[serde(skip)]
    pub passed: bool,
}

impl Default for TerrorRevealed {
    fn default() -> Self {
        TerrorRevealed {
            alliance_offered: Default::default(),
            type_: Default::default(),
            forces_in_sneak_attack: Default::default(),
            sneak_attack_to: Location::from_id(0),
            robbery_takes_card: Default::default(),
            card_to_give_in_sabotage: TreacheryCardManager::get(0),
            passed: Default::default(),
        }
    }
}

impl TerrorRevealed {
    pub fn to(&self, _game: &Game) -> Option<Location> {
        self.sneak_attack_to
    }

    pub fn total_amount_of_forces_added_to_location(&self, _game: &Game) -> i32 {
        i32::from(self.forces_in_sneak_attack)
    }

    pub fn forces_added_to_location(&self, _game: &Game) -> i32 {
        i32::from(self.forces_in_sneak_attack)
    }

    pub fn special_forces_added_to_location(&self, _game: &Game) -> i32 {
        0
    }

    pub fn may_pass(g: &Game) -> bool {
        !g.ship_move.alliance_by_terror_was_offered
    }

    pub fn may_offer_alliance(g: &Game) -> bool {
        let victim = g.get_player(TerrorRevealed::get_victim(g));
        let moritani = g.get_player(Some(Faction::Moritani));

        // Without a victim or a Moritani player there is nobody to ally with.
        let (Some(victim), Some(moritani)) = (victim, moritani) else { return false };
        !g.ship_move.alliance_by_terror_was_offered
            && !g.prevented(FactionAdvantage::MoritaniEnemyOfEnemy)
            && !g.player(victim).is(Faction::Ecaz)
            && !g.player(moritani).have_forces_on_each_others_home_worlds(g, victim)
    }

    pub fn get_territory(g: &Game) -> Option<Territory> {
        g.last_terror_trigger().map(|t| t.territory(g))
    }

    pub fn get_victim(g: &Game) -> Option<Faction> {
        match g.last_terror_trigger() {
            Some(t) => Some(t.initiator()),
            None => None,
        }
    }

    /// `Game.TerrorIn(t)` where `t` may be null: no token is ever in a null territory.
    fn terror_in(g: &Game, t: Option<Territory>) -> impl Iterator<Item = TerrorType> {
        use either::Either;
        use std::iter::empty;
        match t {
            Some(t) => Either::Left(g.terror_in(t)),
            None => Either::Right(empty()),
        }
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn get_types(g: &Game) -> impl Iterator<Item = TerrorType> {
        use either::Either;
        use std::iter::empty;
        if g.last_terror_trigger().is_some() {
            Either::Left(TerrorRevealed::terror_in(g, TerrorRevealed::get_territory(g)))
        } else {
            Either::Right(empty())
        }
    }

    pub fn max_amount_of_forces_in_sneak_attack(game: &Game, p: PlayerId) -> i32 {
        std::cmp::min(5, game.player(p).forces_in_reserve())
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_sneak_attack_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Location> {
        TerrorRevealed::get_territory(g)
            .expect("territory")
            .locations()
            .iter()
            .copied()
            .filter(move |l| TerrorRevealed::open_despite_ally_and_storm_and_occupancy(g, p, *l))
    }

    pub fn open_despite_ally_and_storm_and_occupancy(g: &Game, p: PlayerId, l: Location) -> bool {
        let pl = g.player(p);
        g.is_not_full(p, l)
            && !Sector::in_storm(l.sector(&g.board.map), g.board.sector_in_storm)
            && (!pl.has_ally()
                || g.player(pl.allied_player(g).expect("allied player")).any_forces_in_territory(Some(l.territory())) == 0
                || pl.ally == Some(Faction::Ecaz)
                || (pl.ally == Some(Faction::BeneGesserit)
                    && g.applicable(Rule::AdvisorsDontConflictWithAlly)
                    && g.player(pl.allied_player(g).expect("allied player")).forces_in_territory(Some(l.territory())) == 0))
    }
}

impl EventRules for TerrorRevealed {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if self.passed && !TerrorRevealed::may_pass(game) {
            return Err(Rejection(express!("You must reveal a terror token")));
        }
        if self.alliance_offered && !TerrorRevealed::may_offer_alliance(game) {
            return Err(Rejection(express!("You can't offer an alliance to this faction")));
        }

        if self.passed || self.alliance_offered {
            return Ok(());
        }
        if !TerrorRevealed::get_types(game).any(|v| Some(v) == self.type_) {
            return Err(Rejection(express!("You cannot use this token")));
        }
        if by.faction != Some(Faction::Moritani) {
            return Err(Rejection(express!("Your faction can't reveal terror tokens")));
        }

        if self.type_ == Some(TerrorType::SneakAttack) && self.sneak_attack_to.is_none() && i32::from(self.forces_in_sneak_attack) > 0 {
            return Err(Rejection(express!("You cannot send forces there")));
        }
        if self.type_ == Some(TerrorType::SneakAttack)
            && self.sneak_attack_to.is_some()
            && !TerrorRevealed::valid_sneak_attack_targets(game, by.player()).any(|v| v == self.sneak_attack_to.expect("sneak attack to"))
        {
            return Err(Rejection(express!("Invalid location of sneak attack")));
        }
        if self.type_ == Some(TerrorType::SneakAttack)
            && i32::from(self.forces_in_sneak_attack) > TerrorRevealed::max_amount_of_forces_in_sneak_attack(game, by.player())
        {
            return Err(Rejection(express!("Too many forces selected")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let territory = TerrorRevealed::get_territory(game);
        let victim = TerrorRevealed::get_victim(game);
        let victim_player = game.get_player(victim);

        if self.passed {
            game.log(parts![by.faction, " don't terrorize ", territory]);
            game.ship_move.alliance_by_terror_was_offered = false;
            game.dequeue_intrusion(IntrusionType::Terror)?;
            game.determine_next_shipment_and_move_sub_phase()?;
        } else if self.alliance_offered {
            game.log(parts![by.faction, " offer an alliance to ", victim, " as an alternative to terror"]);
            game.ship_move.alliance_by_terror_offered_to = victim;
            game.ship_move.alliance_by_terror_was_offered = true;
            game.ship_move.paused_terror_phase = Some(game.turn.current_phase);
            game.enter(Phase::AllianceByTerror, true);
        } else {
            game.stone(Milestone::TerrorRevealed);

            if let Some(type_) = self.type_ {
                game.board.terror_on_planet.shift_remove(&type_);
            }

            // C# parity: Passed is always false here.
            if self.passed || TerrorRevealed::terror_in(game, territory).next().is_none() {
                game.dequeue_intrusion(IntrusionType::Terror)?;
            }

            game.ship_move.alliance_by_terror_was_offered = false;

            match self.type_ {
                Some(TerrorType::Assassination) => {
                    let victim_player = victim_player.required("victim player")?;
                    let random_leader = game
                        .player(victim_player)
                        .leaders
                        .iter()
                        .copied()
                        .filter(|l| game.is_alive(Hero::Leader(*l)))
                        .collect::<Vec<_>>().into_iter().random_item(&mut game.rng);
                    if let Some(random_leader) = random_leader {
                        let price = if random_leader.hero_type() == HeroType::VariableValue {
                            3
                        } else {
                            random_leader.cost_to_revive()
                        };
                        game.log(parts![by.faction, " gain ", Payment::of(price), " from assassinating ", random_leader, " in ", territory]);
                        let p = game.player_mut(by.player());
                        crate::game::amounts::gain(&mut p.resources, price, "spice")?;
                        game.kill_hero(Hero::Leader(random_leader));
                    } else {
                        game.log(parts![by.faction, " fail to assassinate a leader in ", territory]);
                    }
                }

                Some(TerrorType::Atomics) => {
                    game.kill_all_forces_in(territory.required("territory")?, false)?;
                    game.kill_ambassador_in(territory.required("territory")?);
                    game.board.atomics_aftermath = territory;

                    while game.player(by.player()).hand_size_exceeded(game) {
                        let card = game.player(by.player()).treachery_cards.iter().copied().collect::<Vec<_>>().into_iter().random_item(&mut game.rng);
                        game.discard_card_of(Some(by.player()), card, true);
                    }

                    if game.player(by.player()).has_ally() {
                        while game.player(game.player(by.player()).allied_player(game).required("allied player")?).hand_size_exceeded(game) {
                            let ally = game.player(by.player()).allied_player(game).required("allied player")?;
                            let ally2 = game.player(by.player()).allied_player(game).required("allied player")?;
                            let card = game.player(ally2).treachery_cards.iter().copied().collect::<Vec<_>>().into_iter().random_item(&mut game.rng);
                            game.discard_card_of(Some(ally), card, true);
                        }
                    }

                    // C# parity: Atomics records the Metheor milestone.
                    game.stone(Milestone::MetheorUsed);
                    game.log(parts![by.faction, " DETONATE ATOMICS in ", territory]);
                }

                Some(TerrorType::Extortion) => {
                    game.log(parts![by.faction, " will get ", Payment::of(5), " from ", self.type_, " during ", MainPhase::Contemplate]);
                    let p = game.player_mut(by.player());
                    p.extortion += 5;
                }

                Some(TerrorType::Robbery) => {
                    if self.robbery_takes_card {
                        let card = game.draw_treachery_card();
                        game.player_mut(by.player()).treachery_cards.push(card.required("card")?);
                        game.log(parts![by.faction, " draw a Treachery Card"]);
                    } else {
                        let victim_player = victim_player.required("victim player")?;
                        // Half, rounded up.
                        let amount_stolen = game.player(victim_player).resources.div_ceil(2);
                        let v = game.player_mut(victim_player);
                        v.resources -= amount_stolen;
                        let amount_stolen = i32::from(amount_stolen);
                        let p = game.player_mut(by.player());
                        crate::game::amounts::gain(&mut p.resources, amount_stolen, "spice")?;
                        game.log(parts![by.faction, " steal ", Payment::of(amount_stolen), " by ", self.type_, " in ", territory]);
                    }
                }

                Some(TerrorType::Sabotage) => {
                    let victim_player = victim_player.required("victim player")?;
                    let victim_faction = game.player(victim_player).faction;
                    game.log(parts![by.faction, " sabotage ", victim_faction]);

                    if !game.player(victim_player).treachery_cards.is_empty() {
                        let card = game
                            .player(victim_player)
                            .treachery_cards
                            .iter()
                            .copied()
                            .collect::<Vec<_>>().into_iter().random_item(&mut game.rng);
                        game.discard_card(card, true)?;
                    } else {
                        game.log(parts![victim_faction, " have no treachery cards to discard"]);
                    }

                    if let Some(card) = self.card_to_give_in_sabotage {
                        // List.Remove: the first equal element
                        let cards = &mut game.player_mut(by.player()).treachery_cards;
                        if let Some(pos) = cards.iter().position(|c| *c == card) {
                            cards.remove(pos);
                        }
                        game.player_mut(victim_player).treachery_cards.push(card);
                        let victim_faction = game.player(victim_player).faction;
                        game.log(parts![by.faction, " give a treachery card to ", victim_faction]);
                    }
                }

                Some(TerrorType::SneakAttack) => {
                    if let Some(sneak_attack_to) = self.sneak_attack_to {
                        let force = game.player(by.player()).force();
                        game.log(parts![by.faction, " sneak attack ", territory, " with ", i32::from(self.forces_in_sneak_attack), force]);
                        Player::ship_forces(game, by.player(), sneak_attack_to, i32::from(self.forces_in_sneak_attack))?;
                        game.check_intrusion(&Event::from(self.clone()), by.faction.expect("faction"))?;
                    }
                }

                _ => {}
            }

            game.determine_next_shipment_and_move_sub_phase()?;
        }

        // C# parity: also runs when an alliance was offered (Passed is false then).
        if !self.passed && self.type_ == Some(TerrorType::Robbery) && self.robbery_takes_card {
            let me = by.player();
            if game.player(me).treachery_cards.len() as i32 > game.player(me).maximum_number_of_cards(game) {
                game.let_player_discard_treachery_card_of_choice(by.faction.required("faction")?);
            }
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        if self.alliance_offered {
            return express!(by.faction, " offer an alliance instead of terror");
        }
        if self.passed {
            return express!(by.faction, " don't terrorize");
        }
        express!(by.faction, " reveal a terror token")
    }
}

impl crate::events::location_event::LocationEvent for TerrorRevealed {
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
