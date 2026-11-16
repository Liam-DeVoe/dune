use crate::prelude::*;
use crate::messages::part::Part;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct AmbassadorActivated {
    pub bene_gesserit_selected_ambassador: Option<Ambassador>,
    #[serde(with = "crate::events::wire::id_list")]
    #[ts(as = "Option<Vec<i32>>")]
    pub choam_card_ids: Option<String>,
    pub ecaz_offer_alliance: bool,
    pub ecaz_give_vidal_to_ally: bool,
    pub ecaz_take_vidal: bool,
    pub fremen_or_guild_to: Option<Location>,
    #[serde(with = "crate::events::wire::force_list")]
    #[ts(as = "Option<Vec<crate::events::wire::ForceEntry>>")]
    pub fremen_force_locations: Option<String>,
    pub ixian_card: Option<TreacheryCard>,
    pub guild_force_amount: u16,
    pub tleilaxu_amount_of_forces: u16,
    pub tleilaxu_hero: Option<Hero>,
    pub tleilaxu_assign_skill: bool,
    #[serde(skip)]
    pub passed: bool,
}

impl Default for AmbassadorActivated {
    fn default() -> Self {
        AmbassadorActivated {
            bene_gesserit_selected_ambassador: Default::default(),
            choam_card_ids: Some(String::new()),
            ecaz_offer_alliance: Default::default(),
            ecaz_give_vidal_to_ally: Default::default(),
            ecaz_take_vidal: Default::default(),
            fremen_or_guild_to: Location::from_id(0),
            fremen_force_locations: Some("".to_string()),
            ixian_card: TreacheryCardManager::get(0),
            guild_force_amount: Default::default(),
            tleilaxu_amount_of_forces: Default::default(),
            tleilaxu_hero: None,
            tleilaxu_assign_skill: Default::default(),
            passed: Default::default(),
        }
    }
}

impl AmbassadorActivated {
    pub fn choam_cards(&self) -> Vec<TreacheryCard> {
        Event::id_string_to_objects(self.choam_card_ids.as_deref(), &TreacheryCardManager::lookup())
    }

    pub fn set_choam_cards(&mut self, value: &[TreacheryCard]) {
        self.choam_card_ids = Some(Event::objects_to_id_string(value, &TreacheryCardManager::lookup()));
    }

    /// The Fremen ambassador's force list, of the submitter `by`. Re-parsed on every read.
    pub fn fremen_force_locations(&self, game: &Game, by: By) -> IndexMap<Location, Battalion> {
        PlacementEvent::force_locations_by(game, by, &self.fremen_force_locations)
    }

    pub fn set_fremen_force_locations(&mut self, game: &Game, value: &IndexMap<Location, Battalion>) {
        self.fremen_force_locations = Some(PlacementEvent::force_locations_string(game, value));
    }

    pub fn get_territory(g: &Game) -> Option<Territory> {
        g.last_ambassador_trigger().map(|t| t.territory(g))
    }

    pub fn get_victim(g: &Game) -> Option<Faction> {
        match g.last_ambassador_trigger() {
            Some(t) => Some(t.initiator()),
            None => None,
        }
    }

    pub fn get_victim_player(g: &Game) -> Option<PlayerId> {
        let victim = AmbassadorActivated::get_victim(g);
        if victim != None { g.get_player(victim) } else { None }
    }

    pub fn get_ambassador(g: &Game) -> Option<Ambassador> {
        let territory = AmbassadorActivated::get_territory(g);

        match territory {
            Some(territory) if g.last_ambassador_trigger().is_some() => g.ambassador_in(territory),
            _ => None,
        }
    }

    pub fn alliance_can_be_offered(g: &Game, p: PlayerId) -> bool {
        let victim_player = AmbassadorActivated::get_victim_player(g);
        !g.player(p).has_ally() && victim_player.map(|v| !g.player(v).has_ally()) == Some(true)
    }

    pub fn vidal_can_be_offered_to_new_ally(g: &Game, _p: PlayerId) -> bool {
        g.vidal_is_alive()
    }

    pub fn vidal_can_be_taken(g: &Game, p: PlayerId) -> bool {
        (g.vidal().is_some() && !g.player(p).has_leader(g.vidal().expect("vidal")))
            && g.vidal_is_alive()
            && !g.vidal_is_captured_or_ghola()
            && g.occupier_of(World::Ecaz).is_none()
    }

