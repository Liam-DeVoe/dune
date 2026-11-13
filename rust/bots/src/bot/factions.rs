use crate::prelude::*;
use std::cell::OnceCell;

/// A location or a whole territory, where a player's forces can be counted.
trait ForcesIn: Copy {
    /// How many forces `player` has here, special forces and advisors included.
    fn forces_of(self, game: &Game, player: PlayerIndex) -> i32;

    /// The forces of all of `players` here.
    fn forces_of_all(self, game: &Game, players: &[PlayerIndex]) -> i32 {
        players.iter().map(|p| self.forces_of(game, *p)).sum()
    }
}

impl ForcesIn for Location {
    fn forces_of(self, game: &Game, player: PlayerIndex) -> i32 {
        game.player(player).any_forces_in(Some(self))
    }
}

impl ForcesIn for Territory {
    fn forces_of(self, game: &Game, player: PlayerIndex) -> i32 {
        game.player(player).any_forces_in_territory(Some(self))
    }
}

/// Whether Arrakeen or Carthag holds no forces at all.
fn arrakeen_or_carthag_is_empty(game: &Game) -> bool {
    let forces_in = |l: Location| game.battalions_in(l).iter().map(|b| b.total_amount_of_forces()).sum::<i32>();
    forces_in(game.map.arrakeen()) == 0 || forces_in(game.map.carthag()) == 0
}

/// A voice command: the opponent must use, or may not use, a card type.
#[derive(Clone, Copy)]
struct VoiceCommand {
    must: bool,
    type_: TreacheryCardType,
}

impl VoiceCommand {
    fn must_use(type_: TreacheryCardType) -> Self {
        VoiceCommand { must: true, type_ }
    }

    fn may_not_use(type_: TreacheryCardType) -> Self {
        VoiceCommand { must: false, type_ }
    }

    /// `may_not_use(type_)` if `condition` holds.
    fn may_not_use_if(condition: bool, type_: TreacheryCardType) -> Option<Self> {
        condition.then_some(Self::may_not_use(type_))
    }
}

/// What a battle opponent is known to hold, or might hold.
struct OpponentCards {
    known_defenses: Vec<TreacheryCard>,
    known_weapons: Vec<TreacheryCard>,
    has_or_might_have: Vec<TreacheryCard>,
    unknown_count: i32,
}

impl OpponentCards {
    fn new(game: &Game, player: PlayerIndex, opponent: PlayerIndex) -> Self {
        OpponentCards {
            known_defenses: ClassicBot::known_opponent_defenses(game, player, opponent),
            known_weapons: ClassicBot::known_opponent_weapons(game, player, opponent),
            unknown_count: ClassicBot::nr_of_unknown_opponent_cards(game, player, opponent),
            has_or_might_have: ClassicBot::cards_player_has_or_might_have(game, opponent),
        }
    }

    fn might_have(&self, f: impl Fn(TreacheryCard) -> bool) -> bool {
        self.has_or_might_have.iter().any(|c| f(*c))
    }

    fn known_defense(&self, f: impl Fn(&TreacheryCard) -> bool) -> Option<TreacheryCard> {
        self.known_defenses.iter().copied().find(f)
    }

    fn known_weapon(&self, f: impl Fn(&TreacheryCard) -> bool) -> Option<TreacheryCard> {
        self.known_weapons.iter().copied().find(f)
    }

    fn knows_defense(&self, f: impl Fn(&TreacheryCard) -> bool) -> bool {
        self.known_defense(f).is_some()
    }

    fn knows_weapon(&self, f: impl Fn(&TreacheryCard) -> bool) -> bool {
        self.known_weapon(f).is_some()
    }

    /// A voice that keeps the opponent from defending against `weapon`: preferably by forcing a
    /// defense that doesn't work against it, otherwise by forbidding one that does.
    fn voice_against_defenses(&self, game: &Game, weapon: TreacheryCard) -> Option<VoiceCommand> {
        use TreacheryCardType as T;
        let must_use = |d: TreacheryCard| VoiceCommand::must_use(d.type_());
        let names_special_cards = || game.applicable(Rule::BeneGesseritVoiceMustNameSpecialCards);

        if weapon.type_() == T::ProjectileAndPoison {
            self.known_defense(|d| d.type_() != T::ShieldAndAntidote)
                .map(must_use)
                .or_else(|| VoiceCommand::may_not_use_if(self.might_have(|c| c.type_() == T::ShieldAndAntidote), T::ShieldAndAntidote))
        } else if weapon.is_laser() {
            self.known_defense(|d| d.is_poison_defense() && !d.is_shield())
                .map(must_use)
                .or_else(|| VoiceCommand::may_not_use_if(self.might_have(|c| c.is_shield()), T::Shield))
        } else if weapon.is_projectile_weapon() {
            self.known_defense(|d| d.is_poison_defense() && !d.is_projectile_defense()).map(must_use).or_else(|| {
                if !self.knows_defense(|d| d.is_projectile_defense() && d.type_() != T::WeirdingWay)
                    && self.knows_defense(|d| d.type_() == T::WeirdingWay)
                    && self.knows_weapon(|d| d.is_weapon())
                    && self.might_have(|c| c.type_() == T::WeirdingWay)
                {
                    Some(VoiceCommand::may_not_use(T::WeirdingWay))
                } else if !names_special_cards() && self.might_have(|c| c.is_projectile_defense()) {
                    Some(VoiceCommand::may_not_use(T::ProjectileDefense))
                } else if self.might_have(|c| c.type_() == T::ShieldAndAntidote) {
                    Some(VoiceCommand::may_not_use(T::ShieldAndAntidote))
                } else {
                    VoiceCommand::may_not_use_if(self.might_have(|c| c.type_() == T::Shield), T::Shield)
                }
            })
        } else if weapon.is_poison_weapon() {
            // C# parity: `d.IsProjectileDefense && !d.IsProjectileDefense` is never true (hazards 18).
            self.known_defense(|d| d.is_projectile_defense() && !d.is_projectile_defense()).map(must_use).or_else(|| {
                if !names_special_cards() && self.might_have(|c| c.is_poison_defense()) {
                    Some(VoiceCommand::may_not_use(T::PoisonDefense))
                } else if self.might_have(|c| c.type_() == T::ShieldAndAntidote) {
                    Some(VoiceCommand::may_not_use(T::ShieldAndAntidote))
                } else if self.might_have(|c| c.type_() == T::Chemistry) {
                    Some(VoiceCommand::may_not_use(T::Chemistry))
                } else {
                    VoiceCommand::may_not_use_if(self.might_have(|c| c.type_() == T::Antidote), T::Antidote)
                }
            })
        } else {
            None
        }
    }

    /// A voice that keeps the opponent from getting through `defense`: preferably by forcing a
    /// weapon it stops, otherwise by forbidding one it doesn't. Also returns what the voice
    /// tells about my hero surviving, if anything.
    fn voice_against_weapons(&self, defense: TreacheryCard) -> Option<(VoiceCommand, Option<bool>)> {
        use TreacheryCardType as T;
        let forced = |w: TreacheryCard| (VoiceCommand::must_use(w.type_()), Some(true));
        let forbidden = |t: TreacheryCardType| (VoiceCommand::may_not_use(t), None);
        let forbidden_if_known = |t: TreacheryCardType| {
            self.knows_weapon(|w| w.type_() == t).then(|| (VoiceCommand::may_not_use(t), Some(self.unknown_count == 0)))
        };

        if defense.type_() == T::ShieldAndAntidote {
            self.known_weapon(|w| w.type_() != T::Laser && w.type_() != T::PoisonTooth && w.type_() != T::Rockmelter)
                .map(forced)
                .or_else(|| {
                    [T::PoisonTooth, T::Laser, T::Rockmelter]
                        .into_iter()
                        .find(|t| self.knows_weapon(|w| w.type_() == *t))
                        .map(forbidden)
                })
        } else if defense.is_projectile_defense() {
            self.known_weapon(|w| w.is_projectile_weapon() && !w.is_poison_weapon())
                .map(forced)
                .or_else(|| forbidden_if_known(T::Laser))
                .or_else(|| forbidden_if_known(T::PoisonTooth))
                .or_else(|| {
                    if !self.knows_weapon(|d| d.is_poison_weapon() && d.type_() != T::Chemistry)
                        && self.knows_weapon(|d| d.type_() == T::Chemistry)
                        && self.knows_defense(|d| d.is_defense())
                        && self.might_have(|c| c.type_() == T::Chemistry)
                    {
                        Some(forbidden(T::Chemistry))
                    } else {
                        self.might_have(|c| c.is_poison_weapon()).then(|| forbidden(T::Poison))
                    }
                })
        } else if defense.is_poison_defense() {
            self.known_weapon(|w| !w.is_projectile_weapon() && w.is_poison_weapon())
                .map(forced)
                .or_else(|| forbidden_if_known(T::Laser))
                .or_else(|| forbidden_if_known(T::PoisonTooth))
                .or_else(|| self.might_have(|c| c.is_projectile_weapon()).then(|| forbidden(T::Projectile)))
        } else {
            None
        }
    }
}

