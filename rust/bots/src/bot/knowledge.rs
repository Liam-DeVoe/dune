use crate::prelude::*;
use dotnet_compat::linq::sum_f32;
use dotnet_compat::math::cmp_f32;
use treachery_engine::game::PlayerSequence;

/// `game.KnownCards(player).Contains(c)` inside a lazy query's filter. `KnownCards` is pure, so it
/// is computed once, but only when the first element reaches the filter: an empty source never
/// calls it (and so never throws from it).
struct LazyKnownCards<'a> {
    game: &'a Game,
    player: PlayerIndex,
    cards: Option<Vec<TreacheryCard>>,
}

impl<'a> LazyKnownCards<'a> {
    fn new(game: &'a Game, player: PlayerIndex) -> Self {
        LazyKnownCards { game, player, cards: None }
    }

    fn contains(&mut self, c: TreacheryCard) -> bool {
        let (game, player) = (self.game, self.player);
        self.cards.get_or_insert_with(|| game.known_cards(player)).contains(&c)
    }
}

/// `a.Union(b)`: the distinct elements of both, in order of first occurrence.
fn union<T: PartialEq + Clone>(a: impl IntoIterator<Item = T>, b: impl IntoIterator<Item = T>) -> Vec<T> {
    a.into_iter().chain(b).cs_distinct()
}

impl ClassicBot {
    pub fn max_expected_storm_moves(&self, game: &Game) -> i32 {
        if game.has_storm_prescience(Some(self.player)) {
            game.next_storm_moves
        } else {
            self.param.shipment_expected_storm_moves_when_unknown
        }
    }

    pub fn may_flip_to_advisors(&self, game: &Game) -> bool {
        self.faction(game) == Faction::BeneGesserit && game.applicable(Rule::BeneGesseritAdvisors)
    }

    pub fn forces_on_planet(&self, game: &Game) -> NetDictionary<Location, Battalion> {
        game.player(self.player).forces_on_planet()
    }

    pub fn others(&self, game: &Game) -> Vec<PlayerIndex> {
        let faction = self.faction(game);
        game.player_indices().filter(|p| game.player(*p).faction != faction).collect()
    }

    pub fn opponents(&self, game: &Game) -> Vec<PlayerIndex> {
        game.player_indices()
            .filter(|p| game.player(*p).faction != self.faction(game) && game.player(*p).faction != self.ally(game))
            .collect()
    }

    pub fn me_and_my_ally(&self, game: &Game) -> Vec<PlayerIndex> {
        game.player_indices()
            .filter(|p| game.player(*p).faction == self.faction(game) || game.player(*p).faction == self.ally(game))
            .collect()
    }

    pub fn win_was_predicted_by_me_this_turn(&self, game: &Game, opponent_faction: Faction) -> bool {
        let ally = game.player(game.get_player(Some(opponent_faction)).nre()).ally;
        let me = game.player(self.player);
        self.faction(game) == Faction::BeneGesserit
            && game.current_turn == me.predicted_turn
            && (opponent_faction == me.predicted_faction || ally == me.predicted_faction)
    }

    pub fn last_turn(&self, game: &Game) -> bool {
        game.current_turn == game.maximum_turns()
    }

    pub fn almost_last_turn(&self, game: &Game) -> bool {
        game.current_turn >= game.maximum_turns().wrapping_sub(1)
    }

    pub fn opponents_to_ship_and_move(&self, game: &Game) -> Vec<PlayerIndex> {
        self.opponents(game)
            .into_iter()
            .filter(|p| !game.has_acted_or_passed.contains(&game.player(*p).faction))
            .collect()
    }

    pub fn nr_of_non_winning_players_to_ship_and_move_including_me(&self, game: &Game) -> i32 {
        let non_winning = game.player_indices().filter(|p| !game.meets_normal_victory_condition(*p, true)).count() as i32;
        non_winning - game.has_acted_or_passed.len() as i32
    }

    pub fn i_am_winning(&self, game: &Game) -> bool {
        game.meets_normal_victory_condition(self.player, true)
    }

    pub fn opponents_are_winning(&self, game: &Game) -> bool {
        self.opponents(game).into_iter().any(|o| game.meets_normal_victory_condition(o, true))
    }

    pub fn is_winning(&self, game: &Game, p: PlayerIndex) -> bool {
        game.meets_normal_victory_condition(p, true)
    }

    pub fn is_winning_faction(&self, game: &Game, f: Faction) -> bool {
        // GetPlayer(f) may be null; MeetsNormalVictoryCondition then throws where C# does.
        game.meets_normal_victory_condition(game.get_player(Some(f)).or_null(), true)
    }

    pub fn my_prescience(&self, game: &Game) -> Option<Rc<Prescience>> {
        match &game.current_prescience {
            Some(p) if p.initiator() == self.faction(game) || p.initiator() == self.ally(game) => Some(p.clone()),
            _ => None,
        }
    }

    pub fn my_clairvoyance_about_enemy_defense_in_current_battle(game: &Game, player: PlayerIndex) -> Option<ClairVoyanceQandA> {
        Self::clairvoyance_in_current_battle(game, player, ClairvoyanceQuestion::CardTypeAsDefenseInBattle)
    }

    pub fn player_clairvoyance_about_enemy_weapon_in_current_battle(game: &Game, player: PlayerIndex) -> Option<ClairVoyanceQandA> {
        Self::clairvoyance_in_current_battle(game, player, ClairvoyanceQuestion::CardTypeAsWeaponInBattle)
    }

