use crate::prelude::*;

/// The state of every leader, and which are captured or assassinated.
#[derive(Debug)]
pub struct Leaders {
    pub leader_state: IndexMap<Hero, LeaderState>,
    pub captured_leaders: IndexMap<Leader, Faction>,
    pub assassinated: Vec<Leader>,
    pub ecaz_loyal_leader: Option<Leader>,
    pub player_to_set_aside_vidal: Option<PlayerId>,
    pub when_to_set_aside_vidal: Option<VidalMoment>,
}

impl Leaders {
    pub fn new() -> Self {
        Leaders {
            leader_state: Default::default(),
            captured_leaders: Default::default(),
            assassinated: Vec::new(),
            ecaz_loyal_leader: None,
            player_to_set_aside_vidal: None,
            when_to_set_aside_vidal: Default::default(),
        }
    }
}

/// `List<T>.Remove(item)`: removes the first equal element, returns whether it found one.
fn list_remove<T: PartialEq>(list: &mut Vec<T>, item: &T) -> bool {
    match list.iter().position(|x| x == item) {
        Some(i) => {
            list.remove(i);
            true
        }
        None => false,
    }
}

impl Game {
    pub fn kill_hero(&mut self, h: Hero) {
        if !matches!(h, Hero::Leader(_) | Hero::Messiah) {
            return;
        }

        self.leaders.leader_state[&h].kill(&mut self.decks.skill_deck, &mut self.counters);
        self.stone(Milestone::LeaderKilled);
        self.determine_if_captured_leaders_must_be_released();
        self.determine_if_killed_ghola_returns_to_original_faction(h);

        if h.hero_type() != HeroType::Vidal {
            return;
        }
        let current_owner = self.owner_of(h);
        // (Leader)h: only a Leader has HeroType Vidal, so this cast never fails in practice.
        let leader_to_remove = match h.as_leader() {
            Some(l) => l,
            None => panic!("Unable to cast object of type 'Treachery.Shared.Messiah' to type 'Treachery.Shared.Leader'."),
        };
        list_remove(&mut self.player_mut(current_owner.expect("current owner")).leaders, &leader_to_remove);
        let ecaz = self.get_player(Some(Faction::Ecaz));
        if let Some(ecaz) = ecaz {
            self.player_mut(ecaz).leaders.push(leader_to_remove);
        }
    }

    pub fn revive(&mut self, initiator: PlayerId, h: Hero) {
        self.leaders.leader_state[&h].revive();
        self.leaders.leader_state[&h].current_territory = None;

        let current_owner = self.owner_of(h);
        if let (Some(current_owner), Hero::Leader(l)) = (current_owner, h) {
            list_remove(&mut self.player_mut(current_owner).leaders, &l);
            self.player_mut(initiator).leaders.push(l);
        }
    }

    pub fn determine_if_killed_ghola_returns_to_original_faction(&mut self, l: Hero) {
        let tleilaxu = self.get_player(Some(Faction::Tleilaxu));
        if let (Some(tleilaxu), Hero::Leader(leader)) = (tleilaxu, l) {
            if self.player(tleilaxu).leaders.contains(&leader) && l.faction() != Some(Faction::Tleilaxu) {
                list_remove(&mut self.player_mut(tleilaxu).leaders, &leader);

                if let Some(owner) = self.get_player(l.faction()) {
                    self.player_mut(owner).leaders.push(leader);
                }
            }
        }
    }

    pub fn assassinate_leader(&mut self, l: Leader) {
        self.leaders.leader_state[&Hero::Leader(l)].assassinate(&mut self.decks.skill_deck, &mut self.counters);
        self.stone(Milestone::LeaderKilled);

        self.determine_if_captured_leaders_must_be_released();
        self.determine_if_killed_ghola_returns_to_original_faction(Hero::Leader(l));
    }

    pub fn is_alive(&self, l: Hero) -> bool {
        self.leaders.leader_state[&l].alive()
    }

    pub fn is_face_down_dead(&self, l: Hero) -> bool {
        self.leaders.leader_state[&l].is_face_down_dead()
    }