impl ClassicBot {
    // ---- Guild ----

    pub fn determine_set_shipment_permission(&self, game: &Game) -> Option<Rc<SetShipmentPermission>> {
        if game.current_phase != Phase::BeginningOfShipAndMove {
            return None;
        }

        let to_deny: Vec<Faction> =
            game.shipment_permissions.keys().copied().filter(|f| self.is_winning_opponent_faction(game, *f)).collect();
        if !to_deny.is_empty() {
            let mut e = SetShipmentPermission::new(game, self.faction(game));
            e.factions = Some(to_deny);
            e.permission = ShipmentPermission::None;
            return Some(Rc::new(e));
        }

        let discount_permission = if self.resources(game) > 10 || !self.winning_opponents_i_wish_to_attack(game, 99, true).is_empty() {
            ShipmentPermission::GuildRate
        } else {
            ShipmentPermission::None
        };
        let permission = ShipmentPermission(ShipmentPermission::Cross.0 | ShipmentPermission::ToHomeworld.0 | discount_permission.0);
        let to_allow: Vec<Faction> = SetShipmentPermission::valid_targets(game, self.player)
            .filter(|f| !self.is_winning_opponent_faction(game, *f) && game.shipment_permissions.get(f) != Some(&permission))
            .collect();
        if to_allow.is_empty() {
            return None;
        }
        let mut e = SetShipmentPermission::new(game, self.faction(game));
        e.factions = Some(to_allow);
        e.permission = permission;
        Some(Rc::new(e))
    }

    // ---- Emperor ----

    pub fn determine_emperor_discarded(&self, game: &Game) -> Option<Rc<EmperorDiscarded>> {
        let me = game.player(self.player);
        if !(me.resources >= 10 && !me.has_room_for_cards(game)) {
            return None;
        }

        let worst_card = me.treachery_cards.iter().copied().min_by_key(|c| self.card_quality(game, *c, None))?;

        if self.card_quality(game, worst_card, None) <= 1 {
            let mut e = EmperorDiscarded::new(game, self.faction(game));
            e.set_card(worst_card.opt());
            Some(Rc::new(e))
        } else {
            None
        }
    }

    // ---- Ixian ----

    pub fn determine_replaced_card_won(&self, game: &Game) -> Rc<ReplacedCardWon> {
        let replace = game.card_just_won.is_some_and(|c| self.card_quality(game, c, Some(self.player)) <= 2);
        let mut e = ReplacedCardWon::new(game, self.faction(game));
        e.passed = !replace;
        Rc::new(e)
    }

    pub fn determine_ixian_removed_card_from_auction(&self, game: &Game) -> Rc<IxianRemovedCardFromAuction> {
        use TreacheryCardType as T;
        let me = game.player(self.player);
        let p = Some(self.player);
        let items = || game.cards_on_auction.as_ref().nre().items.iter().copied();
        let i_have = |f: fn(&TreacheryCard) -> bool| me.treachery_cards.iter().any(f);

        // (whether to look, which card to take off the auction): take away what would counter my
        // cards, or what I lack a counter for.
        let rules: [(bool, fn(&TreacheryCard) -> bool); 6] = [
            (i_have(|c| c.type_() == T::ProjectileAndPoison), |c| c.type_() == T::ShieldAndAntidote),
            (!i_have(|c| c.type_() == T::ShieldAndAntidote), |c| c.type_() == T::ProjectileAndPoison),
            (i_have(|c| c.is_projectile_weapon()), |c| c.is_projectile_defense()),
            (i_have(|c| c.is_poison_weapon()), |c| c.is_poison_defense()),
            (!i_have(|c| c.is_projectile_defense()), |c| c.is_projectile_weapon()),
            (!i_have(|c| c.is_poison_defense()), |c| c.is_poison_weapon()),
        ];

        let to_be_removed = rules
            .iter()
            .find_map(|(applies, wanted)| if *applies { items().find(wanted) } else { None })
            .or_else(|| {
                if me.treachery_cards.len() >= 3 || self.resources_including_ally_and_emperor_contribution(game) < 5 {
                    items().highest_or_default(&self.random, |c| self.card_quality(game, *c, p))
                } else {
                    items().lowest_or_default(&self.random, |c| self.card_quality(game, *c, p))
                }
            })
            .unwrap_or_else(|| items().cs_first());

        let quality = self.card_quality(game, to_be_removed, p);
        let put_on_top = (quality <= 2 && self.ally(game) == Faction::None) || (quality >= 4 && self.ally(game) != Faction::None);

        let mut e = IxianRemovedCardFromAuction::new(game, self.faction(game));
        e.set_card(to_be_removed.opt());
        e.put_on_top = put_on_top;
        Rc::new(e)
    }

    pub fn determine_ixian_swapped_card_on_bid(&self, game: &Game) -> Rc<IxianSwappedCardOnBid> {
        let p = Some(self.player);
        let worst_card = game
            .player(self.player)
            .treachery_cards
            .iter()
            .copied()
            .lowest_or_default(&self.random, |c| self.card_quality(game, *c, p))
            .filter(|c| self.card_quality(game, *c, p) <= 2);

        let mut e = IxianSwappedCardOnBid::new(game, self.faction(game));
        e.passed = worst_card.is_none();
        if worst_card.is_some() {
            e.set_card(worst_card);
        }
        Rc::new(e)
    }

    pub fn determine_ixian_selected_starting_card(&self, game: &Game) -> Rc<IxianSelectedStartingCard> {
        use TreacheryCardType as T;
        let cards = || game.starting_treachery_cards.as_ref().nre().items.iter().copied();
        let any = |f: fn(&TreacheryCard) -> bool| cards().any(|c| f(&c));

        // (whether to look, which card to pick), in order of preference.
        let rules: [(bool, fn(&TreacheryCard) -> bool); 12] = [
            (true, |c| c.type_() == T::ProjectileAndPoison),
            (true, |c| c.type_() == T::ShieldAndAntidote),
            (true, |c| c.type_() == T::Laser),
            (any(|c| c.is_poison_weapon()) && !any(|c| c.is_poison_defense()), |c| c.is_poison_weapon()),
            (any(|c| c.is_projectile_weapon()) && !any(|c| c.is_projectile_defense()), |c| c.is_projectile_weapon()),
            (true, |c| c.type_() == T::WeirdingWay),
            (true, |c| c.type_() == T::Chemistry),
            (true, |c| c.is_projectile_weapon()),
            (true, |c| c.is_poison_weapon()),
            (true, |c| c.is_projectile_defense()),
            (true, |c| c.is_poison_defense()),
            (true, |c| c.type_() != T::Useless),
        ];

        let card = rules
            .iter()
            .find_map(|(applies, wanted)| if *applies { cards().find(wanted) } else { None })
            .or_else(|| cards().next().and_then(|c| c.opt()));

        let mut e = IxianSelectedStartingCard::new(game, self.faction(game));
        e.set_card(card);
        Rc::new(e)
    }

    pub fn determine_perform_hms_movement(&self, game: &Game) -> Rc<PerformHmsMovement> {
        let current_location = game.map.hidden_mobile_stronghold().attached_to_location(&game.map);
        let faction = self.faction(game);
        let target_event = |target: Option<Location>| {
            let mut e = PerformHmsMovement::new(game, faction);
            e.passed = false;
            e.set_target(game, target);
            Rc::new(e)
        };
        let step_towards = |target: Location, distance: i32| {
            PerformHmsMovement::valid_locations(game).find(|l| self.within_distance(game, *l, target, distance))
        };

        let richest_adjacent_spice_location = PerformHmsMovement::valid_locations(game)
            .filter(|l| current_location != Some(*l) && self.resources_in(game, *l) > 0)
            .highest_or_default(&self.random, |l| self.resources_in(game, *l));
        if let Some(l) = richest_adjacent_spice_location {
            return target_event(Some(l));
        }

        let reachable_from_current_location =
            game.map.find_neighbours(current_location.or_null(), game.hms_moves_left, false, faction, game, false);

        let richest_reachable_spice_location = reachable_from_current_location
            .iter()
            .copied()
            .filter(|l| current_location != Some(*l) && self.resources_in(game, *l) > 0)
            .highest_or_default(&self.random, |l| self.resources_in(game, *l));
        if let Some(richest) = richest_reachable_spice_location {
            return target_event(step_towards(richest, 1).or_else(|| step_towards(richest, 2)));
        }

        // With nowhere to go, move toward Imperial Basin or Polar Sink.
        let alternative_location =
            if arrakeen_or_carthag_is_empty(game) { game.map.imperial_basin().middle_location() } else { game.map.polar_sink() };

        if current_location != Some(alternative_location) {
            let next = PerformHmsMovement::valid_locations(game)
                .find(|l| *l == alternative_location)
                .or_else(|| step_towards(alternative_location, 1))
                .or_else(|| step_towards(alternative_location, 2));
            if next.is_some() {
                return target_event(next);
            }
        }

        let mut e = PerformHmsMovement::new(game, faction);
        e.passed = true;
        Rc::new(e)
    }