    /// The shared body of the two members above: they differ only in the question.
    fn clairvoyance_in_current_battle(game: &Game, player: PlayerIndex, question: ClairvoyanceQuestion) -> Option<ClairVoyanceQandA> {
        let (Some(cv), Some(qna)) = (&game.latest_clairvoyance, &game.latest_clairvoyance_qn_a) else {
            return None;
        };
        // `LatestClairvoyanceBattle == CurrentBattle` compares references.
        let same_battle = match (&game.latest_clairvoyance_battle, &game.current_battle) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            _ => false,
        };
        let p = game.player(player);
        if same_battle
            && (cv.initiator() == p.faction || cv.initiator() == p.ally)
            && (cv.question == question || cv.question == ClairvoyanceQuestion::CardTypeInBattle)
        {
            Some(qna.clone())
        } else {
            None
        }
    }

    pub fn my_voice(&self, game: &Game) -> Option<Rc<Voice>> {
        match &game.current_voice {
            Some(v) if self.faction(game) == Faction::BeneGesserit || self.ally(game) == Faction::BeneGesserit => Some(v.clone()),
            _ => None,
        }
    }

    pub fn may_use_useless_as_karma(&self, game: &Game) -> bool {
        self.faction(game) == Faction::BeneGesserit && game.applicable(Rule::BeneGesseritWorthlessAsKarma)
    }

    pub fn cards_unknown_to_player(game: &Game, player: PlayerIndex) -> Vec<TreacheryCard> {
        let mut known = LazyKnownCards::new(game, player);
        TreacheryCardManager::get_cards_in_play(game).filter(|c| !known.contains(*c)).collect()
    }

    pub fn opponent_cards_unknown_to_player(game: &Game, player: PlayerIndex, opponent: PlayerIndex) -> Vec<TreacheryCard> {
        let mut known = LazyKnownCards::new(game, player);
        game.player(opponent).treachery_cards.iter().copied().filter(|c| !known.contains(*c)).collect()
    }

    pub fn is_known_to(game: &Game, player: PlayerIndex, card: TreacheryCard) -> bool {
        game.known_cards(player).contains(&card)
    }

    pub fn cards_player_has_or_might_have(game: &Game, player: PlayerIndex) -> Vec<TreacheryCard> {
        let known = game.known_cards(player);
        let hand = &game.player(player).treachery_cards;
        let mut result: Vec<TreacheryCard> = hand.iter().copied().filter(|c| known.contains(c)).collect();

        let player_has_unknown_cards = hand.iter().any(|c| !known.contains(c));
        if player_has_unknown_cards {
            result.extend(Self::cards_unknown_to_player(game, player));
        }

        result
    }

    pub fn known_cards_held_by_other_player(game: &Game, player: PlayerIndex, other_player: PlayerIndex) -> Vec<TreacheryCard> {
        let known = game.known_cards(player);
        game.player(other_player).treachery_cards.iter().copied().filter(|c| known.contains(c)).collect()
    }

    pub fn card_quality(&self, game: &Game, card_to_rate: TreacheryCard, for_whom: Option<PlayerIndex>) -> i32 {
        let for_whom = for_whom.and_then(CsNull::opt);
        let me = game.player(self.player);
        let mut cards_to_take_into_account: Vec<TreacheryCard> = me.treachery_cards.clone();
        if let Some(for_whom) = for_whom {
            if for_whom != self.player {
                let my_known_cards = game.known_cards(self.player);
                cards_to_take_into_account =
                    game.player(for_whom).treachery_cards.iter().copied().filter(|c| my_known_cards.contains(c)).collect();
            }
        }

        cs::list_remove(&mut cards_to_take_into_account, card_to_rate.opt());

        let c = &cards_to_take_into_account;
        // `cardsToTakeIntoAccount.Any(c => c != cardToRate && c.IsX)`
        let any_other = |pred: fn(TreacheryCard) -> bool| c.iter().any(|x| *x != card_to_rate && pred(*x));

        if card_to_rate.type_() == TreacheryCardType::Useless {
            return 0;
        }

        if card_to_rate.type_() == TreacheryCardType::Thumper
            || card_to_rate.type_() == TreacheryCardType::Harvester
            || card_to_rate.type_() == TreacheryCardType::Flight
            || card_to_rate.type_() == TreacheryCardType::Juice
            || card_to_rate.is_mirror_weapon()
        {
            return 1;
        }

        if card_to_rate.type_() == TreacheryCardType::ProjectileAndPoison {
            return 5;
        }
        if card_to_rate.type_() == TreacheryCardType::ShieldAndAntidote {
            return 5;
        }
        if card_to_rate.type_() == TreacheryCardType::Laser {
            return 5;
        }
        if card_to_rate.type_() == TreacheryCardType::Rockmelter {
            return 5;
        }
        if card_to_rate.type_() == TreacheryCardType::Karma && self.faction(game) == Faction::Harkonnen && !me.special_karma_power_used {
            return 5;
        }

        let quality_when_obtaining_both_kinds =
            if self.faction(game) == Faction::Atreides || self.faction(game) == Faction::BeneGesserit { 5 } else { 4 };
        if card_to_rate.is_projectile_defense()
            && !any_other(TreacheryCard::is_projectile_defense)
            && any_other(TreacheryCard::is_poison_defense)
        {
            return quality_when_obtaining_both_kinds;
        }
        if card_to_rate.is_poison_defense() && !any_other(TreacheryCard::is_poison_defense) && any_other(TreacheryCard::is_projectile_defense) {
            return quality_when_obtaining_both_kinds;
        }
        if card_to_rate.is_projectile_weapon() && !any_other(TreacheryCard::is_projectile_weapon) && any_other(TreacheryCard::is_poison_weapon) {
            return quality_when_obtaining_both_kinds;
        }
        if card_to_rate.is_poison_weapon() && !any_other(TreacheryCard::is_poison_weapon) && any_other(TreacheryCard::is_projectile_weapon) {
            return quality_when_obtaining_both_kinds;
        }

        if self.faction(game) == Faction::BeneGesserit {
            if card_to_rate.is_projectile_weapon() && !any_other(TreacheryCard::is_projectile_weapon) {
                return 5;
            }
            if card_to_rate.is_poison_weapon() && !any_other(TreacheryCard::is_poison_weapon) {
                return 5;
            }
        }

        if card_to_rate.type_() == TreacheryCardType::Chemistry {
            return 4;
        }
        if card_to_rate.type_() == TreacheryCardType::WeirdingWay {
            return 4;
        }
        if card_to_rate.type_() == TreacheryCardType::ArtilleryStrike {
            return 4;
        }
        if card_to_rate.type_() == TreacheryCardType::PoisonTooth {
            return 4;
        }

        if card_to_rate.type_() == TreacheryCardType::SearchDiscarded {
            return 3;
        }
        if card_to_rate.type_() == TreacheryCardType::TakeDiscarded {
            return 3;
        }

        if card_to_rate.is_poison_weapon() && !any_other(TreacheryCard::is_poison_weapon) {
            return 3;
        }
        if card_to_rate.is_projectile_weapon() && !any_other(TreacheryCard::is_projectile_weapon) {
            return 3;
        }
        if card_to_rate.is_poison_defense() && !any_other(TreacheryCard::is_poison_defense) {
            return 3;
        }
        if card_to_rate.is_projectile_defense() && !any_other(TreacheryCard::is_projectile_defense) {
            return 3;
        }

        2
    }

    pub fn i_am_desperate_for_resources(&self, game: &Game) -> bool {
        self.resources_including_ally_contribution(game) < 5
    }

    pub fn resources_including_ally_contribution(&self, game: &Game) -> i32 {
        self.resources(game).wrapping_add(self.resources_from_ally(game))
    }

    pub fn resources_including_ally_and_emperor_contribution(&self, game: &Game) -> i32 {
        self.resources(game).wrapping_add(self.resources_from_ally(game)).wrapping_add(self.resources_from_emperor(game))
    }

    pub fn resources_from_ally(&self, game: &Game) -> i32 {
        if self.ally(game) != Faction::None { game.get_permitted_use_of_ally_resources(self.faction(game)) } else { 0 }
    }

    pub fn resources_from_emperor(&self, game: &Game) -> i32 {
        game.spice_for_bids_emperor_can_pay(self.faction(game))
    }

    pub fn ally_resources(&self, game: &Game) -> i32 {
        self.allied_player(game).map(|a| game.player(a).resources).unwrap_or(0)
    }

    pub fn resources_in(&self, game: &Game, l: Location) -> i32 {
        game.resources_on_planet.get(&l).copied().unwrap_or(0)
    }

    pub fn has_resources(&self, game: &Game, l: Location) -> bool {
        game.resources_on_planet.contains_key(&l)
    }

    pub fn is_stronghold(&self, game: &Game, l: Location) -> bool {
        l.is_stronghold()
            || game.is_special_stronghold(l.territory())
            || l.as_discovered_location().is_some_and(|dl| {
                dl.discovery() == DiscoveryToken::Cistern
                    || dl.discovery() == DiscoveryToken::TestingStation
                    || dl.discovery() == DiscoveryToken::ProcessingStation
                    || (dl.discovery() == DiscoveryToken::Shrine
                        && game.player(self.player).has_treachery_card_type(TreacheryCardType::Clairvoyance))
            })
    }

    pub fn not_occupied_by_others(&self, game: &Game, l: Location) -> bool {
        self.not_occupied_by_others_territory(game, l.territory())
    }

    pub fn not_occupied_by_others_territory(&self, game: &Game, t: Territory) -> bool {
        game.nr_of_occupants_excluding_faction_territory_faction(t, self.faction(game)) == 0 && self.ally_doesnt_block(game, t)
    }

    pub fn not_occupied(&self, game: &Game, t: Territory) -> bool {
        !game.is_occupied_territory(t)
    }

    pub fn not_occupied_location(&self, game: &Game, l: Location) -> bool {
        !game.is_occupied_territory(l.territory())
    }

    pub fn vacant(&self, game: &Game, l: Location) -> bool {
        self.vacant_territory(game, l.territory())
    }

    pub fn vacant_territory(&self, game: &Game, t: Territory) -> bool {
        !game.any_forces_in(t)
    }

    pub fn occupied_by_opponent(&self, game: &Game, t: Territory) -> bool {
        self.opponents(game).into_iter().any(|o| game.player(o).occupies_territory(game, Some(t)))
    }

    pub fn occupied_by_opponent_location(&self, game: &Game, l: Location) -> bool {
        self.occupied_by_opponent(game, l.territory())
    }

    pub fn get_opponent_that_occupies(&self, game: &Game, l: Location) -> Option<PlayerIndex> {
        self.get_opponent_that_occupies_territory(game, l.territory())
    }

    pub fn get_opponent_that_occupies_territory(&self, game: &Game, t: Territory) -> Option<PlayerIndex> {
        self.opponents(game).into_iter().cs_first_or_default_where(|o| game.player(*o).occupies_territory(game, Some(t)))
    }

    pub fn storm_will_probably_hit(&self, game: &Game, l: Location) -> bool {
        if game.is_protected_from_storm(l) || self.last_turn(game) {
            return false;
        }

        // C# re-reads MaxExpectedStormMoves on every iteration; it is pure.
        let mut i = 1;
        while i <= self.max_expected_storm_moves(game) {
            if cs::rem(game.sector_in_storm.wrapping_add(i), Map::NUMBER_OF_SECTORS) == l.sector(&game.map) {
                return true;
            }
            i += 1;
        }

        false
    }

    pub fn vacant_and_safe_from_storm(&self, game: &Game, location: Location) -> bool {
        if self.faction(game) == Faction::Fremen && !game.prevented(FactionAdvantage::FremenProtectedFromStorm) {
            return self.not_occupied(game, location.territory());
        }
        self.not_occupied(game, location.territory())
            && location.sector(&game.map) != game.sector_in_storm
            && !self.storm_will_probably_hit(game, location)
    }

    pub fn i_dont_have_advisors_in(&self, game: &Game, l: Location) -> bool {
        self.faction(game) != Faction::BeneGesserit || game.player(self.player).special_forces_in_territory(Some(l.territory())) == 0
    }

    pub fn ally_doesnt_block(&self, game: &Game, t: Territory) -> bool {
        let ally = self.ally(game);
        ally == Faction::None
            || ally == Faction::Ecaz
            || self.faction(game) == Faction::Ecaz
            || game.player(self.allied_player(game).or_null()).any_forces_in_territory(Some(t)) == 0
            || (self.faction(game) == Faction::BeneGesserit
                && game.applicable(Rule::AdvisorsDontConflictWithAlly)
                && game.player(self.player).special_forces_in_territory(Some(t)) > 0)
            || (ally == Faction::BeneGesserit
                && game.applicable(Rule::AdvisorsDontConflictWithAlly)
                && game.player(self.allied_player(game).or_null()).special_forces_in_territory(Some(t)) > 0)
    }

    pub fn ally_doesnt_block_location(&self, game: &Game, l: Location) -> bool {
        self.ally_doesnt_block(game, l.territory())
    }

    pub fn within_range(&self, game: &Game, from: Location, to: Location, b: &Battalion) -> bool {
        let only_advisors = b.faction == Faction::BeneGesserit && b.amount_of_forces == 0;

        let will_get_ornithopters = if !only_advisors
            && !game.applicable(Rule::MovementBonusRequiresOccupationBeforeMovement)
            && (from == game.map.arrakeen() || from == game.map.carthag())
        {
            3
        } else {
            0
        };

        let move_distance = will_get_ornithopters.max(game.determine_maximum_move_distance(self.player, std::slice::from_ref(b)));

        //Game.ForcesOnPlanet used to be null
        game.map.find_neighbours(from, move_distance, false, self.faction(game), game, true).contains(&to)
    }

    pub fn within_distance(&self, game: &Game, from: Location, to: Location, distance: i32) -> bool {
        game.map.find_neighbours(from, distance, false, self.faction(game), game, true).contains(&to)
    }

    pub fn probably_safe_from_monster(&self, game: &Game, t: Territory) -> bool {
        game.current_turn == game.maximum_turns()
            || game.protected_from_monster(self.player)
            // LatestSpiceCardA or its Location may be null: NullReferenceException (hazards 16.2).
            || t != game.latest_spice_card_a.nre().location().nre().territory()
            || game.sand_trout_occured()
            || !game.has_resource_deck_prescience(Some(self.player))
            || game.resource_card_deck.as_ref().nre().top().and_then(CsNull::opt).is_some_and(|c| !c.is_shai_hulud())
    }

    pub fn occupying_opponent_in(&self, game: &Game, player: PlayerIndex, t: Territory) -> Option<PlayerIndex> {
        let me = game.player(player);
        game.player_indices()
            .filter(|p| {
                let pl = game.player(*p);
                pl.faction != me.faction && pl.faction != me.ally && pl.occupies_territory(game, Some(t))
            })
            .highest_or_default(&self.random, |p| Self::max_dial(game, *p, t, Some(player), false))
    }

    pub fn occupying_opponents_in(game: &Game, player: PlayerIndex, t: Territory) -> Vec<PlayerIndex> {
        let me = game.player(player);
        game.player_indices()
            .filter(|p| {
                let pl = game.player(*p);
                pl.faction != me.faction && pl.faction != me.ally && pl.occupies_territory(game, Some(t))
            })
            .collect()
    }

    pub fn occupying_opponents_in_game_player_location(game: &Game, player: PlayerIndex, l: Location) -> Vec<PlayerIndex> {
        Self::occupying_opponents_in(game, player, l.territory())
    }

    pub fn in_storm(&self, game: &Game, l: Location) -> bool {
        l.sector(&game.map) == game.sector_in_storm
    }

    pub fn battalion_that_should_be_moved_due_to_ally_presence(&self, game: &Game) -> Option<BattalionInLocation> {
        if self.ally(game) == Faction::None {
            return None;
        }

        if game.has_acted_or_passed.contains(&self.ally(game)) {
            //Ally has already acted => move the biggest battalion
            return self
                .forces_on_planet(game)
                .iter()
                .filter(|(l, _)| {
                    **l != game.map.polar_sink() && !self.in_storm(game, **l) && !self.ally_doesnt_block_location(game, **l)
                })
                .map(|(l, b)| BattalionInLocation::new(*l, b.clone()))
                .highest_or_default(&self.random, |x| x.battalion.total_amount_of_forces());
        }

        //Ally has not acted yet => move the smallest battalion
        self.forces_on_planet(game)
            .iter()
            .filter(|(l, _)| **l != game.map.polar_sink() && !self.in_storm(game, **l) && !self.ally_doesnt_block(game, l.territory()))
            .map(|(l, b)| BattalionInLocation::new(*l, b.clone()))
            .lowest_or_default(&self.random, |x| x.battalion.total_amount_of_forces())
    }

    pub fn may_flee_out_of(&self, game: &Game, l: Location) -> bool {
        !self.is_stronghold(game, l) || !(self.i_am_winning(game) || self.opponents_are_winning(game))
    }

    pub fn biggest_battalion_threatened_by_storm_without_spice(&self, game: &Game) -> Option<BattalionInLocation> {
        self.forces_on_planet(game)
            .iter()
            .filter(|(l, _)| {
                self.storm_will_probably_hit(game, **l)
                    && !self.in_storm(game, **l)
                    && self.may_flee_out_of(game, **l)
                    && !self.has_resources(game, **l)
            })
            .map(|(l, b)| BattalionInLocation::new(*l, b.clone()))
            .highest_or_default(&self.random, |x| x.battalion.total_amount_of_forces())
    }

    pub fn biggest_battalion_in_spiceless_non_stronghold_location_on_rock(&self, game: &Game) -> Option<BattalionInLocation> {
        self.forces_on_planet(game)
            .iter()
            .filter(|(l, _)| {
                !self.is_stronghold(game, **l)
                    && l.sector(&game.map) != game.sector_in_storm
                    && game.is_protected_from_storm(**l)
                    && self.resources_in(game, **l) == 0
                    && (!game.player(self.player).has_treachery_card_type(TreacheryCardType::Metheor)
                        || l.territory() != game.map.pasty_mesa())
            })
            .map(|(l, b)| BattalionInLocation::new(*l, b.clone()))
            .highest_or_default(&self.random, |x| x.battalion.total_amount_of_forces())
    }

    pub fn biggest_battalion_in_spiceless_non_stronghold_location_in_sand_or_not_near_stronghold(&self, game: &Game) -> Option<BattalionInLocation> {
        self.forces_on_planet(game)
            .iter()
            .filter(|(l, b)| {
                !self.is_stronghold(game, **l)
                    && l.sector(&game.map) != game.sector_in_storm
                    && (!game.is_protected_from_storm(**l) || !game.map.strongholds().any(|s| self.within_range(game, **l, s, b)))
                    && self.resources_in(game, **l) == 0
                    && (!game.player(self.player).has_treachery_card_type(TreacheryCardType::Metheor)
                        || l.territory() != game.map.pasty_mesa())
            })
            .map(|(l, b)| BattalionInLocation::new(*l, b.clone()))
            .highest_or_default(&self.random, |x| x.battalion.total_amount_of_forces())
    }

    pub fn biggest_battalion_in_spiceless_non_stronghold_location_not_near_stronghold_and_spice(&self, game: &Game) -> Option<BattalionInLocation> {
        // BestSafeAndNearbyResources draws inside this filter (hazards 16.4). highest_or_default
        // runs the whole filter before its own draw, as C#'s ToArray does.
        self.forces_on_planet(game)
            .iter()
            .filter(|(l, b)| {
                !self.is_stronghold(game, **l)
                    && l.sector(&game.map) != game.sector_in_storm
                    && self.resources_in(game, **l) == 0
                    && (!game.player(self.player).has_treachery_card_type(TreacheryCardType::Metheor)
                        || l.territory() != game.map.pasty_mesa())
                    && self.vacant_and_safe_nearby_stronghold(game, **l, b).is_none()
                    && self.best_safe_and_nearby_resources(game, **l, b, false).is_none()
            })
            .map(|(l, b)| BattalionInLocation::new(*l, b.clone()))
            .highest_or_default(&self.random, |x| x.battalion.total_amount_of_forces())
    }

    pub fn biggest_large_unthreatened_movable_battalion_in_stronghold_near_vacant_stronghold(&self, game: &Game) -> Option<BattalionInLocation> {
        self.forces_on_planet(game)
            .iter()
            .filter(|(l, b)| {
                self.is_stronghold(game, **l)
                    && self.not_occupied_by_others(game, **l)
                    && l.sector(&game.map) != game.sector_in_storm
                    && b.total_amount_of_forces() >= 8
                    && self.vacant_and_safe_nearby_stronghold(game, **l, b).is_some()
            })
            .map(|(l, b)| BattalionInLocation::new(*l, b.clone()))
            .highest_or_default(&self.random, |x| x.battalion.total_amount_of_forces())
    }

    pub fn biggest_movable_stack_of_advisors_in_stronghold_near_vacant_stronghold(&self, game: &Game) -> Option<BattalionInLocation> {
        self.forces_on_planet(game)
            .iter()
            .filter(|(l, b)| {
                b.faction == Faction::BeneGesserit
                    && b.amount_of_special_forces > 0
                    && self.is_stronghold(game, **l)
                    && self.not_occupied_by_others(game, **l)
                    && !self.in_storm(game, **l)
                    && self.vacant_and_safe_nearby_stronghold(game, **l, b).is_some()
            })
            .map(|(l, b)| BattalionInLocation::new(*l, b.clone()))
            .highest_or_default(&self.random, |x| x.battalion.total_amount_of_forces())
    }

    pub fn biggest_large_unthreatened_movable_battalion_in_stronghold_near_spice(&self, game: &Game) -> Option<BattalionInLocation> {
        // BestSafeAndNearbyResources draws inside this filter (hazards 16.4).
        self.forces_on_planet(game)
            .iter()
            .filter(|(l, b)| {
                self.is_stronghold(game, **l)
                    && self.not_occupied_by_others_territory(game, l.territory())
                    && !self.in_storm(game, **l)
                    && b.total_amount_of_forces() >= (if self.i_am_desperate_for_resources(game) { 5 } else { 7 })
                    && self.best_safe_and_nearby_resources(game, **l, b, false).is_some()
            })
            .map(|(l, b)| BattalionInLocation::new(*l, b.clone()))
            .highest_or_default(&self.random, |x| x.battalion.total_amount_of_forces())
    }

    pub fn best_safe_and_nearby_resources(&self, game: &Game, location: Location, b: &Battalion, may_fight: bool) -> Option<Location> {
        // `.HighestOrDefault(...).Key`: the default KeyValuePair has a null Key.
        game.resources_on_planet
            .iter()
            .filter(|(l, _)| self.is_safe_and_nearby(game, location, **l, b, may_fight))
            .map(|(l, v)| (*l, *v))
            .highest_or_default(&self.random, |r| r.1)
            .map(|r| r.0)
    }

    pub fn best_safe_and_nearby_discovery(&self, game: &Game, location: Location, b: &Battalion, may_fight: bool) -> Option<Location> {
        game.discoveries_on_planet
            .keys()
            .copied()
            .cs_first_or_default_where(|l| self.is_safe_and_nearby(game, location, *l, b, may_fight))
    }

    pub fn nearby_battalions_outside_strongholds(&self, game: &Game, l: Location) -> Vec<Battalion> {
        self.forces_on_planet(game)
            .iter()
            .filter(|(k, v)| !k.is_stronghold() && self.within_range(game, **k, l, v))
            .map(|(_, v)| v.clone())
            .collect()
    }

    pub fn is_safe_and_nearby(&self, game: &Game, source: Location, destination: Location, b: &Battalion, may_fight: bool) -> bool {
        let opponent = self.get_opponent_that_occupies_territory(game, destination.territory());

        self.within_range(game, source, destination, b)
            && self.ally_doesnt_block(game, destination.territory())
            && self.probably_safe_from_monster(game, destination.territory())
            && (opponent.is_none()
                || (may_fight
                    && self.get_dial_needed_territory_player_bool(game, destination.territory(), opponent, false)
                        < Self::max_dial_game_int32_battalion_faction(game, self.resources(game), b, game.player(opponent.nre()).faction)))
            && !self.storm_will_probably_hit(game, destination)
    }

    pub fn valid_movement_locations(&self, game: &Game, from: Location, battalion: &Battalion) -> Vec<Location> {
        // `forbidden` is a lazy query: C# runs it again for every target (it is pure).
        let faction = self.faction(game);
        let forbidden = || {
            game.deals
                .iter()
                .filter(move |deal| deal.bound_faction == faction && deal.type_ == DealType::DontShipOrMoveTo)
                .map(|deal| deal.get_parameter1(game))
        };
        PlacementEvent::valid_targets(game, self.player, from, battalion)
            .into_iter()
            .filter(|l| !forbidden().any(|t| t == Some(l.territory())))
            .collect()
    }

    pub fn vacant_and_safe_nearby_stronghold(&self, game: &Game, from: Location, battalion: &Battalion) -> Option<Location> {
        self.valid_movement_locations(game, from, battalion)
            .into_iter()
            .cs_first_or_default_where(|to| self.is_stronghold(game, *to) && !self.storm_will_probably_hit(game, *to) && self.vacant(game, *to))
    }

    pub fn unthreatened_and_safe_nearby_stronghold(&self, game: &Game, from: Location, battalion: &Battalion) -> Option<Location> {
        self.valid_movement_locations(game, from, battalion).into_iter().cs_first_or_default_where(|to| {
            self.is_stronghold(game, *to) && !self.storm_will_probably_hit(game, *to) && self.not_occupied_by_others(game, *to)
        })
    }

    pub fn weak_and_safe_nearby_stronghold(&self, game: &Game, from: Location, battalion: &Battalion) -> Option<Location> {
        self.valid_movement_locations(game, from, battalion).into_iter().cs_first_or_default_where(|to| {
            self.is_stronghold(game, *to)
                && game.player(self.player).any_forces_in(Some(*to)) > 0
                && self.ally_doesnt_block(game, to.territory())
                && !self.storm_will_probably_hit(game, *to)
        })
    }

    pub fn nearby_stronghold_of_winning_opponent(&self, game: &Game, from: Location, battalion: &Battalion, include_bots: bool) -> Option<Location> {
        self.valid_movement_locations(game, from, battalion)
            .into_iter()
            .filter(|to| {
                self.is_stronghold(game, *to)
                    && self.ally_doesnt_block(game, to.territory())
                    && self
                        .winning_opponents_i_wish_to_attack(game, 20, include_bots)
                        .into_iter()
                        .any(|opponent| game.player(opponent).occupies(game, Some(*to)))
            })
            .lowest_or_default(&self.random, |l| self.total_max_dial_of_opponents(game, l.territory()))
    }

    pub fn nearby_stronghold_of_almost_winning_opponent(&self, game: &Game, from: Location, battalion: &Battalion, include_bots: bool) -> Option<Location> {
        self.valid_movement_locations(game, from, battalion)
            .into_iter()
            .filter(|to| {
                self.is_stronghold(game, *to)
                    && self.ally_doesnt_block(game, to.territory())
                    && self
                        .almost_winning_opponents_i_wish_to_attack(game, 20, include_bots)
                        .into_iter()
                        .any(|opponent| game.player(opponent).occupies(game, Some(*to)))
            })
            .lowest_or_default(&self.random, |l| self.total_max_dial_of_opponents(game, l.territory()))
    }

    pub fn is_winning_opponent(&self, game: &Game, p: PlayerIndex) -> bool {
        p != self.player && game.player(p).faction != self.ally(game) && game.meets_normal_victory_condition(p, true)
    }

    pub fn is_winning_opponent_faction(&self, game: &Game, f: Faction) -> bool {
        self.is_winning_opponent(game, game.get_player(Some(f)).or_null())
    }

    pub fn is_almost_winning_opponent(&self, game: &Game, opponent: PlayerIndex) -> bool {
        let o = game.player(opponent);
        opponent != self.player
            && Some(opponent) != self.allied_player(game)
            && game.number_of_victory_points(opponent, true).wrapping_add(1) >= game.threshold_for_win(opponent)
            && (Self::can_ship(game, opponent)
                || (o.has_ally() && Self::can_ship(game, o.allied_player(game).or_null()))
                || o.tech_tokens.len() >= 2)
    }

    pub fn winning_opponents_i_wish_to_attack(&self, game: &Game, maximum_challenged_strongholds: i32, include_bots: bool) -> Vec<PlayerIndex> {
        game.player_indices()
            .filter(|p| {
                let pl = game.player(*p);
                (include_bots || !pl.is_bot(game) || !pl.ally_is_bot(game))
                    && self.is_winning_opponent(game, *p)
                    && game.count_challenged_victory_points(*p) <= maximum_challenged_strongholds
                    && !self.win_was_predicted_by_me_this_turn(game, pl.faction)
            })
            .collect()
    }

    pub fn almost_winning_opponents_i_wish_to_attack(&self, game: &Game, maximum_challenged_strongholds: i32, include_bots: bool) -> Vec<PlayerIndex> {
        game.player_indices()
            .filter(|p| {
                let pl = game.player(*p);
                (include_bots || !pl.is_bot(game) || !pl.ally_is_bot(game))
                    && self.is_almost_winning_opponent(game, *p)
                    && game.count_challenged_victory_points(*p) <= maximum_challenged_strongholds
                    && !self.win_was_predicted_by_me_this_turn(game, pl.faction)
            })
            .collect()
    }

    pub fn winnable_nearby_stronghold(&self, game: &Game, from: Location, battalion: &Battalion) -> Option<Location> {
        // `ValidMovementLocations(...)` runs at once; the rest of the query is lazy and runs when
        // OrderBy buffers it, after `resourcesForBattle` (hazards 16.4). Each target goes through
        // every stage, with its draws, before the next target starts.
        let targets = self.valid_movement_locations(game, from, battalion);

        let resources_for_battle =
            if self.ally(game) == Faction::Choam { self.resources_including_ally_contribution(game) } else { self.resources(game) };

        struct Candidate {
            stronghold: Location,
            dial_needed: f32,
        }

        let me = game.player(self.player);
        let mut winnable: Vec<Candidate> = Vec::new();
        for to in targets {
            if !(self.is_stronghold(game, to)
                && self.occupied_by_opponent_location(game, to)
                && self.ally_doesnt_block_location(game, to)
                && !self.storm_will_probably_hit(game, to))
            {
                continue;
            }
            let Some(opponent) = self.occupying_opponent_in(game, self.player, to.territory()) else {
                continue;
            };
            let opponent = game.player(opponent).faction;
            let dial_needed = self.get_dial_needed_territory_player_bool(
                game,
                to.territory(),
                self.get_opponent_that_occupies_territory(game, to.territory()),
                true,
            );

            if self.win_was_predicted_by_me_this_turn(game, opponent)
                || self.determine_dial_shortage_for_battle(
                    game,
                    dial_needed,
                    opponent,
                    to.territory(),
                    battalion.amount_of_forces.wrapping_add(me.forces_in(Some(to))),
                    battalion.amount_of_special_forces.wrapping_add(me.special_forces_in(Some(to))),
                    resources_for_battle,
                ) <= 0.0
            {
                winnable.push(Candidate { stronghold: to, dial_needed });
            }
        }

        // OrderBy is stable.
        winnable.sort_by(|a, b| cmp_f32(a.dial_needed, b.dial_needed));
        winnable.into_iter().next().map(|s| s.stronghold)
    }

    pub fn i_must_pay_for_forces_in_battle(&self, game: &Game) -> bool {
        Battle::must_pay_for_any_forces_in_battle(game, self.player)
    }

    pub fn max_dial(game: &Game, player: PlayerIndex, t: Territory, opponent: Option<PlayerIndex>, ignore_spice_dialing: bool) -> f32 {
        let opponent = opponent.and_then(CsNull::opt);
        let p = game.player(player);
        let mut count_forces_for_richese = 0;
        if p.faction == Faction::Richese && p.special_forces_in_territory(Some(t)) > 0 {
            // C# parity: the condition is always true here (hazards 18.5), so the else branch is dead.
            count_forces_for_richese = if p.faction == Faction::Richese || p.ally == Faction::Richese {
                game.current_no_field_value
            } else if game.latest_revealed_no_field_value == 5 {
                3
            } else {
                5
            };
        }

        Self::max_dial_game_int32_int32_int32_player_faction(
            game,
            if ignore_spice_dialing { 99 } else { p.resources },
            p.forces_in_territory(Some(t)).wrapping_add(count_forces_for_richese),
            if p.faction != Faction::Richese { p.special_forces_in_territory(Some(t)) } else { 0 },
            player,
            opponent.map(|o| game.player(o).faction).unwrap_or(Faction::Harkonnen),
        )
    }

    pub fn max_dial_game_int32_battalion_faction(game: &Game, resources: i32, battalion: &Battalion, opponent: Faction) -> f32 {
        Self::max_dial_game_int32_int32_int32_player_faction(
            game,
            resources,
            battalion.amount_of_forces,
            battalion.amount_of_special_forces,
            game.get_player(Some(battalion.faction)).or_null(),
            opponent,
        )
    }

    pub fn max_dial_game_int32_int32_int32_player_faction(game: &Game, resources: i32, forces: i32, special_forces: i32, p: PlayerIndex, opponent_faction: Faction) -> f32 {
        let mut spice = if Battle::must_pay_for_any_forces_in_battle(game, p) { resources } else { 99 };

        let special_forces_at_full_strength = special_forces.min(spice);
        spice = spice.wrapping_sub(special_forces_at_full_strength);
        let special_forces_at_half_strength = special_forces.wrapping_sub(special_forces_at_full_strength);

        let forces_at_full_strength = forces.min(spice);
        let forces_at_half_strength = forces.wrapping_sub(forces_at_full_strength);

        let faction = game.player(p).faction;
        Battle::determine_special_force_strength(game, faction, opponent_faction)
            * (special_forces_at_full_strength as f32 + 0.5f32 * special_forces_at_half_strength as f32)
            + Battle::determine_normal_force_strength(game, faction)
                * (forces_at_full_strength as f32 + 0.5f32 * forces_at_half_strength as f32)
    }

    pub fn total_max_dial_of_opponents(&self, game: &Game, t: Territory) -> f32 {
        // Enumerable.Sum over floats adds in double.
        sum_f32(self.opponents(game).into_iter().map(|o| Self::max_dial(game, o, t, Some(self.player), false)))
    }

    pub fn i_will_be_aggressor_against(&self, game: &Game, opponent: Option<PlayerIndex>) -> bool {
        let Some(opponent) = opponent.and_then(CsNull::opt) else {
            return false;
        };

        let first_player_position = game.player(PlayerSequence::determine_first_player(game)).seat;

        for i in 0..game.maximum_players() {
            let position = cs::rem(first_player_position.wrapping_add(i), game.maximum_players());
            if position == game.player(self.player).seat {
                return true;
            }
            if position == game.player(opponent).seat {
                return false;
            }
        }

        false
    }

    pub fn can_ship(game: &Game, player: PlayerIndex) -> bool {
        game.current_main_phase < MainPhase::ShipmentAndMove
            || (game.current_main_phase == MainPhase::ShipmentAndMove && !game.has_acted_or_passed.contains(&game.player(player).faction))
    }

    pub fn nr_of_battles_to_fight(&self, game: &Game) -> i32 {
        Battle::battles_to_be_fought(game, self.player, false).len() as i32
    }

    pub fn max_reinforced_dial_to(game: &Game, player: PlayerIndex, opponent: PlayerIndex, to: Territory) -> f32 {
        //if (player == null || to == null) return 0;

        if Self::can_ship(game, opponent) {
            let mut special_forces = 0i32;
            let mut normal_forces = 0i32;

            let o = game.player(opponent);
            let opponent_resources = o.resources.wrapping_add(o.allied_resources(game));

            let opponent_may_use_worthless_as_karma =
                o.faction == Faction::BeneGesserit && game.applicable(Rule::BeneGesseritWorthlessAsKarma);
            // C# parity: Karma is tested twice; worthless cards never count (hazards 18.5).
            let has_karma = Self::known_cards_held_by_other_player(game, player, opponent).into_iter().any(|c| {
                c.type_() == TreacheryCardType::Karma || (opponent_may_use_worthless_as_karma && c.type_() == TreacheryCardType::Karma)
            });

            let cost = |n: i32, s: i32| {
                Shipment::determine_cost_game_player_int32_int32_location_boolean_boolean_boolean_boolean_boolean(
                    game,
                    opponent,
                    n,
                    s,
                    to.middle_location(),
                    has_karma,
                    false,
                    false,
                    false,
                    false,
                )
            };

            while special_forces + 1 <= o.special_forces_in_reserve() && cost(normal_forces, special_forces + 1) <= opponent_resources {
                special_forces += 1;
            }

            while normal_forces + 1 <= o.forces_in_reserve() && cost(normal_forces + 1, special_forces) <= opponent_resources {
                normal_forces += 1;
            }

            return special_forces as f32 * Battle::determine_special_force_strength(game, o.faction, game.player(player).faction)
                + normal_forces as f32 * Battle::determine_normal_force_strength(game, o.faction);
        }

        0.0
    }

    pub fn safe_or_known_traitor_leaders(&self, game: &Game) -> Vec<Hero> {
        let me = game.player(self.player);
        let ally = if self.ally(game) != Faction::None { self.allied_player(game) } else { None };
        let known_non_traitors_by_ally: Vec<Hero> = match ally {
            Some(a) => union(game.player(a).traitors.iter().copied(), game.player(a).known_non_traitors.iter().copied()),
            None => Vec::new(),
        };
        let known_non_traitors = union(
            union(me.traitors.iter().copied(), me.known_non_traitors.iter().copied()),
            known_non_traitors_by_ally,
        );

        let my_known_traitors_and_non_traitors = union(me.traitors.iter().copied(), known_non_traitors);
        let ally_known_traitors_and_non_traitors: Vec<Hero> = if me.has_ally() {
            // AlliedPlayer! throws if the ally isn't seated.
            let a = game.player(self.allied_player(game).nre());
            union(a.traitors.iter().copied(), a.known_non_traitors.iter().copied())
        } else {
            Vec::new()
        };
        let revealed_or_told_traitors = game.player_indices().flat_map(|p| {
            let pl = game.player(p);
            union(pl.revealed_traitors.iter().copied(), pl.told_traitors.iter().copied())
        });

        union(union(my_known_traitors_and_non_traitors, ally_known_traitors_and_non_traitors), revealed_or_told_traitors)
    }

    pub fn known_opponent_weapons(game: &Game, player: PlayerIndex, opponent: PlayerIndex) -> Vec<TreacheryCard> {
        let mut known = LazyKnownCards::new(game, player);
        game.player(opponent).treachery_cards.iter().copied().filter(|c| c.is_weapon() && known.contains(*c)).collect()
    }

    pub fn known_opponent_cards(game: &Game, player: PlayerIndex, opponent: PlayerIndex) -> Vec<TreacheryCard> {
        let mut known = LazyKnownCards::new(game, player);
        game.player(opponent).treachery_cards.iter().copied().filter(|c| known.contains(*c)).collect()
    }

    pub fn known_opponent_defenses(game: &Game, player: PlayerIndex, opponent: PlayerIndex) -> Vec<TreacheryCard> {
        let mut known = LazyKnownCards::new(game, player);
        game.player(opponent).treachery_cards.iter().copied().filter(|c| c.is_defense() && known.contains(*c)).collect()
    }

    pub fn defenses(game: &Game, player: PlayerIndex, using_this_weapon: Option<TreacheryCard>, territory: Option<Territory>) -> Vec<TreacheryCard> {
        let using_this_weapon = using_this_weapon.and_then(CsNull::opt);
        let territory = territory.and_then(CsNull::opt);
        Battle::valid_defenses(game, player, using_this_weapon, territory, false).into_iter().map(OrNull::or_null).collect()
    }

    pub fn weapons(game: &Game, player: PlayerIndex, using_this_defense: Option<TreacheryCard>, using_this_hero: Option<Hero>, territory: Option<Territory>) -> Vec<TreacheryCard> {
        let using_this_defense = using_this_defense.and_then(CsNull::opt);
        let using_this_hero = using_this_hero.and_then(CsNull::opt);
        let territory = territory.and_then(CsNull::opt);
        Battle::valid_weapons(game, player, using_this_defense, using_this_hero, territory, false).into_iter().map(OrNull::or_null).collect()
    }

    pub fn useless_as_weapon(game: &Game, player: PlayerIndex, using_this_defense: Option<TreacheryCard>) -> Option<TreacheryCard> {
        let using_this_defense = using_this_defense.and_then(CsNull::opt);
        Self::weapons(game, player, using_this_defense, None, None)
            .into_iter()
            .cs_first_or_default_where(|c| c.type_() == TreacheryCardType::Useless)
    }

    pub fn useless_as_defense(game: &Game, player: PlayerIndex, using_this_weapon: Option<TreacheryCard>) -> Option<TreacheryCard> {
        let using_this_weapon = using_this_weapon.and_then(CsNull::opt);
        // LastOrDefault(predicate) on a List walks it backwards.
        cs::last_or_default_where_list(&Self::defenses(game, player, using_this_weapon, None), |c| {
            c.type_() == TreacheryCardType::Useless
        })
        .copied()
    }

    pub fn may_play_no_weapon(game: &Game, player: PlayerIndex, using_this_defense: Option<TreacheryCard>) -> bool {
        let using_this_defense = using_this_defense.and_then(CsNull::opt);
        Battle::valid_weapons(game, player, using_this_defense, None, None, true).iter().any(|c| c.and_then(CsNull::opt).is_none())
    }

    pub fn may_play_no_defense(game: &Game, player: PlayerIndex, using_this_weapon: Option<TreacheryCard>) -> bool {
        let using_this_weapon = using_this_weapon.and_then(CsNull::opt);
        Battle::valid_defenses(game, player, using_this_weapon, None, true).iter().any(|c| c.and_then(CsNull::opt).is_none())
    }

    pub fn count_different_weapon_types(cards: &[TreacheryCard]) -> i32 {
        let mut result = 0;
        if cards.iter().any(|card| card.is_projectile_weapon() && card.type_() != TreacheryCardType::ProjectileAndPoison) {
            result += 1;
        }
        if cards.iter().any(|card| card.is_poison_weapon() && card.type_() != TreacheryCardType::ProjectileAndPoison) {
            result += 1;
        }
        if cards.iter().any(|card| card.type_() == TreacheryCardType::ProjectileAndPoison) {
            result += 1;
        }
        if cards.iter().any(|card| card.is_laser()) {
            result += 1;
        }
        if cards.iter().any(|card| card.is_artillery()) {
            result += 1;
        }
        if cards.iter().any(|card| card.is_poison_tooth()) {
            result += 1;
        }
        result
    }

    pub fn count_different_defense_types(cards: &[TreacheryCard]) -> i32 {
        let mut result = 0;
        if cards.iter().any(|card| card.is_projectile_defense() && card.type_() != TreacheryCardType::ShieldAndAntidote) {
            result += 1;
        }
        if cards.iter().any(|card| card.is_poison_defense() && card.type_() != TreacheryCardType::ShieldAndAntidote) {
            result += 1;
        }
        if cards.iter().any(|card| card.type_() == TreacheryCardType::ShieldAndAntidote) {
            result += 1;
        }
        result
    }
}

#[derive(Clone, Debug)]
pub struct BattalionInLocation {
    pub location: Location,
    pub battalion: Battalion,
}

impl BattalionInLocation {
    pub fn new(location: Location, battalion: Battalion) -> Self {
        BattalionInLocation { location, battalion }
    }
}
