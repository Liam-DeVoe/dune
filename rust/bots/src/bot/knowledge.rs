//! What the bot knows about the board, the cards and its opponents.

use crate::prelude::*;
use dotnet_compat::math::cmp_f32;
use treachery_engine::game::PlayerSequence;

/// A location or a territory: questions about who is somewhere are asked per territory.
pub trait Place: Copy {
    fn territory(self) -> Territory;
}

impl Place for Location {
    fn territory(self) -> Territory {
        Location::territory(self)
    }
}

impl Place for Territory {
    fn territory(self) -> Territory {
        self
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

/// `items` in order, each once.
fn distinct<T: PartialEq>(items: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut result = Vec::new();
    for item in items {
        if !result.contains(&item) {
            result.push(item);
        }
    }
    result
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

    /// Every player but me.
    pub fn others(&self, game: &Game) -> Vec<PlayerIndex> {
        let faction = self.faction(game);
        game.player_indices().filter(|p| game.player(*p).faction != faction).collect()
    }

    /// Every player but me and my ally.
    pub fn opponents(&self, game: &Game) -> Vec<PlayerIndex> {
        let (faction, ally) = (self.faction(game), self.ally(game));
        game.player_indices().filter(|p| game.player(*p).faction != faction && game.player(*p).faction != ally).collect()
    }

    pub fn me_and_my_ally(&self, game: &Game) -> Vec<PlayerIndex> {
        let (faction, ally) = (self.faction(game), self.ally(game));
        game.player_indices().filter(|p| game.player(*p).faction == faction || game.player(*p).faction == ally).collect()
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
        game.current_turn >= game.maximum_turns() - 1
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
        // An unseated faction throws in meets_normal_victory_condition.
        game.meets_normal_victory_condition(game.get_player(Some(f)).or_null(), true)
    }

    /// The prescience in the current battle, if I or my ally used it.
    pub fn my_prescience(&self, game: &Game) -> Option<Rc<Prescience>> {
        game.current_prescience.clone().filter(|p| p.initiator() == self.faction(game) || p.initiator() == self.ally(game))
    }

    pub fn my_clairvoyance_about_enemy_defense_in_current_battle(game: &Game, player: PlayerIndex) -> Option<ClairVoyanceQandA> {
        Self::clairvoyance_in_current_battle(game, player, ClairvoyanceQuestion::CardTypeAsDefenseInBattle)
    }

    pub fn player_clairvoyance_about_enemy_weapon_in_current_battle(game: &Game, player: PlayerIndex) -> Option<ClairVoyanceQandA> {
        Self::clairvoyance_in_current_battle(game, player, ClairvoyanceQuestion::CardTypeAsWeaponInBattle)
    }

    /// The answer to a clairvoyance about the current battle that `player` or their ally asked:
    /// `question`, or the question that covers both weapon and defense.
    fn clairvoyance_in_current_battle(game: &Game, player: PlayerIndex, question: ClairvoyanceQuestion) -> Option<ClairVoyanceQandA> {
        let (Some(cv), Some(qna)) = (&game.latest_clairvoyance, &game.latest_clairvoyance_qn_a) else {
            return None;
        };
        let same_battle = match (&game.latest_clairvoyance_battle, &game.current_battle) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            _ => false,
        };
        let p = game.player(player);
        let asked_by_us = cv.initiator() == p.faction || cv.initiator() == p.ally;
        let about_this = cv.question == question || cv.question == ClairvoyanceQuestion::CardTypeInBattle;
        (same_battle && asked_by_us && about_this).then(|| qna.clone())
    }

    /// The Voice in the current battle, if I or my ally are Bene Gesserit.
    pub fn my_voice(&self, game: &Game) -> Option<Rc<Voice>> {
        let bene_gesserit = self.faction(game) == Faction::BeneGesserit || self.ally(game) == Faction::BeneGesserit;
        game.current_voice.clone().filter(|_| bene_gesserit)
    }

    pub fn may_use_useless_as_karma(&self, game: &Game) -> bool {
        self.faction(game) == Faction::BeneGesserit && game.applicable(Rule::BeneGesseritWorthlessAsKarma)
    }

    pub fn cards_unknown_to_player(game: &Game, player: PlayerIndex) -> Vec<TreacheryCard> {
        let known = game.known_cards(player);
        TreacheryCardManager::get_cards_in_play(game).filter(|c| !known.contains(c)).collect()
    }

    pub fn opponent_cards_unknown_to_player(game: &Game, player: PlayerIndex, opponent: PlayerIndex) -> Vec<TreacheryCard> {
        let known = game.known_cards(player);
        game.player(opponent).treachery_cards.iter().copied().filter(|c| !known.contains(c)).collect()
    }

    pub fn is_known_to(game: &Game, player: PlayerIndex, card: TreacheryCard) -> bool {
        game.known_cards(player).contains(&card)
    }

    /// The cards of `player` that everyone knows of, and if they hold unknown cards, every card
    /// whose whereabouts `player` doesn't know.
    pub fn cards_player_has_or_might_have(game: &Game, player: PlayerIndex) -> Vec<TreacheryCard> {
        let known = game.known_cards(player);
        let hand = &game.player(player).treachery_cards;
        let mut result: Vec<TreacheryCard> = hand.iter().copied().filter(|c| known.contains(c)).collect();
        if hand.iter().any(|c| !known.contains(c)) {
            result.extend(Self::cards_unknown_to_player(game, player));
        }
        result
    }

    pub fn known_cards_held_by_other_player(game: &Game, player: PlayerIndex, other_player: PlayerIndex) -> Vec<TreacheryCard> {
        Self::known_opponent_cards(game, player, other_player)
    }

    /// How much I (or `for_whom`, as far as I know their hand) want `card`, from 0 to 5.
    pub fn card_quality(&self, game: &Game, card: TreacheryCard, for_whom: Option<PlayerIndex>) -> i32 {
        use TreacheryCardType as T;
        let for_whom = for_whom.and_then(CsNull::opt);
        let me = game.player(self.player);
        let mut other_cards: Vec<TreacheryCard> = match for_whom {
            Some(p) if p != self.player => Self::known_opponent_cards(game, self.player, p),
            _ => me.treachery_cards.clone(),
        };
        if let Some(i) = other_cards.iter().position(|c| *c == card) {
            other_cards.remove(i);
        }
        let any_other = |pred: fn(TreacheryCard) -> bool| other_cards.iter().any(|x| *x != card && pred(*x));

        let ty = card.type_();
        if ty == T::Useless {
            return 0;
        }
        if matches!(ty, T::Thumper | T::Harvester | T::Flight | T::Juice) || card.is_mirror_weapon() {
            return 1;
        }
        if matches!(ty, T::ProjectileAndPoison | T::ShieldAndAntidote | T::Laser | T::Rockmelter) {
            return 5;
        }
        if ty == T::Karma && self.faction(game) == Faction::Harkonnen && !me.special_karma_power_used {
            return 5;
        }

        // A card that completes a pair of both kinds of defense, or both kinds of weapon.
        let completes_pair = (card.is_projectile_defense()
            && !any_other(TreacheryCard::is_projectile_defense)
            && any_other(TreacheryCard::is_poison_defense))
            || (card.is_poison_defense() && !any_other(TreacheryCard::is_poison_defense) && any_other(TreacheryCard::is_projectile_defense))
            || (card.is_projectile_weapon() && !any_other(TreacheryCard::is_projectile_weapon) && any_other(TreacheryCard::is_poison_weapon))
            || (card.is_poison_weapon() && !any_other(TreacheryCard::is_poison_weapon) && any_other(TreacheryCard::is_projectile_weapon));
        if completes_pair {
            return if matches!(self.faction(game), Faction::Atreides | Faction::BeneGesserit) { 5 } else { 4 };
        }

        let first_weapon_of_its_kind = (card.is_projectile_weapon() && !any_other(TreacheryCard::is_projectile_weapon))
            || (card.is_poison_weapon() && !any_other(TreacheryCard::is_poison_weapon));
        if self.faction(game) == Faction::BeneGesserit && first_weapon_of_its_kind {
            return 5;
        }
        if matches!(ty, T::Chemistry | T::WeirdingWay | T::ArtilleryStrike | T::PoisonTooth) {
            return 4;
        }
        if matches!(ty, T::SearchDiscarded | T::TakeDiscarded) {
            return 3;
        }
        let first_defense_of_its_kind = (card.is_poison_defense() && !any_other(TreacheryCard::is_poison_defense))
            || (card.is_projectile_defense() && !any_other(TreacheryCard::is_projectile_defense));
        if first_weapon_of_its_kind || first_defense_of_its_kind {
            return 3;
        }
        2
    }

    pub fn i_am_desperate_for_resources(&self, game: &Game) -> bool {
        self.resources_including_ally_contribution(game) < 5
    }

    pub fn resources_including_ally_contribution(&self, game: &Game) -> i32 {
        self.resources(game) + self.resources_from_ally(game)
    }

    pub fn resources_including_ally_and_emperor_contribution(&self, game: &Game) -> i32 {
        self.resources(game) + self.resources_from_ally(game) + self.resources_from_emperor(game)
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

    /// A stronghold, or a discovery the bot treats as one.
    pub fn is_stronghold(&self, game: &Game, l: Location) -> bool {
        l.is_stronghold()
            || game.is_special_stronghold(l.territory())
            || l.as_discovered_location().is_some_and(|dl| {
                let discovery = dl.discovery();
                discovery == DiscoveryToken::Cistern
                    || discovery == DiscoveryToken::TestingStation
                    || discovery == DiscoveryToken::ProcessingStation
                    || (discovery == DiscoveryToken::Shrine
                        && game.player(self.player).has_treachery_card_type(TreacheryCardType::Clairvoyance))
            })
    }

    /// No one but me (and an ally that doesn't block me) occupies `place`.
    pub fn not_occupied_by_others(&self, game: &Game, place: impl Place) -> bool {
        let t = place.territory();
        game.nr_of_occupants_excluding_faction_territory_faction(t, self.faction(game)) == 0 && self.ally_doesnt_block(game, t)
    }

    pub fn not_occupied(&self, game: &Game, place: impl Place) -> bool {
        !game.is_occupied_territory(place.territory())
    }

    /// No forces at all in `place`.
    pub fn vacant(&self, game: &Game, place: impl Place) -> bool {
        !game.any_forces_in(place.territory())
    }

    pub fn occupied_by_opponent(&self, game: &Game, place: impl Place) -> bool {
        self.get_opponent_that_occupies(game, place).is_some()
    }

    /// The first opponent that occupies `place`.
    pub fn get_opponent_that_occupies(&self, game: &Game, place: impl Place) -> Option<PlayerIndex> {
        let t = place.territory();
        self.opponents(game).into_iter().find(|o| game.player(*o).occupies_territory(game, Some(t)))
    }

    pub fn storm_will_probably_hit(&self, game: &Game, l: Location) -> bool {
        if game.is_protected_from_storm(l) || self.last_turn(game) {
            return false;
        }
        let sector = l.sector(&game.map);
        (1..=self.max_expected_storm_moves(game)).any(|i| (game.sector_in_storm + i) % Map::NUMBER_OF_SECTORS == sector)
    }

    pub fn vacant_and_safe_from_storm(&self, game: &Game, location: Location) -> bool {
        if self.faction(game) == Faction::Fremen && !game.prevented(FactionAdvantage::FremenProtectedFromStorm) {
            return self.not_occupied(game, location);
        }
        self.not_occupied(game, location) && !self.in_storm(game, location) && !self.storm_will_probably_hit(game, location)
    }

    pub fn i_dont_have_advisors_in(&self, game: &Game, l: Location) -> bool {
        self.faction(game) != Faction::BeneGesserit || game.player(self.player).special_forces_in_territory(Some(l.territory())) == 0
    }

    /// My ally's forces in `place` don't keep me out of it.
    pub fn ally_doesnt_block(&self, game: &Game, place: impl Place) -> bool {
        let t = place.territory();
        let (faction, ally) = (self.faction(game), self.ally(game));
        let advisors_dont_conflict = game.applicable(Rule::AdvisorsDontConflictWithAlly);
        ally == Faction::None
            || ally == Faction::Ecaz
            || faction == Faction::Ecaz
            || game.player(self.allied_player(game).or_null()).any_forces_in_territory(Some(t)) == 0
            || (faction == Faction::BeneGesserit && advisors_dont_conflict && game.player(self.player).special_forces_in_territory(Some(t)) > 0)
            || (ally == Faction::BeneGesserit
                && advisors_dont_conflict
                && game.player(self.allied_player(game).or_null()).special_forces_in_territory(Some(t)) > 0)
    }

    pub fn within_range(&self, game: &Game, from: Location, to: Location, b: &Battalion) -> bool {
        let only_advisors = b.faction == Faction::BeneGesserit && b.amount_of_forces == 0;
        let will_get_ornithopters = !only_advisors
            && !game.applicable(Rule::MovementBonusRequiresOccupationBeforeMovement)
            && (from == game.map.arrakeen() || from == game.map.carthag());
        let ornithopter_distance = if will_get_ornithopters { 3 } else { 0 };
        let move_distance = ornithopter_distance.max(game.determine_maximum_move_distance(self.player, std::slice::from_ref(b)));
        self.within_distance(game, from, to, move_distance)
    }

    pub fn within_distance(&self, game: &Game, from: Location, to: Location, distance: i32) -> bool {
        game.map.find_neighbours(from, distance, false, self.faction(game), game, true).contains(&to)
    }

    pub fn probably_safe_from_monster(&self, game: &Game, t: Territory) -> bool {
        game.current_turn == game.maximum_turns()
            || game.protected_from_monster(self.player)
            // Throws when there is no spice card yet, or it has no location.
            || t != game.latest_spice_card_a.nre().location().nre().territory()
            || game.sand_trout_occured()
            || !game.has_resource_deck_prescience(Some(self.player))
            || game.resource_card_deck.as_ref().nre().top().and_then(CsNull::opt).is_some_and(|c| !c.is_shai_hulud())
    }

    /// Of the opponents of `player` that occupy `t`, the one with the highest dial.
    pub fn occupying_opponent_in(&self, game: &Game, player: PlayerIndex, t: Territory) -> Option<PlayerIndex> {
        Self::occupying_opponents_in(game, player, t)
            .into_iter()
            .highest_or_default(&self.random, |p| Self::max_dial(game, *p, t, Some(player), false))
    }

    pub fn occupying_opponents_in(game: &Game, player: PlayerIndex, place: impl Place) -> Vec<PlayerIndex> {
        let t = place.territory();
        let me = game.player(player);
        game.player_indices()
            .filter(|p| {
                let pl = game.player(*p);
                pl.faction != me.faction && pl.faction != me.ally && pl.occupies_territory(game, Some(t))
            })
            .collect()
    }

    pub fn in_storm(&self, game: &Game, l: Location) -> bool {
        l.sector(&game.map) == game.sector_in_storm
    }

    /// My battalions on the planet for which `keep` holds, in board order. `keep` runs for every
    /// battalion before any is picked; it may draw.
    fn battalions_where(&self, game: &Game, keep: impl Fn(Location, &Battalion) -> bool) -> Vec<BattalionInLocation> {
        self.forces_on_planet(game)
            .iter()
            .filter(|(l, b)| keep(**l, b))
            .map(|(l, b)| BattalionInLocation::new(*l, b.clone()))
            .collect()
    }

    /// The battalion with the most forces (a random one of those on a tie).
    fn biggest(&self, battalions: Vec<BattalionInLocation>) -> Option<BattalionInLocation> {
        battalions.into_iter().highest_or_default(&self.random, |x| x.battalion.total_amount_of_forces())
    }

    /// A battalion that shares a territory with my ally: the biggest if my ally has already
    /// moved, the smallest otherwise.
    pub fn battalion_that_should_be_moved_due_to_ally_presence(&self, game: &Game) -> Option<BattalionInLocation> {
        if self.ally(game) == Faction::None {
            return None;
        }
        let candidates =
            self.battalions_where(game, |l, _| l != game.map.polar_sink() && !self.in_storm(game, l) && !self.ally_doesnt_block(game, l));
        if game.has_acted_or_passed.contains(&self.ally(game)) {
            self.biggest(candidates)
        } else {
            candidates.into_iter().lowest_or_default(&self.random, |x| x.battalion.total_amount_of_forces())
        }
    }

    pub fn may_flee_out_of(&self, game: &Game, l: Location) -> bool {
        !self.is_stronghold(game, l) || !(self.i_am_winning(game) || self.opponents_are_winning(game))
    }

    pub fn biggest_battalion_threatened_by_storm_without_spice(&self, game: &Game) -> Option<BattalionInLocation> {
        self.biggest(self.battalions_where(game, |l, _| {
            self.storm_will_probably_hit(game, l) && !self.in_storm(game, l) && self.may_flee_out_of(game, l) && !self.has_resources(game, l)
        }))
    }

    /// Not a stronghold, not in the storm, no spice, and not Pasty Mesa if I hold a Metheor.
    fn spiceless_non_stronghold_outside_storm(&self, game: &Game, l: Location) -> bool {
        !self.is_stronghold(game, l)
            && !self.in_storm(game, l)
            && self.resources_in(game, l) == 0
            && (!game.player(self.player).has_treachery_card_type(TreacheryCardType::Metheor) || l.territory() != game.map.pasty_mesa())
    }

    pub fn biggest_battalion_in_spiceless_non_stronghold_location_on_rock(&self, game: &Game) -> Option<BattalionInLocation> {
        self.biggest(self.battalions_where(game, |l, _| {
            self.spiceless_non_stronghold_outside_storm(game, l) && game.is_protected_from_storm(l)
        }))
    }

    pub fn biggest_battalion_in_spiceless_non_stronghold_location_in_sand_or_not_near_stronghold(&self, game: &Game) -> Option<BattalionInLocation> {
        self.biggest(self.battalions_where(game, |l, b| {
            self.spiceless_non_stronghold_outside_storm(game, l)
                && (!game.is_protected_from_storm(l) || !game.map.strongholds().any(|s| self.within_range(game, l, s, b)))
        }))
    }

    pub fn biggest_battalion_in_spiceless_non_stronghold_location_not_near_stronghold_and_spice(&self, game: &Game) -> Option<BattalionInLocation> {
        // best_safe_and_nearby_resources draws, so it goes last.
        self.biggest(self.battalions_where(game, |l, b| {
            self.spiceless_non_stronghold_outside_storm(game, l)
                && self.vacant_and_safe_nearby_stronghold(game, l, b).is_none()
                && self.best_safe_and_nearby_resources(game, l, b, false).is_none()
        }))
    }

    pub fn biggest_large_unthreatened_movable_battalion_in_stronghold_near_vacant_stronghold(&self, game: &Game) -> Option<BattalionInLocation> {
        self.biggest(self.battalions_where(game, |l, b| {
            self.is_stronghold(game, l)
                && self.not_occupied_by_others(game, l)
                && !self.in_storm(game, l)
                && b.total_amount_of_forces() >= 8
                && self.vacant_and_safe_nearby_stronghold(game, l, b).is_some()
        }))
    }

    pub fn biggest_movable_stack_of_advisors_in_stronghold_near_vacant_stronghold(&self, game: &Game) -> Option<BattalionInLocation> {
        self.biggest(self.battalions_where(game, |l, b| {
            b.faction == Faction::BeneGesserit
                && b.amount_of_special_forces > 0
                && self.is_stronghold(game, l)
                && self.not_occupied_by_others(game, l)
                && !self.in_storm(game, l)
                && self.vacant_and_safe_nearby_stronghold(game, l, b).is_some()
        }))
    }

    pub fn biggest_large_unthreatened_movable_battalion_in_stronghold_near_spice(&self, game: &Game) -> Option<BattalionInLocation> {
        // best_safe_and_nearby_resources draws, so it goes last.
        let minimum_forces = if self.i_am_desperate_for_resources(game) { 5 } else { 7 };
        self.biggest(self.battalions_where(game, |l, b| {
            self.is_stronghold(game, l)
                && self.not_occupied_by_others(game, l)
                && !self.in_storm(game, l)
                && b.total_amount_of_forces() >= minimum_forces
                && self.best_safe_and_nearby_resources(game, l, b, false).is_some()
        }))
    }

    /// The richest spice location that `b` can safely reach from `location`.
    pub fn best_safe_and_nearby_resources(&self, game: &Game, location: Location, b: &Battalion, may_fight: bool) -> Option<Location> {
        game.resources_on_planet
            .iter()
            .filter(|(l, _)| self.is_safe_and_nearby(game, location, **l, b, may_fight))
            .map(|(l, amount)| (*l, *amount))
            .highest_or_default(&self.random, |r| r.1)
            .map(|r| r.0)
    }

    pub fn best_safe_and_nearby_discovery(&self, game: &Game, location: Location, b: &Battalion, may_fight: bool) -> Option<Location> {
        game.discoveries_on_planet.keys().copied().find(|l| self.is_safe_and_nearby(game, location, *l, b, may_fight))
    }

    pub fn nearby_battalions_outside_strongholds(&self, game: &Game, l: Location) -> Vec<Battalion> {
        self.forces_on_planet(game)
            .iter()
            .filter(|(k, v)| !k.is_stronghold() && self.within_range(game, **k, l, v))
            .map(|(_, v)| v.clone())
            .collect()
    }

    /// `b` can reach `destination` from `source`, and will be safe there: no ally in the way, no
    /// monster or storm expected, and no opponent, unless `may_fight` and `b` can beat them.
    pub fn is_safe_and_nearby(&self, game: &Game, source: Location, destination: Location, b: &Battalion, may_fight: bool) -> bool {
        let t = destination.territory();
        let opponent = self.get_opponent_that_occupies(game, t);
        let can_beat = |opponent: PlayerIndex| {
            may_fight
                && self.dial_needed_against(game, t, Some(opponent), false)
                    < Self::max_dial_of_battalion(game, self.resources(game), b, game.player(opponent).faction)
        };

        self.within_range(game, source, destination, b)
            && self.ally_doesnt_block(game, t)
            && self.probably_safe_from_monster(game, t)
            && opponent.is_none_or(can_beat)
            && !self.storm_will_probably_hit(game, destination)
    }

    /// Where `battalion` may move from `from`, leaving out territories I promised not to enter.
    pub fn valid_movement_locations(&self, game: &Game, from: Location, battalion: &Battalion) -> Vec<Location> {
        let faction = self.faction(game);
        let targets = PlacementEvent::valid_targets(game, self.player, from, battalion);
        if targets.is_empty() {
            return targets;
        }
        let forbidden: Vec<Option<Territory>> = game
            .deals
            .iter()
            .filter(|deal| deal.bound_faction == faction && deal.type_ == DealType::DontShipOrMoveTo)
            .map(|deal| deal.get_parameter1(game))
            .collect();
        targets.into_iter().filter(|l| !forbidden.contains(&Some(l.territory()))).collect()
    }

    /// The first stronghold `battalion` can move to that is empty and not about to be hit by the storm.
    pub fn vacant_and_safe_nearby_stronghold(&self, game: &Game, from: Location, battalion: &Battalion) -> Option<Location> {
        self.valid_movement_locations(game, from, battalion)
            .into_iter()
            .find(|to| self.is_stronghold(game, *to) && !self.storm_will_probably_hit(game, *to) && self.vacant(game, *to))
    }

    pub fn unthreatened_and_safe_nearby_stronghold(&self, game: &Game, from: Location, battalion: &Battalion) -> Option<Location> {
        self.valid_movement_locations(game, from, battalion)
            .into_iter()
            .find(|to| self.is_stronghold(game, *to) && !self.storm_will_probably_hit(game, *to) && self.not_occupied_by_others(game, *to))
    }

    /// The first stronghold `battalion` can move to where I already have forces.
    pub fn weak_and_safe_nearby_stronghold(&self, game: &Game, from: Location, battalion: &Battalion) -> Option<Location> {
        self.valid_movement_locations(game, from, battalion).into_iter().find(|to| {
            self.is_stronghold(game, *to)
                && game.player(self.player).any_forces_in(Some(*to)) > 0
                && self.ally_doesnt_block(game, *to)
                && !self.storm_will_probably_hit(game, *to)
        })
    }

    pub fn nearby_stronghold_of_winning_opponent(&self, game: &Game, from: Location, battalion: &Battalion, include_bots: bool) -> Option<Location> {
        self.weakest_nearby_stronghold_held_by(game, from, battalion, || self.winning_opponents_i_wish_to_attack(game, 20, include_bots))
    }

    pub fn nearby_stronghold_of_almost_winning_opponent(&self, game: &Game, from: Location, battalion: &Battalion, include_bots: bool) -> Option<Location> {
        self.weakest_nearby_stronghold_held_by(game, from, battalion, || self.almost_winning_opponents_i_wish_to_attack(game, 20, include_bots))
    }

    /// Of the strongholds `battalion` can move to that one of `opponents` occupies, the one where
    /// my opponents dial least. `opponents` is asked again for every stronghold.
    fn weakest_nearby_stronghold_held_by(
        &self,
        game: &Game,
        from: Location,
        battalion: &Battalion,
        opponents: impl Fn() -> Vec<PlayerIndex>,
    ) -> Option<Location> {
        self.valid_movement_locations(game, from, battalion)
            .into_iter()
            .filter(|to| {
                self.is_stronghold(game, *to)
                    && self.ally_doesnt_block(game, *to)
                    && opponents().into_iter().any(|opponent| game.player(opponent).occupies(game, Some(*to)))
            })
            .lowest_or_default(&self.random, |l| self.total_max_dial_of_opponents(game, l.territory()))
    }

    pub fn is_winning_opponent(&self, game: &Game, p: PlayerIndex) -> bool {
        p != self.player && game.player(p).faction != self.ally(game) && game.meets_normal_victory_condition(p, true)
    }

    pub fn is_winning_opponent_faction(&self, game: &Game, f: Faction) -> bool {
        self.is_winning_opponent(game, game.get_player(Some(f)).or_null())
    }

    /// An opponent one victory point short of winning, who can still ship forces or holds two
    /// tech tokens.
    pub fn is_almost_winning_opponent(&self, game: &Game, opponent: PlayerIndex) -> bool {
        let o = game.player(opponent);
        opponent != self.player
            && Some(opponent) != self.allied_player(game)
            && game.number_of_victory_points(opponent, true) + 1 >= game.threshold_for_win(opponent)
            && (Self::can_ship(game, opponent)
                || (o.has_ally() && Self::can_ship(game, o.allied_player(game).or_null()))
                || o.tech_tokens.len() >= 2)
    }

    pub fn winning_opponents_i_wish_to_attack(&self, game: &Game, maximum_challenged_strongholds: i32, include_bots: bool) -> Vec<PlayerIndex> {
        self.opponents_i_wish_to_attack(game, maximum_challenged_strongholds, include_bots, |p| self.is_winning_opponent(game, p))
    }

    pub fn almost_winning_opponents_i_wish_to_attack(&self, game: &Game, maximum_challenged_strongholds: i32, include_bots: bool) -> Vec<PlayerIndex> {
        self.opponents_i_wish_to_attack(game, maximum_challenged_strongholds, include_bots, |p| self.is_almost_winning_opponent(game, p))
    }

    fn opponents_i_wish_to_attack(
        &self,
        game: &Game,
        maximum_challenged_strongholds: i32,
        include_bots: bool,
        threatening: impl Fn(PlayerIndex) -> bool,
    ) -> Vec<PlayerIndex> {
        game.player_indices()
            .filter(|p| {
                let pl = game.player(*p);
                (include_bots || !pl.is_bot(game) || !pl.ally_is_bot(game))
                    && threatening(*p)
                    && game.count_challenged_victory_points(*p) <= maximum_challenged_strongholds
                    && !self.win_was_predicted_by_me_this_turn(game, pl.faction)
            })
            .collect()
    }

    /// Of the strongholds held by an opponent that `battalion` can move to and win a battle in,
    /// the one that needs the lowest dial.
    pub fn winnable_nearby_stronghold(&self, game: &Game, from: Location, battalion: &Battalion) -> Option<Location> {
        let targets = self.valid_movement_locations(game, from, battalion);
        let resources_for_battle =
            if self.ally(game) == Faction::Choam { self.resources_including_ally_contribution(game) } else { self.resources(game) };
        let me = game.player(self.player);

        // Each target goes through every check, with its draws, before the next one starts.
        let mut winnable: Vec<(Location, f32)> = Vec::new();
        for to in targets {
            if !(self.is_stronghold(game, to)
                && self.occupied_by_opponent(game, to)
                && self.ally_doesnt_block(game, to)
                && !self.storm_will_probably_hit(game, to))
            {
                continue;
            }
            let Some(opponent) = self.occupying_opponent_in(game, self.player, to.territory()) else {
                continue;
            };
            let opponent = game.player(opponent).faction;
            let dial_needed = self.dial_needed_against(game, to.territory(), self.get_opponent_that_occupies(game, to), true);

            if self.win_was_predicted_by_me_this_turn(game, opponent)
                || self.determine_dial_shortage_for_battle(
                    game,
                    dial_needed,
                    opponent,
                    to.territory(),
                    battalion.amount_of_forces + me.forces_in(Some(to)),
                    battalion.amount_of_special_forces + me.special_forces_in(Some(to)),
                    resources_for_battle,
                ) <= 0.0
            {
                winnable.push((to, dial_needed));
            }
        }

        winnable.into_iter().min_by(|a, b| cmp_f32(a.1, b.1)).map(|(stronghold, _)| stronghold)
    }

    pub fn i_must_pay_for_forces_in_battle(&self, game: &Game) -> bool {
        Battle::must_pay_for_any_forces_in_battle(game, self.player)
    }

    /// The strength `player` can put into a battle in `t` against `opponent` (Harkonnen if none).
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

        Self::max_dial_of_forces(
            game,
            if ignore_spice_dialing { 99 } else { p.resources },
            p.forces_in_territory(Some(t)) + count_forces_for_richese,
            if p.faction != Faction::Richese { p.special_forces_in_territory(Some(t)) } else { 0 },
            player,
            opponent.map(|o| game.player(o).faction).unwrap_or(Faction::Harkonnen),
        )
    }

    pub fn max_dial_of_battalion(game: &Game, resources: i32, battalion: &Battalion, opponent: Faction) -> f32 {
        Self::max_dial_of_forces(
            game,
            resources,
            battalion.amount_of_forces,
            battalion.amount_of_special_forces,
            game.get_player(Some(battalion.faction)).or_null(),
            opponent,
        )
    }

    /// The strength of `forces` and `special_forces` of `p`, paying for as many as `resources`
    /// allow (special forces first); unpaid forces count half.
    pub fn max_dial_of_forces(game: &Game, resources: i32, forces: i32, special_forces: i32, p: PlayerIndex, opponent_faction: Faction) -> f32 {
        let mut spice = if Battle::must_pay_for_any_forces_in_battle(game, p) { resources } else { 99 };

        let special_forces_at_full_strength = special_forces.min(spice);
        spice -= special_forces_at_full_strength;
        let special_forces_at_half_strength = special_forces - special_forces_at_full_strength;

        let forces_at_full_strength = forces.min(spice);
        let forces_at_half_strength = forces - forces_at_full_strength;

        let faction = game.player(p).faction;
        Battle::determine_special_force_strength(game, faction, opponent_faction)
            * (special_forces_at_full_strength as f32 + 0.5 * special_forces_at_half_strength as f32)
            + Battle::determine_normal_force_strength(game, faction) * (forces_at_full_strength as f32 + 0.5 * forces_at_half_strength as f32)
    }

    pub fn total_max_dial_of_opponents(&self, game: &Game, t: Territory) -> f32 {
        // Summed in double precision.
        self.opponents(game).into_iter().map(|o| Self::max_dial(game, o, t, Some(self.player), false) as f64).sum::<f64>() as f32
    }

    /// Whether I come before `opponent` in this turn's player order.
    pub fn i_will_be_aggressor_against(&self, game: &Game, opponent: Option<PlayerIndex>) -> bool {
        let Some(opponent) = opponent.and_then(CsNull::opt) else {
            return false;
        };
        let first_player_position = game.player(PlayerSequence::determine_first_player(game)).seat;
        for i in 0..game.maximum_players() {
            let position = (first_player_position + i) % game.maximum_players();
            if position == game.player(self.player).seat {
                return true;
            }
            if position == game.player(opponent).seat {
                return false;
            }
        }
        false
    }

    /// Whether `player` still has to ship this turn.
    pub fn can_ship(game: &Game, player: PlayerIndex) -> bool {
        game.current_main_phase < MainPhase::ShipmentAndMove
            || (game.current_main_phase == MainPhase::ShipmentAndMove && !game.has_acted_or_passed.contains(&game.player(player).faction))
    }

    pub fn nr_of_battles_to_fight(&self, game: &Game) -> i32 {
        Battle::battles_to_be_fought(game, self.player, false).len() as i32
    }

    /// The strength `opponent` can still ship into `to` with the spice they and their ally have.
    pub fn max_reinforced_dial_to(game: &Game, player: PlayerIndex, opponent: PlayerIndex, to: Territory) -> f32 {
        if !Self::can_ship(game, opponent) {
            return 0.0;
        }

        let o = game.player(opponent);
        let opponent_resources = o.resources + o.allied_resources(game);

        let opponent_may_use_worthless_as_karma = o.faction == Faction::BeneGesserit && game.applicable(Rule::BeneGesseritWorthlessAsKarma);
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

        let (mut special_forces, mut normal_forces) = (0, 0);
        while special_forces < o.special_forces_in_reserve() && cost(normal_forces, special_forces + 1) <= opponent_resources {
            special_forces += 1;
        }
        while normal_forces < o.forces_in_reserve() && cost(normal_forces + 1, special_forces) <= opponent_resources {
            normal_forces += 1;
        }

        special_forces as f32 * Battle::determine_special_force_strength(game, o.faction, game.player(player).faction)
            + normal_forces as f32 * Battle::determine_normal_force_strength(game, o.faction)
    }

    /// The leaders that I know won't betray me: my traitors and known non-traitors and my ally's,
    /// and every traitor revealed or told.
    pub fn safe_or_known_traitor_leaders(&self, game: &Game) -> Vec<Hero> {
        let me = game.player(self.player);
        let mut leaders: Vec<Hero> = me.traitors.iter().chain(me.known_non_traitors.iter()).copied().collect();
        if me.has_ally() {
            // Throws when my ally isn't seated.
            let ally = game.player(self.allied_player(game).nre());
            leaders.extend(ally.traitors.iter().chain(ally.known_non_traitors.iter()));
        }
        for p in game.player_indices() {
            let pl = game.player(p);
            leaders.extend(pl.revealed_traitors.iter().chain(pl.told_traitors.iter()));
        }
        distinct(leaders)
    }

    pub fn known_opponent_weapons(game: &Game, player: PlayerIndex, opponent: PlayerIndex) -> Vec<TreacheryCard> {
        Self::known_opponent_cards(game, player, opponent).into_iter().filter(|c| c.is_weapon()).collect()
    }

    /// The cards of `opponent` that `player` knows of.
    pub fn known_opponent_cards(game: &Game, player: PlayerIndex, opponent: PlayerIndex) -> Vec<TreacheryCard> {
        let known = game.known_cards(player);
        game.player(opponent).treachery_cards.iter().copied().filter(|c| known.contains(c)).collect()
    }

    pub fn known_opponent_defenses(game: &Game, player: PlayerIndex, opponent: PlayerIndex) -> Vec<TreacheryCard> {
        Self::known_opponent_cards(game, player, opponent).into_iter().filter(|c| c.is_defense()).collect()
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

    /// The first useless card `player` can play as a weapon.
    pub fn useless_as_weapon(game: &Game, player: PlayerIndex, using_this_defense: Option<TreacheryCard>) -> Option<TreacheryCard> {
        Self::weapons(game, player, using_this_defense, None, None).into_iter().find(|c| c.type_() == TreacheryCardType::Useless)
    }

    /// The last useless card `player` can play as a defense.
    pub fn useless_as_defense(game: &Game, player: PlayerIndex, using_this_weapon: Option<TreacheryCard>) -> Option<TreacheryCard> {
        Self::defenses(game, player, using_this_weapon, None).into_iter().rev().find(|c| c.type_() == TreacheryCardType::Useless)
    }

    pub fn may_play_no_weapon(game: &Game, player: PlayerIndex, using_this_defense: Option<TreacheryCard>) -> bool {
        let using_this_defense = using_this_defense.and_then(CsNull::opt);
        Battle::valid_weapons(game, player, using_this_defense, None, None, true).iter().any(|c| c.and_then(CsNull::opt).is_none())
    }

    pub fn may_play_no_defense(game: &Game, player: PlayerIndex, using_this_weapon: Option<TreacheryCard>) -> bool {
        let using_this_weapon = using_this_weapon.and_then(CsNull::opt);
        Battle::valid_defenses(game, player, using_this_weapon, None, true).iter().any(|c| c.and_then(CsNull::opt).is_none())
    }

    /// How many kinds of weapon `cards` hold: projectile, poison, both, lasgun, artillery, poison tooth.
    pub fn count_different_weapon_types(cards: &[TreacheryCard]) -> i32 {
        let both = |c: &TreacheryCard| c.type_() == TreacheryCardType::ProjectileAndPoison;
        let kinds: [&dyn Fn(&TreacheryCard) -> bool; 6] = [
            &|c| c.is_projectile_weapon() && !both(c),
            &|c| c.is_poison_weapon() && !both(c),
            &both,
            &|c| c.is_laser(),
            &|c| c.is_artillery(),
            &|c| c.is_poison_tooth(),
        ];
        kinds.iter().filter(|kind| cards.iter().any(|c| kind(c))).count() as i32
    }

    /// How many kinds of defense `cards` hold: projectile, poison, both.
    pub fn count_different_defense_types(cards: &[TreacheryCard]) -> i32 {
        let both = |c: &TreacheryCard| c.type_() == TreacheryCardType::ShieldAndAntidote;
        let kinds: [&dyn Fn(&TreacheryCard) -> bool; 3] = [&|c| c.is_projectile_defense() && !both(c), &|c| c.is_poison_defense() && !both(c), &both];
        kinds.iter().filter(|kind| cards.iter().any(|c| kind(c))).count() as i32
    }
}