    pub fn determine_perform_hms_placement(&self, game: &Game) -> Rc<PerformHmsPlacement> {
        let faction = self.faction(game);
        let target = if faction != Faction::Ixian {
            game.map.shield_wall().locations().iter().copied().cs_first()
        } else if arrakeen_or_carthag_is_empty(game) {
            let imperial_basin = game.map.imperial_basin();
            PerformHmsPlacement::valid_locations(game, self.player).cs_first_where(|l| l.territory() == imperial_basin)
        } else {
            game.map.polar_sink()
        };

        let mut e = PerformHmsPlacement::new(game, faction);
        e.set_target(game, target.opt());
        Rc::new(e)
    }

    // ---- Fremen ----

    pub fn determine_take_losses(&self, game: &Game) -> Rc<TakeLosses> {
        let losses = TakeLosses::losses_to_take(game).amount;
        let normal_forces = std::cmp::min(losses, TakeLosses::valid_max_force_amount(game, self.player));
        let mut e = TakeLosses::new(game, self.faction(game));
        e.force_amount = normal_forces;
        e.special_force_amount = losses - normal_forces;
        e.use_useless_card = TakeLosses::can_prevent_losses(game, self.player);
        Rc::new(e)
    }

    pub fn determine_perform_fremen_setup(&self, game: &Game) -> Rc<PerformFremenSetup> {
        let faction = self.faction(game);
        let special_forces_in_reserve = game.player(self.player).special_forces_in_reserve();
        let mut force_locations: NetDictionary<Location, Battalion> = NetDictionary::new();

        let south = game.map.false_wall_south().middle_location();
        let forces = 3 + self.d(1, 4);
        let special = if special_forces_in_reserve > 0 { 1 } else { 0 };
        force_locations.add(south, Battalion::new(faction, forces, special, south));

        let forces_left = 10 - (forces + special);
        if forces_left > 0 {
            let special = if special_forces_in_reserve > 1 { 1 } else { 0 };
            let west = game.map.false_wall_west().middle_location();
            force_locations.add(west, Battalion::new(faction, forces_left - special, special, west));
        }

        let mut e = PerformFremenSetup::new(game, faction);
        e.set_force_locations(game, &force_locations);
        Rc::new(e)
    }

    pub fn determine_fremen_sent_monster(&self, game: &Game) -> Rc<FremenSentMonster> {
        let target = FremenSentMonster::valid_targets(game).highest_or_default(&self.random, |t| {
            self.total_max_dial_of_opponents(game, *t) + (2 * t.forces_of(game, self.player)) as f32
        });
        let mut e = FremenSentMonster::new(game, self.faction(game));
        e.set_territory(game, target);
        Rc::new(e)
    }

    /// Where Fremen forces riding a worm (or moved by the Fremen ambassador) go: the first of
    /// Tuek's Sietch, Carthag, Arrakeen and Habbanya Sietch that is vacant and safe from the storm,
    /// otherwise the vacant, storm-safe location with the most spice.
    fn vacant_stronghold_or_richest_spice(&self, game: &Game, valid_locations: &[Location]) -> Option<Location> {
        let map = &game.map;
        let usable = |l: Location| valid_locations.contains(&l) && self.vacant_and_safe_from_storm(game, l);
        [map.tueks_sietch(), map.carthag(), map.arrakeen(), map.habbanya_sietch()].into_iter().find(|l| usable(*l)).or_else(|| {
            game.resources_on_planet
                .iter()
                .map(|(l, amount)| (*l, *amount))
                .filter(|(l, _)| usable(*l))
                .highest_or_default(&self.random, |(_, amount)| *amount)
                .map(|(l, _)| l)
        })
    }

    pub fn determine_fremen_rides_monster(&self, game: &Game) -> Rc<FremenRidesMonster> {
        let me = game.player(self.player);
        let valid_locations: Vec<Location> = FremenRidesMonster::valid_targets(game, self.player).collect();
        // `add` throws if a source is listed twice.
        let mut battalions_to_move: NetDictionary<Location, Battalion> = NetDictionary::new();
        for l in FremenRidesMonster::valid_sources(game) {
            battalions_to_move.add(l, me.battalion_in_location(l).nre());
        }
        let forces_from_reserves = std::cmp::min(FremenRidesMonster::max_forces_from_reserves(game, self.player, false), 4);
        let special_forces_from_reserves = std::cmp::min(FremenRidesMonster::max_forces_from_reserves(game, self.player, true), 2);

        let target = self.vacant_stronghold_or_richest_spice(game, &valid_locations).or_else(|| {
            let strength = battalions_to_move.values().map(|b| b.amount_of_forces + 2 * b.amount_of_special_forces).sum::<i32>()
                + forces_from_reserves
                + 2 * special_forces_from_reserves;
            let can_take = |l: &Location| self.total_max_dial_of_opponents(game, l.territory()) < strength as f32;

            valid_locations
                .iter()
                .copied()
                .filter(|l| game.resources_on_planet.contains_key(l) && can_take(l))
                .highest_or_default(&self.random, |l| *game.resources_on_planet.index(l))
                .or_else(|| {
                    valid_locations
                        .iter()
                        .copied()
                        .filter(|l| *l != game.map.sietch_tabr() && l.is_stronghold() && can_take(l))
                        .lowest_or_default(&self.random, |l| self.total_max_dial_of_opponents(game, l.territory()))
                })
        });
        let target = target.unwrap_or(game.map.polar_sink());

        let total_forces = battalions_to_move.values().map(|b| b.total_amount_of_forces()).sum::<i32>()
            + forces_from_reserves
            + special_forces_from_reserves;

        let mut e = FremenRidesMonster::new(game, self.faction(game));
        if total_forces > 0 {
            e.passed = false;
            e.set_force_locations(game, &NetDictionary::from_dictionary(&battalions_to_move));
            e.set_to(game, Some(target));
            e.forces_from_reserves = forces_from_reserves;
            e.special_forces_from_reserves = special_forces_from_reserves;
        } else {
            e.passed = true;
        }
        Rc::new(e)
    }

    // ---- BeneGesserit ----

    pub fn determine_bene_gesserit_placement(&self, game: &Game) -> Rc<PerformBeneGesseritPlacement> {
        let map = &game.map;
        let empty = |l: Location| l.forces_of(game, self.player) == 0;
        let target = if empty(map.arrakeen()) {
            map.carthag()
        } else if empty(map.carthag()) {
            map.arrakeen()
        } else if empty(map.habbanya_sietch()) {
            map.habbanya_sietch()
        } else {
            map.carthag()
        };

        let mut e = PerformBeneGesseritPlacement::new(game, self.faction(game));
        e.set_target(game, Some(target));
        Rc::new(e)
    }

    pub fn determine_bene_gesserit_flip(&self, game: &Game) -> Rc<BeneGesseritFlip> {
        let territory = BeneGesseritFlip::get_territory(game);

        let mut e = BeneGesseritFlip::new(game, self.faction(game));
        e.as_advisors = !self.i_want_to_be_fighters_in(game, territory);
        Rc::new(e)
    }

    /// Whether `x` is an opponent (not me or my ally) who meets the normal victory condition and
    /// has fewer than 2 challenged victory points. Players are checked in order up to `x`.
    fn potential_winning_opponents_contains(&self, game: &Game, x: Option<PlayerIndex>) -> bool {
        let allied = self.allied_player(game);
        game.player_indices()
            .filter(|p| {
                *p != self.player
                    && Some(*p) != allied
                    && game.meets_normal_victory_condition(*p, true)
                    && game.count_challenged_victory_points(*p) < 2
            })
            .any(|p| Some(p) == x)
    }