    pub fn get_valid_bene_gesserit_ambassadors(g: &Game) -> Vec<Ambassador> {
        g.decks.unassigned_ambassadors.as_ref().expect("unassigned ambassadors").items.clone()
    }

    pub fn get_valid_choam_cards(game: &Game, p: PlayerId) -> Vec<TreacheryCard> {
        game.player(p).treachery_cards.clone()
    }

    pub fn get_valid_ixian_cards(game: &Game, p: PlayerId) -> Vec<TreacheryCard> {
        game.player(p).treachery_cards.clone()
    }

    pub fn valid_fremen_sources(g: &Game, p: PlayerId) -> Vec<Territory> {
        PlacementEvent::valid_movement_sources(g, p)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_fremen_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Location> {
        g.board.map
            .locations(false)
            .filter(move |l| l.visible(&g.board.map) && !Sector::in_storm(l.sector(&g.board.map), g.board.sector_in_storm) && g.is_not_full(p, *l))
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn valid_guild_targets(g: &Game, p: PlayerId) -> impl Iterator<Item = Location> {
        let hms = g.board.map.hidden_mobile_stronghold().location();
        g.board.map
            .locations(g.applicable(Rule::Homeworlds))
            .filter(move |l| {
                !Sector::in_storm(l.sector(&g.board.map), g.board.sector_in_storm)
                    && (*l != hms || g.player(p).is(Faction::Ixian))
                    && !g.contains_conflicting_ally(p, *l)
                    && Shipment::is_either_valid_homeworld_or_no_homeworld(g, p, *l, false)
                    && Shipment::is_either_valid_discovery_or_no_discovery(&g.board.map, *l)
                    && g.is_not_full(p, *l)
            })
    }

    pub fn valid_guild_max_forces(game: &Game, p: PlayerId) -> i32 {
        std::cmp::min(game.player(p).forces_in_reserve(), 4)
    }

    pub fn valid_tleilaxu_max_amount(game: &Game, p: PlayerId) -> i32 {
        std::cmp::min(i32::from(game.player(p).forces_killed), 4)
    }

    pub fn valid_tleilaxu_heroes(game: &Game, player: PlayerId) -> Vec<Hero> {
        game.killed_heroes(player)
    }

    pub fn may_pass(game: &Game, p: PlayerId) -> bool {
        game.player(p).faction == Some(Faction::Ecaz)
    }

    pub fn handle_ambassador(&self, game: &mut Game, by: By, ambassador_faction: Ambassador, victim: Faction) -> GameResult {
        let victim_player = game.get_player(Some(victim));

        game.log(parts![by.faction, " activate the ", ambassador_faction, " ambassador"]);

        match ambassador_faction {
            Ambassador::Atreides => {
                if !game.player(victim_player.required("victim player")?).treachery_cards.is_empty() {
                    game.log(parts![by.faction, " see all treachery cards owned by ", victim]);
                    // A copy of the hand as it is now: later changes to it don't change this message.
                    let hand = &game.player(victim_player.required("victim player")?).treachery_cards;
                    let cards = Part::list(hand.iter().copied());
                    game.log_to(by.faction.required("faction")?, parts![victim, " own: ", cards]);
                } else {
                    game.log(parts![victim, " don't own any cards"]);
                }

                game.determine_next_shipment_and_move_sub_phase()?;
            }

            Ambassador::Choam => {
                let mut total_earned = 0i32;
                for c in self.choam_cards() {
                    total_earned += 3;
                    game.discard_card_of(Some(by.player()), Some(c), true);
                }

                game.log(parts![by.faction, " get ", Payment::of(total_earned)]);
                let p = game.player_mut(by.player());
                crate::game::amounts::gain(&mut p.resources, total_earned, "spice")?;
                game.determine_next_shipment_and_move_sub_phase()?;
            }

            Ambassador::Ecaz => {
                if self.ecaz_offer_alliance {
                    let vidal = game.vidal();
                    game.log(parts![
                            by.faction,
                            " offer an alliance to ",
                            victim,
                            express_part_if!(self.ecaz_give_vidal_to_ally; " offering ", vidal, " if they accept")
                        ]);
                    game.interrupt(Interrupt::AllianceByAmbassador {
                        offered_to: victim,
                        offered_by: by,
                        give_vidal_to_ally: self.ecaz_give_vidal_to_ally,
                        take_vidal: self.ecaz_take_vidal,
                    });
                } else {
                    if self.ecaz_take_vidal {
                        game.take_vidal(by.player(), VidalMoment::Never);
                    }

                    game.determine_next_shipment_and_move_sub_phase()?;
                }
            }

            Ambassador::Emperor => {
                let p = game.player_mut(by.player());
                p.resources += 5;
                game.log(parts![by.faction, " get ", Payment::of(5)]);
                game.determine_next_shipment_and_move_sub_phase()?;
            }

            Ambassador::Fremen => {
                let force_locations = self.fremen_force_locations(game, by);
                let this = Event::from(self.clone());
                game.perform_move_from_locations(by.player(), &force_locations, &this, false, false)?;
                game.turn.recent_moves.push(RecentMove { initiator: by.faction.expect("faction"), to: self.fremen_or_guild_to, force_locations: force_locations.clone() });
                game.check_intrusion(&this, by.faction.expect("faction"))?;
                game.determine_next_shipment_and_move_sub_phase()?;
            }

            Ambassador::Harkonnen => {
                if victim == Faction::Harkonnen {
                    game.log(parts![by.faction, " see one of the ", Faction::Harkonnen, " traitors"]);
                } else if victim == Faction::Tleilaxu {
                    game.log(parts![by.faction, " see one of the ", Faction::Tleilaxu, " unrevealed Face Dancers"]);
                } else {
                    game.log(parts![by.faction, " see the ", victim, " traitor"]);
                }

                let vp = game.player(victim_player.required("victim player")?);
                let to_select_from: Vec<Hero> = if victim == Faction::Tleilaxu {
                    vp.face_dancers.iter().copied().filter(|t| !vp.revealed_face_dancers.contains(t)).collect()
                } else {
                    vp.traitors.clone()
                };

                let revealed = to_select_from.into_iter().random_item(&mut game.rng);
                game.log_to(by.faction.required("faction")?, parts![victim, " reveal ", revealed]);
                game.log_to(victim, parts![by.faction, " get to see ", revealed]);
                game.determine_next_shipment_and_move_sub_phase()?;
            }

            Ambassador::Ixian => {
                game.discard_card_of(Some(by.player()), self.ixian_card, true);
                game.log(parts![by.faction, " draw a new card"]);
                let drawn = game.draw_treachery_card();
                if let Some(drawn) = drawn {
                    game.player_mut(by.player()).treachery_cards.push(drawn);
                }
                game.determine_next_shipment_and_move_sub_phase()?;
            }

            Ambassador::Richese => {
                game.log(parts![by.faction, " buy a card for ", Payment::of(3)]);
                let p = game.player_mut(by.player());
                p.resources -= 3;
                let drawn_card = game.draw_treachery_card();
                if let Some(drawn_card) = drawn_card {
                    game.player_mut(by.player()).treachery_cards.push(drawn_card);
                }
                game.determine_next_shipment_and_move_sub_phase()?;
            }

            Ambassador::Guild => {
                let force = game.player(by.player()).force();
                let to = self.fremen_or_guild_to;
                game.log(parts![by.faction, " send ", i32::from(self.guild_force_amount), force, " to ", to]);
                if let Some(to) = self.fremen_or_guild_to {
                    Player::ship_forces(game, by.player(), to, i32::from(self.guild_force_amount))?;
                }
                let this = Event::from(self.clone());
                game.turn.last_shipment_or_movement = Some(ShipmentOrMovement { to: self.fremen_or_guild_to });
                game.check_intrusion(&this, by.faction.expect("faction"))?;
                game.determine_next_shipment_and_move_sub_phase()?;
            }

            Ambassador::Tleilaxu => {
                game.determine_next_shipment_and_move_sub_phase()?;
                if let Some(tleilaxu_hero) = self.tleilaxu_hero {
                    if !game.is_face_down_dead(tleilaxu_hero) {
                        game.log(parts![by.faction, " revive ", tleilaxu_hero]);
                    } else {
                        game.log(parts![by.faction, " revive a face down leader"]);
                    }

                    game.revive(by.player(), tleilaxu_hero);

                    if self.tleilaxu_assign_skill {
                        game.prepare_skill_assignment_to_revived_leader(by.player(), tleilaxu_hero.as_leader());
                    }
                } else {
                    let force = game.player(by.player()).force();
                    game.log(parts![by.faction, " revive ", i32::from(self.tleilaxu_amount_of_forces), " ", force]);
                    Player::revive_forces(game, by.player(), i32::from(self.tleilaxu_amount_of_forces))?;
                }
            }

            _ => {}
        }
        Ok(())
    }
}

impl EventRules for AmbassadorActivated {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if !self.passed {
            // `var player = Player;` is only dereferenced where needed.
            let player = || by.player();
            let mut ambassador = AmbassadorActivated::get_ambassador(game);

            if ambassador == Some(Ambassador::BeneGesserit) {
                if !AmbassadorActivated::get_valid_bene_gesserit_ambassadors(game).iter().any(|a| Some(*a) == self.bene_gesserit_selected_ambassador) {
                    return Err(Rejection(express!("Invalid Ambassador selected")));
                }
                ambassador = self.bene_gesserit_selected_ambassador;
            }

            match ambassador {
                Some(Ambassador::Choam) => {
                    if self
                        .choam_cards()
                        .iter()
                        .any(|c| !AmbassadorActivated::get_valid_choam_cards(game, player()).contains(c))
                    {
                        return Err(Rejection(express!("Invalid card selected")));
                    }
                }

                Some(Ambassador::Ecaz) => {
                    if self.ecaz_offer_alliance && !AmbassadorActivated::alliance_can_be_offered(game, player()) {
                        return Err(Rejection(express!("You can't offer an alliance")));
                    }
                    if self.ecaz_take_vidal && !AmbassadorActivated::vidal_can_be_taken(game, by.player()) {
                        return Err(Rejection(express!("You can't take ", game.vidal())));
                    }
                    if self.ecaz_offer_alliance
                        && self.ecaz_give_vidal_to_ally
                        && !AmbassadorActivated::vidal_can_be_offered_to_new_ally(game, player())
                    {
                        return Err(Rejection(express!("You can't offer ", game.vidal(), " to your ally")));
                    }
                    if self.ecaz_take_vidal && self.ecaz_give_vidal_to_ally {
                        return Err(Rejection(express!("You don't have ", game.vidal(), " yet")));
                    }
                    if self.ecaz_take_vidal && self.ecaz_offer_alliance {
                        return Err(Rejection(express!("You can't both take ", game.vidal(), " AND offer an alliance")));
                    }
                }

                Some(Ambassador::Fremen) => {
                    if self.fremen_force_locations(game, by).iter().any(|(k, _)| game.is_in_storm(*k)) {
                        return Err(Rejection(express!("Can't move from storm")));
                    }
                    if self
                        .fremen_force_locations(game, by)
                        .iter()
                        .any(|(k, v)| game.player(player()).forces_in(Some(*k)) < v.forces())
                    {
                        return Err(Rejection(express!("Invalid amount of ", game.player(by.player()).force())));
                    }
                    if self
                        .fremen_force_locations(game, by)
                        .iter()
                        .any(|(k, v)| game.player(player()).special_forces_in(Some(*k)) < v.special_forces())
                    {
                        return Err(Rejection(express!("Invalid amount of ", game.player(by.player()).special_force())));
                    }
                    let to = self.fremen_or_guild_to;
                    if !AmbassadorActivated::valid_fremen_targets(game, player()).into_iter().any(|l| Some(l) == to) {
                        return Err(Rejection(express!("Invalid target location")));
                    }
                }

                Some(Ambassador::Ixian) => {
                    if AmbassadorActivated::get_valid_ixian_cards(game, player()).is_empty() {
                        return Err(Rejection(express!("You don't have a card to discard")));
                    }
                    if self.ixian_card.is_some()
                        && !AmbassadorActivated::get_valid_ixian_cards(game, player()).contains(&self.ixian_card.expect("ixian card"))
                    {
                        return Err(Rejection(express!("Invalid card selected")));
                    }
                }

                Some(Ambassador::Richese) => {
                    if i32::from(game.player(player()).resources) < 3 {
                        return Err(Rejection(express!("You don't have enough ", Concept::Resource, " to buy a card")));
                    }
                    if !game.player(player()).has_room_for_cards(game) {
                        return Err(Rejection(express!("You don't have room for an additional card")));
                    }
                }

                Some(Ambassador::Guild) => {
                    let to = self.fremen_or_guild_to;
                    if !AmbassadorActivated::valid_guild_targets(game, player()).into_iter().any(|l| Some(l) == to) {
                        return Err(Rejection(express!("Invalid target location")));
                    }
                    if i32::from(self.guild_force_amount) > AmbassadorActivated::valid_guild_max_forces(game, player()) {
                        return Err(Rejection(express!("Invalid amount of forces")));
                    }
                }

                Some(Ambassador::Tleilaxu) => {
                    if i32::from(self.tleilaxu_amount_of_forces) > AmbassadorActivated::valid_tleilaxu_max_amount(game, player()) {
                        return Err(Rejection(express!("You can't revive that many")));
                    }
                    if i32::from(self.tleilaxu_amount_of_forces) > 0 && self.tleilaxu_hero.is_some() {
                        return Err(Rejection(express!("You can't revive both forces and a leader")));
                    }
                    if self.tleilaxu_hero.is_some()
                        && !AmbassadorActivated::valid_tleilaxu_heroes(game, player()).contains(&self.tleilaxu_hero.expect("tleilaxu hero"))
                    {
                        return Err(Rejection(express!("Invalid leader")));
                    }
                    if self.tleilaxu_assign_skill && self.tleilaxu_hero.is_none() {
                        return Err(Rejection(express!("Select a leader to assign a skill to")));
                    }
                    if self.tleilaxu_assign_skill && !Revival::may_assign_skill(game, by.player(), self.tleilaxu_hero) {
                        return Err(Rejection(express!("You can't assign a skill to this leader")));
                    }
                }

                _ => {}
            }
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let ambassador_faction = AmbassadorActivated::get_ambassador(game);
        let territory = AmbassadorActivated::get_territory(game);
        let victim = AmbassadorActivated::get_victim(game);
        game.dequeue_intrusion(IntrusionType::Ambassador)?;

        if let (false, Some(territory)) = (self.passed, territory) {
            game.board.ambassadors_on_planet.shift_remove(&territory);
            game.stone(Milestone::AmbassadorActivated);
            let mut ambassador_faction = ambassador_faction.required("ambassador")?;

            let ecaz = game.get_player(Some(Faction::Ecaz));

            if ambassador_faction == Ambassador::BeneGesserit {
                ambassador_faction = self.bene_gesserit_selected_ambassador.required("the ambassador Bene Gesserit selected")?;
                game.log(parts!["The ", Ambassador::BeneGesserit, " Ambassador is removed from the game"]);
            } else if ambassador_faction == Ambassador::Ecaz {
                game.player_mut(ecaz.required("ecaz")?).ambassadors.push(ambassador_faction);
                game.log(parts!["The ", ambassador_faction, " Ambassador returns to supply"]);
            } else {
                game.decks.ambassadors_set_aside.push(ambassador_faction);
                game.log(parts!["The ", ambassador_faction, " Ambassador is set aside"]);
            }

            self.handle_ambassador(game, by, ambassador_faction, victim.required("faction")?)?;

            let ecaz = ecaz.required("ecaz")?;
            let all_ecaz = game
                .player(ecaz)
                .ambassadors
                .iter()
                .chain(game.board.ambassadors_on_planet.values())
                .all(|f| *f == Ambassador::Ecaz);
            if all_ecaz {
                game.assign_random_ambassadors(ecaz);
                game.log(parts![Faction::Ecaz, " draw 5 random Ambassadors"]);
            }
        } else {
            game.log(parts![by.faction, " don't activate an Ambassador"]);
            game.determine_next_shipment_and_move_sub_phase()?;
        }
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, if self.passed { " don't" } else { "" }, " activate an Ambassador")
    }
}

impl crate::events::location_event::LocationEvent for AmbassadorActivated {
    fn to(&self, _game: &Game) -> Option<Location> {
        self.fremen_or_guild_to
    }

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32 {
        let (forces, special_forces) = PlacementEvent::force_amounts(game, &self.fremen_force_locations);
        forces + special_forces
    }

    fn forces_added_to_location(&self, game: &Game) -> i32 {
        PlacementEvent::force_amounts(game, &self.fremen_force_locations).0
    }

    fn special_forces_added_to_location(&self, game: &Game) -> i32 {
        PlacementEvent::force_amounts(game, &self.fremen_force_locations).1
    }
}