    pub fn death_count(&self, h: Hero) -> i32 {
        self.leaders.leader_state[&h].death_counter
    }

    pub fn can_fight_in(&self, h: Hero, t: Territory) -> bool {
        let territory = self.leaders.leader_state[&h].current_territory;
        territory.is_none() || territory == Some(t)
    }

    pub fn skilled_as(&self, leader: Hero, skill: LeaderSkill) -> bool {
        self.skill(Some(leader)) == Some(skill)
    }

    pub fn has_skill_in_front_of_shield(&self, p: PlayerId, skill: LeaderSkill) -> bool {
        self.player(p)
            .leaders
            .iter()
            .any(|l| self.skill(Some(Hero::Leader(*l))) == Some(skill) && self.is_in_front_of_shield(Some(Hero::Leader(*l))))
    }

    pub fn is_skilled(&self, l: Option<Hero>) -> bool {
        l.is_some() && self.skill(l) != None
    }

    pub fn player_skilled_as(&self, skill: LeaderSkill) -> Option<PlayerId> {
        self.player_indices().find(|p| self.has_skill_in_front_of_shield(*p, skill))
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn get_skilled_leaders(&self, player: PlayerId) -> impl Iterator<Item = Leader> {
        self.player(player).leaders.iter().copied().filter(move |l| self.is_skilled(Some(Hero::Leader(*l))))
    }

    pub fn get_skill(&self, p: PlayerId) -> Option<LeaderSkill> {
        let hero = self
            .player(p)
            .leaders
            .iter()
            .copied()
            .filter(|l| self.is_skilled(Some(Hero::Leader(*l))))
            .find(|l| self.is_in_front_of_shield(Some(Hero::Leader(*l))));
        hero.and_then(|h| self.skill(Some(Hero::Leader(h))))
    }

    pub fn skill(&self, l: Option<Hero>) -> Option<LeaderSkill> {
        let Some(l) = l else { return None };

        match self.leaders.leader_state.get(&l) {
            Some(state) => state.skill,
            None => None,
        }
    }

    pub fn set_skill(&mut self, l: Leader, skill: Option<LeaderSkill>) {
        self.leaders.leader_state[&Hero::Leader(l)].skill = skill;
    }

    pub fn set_in_front_of_shield(&mut self, l: Leader, value: bool) {
        if let Some(ls) = self.leaders.leader_state.get_mut(&Hero::Leader(l)) {
            ls.in_front_of_shield = value;
        }
    }

    pub fn is_in_front_of_shield(&self, l: Option<Hero>) -> bool {
        let Some(l) = l else { return false };
        self.leaders.leader_state.contains_key(&l) && self.leaders.leader_state[&l].in_front_of_shield
    }

    pub fn messiah_is_alive(&self) -> bool {
        self.is_alive(LeaderManager::messiah())
    }

    pub fn has_something_to_revive(&self, player: PlayerId) -> bool {
        let mut player = player;
        loop {
            let p = self.player(player);
            if p.forces_killed > 0 || p.special_forces_killed > 0 || !Revival::valid_revival_heroes(self, player).next().is_none() {
                return true;
            }

            let ally = self.get_player(p.ally);
            match ally {
                Some(ally) if p.is(Faction::Tleilaxu) && p.ally != None => player = ally,
                _ => return false,
            }
        }
    }

    pub fn killed_heroes(&self, p: PlayerId) -> Vec<Hero> {
        let mut result: Vec<Hero> = Vec::new();
        result.extend(self.player(p).leaders.iter().filter(|l| !self.is_alive(Hero::Leader(**l))).map(|l| Hero::Leader(*l)));

        if self.player(p).is(Faction::Atreides) && !self.is_alive(LeaderManager::messiah()) {
            result.push(LeaderManager::messiah());
        }

        result
    }
}

#[derive(Clone, Debug)]
pub struct LeaderState {
    pub current_territory: Option<Territory>,
    pub death_counter: i32,
    /// When the leader last died, from `game.counters`. Only the order means something: the
    /// view lists the leaders in the tanks by it.
    pub time_of_death: i32,
    pub skill: Option<LeaderSkill>,
    pub in_front_of_shield: bool,
}

impl Default for LeaderState {
    fn default() -> Self {
        LeaderState {
            current_territory: Default::default(),
            death_counter: Default::default(),
            time_of_death: Default::default(),
            skill: Default::default(),
            in_front_of_shield: Default::default(),
        }
    }
}

impl LeaderState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alive(&self) -> bool {
        self.death_counter % 2 == 0
    }