    pub fn i_want_to_be_fighters_in(&self, game: &Game, territory: Territory) -> bool {
        if !territory.is_stronghold() {
            return false;
        }

        if self.i_am_winning(game) {
            return true;
        }

        let Some(opponent) = self.get_opponent_that_occupies(game, territory) else {
            return false;
        };

        let amount_i_can_reinforce = ClassicBot::max_reinforced_dial_to(game, self.player, opponent, territory);
        let max_dial = ClassicBot::max_dial(game, self.player, territory, Some(opponent), false);
        let dial_needed = self.dial_needed_against(game, territory, Some(opponent), false);

        (!game.player(opponent).is(Faction::Harkonnen) && self.win_was_predicted_by_me_this_turn(game, game.player(opponent).faction))
            || dial_needed <= max_dial
            || (self.last_turn(game) && dial_needed - 1.0 <= amount_i_can_reinforce + max_dial)
            || (self.potential_winning_opponents_contains(game, Some(opponent)) && dial_needed <= amount_i_can_reinforce + max_dial)
    }

    pub fn determine_bene_gesserit_battle_announcement(&self, game: &Game) -> Option<Rc<BeneGesseritBattleAnnouncement>> {
        if game.current_main_phase > MainPhase::Bidding
            && self.nr_of_battles_to_fight(game) <= Battle::valid_battle_heroes(game, self.player).len() as i32
        {
            // The filter (which can draw) runs over every territory first, then the selector runs
            // twice per remaining territory.
            let territory = BeneGesseritBattleAnnouncement::valid_territories(game, self.player)
                .filter(|t| self.i_want_to_announce_battle_in(game, *t))
                .lowest_or_default(&self.random, |t| {
                    self.dial_needed_against(game, *t, self.get_opponent_that_occupies(game, *t), false)
                });

            if let Some(territory) = territory {
                let mut e = BeneGesseritBattleAnnouncement::new(game, self.faction(game));
                e.set_territory(game, Some(territory));
                return Some(Rc::new(e));
            }
        }

        None
    }

    pub fn i_want_to_announce_battle_in(&self, game: &Game, territory: Territory) -> bool {
        let Some(opponent) = self.get_opponent_that_occupies(game, territory) else {
            return false;
        };

        let dial_needed = self.dial_needed_against(game, territory, Some(opponent), false);

        let last_turn_confidence_bonus = if self.last_turn(game) { 3 } else { 0 };

        (territory.is_stronghold() && self.win_was_predicted_by_me_this_turn(game, game.player(opponent).faction))
            || dial_needed + last_turn_confidence_bonus as f32
                <= ClassicBot::max_dial(game, self.player, territory, Some(opponent), false)
                    + ClassicBot::max_reinforced_dial_to(game, self.player, opponent, territory)
    }

    pub fn determine_bene_gesserit_accompanies(&self, game: &Game) -> Rc<BeneGesseritAccompanies> {
        let me = game.player(self.player);
        let faction = self.faction(game);
        let accompany = |location: Location, extra_advisor: bool| {
            let mut e = BeneGesseritAccompanies::new(game, faction);
            e.set_location(game, Some(location));
            e.accompanies = true;
            e.extra_advisor = extra_advisor;
            Rc::new(e)
        };
        let no_accompany = || {
            let mut e = BeneGesseritAccompanies::new(game, faction);
            e.set_location(game, None);
            e.accompanies = false;
            Rc::new(e)
        };

        if me.forces_in_reserve() <= 0 {
            return no_accompany();
        }

        let target = BeneGesseritAccompanies::valid_targets(game, self.player).into_iter().find(|l| l.is_stronghold());

        if let Some(target) = target {
            let opponent = self.get_opponent_that_occupies(game, target);
            let shipping_opponent_can_win = self.potential_winning_opponents_contains(game, opponent);

            if !shipping_opponent_can_win
                && (self.resources_including_ally_contribution(game) < 5 || me.forces_in_reserve() > 3)
                && target.forces_of(game, self.player) < 8
                && ((!self.last_turn(game) && target.forces_of(game, self.player) > 0)
                    || (!me.has_ally() && me.forces_in_locations.keys().filter(|l| self.is_stronghold(game, **l)).count() <= 3))
            {
                return accompany(target, false);
            }
        }

        let polar_sink = game.map.polar_sink();
        if BeneGesseritAccompanies::valid_targets(game, self.player).contains(&polar_sink)
            && (self.resources_including_ally_contribution(game) < 5 || me.forces_in_reserve() > 3)
            && !(self.last_turn(game) && game.has_acted_or_passed.contains(&faction))
            && polar_sink.forces_of(game, self.player) < 8
        {
            return accompany(polar_sink, BeneGesseritAccompanies::may_send_extra_advisor(game, self.player, polar_sink));
        }

        no_accompany()
    }

    pub fn determine_bene_gesserit_prediction(&self, game: &Game) -> Rc<BeneGesseritPrediction> {
        let humans = || self.others(game).into_iter().filter(|p| !game.is_bot(*p));
        let predicted = (if self.d(1, 2) == 1 { humans().random_or_default(&self.random) } else { None })
            .or_else(|| humans().random_or_default(&self.random))
            .or_else(|| self.others(game).into_iter().random_or_default(&self.random));

        let predicted = game.player(predicted.nre());
        let maximum_turns = game.maximum_turns();
        // In `self.d(1, sides)` with a drawn `sides`, the draw for `sides` comes first.
        // A negative `sides` throws after its draw.
        let turn = if predicted.is(Faction::Harkonnen) {
            let sides = if self.d(1, 2) == 1 { std::cmp::min(3, maximum_turns) } else { maximum_turns };
            self.d(1, sides)
        } else if predicted.is(Faction::Atreides) || predicted.is(Faction::Ixian) {
            if self.d(1, 2) == 1 { self.d(1, maximum_turns) } else { 2 + self.d(1, maximum_turns - 2) }
        } else if predicted.is(Faction::Tleilaxu) {
            3 + self.d(1, maximum_turns - 3)
        } else if self.d(1, 2) == 1 {
            1 + self.d(1, maximum_turns - 1)
        } else {
            2 + self.d(1, maximum_turns - 2)
        };

        let mut e = BeneGesseritPrediction::new(game, self.faction(game));
        e.to_win = predicted.faction;
        e.turn = turn;
        Rc::new(e)
    }

    pub fn determine_voice(&mut self, game: &Game) -> Option<Rc<Voice>> {
        self.voice_plan = None;

        let battle = game.current_battle.clone().nre();
        if battle.is_aggressor_or_defender(game, self.player) {
            let opponent = battle.opponent_of_faction(game, self.faction(game));
            let plan = self.best_voice(game, game.current_battle.clone(), self.player, opponent.or_null());
            let voice = plan.voice.clone();
            self.voice_plan = Some(plan);
            return Some(voice);
        }

        None
    }

    /// The voice `p` should use against `opponent`, with the weapon and defense it is meant to
    /// support. `battle` is kept by reference: it is later compared with the current battle by
    /// identity.
    pub fn best_voice(&self, game: &Game, battle: Option<Rc<BattleInitiated>>, p: PlayerIndex, opponent: PlayerIndex) -> VoicePlan {
        use TreacheryCardType as T;

        // C# parity: when I predicted this opponent's win, a voice forcing a lasgun was built here
        // and then overwritten below; only these two flags remain.
        let win_predicted = self.win_was_predicted_by_me_this_turn(game, game.player(opponent).faction);
        let mut result = VoicePlan {
            battle,
            voice: Rc::new(Voice::default()),
            weapon_to_use: None,
            defense_to_use: None,
            player_hero_will_certainly_survive: win_predicted,
            opponent_hero_will_certainly_be_zero: win_predicted,
        };

        let opponent_cards = OpponentCards::new(game, self.player, opponent);
        let known_cards = game.known_cards(self.player);

        let weapons: Vec<TreacheryCard> = ClassicBot::weapons(game, self.player, None, None, None)
            .into_iter()
            .filter(|w| w.type_() != T::Useless && w.type_() != T::ArtilleryStrike && w.type_() != T::PoisonTooth)
            .collect();
        let find_weapon = |f: &dyn Fn(&TreacheryCard) -> bool| weapons.iter().copied().find(f);
        result.weapon_to_use = find_weapon(&|w| w.type_() == T::ProjectileAndPoison)
            .or_else(|| find_weapon(&|w| w.type_() == T::Laser))
            .or_else(|| find_weapon(&|w| known_cards.contains(w)))
            .or_else(|| weapons.first().copied());

        // With a weapon, first try to disable the opponent's matching defense.
        let mut command = None;
        let opponent_might_have_defenses = opponent_cards.known_defenses.is_empty() || opponent_cards.unknown_count != 0;
        if opponent_might_have_defenses {
            if let Some(weapon_to_use) = result.weapon_to_use {
                result.opponent_hero_will_certainly_be_zero = true;
                command = opponent_cards.voice_against_defenses(game, weapon_to_use);
            }
        }

        // Otherwise, try to disable the opponent's weapon that my defense doesn't stop.
        if command.is_none() {
            let opponent_might_have_weapons = opponent_cards.known_weapons.is_empty() || opponent_cards.unknown_count != 0;

            let mut defense: Option<TreacheryCard> = None;
            self.determine_best_defense(game, self.player, opponent, None, &mut defense);
            result.defense_to_use = defense.and_then(|d| d.opt());

            if opponent_might_have_weapons {
                if let Some((against_weapons, hero_survives)) =
                    result.defense_to_use.and_then(|d| opponent_cards.voice_against_weapons(d))
                {
                    command = Some(against_weapons);
                    if let Some(survives) = hero_survives {
                        result.player_hero_will_certainly_survive = survives;
                    }
                }
            }
        }

        let command = command
            .or_else(|| {
                let opponent_has_known_mercenary = game
                    .player(opponent)
                    .treachery_cards
                    .iter()
                    .any(|c| known_cards.contains(c) && c.type_() == T::Mercenary);
                opponent_has_known_mercenary.then(|| VoiceCommand::must_use(T::Mercenary))
            })
            .unwrap_or_else(|| {
                let might_have = |f: fn(TreacheryCard) -> bool| opponent_cards.might_have(f);
                VoiceCommand::may_not_use(if self.d(1, 2) > 1 {
                    if might_have(|c| c.is_projectile_weapon()) { T::Projectile } else { T::Poison }
                } else if might_have(|c| c.is_poison_weapon()) {
                    T::Poison
                } else {
                    T::Projectile
                })
            });

        let mut voice = Voice::new(game, game.player(p).faction);
        voice.must = command.must;
        voice.type_ = command.type_;
        result.voice = Rc::new(voice);

        result
    }

    // ---- Atreides ----

    pub fn determine_prescience(&self, game: &Game) -> Option<Rc<Prescience>> {
        let battle = game.current_battle.clone().nre();
        let opponent = battle.opponent_of_faction(game, self.faction(game));

        if Voice::may_use_voice(game, opponent.or_null()) && game.current_voice.is_none() && battle.plan_of(game, opponent).is_none() {
            // Wait for voice or a finalized battle plan.
            return None;
        }

        if !battle.is_aggressor_or_defender(game, self.player) {
            return None;
        }

        let existing_aspect = game.current_prescience.as_ref().map_or(PrescienceAspect::None, |p| p.aspect);
        let mut e = Prescience::new(game, self.faction(game));
        let territory = battle.territory(game).or_null();
        let max_dial = ClassicBot::max_dial(game, self.player, territory, opponent, false);
        e.aspect = self.best_prescience(game, self.player, opponent.or_null(), max_dial, existing_aspect, territory);
        Some(Rc::new(e))
    }

    /// Which aspect of `opponent`'s battle plan `p` should ask for, other than `earlier_prescience`.
    pub fn best_prescience(&self, game: &Game, p: PlayerIndex, opponent: PlayerIndex, max_force_strength_in_battle: f32, earlier_prescience: PrescienceAspect, territory: Territory) -> PrescienceAspect {
        use TreacheryCardType as T;
        let known_cards = game.known_cards(p);
        let my_defenses: Vec<TreacheryCard> =
            ClassicBot::defenses(game, p, None, Some(territory)).into_iter().filter(|c| known_cards.contains(c)).collect();
        let my_weapons: Vec<TreacheryCard> =
            ClassicBot::weapons(game, p, None, None, Some(territory)).into_iter().filter(|c| known_cards.contains(c)).collect();
        let i_have_defense = |f: fn(&TreacheryCard) -> bool| my_defenses.iter().any(f);
        let i_have_weapon = |f: fn(&TreacheryCard) -> bool| my_weapons.iter().any(f);

        let cards_opponent_has_or_might_have = ClassicBot::cards_player_has_or_might_have(game, opponent);
        let weapon_is_certain = ClassicBot::count_different_weapon_types(&cards_opponent_has_or_might_have) <= 1;
        let defense_is_certain = ClassicBot::count_different_defense_types(&cards_opponent_has_or_might_have) <= 1;

        // Knowing the weapon helps when I must pick a defense; a shield snooper makes it moot.
        let may_ask_weapon =
            earlier_prescience != PrescienceAspect::Weapon && !weapon_is_certain && !i_have_defense(|d| d.type_() == T::ShieldAndAntidote);
        // Knowing the defense helps when I must pick a weapon; a poison blade makes it moot.
        let may_ask_defense =
            earlier_prescience != PrescienceAspect::Defense && !defense_is_certain && !i_have_weapon(|w| w.type_() == T::ProjectileAndPoison);
        let choice_of_defense = i_have_defense(|d| d.is_projectile_defense()) && i_have_defense(|d| d.is_poison_defense());
        let choice_of_weapon = i_have_weapon(|w| w.is_projectile_weapon()) && i_have_weapon(|w| w.is_poison_weapon());

        if may_ask_weapon && choice_of_defense {
            PrescienceAspect::Weapon
        } else if may_ask_defense && choice_of_weapon {
            PrescienceAspect::Defense
        } else if may_ask_weapon && !my_defenses.is_empty() {
            PrescienceAspect::Weapon
        } else if may_ask_defense && !my_weapons.is_empty() {
            PrescienceAspect::Defense
        } else if earlier_prescience != PrescienceAspect::Dial
            && max_force_strength_in_battle > 2.0
            && Prescience::valid_aspects(game, p).contains(&PrescienceAspect::Dial)
        {
            PrescienceAspect::Dial
        } else if earlier_prescience != PrescienceAspect::Leader {
            PrescienceAspect::Leader
        } else {
            PrescienceAspect::None
        }
    }

    // ---- Tleilaxu ----

    pub fn determine_set_increased_revival_limits(&self, game: &Game) -> Option<Rc<SetIncreasedRevivalLimits>> {
        let targets: Vec<Faction> = SetIncreasedRevivalLimits::valid_targets(game, self.player).collect();
        if game.factions_with_increased_revival_limits.as_ref().nre().len() != targets.len() {
            let mut e = SetIncreasedRevivalLimits::new(game, self.faction(game));
            e.factions = Some(targets);
            return Some(Rc::new(e));
        }

        None
    }

    pub fn determine_face_dancer_revealed(&self, game: &Game) -> Rc<FaceDancerRevealed> {
        let mut e = FaceDancerRevealed::new(game, self.faction(game));
        e.passed = true;

        if FaceDanced::may_call_face_dancer(game, self.player) {
            // The winning hero is looked up (and may throw) once per face dancer.
            let face_dancer =
                game.player(self.player).face_dancers.iter().copied().find(|f| game.winner_hero().nre().is_face_dancer(*f));
            let face_danced_hero_is_living_leader = face_dancer.is_some_and(|f| f.as_leader().is_some() && game.is_alive(f));

            if (FaceDanced::maximum_number_of_forces(game, self.player) > 0 || face_danced_hero_is_living_leader)
                && game.battle_winner != Some(self.ally(game))
            {
                e.passed = false;
            }
        }

        Rc::new(e)
    }

    pub fn determine_face_danced(&self, game: &Game) -> Rc<FaceDanced> {
        let faction = self.faction(game);
        let passed = || {
            let mut e = FaceDanced::new(game, faction);
            e.passed = true;
            Rc::new(e)
        };

        let mut to_place = FaceDanced::maximum_number_of_forces(game, self.player);

        let Some(biggest) = self.biggest_battalion_threatened_by_storm_without_spice(game) else {
            return passed();
        };

        let to_take = biggest.battalion.take(to_place, false);
        to_place -= to_take.total_amount_of_forces();

        let from_reserves = std::cmp::min(game.player(self.player).forces_in_reserve(), to_place);

        let target_location = FaceDanced::valid_target_locations(game)
            .find(|l| game.resources_on_planet.contains_key(l))
            .or_else(|| FaceDanced::valid_target_locations(game).next().and_then(|l| l.opt()));

        let Some(target_location) = target_location else {
            return passed();
        };

        let mut target_locations: NetDictionary<Location, Battalion> = NetDictionary::new();
        target_locations.add(
            target_location,
            Battalion::new(faction, to_take.amount_of_forces + from_reserves, to_take.amount_of_special_forces, target_location),
        );
        let mut forces_from_planet: NetDictionary<Location, Battalion> = NetDictionary::new();
        forces_from_planet.add(biggest.location, to_take);

        let mut result = FaceDanced::new(game, faction);
        result.set_force_locations(game, &forces_from_planet);
        result.forces_from_reserve = from_reserves;
        result.set_target_force_locations(game, &target_locations);
        Rc::new(result)
    }