    pub fn is_face_down_dead(&self) -> bool {
        self.death_counter % 4 == 3
    }

    /// Takes the skill deck and the counters instead of the game, because this state lives in
    /// `game.leaders` and Rust can't borrow both.
    pub fn kill(&mut self, skill_deck: &mut Option<Deck<LeaderSkill>>, counters: &mut Counters) {
        if let Some(skill) = self.skill {
            skill_deck.as_mut().expect("skill deck").put_on_top(skill);
            self.skill = None;
            self.in_front_of_shield = false;
        }

        self.death_counter += 1;
        self.time_of_death = counters.next_leader_death();
    }

    pub fn assassinate(&mut self, skill_deck: &mut Option<Deck<LeaderSkill>>, counters: &mut Counters) {
        self.kill(skill_deck, counters);
        if !self.is_face_down_dead() {
            self.revive();
            self.kill(skill_deck, counters);
        }
    }

    pub fn revive(&mut self) {
        self.death_counter += 1;
    }

}

impl Game {
    pub fn initialize_leader_state(&mut self) {
        for l in LeaderManager::leaders() {
            self.leaders.leader_state.insert_new(Hero::Leader(*l), LeaderState { death_counter: 0, current_territory: None, skill: None, in_front_of_shield: false, ..Default::default() });
        }

        self.leaders.leader_state.insert_new(LeaderManager::messiah(), LeaderState { death_counter: 0, current_territory: None, ..Default::default() });
    }

    pub fn check_if_occupier_takes_vidal(&mut self, previous_occupier_of_ecaz_homeworld: Option<PlayerId>) {
        if self.vidal().is_none() {
            return;
        }

        let occupier_of_ecaz_homeworld = self.occupier_of(World::Ecaz);
        if let Some(occupier) = occupier_of_ecaz_homeworld {
            let vidal = self.vidal().expect("vidal");
            if !self.player(occupier).leaders.contains(&vidal) {
                self.take_vidal(occupier, VidalMoment::WhileEcazWorldIsOccupied);
            }
        } else if let Some(previous) = previous_occupier_of_ecaz_homeworld {
            let vidal = self.vidal();
            if vidal.map_or(false, |v| self.player(previous).leaders.contains(&v)) {
                let vidal = self.vidal().expect("vidal");
                // List.Remove: the first equal element
                let leaders = &mut self.player_mut(previous).leaders;
                if let Some(pos) = leaders.iter().position(|l| *l == vidal) {
                    leaders.remove(pos);
                }
                let faction = self.player(previous).faction;
                let vidal = self.vidal();
                self.log(vec![faction.into(), " lose ".into(), vidal.into()]);
            }
        }
    }

    pub fn set_aside_vidal(&mut self) {
        if self.vidal().is_none() || self.leaders.player_to_set_aside_vidal.is_none() {
            return;
        }

        let vidal = self.vidal().expect("vidal");
        let p = self.leaders.player_to_set_aside_vidal.expect("player to set aside vidal");
        if self.is_alive(Hero::Leader(vidal)) && self.player(p).leaders.contains(&vidal) {
            let leaders = &mut self.player_mut(p).leaders;
            if let Some(pos) = leaders.iter().position(|l| *l == vidal) {
                leaders.remove(pos);
            }
            let vidal = self.vidal();
            self.log(vec![vidal.into(), " is set aside".into()]);
        }

        self.leaders.player_to_set_aside_vidal = None;
        self.leaders.when_to_set_aside_vidal = None;
    }
}