    pub fn determine_face_dancer_replaced(&self, game: &Game) -> Rc<FaceDancerReplaced> {
        let mut replaceable: Vec<Hero> = FaceDancerReplaced::valid_face_dancers(game, self.player).collect();
        replaceable.sort_by_key(|f| f.value());

        let me = game.player(self.player);
        let allied_leaders: Option<Vec<Leader>> = self.allied_player(game).map(|a| game.player(a).leaders.clone());
        let leaders_contain = |leaders: &[Leader], f: Hero| f.as_leader().is_some_and(|l| leaders.contains(&l));

        let to_replace = replaceable
            .iter()
            .copied()
            .find(|f| leaders_contain(&me.leaders, *f) || allied_leaders.as_ref().is_some_and(|al| leaders_contain(al, *f)))
            .or_else(|| replaceable.iter().copied().find(|f| f.as_leader().is_some() && !game.is_alive(*f)));

        let mut e = FaceDancerReplaced::new(game, self.faction(game));
        e.passed = to_replace.is_none();
        if to_replace.is_some() {
            e.set_selected_dancer(to_replace);
        }
        Rc::new(e)
    }

    pub fn determine_request_tleilaxu_revival(&mut self, game: &Game) -> Option<Rc<RequestTleilaxuRevival>> {
        let faction = self.faction(game);
        if game.current_revival_requests.iter().any(|r| r.initiator() == faction)
            || game.early_revivals_offers.keys().any(|h| h.faction() == faction)
        {
            return None;
        }

        if self.turn_when_revival_was_requested == game.current_turn {
            return None;
        }

        let most_valuable = |filter: &dyn Fn(&Hero) -> bool| {
            RequestTleilaxuRevival::valid_targets(game, self.player).filter(filter).highest_or_default(&self.random, |l| l.value())
        };

        // The safe leaders are computed on the first target, and only if there is one (it can throw).
        let safe: OnceCell<Vec<Hero>> = OnceCell::new();
        let mut to_revive = most_valuable(&|l| safe.get_or_init(|| self.safe_or_known_traitor_leaders(game)).contains(l));

        if to_revive.is_none() {
            let known_opponent_traitors: Vec<Hero> =
                self.opponents(game).iter().flat_map(|p| game.player(*p).revealed_traitors.iter().copied()).collect();
            to_revive = most_valuable(&|l| !known_opponent_traitors.contains(l));
        }

        if to_revive.is_some() && Battle::valid_battle_heroes(game, self.player).len() <= 3 {
            to_revive = most_valuable(&|_| true);
        }

        let to_revive = to_revive?;
        self.turn_when_revival_was_requested = game.current_turn;
        let mut e = RequestTleilaxuRevival::new(game, faction);
        e.set_hero(Some(to_revive));
        Some(Rc::new(e))
    }

    pub fn determine_accept_or_cancel_tleilaxu_revival(&mut self, game: &Game) -> Option<Rc<AcceptOrCancelTleilaxuRevival>> {
        let request_to_handle = game.current_revival_requests.first().cloned()?;
        let hero = request_to_handle.hero().or_null();

        let price = if request_to_handle.initiator() == self.ally(game) {
            0
        } else if let Some(price) = self.price_set_earlier.get(&hero) {
            *price
        } else {
            let me = game.player(self.player);
            let hero_is_ours = (me.face_dancers.contains(&hero) && !me.revealed_face_dancers.contains(&hero))
                || (me.has_ally() && {
                    let ally = game.player(self.allied_player(game).nre());
                    ally.traitors.contains(&hero) && !ally.revealed_traitors.contains(&hero)
                });
            let price = if hero_is_ours { 1 + self.d(1, hero.value()) } else { 2 + self.d(2, hero.value()) };
            self.price_set_earlier.insert(hero, price);
            price
        };

        let mut e = AcceptOrCancelTleilaxuRevival::new(game, self.faction(game));
        e.set_hero(hero.opt());
        e.price = price;
        e.cancel = false;
        Some(Rc::new(e))
    }

    // ---- Choam ----

    pub fn determine_resources_audited(&self, game: &Game) -> Option<Rc<ResourcesAudited>> {
        let battle = match &game.current_battle {
            Some(b) if b.is_aggressor_or_defender(game, self.player) => b.clone(),
            _ => return None,
        };

        let opponent = game.player(battle.opponent_of(game, self.player).nre());
        let me = game.player(self.player);

        if opponent.treachery_cards.iter().any(|tc| !me.known_cards.contains(tc))
            && ResourcesAudited::valid_factions(game, self.player).any(|f| f == opponent.faction)
        {
            let mut e = ResourcesAudited::new(game, self.faction(game));
            e.target = opponent.faction;
            return Some(Rc::new(e));
        }

        None
    }

    /// How many players other than my ally have fewer than 2 spice (or are Bene Gesserit, when
    /// they receive charity automatically).
    fn nr_of_poor_players(&self, game: &Game) -> i32 {
        let ally = self.ally(game);
        game.player_indices()
            .map(|p| game.player(p))
            .filter(|p| p.faction != ally)
            .filter(|p| p.resources < 2 || (game.applicable(Rule::BeneGesseritAutoCharity) && p.faction == Faction::BeneGesserit))
            .count() as i32
    }

    pub fn determine_choam_economics(&self, game: &Game) -> Option<Rc<ChoamEconomics>> {
        let status = if self.resources_including_ally_contribution(game) >= 14 && self.nr_of_poor_players(game) >= 2 {
            ChoamEconomicsStatus::Cancel
        } else if self.resources_including_ally_contribution(game) <= 8 && self.nr_of_poor_players(game) <= 2 {
            ChoamEconomicsStatus::Double
        } else {
            return None;
        };

        let mut e = ChoamEconomics::new(game, self.faction(game));
        e.status = status;
        Some(Rc::new(e))
    }

    pub fn determine_choam_remove_force(&self, game: &Game) -> Option<Rc<ChoamRemoveForce>> {
        let opponents = self.winning_opponents_i_wish_to_attack(game, 20, true);

        let map = &game.map;
        let strongholds: [Location; 6] = [
            map.carthag(),
            map.arrakeen(),
            map.tueks_sietch(),
            map.sietch_tabr(),
            map.habbanya_sietch(),
            map.hidden_mobile_stronghold().into(),
        ];

        for stronghold in strongholds {
            let opponent_with_one_force_in_stronghold = opponents
                .iter()
                .copied()
                .find(|opp| game.player(*opp).battalion_in_location(stronghold).nre().total_amount_of_forces() == 1);

            if let Some(opp) = opponent_with_one_force_in_stronghold {
                let mut e = ChoamRemoveForce::new(game, self.faction(game));
                e.set_location(game, Some(stronghold));
                e.target = game.player(opp).faction;
                e.special_force = game.player(opp).special_forces_in(Some(stronghold)) > 0;
                return Some(Rc::new(e));
            }
        }

        None
    }

    // ---- Moritani ----

    pub fn determine_extortion_prevented(&self, game: &Game) -> Option<Rc<ExtortionPrevented>> {
        if game.player(self.player).ally != Faction::Moritani && self.resources(game) > 12 && self.d(1, 6) >= 3 {
            return Some(Rc::new(ExtortionPrevented::new(game, self.faction(game))));
        }

        None
    }

    pub fn determine_perform_moritani_setup(&self, game: &Game) -> Rc<PerformMoritaniSetup> {
        let map = &game.map;
        let valid = || PerformMoritaniSetup::valid_locations(game);
        let preferred = [
            map.carthag(),
            map.arrakeen(),
            map.tueks_sietch(),
            map.habbanya_sietch(),
            map.imperial_basin().middle_location(),
            map.polar_sink(),
        ];

        let target = preferred
            .into_iter()
            .find(|l| valid().any(|v| v == *l))
            .unwrap_or_else(|| valid().cs_first_where(|l| l.is_protected_from_storm()));

        let mut e = PerformMoritaniSetup::new(game, self.faction(game));
        e.set_target(game, target.opt());
        Rc::new(e)
    }

    pub fn determine_terror_planted(&self, game: &Game) -> Option<Rc<TerrorPlanted>> {
        if game.current_main_phase != MainPhase::Contemplate {
            return None;
        }

        let available_to_place: Vec<TerrorType> =
            TerrorPlanted::valid_terror_types(game, false).filter(|t| !game.terror_on_planet.contains_key(t)).collect();
        let valid_strongholds: Vec<Territory> = TerrorPlanted::valid_strongholds(game, self.player).collect();

        let mut e = TerrorPlanted::new(game, self.faction(game));
        if available_to_place.is_empty() || valid_strongholds.is_empty() {
            e.passed = true;
            return Some(Rc::new(e));
        }

        let opponents = self.opponents(game);
        let can_place = |t: TerrorType| available_to_place.contains(&t);

        let type_ = if can_place(TerrorType::Robbery)
            && opponents.iter().map(|p| game.player(*p).resources).sum::<i32>() > opponents.len() as i32 * 10
        {
            TerrorType::Robbery
        } else if can_place(TerrorType::Atomics)
            && valid_strongholds
                .iter()
                .any(|t| t.forces_of_all(game, &opponents) > 12 && t.forces_of_all(game, &self.me_and_my_ally(game)) < 2)
        {
            TerrorType::Atomics
        } else if can_place(TerrorType::Extortion) && self.resources(game) < 5 {
            TerrorType::Extortion
        } else {
            available_to_place.iter().copied().random_or_default(&self.random).unwrap_or_default()
        };

        let strongholds = || valid_strongholds.iter().copied();
        let mut stronghold = if type_ == TerrorType::Atomics {
            strongholds().highest_or_default(&self.random, |t| t.forces_of_all(game, &opponents))
        } else {
            strongholds().highest_or_default(&self.random, |t| t.forces_of(game, self.player))
        };

        if stronghold.is_none() && game.player(self.player).has_ally() {
            stronghold = strongholds().highest_or_default(&self.random, |t| t.forces_of(game, self.allied_player(game).nre()) > 0);
        }

        let stronghold = stronghold.or_else(|| strongholds().random_or_default(&self.random));

        if let Some(stronghold) = stronghold {
            e.type_ = type_;
            e.set_stronghold(game, Some(stronghold));
        } else {
            e.passed = true;
        }
        Some(Rc::new(e))
    }

    pub fn determine_terror_revealed(&self, game: &Game) -> Rc<TerrorRevealed> {
        // For now this is mostly random.
        let territory = TerrorRevealed::get_territory(game);
        // C# parity: `AlliedPlayer?.AnyForcesIn(territory) == 0` is false without an ally.
        let may_use_atomics = self.opponents(game).iter().map(|o| game.player(*o).any_forces_in_territory(territory)).sum::<i32>() > 5
            && self.allied_player(game).map(|a| game.player(a).any_forces_in_territory(territory)) == Some(0);

        let token_is_valid = |t: &TerrorType| {
            (*t != TerrorType::SneakAttack || TerrorRevealed::valid_sneak_attack_targets(game, self.player).next().is_some())
                && (*t != TerrorType::Atomics || may_use_atomics)
        };

        let mut e = TerrorRevealed::new(game, self.faction(game));
        if !TerrorRevealed::get_types(game).any(|t| token_is_valid(&t)) {
            e.passed = true;
            return Rc::new(e);
        }

        let type_ = TerrorRevealed::get_types(game).filter(token_is_valid).random_or_default(&self.random).unwrap_or_default();
        let card_in_sabotage = game.player(self.player).treachery_cards.iter().copied().find(|c| c.is_useless());
        let victim = game.get_player(Some(TerrorRevealed::get_victim(game)));

        let offer_alliance = TerrorRevealed::may_offer_alliance(game)
            && self.player_standing(game, victim.or_null()) as f32 > 1.5f32 * self.player_standing(game, self.player) as f32;

        if offer_alliance {
            e.alliance_offered = true;
            return Rc::new(e);
        }
        e.type_ = type_;
        e.robbery_takes_card = false;
        e.set_card_to_give_in_sabotage(card_in_sabotage);
        e.forces_in_sneak_attack = TerrorRevealed::max_amount_of_forces_in_sneak_attack(game, self.player);
        e.set_sneak_attack_to(game, TerrorRevealed::valid_sneak_attack_targets(game, self.player).next().and_then(|l| l.opt()));
        Rc::new(e)
    }

    // ---- Ecaz ----

    pub fn determine_ambassador_placed(&mut self, game: &Game) -> Option<Rc<AmbassadorPlaced>> {
        let resources = self.resources(game);
        if (resources <= 1 || game.ambassadors_placed_this_turn != 0) && resources <= 3 + game.ambassadors_placed_this_turn {
            return None;
        }

        let me = game.player(self.player);
        let valid = || AmbassadorPlaced::valid_strongholds(game, self.player);
        let mut stronghold = valid().filter(|s| s.forces_of(game, self.player) > 0).random_or_default(&self.random);
        if stronghold.is_none() && me.has_ally() {
            stronghold = valid().filter(|s| s.forces_of(game, self.allied_player(game).nre()) > 0).random_or_default(&self.random);
        }

        // In a stronghold where my side already has forces, an ambassador that keeps others out helps.
        let avoid_entering = stronghold.is_some();

        let stronghold = stronghold
            .or_else(|| valid().filter(|s| self.vacant(game, *s)).random_or_default(&self.random))
            .or_else(|| valid().random_or_default(&self.random));

        let available = AmbassadorPlaced::valid_ambassadors(game, self.player);
        let can_place = |a: Ambassador| available.contains(&a);
        let p = Some(self.player);
        let ambassador = if avoid_entering && can_place(Ambassador::Harkonnen) {
            Ambassador::Harkonnen
        } else if avoid_entering && can_place(Ambassador::Atreides) {
            Ambassador::Atreides
        } else if can_place(Ambassador::Tleilaxu)
            && me.leaders.iter().filter(|l| !game.is_alive(Hero::Leader(**l))).count() >= 2
            && me.forces_killed >= 4
        {
            Ambassador::Tleilaxu
        } else if can_place(Ambassador::Guild)
            && me.treachery_cards.iter().any(|c| c.is_weapon())
            && me.treachery_cards.iter().any(|c| c.is_defense())
        {
            Ambassador::Guild
        } else if can_place(Ambassador::Choam) && me.treachery_cards.iter().filter(|c| self.card_quality(game, **c, p) <= 1).count() >= 2 {
            Ambassador::Choam
        } else if can_place(Ambassador::Ixian) && me.treachery_cards.iter().any(|c| self.card_quality(game, *c, p) <= 1) {
            Ambassador::Ixian
        } else if can_place(Ambassador::Richese) && me.has_room_for_cards(game) && me.resources > 6 {
            Ambassador::Richese
        } else if can_place(Ambassador::Emperor) && self.resources(game) <= 4 {
            Ambassador::Emperor
        } else if can_place(Ambassador::Fremen) && self.determine_moved_battalion(game, false).is_some() {
            Ambassador::Fremen
        } else if can_place(Ambassador::Ecaz) && !me.has_ally() {
            Ambassador::Ecaz
        } else if game.ambassadors_placed_this_turn == 0 {
            AmbassadorPlaced::valid_ambassadors(game, self.player).into_iter().random_or_default(&self.random).unwrap_or_default()
        } else {
            Ambassador::None
        };

        if ambassador == Ambassador::None {
            return None;
        }
        let mut e = AmbassadorPlaced::new(game, self.faction(game));
        e.ambassador = ambassador;
        e.set_stronghold(game, stronghold);
        Some(Rc::new(e))
    }

    pub fn determine_ambassador_activated(&mut self, game: &Game) -> Option<Rc<AmbassadorActivated>> {
        let victim_player = AmbassadorActivated::get_victim_player(game);
        let mut ambassador = AmbassadorActivated::get_ambassador(game);
        let mut bene_gesserit_selected_ambassador = Ambassador::None;

        if ambassador == Ambassador::BeneGesserit {
            // For now this is random.
            bene_gesserit_selected_ambassador = AmbassadorActivated::get_valid_bene_gesserit_ambassadors(game)
                .into_iter()
                .random_or_default(&self.random)
                .unwrap_or_default();
            ambassador = bene_gesserit_selected_ambassador;
        }

        let faction = self.faction(game);
        let p = Some(self.player);
        let new_event = || {
            let mut e = AmbassadorActivated::new(game, faction);
            e.bene_gesserit_selected_ambassador = bene_gesserit_selected_ambassador;
            e
        };

        match ambassador {
            Ambassador::Choam => {
                let to_discard: Vec<TreacheryCard> = AmbassadorActivated::get_valid_choam_cards(game, self.player)
                    .into_iter()
                    .filter(|c| self.card_quality(game, *c, p) < 2)
                    .collect();
                if !to_discard.is_empty() {
                    let mut e = new_event();
                    e.set_choam_cards(&to_discard);
                    return Some(Rc::new(e));
                }
            }

            Ambassador::Ecaz => {
                let offer_alliance = AmbassadorActivated::alliance_can_be_offered(game, self.player)
                    && self.player_standing(game, victim_player.or_null()) as f64 > 0.33 * self.player_standing(game, self.player) as f64;
                let take_vidal = !offer_alliance && AmbassadorActivated::vidal_can_be_taken(game, self.player);
                let offer_vidal = offer_alliance
                    && AmbassadorActivated::vidal_can_be_offered_to_new_ally(game, self.player)
                    && ClassicBot::heroes_for_battle(game, self.player, true).len() >= 3
                    && ClassicBot::heroes_for_battle(game, victim_player.or_null(), true).len() < 3;

                if offer_alliance || take_vidal {
                    let mut e = new_event();
                    e.ecaz_offer_alliance = offer_alliance;
                    e.ecaz_take_vidal = take_vidal;
                    e.ecaz_give_vidal_to_ally = offer_vidal;
                    return Some(Rc::new(e));
                }
            }

            Ambassador::Fremen => {
                let valid_locations: Vec<Location> = AmbassadorActivated::valid_fremen_targets(game, self.player).collect();
                let target = self.vacant_stronghold_or_richest_spice(game, &valid_locations);

                if target.is_some() {
                    if let Some(to_move) = self.determine_moved_battalion(game, true) {
                        if AmbassadorActivated::valid_fremen_sources(game, self.player).contains(&to_move.from.territory()) {
                            let mut forces_to_move: NetDictionary<Location, Battalion> = NetDictionary::new();
                            forces_to_move.add(to_move.from, to_move.battalion.clone());

                            let mut e = new_event();
                            e.set_fremen_or_guild_to(game, target);
                            e.set_fremen_force_locations(game, &forces_to_move);
                            return Some(Rc::new(e));
                        }
                    }
                }
            }

            Ambassador::Ixian => {
                let to_replace =
                    AmbassadorActivated::get_valid_ixian_cards(game, self.player).into_iter().find(|c| self.card_quality(game, *c, p) < 2);
                if to_replace.is_some() {
                    let mut e = new_event();
                    e.set_ixian_card(to_replace);
                    return Some(Rc::new(e));
                }
            }

            Ambassador::Richese => {
                if self.resources(game) > 3 && game.player(self.player).has_room_for_cards(game) {
                    return Some(Rc::new(new_event()));
                }
            }

            Ambassador::Guild => {
                let dangerous_opponents: Vec<PlayerIndex> =
                    self.opponents(game).into_iter().filter(|o| self.is_almost_winning_opponent(game, *o)).collect();

                // Each target goes through every step below, including the draws in
                // `construct_attack`, before the next target starts. `lowest_or_default` collects
                // the attacks first, then runs its selector twice per attack.
                let attack = AmbassadorActivated::valid_guild_targets(game, self.player)
                    .filter(|l| l.is_stronghold())
                    .filter(|l| dangerous_opponents.is_empty() || dangerous_opponents.iter().any(|o| game.player(*o).occupies(game, Some(*l))))
                    .filter(|l| {
                        l.territory().is_stronghold()
                            && l.forces_of(game, self.player) == 0
                            && self.ally_doesnt_block(game, l.territory())
                            && !self.storm_will_probably_hit(game, *l)
                            && !self.in_storm(game, *l)
                            && self.i_dont_have_advisors_in(game, *l)
                    })
                    .map(|l| self.construct_attack(game, l, 0, 0, 4))
                    .filter(|s| match s.opponent {
                        Some(o) if s.forces_to_ship <= 4 => !self.win_was_predicted_by_me_this_turn(game, game.player(o).faction),
                        _ => false,
                    })
                    .filter(|s| s.has_forces() && s.shortage_for_shipment == 0.0)
                    .lowest_or_default(&self.random, |s| s.dial_needed + ClassicBot::determine_penalty(game, s.opponent));

                if let Some(attack) = attack {
                    let mut e = new_event();
                    e.set_fremen_or_guild_to(game, Some(attack.location));
                    e.guild_force_amount = AmbassadorActivated::valid_guild_max_forces(game, self.player);
                    return Some(Rc::new(e));
                }
            }

            Ambassador::Tleilaxu => {
                let most_valuable = |filter: &dyn Fn(&Hero) -> bool| {
                    AmbassadorActivated::valid_tleilaxu_heroes(game, self.player)
                        .into_iter()
                        .filter(filter)
                        .highest_or_default(&self.random, |l| l.value())
                };
                let safe: OnceCell<Vec<Hero>> = OnceCell::new();
                let hero_to_revive = most_valuable(&|l| safe.get_or_init(|| self.safe_or_known_traitor_leaders(game)).contains(l))
                    .or_else(|| most_valuable(&|_| true));

                if hero_to_revive.is_some()
                    && (game.player(self.player).forces_in_reserve() > 3 || Battle::valid_battle_heroes(game, self.player).len() <= 1)
                {
                    let mut e = new_event();
                    e.set_tleilaxu_hero(hero_to_revive);
                    e.tleilaxu_assign_skill = Revival::may_assign_skill(game, self.player, hero_to_revive);
                    return Some(Rc::new(e));
                }
                if AmbassadorActivated::valid_tleilaxu_max_amount(game, self.player) >= 3 {
                    let mut e = new_event();
                    e.tleilaxu_amount_of_forces = AmbassadorActivated::valid_tleilaxu_max_amount(game, self.player);
                    return Some(Rc::new(e));
                }
            }

            _ => return Some(Rc::new(new_event())),
        }

        self.pass_ambassador_activated(game)
    }

    pub fn pass_ambassador_activated(&self, game: &Game) -> Option<Rc<AmbassadorActivated>> {
        if AmbassadorActivated::may_pass(game, self.player) {
            let mut e = AmbassadorActivated::new(game, self.faction(game));
            e.passed = true;
            Some(Rc::new(e))
        } else {
            None
        }
    }

    // ---- Richese ----

    pub fn determine_richese_announces_black_market(&self, game: &Game) -> Rc<RicheseAnnouncesBlackMarket> {
        let p = Some(self.player);
        let card = game.player(self.player).treachery_cards.iter().copied().find(|c| self.card_quality(game, *c, p) < 3);
        let mut e = RicheseAnnouncesBlackMarket::new(game, self.faction(game));
        if card.is_some() {
            e.passed = false;
            e.set_card(card);
            // The auction type is drawn before the direction.
            e.auction_type = if self.d(1, 2) > 1 { AuctionType::BlackMarketSilent } else { AuctionType::BlackMarketOnceAround };
            e.direction = if self.d(1, 2) > 1 { 1 } else { -1 };
            return Rc::new(e);
        }
        e.passed = true;
        Rc::new(e)
    }

    pub fn determine_richese_revealed_no_field(&self, game: &Game) -> Option<Rc<RicheseRevealedNoField>> {
        if game.current_phase == Phase::ShipmentAndMoveConcluded {
            let location_with_no_field =
                self.forces_on_planet(game).iter().find(|(_, b)| b.amount_of_special_forces > 0).map(|(l, _)| *l);
            if let Some(l) = location_with_no_field {
                if game.resources_on_planet.contains_key(&l) && !self.occupied_by_opponent(game, l.territory()) {
                    return Some(Rc::new(RicheseRevealedNoField::new(game, self.faction(game))));
                }
            }
        }

        None
    }

    pub fn determine_richese_announces_auction(&self, game: &Game) -> Rc<RicheseAnnouncesAuction> {
        let mut e = RicheseAnnouncesAuction::new(game, self.faction(game));
        e.first = self.d(1, 2) > 1;
        Rc::new(e)
    }

    pub fn determine_richese_specifies_auction(&self, game: &Game) -> Rc<RicheseSpecifiesAuction> {
        // A copy of the cache, shuffled with the bot's random.
        let mut to_auction = Deck::new(game.richese_cache.iter().copied(), Some(self.random.clone()));
        to_auction.shuffle();
        let mut e = RicheseSpecifiesAuction::new(game, self.faction(game));
        let card = if !to_auction.is_empty() { Some(to_auction.draw()) } else { None };
        e.set_card(card.and_then(|c| c.opt()));
        e.auction_type = if self.d(1, 2) > 1 { AuctionType::RicheseSilent } else { AuctionType::RicheseOnceAround };
        e.direction = if self.d(1, 2) > 1 { 1 } else { -1 };
        Rc::new(e)
    }

    pub fn determine_richese_keeps_unsold_card(&self, game: &Game) -> Rc<RicheseKeepsUnsoldCard> {
        let mut e = RicheseKeepsUnsoldCard::new(game, self.faction(game));
        e.passed = false;
        Rc::new(e)
    }

    // ---- Harkonnen ----

    pub fn determine_captured(&self, game: &Game) -> Rc<Captured> {
        let mut e = Captured::new(game, self.faction(game));
        e.passed = false;
        Rc::new(e)
    }
}

#[derive(Clone)]
pub struct VoicePlan {
    /// Compared with the current battle by identity.
    pub battle: Option<Rc<BattleInitiated>>,
    pub voice: Rc<Voice>,
    pub weapon_to_use: Option<TreacheryCard>,
    pub defense_to_use: Option<TreacheryCard>,
    pub player_hero_will_certainly_survive: bool,
    pub opponent_hero_will_certainly_be_zero: bool,
}
